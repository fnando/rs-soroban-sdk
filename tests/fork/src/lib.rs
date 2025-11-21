#![no_std]
use soroban_sdk::contract;

#[contract]
pub struct Contract;

#[cfg(test)]
mod test {
    extern crate std;
    use bytes_lit::bytes;
    use soroban_ledger_snapshot_source_meta::MetaSnapshotSource;
    use soroban_sdk::{token::TokenClient, Address, Env};
    use std::path::PathBuf;
    use std::string::ToString;

    fn test() {
        let meta_url =
            "https://aws-public-blockchain.s3.us-east-2.amazonaws.com/v1.1/stellar/ledgers/pubnet"
                .to_string();
        let rpc_url = "https://mainnet.sorobanrpc.com".to_string();
        let archive_url = "https://history.stellar.org/prd/core-live/core_live_001".to_string();
        let archive_checkpoint_ledger_count = 64;
        let ledger = 59914751;
        // "6fc2e483896276816b6d3b8d1df778bc978521f51561faa407ab8bb1949e6a1b" (tx 96 in 59914751) expect balance 945997587
        // "f63788bd8d16888d248f2bc31c95a186854b71b2f8c03489381cd845fc577f3d" (tx 97 in 59914751) expect balance 945997387
        // "2198582798cc112941ec1e6b7a53ea590bf176be33b744076db2be1f722cf83d" (tx 98 in ...)
        let tx_hash = bytes!(0x6fc2e483896276816b6d3b8d1df778bc978521f51561faa407ab8bb1949e6a1b);
        let cache_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("test_snapshot_source_cache")
            .join("pubnet");
        let meta = MetaSnapshotSource::new(
            meta_url,
            rpc_url,
            archive_url,
            archive_checkpoint_ledger_count,
            ledger,
            Some(tx_hash),
            cache_path,
        );
        let e = Env::from_ledger_snapshot(meta);
        let a = Address::from_str(
            &e,
            "CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA",
        );
        let t = TokenClient::new(&e, &a);

        std::println!("test one");
        let a2 = Address::from_str(
            &e,
            "GCZALA54U25HEBPGLRW6AXPWF25QW2LPZQCNUKNLFMCY25EJXAKZNIWZ",
        );
        let res = t.balance(&a2);
        std::println!("result {res}");
    }

    #[test] fn test_1() { test() }
    #[test] fn test_2() { test() }
    #[test] fn test_3() { test() }
    #[test] fn test_4() { test() }
    #[test] fn test_5() { test() }
    #[test] fn test_6() { test() }
    #[test] fn test_7() { test() }
    #[test] fn test_8() { test() }
    #[test] fn test_9() { test() }
    #[test] fn test_10() { test() }
    #[test] fn test_11() { test() }
    #[test] fn test_12() { test() }
    #[test] fn test_13() { test() }
    #[test] fn test_14() { test() }
    #[test] fn test_15() { test() }
    #[test] fn test_16() { test() }
    #[test] fn test_17() { test() }
    #[test] fn test_18() { test() }
    #[test] fn test_19() { test() }
    #[test] fn test_20() { test() }
    #[test] fn test_21() { test() }
    #[test] fn test_22() { test() }
    #[test] fn test_23() { test() }
    #[test] fn test_24() { test() }
    #[test] fn test_25() { test() }
    #[test] fn test_26() { test() }
    #[test] fn test_27() { test() }
    #[test] fn test_28() { test() }
    #[test] fn test_29() { test() }
    #[test] fn test_30() { test() }
    #[test] fn test_31() { test() }
    #[test] fn test_32() { test() }
    #[test] fn test_33() { test() }
    #[test] fn test_34() { test() }
    #[test] fn test_35() { test() }
    #[test] fn test_36() { test() }
    #[test] fn test_37() { test() }
    #[test] fn test_38() { test() }
    #[test] fn test_39() { test() }
    #[test] fn test_40() { test() }
    #[test] fn test_41() { test() }
    #[test] fn test_42() { test() }
    #[test] fn test_43() { test() }
    #[test] fn test_44() { test() }
    #[test] fn test_45() { test() }
    #[test] fn test_46() { test() }
    #[test] fn test_47() { test() }
    #[test] fn test_48() { test() }
    #[test] fn test_49() { test() }
    #[test] fn test_50() { test() }
}
