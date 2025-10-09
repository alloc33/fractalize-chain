//! # Exchange Management
//!
//! Combines chain interfaces and DEX protocols into concrete exchange instances.
//! Provides a unified interface for iterating over all supported exchanges.

use crate::{
	chains::{ChainInterface, EvmChain},
	protocols::{
		DexProtocol, PancakeSwapProtocol, QuickSwapProtocol, TraderJoeProtocol, UniswapV2Protocol,
		UniswapV3Protocol,
	},
	types::TokenPair,
};
use sp_io;
use sp_runtime::offchain::http;

/// Exchange instance combining chain + protocol (address resolved dynamically)
pub struct Exchange<C: ChainInterface, P: DexProtocol<C>> {
	rpc_url: &'static str,
	exchange_name: &'static str,
	exchange_id: u8,
	_phantom: core::marker::PhantomData<(C, P)>,
}

impl<C: ChainInterface, P: DexProtocol<C>> Exchange<C, P> {
	pub const fn new(rpc_url: &'static str, exchange_name: &'static str, exchange_id: u8) -> Self {
		Self { rpc_url, exchange_name, exchange_id, _phantom: core::marker::PhantomData }
	}

	pub fn fetch_price(
		&self,
		pair: TokenPair,
		timeout_ms: u64,
		min_price: u64,
		max_price: u64,
	) -> Result<(u64, u64), http::Error> {
		let pool_address = P::get_pool_address(pair);
		let call_data = P::get_call_data(pair);
		let raw_response = C::call_contract(self.rpc_url, pool_address, &call_data, timeout_ms)?;

		let price_f64 = P::parse_price(raw_response).map_err(|_| http::Error::Unknown)?;
		let price_micro = (price_f64 * 1_000_000.0) as u64;

		// Validate price is within acceptable bounds
		if price_micro < min_price || price_micro > max_price {
			log::error!(
				"Price out of bounds: {} (min: {}, max: {})",
				price_micro,
				min_price,
				max_price
			);
			return Err(http::Error::Unknown);
		}

		let timestamp = sp_io::offchain::timestamp().unix_millis();

		log::info!("✅ {} | {} | ${:.2}", self.exchange_name, pair.as_str(), price_f64);

		Ok((price_micro, timestamp))
	}
}

/// Common interface for all exchanges - enables clean iteration
pub trait ExchangeInterface {
	fn fetch_price(
		&self,
		pair: TokenPair,
		timeout_ms: u64,
		min_price: u64,
		max_price: u64,
	) -> Result<(u64, u64), http::Error>;
	fn get_exchange_id(&self) -> u8;
	fn get_name(&self) -> &str;
}

/// Implement the interface for all Exchange types
impl<C: ChainInterface, P: DexProtocol<C>> ExchangeInterface for Exchange<C, P> {
	fn fetch_price(
		&self,
		pair: TokenPair,
		timeout_ms: u64,
		min_price: u64,
		max_price: u64,
	) -> Result<(u64, u64), http::Error> {
		self.fetch_price(pair, timeout_ms, min_price, max_price)
	}

	fn get_exchange_id(&self) -> u8 {
		self.exchange_id
	}

	fn get_name(&self) -> &str {
		self.exchange_name
	}
}

/// Exchange definitions using clean abstraction
pub mod registry {
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

	/// Get all exchanges for iteration - clean and scalable!
	pub fn get_all_exchanges() -> sp_std::vec::Vec<&'static dyn ExchangeInterface> {
		sp_std::vec![
			&UNISWAP_ETH,
			&SUSHISWAP_ETH,
			&PANCAKESWAP_BSC,
			&QUICKSWAP_POLYGON,
			&TRADERJOE_AVAX,
		]
	}
}
