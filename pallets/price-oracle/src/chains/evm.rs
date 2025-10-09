use super::ChainInterface;
use crate::utils::extract_result_data;
use sp_runtime::offchain::http;
use sp_std::{str, vec::Vec};

extern crate alloc;
use alloc::format;

/// EVM-compatible chains (Ethereum, BSC, Polygon, Avalanche, Arbitrum, etc.)
pub struct EvmChain;

impl ChainInterface for EvmChain {
	type RawResponse = Vec<u8>;

	fn call_contract(
		rpc_url: &str,
		address: &str,
		data: &[u8],
		timeout_ms: u64,
	) -> Result<Self::RawResponse, http::Error> {
		// Create JSON-RPC request
		let hex_data = array_bytes::bytes2hex("", data);
		let json_body = format!(
			r#"{{"jsonrpc":"2.0","method":"eth_call","params":[{{"to":"{}","data":"0x{}"}},"latest"],"id":1}}"#,
			address, hex_data
		);

		// Make HTTP request with configurable timeout
		let request = http::Request::post(rpc_url, alloc::vec![json_body]);
		let pending = request
			.add_header("Content-Type", "application/json")
			.deadline(
				sp_io::offchain::timestamp()
					.add(sp_runtime::offchain::Duration::from_millis(timeout_ms)),
			)
			.send()
			.map_err(|_| http::Error::IoError)?;

		let response = pending.wait().map_err(|_| http::Error::DeadlineReached)?;

		if response.code != 200 {
			return Err(http::Error::Unknown);
		}

		let body = response.body().collect::<Vec<u8>>();
		let response_str = sp_std::str::from_utf8(&body).map_err(|_| http::Error::Unknown)?;

		// Parse JSON to extract result hex data using the working logic
		let hex_data = extract_result_data(response_str)?;
		array_bytes::hex2bytes(&hex_data).map_err(|_| {
			log::error!("Failed to decode hex data: {}", hex_data);
			http::Error::Unknown
		})
	}
}

