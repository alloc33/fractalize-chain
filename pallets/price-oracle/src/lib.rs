#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use alloy_primitives::{hex, U256};
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_runtime::offchain::{http, Duration};
	use sp_std::{collections::btree_map::BTreeMap, str, vec, vec::Vec};
	extern crate alloc;
	use alloc::{
		format,
		string::{String, ToString},
	};

	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
	pub enum TokenPair {
		EthUsd,
		BtcUsd,
		SolUsd,
		AvaxUsd,
	}

	impl TokenPair {
		pub fn as_str(&self) -> &'static str {
			match self {
				TokenPair::EthUsd => "ETH/USD",
				TokenPair::BtcUsd => "BTC/USD",
				TokenPair::SolUsd => "SOL/USD",
				TokenPair::AvaxUsd => "AVAX/USD",
			}
		}

		pub fn to_hash(&self) -> [u8; 32] {
			sp_io::hashing::blake2_256(self.as_str().as_bytes())
		}
	}

	/// Chain-specific interface for contract calls
	trait ChainInterface {
		type RawResponse;

		/// Make a contract call and return raw response
		fn call_contract(
			rpc_url: &str,
			address: &str,
			data: &[u8],
		) -> Result<Self::RawResponse, http::Error>;
	}

	/// EVM-compatible chains (Ethereum, BSC, Polygon, Avalanche, Arbitrum, etc.)
	struct EvmChain;
	impl ChainInterface for EvmChain {
		type RawResponse = Vec<u8>;

		fn call_contract(
			rpc_url: &str,
			address: &str,
			data: &[u8],
		) -> Result<Self::RawResponse, http::Error> {
			let rpc_request = format!(
				"{{\"jsonrpc\":\"2.0\",\"method\":\"eth_call\",\"params\":[{{\"to\":\"{}\",\"data\":\"0x{}\"}},\"latest\"],\"id\":1}}",
				address,
				hex::encode(data)
			);

			let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(10000));
			let request = http::Request::post(rpc_url, vec![rpc_request.into_bytes()])
				.add_header("Content-Type", "application/json")
				.deadline(deadline);

			let pending = request.send().map_err(|_| http::Error::IoError)?;
			let response =
				pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;

			if response.code != 200 {
				log::error!("RPC error: HTTP {}", response.code);
				return Err(http::Error::Unknown);
			}

			let body = response.body().collect::<Vec<u8>>();
			let response_str = sp_std::str::from_utf8(&body).map_err(|_| http::Error::Unknown)?;

			// Parse JSON to extract result hex data using the working logic
			let hex_data = extract_result_data(response_str)?;
			hex::decode(&hex_data).map_err(|_| {
				log::error!("Failed to decode hex data: {}", hex_data);
				http::Error::Unknown
			})
		}
	}

	/// Extract hex data from JSON RPC response - matches working logic
	fn extract_result_data(json: &str) -> Result<String, http::Error> {
		// Simple JSON parsing to get "result" field
		if let Some(start) = json.find("\"result\":\"") {
			let data_start = start + 10; // Skip '"result":"'
			if let Some(end) = json[data_start..].find('"') {
				let hex_data = &json[data_start..data_start + end];
				if let Some(stripped) = hex_data.strip_prefix("0x") {
					return Ok(stripped.to_string());
				}
			}
		}
		Err(http::Error::Unknown)
	}

	/// DEX protocol abstraction (independent of chain)
	trait DexProtocol<C: ChainInterface> {
		/// Get pool address for a specific trading pair
		fn get_pool_address(pair: TokenPair) -> &'static str;

		/// Get function call data for price query
		fn get_call_data(pair: TokenPair) -> Vec<u8>;

		/// Parse raw response to extract price
		fn parse_price(response: C::RawResponse) -> Result<f64, &'static str>;
	}

	/// Uniswap V3 protocol (works on any EVM chain)
	struct UniswapV3Protocol;
	impl DexProtocol<EvmChain> for UniswapV3Protocol {
		fn get_pool_address(pair: TokenPair) -> &'static str {
			match pair {
				TokenPair::EthUsd => "0x8ad599c3a0ff1de082011efddc58f1908eb6e6d8", // ETH/USDC 0.3%
				TokenPair::BtcUsd => "0x99ac8ca7087fa4a2a1fb6357269965a2014abc35", /* WBTC/USDC 0.3% */
				TokenPair::SolUsd => "0xd0fc8ba7e267f2bc56044a7715a489d851dc6d78", // SOL/USDC 0.3%
				TokenPair::AvaxUsd => "0xfab5a05c933f1a2463e334e011992e897d56ef0a", /* AVAX/USDC 0.3% */
			}
		}

		fn get_call_data(_pair: TokenPair) -> Vec<u8> {
			hex!("3850c7bd").to_vec() // slot0() selector
		}

		fn parse_price(data: Vec<u8>) -> Result<f64, &'static str> {
			if data.len() < 32 {
				return Err("Invalid Uniswap V3 response length");
			}

			let sqrt_price_x96 = U256::from_be_slice(&data[0..32]);
			let sqrt_price_f64 = sqrt_price_x96.to::<u128>() as f64;
			let q96_f64 = (1u128 << 96) as f64;
			let sqrt_ratio = sqrt_price_f64 / q96_f64;
			let price_ratio = sqrt_ratio * sqrt_ratio;
			let eth_per_usdc = price_ratio * 1e-12;
			let usd_per_eth = 1.0 / eth_per_usdc;

			if usd_per_eth > 1000.0 && usd_per_eth < 20000.0 {
				Ok(usd_per_eth)
			} else {
				Err("Price out of reasonable range")
			}
		}
	}

	/// Uniswap V2 protocol (works on any EVM chain - PancakeSwap, SushiSwap, QuickSwap, etc.)
	struct UniswapV2Protocol;
	impl DexProtocol<EvmChain> for UniswapV2Protocol {
		fn get_pool_address(pair: TokenPair) -> &'static str {
			match pair {
				TokenPair::EthUsd => "0x397ff1542f962076d0bfe58ea045ffa2d347aca0", // WETH/USDC
				TokenPair::BtcUsd => "0xceff51756c56ceffca006cd410b03ffc46dd3a58", // WBTC/USDC
				TokenPair::SolUsd => "0x2b3e659649c0c0dd0a7fac6f11f4a8a8dffdaeee", /* SOL/USDC (if exists) */
				TokenPair::AvaxUsd => "0xc3f279090a47e80990fe3a9c30d24cb117ef91a8", /* AVAX/USDC (if exists) */
			}
		}

		fn get_call_data(_pair: TokenPair) -> Vec<u8> {
			hex!("0902f1ac").to_vec() // getReserves() selector
		}

		fn parse_price(data: Vec<u8>) -> Result<f64, &'static str> {
			if data.len() < 96 {
				return Err("Invalid V2 response length");
			}

			let reserve0_u256 = U256::from_be_slice(&data[0..32]);
			let reserve1_u256 = U256::from_be_slice(&data[32..64]);
			let reserve0_u128 = reserve0_u256.to::<u128>();
			let reserve1_u128 = reserve1_u256.to::<u128>();

			if reserve0_u128 == 0 || reserve1_u128 == 0 {
				return Err("Zero liquidity in pool");
			}

			let reserve0_scaled = reserve0_u128 as f64;
			let reserve1_scaled = reserve1_u128 as f64;
			let ratio1 = reserve0_scaled / reserve1_scaled;
			let ratio2 = reserve1_scaled / reserve0_scaled;

			// Try different decimal adjustments to find reasonable ETH price
			let price_options = [
				ratio1 * 1e12,  // reserve0=USDC, reserve1=ETH
				ratio2 * 1e-12, // reserve0=ETH, reserve1=USDC
				ratio1,         // Same decimals
				ratio2,         // Same decimals inverted
			];

			for price in price_options {
				if price > 1000.0 && price < 20000.0 {
					return Ok(price);
				}
			}

			Err("No reasonable ETH price found")
		}
	}

	/// PancakeSwap protocol (BSC chain)
	struct PancakeSwapProtocol;
	impl DexProtocol<EvmChain> for PancakeSwapProtocol {
		fn get_pool_address(pair: TokenPair) -> &'static str {
			match pair {
				TokenPair::EthUsd => "0xea26b78255df2bbc31c1ebf60010d78670185bd0", /* ETH/USDC on BSC */
				TokenPair::BtcUsd => "0xd99c7f6c65857ac913a8f880a4cb84032ab2fc5b", /* BTCB/USDC on BSC */
				TokenPair::SolUsd => "0x1b96b92314c44b159149f7e0303511fb2fc4774f", /* SOL/USDC on BSC */
				TokenPair::AvaxUsd => "0x40afdb9a8bb67ba8ab8cedc4ee0c6ecf9bbdaaf6", /* AVAX/USDC on BSC */
			}
		}

		fn get_call_data(_pair: TokenPair) -> Vec<u8> {
			hex!("0902f1ac").to_vec() // getReserves() selector
		}

		fn parse_price(data: Vec<u8>) -> Result<f64, &'static str> {
			if data.len() < 96 {
				return Err("Invalid PancakeSwap response length");
			}

			let reserve0_u256 = U256::from_be_slice(&data[0..32]);
			let reserve1_u256 = U256::from_be_slice(&data[32..64]);
			let reserve0_u128 = reserve0_u256.to::<u128>();
			let reserve1_u128 = reserve1_u256.to::<u128>();

			if reserve0_u128 == 0 || reserve1_u128 == 0 {
				return Err("Zero liquidity in PancakeSwap pool");
			}

			let reserve0_scaled = reserve0_u128 as f64;
			let reserve1_scaled = reserve1_u128 as f64;
			let ratio1 = reserve0_scaled / reserve1_scaled;
			let ratio2 = reserve1_scaled / reserve0_scaled;

			// Try different decimal adjustments to find reasonable ETH price
			let price_options = [
				ratio1 * 1e12,  // reserve0=USDC, reserve1=ETH
				ratio2 * 1e-12, // reserve0=ETH, reserve1=USDC
				ratio1,         // Same decimals
				ratio2,         // Same decimals inverted
			];

			for price in price_options {
				if price > 1000.0 && price < 20000.0 {
					return Ok(price);
				}
			}

			Err("No reasonable ETH price found on PancakeSwap")
		}
	}

	/// QuickSwap protocol (Polygon chain)
	struct QuickSwapProtocol;
	impl DexProtocol<EvmChain> for QuickSwapProtocol {
		fn get_pool_address(pair: TokenPair) -> &'static str {
			match pair {
				TokenPair::EthUsd => "0x853ee4b2a13f8a742d64c8f088be7ba2131f670d", /* ETH/USDC on Polygon */
				TokenPair::BtcUsd => "0xf6a637525402643b0654a54bead2cb9a83c8b498", /* WBTC/USDC on Polygon */
				TokenPair::SolUsd => "0x69015912aa33720b842dcd6ac78d8ac8be25f32a", /* SOL/USDC on Polygon */
				TokenPair::AvaxUsd => "0xb0b195aefa3650a6908f15cdac7d92f8a5791b0b", /* AVAX/USDC on Polygon */
			}
		}

		fn get_call_data(_pair: TokenPair) -> Vec<u8> {
			hex!("0902f1ac").to_vec() // getReserves() selector
		}

		fn parse_price(data: Vec<u8>) -> Result<f64, &'static str> {
			if data.len() < 96 {
				return Err("Invalid QuickSwap response length");
			}

			let reserve0_u256 = U256::from_be_slice(&data[0..32]);
			let reserve1_u256 = U256::from_be_slice(&data[32..64]);
			let reserve0_u128 = reserve0_u256.to::<u128>();
			let reserve1_u128 = reserve1_u256.to::<u128>();

			if reserve0_u128 == 0 || reserve1_u128 == 0 {
				return Err("Zero liquidity in QuickSwap pool");
			}

			let reserve0_scaled = reserve0_u128 as f64;
			let reserve1_scaled = reserve1_u128 as f64;
			let ratio1 = reserve0_scaled / reserve1_scaled;
			let ratio2 = reserve1_scaled / reserve0_scaled;

			// Try different decimal adjustments to find reasonable ETH price
			let price_options = [
				ratio1 * 1e12,  // reserve0=USDC, reserve1=ETH
				ratio2 * 1e-12, // reserve0=ETH, reserve1=USDC
				ratio1,         // Same decimals
				ratio2,         // Same decimals inverted
			];

			for price in price_options {
				if price > 1000.0 && price < 20000.0 {
					return Ok(price);
				}
			}

			Err("No reasonable ETH price found on QuickSwap")
		}
	}

	/// Trader Joe protocol (Avalanche chain)
	struct TraderJoeProtocol;
	impl DexProtocol<EvmChain> for TraderJoeProtocol {
		fn get_pool_address(pair: TokenPair) -> &'static str {
			match pair {
				TokenPair::EthUsd => "0xfe15c2695f1f920da45c30aae47d11de51007af9", /* ETH/USDC on Avalanche */
				TokenPair::BtcUsd => "0x2fb245b9c0fb306e93c5764d5e9a8b4e975e72b7", /* BTC/USDC on Avalanche */
				TokenPair::SolUsd => "0x2d9e55e67c3b1c42823c8b618bb3e5b1c6e8e2ea", /* SOL/USDC on Avalanche */
				TokenPair::AvaxUsd => "0xa389f9430876455c36478deea9769b7ca4e3ddb1", /* AVAX/USDC on Avalanche */
			}
		}

		fn get_call_data(_pair: TokenPair) -> Vec<u8> {
			hex!("0902f1ac").to_vec() // getReserves() selector
		}

		fn parse_price(data: Vec<u8>) -> Result<f64, &'static str> {
			if data.len() < 96 {
				return Err("Invalid Trader Joe response length");
			}

			let reserve0_u256 = U256::from_be_slice(&data[0..32]);
			let reserve1_u256 = U256::from_be_slice(&data[32..64]);
			let reserve0_u128 = reserve0_u256.to::<u128>();
			let reserve1_u128 = reserve1_u256.to::<u128>();

			if reserve0_u128 == 0 || reserve1_u128 == 0 {
				return Err("Zero liquidity in Trader Joe pool");
			}

			let reserve0_scaled = reserve0_u128 as f64;
			let reserve1_scaled = reserve1_u128 as f64;
			let ratio1 = reserve0_scaled / reserve1_scaled;
			let ratio2 = reserve1_scaled / reserve0_scaled;

			// Try different decimal adjustments to find reasonable price
			let price_options = [
				ratio1 * 1e12,  // reserve0=USDC, reserve1=Token
				ratio2 * 1e-12, // reserve0=Token, reserve1=USDC
				ratio1,         // Same decimals
				ratio2,         // Same decimals inverted
			];

			for price in price_options {
				if price > 1000.0 && price < 20000.0 {
					return Ok(price);
				}
			}

			Err("No reasonable price found on Trader Joe")
		}
	}

	/// Exchange instance combining chain + protocol (address resolved dynamically)
	struct Exchange<C: ChainInterface, P: DexProtocol<C>> {
		rpc_url: &'static str,
		exchange_name: &'static str,
		exchange_id: u8,
		_phantom: core::marker::PhantomData<(C, P)>,
	}

	impl<C: ChainInterface, P: DexProtocol<C>> Exchange<C, P> {
		const fn new(rpc_url: &'static str, exchange_name: &'static str, exchange_id: u8) -> Self {
			Self { rpc_url, exchange_name, exchange_id, _phantom: core::marker::PhantomData }
		}

		fn fetch_price(&self, pair: TokenPair) -> Result<(u64, u64), http::Error> {
			let pool_address = P::get_pool_address(pair);
			let call_data = P::get_call_data(pair);
			let raw_response = C::call_contract(self.rpc_url, pool_address, &call_data)?;

			let price_f64 = P::parse_price(raw_response).map_err(|_| http::Error::Unknown)?;
			let price_micro = (price_f64 * 1_000_000.0) as u64;
			let timestamp = sp_io::offchain::timestamp().unix_millis();

			log::info!("✅ {} | {} | ${:.2}", self.exchange_name, pair.as_str(), price_f64);

			Ok((price_micro, timestamp))
		}
	}

	/// Common interface for all exchanges - enables clean iteration
	trait ExchangeInterface {
		fn fetch_price(&self, pair: TokenPair) -> Result<(u64, u64), http::Error>;
		fn get_exchange_id(&self) -> u8;
		fn get_name(&self) -> &str;
	}

	/// Implement the interface for all Exchange types
	impl<C: ChainInterface, P: DexProtocol<C>> ExchangeInterface for Exchange<C, P> {
		fn fetch_price(&self, pair: TokenPair) -> Result<(u64, u64), http::Error> {
			self.fetch_price(pair)
		}

		fn get_exchange_id(&self) -> u8 {
			self.exchange_id
		}

		fn get_name(&self) -> &str {
			self.exchange_name
		}
	}

	/// Exchange definitions using clean abstraction
	mod exchanges {
		use super::*;

		// Ethereum exchanges
		pub static UNISWAP_ETH: Exchange<EvmChain, UniswapV3Protocol> =
			Exchange::new("https://eth.llamarpc.com", "Uniswap V3 (Ethereum)", 1);

		pub static SUSHISWAP_ETH: Exchange<EvmChain, UniswapV2Protocol> =
			Exchange::new("https://eth.llamarpc.com", "SushiSwap V2 (Ethereum)", 2);

		// BSC exchanges
		pub static PANCAKESWAP_BSC: Exchange<EvmChain, PancakeSwapProtocol> =
			Exchange::new("https://bsc-dataseed.binance.org", "PancakeSwap V2 (BSC)", 3);

		// Polygon exchanges
		pub static QUICKSWAP_POLYGON: Exchange<EvmChain, QuickSwapProtocol> =
			Exchange::new("https://polygon-rpc.com", "QuickSwap V2 (Polygon)", 4);

		// Avalanche exchanges
		pub static TRADERJOE_AVAX: Exchange<EvmChain, TraderJoeProtocol> =
			Exchange::new("https://api.avax.network/ext/bc/C/rpc", "Trader Joe (Avalanche)", 5);

		/// Option 1: Dynamic dispatch (clean, tiny overhead)
		pub fn get_all_exchanges() -> Vec<&'static dyn ExchangeInterface> {
			vec![
				&UNISWAP_ETH,
				&SUSHISWAP_ETH,
				&PANCAKESWAP_BSC,
				&QUICKSWAP_POLYGON,
				&TRADERJOE_AVAX,
			]
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Price updated for a token pair
		PriceUpdated { token_pair: [u8; 32], price: u64, timestamp: u64 },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Price data not found
		PriceNotFound,
	}

	#[pallet::storage]
	#[pallet::getter(fn price_data)]
	pub type PriceData<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		[u8; 32], // Token pair hash
		Blake2_128Concat,
		u8,         // Exchange ID
		(u64, u64), // (price_micro, timestamp)
		OptionQuery,
	>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(block_number: BlockNumberFor<T>) {
			log::info!("Running offchain worker at block: {:?}", block_number);

			// Only run every 10 blocks to avoid spam
			// every 3 block temporarily for testing
			if block_number % 3u32.into() != sp_runtime::traits::Zero::zero() {
				return;
			}

			// Fetch prices from all exchanges for all pairs - TRULY FLEXIBLE!
			let pairs_to_fetch = [TokenPair::EthUsd]; // Start with working pair, expand gradually

			for pair in pairs_to_fetch {
				for exchange in exchanges::get_all_exchanges() {
					if let Err(_e) = Self::fetch_and_store_price(exchange, pair) {
						log::error!("❌ {} | {}", exchange.get_name(), pair.as_str());
					}
				}
			}
		}
	}

	impl<T: Config> Pallet<T> {
		/// Clean function to fetch and store price from any exchange
		fn fetch_and_store_price(
			exchange: &dyn ExchangeInterface,
			pair: TokenPair,
		) -> Result<(), http::Error> {
			let (price_micro, timestamp) = exchange.fetch_price(pair)?;
			let pair_hash = pair.to_hash();

			<PriceData<T>>::insert(
				&pair_hash,
				exchange.get_exchange_id(),
				(price_micro, timestamp),
			);

			<Pallet<T>>::deposit_event(Event::PriceUpdated {
				token_pair: pair_hash,
				price: price_micro,
				timestamp,
			});

			// Price already logged in fetch_price with ✅

			Ok(())
		}

		/// Get price data for a token pair from a specific exchange
		pub fn get_price(pair_hash: [u8; 32], exchange_id: u8) -> Option<(u64, u64)> {
			<PriceData<T>>::get(pair_hash, exchange_id)
		}

		/// Get all prices for a token pair from all exchanges
		pub fn get_all_prices(pair_hash: [u8; 32]) -> BTreeMap<u8, (u64, u64)> {
			let mut prices = BTreeMap::new();

			// Check all known exchange IDs
			for exchange_id in 1..=5 {
				if let Some(price_data) = Self::get_price(pair_hash, exchange_id) {
					prices.insert(exchange_id, price_data);
				}
			}

			prices
		}
	}
}
