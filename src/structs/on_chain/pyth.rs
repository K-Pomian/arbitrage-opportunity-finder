use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use pyth_sdk_solana::Price;
use solana_client::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;

const PYTH_RPC_URL: &str = "http:/pythnet.rpcpool.com";

pub struct Pyth {
    client: RpcClient,
}

impl Pyth {
    pub fn new() -> Self {
        Self {
            client: RpcClient::new(PYTH_RPC_URL),
        }
    }

    pub fn get_price(&self, price_id: &Pubkey) -> Result<Option<Price>> {
        let mut sol_usd_price_account = self.client.get_account(price_id)?;
        let sol_price_feed =
            pyth_sdk_solana::load_price_feed_from_account(price_id, &mut sol_usd_price_account)?;

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Ok(sol_price_feed.get_price_no_older_than(current_time, 60))
    }
}
