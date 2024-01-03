use structs::{
    arbitrage_finder::ArbitrageFinder,
    state::{State, STATE},
};
use tokio::task::JoinHandle;

mod config;
mod structs;

#[tokio::main]
async fn main() {
    let tasks = vec![
        handle_pyth_price_update().await,
        handle_binance_ticker_data_update().await,
        handle_finding_arbitrage_opportunities().await,
    ];

    handle_shutdown(tasks).await;
}

async fn handle_pyth_price_update() -> JoinHandle<()> {
    println!("Spawning Pyth price updater");

    tokio::spawn({
        let state = STATE.get_or_init(|| async { State::new().await }).await;

        async move {
            loop {
                state.update_latest_pyth_price().await;
            }
        }
    })
}

async fn handle_binance_ticker_data_update() -> JoinHandle<()> {
    println!("Spawning Binance ticker data updater");

    tokio::spawn({
        let state = STATE.get_or_init(|| async { State::new().await }).await;

        async move {
            loop {
                state.update_latest_binance_ticker_data().await;
            }
        }
    })
}

async fn handle_finding_arbitrage_opportunities() -> JoinHandle<()> {
    println!("Searching for arbitrage opportunities");

    tokio::spawn({
        let state = STATE.get_or_init(|| async { State::new().await }).await;
        let mut arbitrage_finder = ArbitrageFinder::new();

        async move {
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
    })
}

async fn handle_shutdown(tasks: Vec<JoinHandle<()>>) {
    match tokio::signal::ctrl_c().await {
        Ok(()) => {
            println!("\nAborting tasks...");
            tasks.into_iter().for_each(|task| task.abort());

            println!("Terminating Binance WS connection...");
            let state = STATE.get_or_init(|| async { State::new().await }).await;
            state.terminate().await;

            println!("Finished");
        }
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
        }
    }
}
