use cargo_metadata::MetadataCommand;
use directories::ProjectDirs;
use sha2::{Digest, Sha256};
use soroban_ledger_fetch::{cache, LedgerEntryFetcher, LedgerEntryWithTtl, Network};
use soroban_sdk::testutils::SnapshotSourceInput;
use soroban_sdk::testutils::{HostError, SnapshotSource};
use soroban_sdk::xdr::{LedgerEntry, LedgerKey, Limits, WriteXdr};
use std::path::PathBuf;
use std::rc::Rc;

/// Snapshot source that downloads ledger meta and searches for ledger entries
/// based on a specific transaction context.
pub struct TxSnapshotSource {
    fetcher: LedgerEntryFetcher,
    cache_path: PathBuf,
}

impl TxSnapshotSource {
    /// Create a new TxSnapshotSource
    ///
    /// The cache path is automatically computed as `<workspace_root>/tests-snapshot-source/<network_id_hex>`
    /// using cargo metadata to find the workspace root.
    ///
    /// # Arguments
    /// * `network` - Network configuration with URLs for meta storage, RPC, and history archive
    /// * `ledger` - Ledger sequence number
    /// * `tx_hash` - Optional transaction hash
    ///
    /// # Panics
    /// Panics if the workspace root cannot be determined via cargo metadata.
    pub fn new(network: Network, ledger: u32, tx_hash: Option<[u8; 32]>) -> Self {
        let workspace_root: PathBuf = MetadataCommand::new()
            .exec()
            .expect("failed to get cargo metadata")
            .workspace_root
            .into();
        let cache_path = workspace_root
            .join("tests-snapshot-source")
            .join(network.network_id_hex());
        let fetcher_cache_path = ProjectDirs::from("org", "stellar", "soroban-sdk")
            .expect("failed to get project directories")
            .cache_dir()
            .join("ledger-fetch");
        Self {
            fetcher: LedgerEntryFetcher::new(network, ledger, tx_hash, fetcher_cache_path),
            cache_path,
        }
    }

    /// Fetch a ledger entry, using workspace-level caching
    fn fetch(&self, key: &LedgerKey) -> Option<LedgerEntryWithTtl> {
        // Compute cache file path: <cache_path>/<ledger>/<hash_of_key>.json
        let key_xdr = key.to_xdr(Limits::none()).expect("failed to encode key");
        let key_hash = Sha256::digest(&key_xdr);
        let ledger_cache_dir = self.cache_path.join(format!("{}", self.fetcher.ledger()));

        // Ensure cache directory exists
        std::fs::create_dir_all(&ledger_cache_dir).expect("failed to create cache directory");

        // Use cache function to handle reading/writing cache file
        let fetch_read = cache(
            ledger_cache_dir.join(format!("{:x}.json", key_hash)),
            |write| -> Result<(), Box<dyn std::error::Error>> {
                // Fetch the data from the underlying fetcher
                let result = self.fetcher.fetch(key)?;

                // Serialize to JSON
                serde_json::to_writer_pretty(write, &result)?;

                Ok(())
            },
        )
        .expect("failed to cache entry");

        // Parse the cached result
        serde_json::from_reader(fetch_read).expect("failed to parse cached entry")
    }
}

impl From<TxSnapshotSource> for SnapshotSourceInput {
    fn from(source: TxSnapshotSource) -> Self {
        Self {
            source: Rc::new(source),
            ledger_info: None,
            snapshot: None,
        }
    }
}

impl SnapshotSource for TxSnapshotSource {
    fn get(
        &self,
        key: &Rc<LedgerKey>,
    ) -> Result<Option<(Rc<LedgerEntry>, Option<u32>)>, HostError> {
        Ok(self.fetch(key).map(|e| (Rc::new(e.entry), e.ttl)))
    }
}
