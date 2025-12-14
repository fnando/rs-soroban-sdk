use sha2::{Digest, Sha256};
use soroban_ledger_fetch_from_history_archive::{
    get_bucket, get_history, parse_bucket, parse_history,
};
use soroban_ledger_fetch_from_meta_storage::{get_ledger, parse_ledger};
use soroban_ledger_fetch_from_rpc::{get_ledger_entry, parse_ledger_entry};
use soroban_sdk::xdr::{BucketEntry, LedgerEntry, LedgerKey, Limited, Limits, WriteXdr};
use std::path::PathBuf;

mod cache;
pub use cache::{cache, CacheError};

mod iter;
pub use iter::{LedgerEntryChangesIterator, ProcessingPhase};

/// A ledger entry with its optional TTL (time-to-live)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LedgerEntryWithTtl {
    pub entry: LedgerEntry,
    pub ttl: Option<u32>,
}

/// Error type for LedgerEntryFetcher operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cache error: {0}")]
    Cache(#[from] cache::CacheError<Box<dyn std::error::Error>>),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("xdr error: {0}")]
    Xdr(#[from] soroban_sdk::xdr::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("meta storage error: {0}")]
    MetaStorage(#[from] soroban_ledger_fetch_from_meta_storage::Error),
    #[error("rpc error: {0}")]
    Rpc(#[from] soroban_ledger_fetch_from_rpc::Error),
    #[error("history archive error: {0}")]
    HistoryArchive(#[from] soroban_ledger_fetch_from_history_archive::Error),
}

/// Returns true if the ledger is a checkpoint ledger for the given checkpoint frequency.
pub fn is_checkpoint_ledger(ledger: u32, checkpoint_frequency: u32) -> bool {
    (ledger + 1) % checkpoint_frequency == 0
}

/// Network configuration for fetching ledger data
///
/// Contains URLs for SEP-54 meta storage, RPC, and history archive,
/// as well as the checkpoint frequency for the network.
#[derive(Debug, Clone)]
pub struct Network {
    /// Network passphrase (e.g., "Public Global Stellar Network ; September 2015")
    pub passphrase: String,
    /// URL to the SEP-54 ledger meta storage
    pub meta_url: String,
    /// URL to the RPC
    pub rpc_url: String,
    /// URL to the History Archive storage
    pub archive_url: String,
    /// Number of ledgers between checkpoints
    pub archive_checkpoint_ledger_count: u32,
}

impl Network {
    /// Create a Network configuration for Stellar mainnet with default URLs
    ///
    /// Uses default mainnet URLs:
    /// - SEP-54 meta storage: AWS public blockchain
    /// - RPC: mainnet.sorobanrpc.com
    /// - History archive: history.stellar.org
    pub fn mainnet() -> Self {
        Self {
            passphrase: "Public Global Stellar Network ; September 2015".to_string(),
            meta_url: "https://aws-public-blockchain.s3.us-east-2.amazonaws.com/v1.1/stellar/ledgers/pubnet".to_string(),
            rpc_url: "https://mainnet.sorobanrpc.com".to_string(),
            archive_url: "https://history.stellar.org/prd/core-live/core_live_001".to_string(),
            archive_checkpoint_ledger_count: 64,
        }
    }

    /// Create a Network configuration for Stellar testnet with default URLs
    ///
    /// Uses default testnet URLs:
    /// - SEP-54 meta storage: AWS public blockchain
    /// - RPC: soroban-testnet.stellar.org
    /// - History archive: history.stellar.org
    pub fn testnet() -> Self {
        Self {
            passphrase: "Test SDF Network ; September 2015".to_string(),
            meta_url: "https://aws-public-blockchain.s3.us-east-2.amazonaws.com/v1.1/stellar/ledgers/testnet/2025-08-14".to_string(),
            rpc_url: "https://soroban-testnet.stellar.org".to_string(),
            archive_url: "https://history.stellar.org/prd/core-testnet/core_testnet_001".to_string(),
            archive_checkpoint_ledger_count: 64,
        }
    }

    /// Returns the network ID, which is the SHA256 hash of the network passphrase.
    pub fn network_id(&self) -> [u8; 32] {
        Sha256::digest(self.passphrase.as_bytes()).into()
    }

    /// Returns the network ID as a hex-encoded string.
    pub fn network_id_hex(&self) -> String {
        hex::encode(self.network_id())
    }
}

/// Fetcher for ledger entries that downloads ledger meta and searches for entries
pub struct LedgerEntryFetcher {
    network: Network,
    ledger: u32,
    tx_hash: Option<[u8; 32]>,
    cache_path: PathBuf,
}

impl LedgerEntryFetcher {
    /// Create a new LedgerEntryFetcher
    ///
    /// # Arguments
    /// * `network` - Network configuration with URLs for meta storage, RPC, and history archive
    /// * `ledger` - Ledger sequence number
    /// * `tx_hash` - Optional transaction hash
    /// * `cache_path` - Path to store cache files
    pub fn new(
        network: Network,
        ledger: u32,
        tx_hash: Option<[u8; 32]>,
        cache_path: PathBuf,
    ) -> Self {
        Self {
            network,
            ledger,
            tx_hash,
            cache_path,
        }
    }

    /// Returns the ledger sequence number this fetcher is configured for.
    pub fn ledger(&self) -> u32 {
        self.ledger
    }

    /// Fetch a ledger entry by key
    ///
    /// This method uses several layers of caching to the system cache directory to avoid refetching entries.
    pub fn fetch(&self, key: &LedgerKey) -> Result<Option<LedgerEntryWithTtl>, Error> {
        self.fetch_with_entry_cache(key)
    }

    fn fetch_with_entry_cache(&self, key: &LedgerKey) -> Result<Option<LedgerEntryWithTtl>, Error> {
        let cache_path = &self.cache_path;

        // Compute cache file path: <cache_path>/<ledger>/<hash_of_key>.json
        let key_xdr = key.to_xdr(Limits::none())?;
        let key_hash = Sha256::digest(&key_xdr);
        let ledger_cache_dir = cache_path.join(format!("{}", self.ledger));

        // Ensure cache directory exists
        std::fs::create_dir_all(&ledger_cache_dir)?;

        // Use cache function to handle reading/writing cache file
        let fetch_read = cache(
            ledger_cache_dir.join(format!("{:x}.json", key_hash)),
            |write| {
                // Fetch the data
                let result = self.fetch_with_dl_cache(key, &cache_path)?;

                // Serialize to JSON
                serde_json::to_writer_pretty(write, &result)?;

                Ok(())
            },
        )?;

        // Parse the cached result
        Ok(serde_json::from_reader(fetch_read)?)
    }

    fn fetch_with_dl_cache(
        &self,
        key: &LedgerKey,
        cache_path: &PathBuf,
    ) -> Result<Option<LedgerEntryWithTtl>, Error> {
        eprintln!("looking up key {}", serde_json::to_string(key)?);

        std::fs::create_dir_all(cache_path)?;

        // Calculate checkpoint boundaries
        let checkpoint_count = self.network.archive_checkpoint_ledger_count;
        let prev_checkpoint = ((self.ledger + 1) / checkpoint_count) * checkpoint_count - 1;
        let ledgers_to_checkpoint = self.ledger - prev_checkpoint;

        // Prefetch all meta for ledgers from starting ledger down to the checkpoint (in background)
        let prefetch_cache_path = cache_path.clone();
        let prefetch_meta_url = self.network.meta_url.clone();
        let prefetch_start_ledger = self.ledger;
        std::thread::spawn(move || {
            Self::prefetch_meta(
                &prefetch_meta_url,
                &prefetch_cache_path,
                prefetch_start_ledger,
                ledgers_to_checkpoint,
            );
        });

        // Phase 1: Check the starting ledger
        eprintln!("searching ledger meta for {}", self.ledger);
        if let Some(result) = self.fetch_from_meta(&cache_path, self.ledger, key)? {
            return Ok(result);
        }

        // Optimization: Try RPC
        eprintln!("searching rpc");
        if let Some(result) = self.fetch_from_rpc(&cache_path, self.ledger, key)? {
            return Ok(result);
        }

        // Phase 2: Search through previous ledgers down to the previous checkpoint
        for ledger in (prev_checkpoint + 1..self.ledger).rev() {
            eprintln!("searching ledger meta for {ledger}");
            if let Some(result) = self.fetch_from_meta(&cache_path, ledger, key)? {
                return Ok(result);
            }
        }

        // Phase 3: Fetch from history archive at the previous checkpoint
        eprintln!("searching ledger buckets for {prev_checkpoint}");
        self.fetch_from_archive(&cache_path, prev_checkpoint, key)
    }

    fn prefetch_meta(meta_url: &str, cache_path: &PathBuf, start_ledger: u32, count: u32) {
        use std::thread;

        let ledgers_to_cache: Vec<u32> = (0..count)
            .filter_map(|i| start_ledger.checked_sub(i))
            .collect();

        eprintln!(
            "prefectching meta for {} ledgers: {:?}",
            ledgers_to_cache.len(),
            ledgers_to_cache
        );

        // Process in chunks of 10 to avoid too many open files
        const MAX_CONCURRENT_DOWNLOADS: usize = 10;
        for chunk in ledgers_to_cache.chunks(MAX_CONCURRENT_DOWNLOADS) {
            let handles: Vec<_> = chunk
                .iter()
                .map(|&l| {
                    let meta_url = meta_url.to_string();
                    let path = cache_path.join(format!("ledger-{l}.xdr"));
                    thread::spawn(move || {
                        let _ = cache(path, |write| {
                            get_ledger(&meta_url, l, write)
                                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
                        });
                    })
                })
                .collect();

            for handle in handles {
                let _ = handle.join();
            }
        }
    }

    fn fetch_from_meta(
        &self,
        cache_path: &PathBuf,
        ledger: u32,
        key: &LedgerKey,
    ) -> Result<Option<Option<LedgerEntryWithTtl>>, Error> {
        let meta_read = cache(cache_path.join(format!("ledger-{ledger}.xdr")), |write| {
            get_ledger(&self.network.meta_url, ledger, write)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        })?;
        let meta = parse_ledger(meta_read)?;

        // Only pass tx_hash for the starting ledger; for earlier ledgers, iterate fully
        let tx_hash_filter = if ledger == self.ledger {
            self.tx_hash
        } else {
            None
        };
        let changes = LedgerEntryChangesIterator::new(&meta, tx_hash_filter);
        for (_phase, tx_hash, change_key, change_entry) in changes {
            let _tx_hash_short = tx_hash
                .iter()
                .take(7)
                .map(|b| format!("{:02x}", b))
                .collect::<String>();
            //eprintln!("current phase: {:?}, tx_hash: {}", phase, tx_hash_short);
            if &change_key == key {
                if let Some(entry) = change_entry {
                    eprintln!("returned entry (meta)");
                    return Ok(Some(Some(LedgerEntryWithTtl { entry, ttl: None })));
                } else {
                    return Ok(Some(None));
                }
            }
        }

        Ok(None)
    }

    fn fetch_from_rpc(
        &self,
        cache_path: &PathBuf,
        ledger: u32,
        key: &LedgerKey,
    ) -> Result<Option<Option<LedgerEntryWithTtl>>, Error> {
        eprintln!("checking rpc");
        let key_xdr = key.to_xdr(Limits::none())?;
        let key_hash = Sha256::digest(&key_xdr);
        let rpc_read = cache(
            cache_path.join(format!("rpc-{ledger}-{key_hash:x}.json")),
            |write| {
                get_ledger_entry(&self.network.rpc_url, key, write)
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
            },
        )?;
        if let Some((entry, ttl)) = parse_ledger_entry(rpc_read)? {
            eprintln!(
                "found entry with rpc: {} (ttl: {ttl:?})",
                serde_json::to_string(&entry)?
            );
            if entry.last_modified_ledger_seq < ledger {
                eprintln!("returned entry (rpc)");
                return Ok(Some(Some(LedgerEntryWithTtl { entry, ttl })));
            } else {
                eprintln!("entry from rpc is too new");
            }
        }

        Ok(None)
    }

    fn fetch_from_archive(
        &self,
        cache_path: &PathBuf,
        ledger: u32,
        key: &LedgerKey,
    ) -> Result<Option<LedgerEntryWithTtl>, Error> {
        eprintln!("loading ledger {ledger} from checkpoint");
        let history_read = cache(
            cache_path.join(format!("history-{}.json", ledger)),
            |write| {
                get_history(&self.network.archive_url, ledger, write)
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
            },
        )?;
        let history = parse_history(history_read)?;
        let buckets = history
            .current_buckets
            .iter()
            .flat_map(|b| [&b.curr, &b.snap])
            .filter(|b| *b != "0000000000000000000000000000000000000000000000000000000000000000");
        for bucket in buckets {
            let bucket_path = cache_path.join(format!("bucket-{bucket}.xdr"));
            eprintln!("loading {bucket}");
            let bucket_read = cache(bucket_path.clone(), |write| {
                let content_length = get_bucket(&self.network.archive_url, bucket, write)
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                if let Some(len) = content_length {
                    eprintln!("downloaded {bucket} ({len} bytes compressed)");
                }
                Ok(())
            })?;
            let file_size = std::fs::metadata(&bucket_path)
                .map(|m| m.len())
                .unwrap_or(0);
            let mut limited_reader = Limited::new(bucket_read, Limits::none());
            let bucket_entries_iter = parse_bucket(&mut limited_reader);
            eprintln!("searching {bucket} ({file_size} bytes)");
            for entry_result in bucket_entries_iter {
                let entry = entry_result?.0;
                match entry {
                    BucketEntry::Liveentry(ledger_entry) | BucketEntry::Initentry(ledger_entry) => {
                        if ledger_entry.to_key() == *key {
                            eprintln!("returned entry (archive)");
                            return Ok(Some(LedgerEntryWithTtl {
                                entry: ledger_entry,
                                ttl: None,
                            }));
                        }
                    }
                    BucketEntry::Deadentry(dead_entry) => {
                        if dead_entry == *key {
                            eprintln!("returned entry none (archive)");
                            return Ok(None);
                        }
                    }
                    BucketEntry::Metaentry(_) => {}
                }
            }
        }

        // TODO: If the entry isn't found by here, and the entry is an entry that can be
        // evited to the hot archive (contract data that is persisted only, or contract
        // code), then get the hot archive buckets.
        eprintln!("returned entry none (not in archive)");
        Ok(None)
    }
}
