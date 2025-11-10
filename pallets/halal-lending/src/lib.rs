#![cfg_attr(not(feature = "std"), no_std)]

mod types;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	pub use crate::types::{LoanPosition, LoanPositionOf, LoanStatus, PriceProvider, WeightInfo};
	use frame_support::{
		dispatch::DispatchResult,
		pallet_prelude::*,
		traits::{ExistenceRequirement, Get},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use parity_scale_codec::MaxEncodedLen;
	use sp_runtime::{
		traits::{AccountIdConversion, AtLeast32BitUnsigned},
		FixedPointNumber, FixedU128, Permill,
	};

	// Import Bifrost types
	use bifrost_primitives::{Balance, CurrencyId};
	use orml_traits::MultiCurrency;

	// Note: NO interest, NO fees, NO profit_share_rate!
	// Platform profit comes ONLY from staking rewards on locked vToken collateral
	// Borrower repays exactly what they borrowed - nothing more!

	#[pallet::pallet]
	pub struct Pallet<T>(_);

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

	/// Track original collateral amount (before rewards)
	#[pallet::storage]
	#[pallet::getter(fn original_collateral)]
	pub type OriginalCollateral<T: Config> =
		StorageMap<_, Blake2_128Concat, T::LoanId, Balance, ValueQuery>;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Multi-currency support (use Bifrost's currencies pallet)
		type MultiCurrency: MultiCurrency<
			Self::AccountId,
			CurrencyId = CurrencyId,
			Balance = Balance,
		>;

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

		/// Weight information for extrinsics
		type WeightInfo: WeightInfo;

		#[pallet::constant]
		type StakingRewardFee: Get<Permill>;

		/// Treasury account that receives platform revenue
		type TreasuryAccount: Get<Self::AccountId>;
	}

	// Helper functions implementation
	impl<T: Config> Pallet<T> {
		/// Calculate current LTV ratio for a loan
		/// Returns LTV as Permill (e.g., 750000 = 75%)
		pub fn calculate_ltv(loan_id: T::LoanId) -> Result<Permill, DispatchError> {
			let loan = Loans::<T>::get(loan_id).ok_or(Error::<T>::LoanNotFound)?;

			// Get current collateral value in loan currency
			let collateral_value = Self::get_collateral_value(
				loan.collateral_vtoken,
				loan.collateral_amount,
				loan.loan_currency,
			)?;

			// LTV = (loan_amount / collateral_value) * 100%
			let ltv: Permill = Permill::from_rational(loan.loan_amount, collateral_value);

			Ok(ltv)
		}

		/// Get collateral value in terms of loan currency
		/// Example: 1000 vDOT worth how much USDC?
		pub fn get_collateral_value(
			collateral_currency: CurrencyId,
			collateral_amount: Balance,
			loan_currency: CurrencyId,
		) -> Result<Balance, DispatchError> {
			// Get price of collateral in USD (or base currency)
			let collateral_price = T::PriceProvider::get_price(&collateral_currency)
				.ok_or(Error::<T>::PriceNotAvailable)?;

			// Get price of loan currency in USD
			let loan_price = T::PriceProvider::get_price(&loan_currency)
				.ok_or(Error::<T>::PriceNotAvailable)?;

			// Calculate collateral value in loan currency
			// value = (collateral_amount * collateral_price) / loan_price
			let collateral_value_usd =
				collateral_price.checked_mul_int(collateral_amount).ok_or(Error::<T>::ArithmeticOverflow)?;

			let value = loan_price
				.reciprocal()
				.and_then(|reciprocal| reciprocal.checked_mul_int(collateral_value_usd))
				.ok_or(Error::<T>::ArithmeticOverflow)?;

			Ok(value)
		}

		/// Check if loan is eligible for liquidation
		pub fn is_liquidatable(loan_id: T::LoanId) -> Result<bool, DispatchError> {
			let loan = Loans::<T>::get(loan_id).ok_or(Error::<T>::LoanNotFound)?;

			// Only active loans can be liquidated
			if loan.status != LoanStatus::Active {
				return Ok(false);
			}

			let current_ltv = Self::calculate_ltv(loan_id)?;
			let threshold = T::LiquidationThreshold::get();

			Ok(current_ltv >= threshold)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a new loan by depositing vToken collateral
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_loan())]
		pub fn create_loan(
			origin: OriginFor<T>,
			vtoken_id: CurrencyId,
			collateral_amount: Balance,
			loan_currency: CurrencyId,
			loan_amount: Balance,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Get next loan ID
			let loan_id = NextLoanId::<T>::get();
			ensure!(
				Self::is_valid_vtoken_collateral(vtoken_id),
				Error::<T>::InvalidCollateralCurrency
			);
			// Transfer vToken collateral from user to pallet
			T::MultiCurrency::transfer(
				vtoken_id,
				&who,
				&Self::account_id(),
				collateral_amount,
				ExistenceRequirement::AllowDeath,
			)?;

			// Create loan position
			let loan = LoanPosition {
				borrower: who.clone(),
				collateral_vtoken: vtoken_id,
				collateral_amount,
				loan_currency,
				loan_amount,
				created_at: <frame_system::Pallet<T>>::block_number(),
				status: LoanStatus::Active,
			};

			// Store loan
			Loans::<T>::insert(loan_id, loan);

			OriginalCollateral::<T>::insert(loan_id, collateral_amount);

			UserLoans::<T>::try_mutate(&who, |loans| {
				if let Some(ref mut loan_vec) = loans {
					loan_vec
						.try_push(loan_id)
						.map_err(|_| Error::<T>::TooManyLoans)?;
				} else {
					let mut new_vec = BoundedVec::default();
					new_vec
						.try_push(loan_id)
						.map_err(|_| Error::<T>::TooManyLoans)?;
					*loans = Some(new_vec);
				}
				Ok::<(), DispatchError>(())
			})?;
			NextLoanId::<T>::put(
				loan_id
					.checked_add(&T::LoanId::from(1u32))
					.ok_or(Error::<T>::ArithmeticOverflow)?,
			);

			// Transfer loan currency to borrower
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
				collateral_currency: vtoken_id,
				collateral_amount,
				loan_currency,
				loan_amount,
			});
			Ok(())
		}

		/// Repay loan and withdraw collateral
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::repay_loan())]
		pub fn repay_loan(origin: OriginFor<T>, loan_id: T::LoanId) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let mut loan = Loans::<T>::get(loan_id).ok_or(Error::<T>::LoanNotFound)?;
			ensure!(loan.borrower == who, Error::<T>::NotLoanOwner);
			ensure!(loan.status == LoanStatus::Active, Error::<T>::LoanNotActive);

			// ⚠️ TRUE HALAL: Borrower repays EXACTLY what they borrowed - NO MORE!
			// NO interest, NO fees, NO profit sharing
			// Platform profit comes from staking rewards on the locked vToken collateral

			// Transfer loan repayment from user to pallet (exact amount borrowed)
			T::MultiCurrency::transfer(
				loan.loan_currency, // USDT or whatever they borrowed
				&who,
				&Self::account_id(),
				loan.loan_amount, // Exact amount - nothing more!
				ExistenceRequirement::AllowDeath,
			)?;

			// Return collateral to user
			T::MultiCurrency::transfer(
				loan.collateral_vtoken,
				&Self::account_id(),
				&who,
				loan.collateral_amount,
				ExistenceRequirement::AllowDeath,
			)?;

			// Update loan status
			loan.status = LoanStatus::Repaid;
			Loans::<T>::insert(loan_id, loan);

			Self::deposit_event(Event::LoanRepaid {
				loan_id,
				borrower: who,
			});
			Ok(())
		}

		/// Claim accumulated staking rewards from locked collateral
		/// Can be called by anyone (typically automated)
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::claim_rewards())]
		pub fn claim_staking_rewards(origin: OriginFor<T>, loan_id: T::LoanId) -> DispatchResult {
			ensure_signed(origin)?; // Anyone can trigger

			let loan = Loans::<T>::get(loan_id).ok_or(Error::<T>::LoanNotFound)?;

			// Only claim from active loans
			ensure!(loan.status == LoanStatus::Active, Error::<T>::LoanNotActive);

			// Get current vDOT balance (includes accumulated rewards)
			let current_balance =
				T::MultiCurrency::free_balance(loan.collateral_vtoken, &Self::account_id());

			// Get original collateral amount
			let original_amount = OriginalCollateral::<T>::get(loan_id);

			// Calculate rewards earned (current - original)
			let rewards = current_balance.saturating_sub(original_amount);

			// Only proceed if there are rewards to claim
			ensure!(rewards > 0, Error::<T>::NoRewardsToClaim);

			// Calculate platform's share (e.g., 30%)
			let platform_share = T::StakingRewardFee::get().mul_floor(rewards);

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
				user_share: rewards.saturating_sub(platform_share),
			});

			Ok(())
		}

		/// Liquidate an under-collateralized loan
		/// Anyone can call this to liquidate unhealthy loans
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::liquidate_loan())]
		pub fn liquidate_loan(
			origin: OriginFor<T>,
			loan_id: T::LoanId,
		) -> DispatchResult {
			let liquidator = ensure_signed(origin)?;

			// Get loan details
			let mut loan = Loans::<T>::get(loan_id).ok_or(Error::<T>::LoanNotFound)?;

			// Verify loan is active
			ensure!(loan.status == LoanStatus::Active, Error::<T>::LoanNotActive);

			// Check if loan is eligible for liquidation
			ensure!(Self::is_liquidatable(loan_id)?, Error::<T>::LoanNotLiquidatable);

			// Calculate liquidation bonus
			let bonus = T::LiquidationBonus::get().mul_floor(loan.collateral_amount);
			let liquidator_reward =
				loan.collateral_amount.checked_add(bonus).ok_or(Error::<T>::ArithmeticOverflow)?;

			// Liquidator pays off the loan
			T::MultiCurrency::transfer(
				loan.loan_currency,
				&liquidator,
				&Self::account_id(),
				loan.loan_amount,
				ExistenceRequirement::AllowDeath,
			)?;

			// Liquidator receives collateral + bonus
			T::MultiCurrency::transfer(
				loan.collateral_vtoken,
				&Self::account_id(),
				&liquidator,
				liquidator_reward,
				ExistenceRequirement::AllowDeath,
			)?;

			// Update loan status
			loan.status = LoanStatus::Liquidated;
			Loans::<T>::insert(loan_id, loan.clone());

			// Emit event
			Self::deposit_event(Event::LoanLiquidated {
				loan_id,
				borrower: loan.borrower,
				liquidator: liquidator.clone(),
				collateral_liquidated: liquidator_reward,
				debt_covered: loan.loan_amount,
			});

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Get the account ID of the pallet
		pub fn account_id() -> T::AccountId {
			PalletId(*b"hlallend").into_account_truncating()
		}

		/// Check if a currency is a valid vToken for collateral
		pub fn is_valid_vtoken_collateral(currency_id: CurrencyId) -> bool {
			matches!(
				currency_id,
				CurrencyId::VToken(_)
					| CurrencyId::VToken2(_)
					| CurrencyId::VSToken(_)
					| CurrencyId::VSToken2(_)
			)
		}

		/// Get the underlying token for a vToken
		/// Example: vDOT (VToken2(0)) → DOT (Token2(0))
		pub fn get_underlying_token(vtoken: CurrencyId) -> Option<CurrencyId> {
			match vtoken {
				CurrencyId::VToken2(id) => Some(CurrencyId::Token2(id)),
				CurrencyId::VToken(symbol) => Some(CurrencyId::Token(symbol)),
				_ => None,
			}
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Loan created [loan_id, borrower, collateral_amount, loan_amount]
		LoanCreated {
			loan_id: T::LoanId,
			borrower: T::AccountId,
			collateral_currency: CurrencyId,
			collateral_amount: Balance,
			loan_currency: CurrencyId,
			loan_amount: Balance,
		},
		/// Loan repaid [loan_id, borrower]
		LoanRepaid {
			loan_id: T::LoanId,
			borrower: T::AccountId,
		},
		/// Loan liquidated [loan_id, borrower, liquidator, collateral_liquidated, debt_covered]
		LoanLiquidated {
			loan_id: T::LoanId,
			borrower: T::AccountId,
			liquidator: T::AccountId,
			collateral_liquidated: Balance,
			debt_covered: Balance,
		},

		/// Staking rewards claimed [loan_id, total_rewards, platform_share, user_share]
		RewardsClaimed {
			loan_id: T::LoanId,
			total_rewards: Balance,
			platform_share: Balance,
			user_share: Balance,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Loan not found
		LoanNotFound,
		/// Caller is not the loan owner
		NotLoanOwner,
		/// Loan is not active
		LoanNotActive,
		/// Insufficient collateral for requested loan
		InsufficientCollateral,
		/// Price not available from oracle
		PriceNotAvailable,
		/// LTV ratio too high
		LTVTooHigh,
		/// Arithmetic overflow
		ArithmeticOverflow,
		/// Too many loans for user
		TooManyLoans,
		/// Collateral must be a valid vToken
		InvalidCollateralCurrency,
		/// No rewards available to claim
		NoRewardsToClaim,
		/// Loan is not eligible for liquidation (health factor is good)
		LoanNotLiquidatable,
	}
}
