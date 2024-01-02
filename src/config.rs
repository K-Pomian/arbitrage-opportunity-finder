use clap::Parser;
use tokio::sync::OnceCell;

pub static CONFIG: OnceCell<Config> = OnceCell::const_new();

#[derive(Parser)]
pub struct Config {
    // Pair from Binance spot market
    #[arg(long, short, default_value = "solusdt")]
    pub binance_ticker: String,

    // Price id pubkey from Pyth
    // List of available ids (Solana) can be found here:
    // https://pyth.network/price-feeds?cluster=solana-mainnet-beta
    #[arg(
        long,
        short,
        default_value = "H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG"
    )]
    pub pyth_price_id: String,
}

impl Config {
    pub fn new() -> Self {
        Self::parse()
    }
}
