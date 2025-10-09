//! # Chain Interfaces
//!
//! Abstractions for different blockchain networks (EVM, Solana, etc.).
//! Each chain implements the ChainInterface trait for smart contract calls.

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
		timeout_ms: u64,
	) -> Result<Self::RawResponse, http::Error>;
}