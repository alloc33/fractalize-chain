use crate::utils::extract_result_data;

#[test]
fn extract_result_data_valid_json() {
	// Test valid JSON-RPC response
	let valid_response = r#"{"jsonrpc":"2.0","result":"0x1234567890abcdef","id":1}"#;
	let result = extract_result_data(valid_response);
	assert!(result.is_ok(), "Should extract data from valid JSON");
	assert_eq!(result.unwrap(), "1234567890abcdef");
}

#[test]
fn extract_result_data_with_0x_prefix() {
	// Test response where result already has 0x prefix
	let response_with_prefix = r#"{"jsonrpc":"2.0","result":"0x1234567890abcdef","id":1}"#;
	let result = extract_result_data(response_with_prefix);
	assert!(result.is_ok(), "Should handle 0x prefix correctly");
	assert_eq!(result.unwrap(), "1234567890abcdef");
}

#[test]
fn extract_result_data_without_0x_prefix() {
	// Test response where result doesn't have 0x prefix
	let response_without_prefix = r#"{"jsonrpc":"2.0","result":"1234567890abcdef","id":1}"#;
	let result = extract_result_data(response_without_prefix);
	assert!(result.is_ok(), "Should handle missing 0x prefix");
	assert_eq!(result.unwrap(), "1234567890abcdef");
}

#[test]
fn extract_result_data_invalid_json() {
	// Test malformed JSON that doesn't have the expected pattern
	let invalid_json = r#"{"jsonrpc":"2.0","result""#;
	let result = extract_result_data(invalid_json);
	assert!(result.is_err(), "Should reject malformed JSON");
}

#[test]
fn extract_result_data_missing_result_field() {
	// Test JSON without result field
	let missing_result = r#"{"jsonrpc":"2.0","error":"Something went wrong","id":1}"#;
	let result = extract_result_data(missing_result);
	assert!(result.is_err(), "Should reject JSON without result field");
}

#[test]
fn extract_result_data_null_result() {
	// Test JSON with null result
	let null_result = r#"{"jsonrpc":"2.0","result":null,"id":1}"#;
	let result = extract_result_data(null_result);
	assert!(result.is_err(), "Should reject null result");
}

#[test]
fn extract_result_data_empty_result() {
	// Test JSON with empty result
	let empty_result = r#"{"jsonrpc":"2.0","result":"","id":1}"#;
	let result = extract_result_data(empty_result);
	assert!(result.is_err(), "Should reject empty result");
}

#[test]
fn extract_result_data_non_string_result() {
	// Test JSON with non-string result
	let number_result = r#"{"jsonrpc":"2.0","result":12345,"id":1}"#;
	let result = extract_result_data(number_result);
	assert!(result.is_err(), "Should reject non-string result");
}

#[test]
fn extract_result_data_various_hex_lengths() {
	// Test various valid hex string lengths
	let test_cases = [
		(r#"{"jsonrpc":"2.0","result":"0x12","id":1}"#, "12"),
		(r#"{"jsonrpc":"2.0","result":"0x1234","id":1}"#, "1234"),
		(r#"{"jsonrpc":"2.0","result":"0x123456789abcdef0","id":1}"#, "123456789abcdef0"),
	];

	for (input, expected) in test_cases {
		let result = extract_result_data(input);
		assert!(result.is_ok(), "Should handle hex string: {}", input);
		assert_eq!(result.unwrap(), expected);
	}
}

#[test]
fn extract_result_data_real_world_examples() {
	// Test with realistic Ethereum RPC responses

	// Typical Uniswap V3 slot0() response
	let uniswap_v3_response = r#"{"jsonrpc":"2.0","id":1,"result":"0x000000000000000000000000000000000001c9c380000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"}"#;
	let result = extract_result_data(uniswap_v3_response);
	assert!(result.is_ok(), "Should parse real Uniswap V3 response");
	let hex_data = result.unwrap();
	assert!(hex_data.len() > 100, "Should return substantial hex data");

	// Typical Uniswap V2 getReserves() response
	let uniswap_v2_response = r#"{"jsonrpc":"2.0","id":1,"result":"0x000000000000000000000000000000000000000000000000000000174876e800000000000000000000000000000000000000000000000013da329b633647000000000000000000000000000000000000000000000000000000000000000065a0a0a0"}"#;
	let result = extract_result_data(uniswap_v2_response);
	assert!(result.is_ok(), "Should parse real Uniswap V2 response");
	let hex_data = result.unwrap();
	assert!(hex_data.len() > 100, "Should return substantial hex data");
}

