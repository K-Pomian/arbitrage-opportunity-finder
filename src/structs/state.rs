use std::{str::FromStr, sync::Arc};

use pyth_sdk_solana::Price;
use solana_program::pubkey::Pubkey;
use tokio::sync::{OnceCell, RwLock};

use crate::config::{Config, CONFIG};

use super::{
    cex::binance::{Binance, BookTickerData},
    on_chain::pyth::Pyth,
};

pub static STATE: OnceCell<State> = OnceCell::const_new();

pub struct State {
    pyth: Pyth,
    binance: Binance,
    pyth_price_id: Pubkey,
    latest_pyth_price: Arc<RwLock<Option<Price>>>,
    latest_binance_ticker_data: Arc<RwLock<Option<BookTickerData>>>,
}

impl State {
    pub async fn new() -> Self {
        let config = CONFIG.get_or_init(|| async { Config::new() }).await;
        let (mut binance, _) = Binance::connect()
            .await
            .expect("Could not connect to Binance WS");
        binance.subscribe_to_ticker(&config.binance_ticker).await;

        Self {
            pyth: Pyth::new(),
            binance,
            pyth_price_id: Pubkey::from_str(&config.pyth_price_id).unwrap(),
            latest_pyth_price: Arc::new(RwLock::new(None)),
            latest_binance_ticker_data: Arc::new(RwLock::new(None)),
        }
    }

    pub fn get_latest_pyth_price(&self) -> Arc<RwLock<Option<Price>>> {
        self.latest_pyth_price.clone()
    }

    pub fn get_latest_binance_ticker_data(&self) -> Arc<RwLock<Option<BookTickerData>>> {
        self.latest_binance_ticker_data.clone()
    }

    pub async fn update_latest_pyth_price(&self) {
        let maybe_price = self
            .pyth
            .get_price(&self.pyth_price_id)
            .expect("Could not load price feed from account");
        *self.latest_pyth_price.write().await = maybe_price;
    }

    pub async fn update_latest_binance_price(&self) {
        if let Some(binance_response) = self.binance.read_next_message().await {
            *self.latest_binance_ticker_data.write().await = Some(binance_response.data);
        }
    }
}
