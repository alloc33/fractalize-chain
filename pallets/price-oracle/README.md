# Pallet Price Oracle

A production-ready Substrate pallet for fetching real-time cryptocurrency prices from multiple DEXs via direct smart contract calls. Part of building arbitrage infrastructure for multi-chain opportunities.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      OFFCHAIN WORKER                            │
│                    (Runs every N blocks)                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐         │
│  │  Uniswap V3  │   │  PancakeSwap │   │  Trader Joe  │         │
│  │  (Ethereum)  │   │     (BSC)    │   │ (Avalanche)  │ ...     │
│  └──────┬───────┘   └──────┬───────┘   └──────┬───────┘         │
│         │                  │                  │                 │
│         └──────────────────┼──────────────────┘                 │
│                            │                                    │
│                            ▼                                    │
│                   Fetch pool prices                             │
│                   (Direct RPC Calls)                            │
│                            │                                    │
│                            ▼                                    │
│                   Validate Price Bounds                         │
│                            │                                    │
│                            ▼                                    │
│              Create submit_price_unsigned()                     │
│                                                                 │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             │ Submit Unsigned Transaction
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                   TRANSACTION POOL                              │
│                                                                 │
│  validate_unsigned() ──────► Priority: MAX                      │
│                              Longevity: 3 blocks                │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             │ Include in Block
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                      ON-CHAIN STORAGE                           │
│                                                                 │
│  StorageDoubleMap<TokenPairHash, ExchangeId, (Price, Time)>     │
│                                                                 │
│  ┌─────────────┬──────────┬──────────────────┐                  │
│  │  ETH/USD    │ Exchange │  Price & Time    │                  │
│  ├─────────────┼──────────┼──────────────────┤                  │
│  │ 0x1a2b3c... │    1     │ (3200.50, t1)    │                  │
│  │ 0x1a2b3c... │    2     │ (3201.20, t2)    │                  │
│  │ 0x1a2b3c... │    3     │ (3199.80, t3)    │                  │
│  └─────────────┴──────────┴──────────────────┘                  │
│                                                                 │
│  Query via: get_price(pair_hash, exchange_id)                   │
│             get_all_prices(pair_hash)                           │
└─────────────────────────────────────────────────────────────────┘
```

## Features

- **Direct DEX integration** - No API dependencies, fetches via smart contract calls
- **Multi-chain support** - Ethereum, BSC, Polygon, Avalanche
- **Unsigned transactions** - Offchain workers submit prices to on-chain storage
- **Price validation** - Built-in bounds checking and validation
- **Configurable** - Update intervals, timeouts, and exchange limits

## Current Status

**ETH/USD only** - Currently implements ETH/USD price fetching across 5 major DEXs. Additional trading pairs will be added in future releases.

## Supported Exchanges

- Uniswap V3 (Ethereum)
- SushiSwap V2 (Ethereum)  
- PancakeSwap V2 (BSC)
- QuickSwap V2 (Polygon)
- Trader Joe (Avalanche)

## Installation

```toml
[dependencies]
pallet-price-oracle = { git = "https://github.com/alloc33/pallet-price-oracle" }
```

## Configuration

```rust
impl pallet_price_oracle::Config for Runtime {
    type UpdateInterval = ConstU64<10>;      // Update every 10 blocks
    type HttpTimeout = ConstU64<10000>;      // 10 second timeout  
    type MaxExchangesPerBlock = ConstU8<3>;  // Max 3 exchanges per block
}
```

## Usage

```rust
use pallet_price_oracle::types::TokenPair;

// Get ETH/USD price from specific exchange
let eth_hash = TokenPair::EthUsd.to_hash();
let price = PriceOracle::get_price(eth_hash, exchange_id);

// Get all ETH/USD prices
let all_prices = PriceOracle::get_all_prices(eth_hash);
```

Built for [FractalizeChain](https://github.com/alloc33/fractalize-chain)
