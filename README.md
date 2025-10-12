# FractalizeChain

A Substrate-based blockchain specialized for cross-exchange arbitrage detection and MEV extraction.

## Overview

FractalizeChain democratizes MEV by enabling automated arbitrage across multiple DEXs through direct smart contract integration and flash loan execution.

**Core Innovation:** Direct DEX contract calls instead of API dependencies for real-time price discovery across chains.

## Current Status

⚠️ **Early Development** - Only price oracle pallet implemented

**Implemented:**
- ✅ Price Oracle Pallet with off-chain workers
- ✅ Multi-chain DEX support (Ethereum, BSC, Polygon, Avalanche)  
- ✅ Modular exchange abstractions (Uniswap V2/V3, PancakeSwap, etc.)

**Planned:**
- Flash loan pallet for capital-free arbitrage
- MEV detection and execution engine
- Cross-chain bridge integration

## Architecture

```rust
// Off-chain workers fetch prices via direct contract calls
eth_call(Uniswap_V3.slot0()) → ETH: $4,530
eth_call(PancakeSwap.getReserves()) → ETH: $4,545
// Arbitrage opportunity: $15 spread
```

### Price Oracle Pallet

```rust
#[pallet::hooks]
impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
    fn offchain_worker(block_number: BlockNumberFor<T>) {
        for pair in [TokenPair::EthUsd] {
            let exchanges = registry::get_all_exchanges();
            for exchange in exchanges {
                Self::fetch_prices(exchange, pair)?;
            }
        }
    }
}
```

**Supported Exchanges:**
- Uniswap V3 (Ethereum)
- SushiSwap V2 (Ethereum) 
- PancakeSwap V2 (BSC)
- QuickSwap V2 (Polygon)
- Trader Joe (Avalanche)

## Build & Run

```bash
cargo build --release
./target/release/fractalize-chain-node --dev

# Monitor price updates
RUST_LOG=runtime=debug ./target/release/fractalize-chain-node --dev
```

## Next Steps

1. **Flash Loan Pallet** - Native flash loan functionality
2. **Arbitrage Engine** - MEV detection and automated execution  
3. **Cross-chain Support** - Multi-chain arbitrage opportunities

---

Built with Substrate | Focus on decentralized MEV extraction
