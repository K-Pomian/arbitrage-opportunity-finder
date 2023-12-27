use std::sync::Arc;

use pyth_sdk_solana::Price;
use tokio::sync::RwLock;

pub struct State {
    latest_pyth_price: Arc<RwLock<Option<Price>>>,
}

impl State {
    pub fn new() -> Self {
        Self {
            latest_pyth_price: Arc::new(RwLock::new(None)),
        }
    }

    pub fn get_latest_pyth_price(&self) -> Arc<RwLock<Option<Price>>> {
        self.latest_pyth_price.clone()
    }
}
