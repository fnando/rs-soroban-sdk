# soroban-ledger-snapshot-source-meta

A `SnapshotSource` implementation for the Soroban SDK that fetches ledger entries by downloading and searching ledger meta from multiple sources: SEP-54 meta storage, RPC, and history archives.

## Usage

Add the dependency:

```toml
[dependencies]
soroban-ledger-snapshot-source-meta = "23"
```

Use it in tests:

```rust
use bytes_lit::bytes;
use soroban_ledger_snapshot_source_meta::MetaSnapshotSource;
use soroban_sdk::Env;
use std::path::PathBuf;

let meta_url = "https://aws-public-blockchain.s3.us-east-2.amazonaws.com/v1.1/stellar/ledgers/pubnet";
let rpc_url = "https://mainnet.sorobanrpc.com";
let archive_url = "https://history.stellar.org/prd/core-live/core_live_001";
let tx_hash = bytes!(0x6fc2e483896276816b6d3b8d1df778bc978521f51561faa407ab8bb1949e6a1b);

let source = MetaSnapshotSource::new(
    meta_url.to_string(),
    rpc_url.to_string(),
    archive_url.to_string(),
    64,        // Checkpoint ledger count
    59914751,  // Ledger sequence
    Some(tx_hash),
    PathBuf::from("/tmp/cache"),
);

let env = Env::from_ledger_snapshot(source);
```
