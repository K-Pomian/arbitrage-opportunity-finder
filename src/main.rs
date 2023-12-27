use std::{
    str::FromStr,
    sync::Arc,
    thread::sleep,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use pyth_sdk_solana::Price;
use solana_client::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use structs::state::State;
use tokio::sync::RwLock;

mod structs;

const PYTH_RPC_URL: &str = "http:/pythnet.rpcpool.com";
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

    loop {
        sleep(Duration::from_millis(50));
        let pyth_price_read = *state.get_latest_pyth_price.read().await;
        println!("{:?}", pyth_price_read);
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
