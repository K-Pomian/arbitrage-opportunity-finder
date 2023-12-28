use structs::state::{State, STATE};

mod config;
mod structs;

#[tokio::main]
async fn main() {
    tokio::spawn({
        let state = STATE.get_or_init(|| async { State::new().await }).await;

        async move {
            loop {
                state.update_latest_pyth_price().await;
            }
        }
    });

    tokio::spawn({
        let state = STATE.get_or_init(|| async { State::new().await }).await;

        async move {
            loop {
                state.update_latest_binance_price().await;
            }
        }
    });

    let state = STATE.get_or_init(|| async { State::new().await }).await;
    loop {
        println!("{:?}", state.get_latest_binance_ticker_data().read().await);
        println!("{:?}", state.get_latest_pyth_price().read().await);
    }
}
