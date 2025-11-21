#![no_std]
use soroban_sdk::contract;

#[contract]
pub struct Contract;

#[cfg(test)]
mod test {
    extern crate std;
    use soroban_sdk::{token::TokenClient, Address, Env};
    use soroban_ledger_snapshot_source_meta::{MetaSnapshotSource, S3Config};
    use std::string::ToString;
    use url::Url;

    #[test]
    fn test_rpc() {
        let s3_config = S3Config {
            bucket: "aws-public-blockchain".to_string(),
            region: "us-east-2".to_string(),
            root_path: "v1.1/stellar/ledgers/pubnet/".to_string(),
        };
        let archive_url =
            Url::parse("https://history.stellar.org/prd/core-live/core_live_001/").unwrap();
        let meta = MetaSnapshotSource::new(s3_config, archive_url, 59914763, None).unwrap();
        let e = Env::from_ledger_snapshot(meta);
        let a = Address::from_str(
            &e,
            "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
        );
        let t = TokenClient::new(&e, &a);

        std::println!("test one");
        let a2 = Address::from_str(
            &e,
            "GCZALA54U25HEBPGLRW6AXPWF25QW2LPZQCNUKNLFMCY25EJXAKZNIWZ",
        );
        let res = t.balance(&a2);
        std::println!("result {res}");

        //std::println!("waiting");
        //std::thread::sleep(std::time::Duration::from_secs(20));

        //std::println!("test two");
        //let a3 = Address::from_str(
        //    &e,
        //    "GAFUHA24GY66NPICT7KIP4QWZVCWQEL6OMR5QSY6JNDTCBMX72LHPKKQ",
        //);
        //let res2 = t.balance(&a3);
        //std::println!("result {res2}");
    }
}
