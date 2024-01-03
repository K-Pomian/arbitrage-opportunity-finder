use std::{str::FromStr, sync::Arc};

use pyth_sdk_solana::Price;
use rust_decimal::Decimal;
use tokio::sync::RwLock;

use super::cex::binance::BookTickerData;

/*
    Struct for finding arbitrage opportunities between Binance and DEXes
*/
pub struct ArbitrageFinder {
    last_found: Option<ArbitrageOpportunity>,
}

impl ArbitrageFinder {
    pub fn new() -> Self {
        return Self { last_found: None };
    }

    /*
        Compares Binance and Pyth prices to find arbitrage opportunities
    */
    pub async fn find_opportunity(
        &mut self,
        latest_pyth_price: Arc<RwLock<Option<Price>>>,
        latest_binance_ticker_data: Arc<RwLock<Option<BookTickerData>>>,
        binance_fee: Decimal,
    ) -> Option<ArbitrageOpportunity> {
        let (latest_pyth_price_read, latest_binance_ticker_data_read) =
            tokio::join!(latest_pyth_price.read(), latest_binance_ticker_data.read());

        if latest_pyth_price_read.is_none() || latest_binance_ticker_data_read.is_none() {
            return None;
        }

        let pyth_price = (*latest_pyth_price_read).unwrap();
        drop(latest_pyth_price_read);
        let binance_ticker_data = (*latest_binance_ticker_data_read).clone().unwrap();
        drop(latest_binance_ticker_data_read);

        let (pyth_confident_95_price_higher, pyth_confident_95_price_lower) =
            self.calculate_pyth_confident_95_price(pyth_price);

        // Search for SellBinanceBuyDex opportunity
        let binance_best_bid_price = Decimal::from_str(&binance_ticker_data.b).unwrap();
        if binance_best_bid_price.gt(&pyth_confident_95_price_higher) {
            let quantity = Decimal::from_str(&binance_ticker_data.B).unwrap();
            return self.calculate_arbitrage_opportunity(
                binance_best_bid_price,
                pyth_confident_95_price_higher,
                binance_fee,
                quantity,
                ArbitrageDirection::SellBinanceBuyDex,
            );
        }

        // Search for BuyBinanceSellDex opportunity
        let binance_best_ask_price = Decimal::from_str(&binance_ticker_data.a).unwrap();
        if binance_best_ask_price.lt(&pyth_confident_95_price_lower) {
            let quantity = Decimal::from_str(&binance_ticker_data.A).unwrap();
            return self.calculate_arbitrage_opportunity(
                binance_best_ask_price,
                pyth_confident_95_price_lower,
                binance_fee,
                quantity,
                ArbitrageDirection::BuyBinanceSellDex,
            );
        }

        None
    }

    /*
        Calculates probable (95%) price using Pyth price and confidence feed and Laplace distribution
        https://docs.pyth.network/documentation/solana-price-feeds/best-practices#confidence-intervals
    */
    fn calculate_pyth_confident_95_price(&self, pyth_price: Price) -> (Decimal, Decimal) {
        let exponential = pyth_price.expo.abs() as u32;
        let price = Decimal::new(pyth_price.price, exponential);
        let confidence = Decimal::new(pyth_price.conf.try_into().unwrap(), exponential);
        let confidence_95 = confidence.checked_mul(Decimal::new(212, 2)).unwrap();

        (
            price.checked_add(confidence_95).unwrap(),
            price.checked_sub(confidence_95).unwrap(),
        )
    }

    /*
        Calculates estimated profit and returns Option<ArbitrageOpportunity> instance depending on the calculation
    */
    fn calculate_arbitrage_opportunity(
        &mut self,
        binance_price: Decimal,
        pyth_price: Decimal,
        binance_fee: Decimal,
        quantity: Decimal,
        arbitrage_direction: ArbitrageDirection,
    ) -> Option<ArbitrageOpportunity> {
        let estimated_profit = (binance_price - pyth_price)
            .abs()
            .checked_mul(quantity)
            .unwrap()
            - quantity
                .checked_mul(binance_price)
                .unwrap()
                .checked_mul(binance_fee)
                .unwrap();

        if estimated_profit.le(&Decimal::ZERO) {
            return None;
        }

        let opportunity = ArbitrageOpportunity {
            direction: arbitrage_direction,
            quantity: quantity.normalize(),
            estimated_profit: estimated_profit.normalize().round_dp(8),
            binance_price: binance_price.normalize(),
            pyth_price: pyth_price.normalize(),
        };

        if let Some(last_opportunity) = self.last_found {
            if last_opportunity == opportunity {
                return None;
            }
        }
        self.last_found = Some(opportunity);

        return self.last_found;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArbitrageOpportunity {
    pub direction: ArbitrageDirection,
    pub quantity: Decimal,
    pub estimated_profit: Decimal,
    pub binance_price: Decimal,
    pub pyth_price: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArbitrageDirection {
    SellBinanceBuyDex,
    BuyBinanceSellDex,
}

#[cfg(test)]
mod tests {
    use std::{str::FromStr, sync::Arc};

    use pyth_sdk_solana::Price;
    use rust_decimal::Decimal;
    use tokio::sync::RwLock;

    use crate::structs::cex::binance::BookTickerData;

    use super::{ArbitrageDirection, ArbitrageFinder};

    #[test]
    fn test_calculate_pyth_confident_95_price() {
        let arbitrage_finder = ArbitrageFinder::new();
        let price = Price {
            price: 4856126854,
            conf: 612455,
            expo: -5,
            ..Default::default()
        };

        let (higher, lower) = arbitrage_finder.calculate_pyth_confident_95_price(price);
        assert_eq!(lower.normalize().to_string(), "48548.284494");
        assert_eq!(higher.normalize().to_string(), "48574.252586");
    }

    #[tokio::test]
    async fn test_find_opportunity_data_none() {
        let mut arbitrage_finder = ArbitrageFinder::new();

        // Both none
        {
            let result = arbitrage_finder
                .find_opportunity(
                    Arc::new(RwLock::new(None)),
                    Arc::new(RwLock::new(None)),
                    Decimal::default(),
                )
                .await;
            assert!(result.is_none());
        }

        // Only binance data none
        {
            let result = arbitrage_finder
                .find_opportunity(
                    Arc::new(RwLock::new(Some(Price::default()))),
                    Arc::new(RwLock::new(None)),
                    Decimal::default(),
                )
                .await;
            assert!(result.is_none());
        }

        // Only pyth data none
        {
            let result = arbitrage_finder
                .find_opportunity(
                    Arc::new(RwLock::new(None)),
                    Arc::new(RwLock::new(Some(BookTickerData::default()))),
                    Decimal::default(),
                )
                .await;
            assert!(result.is_none());
        }
    }

    #[tokio::test]
    async fn test_find_opportunity() {
        let mut arbitrage_finder = ArbitrageFinder::new();

        // SellBinanceBuyDex direction
        {
            // l: 68.43263012 h: 71.27225988
            let latest_pyth_price = Arc::new(RwLock::new(Some(Price {
                price: 69852445,
                conf: 669724,
                expo: -6,
                ..Default::default()
            })));
            let latest_binance_ticker_data = Arc::new(RwLock::new(Some(BookTickerData {
                b: "71.3833".to_string(),
                B: "0.8574".to_string(),
                a: "72.0012".to_string(),
                A: "0.9245".to_string(),
                ..Default::default()
            })));

            let result = arbitrage_finder
                .find_opportunity(
                    latest_pyth_price,
                    latest_binance_ticker_data,
                    Decimal::new(1, 3),
                )
                .await
                .unwrap();
            assert_eq!(result.direction, ArbitrageDirection::SellBinanceBuyDex);
            assert_eq!(result.quantity, Decimal::from_str("0.8574").unwrap());
            assert_eq!(
                result.estimated_profit,
                Decimal::from_str("0.03400176").unwrap()
            );
        }

        // SellBinanceBuyDex direction, but too large fee
        {
            // l: 68.43263012 h: 71.27225988
            let latest_pyth_price = Arc::new(RwLock::new(Some(Price {
                price: 69852445,
                conf: 669724,
                expo: -6,
                ..Default::default()
            })));
            let latest_binance_ticker_data = Arc::new(RwLock::new(Some(BookTickerData {
                b: "71.3833".to_string(),
                B: "0.8574".to_string(),
                a: "72.0012".to_string(),
                A: "0.9245".to_string(),
                ..Default::default()
            })));

            let result = arbitrage_finder
                .find_opportunity(
                    latest_pyth_price,
                    latest_binance_ticker_data,
                    Decimal::new(5, 3),
                )
                .await;
            assert!(result.is_none())
        }

        // BuyBinanceSellDex direction
        {
            // l: 68.43263012 h: 71.27225988
            let latest_pyth_price = Arc::new(RwLock::new(Some(Price {
                price: 69852445,
                conf: 669724,
                expo: -6,
                ..Default::default()
            })));
            let latest_binance_ticker_data = Arc::new(RwLock::new(Some(BookTickerData {
                b: "67.5421".to_string(),
                B: "1.1258".to_string(),
                a: "67.8423".to_string(),
                A: "2.5569".to_string(),
                ..Default::default()
            })));

            let result = arbitrage_finder
                .find_opportunity(
                    latest_pyth_price,
                    latest_binance_ticker_data,
                    Decimal::new(1, 3),
                )
                .await
                .unwrap();
            assert_eq!(result.direction, ArbitrageDirection::BuyBinanceSellDex);
            assert_eq!(result.quantity, Decimal::from_str("2.5569").unwrap());
            assert_eq!(
                result.estimated_profit,
                Decimal::from_str("1.33594911").unwrap()
            );
        }

        // BuyBinanceSellDex direction, but too large fee
        {
            // l: 68.43263012 h: 71.27225988
            let latest_pyth_price = Arc::new(RwLock::new(Some(Price {
                price: 69852445,
                conf: 669724,
                expo: -6,
                ..Default::default()
            })));
            let latest_binance_ticker_data = Arc::new(RwLock::new(Some(BookTickerData {
                b: "67.5421".to_string(),
                B: "1.1258".to_string(),
                a: "67.8423".to_string(),
                A: "2.5569".to_string(),
                ..Default::default()
            })));

            let result = arbitrage_finder
                .find_opportunity(
                    latest_pyth_price,
                    latest_binance_ticker_data,
                    Decimal::new(1, 2),
                )
                .await;
            assert!(result.is_none());
        }

        // No opportunity found
        {
            // l: 68.43263012 h: 71.27225988
            let latest_pyth_price = Arc::new(RwLock::new(Some(Price {
                price: 69852445,
                conf: 669724,
                expo: -6,
                ..Default::default()
            })));
            let latest_binance_ticker_data = Arc::new(RwLock::new(Some(BookTickerData {
                b: "69.2222".to_string(),
                B: "1.1258".to_string(),
                a: "69.1111".to_string(),
                A: "2.5569".to_string(),
                ..Default::default()
            })));

            let result = arbitrage_finder
                .find_opportunity(
                    latest_pyth_price,
                    latest_binance_ticker_data,
                    Decimal::new(1, 3),
                )
                .await;
            assert!(result.is_none());
        }
    }
}
