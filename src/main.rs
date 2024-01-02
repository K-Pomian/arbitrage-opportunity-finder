use structs::{
    arbitrage_finder::ArbitrageFinder,
    state::{State, STATE},
};

mod config;
mod structs;

#[tokio::main]
async fn main() {
    handle_pyth_price_update();
    handle_binance_ticker_data_update();
    handle_finding_arbitrage_opportunities();
}

fn handle_pyth_price_update() {
    tokio::spawn({
        let state = STATE.get_or_init(|| async { State::new().await }).await;

        async move {
            loop {
                state.update_latest_pyth_price().await;
            }
        }
    });
}

fn handle_binance_ticker_data_update() {
    tokio::spawn({
        let state = STATE.get_or_init(|| async { State::new().await }).await;

        async move {
            loop {
                state.update_latest_binance_price().await;
            }
        }
    });
}

fn handle_finding_arbitrage_opportunities() {
    let state = STATE.get_or_init(|| async { State::new().await }).await;
    let mut arbitrage_finder = ArbitrageFinder::new();

    loop {
        let maybe_opportunity = arbitrage_finder
            .find_opportunity(
                state.get_latest_pyth_price(),
                state.get_latest_binance_ticker_data(),
            )
            .await;
        if let Some(opportunity) = maybe_opportunity {
            println!("Found an opportunity!\n{:#?}\n", opportunity);
        }
    }
}
