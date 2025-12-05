# Code Review: rs-soroban-sdk-snapshot-source-meta branch

Review of changes between `rs-soroban-sdk-snapshot-source` and `rs-soroban-sdk-snapshot-source-meta` branches.

## Summary

This branch adds several new crates for fetching ledger data from multiple sources (SEP-54 meta storage, RPC, and history archives) to enable forking/replaying ledger state in tests. The architecture cleanly separates concerns into distinct crates.

## New Crates

| Crate | Purpose |
|-------|---------|
| `soroban-ledger-meta-storage` | Download ledger meta from SEP-54 storage |
| `soroban-ledger-meta-storage-cli` | CLI tool for downloading ledger meta |
| `soroban-ledger-rpc` | Fetch ledger entries from Stellar RPC |
| `soroban-ledger-history-archive` | Access history archives for bucket data |
| `soroban-ledger-snapshot-source-rpc` | `SnapshotSource` impl using RPC |
| `soroban-ledger-snapshot-source-meta` | `SnapshotSource` impl using meta + RPC + archives |

---

## Issues

### Critical

#### 2. ~~RPC snapshot source doesn't handle `None` case~~ - ADDRESSED

### Medium

#### 7. Debug `eprintln!` statements throughout production code

Multiple files contain `eprintln!` for logging:
- `soroban-ledger-snapshot-source-meta/src/lib.rs:122,132,184,187,204,215-218,224,236,251,258,264,285`
- `soroban-ledger-history-archive/src/lib.rs:50`
- `soroban-ledger-snapshot-source-rpc/src/lib.rs:31,42`

Consider using a proper logging framework (e.g., `tracing` or `log`) or removing debug output for production.

#### 8. ~~Potential panic in iterator on empty ledger~~ - ADDRESSED

#### 9. TODO comment indicates incomplete implementation (`soroban-ledger-snapshot-source-meta/src/lib.rs:282-284`)

```rust
// TODO: If the entry isn't found by here, and the entry is an entry that can be
// evited to the hot archive (contract data that is persisted only, or contract
// code), then get the hot archive buckets.
```

Hot archive lookup is not implemented.

#### 10. TODO comment for unclear semantics (`soroban-ledger-snapshot-source-meta/src/iter.rs:380`)

```rust
TransactionMeta::V1(m) => Some(&m.tx_changes), // TODO: Should this be before or after, or just ignored?
```

The handling of V1 transaction meta changes is uncertain.

### Low

#### 13. Missing documentation

Most public functions and types lack documentation. Key areas:
- `LedgerEntryChangesIterator` behavior with `tx_hash` parameter
- `MetaSnapshotSource` constructor parameters
- Error types and their meanings

#### 14. ~~Hardcoded constants~~ - ADDRESSED

#### 15. ~~`.lock` files left behind in cache directory~~ - WONTFIX

#### 16. Test file duplication (`tests/fork/test_snapshots/`)

There are 102 nearly identical snapshot JSON files (50 in `test/` and 50 in `test/tests/`). Consider whether all are necessary or if they can be deduplicated.

---

## Recommendations

1. **Fix the critical CLI import bug** - the CLI won't compile as-is.

2. ~~**Fix the RPC snapshot source pattern match** - add handling for `Ok(None)`.~~ - ADDRESSED

3. **Standardize `thiserror` version** across all crates.

4. **Add `default-features = false`** to all `reqwest` dependencies for consistency.

5. **Remove unused `sha2`** from history-archive crate.

6. **Replace `eprintln!`** with a logging framework or make it optional via a feature flag.

7. **Add unit tests** for the new crates - currently only integration tests exist.

8. **Document public APIs** - especially the iterator behavior and snapshot source configuration.

9. **Clean up `.lock` files** after successful cache operations.

10. **Address TODOs** before merging, or convert to tracked issues.

---

## Positive Observations

- Clean separation of concerns across crates
- Good use of the `cache` abstraction for download caching with atomic writes
- Comprehensive handling of all `LedgerCloseMeta` and `TransactionMeta` versions
- Proper use of file locking for concurrent access safety
- The iterator design for traversing ledger changes is well thought out
