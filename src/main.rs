use std::{
    str::FromStr,
    sync::Arc,
    thread::sleep,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use pyth_sdk_solana::Price;
use solana_program::pubkey::Pubkey;
use structs::{
    cex::binance::{Binance, BookTickerData},
    on_chain::pyth::Pyth,
    state::State,
};
use tokio::sync::RwLock;

mod structs;

const PYTH_SOL_USD_PRICE_ID: &str = "H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG";

#[tokio::main]
async fn main() {
    let state = State::new();

    tokio::spawn({
        let latest_pyth_price = state.get_latest_pyth_price();

        async move {
            update_last_pyth_price(latest_pyth_price).await;
        }
    });

    tokio::spawn({
        let latest_cex_ticker_data = state.get_latest_cex_ticker_data();

        async move {
            update_last_cex_price(latest_cex_ticker_data).await;
        }
    });

    loop {}
}

async fn update_last_pyth_price(latest_pyth_price: Arc<RwLock<Option<Price>>>) {
    let pyth = Pyth::new();
    let sol_usd_price_key =
        Pubkey::from_str(PYTH_SOL_USD_PRICE_ID).expect("Could not parse pubkey");

    loop {
        let maybe_price = pyth
            .get_price(&sol_usd_price_key)
            .expect("Could not load price feed from account");
        let mut latest_pyth_price_write = latest_pyth_price.write().await;
        *latest_pyth_price_write = maybe_price;
    }
}

async fn update_last_cex_price(latest_cex_price: Arc<RwLock<Option<BookTickerData>>>) {
    let (mut binance, _) = Binance::connect()
        .await
        .expect("Could not connect to Binance WS");
    binance
        .subscribe_to_ticker("solusdt")
        .await
        .expect("Could not subscribe to the ticker");

    while let Some(binance_response) = binance.read_next_message().await {
        println!("{:?}", binance_response);

        *latest_cex_price.write().await = Some(binance_response.data);
    }
}
