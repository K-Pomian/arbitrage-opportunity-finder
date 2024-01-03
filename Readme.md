# Overview
The application connects with Binance and Pyth in purpose of finding arbitrage opportunities between the CEX and Solana markets.

# How to run
You should run one of the following commands:
```
cargo run --release
```
or
```
cargo run --release -- -b <binance_ticker> -p <pyth_price_account_pubkey>
```
The default values for the arguments are respectively `solusdt` and `H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG`, which is the pubkey of the Solana account holding price information about SOL/USD pair.

# Additional information
Full list of Binance tickers can be found [here](https://api.binance.com/api/v3/exchangeInfo).  
Full list of Pyth's Solana price accounts' pubkeys can be found [here](https://pyth.network/price-feeds?cluster=solana-mainnet-beta)
