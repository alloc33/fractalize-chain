use sp_runtime::offchain::http;
extern crate alloc;
use alloc::string::{String, ToString};

/// Extract hex data from JSON RPC response
pub fn extract_result_data(json: &str) -> Result<String, http::Error> {
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

