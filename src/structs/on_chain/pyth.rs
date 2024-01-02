use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use pyth_sdk_solana::Price;
use solana_client::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;

const PYTH_RPC_URL: &str = "http:/pythnet.rpcpool.com";

/*
    Struct representing a Pyth connection
*/
pub struct Pyth {
    client: RpcClient,
}

impl Pyth {
    pub fn new() -> Self {
        Self {
            client: RpcClient::new(PYTH_RPC_URL),
        }
    }

    /*
        Fetches the most current price from Pyth
    */
    pub fn get_price(&self, price_id: &Pubkey) -> Result<Option<Price>> {
        let mut price_account = self.client.get_account(price_id)?;
        let price_feed =
            pyth_sdk_solana::load_price_feed_from_account(price_id, &mut price_account)?;

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Ok(price_feed.get_price_no_older_than(current_time, 60))
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use solana_program::pubkey::Pubkey;

    use super::Pyth;

    #[test]
    fn test_get_price_account_does_not_exist() {
        let pyth = Pyth::new();
        let invalid_pubkey = Pubkey::from([0; 32]);
        let result = pyth.get_price(&invalid_pubkey);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_price_not_price_account() {
        let pyth = Pyth::new();
        let not_price_account_pubkey =
            Pubkey::from_str("8pwb2jNPKvji1P76fib494WkZKH7RFPgMmGkS6a3kxp9").unwrap();
        let result = pyth.get_price(&not_price_account_pubkey);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_price() {
        let pyth = Pyth::new();
        let sol_usd_price_pubkey =
            Pubkey::from_str("H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG").unwrap();
        let result = pyth.get_price(&sol_usd_price_pubkey);
        assert!(result.is_ok());
    }
}
