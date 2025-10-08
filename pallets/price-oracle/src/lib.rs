#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use frame_support::{pallet_prelude::*, Blake2_128Concat};
	use frame_system::pallet_prelude::{BlockNumberFor, *};
	use sp_runtime::offchain::{http, Duration};
	use sp_std::{str, vec, vec::Vec};
	extern crate alloc;
	use alloc::{
		format,
		string::{String, ToString},
	};
	use alloy_core::primitives::U256;
	use alloy_primitives::hex;

	/// Token pair enum for type safety
	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
	pub enum TokenPair {
		EthUsd,
		BtcUsd,
	}

	impl TokenPair {
		pub fn as_str(&self) -> &'static str {
			match self {
				Self::EthUsd => "ETH/USD",
				Self::BtcUsd => "BTC/USD",
			}
		}

		pub fn hash(&self) -> [u8; 32] {
			sp_io::hashing::blake2_256(self.as_str().as_bytes())
		}
	}

	/// DEX contract addresses
	struct DexContracts;
	impl DexContracts {
		/// Uniswap V3 ETH/USDC pool (0.3% fee tier)
		const UNISWAP_ETH_USDC: &'static str = "0x8ad599c3a0ff1de082011efddc58f1908eb6e6d8";
		/// SushiSwap V2 ETH/USDC pool
		const SUSHISWAP_ETH_USDC: &'static str = "0x397ff1542f962076d0bfe58ea045ffa2d347aca0";
		/// PancakeSwap V2 ETH/USDC pool (BSC)
		const PANCAKESWAP_ETH_USDC: &'static str = "0xea26b78255df2bbc31c1ebf60010d78670185bd0";
		/// QuickSwap V2 ETH/USDC pool (Polygon)
		const QUICKSWAP_ETH_USDC: &'static str = "0x853ee4b2a13f8a742d64c8f088be7ba2131f670d";
	}

	/// Generic trait for fetching prices from DEX contracts
	trait DexPriceFetcher {
		const EXCHANGE_ID: u8;
		const RPC_URL: &'static str;

		/// Get contract address for token pair
		fn get_contract_address(pair: TokenPair) -> Option<&'static str>;

		/// Parse price from contract call result
		fn parse_contract_price(data: &[u8]) -> Result<f64, &'static str>;

		/// Get exchange name for logging
		fn exchange_name() -> &'static str;

		/// Get function selector for price query (slot0() for V3, getReserves() for V2)
		fn get_function_selector() -> [u8; 4];

		/// Fetch price via direct contract call
		fn fetch_price(pair: TokenPair) -> Result<(u64, u64), http::Error> {
			log::info!(
				"Fetching {} from {} via contract call...",
				pair.as_str(),
				Self::exchange_name()
			);

			let contract_addr = Self::get_contract_address(pair).ok_or(http::Error::Unknown)?;

			// Get the appropriate function selector for this exchange
			let call_data = Self::get_function_selector();

			let rpc_request = format!(
				"{{\"jsonrpc\":\"2.0\",\"method\":\"eth_call\",\"params\":[{{\"to\":\"{}\",\"data\":\"0x{}\"}},\"latest\"],\"id\":1}}",
				contract_addr,
				hex::encode(&call_data)
			);

			let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(10000));
			let request = http::Request::post(Self::RPC_URL, vec![rpc_request.into_bytes()])
				.add_header("Content-Type", "application/json");
			let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;
			let response =
				pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;

			if response.code != 200 {
				log::error!("RPC error from {}: {}", Self::exchange_name(), response.code);
				return Err(http::Error::Unknown);
			}

			let body = response.body().collect::<Vec<u8>>();
			let body_str = str::from_utf8(&body).map_err(|_| http::Error::Unknown)?;

			log::info!("{} contract response: {}", Self::exchange_name(), body_str);

			// Parse RPC response to get contract data
			let hex_data = Self::extract_result_data(body_str)?;
			let contract_data = hex::decode(&hex_data).map_err(|_| http::Error::Unknown)?;

			let price =
				Self::parse_contract_price(&contract_data).map_err(|_| http::Error::Unknown)?;
			let price_micros = (price * 1_000_000.0) as u64;
			let timestamp = sp_io::offchain::timestamp().unix_millis();

			log::info!(
				"Got {} price from {}: ${} ({}μ)",
				pair.as_str(),
				Self::exchange_name(),
				price,
				price_micros
			);

			Ok((price_micros, timestamp))
		}

		/// Extract hex data from JSON RPC response
		fn extract_result_data(json: &str) -> Result<String, http::Error> {
			// Simple JSON parsing to get "result" field
			if let Some(start) = json.find("\"result\":\"") {
				let data_start = start + 10; // Skip '"result":"'
				if let Some(end) = json[data_start..].find('"') {
					let hex_data = &json[data_start..data_start + end];
					if hex_data.starts_with("0x") {
						return Ok(hex_data[2..].to_string());
					}
				}
			}
			Err(http::Error::Unknown)
		}
	}

	// ============= DEX Contract Implementations =============

	struct UniswapV3Fetcher;
	impl DexPriceFetcher for UniswapV3Fetcher {
		const EXCHANGE_ID: u8 = 1;
		const RPC_URL: &'static str = "https://eth.llamarpc.com";

		fn get_contract_address(pair: TokenPair) -> Option<&'static str> {
			match pair {
				TokenPair::EthUsd => Some(DexContracts::UNISWAP_ETH_USDC),
				_ => None,
			}
		}

		fn parse_contract_price(data: &[u8]) -> Result<f64, &'static str> {
			if data.len() < 32 {
				return Err("Invalid contract response length");
			}

			// Extract sqrtPriceX96 from slot0() response (first 32 bytes)
			let sqrt_price_x96_bytes = &data[0..32];
			let sqrt_price_x96 = U256::from_be_slice(sqrt_price_x96_bytes);

			// Convert sqrtPriceX96 to ETH price in USD
			// For Uniswap V3 ETH/USDC pool 0x8ad599c3a0ff1de082011efddc58f1908eb6e6d8:
			// token0 = USDC (6 decimals), token1 = ETH (18 decimals)
			
			// Convert to f64 for calculation
			let sqrt_price_f64 = sqrt_price_x96.to::<u128>() as f64;
			let q96_f64 = (1u128 << 96) as f64;
			
			// Calculate price = (sqrtPrice / 2^96)^2
			let sqrt_ratio = sqrt_price_f64 / q96_f64;
			let price_ratio = sqrt_ratio * sqrt_ratio;
			
			// For this specific pool, we need to account for token order and decimals
			// This gives us token1/token0 = ETH/USDC ratio
			// Multiply by 10^(6-18) = 10^-12 to account for decimal difference
			let eth_per_usdc = price_ratio * 1e-12;
			
			// Convert to USD per ETH
			let usd_per_eth = 1.0 / eth_per_usdc;

			if usd_per_eth > 1000.0 && usd_per_eth < 20000.0 {
				Ok(usd_per_eth)
			} else {
				Err("Price out of reasonable range")
			}
		}

		fn exchange_name() -> &'static str {
			"Uniswap V3"
		}

		fn get_function_selector() -> [u8; 4] {
			hex!("3850c7bd") // slot0() selector
		}
	}

	struct SushiswapFetcher;
	impl DexPriceFetcher for SushiswapFetcher {
		const EXCHANGE_ID: u8 = 2;
		const RPC_URL: &'static str = "https://eth.llamarpc.com";

		fn get_contract_address(pair: TokenPair) -> Option<&'static str> {
			match pair {
				TokenPair::EthUsd => Some(DexContracts::SUSHISWAP_ETH_USDC),
				_ => None,
			}
		}

		fn parse_contract_price(data: &[u8]) -> Result<f64, &'static str> {
			// SushiSwap V2 uses getReserves() which returns (reserve0, reserve1, blockTimestampLast)
			if data.len() < 96 {
				return Err("Invalid SushiSwap V2 response length");
			}

			// Extract reserves (first two 32-byte values)
			let reserve0_bytes = &data[0..32];
			let reserve1_bytes = &data[32..64];
			
			let reserve0 = U256::from_be_slice(reserve0_bytes).to::<u128>() as f64;
			let reserve1 = U256::from_be_slice(reserve1_bytes).to::<u128>() as f64;

			// For SushiSwap V2 ETH/USDC pool 0x397ff1542f962076d0bfe58ea045ffa2d347aca0:
			// Need to check which token is token0 vs token1
			// Assuming USDC is token0 (6 decimals) and WETH is token1 (18 decimals)
			
			// Calculate price = reserve0 / reserve1 * 10^(18-6)
			if reserve1 == 0.0 {
				return Err("Zero liquidity in SushiSwap pool");
			}
			
			let price_ratio = reserve0 / reserve1;
			let usd_per_eth = price_ratio * 1e12; // Adjust for decimal difference

			if usd_per_eth > 1000.0 && usd_per_eth < 20000.0 {
				Ok(usd_per_eth)
			} else {
				Err("SushiSwap price out of reasonable range")
			}
		}

		fn exchange_name() -> &'static str {
			"SushiSwap V2"
		}

		fn get_function_selector() -> [u8; 4] {
			hex!("0902f1ac") // getReserves() selector
		}
	}

	struct PancakeswapFetcher;
	impl DexPriceFetcher for PancakeswapFetcher {
		const EXCHANGE_ID: u8 = 3;
		const RPC_URL: &'static str = "https://bsc-dataseed.binance.org";

		fn get_contract_address(pair: TokenPair) -> Option<&'static str> {
			match pair {
				TokenPair::EthUsd => Some(DexContracts::PANCAKESWAP_ETH_USDC),
				_ => None,
			}
		}

		fn parse_contract_price(data: &[u8]) -> Result<f64, &'static str> {
			// PancakeSwap V2 uses same getReserves() interface as SushiSwap V2
			if data.len() < 96 {
				return Err("Invalid PancakeSwap V2 response length");
			}

			// Extract reserves (first two 32-byte values)
			let reserve0_bytes = &data[0..32];
			let reserve1_bytes = &data[32..64];
			
			let reserve0_u256 = U256::from_be_slice(reserve0_bytes);
			let reserve1_u256 = U256::from_be_slice(reserve1_bytes);

			// Convert to u128 first to avoid overflow
			let reserve0_u128 = reserve0_u256.to::<u128>();
			let reserve1_u128 = reserve1_u256.to::<u128>();
			
			if reserve0_u128 == 0 || reserve1_u128 == 0 {
				return Err("Zero liquidity in PancakeSwap pool");
			}

			// Handle large numbers by scaling down before f64 conversion
			// Assuming one is ETH (18 decimals) and one is USDC (6 decimals)
			let reserve0_scaled = reserve0_u128 as f64;
			let reserve1_scaled = reserve1_u128 as f64;
			
			// Calculate both possible price ratios
			let ratio1 = reserve0_scaled / reserve1_scaled;
			let ratio2 = reserve1_scaled / reserve0_scaled;
			
			// Apply decimal adjustments and check which gives reasonable ETH price
			let price1 = ratio1 * 1e12; // If reserve0=USDC, reserve1=ETH
			let price2 = ratio2 * 1e-12; // If reserve0=ETH, reserve1=USDC
			let price3 = ratio1; // Same decimals
			let price4 = ratio2; // Same decimals inverted
			
			// Find the price that's in reasonable ETH range
			for price in [price1, price2, price3, price4] {
				if price > 1000.0 && price < 20000.0 {
					return Ok(price);
				}
			}
			
			Err("PancakeSwap: No reasonable ETH price found")
		}

		fn exchange_name() -> &'static str {
			"PancakeSwap V2 (BSC)"
		}

		fn get_function_selector() -> [u8; 4] {
			hex!("0902f1ac") // getReserves() selector
		}
	}

	struct QuickswapFetcher;
	impl DexPriceFetcher for QuickswapFetcher {
		const EXCHANGE_ID: u8 = 4;
		const RPC_URL: &'static str = "https://polygon-rpc.com";

		fn get_contract_address(pair: TokenPair) -> Option<&'static str> {
			match pair {
				TokenPair::EthUsd => Some(DexContracts::QUICKSWAP_ETH_USDC),
				_ => None,
			}
		}

		fn parse_contract_price(data: &[u8]) -> Result<f64, &'static str> {
			// QuickSwap V2 uses same getReserves() interface as other V2 DEXs
			if data.len() < 96 {
				return Err("Invalid QuickSwap V2 response length");
			}

			// Extract reserves (first two 32-byte values)
			let reserve0_bytes = &data[0..32];
			let reserve1_bytes = &data[32..64];
			
			let reserve0_u256 = U256::from_be_slice(reserve0_bytes);
			let reserve1_u256 = U256::from_be_slice(reserve1_bytes);

			let reserve0_u128 = reserve0_u256.to::<u128>();
			let reserve1_u128 = reserve1_u256.to::<u128>();
			
			if reserve0_u128 == 0 || reserve1_u128 == 0 {
				return Err("Zero liquidity in QuickSwap pool");
			}

			// Convert to f64 for calculations
			let reserve0_scaled = reserve0_u128 as f64;
			let reserve1_scaled = reserve1_u128 as f64;
			
			// Calculate both possible price ratios
			let ratio1 = reserve0_scaled / reserve1_scaled;
			let ratio2 = reserve1_scaled / reserve0_scaled;
			
			// Try different decimal adjustments to find reasonable ETH price
			let price_options = [
				ratio1 * 1e12,  // reserve0=USDC, reserve1=ETH
				ratio2 * 1e-12, // reserve0=ETH, reserve1=USDC  
				ratio1,         // Same decimals
				ratio2,         // Same decimals inverted
			];
			
			// Find the price that's in reasonable ETH range
			for price in price_options {
				if price > 1000.0 && price < 20000.0 {
					return Ok(price);
				}
			}
			
			Err("QuickSwap: No reasonable ETH price found")
		}

		fn exchange_name() -> &'static str {
			"QuickSwap V2 (Polygon)"
		}

		fn get_function_selector() -> [u8; 4] {
			hex!("0902f1ac") // getReserves() selector
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

	/// Storage for token prices from different exchanges
	///
	/// This storage maps (exchange_id, token_pair_hash) → (price, timestamp) to:
	/// - Track prices from multiple DEXs (Uniswap=1, SushiSwap=2, etc.)
	/// - Compare prices across exchanges to find arbitrage opportunities
	/// - Store timestamps to ensure price data freshness
	#[pallet::storage]
	pub type TokenPrices<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		(u8, [u8; 32]), // (exchange_id, token_pair_hash)
		(u64, u64),     // (price, timestamp)
		OptionQuery,
	>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(block_number: BlockNumberFor<T>) {
			log::info!("Running offchain worker at block: {:?}", block_number);

			// Only fetch every 10 block {~2 minutes} to avoid rate limits
			if block_number % 10u32.into() != sp_runtime::traits::Zero::zero() {
				return;
			}

			// Fetch prices from REAL DEXs via direct contract calls
			if let Err(e) = Self::fetch_and_store_price::<UniswapV3Fetcher>(TokenPair::EthUsd) {
				log::error!("Uniswap V3 fetch failed: {:?}", e);
			}
			if let Err(e) = Self::fetch_and_store_price::<SushiswapFetcher>(TokenPair::EthUsd) {
				log::error!("SushiSwap fetch failed: {:?}", e);
			}
			if let Err(e) = Self::fetch_and_store_price::<PancakeswapFetcher>(TokenPair::EthUsd) {
				log::error!("PancakeSwap fetch failed: {:?}", e);
			}
			if let Err(e) = Self::fetch_and_store_price::<QuickswapFetcher>(TokenPair::EthUsd) {
				log::error!("QuickSwap fetch failed: {:?}", e);
			}
		}
	}

	impl<T: Config> Pallet<T> {
		/// Generic function to fetch and store price using any DexPriceFetcher implementation
		fn fetch_and_store_price<F: DexPriceFetcher>(
			token_pair: TokenPair,
		) -> Result<(), http::Error> {
			log::info!("Fetching {} price from {}...", token_pair.as_str(), F::exchange_name());

			// Fetch price using the trait implementation
			let (price_micros, timestamp) = F::fetch_price(token_pair)?;

			// Store the price (this will be committed when the block is imported)
			TokenPrices::<T>::insert(
				(F::EXCHANGE_ID, token_pair.hash()),
				(price_micros, timestamp),
			);

			log::info!(
				"Stored {} price from {}: {}μ at timestamp {}",
				token_pair.as_str(),
				F::exchange_name(),
				price_micros,
				timestamp
			);

			Ok(())
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(10_000)]
		pub fn set_price(origin: OriginFor<T>, token_pair: [u8; 32], price: u64) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			Self::deposit_event(Event::PriceUpdated {
				token_pair,
				price,
				timestamp: 0, // TODO: Add real timestamp
			});

			Ok(())
		}
	}
}
