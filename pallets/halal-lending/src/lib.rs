#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        dispatch::DispatchResult,
        pallet_prelude::*,
        traits::{Get, ExistenceRequirement},
        PalletId,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::{
        traits::{AtLeast32BitUnsigned, AccountIdConversion},
        Permill,
    };
    use parity_scale_codec::{Encode, Decode, MaxEncodedLen};
    use scale_info::TypeInfo;
    
    // Import Bifrost types
    use bifrost_primitives::{CurrencyId, Balance, OraclePriceProvider, USDC};
    use orml_traits::MultiCurrency;

    // Loan status
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub enum LoanStatus {
        Active,
        Repaid,
        Liquidated,
    }

    // Main loan data structure
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct LoanPosition<AccountId, BlockNumber> {
        pub borrower: AccountId,
        pub collateral_vtoken: CurrencyId,
        pub collateral_amount: Balance,
        pub loan_currency: CurrencyId,
        pub loan_amount: Balance,
        pub created_at: BlockNumber,
        pub status: LoanStatus,
    }
    
    // Note: NO interest, NO fees, NO profit_share_rate!
    // Platform profit comes ONLY from staking rewards on locked vToken collateral
    // Borrower repays exactly what they borrowed - nothing more!

    // Type alias for cleaner code
    pub type LoanPositionOf<T> = LoanPosition<
        <T as frame_system::Config>::AccountId,
        BlockNumberFor<T>
    >;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn loans)]
    pub type Loans<T: Config> = StorageMap<_, Blake2_128Concat, T::LoanId, LoanPositionOf<T>>;

    #[pallet::storage]
    #[pallet::getter(fn user_loans)]
    pub type UserLoans<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, BoundedVec<T::LoanId, ConstU32<100>>>;

    #[pallet::storage]
    #[pallet::getter(fn next_loan_id)]
    pub type NextLoanId<T: Config> = StorageValue<_, T::LoanId, ValueQuery>;


    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        
        /// Multi-currency support (use Bifrost's currencies pallet)
        type MultiCurrency: MultiCurrency<Self::AccountId, CurrencyId = CurrencyId, Balance = Balance>;
        
        /// Price provider for collateral valuation (use Bifrost's prices pallet)
        type PriceProvider: OraclePriceProvider;
        
        /// Loan ID type
        type LoanId: Parameter + Member + AtLeast32BitUnsigned + Default + Copy + MaxEncodedLen;
        
        /// Maximum LTV ratio (e.g., 50%)
        #[pallet::constant]
        type MaxLTV: Get<Permill>;
        
        /// Weight information for extrinsics
        type WeightInfo: WeightInfo;
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
        UserLoans::<T>::try_mutate(&who, |loans| {
            if let Some(ref mut loan_vec) = loans {
                loan_vec.try_push(loan_id).map_err(|_| Error::<T>::TooManyLoans)?;
            } else {
                let mut new_vec = BoundedVec::default();
                new_vec.try_push(loan_id).map_err(|_| Error::<T>::TooManyLoans)?;
                *loans = Some(new_vec);
            }
            Ok::<(), DispatchError>(())
        })?;
        NextLoanId::<T>::put(loan_id.checked_add(&T::LoanId::from(1u32)).ok_or(Error::<T>::ArithmeticOverflow)?);
        
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
    pub fn repay_loan(
        origin: OriginFor<T>,
        loan_id: T::LoanId,
    ) -> DispatchResult {
        let who = ensure_signed(origin)?;
        
        let mut loan = Loans::<T>::get(loan_id).ok_or(Error::<T>::LoanNotFound)?;
        ensure!(loan.borrower == who, Error::<T>::NotLoanOwner);
        ensure!(loan.status == LoanStatus::Active, Error::<T>::LoanNotActive);
        
        // ⚠️ TRUE HALAL: Borrower repays EXACTLY what they borrowed - NO MORE!
        // NO interest, NO fees, NO profit sharing
        // Platform profit comes from staking rewards on the locked vToken collateral
        
        // Transfer loan repayment from user to pallet (exact amount borrowed)
        T::MultiCurrency::transfer(
            loan.loan_currency,  // USDT or whatever they borrowed
            &who,
            &Self::account_id(),
            loan.loan_amount,    // Exact amount - nothing more!
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
        
        Self::deposit_event(Event::LoanRepaid { loan_id, borrower: who });
        Ok(())
    }
    }

    impl<T: Config> Pallet<T> {
        /// Get the account ID of the pallet
        pub fn account_id() -> T::AccountId {
            PalletId(*b"hlallend").into_account_truncating()
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
        /// Loan liquidated [loan_id, borrower, liquidator]
        LoanLiquidated {
            loan_id: T::LoanId,
            borrower: T::AccountId,
            liquidator: T::AccountId,
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
    }

    /// Weight functions trait (can use default weights for POC)
    pub trait WeightInfo {
        fn create_loan() -> Weight;
        fn repay_loan() -> Weight;
    }

    /// Default weight implementation for POC
    impl WeightInfo for () {
        fn create_loan() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn repay_loan() -> Weight {
            Weight::from_parts(10_000, 0)
        }
    }
}