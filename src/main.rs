use structs::{
    arbitrage_finder::ArbitrageFinder,
    state::{State, STATE},
};

mod config;
mod structs;

#[tokio::main]
async fn main() {
    handle_pyth_price_update().await;
    handle_binance_ticker_data_update().await;
    handle_finding_arbitrage_opportunities().await;
}

async fn handle_pyth_price_update() {
    println!("Spawning Pyth price updater");

    tokio::spawn({
        let state = STATE.get_or_init(|| async { State::new().await }).await;

        async move {
            loop {
                state.update_latest_pyth_price().await;
            }
        }
    });
}

async fn handle_binance_ticker_data_update() {
    println!("Spawning Binance ticker data updater");

    tokio::spawn({
        let state = STATE.get_or_init(|| async { State::new().await }).await;

        async move {
            loop {
                state.update_latest_binance_price().await;
            }
        }
    });
}

async fn handle_finding_arbitrage_opportunities() {
    println!("Searching for arbitrage opportunities");

    let state = STATE.get_or_init(|| async { State::new().await }).await;
    let mut arbitrage_finder = ArbitrageFinder::new();

    loop {
        let maybe_opportunity = arbitrage_finder
            .find_opportunity(
                state.get_latest_pyth_price(),
                state.get_latest_binance_ticker_data(),
                state.binance_taker_fee,
            )
            .await;
        if let Some(opportunity) = maybe_opportunity {
            println!("Found an opportunity!\n{:#?}\n", opportunity);
        }
    }
}
