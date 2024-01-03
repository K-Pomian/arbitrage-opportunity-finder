use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use futures_util::{
    stream::{SplitSink, SplitStream},
    FutureExt, SinkExt, StreamExt,
};
use serde::Deserialize;
use tokio::{net::TcpStream, sync::RwLock, time::Instant};
use tokio_tungstenite::{
    tungstenite::{handshake::client::Response, Message},
    MaybeTlsStream, WebSocketStream,
};

const BINANCE_WEBSOCKET_URL: &str = "wss://stream.binance.com:9443/stream";

/*
    Struct representing Binance CEX responsible for connecting to Binance WS and fetching data about provided ticker/pair
*/
pub struct Binance {
    write: RwLock<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
    read: RwLock<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
}

impl Binance {
    /*
        Connects to the WS
    */
    pub async fn connect() -> Result<(Self, Response)> {
        let (socket, response) = tokio_tungstenite::connect_async(BINANCE_WEBSOCKET_URL).await?;
        let (write, read) = socket.split();
        Ok((
            Self {
                write: RwLock::new(write),
                read: RwLock::new(read),
            },
            response,
        ))
    }

    /*
        Subscribes to the stream providing data about the ticker/pair
    */
    pub async fn subscribe_to_ticker(&self, ticker: &str) -> Result<i64> {
        let current_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64; // doesn't overflow
        let subscribe_request = format!(
            "{{\"method\":\"SUBSCRIBE\",\"params\":[\"{}@bookTicker\"],\"id\":{}}}",
            ticker, current_timestamp
        );
        let message = Message::Text(subscribe_request);

        self.write.write().await.send(message).await.unwrap();
        let maybe_result = self.read.write().await.next().await; // The first message is a response to the subscribe request

        if let Some(inner) = maybe_result {
            let message = String::from_utf8(inner.unwrap().into_data()).unwrap();
            if !message.contains("\"result\":null") {
                return Err(anyhow!(format!(
                    "Could not subscribe for ticker {}: {}",
                    ticker, message
                )));
            }
        }

        Ok(current_timestamp)
    }

    pub async fn unsubscribe(&self, ticker: &str, id: i64) -> Result<()> {
        let unsubscribe_request = format!(
            "{{\"method\":\"UNSUBSCRIBE\",\"params\":[\"{}@bookTicker\"],\"id\":{}}}",
            ticker, id
        );
        let message = Message::Text(unsubscribe_request);
        self.write.write().await.send(message).await.unwrap();

        let mut read_write_lock = self.read.write().await;

        // Long deadline, bc timeout seems to mess up `read_write_lock.next()` completion time in debug mode
        // and it's not critical for application performance, as unsubscribe is called only while terminating the application process.
        // Moreover, it's highly unlikely that different message than containing `"result": null` is the last one.
        let deadline = Instant::now() + Duration::from_millis(300);
        let error_context = format!("Could not unsubscribe for ticker {} and id {}", ticker, id);

        while let Some(inner) = tokio::time::timeout_at(deadline, read_write_lock.next())
            .await
            .context(error_context.clone())?
        {
            let message = String::from_utf8(inner.unwrap().into_data()).unwrap();
            if message.contains("\"result\":null") {
                return Ok(());
            }
        }

        unreachable!();
    }

    /*
        Reads the next element of the stream and parses the JSON into BinanceResponse object
    */
    pub async fn read_next_message(&self) -> Option<BinanceResponse> {
        self.read
            .write()
            .await
            .next()
            .then(|element| async {
                if let Some(result) = element {
                    let message = result.unwrap();

                    if let Message::Ping(ping) = message {
                        self.write
                            .write()
                            .await
                            .send(Message::Pong(ping))
                            .await
                            .unwrap();
                        return None;
                    }

                    let message_str = String::from_utf8(message.into_data()).unwrap();

                    return Some(serde_json::from_str::<BinanceResponse>(&message_str).unwrap());
                }

                None
            })
            .await
    }
}

/*
    Structs representing JSON messages from the stream
*/

#[derive(Debug, Deserialize)]
pub struct BinanceResponse {
    pub stream: String,
    pub data: BookTickerData,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Clone, Default)]
pub struct BookTickerData {
    pub u: u64,    // order book updateId
    pub s: String, // symbol
    pub b: String, // best bid price
    pub B: String, // best bid quantity
    pub a: String, // best ask price
    pub A: String, // best ask quantity
}

#[cfg(test)]
mod test {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use tokio_tungstenite::tungstenite::http::StatusCode;

    use super::Binance;

    #[tokio::test]
    async fn test_connect() {
        let (_, response) = Binance::connect().await.unwrap();
        assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
    }

    #[tokio::test]
    async fn test_subscribe_to_ticker() {
        let (binance, _) = Binance::connect().await.unwrap();
        let id = binance.subscribe_to_ticker("btcusdt").await.unwrap();
        assert!(
            id <= SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64
        );
    }

    #[tokio::test]
    async fn test_read_next_message_no_subscription() {
        let (binance, _) = Binance::connect().await.unwrap();
        let result =
            tokio::time::timeout(Duration::from_secs(1), binance.read_next_message()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_next_message() {
        let (binance, _) = Binance::connect().await.unwrap();
        binance.subscribe_to_ticker("btcusdt").await.unwrap();

        let next_message = binance.read_next_message().await.unwrap();
        assert_eq!(next_message.stream, "btcusdt@bookTicker".to_string());
        assert_eq!(next_message.data.s, "BTCUSDT".to_string());
    }
}
