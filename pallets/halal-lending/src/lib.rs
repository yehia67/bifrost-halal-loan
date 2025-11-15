#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;

#[polkadot_sdk::frame_support::pallet]
pub mod pallet {
	use polkadot_sdk::{
		frame_support::{
			dispatch::DispatchResult,
			pallet_prelude::*,
			traits::{Get, ExistenceRequirement},
			PalletId,
			weights::Weight,
		},
		frame_system::pallet_prelude::*,
		sp_runtime::{
			traits::{AccountIdConversion, AtLeast32BitUnsigned, Zero, SaturatedConversion},
			FixedU128, Permill, FixedPointNumber,
		},
	};
	use codec::{Encode, Decode, MaxEncodedLen};
	use scale_info::TypeInfo;
	use scale_info::prelude::vec::Vec;

	// Simple types to replace Bifrost dependencies
	pub type Balance = u128;
	pub type CurrencyId = u32;

	// Loan status enum
	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub enum LoanStatus {
		Active,
		Repaid,
		Liquidated,
	}

	// Loan position structure
	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct LoanPosition<AccountId, LoanId> {
		pub borrower: AccountId,
		pub loan_id: LoanId,
		pub collateral_vtoken: CurrencyId,
		pub collateral_amount: Balance,
		pub loan_currency: CurrencyId,
		pub loan_amount: Balance,
		pub status: LoanStatus,
		pub created_at: u32, // block number
	}

	pub type LoanPositionOf<T> = LoanPosition<<T as polkadot_sdk::frame_system::Config>::AccountId, <T as Config>::LoanId>;

	// Simple price provider trait
	pub trait PriceProvider<CurrencyId> {
		type Price;
		fn get_price(currency_id: CurrencyId) -> Option<Self::Price>;
		fn set_price(currency_id: CurrencyId, price: Self::Price) -> DispatchResult;
	}

	// Simple multi-currency trait (simplified version)
	pub trait MultiCurrency<AccountId> {
		type CurrencyId;
		type Balance;

		fn transfer(
			currency_id: Self::CurrencyId,
			from: &AccountId,
			to: &AccountId,
			amount: Self::Balance,
			existence_requirement: ExistenceRequirement,
		) -> DispatchResult;

		fn free_balance(currency_id: Self::CurrencyId, who: &AccountId) -> Self::Balance;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;

		/// Multi-currency support (simplified)
		type MultiCurrency: MultiCurrency<
			Self::AccountId,
			CurrencyId = CurrencyId,
			Balance = Balance,
		>;

		/// Price provider (simplified with setter functionality)
		type PriceProvider: PriceProvider<CurrencyId, Price = FixedU128>;

		/// Loan ID type
		type LoanId: Parameter + Member + AtLeast32BitUnsigned + Default + Copy + MaxEncodedLen;

		/// Maximum LTV ratio (e.g., 50%)
		#[pallet::constant]
		type MaxLTV: Get<Permill>;

		/// Maximum LTV ratio before liquidation (e.g., 75%)
		#[pallet::constant]
		type LiquidationThreshold: Get<Permill>;

		/// Liquidation bonus for liquidators (e.g., 5%)
		#[pallet::constant]
		type LiquidationBonus: Get<Permill>;

		/// Platform's share of staking rewards (e.g., 30%)
		#[pallet::constant]
		type StakingRewardFee: Get<Permill>;

		/// Treasury account that receives platform revenue
		type TreasuryAccount: Get<Self::AccountId>;

		/// Pallet ID for account derivation
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Weight information for extrinsics
		type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	#[pallet::getter(fn loans)]
	pub type Loans<T: Config> = StorageMap<_, Blake2_128Concat, T::LoanId, LoanPositionOf<T>>;

	#[pallet::storage]
	#[pallet::getter(fn user_loans)]
	pub type UserLoans<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BoundedVec<T::LoanId, ConstU32<100>>>;

	#[pallet::storage]
	#[pallet::getter(fn next_loan_id)]
	pub type NextLoanId<T: Config> = StorageValue<_, T::LoanId, ValueQuery>;

	/// Track rewards for each loan
	#[pallet::storage]
	#[pallet::getter(fn loan_rewards)]
	pub type LoanRewards<T: Config> =
		StorageMap<_, Blake2_128Concat, T::LoanId, Balance, ValueQuery>;

	/// Price storage (for the simplified price provider)
	#[pallet::storage]
	#[pallet::getter(fn prices)]
	pub type Prices<T: Config> = StorageMap<_, Blake2_128Concat, CurrencyId, FixedU128>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A loan was created
		LoanCreated {
			loan_id: T::LoanId,
			borrower: T::AccountId,
			collateral_vtoken: CurrencyId,
			collateral_amount: Balance,
			loan_currency: CurrencyId,
			loan_amount: Balance,
		},
		/// A loan was repaid
		LoanRepaid {
			loan_id: T::LoanId,
			borrower: T::AccountId,
		},
		/// A loan was liquidated
		LoanLiquidated {
			loan_id: T::LoanId,
			borrower: T::AccountId,
			liquidator: T::AccountId,
		},
		/// Rewards were claimed from a loan
		RewardsClaimed {
			loan_id: T::LoanId,
			total_rewards: Balance,
			platform_share: Balance,
			user_share: Balance,
		},
		/// Price was set for a currency
		PriceSet {
			currency_id: CurrencyId,
			price: FixedU128,
		},
		/// Rewards were added to a loan
		RewardsAdded {
			loan_id: T::LoanId,
			amount: Balance,
		},
		/// Cycle rewards were distributed to active loans
		CycleRewardsDistributed {
			total_rewards: Balance,
			loans_count: u32,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Loan not found
		LoanNotFound,
		/// Not the loan owner
		NotLoanOwner,
		/// Loan not active
		LoanNotActive,
		/// Insufficient collateral
		InsufficientCollateral,
		/// LTV too high
		LTVTooHigh,
		/// Price not available
		PriceNotAvailable,
		/// Loan already repaid
		LoanAlreadyRepaid,
		/// Cannot liquidate healthy loan
		CannotLiquidateHealthyLoan,
		/// Arithmetic overflow
		ArithmeticOverflow,
	}

	// Helper functions implementation
	impl<T: Config> Pallet<T> {
		/// Get the account ID of the pallet
		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}

		/// Calculate current LTV ratio for a loan
		pub fn calculate_ltv(loan_id: T::LoanId) -> Result<Permill, DispatchError> {
			let loan = Loans::<T>::get(loan_id).ok_or(Error::<T>::LoanNotFound)?;

			// Get current collateral value in loan currency
			let collateral_value = Self::get_collateral_value(
				loan.collateral_vtoken,
				loan.collateral_amount,
				loan.loan_currency,
			)?;

			// LTV = (loan_amount / collateral_value) * 100%
			let ltv = Permill::from_rational(loan.loan_amount, collateral_value);

			Ok(ltv)
		}

		/// Get collateral value in terms of loan currency
		pub fn get_collateral_value(
			collateral_currency: CurrencyId,
			collateral_amount: Balance,
			target_currency: CurrencyId,
		) -> Result<Balance, DispatchError> {
			if collateral_currency == target_currency {
				return Ok(collateral_amount);
			}

			let collateral_price = T::PriceProvider::get_price(collateral_currency)
				.ok_or(Error::<T>::PriceNotAvailable)?;
			let target_price = T::PriceProvider::get_price(target_currency)
				.ok_or(Error::<T>::PriceNotAvailable)?;

			// Calculate value: (collateral_amount * collateral_price) / target_price
			let collateral_value_usd = collateral_price.saturating_mul_int(collateral_amount);
			let target_value = target_price.reciprocal()
				.ok_or(Error::<T>::ArithmeticOverflow)?
				.saturating_mul_int(collateral_value_usd);

			Ok(target_value)
		}

		/// Claim accumulated staking rewards from locked collateral
		/// This is the CORE halal lending mechanism - distributes staking rewards!
		pub fn claim_loan_rewards(loan_id: T::LoanId) -> DispatchResult {
			let loan = Loans::<T>::get(loan_id).ok_or(Error::<T>::LoanNotFound)?;

			// Only claim from active loans
			ensure!(loan.status == LoanStatus::Active, Error::<T>::LoanAlreadyRepaid);

			let rewards = LoanRewards::<T>::get(loan_id);

			// If no rewards, return early
			if rewards == 0 {
				return Ok(());
			}

			// Calculate platform's share (e.g., 30%)
			let platform_share = T::StakingRewardFee::get().mul_floor(rewards);

			// Calculate rewards earned by loanee (rewards - platform_share)
			let loanee_rewards = rewards.saturating_sub(platform_share);

			// Transfer Loanee rewards to loanee
			T::MultiCurrency::transfer(
				loan.collateral_vtoken,
				&Self::account_id(),
				&loan.borrower,
				loanee_rewards,
				ExistenceRequirement::AllowDeath,
			)?;

			// Transfer platform's share to treasury
			T::MultiCurrency::transfer(
				loan.collateral_vtoken,
				&Self::account_id(),
				&T::TreasuryAccount::get(),
				platform_share,
				ExistenceRequirement::AllowDeath,
			)?;

			// Emit event
			Self::deposit_event(Event::RewardsClaimed {
				loan_id,
				total_rewards: rewards,
				platform_share,
				user_share: loanee_rewards,
			});

			// Clear rewards after distribution
			LoanRewards::<T>::remove(loan_id);

			Ok(())
		}

		/// Add rewards to a loan (simulates staking rewards accumulation)
		pub fn add_loan_rewards(loan_id: T::LoanId, amount: Balance) -> DispatchResult {
			ensure!(Loans::<T>::contains_key(loan_id), Error::<T>::LoanNotFound);
			
			LoanRewards::<T>::mutate(loan_id, |rewards| {
				*rewards = rewards.saturating_add(amount);
			});

			Self::deposit_event(Event::RewardsAdded { loan_id, amount });
			Ok(())
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a halal loan
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_loan())]
		pub fn create_loan(
			origin: OriginFor<T>,
			collateral_vtoken: CurrencyId,
			collateral_amount: Balance,
			loan_currency: CurrencyId,
			loan_amount: Balance,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Validate LTV ratio
			let collateral_value = Self::get_collateral_value(
				collateral_vtoken,
				collateral_amount,
				loan_currency,
			)?;

			let ltv = Permill::from_rational(loan_amount, collateral_value);
			ensure!(ltv <= T::MaxLTV::get(), Error::<T>::LTVTooHigh);

			// Transfer collateral to pallet account
			T::MultiCurrency::transfer(
				collateral_vtoken,
				&who,
				&Self::account_id(),
				collateral_amount,
				ExistenceRequirement::AllowDeath,
			)?;

			// Create loan
			let loan_id = NextLoanId::<T>::get();
			let loan = LoanPosition {
				borrower: who.clone(),
				loan_id,
				collateral_vtoken,
				collateral_amount,
				loan_currency,
				loan_amount,
				status: LoanStatus::Active,
				created_at: polkadot_sdk::frame_system::Pallet::<T>::block_number().saturated_into(),
			};

			Loans::<T>::insert(&loan_id, &loan);
			NextLoanId::<T>::put(loan_id + T::LoanId::from(1u32));

			// Add to user loans
			UserLoans::<T>::try_mutate(&who, |loans| -> DispatchResult {
				let mut user_loans = loans.take().unwrap_or_default();
				user_loans.try_push(loan_id).map_err(|_| Error::<T>::ArithmeticOverflow)?;
				*loans = Some(user_loans);
				Ok(())
			})?;

			// Transfer loan amount to borrower
			T::MultiCurrency::transfer(
				loan_currency,
				&Self::account_id(),
				&who,
				loan_amount,
				ExistenceRequirement::AllowDeath,
			)?;

			Self::deposit_event(Event::LoanCreated {
				loan_id,
				borrower: who,
				collateral_vtoken,
				collateral_amount,
				loan_currency,
				loan_amount,
			});

			Ok(())
		}

		/// Repay a loan
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::repay_loan())]
		pub fn repay_loan(
			origin: OriginFor<T>,
			loan_id: T::LoanId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let mut loan = Loans::<T>::get(&loan_id).ok_or(Error::<T>::LoanNotFound)?;
			ensure!(loan.borrower == who, Error::<T>::NotLoanOwner);
			ensure!(loan.status == LoanStatus::Active, Error::<T>::LoanAlreadyRepaid);

			// Claim rewards before repayment
			Self::claim_loan_rewards(loan_id)?;

			// Transfer loan amount back to pallet
			T::MultiCurrency::transfer(
				loan.loan_currency,
				&who,
				&Self::account_id(),
				loan.loan_amount,
				ExistenceRequirement::AllowDeath,
			)?;

			// Return collateral to borrower
			T::MultiCurrency::transfer(
				loan.collateral_vtoken,
				&Self::account_id(),
				&who,
				loan.collateral_amount,
				ExistenceRequirement::AllowDeath,
			)?;

			// Update loan status
			loan.status = LoanStatus::Repaid;
			Loans::<T>::insert(&loan_id, &loan);

			Self::deposit_event(Event::LoanRepaid {
				loan_id,
				borrower: who,
			});

			Ok(())
		}

		/// Set price for a currency (admin function)
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::set_price())]
		pub fn set_price(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			price: FixedU128,
		) -> DispatchResult {
			ensure_root(origin)?;

			T::PriceProvider::set_price(currency_id, price)?;

			Self::deposit_event(Event::PriceSet { currency_id, price });
			Ok(())
		}

		/// Add rewards to a loan (admin function to simulate staking rewards)
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::add_rewards())]
		pub fn add_rewards(
			origin: OriginFor<T>,
			loan_id: T::LoanId,
			amount: Balance,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::add_loan_rewards(loan_id, amount)?;
			Ok(())
		}

		/// Distribute cycle rewards proportionally to all active loans
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::distribute_cycle_rewards())]
		pub fn distribute_cycle_rewards(
			origin: OriginFor<T>, 
			cycle_rewards: Balance
		) -> DispatchResult {
			ensure_root(origin)?;

			let active_loans: Vec<_> = Loans::<T>::iter()
				.filter(|(_, loan)| loan.status == LoanStatus::Active)
				.collect();

			if active_loans.is_empty() {
				return Ok(());
			}

			let total_asset_value: Balance = active_loans
				.iter()
				.map(|(loan_id, loan)| {
					loan.collateral_amount
						.saturating_add(LoanRewards::<T>::get(loan_id))
				})
				.sum();

			let loans_count = active_loans.len() as u32;

			for (loan_id, loan) in active_loans {
				let loan_asset_value = loan.collateral_amount
					.saturating_add(LoanRewards::<T>::get(loan_id));
				
				let proportional_rewards = if total_asset_value > 0 {
					cycle_rewards
						.saturating_mul(loan_asset_value)
						.saturating_div(total_asset_value)
				} else {
					0
				};

				if proportional_rewards > 0 {
					LoanRewards::<T>::mutate(loan_id, |rewards| {
						*rewards = rewards.saturating_add(proportional_rewards);
					});
				}
			}

			Self::deposit_event(Event::CycleRewardsDistributed {
				total_rewards: cycle_rewards,
				loans_count,
			});

			Ok(())
		}

		/// Liquidate an unhealthy loan
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::liquidate_loan())]
		pub fn liquidate_loan(
			origin: OriginFor<T>,
			loan_id: T::LoanId,
		) -> DispatchResult {
			let liquidator = ensure_signed(origin)?;

			let mut loan = Loans::<T>::get(&loan_id).ok_or(Error::<T>::LoanNotFound)?;
			ensure!(loan.status == LoanStatus::Active, Error::<T>::LoanNotActive);

			// Check if loan is unhealthy
			let ltv = Self::calculate_ltv(loan_id)?;
			ensure!(ltv >= T::LiquidationThreshold::get(), Error::<T>::CannotLiquidateHealthyLoan);

			// Claim rewards before liquidation
			Self::claim_loan_rewards(loan_id)?;

			// Calculate liquidation bonus
			let bonus_amount = T::LiquidationBonus::get().mul_floor(loan.collateral_amount);
			let remaining_collateral = loan.collateral_amount.saturating_sub(bonus_amount);

			// Transfer bonus to liquidator
			T::MultiCurrency::transfer(
				loan.collateral_vtoken,
				&Self::account_id(),
				&liquidator,
				bonus_amount,
				ExistenceRequirement::AllowDeath,
			)?;

			// Return remaining collateral to borrower
			if !remaining_collateral.is_zero() {
				T::MultiCurrency::transfer(
					loan.collateral_vtoken,
					&Self::account_id(),
					&loan.borrower,
					remaining_collateral,
					ExistenceRequirement::AllowDeath,
				)?;
			}

			// Update loan status
			loan.status = LoanStatus::Liquidated;
			Loans::<T>::insert(&loan_id, &loan);

			Self::deposit_event(Event::LoanLiquidated {
				loan_id,
				borrower: loan.borrower,
				liquidator,
			});

			Ok(())
		}
	}

	// Simple price provider implementation
	impl<T: Config> PriceProvider<CurrencyId> for Pallet<T> {
		type Price = FixedU128;

		fn get_price(currency_id: CurrencyId) -> Option<Self::Price> {
			Prices::<T>::get(currency_id)
		}

		fn set_price(currency_id: CurrencyId, price: Self::Price) -> DispatchResult {
			Prices::<T>::insert(currency_id, price);
			Ok(())
		}
	}

	// Weight trait
	pub trait WeightInfo {
		fn create_loan() -> Weight;
		fn repay_loan() -> Weight;
		fn set_price() -> Weight;
		fn add_rewards() -> Weight;
		fn distribute_cycle_rewards() -> Weight;
		fn liquidate_loan() -> Weight;
	}

	// Default weight implementation
	impl WeightInfo for () {
		fn create_loan() -> Weight {
			Weight::from_parts(50_000, 0)
		}
		fn repay_loan() -> Weight {
			Weight::from_parts(50_000, 0)
		}
		fn set_price() -> Weight {
			Weight::from_parts(10_000, 0)
		}
		fn add_rewards() -> Weight {
			Weight::from_parts(20_000, 0)
		}
		fn distribute_cycle_rewards() -> Weight {
			Weight::from_parts(100_000, 0)
		}
		fn liquidate_loan() -> Weight {
			Weight::from_parts(60_000, 0)
		}
	}
}
