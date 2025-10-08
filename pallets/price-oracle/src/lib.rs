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
	use sp_std::{vec::Vec, str};

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

			let _ = Self::fetch_ethereum_price();
		}
	}

	impl<T: Config> Pallet<T> {
		fn fetch_ethereum_price() -> Result<(), http::Error> {
			log::info!("Fetching ETH price from CoinGecko...");

			let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(5000));
			let request = http::Request::get(
				"https://api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd",
			);

			let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;
			let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;

			if response.code != 200 {
				log::error!("HTTP error: {}", response.code);
				return Err(http::Error::Unknown);
			}

			let body = response.body().collect::<Vec<u8>>();
			let body_str = str::from_utf8(&body).map_err(|_| http::Error::Unknown)?;

			log::info!("CoinGecko response: {}", body_str);

			// TODO: Parse JSON and store price
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
