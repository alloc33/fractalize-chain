pub mod dex_specific;
mod uniswap_v2;
mod uniswap_v3;

pub use dex_specific::*;
pub use uniswap_v2::*;
pub use uniswap_v3::*;

use crate::{chains::ChainInterface, types::TokenPair};
use sp_std::vec::Vec;

/// DEX protocol abstraction (independent of chain)
pub trait DexProtocol<C: ChainInterface> {
	/// Get pool address for a specific trading pair
	fn get_pool_address(pair: TokenPair) -> &'static str;

	/// Get function call data for price query
	fn get_call_data(pair: TokenPair) -> Vec<u8>;

	/// Parse raw response to extract price
	fn parse_price(response: C::RawResponse) -> Result<f64, &'static str>;
}
