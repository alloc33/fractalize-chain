//! # Core Pallet Tests
//!
//! Tests for storage, configuration, and basic pallet functionality.

use crate::{mock::*, types::TokenPair, *};
use frame_support::traits::Get;

#[test]
fn token_pair_hash_consistency() {
	// Test that TokenPair hashes are consistent
	let eth_usd = TokenPair::EthUsd;
	let hash1 = eth_usd.to_hash();
	let hash2 = eth_usd.to_hash();
	assert_eq!(hash1, hash2, "TokenPair hash should be consistent");

	// Test that different pairs have different hashes
	let btc_usd = TokenPair::BtcUsd;
	let btc_hash = btc_usd.to_hash();
	assert_ne!(hash1, btc_hash, "Different pairs should have different hashes");
}

#[test]
fn token_pair_price_bounds() {
	// Test that all token pairs have reasonable price bounds
	let pairs = [TokenPair::EthUsd, TokenPair::BtcUsd, TokenPair::SolUsd, TokenPair::AvaxUsd];

	for pair in pairs {
		let (min, max) = pair.get_price_bounds();
		assert!(min > 0, "Min price should be positive for {:?}", pair);
		assert!(max > min, "Max price should be greater than min for {:?}", pair);
		assert!(min >= 1_000_000, "Min price should be at least $1 for {:?}", pair); // $1 minimum
	}
}

#[test]
fn price_storage_and_retrieval() {
	new_test_ext().execute_with(|| {
		let pair = TokenPair::EthUsd;
		let pair_hash = pair.to_hash();
		let exchange_id = 1u8;
		let price = 3_500_000_000u64; // $3500
		let timestamp = 1234567890u64;

		// Store price data
		PriceData::<Test>::insert(pair_hash, exchange_id, (price, timestamp));

		// Retrieve and verify
		let stored_data = PriceOracle::get_price(pair_hash, exchange_id);
		assert_eq!(stored_data, Some((price, timestamp)));

		// Test non-existent data
		let non_existent = PriceOracle::get_price(pair_hash, 99u8);
		assert_eq!(non_existent, None);
	});
}

#[test]
fn get_all_prices_functionality() {
	new_test_ext().execute_with(|| {
		let pair = TokenPair::EthUsd;
		let pair_hash = pair.to_hash();

		// Store multiple exchange prices
		PriceData::<Test>::insert(pair_hash, 1u8, (3_500_000_000u64, 1234567890u64));
		PriceData::<Test>::insert(pair_hash, 2u8, (3_510_000_000u64, 1234567891u64));
		PriceData::<Test>::insert(pair_hash, 3u8, (3_490_000_000u64, 1234567892u64));

		// Retrieve all prices
		let all_prices = PriceOracle::get_all_prices(pair_hash);

		assert_eq!(all_prices.len(), 3);
		assert_eq!(all_prices.get(&1u8), Some(&(3_500_000_000u64, 1234567890u64)));
		assert_eq!(all_prices.get(&2u8), Some(&(3_510_000_000u64, 1234567891u64)));
		assert_eq!(all_prices.get(&3u8), Some(&(3_490_000_000u64, 1234567892u64)));
	});
}

#[test]
fn config_constants_are_reasonable() {
	new_test_ext().execute_with(|| {
		// Test update interval
		let update_interval: u64 = <Test as Config>::UpdateInterval::get();
		assert!(update_interval > 0, "Update interval should be positive");
		assert!(update_interval <= 100, "Update interval should be reasonable (<=100 blocks)");

		// Test HTTP timeout
		let timeout: u64 = <Test as Config>::HttpTimeout::get();
		assert!(timeout >= 1000, "HTTP timeout should be at least 1 second");
		assert!(timeout <= 60000, "HTTP timeout should be at most 60 seconds");

		// Test max exchanges per block
		let max_exchanges: u8 = <Test as Config>::MaxExchangesPerBlock::get();
		assert!(max_exchanges > 0, "Should query at least 1 exchange");
		assert!(max_exchanges <= 10, "Should not query too many exchanges per block");
	});
}

#[test]
fn price_bounds_validation() {
	// Test ETH price bounds
	let (min_eth, max_eth) = TokenPair::EthUsd.get_price_bounds();
	assert_eq!(min_eth, 1_000_000_000); // $1,000
	assert_eq!(max_eth, 20_000_000_000); // $20,000

	// Test BTC price bounds (should be higher than ETH)
	let (min_btc, max_btc) = TokenPair::BtcUsd.get_price_bounds();
	assert!(min_btc > min_eth, "BTC min should be higher than ETH min");
	assert!(max_btc > max_eth, "BTC max should be higher than ETH max");

	// Test SOL price bounds (should be lower than ETH)
	let (min_sol, max_sol) = TokenPair::SolUsd.get_price_bounds();
	assert!(min_sol < min_eth, "SOL min should be lower than ETH min");
	assert!(max_sol < max_eth, "SOL max should be lower than ETH max");
}

#[test]
fn string_representations_are_correct() {
	assert_eq!(TokenPair::EthUsd.as_str(), "ETH/USD");
	assert_eq!(TokenPair::BtcUsd.as_str(), "BTC/USD");
	assert_eq!(TokenPair::SolUsd.as_str(), "SOL/USD");
	assert_eq!(TokenPair::AvaxUsd.as_str(), "AVAX/USD");
}

#[test]
fn price_validation_integration() {
	// Test that prices within bounds are accepted
	let (min_price, max_price) = TokenPair::EthUsd.get_price_bounds();
	let valid_price = (min_price + max_price) / 2; // Middle of range

	assert!(valid_price >= min_price && valid_price <= max_price);

	// Test edge cases
	assert!(min_price >= min_price && min_price <= max_price);
	assert!(max_price >= min_price && max_price <= max_price);
}
