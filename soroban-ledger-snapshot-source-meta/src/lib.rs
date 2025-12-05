use directories::ProjectDirs;
use sha2::{Digest, Sha256};
use soroban_ledger_history_archive::{get_bucket, get_history, parse_bucket, parse_history};
use soroban_ledger_meta_storage::{get_ledger, parse_ledger};
use soroban_ledger_rpc::{get_ledger_entry, parse_ledger_entry};
use soroban_sdk::testutils::SnapshotSourceInput;
use soroban_sdk::testutils::{HostError, SnapshotSource};
use soroban_sdk::xdr::{BucketEntry, LedgerEntry, LedgerKey, Limited, Limits, WriteXdr};
use std::path::PathBuf;
use std::rc::Rc;

mod cache;
use cache::cache;

mod iter;
use iter::LedgerEntryChangesIterator;

/// A ledger entry with its optional TTL (time-to-live)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LedgerEntryWithTtl {
    pub entry: LedgerEntry,
    pub ttl: Option<u32>,
}

/// Error type for MetaSnapshotSource operations
#[derive(Debug, thiserror::Error)]
pub enum MetaSnapshotError {
    #[error("cache error: {0}")]
    Cache(#[from] cache::CacheError<Box<dyn std::error::Error>>),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("xdr error: {0}")]
    Xdr(#[from] soroban_sdk::xdr::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("meta storage error: {0}")]
    MetaStorage(#[from] soroban_ledger_meta_storage::Error),
    #[error("rpc error: {0}")]
    Rpc(#[from] soroban_ledger_rpc::Error),
    #[error("history archive error: {0}")]
    HistoryArchive(#[from] soroban_ledger_history_archive::Error),
    #[error("home directory not found")]
    HomeDirectoryNotFound,
}

/// Meta snapshot source that downloads ledger meta and searches for ledger entries
pub struct MetaSnapshotSource {
    meta_url: String,
    rpc_url: String,
    archive_url: String,
    archive_checkpoint_ledger_count: u32,
    ledger: u32,
    tx_hash: Option<[u8; 32]>,
    cache_path: PathBuf,
}

impl MetaSnapshotSource {
    /// Create a new MetaSnapshotSource
    ///
    /// # Arguments
    /// * `meta_url`- URL to the SEP-54 ledger meta storage
    /// * `rpc_url` - URL to the RPC
    /// * `archive_url` - URL to the History Archive storage
    /// * `archive_checkpoint_ledger_count` - Number of ledgers between checkpoints
    /// * `ledger` - Ledger sequence number
    /// * `tx_hash` - Optional transaction hash
    /// * `cache_path` - Path to store cache files
    pub fn new(
        meta_url: String,
        rpc_url: String,
        archive_url: String,
        archive_checkpoint_ledger_count: u32,
        ledger: u32,
        tx_hash: Option<[u8; 32]>,
        cache_path: PathBuf,
    ) -> Self {
        Self {
            meta_url,
            rpc_url,
            archive_url,
            archive_checkpoint_ledger_count,
            ledger,
            tx_hash,
            cache_path,
        }
    }

    /// Create a new MetaSnapshotSource for Stellar pubnet with default URLs
    ///
    /// Uses default mainnet URLs:
    /// - SEP-54 meta storage: AWS public blockchain
    /// - RPC: mainnet.sorobanrpc.com
    /// - History archive: history.stellar.org
    ///
    /// # Arguments
    /// * `ledger` - Ledger sequence number
    /// * `tx_hash` - Optional transaction hash
    /// * `cache_path` - Path to store cache files
    pub fn new_pubnet(ledger: u32, tx_hash: Option<[u8; 32]>, cache_path: PathBuf) -> Self {
        Self::new(
            "https://aws-public-blockchain.s3.us-east-2.amazonaws.com/v1.1/stellar/ledgers/pubnet"
                .to_string(),
            "https://mainnet.sorobanrpc.com".to_string(),
            "https://history.stellar.org/prd/core-live/core_live_001".to_string(),
            64,
            ledger,
            tx_hash,
            cache_path,
        )
    }

    /// Create a new MetaSnapshotSource for Stellar testnet with default URLs
    ///
    /// Uses default testnet URLs:
    /// - SEP-54 meta storage: AWS public blockchain
    /// - RPC: soroban-testnet.stellar.org
    /// - History archive: history.stellar.org
    ///
    /// # Arguments
    /// * `ledger` - Ledger sequence number
    /// * `tx_hash` - Optional transaction hash
    /// * `cache_path` - Path to store cache files
    pub fn new_testnet(ledger: u32, tx_hash: Option<[u8; 32]>, cache_path: PathBuf) -> Self {
        Self::new(
            "https://aws-public-blockchain.s3.us-east-2.amazonaws.com/v1.1/stellar/ledgers/testnet/2025-08-14"
                .to_string(),
            "https://soroban-testnet.stellar.org".to_string(),
            "https://history.stellar.org/prd/core-testnet/core_testnet_001".to_string(),
            64,
            ledger,
            tx_hash,
            cache_path,
        )
    }

    fn fetch(&self, key: &LedgerKey) -> Result<Option<LedgerEntryWithTtl>, MetaSnapshotError> {
        // TODO: Consider replacing this caching into a directory with caching into a
        // LedgerSnapshot file. It would be more compatible with existing functionality.

        // Compute cache file path: <cache_path>/<ledger>/<hash_of_key>.json
        let key_xdr = key.to_xdr(Limits::none())?;
        let key_hash = Sha256::digest(&key_xdr);
        let ledger_cache_dir = self.cache_path.join(format!("{}", self.ledger));

        // Ensure cache directory exists
        std::fs::create_dir_all(&ledger_cache_dir)?;

        // Use cache function to handle reading/writing cache file
        let fetch_read = cache(
            ledger_cache_dir.join(format!("{:x}.json", key_hash)),
            |write| {
                // Fetch the data
                let result = self.fetch_internal(key)?;

                // Serialize to JSON
                serde_json::to_writer_pretty(write, &result)?;

                Ok(())
            },
        )?;

        // Parse the cached result
        Ok(serde_json::from_reader(fetch_read)?)
    }

    fn fetch_internal(
        &self,
        key: &LedgerKey,
    ) -> Result<Option<LedgerEntryWithTtl>, MetaSnapshotError> {
        eprintln!("looking up key {}", serde_json::to_string(key)?);

        let cache_path = ProjectDirs::from("org", "stellar", "soroban-sdk")
            .ok_or(MetaSnapshotError::HomeDirectoryNotFound)?
            .cache_dir()
            .join("meta-snapshot-source");
        std::fs::create_dir_all(&cache_path)?;

        let mut ledger = self.ledger;
        loop {
            eprintln!("loading ledger {ledger}");

            // Search for the ledger entry in this ledger's meta
            if let Some(result) = self.fetch_from_meta(&cache_path, ledger, key)? {
                return Ok(result);
            }

            // Not found in meta, try RPC on first loop
            if ledger == self.ledger {
                if let Some(result) = self.fetch_from_rpc(&cache_path, ledger, key)? {
                    return Ok(result);
                }
            }

            // Not found in meta, if it's a subsequent ledger that is a checkpoint ledger, try history archive
            if ledger != self.ledger && (ledger + 1) % self.archive_checkpoint_ledger_count == 0 {
                return self.fetch_from_archive(&cache_path, ledger, key);
            }

            // Not found in this ledger, try previous ledger
            if ledger == 3 {
                return Ok(None);
            }
            ledger -= 1;
        }
    }

    fn fetch_from_meta(
        &self,
        cache_path: &PathBuf,
        ledger: u32,
        key: &LedgerKey,
    ) -> Result<Option<Option<LedgerEntryWithTtl>>, MetaSnapshotError> {
        let meta_read = cache(cache_path.join(format!("ledger-{ledger}.xdr")), |write| {
            get_ledger(&self.meta_url, ledger, write)
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
        for (phase, tx_hash, change_key, change_entry) in changes {
            let tx_hash_short = tx_hash
                .iter()
                .take(7)
                .map(|b| format!("{:02x}", b))
                .collect::<String>();
            eprintln!("current phase: {:?}, tx_hash: {}", phase, tx_hash_short);
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
    ) -> Result<Option<Option<LedgerEntryWithTtl>>, MetaSnapshotError> {
        eprintln!("checking rpc");
        let key_xdr = key.to_xdr(Limits::none())?;
        let key_hash = Sha256::digest(&key_xdr);
        let rpc_read = cache(
            cache_path.join(format!("rpc-{ledger}-{key_hash:x}.json")),
            |write| {
                get_ledger_entry(&self.rpc_url, key, write)
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
    ) -> Result<Option<LedgerEntryWithTtl>, MetaSnapshotError> {
        eprintln!("loading ledger {ledger} from checkpoint");
        let history_read = cache(
            cache_path.join(format!("history-{}.json", ledger)),
            |write| {
                get_history(&self.archive_url, ledger, write)
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
            eprintln!("loading {bucket}");
            let bucket_read = cache(cache_path.join(format!("bucket-{bucket}.xdr")), |write| {
                get_bucket(&self.archive_url, bucket, write)
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
            })?;
            let mut limited_reader = Limited::new(bucket_read, Limits::none());
            let bucket_entries_iter = parse_bucket(&mut limited_reader);
            eprintln!("searching {bucket}");
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

impl From<MetaSnapshotSource> for SnapshotSourceInput {
    fn from(source: MetaSnapshotSource) -> Self {
        Self {
            source: Rc::new(source),
            ledger_info: None,
            snapshot: None,
        }
    }
}

impl SnapshotSource for MetaSnapshotSource {
    fn get(
        &self,
        key: &Rc<LedgerKey>,
    ) -> Result<Option<(Rc<LedgerEntry>, Option<u32>)>, HostError> {
        Ok(self.fetch(key).unwrap().map(|e| (Rc::new(e.entry), e.ttl)))
    }
}
