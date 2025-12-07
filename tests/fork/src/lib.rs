#![no_std]
use soroban_sdk::contract;

#[contract]
pub struct Contract;

#[cfg(test)]
mod test {
    extern crate std;
    use bytes_lit::bytes;
    use soroban_ledger_snapshot_source_tx::MetaSnapshotSource;
    use soroban_sdk::{testutils::Ledger, token::TokenClient, Address, Env};

    fn test() {
        //cargo run -p soroban-ledger-meta-storage-cli -- --sequence 59914719 | jless
        //line number: 48773
        let ledger = 59914719;
        let tx_hash = bytes!(0x9ed4020a9fc5de9edc1acb484f3ccff7ef9cd02617efaadf24c13d0f60bbfdab);
        let meta = MetaSnapshotSource::new_pubnet(ledger, Some(tx_hash));
        let e = Env::from_ledger_snapshot(meta);
        // TODO: mainnet id, set in MetaSnapshotSource?
        e.ledger().set_network_id(bytes_lit::bytes!(
            0x7ac33997544e3175d266bd022439b22cdb16508c01163f26e5cb2a3e1045a979
        ));
        let contract = Address::from_str(
            &e,
            "CAESLMGW5LYTIEJI7FJHK6SFSWRELLNVX5Q4WR4UZEALMTRWQDBKDPAG",
        );
        let client = TokenClient::new(&e, &contract);

        std::println!("test one");
        let from = Address::from_str(
            &e,
            "GBTDRTG4H5YYD3NKV6V7YNSLNUWWSG7C4MTRPBC7IKDPJL4V7BDIY426",
        );
        let to = Address::from_str(
            &e,
            "GAZRGNL3X2ZMRVITJZUC5ZVSN6G23Y5OVZCMO6E7BTUJTPK4MYPHU5TV",
        );
        let res = client.balance(&from);
        std::println!("from {res}");
        let res = client.balance(&to);
        std::println!("to {res}");
        // "82339283"

        // Tx after:
        let ledger = 59914719;
        let tx_hash = bytes!(0x67885f9c05104cc29b7cb960d49b7b03ca3ddf9ca2bd45008cb0cf0b3307c6df);
        let meta = MetaSnapshotSource::new_pubnet(ledger, Some(tx_hash));
        let e = Env::from_ledger_snapshot(meta);
        // TODO: mainnet id, set in MetaSnapshotSource?
        e.ledger().set_network_id(bytes_lit::bytes!(
            0x7ac33997544e3175d266bd022439b22cdb16508c01163f26e5cb2a3e1045a979
        ));
        let contract = Address::from_str(
            &e,
            "CAESLMGW5LYTIEJI7FJHK6SFSWRELLNVX5Q4WR4UZEALMTRWQDBKDPAG",
        );
        let client = TokenClient::new(&e, &contract);

        std::println!("test two");
        let from = Address::from_str(
            &e,
            "GBTDRTG4H5YYD3NKV6V7YNSLNUWWSG7C4MTRPBC7IKDPJL4V7BDIY426",
        );
        let to = Address::from_str(
            &e,
            "GAZRGNL3X2ZMRVITJZUC5ZVSN6G23Y5OVZCMO6E7BTUJTPK4MYPHU5TV",
        );
        let res = client.balance(&from);
        std::println!("from {res}");
        let res = client.balance(&to);
        std::println!("to {res}");
    }

    #[rustfmt::skip]
    mod tests {
        use super::test;
        #[test] fn test_1() { test() }
        //#[test] fn test_2() { test() }
        //#[test] fn test_3() { test() }
        //#[test] fn test_4() { test() }
        //#[test] fn test_5() { test() }
        //#[test] fn test_6() { test() }
        //#[test] fn test_7() { test() }
        //#[test] fn test_8() { test() }
        //#[test] fn test_9() { test() }
        //#[test] fn test_10() { test() }
        //#[test] fn test_11() { test() }
        //#[test] fn test_12() { test() }
        //#[test] fn test_13() { test() }
        //#[test] fn test_14() { test() }
        //#[test] fn test_15() { test() }
        //#[test] fn test_16() { test() }
        //#[test] fn test_17() { test() }
        //#[test] fn test_18() { test() }
        //#[test] fn test_19() { test() }
        //#[test] fn test_20() { test() }
        //#[test] fn test_21() { test() }
        //#[test] fn test_22() { test() }
        //#[test] fn test_23() { test() }
        //#[test] fn test_24() { test() }
        //#[test] fn test_25() { test() }
        //#[test] fn test_26() { test() }
        //#[test] fn test_27() { test() }
        //#[test] fn test_28() { test() }
        //#[test] fn test_29() { test() }
        //#[test] fn test_30() { test() }
        //#[test] fn test_31() { test() }
        //#[test] fn test_32() { test() }
        //#[test] fn test_33() { test() }
        //#[test] fn test_34() { test() }
        //#[test] fn test_35() { test() }
        //#[test] fn test_36() { test() }
        //#[test] fn test_37() { test() }
        //#[test] fn test_38() { test() }
        //#[test] fn test_39() { test() }
        //#[test] fn test_40() { test() }
        //#[test] fn test_41() { test() }
        //#[test] fn test_42() { test() }
        //#[test] fn test_43() { test() }
        //#[test] fn test_44() { test() }
        //#[test] fn test_45() { test() }
        //#[test] fn test_46() { test() }
        //#[test] fn test_47() { test() }
        //#[test] fn test_48() { test() }
        //#[test] fn test_49() { test() }
        //#[test] fn test_50() { test() }
    }
}
