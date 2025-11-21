use clap::{Parser, ValueEnum};
use soroban_ledger_meta::ledger;
use std::process;
use stellar_xdr::curr::{Limited, Limits, WriteXdr};

/// CLI tool to download LedgerCloseMeta from Stellar public blockchain data
#[derive(Parser)]
#[command()]
struct Args {
    /// Ledger to download
    #[arg(short, long)]
    sequence: u32,

    /// Output format
    #[arg(short, long, value_enum, default_value_t)]
    format: Format,

    /// S3 bucket name
    #[arg(
        long,
        default_value = "https://aws-public-blockchain.s3.us-east-2.amazonaws.com/v1.1/stellar/ledgers/pubnet"
    )]
    meta_url: String,
}

#[derive(Default, Clone, ValueEnum)]
enum Format {
    #[default]
    Json,
    Xdr,
}

fn main() {
    let args = Args::parse();

    // Download the ledger meta
    match ledger(&args.meta_url, args.sequence) {
        Ok(meta) => {
            match args.format {
                Format::Json => match serde_json::to_string_pretty(&meta) {
                    Ok(json) => println!("{}", json),
                    Err(e) => {
                        eprintln!("Error: Failed to serialize to JSON: {}", e);
                        process::exit(1);
                    }
                },
                Format::Xdr => {
                    // Write raw XDR bytes
                    let limits = Limits::none(); // No limits for output
                    let mut stdout = std::io::stdout();
                    let mut limited_writer = Limited::new(&mut stdout, limits);
                    match meta.write_xdr(&mut limited_writer) {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("Error: Failed to write XDR: {}", e);
                            process::exit(1);
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Error: Failed to download ledger meta: {}", e);
            process::exit(1);
        }
    }
}
