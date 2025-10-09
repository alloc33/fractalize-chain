mod evm;

pub use evm::*;

use sp_runtime::offchain::http;

/// Chain-specific interface for contract calls
pub trait ChainInterface {
	type RawResponse;

	/// Make a contract call and return raw response
	fn call_contract(
		rpc_url: &str,
		address: &str,
		data: &[u8],
	) -> Result<Self::RawResponse, http::Error>;
}