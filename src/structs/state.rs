use std::sync::Arc;

use pyth_sdk_solana::Price;
use tokio::sync::RwLock;

use crate::BookTickerData;

pub struct State {
    latest_pyth_price: Arc<RwLock<Option<Price>>>,
    latest_cex_ticker_data: Arc<RwLock<Option<BookTickerData>>>,
}

impl State {
    pub fn new() -> Self {
        Self {
            latest_pyth_price: Arc::new(RwLock::new(None)),
            latest_cex_ticker_data: Arc::new(RwLock::new(None)),
        }
    }

    pub fn get_latest_pyth_price(&self) -> Arc<RwLock<Option<Price>>> {
        self.latest_pyth_price.clone()
    }

    pub fn get_latest_cex_ticker_data(&self) -> Arc<RwLock<Option<BookTickerData>>> {
        self.latest_cex_ticker_data.clone()
    }
}
