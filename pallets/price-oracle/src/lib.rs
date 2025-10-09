#![cfg_attr(not(feature = "std"), no_std)]

// Re-export all modules
pub mod chains;
pub mod exchanges;
pub mod protocols;
pub mod types;
pub mod utils;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use crate::{
		exchanges::{registry, ExchangeInterface},
		types::TokenPair,
	};
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_runtime::offchain::http;
	use sp_std::collections::btree_map::BTreeMap;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// How often to update prices (in blocks)
		#[pallet::constant]
		type UpdateInterval: Get<BlockNumberFor<Self>>;

		/// HTTP request timeout in milliseconds
		#[pallet::constant]
		type HttpTimeout: Get<u64>;

		/// Maximum exchanges to query per block
		#[pallet::constant]
		type MaxExchangesPerBlock: Get<u8>;

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

			// Use configurable update interval
			if block_number % T::UpdateInterval::get() != sp_runtime::traits::Zero::zero() {
				return;
			}

			// Fetch prices from all exchanges for all pairs - TRULY FLEXIBLE!
			let pairs_to_fetch = [TokenPair::EthUsd]; 

			for pair in pairs_to_fetch {
				let exchanges = registry::get_all_exchanges();
				let max_exchanges = T::MaxExchangesPerBlock::get() as usize;
				let exchanges_to_query = if exchanges.len() <= max_exchanges {
					exchanges
				} else {
					// Take first N exchanges (could be randomized later)
					exchanges.into_iter().take(max_exchanges).collect()
				};

				for exchange in exchanges_to_query {
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
			let timeout_ms = T::HttpTimeout::get();
			let (min_price, max_price) = pair.get_price_bounds();

			let (price_micro, timestamp) =
				exchange.fetch_price(pair, timeout_ms, min_price, max_price)?;
			let pair_hash = pair.to_hash();

			<PriceData<T>>::insert(
				pair_hash,
				exchange.get_exchange_id(),
				(price_micro, timestamp),
			);

			<Pallet<T>>::deposit_event(Event::PriceUpdated {
				token_pair: pair_hash,
				price: price_micro,
				timestamp,
			});

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
