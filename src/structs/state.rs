use std::{str::FromStr, sync::Arc};

use pyth_sdk_solana::Price;
use rust_decimal::Decimal;
use solana_program::pubkey::Pubkey;
use tokio::sync::{OnceCell, RwLock};

use crate::config::{Config, CONFIG};

use super::{
    cex::binance::{Binance, BookTickerData},
    on_chain::pyth::Pyth,
};

pub static STATE: OnceCell<State> = OnceCell::const_new();

/*
    Struct managing runtime state of the application
*/
pub struct State {
    pyth: Pyth,
    binance: Binance,
    pyth_price_id: Pubkey,
    latest_pyth_price: Arc<RwLock<Option<Price>>>,
    latest_binance_ticker_data: Arc<RwLock<Option<BookTickerData>>>,
    pub binance_taker_fee: Decimal,
    pub subscription_id: i64,
}

impl State {
    /*
        Establishes connections and creates instance of State
    */
    pub async fn new() -> Self {
        let config = CONFIG.get_or_init(|| async { Config::new() }).await;
        let (binance, _) = Binance::connect()
            .await
            .expect("Could not connect to Binance WS");
        let subscription_id = binance
            .subscribe_to_ticker(&config.binance_ticker)
            .await
            .unwrap();

        Self {
            pyth: Pyth::new(),
            binance,
            pyth_price_id: Pubkey::from_str(&config.pyth_price_id).unwrap(),
            latest_pyth_price: Arc::new(RwLock::new(None)),
            latest_binance_ticker_data: Arc::new(RwLock::new(None)),
            binance_taker_fee: if config.binance_ticker.contains("bnb") {
                Decimal::new(75, 5)
            } else {
                Decimal::new(1, 3)
            },
            subscription_id,
        }
    }

    /*
        Returns a pointer to latest_pyth_price field
    */
    pub fn get_latest_pyth_price(&self) -> Arc<RwLock<Option<Price>>> {
        self.latest_pyth_price.clone()
    }

    /*
        Returns a pointer to latest_binance_ticker_data field
    */
    pub fn get_latest_binance_ticker_data(&self) -> Arc<RwLock<Option<BookTickerData>>> {
        self.latest_binance_ticker_data.clone()
    }

    /*
        Acquires write lock and updates value of latest_pyth_price field
    */
    pub async fn update_latest_pyth_price(&self) {
        let maybe_price = self
            .pyth
            .get_price(&self.pyth_price_id)
            .expect("Could not load price feed from account");
        *self.latest_pyth_price.write().await = maybe_price;
    }

    /*
        Acquires write lock and updates value of latest_binance_ticker_data field
    */
    pub async fn update_latest_binance_ticker_data(&self) {
        if let Some(binance_response) = self.binance.read_next_message().await {
            *self.latest_binance_ticker_data.write().await = Some(binance_response.data);
        }
    }

    /*
        Unsubscribes from the Binance WS ticker stream
    */
    pub async fn terminate(&self) {
        let config = CONFIG.get().unwrap();
        self.binance
            .unsubscribe(&config.binance_ticker, self.subscription_id)
            .await
            .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use super::State;
    use crate::config::{Config, CONFIG};

    #[tokio::test]
    #[ignore = "Uses global static, hence has to be ran manually"]
    async fn test_new_bnb_pair() {
        CONFIG
            .get_or_init(|| async {
                Config {
                    binance_ticker: "bnbusdt".to_string(),
                    pyth_price_id: "4CkQJBxhU8EZ2UjhigbtdaPbpTe6mqf811fipYBFbSYN".to_string(),
                }
            })
            .await;
        let state = State::new().await;
        assert_eq!(state.binance_taker_fee, Decimal::new(75, 5));
    }

    #[tokio::test]
    #[ignore = "Uses global static, hence has to be ran manually"]
    async fn test_new_not_bnb_pair() {
        CONFIG
            .get_or_init(|| async {
                Config {
                    binance_ticker: "solusdt".to_string(),
                    pyth_price_id: "H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG".to_string(),
                }
            })
            .await;
        let state = State::new().await;
        assert_eq!(state.binance_taker_fee, Decimal::new(1, 3));
    }
}
