#![no_std]
use soroban_sdk::contract;

#[contract]
pub struct Contract;

#[cfg(test)]
mod test {
    extern crate std;
    use bytes_lit::bytes;
    use soroban_ledger_snapshot_source_meta::MetaSnapshotSource;
    use soroban_sdk::{testutils::Ledger, token::TokenClient, Address, Env};

    fn test() {
        let ledger = 14751;
        let tx_hash = bytes!(0x6fc2e483896276816b6d3b8d1df778bc978521f51561faa407ab8bb1949e6a1b);
        let meta = MetaSnapshotSource::new_pubnet(ledger, Some(tx_hash));
        let e = Env::from_ledger_snapshot(meta);
        e.ledger().set_network_id(bytes_lit::bytes!(
            0x7ac33997544e3175d266bd022439b22cdb16508c01163f26e5cb2a3e1045a979
        ));
        let a = Address::from_str(&e, "CAS3FL6TLZKDGGSISDBWGGPXT3NRR4DYTZD7YOD3HMYO6LTJUVGRVEAM");
        let t = TokenClient::new(&e, &a);

        std::println!("test one");
        let a2 = Address::from_str(
            &e,
            "GAENIE5LBJIXLMJIAJ7225IUPA6CX7EGHUXRX5FLCZFFAQSG2ZUYSWFK",
        );
        let res = t.balance(&a2);
        std::println!("result {res}");
    }

    #[rustfmt::skip]
    mod tests {
        use super::test;
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
}
