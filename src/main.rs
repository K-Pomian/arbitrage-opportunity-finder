use std::{
    str::FromStr,
    sync::Arc,
    thread::sleep,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use futures_util::{SinkExt, StreamExt};
use pyth_sdk_solana::Price;
use serde::Deserialize;
use solana_client::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use structs::state::State;
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::Message;

mod structs;

const PYTH_RPC_URL: &str = "http:/pythnet.rpcpool.com";
const PYTH_SOL_USD_PRICE_ID: &str = "H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG";

const BINANCE_WEBSOCKET_URL: &str = "wss://stream.binance.com:9443/stream";

#[derive(Debug, Deserialize)]
pub struct BinanceResponse {
    pub stream: String,
    pub data: BookTickerData,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct BookTickerData {
    pub u: u64,    // order book updateId
    pub s: String, // symbol
    pub b: String, // best bid price
    pub B: String, // best bid quantity
    pub a: String, // best ask price
    pub A: String, // best ask quantity
}

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

    loop {
        // sleep(Duration::from_millis(50));
        // let pyth_price_read = *state.get_latest_pyth_price().read().await;
        // println!("{:?}", pyth_price_read);
    }
}

async fn update_last_pyth_price(latest_pyth_price: Arc<RwLock<Option<Price>>>) {
    let solana_rpc_client = RpcClient::new(PYTH_RPC_URL);
    let sol_usd_price_key =
        Pubkey::from_str(PYTH_SOL_USD_PRICE_ID).expect("Could not parse pubkey");

    loop {
        let mut sol_usd_price_account = solana_rpc_client.get_account(&sol_usd_price_key).unwrap();
        let sol_price_feed = pyth_sdk_solana::load_price_feed_from_account(
            &sol_usd_price_key,
            &mut sol_usd_price_account,
        )
        .unwrap();

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let maybe_price = sol_price_feed.get_price_no_older_than(current_time, 60);
        let mut latest_pyth_price_write = latest_pyth_price.write().await;
        *latest_pyth_price_write = maybe_price;
    }
}

async fn update_last_cex_price(latest_cex_price: Arc<RwLock<Option<BookTickerData>>>) {
    let (socket, _) = tokio_tungstenite::connect_async(BINANCE_WEBSOCKET_URL)
        .await
        .expect("Could not connect");
    let (mut write, mut read) = socket.split();

    let current_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let subscribe_request = format!(
        "{{\"method\":\"SUBSCRIBE\",\"params\":[\"btcusdt@bookTicker\"],\"id\":{}}}",
        current_timestamp
    );
    println!("{}", subscribe_request);
    let message = Message::Text(subscribe_request);

    write.send(message).await.unwrap();
    read.next().await; // The first message is a response to the subscribe request

    while let Some(inner) = read.next().await {
        let message = inner.unwrap().into_data();
        let message_str = String::from_utf8(message).unwrap();

        let binance_response =
            serde_json::from_str::<BinanceResponse>(message_str.as_str()).unwrap();
        println!("{:?}", binance_response);

        *latest_cex_price.write().await = Some(binance_response.data);
    }
}
