# FractalizeChain

**Fair Value Extraction infrastructure for Polkadot ecosystem**

A Substrate-based blockchain that democratizes MEV by making value extraction accessible, transparent, and fairly distributed across the network.

---

## The Problem

MEV (Maximal Extractable Value) extraction is currently centralized:
- 90%+ of MEV profits go to sophisticated players with expensive infrastructure
- Individual users have no access to arbitrage opportunities
- Cross-chain arbitrage requires significant capital and technical expertise
- Value extraction happens in the dark, benefiting only the few

**$500M+ in MEV is extracted daily. The system is broken.**

## Our Solution

FractalizeChain provides open infrastructure for fair value extraction:

✅ **Accessible** - Anyone can participate as validator or user
✅ **Transparent** - All opportunities and distributions on-chain
✅ **Fair** - Profits distributed across network participants
✅ **Cross-chain** - Detects opportunities across multiple chains
✅ **Capital-efficient** - Flash loan integration removes capital requirements

### How It Works

```
Offchain Workers → Monitor DEXs across chains (Ethereum, BSC, Polygon, Avalanche)
                 ↓
              Detect arbitrage opportunities
                 ↓
         Submit unsigned transactions to chain
                 ↓
    Validators validate and include in blocks
                 ↓
        Execute arbitrage via flash loans
                 ↓
   Distribute profits fairly: Validators (45%) + Users (35%) + Protocol (15%) + Treasury (5%)
```

**Direct smart contract calls** - No API dependencies, real-time price discovery via contract state queries.

## Current Status

🚧 **Early Development** - Foundation being built

**Implemented:**
- ✅ **Price Oracle Pallet** - Production-ready offchain workers with unsigned transactions
- ✅ **Multi-chain support** - Ethereum, BSC, Polygon, Avalanche
- ✅ **5 DEX integrations** - Uniswap V3, SushiSwap, PancakeSwap, QuickSwap, Trader Joe
- ✅ **Real-time price feeds** - Direct `slot0()` and `getReserves()` calls
- ✅ **27 passing tests** - Production-quality code

**In Development:**
- ⏳ Flash loan pallet (capital-free arbitrage)
- ⏳ Opportunity detection engine
- ⏳ Fair distribution mechanism
- ⏳ Cross-chain bridge integration

## Architecture

### Price Oracle Pallet

Offchain workers fetch prices directly from DEX contracts every N blocks:

```rust
// Offchain worker runs at configured interval
fn offchain_worker(block_number: BlockNumberFor<T>) {
    for pair in supported_pairs {
        let exchanges = registry::get_all_exchanges();
        for exchange in exchanges {
            // Direct contract call (no API)
            let (price, timestamp) = exchange.fetch_price(pair)?;

            // Submit via unsigned transaction
            let call = Call::submit_price_unsigned { pair, exchange_id, price, timestamp };
            T::create_bare(call.into());
            SubmitTransaction::submit_transaction(xt)?;
        }
    }
}
```

**Unsigned transactions** bridge offchain data collection with onchain storage, validated by `ValidateUnsigned` trait.

### Supported DEXs

| Exchange | Chain | Protocol |
|----------|-------|----------|
| Uniswap V3 | Ethereum | V3 (concentrated liquidity) |
| SushiSwap | Ethereum | V2 (constant product) |
| PancakeSwap | BSC | V2 |
| QuickSwap | Polygon | V2 |
| Trader Joe | Avalanche | V2 |

## Quick Start

```bash
# Build the chain
cargo build --release

# Run local development node
./target/release/fractalize-chain-node --dev

# Watch price oracle activity
RUST_LOG=runtime=debug,pallet_price_oracle=debug \
  ./target/release/fractalize-chain-node --dev
```

## Development Roadmap

**Phase 1: Price Infrastructure** ✅ (Current)
- Multi-chain price oracle with offchain workers
- Direct DEX contract integration
- Unsigned transaction validation

**Phase 2: Opportunity Detection** 🚧 (Next 4-8 weeks)
- Cross-chain arbitrage detection logic
- Profit calculation with gas cost estimation
- Minimum viable opportunity thresholds

**Phase 3: Flash Loan Integration** (8-16 weeks)
- Native flash loan pallet
- Integration with price oracle
- Atomic arbitrage execution

**Phase 4: Fair Distribution** (16-24 weeks)
- Validator reward mechanism
- User profit distribution
- Protocol fee collection
- Treasury governance

**Phase 5: Testnet Launch** (24+ weeks)
- Public testnet deployment
- Validator recruitment
- Community building
- Economic model testing

## Why Substrate?

- **Offchain workers** - Perfect for fetching external DEX data
- **Unsigned transactions** - Offchain → onchain data flow without accounts
- **Modular pallets** - Clean separation of concerns (oracle, flash loans, arbitrage)
- **Production-ready** - Battle-tested framework from Parity
- **Polkadot ecosystem** - Aligns with JAM vision and Web3 infrastructure

## Project Philosophy

**Open Source Infrastructure**
- All core pallets are open source
- Community can audit, contribute, and fork
- Transparent by default

**Fair by Design**
- Profit distribution hardcoded in protocol
- No extraction by core team beyond protocol fee
- Validators and users share majority of profits

**Solving Real Problems**
- MEV centralization is harmful to crypto ecosystem
- Democratizing access creates more fair markets
- Building public goods for Polkadot

## Contributing

This is open infrastructure for the ecosystem. Contributions welcome:

- 🐛 **Issues** - Bug reports, feature requests
- 💡 **Pallets** - Extend functionality
- 📝 **Documentation** - Improve clarity
- 🧪 **Tests** - Increase coverage

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Project Status

**Active Development** - Building towards testnet launch

This is a long-term infrastructure project, not a get-rich-quick scheme. We're solving hard problems (cross-chain coordination, fair distribution, flash loan security) and building openly.

**Progress updates:**
- Follow development in GitHub issues
- Technical deep-dives on [blog](#)
- Join discussion in [Discord](#)

---

**Built with Substrate | Open Source Infrastructure | Fair Value Extraction**

*Making MEV accessible to everyone, not just the few.*
