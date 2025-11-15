#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		dispatch::DispatchResult,
		pallet_prelude::*,
		traits::Get,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::AtLeast32BitUnsigned;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		
		/// Loan ID type
		type LoanId: Parameter + Member + AtLeast32BitUnsigned + Default + Copy + MaxEncodedLen;
		
		/// Maximum number of loans per user
		#[pallet::constant]
		type MaxLoansPerUser: Get<u32>;
		
		/// Weight information for extrinsics
		type WeightInfo: WeightInfo;
	}

	// Simple loan structure
	#[pallet::storage]
	#[pallet::getter(fn loans)]
	pub type Loans<T: Config> = StorageMap<_, Blake2_128Concat, T::LoanId, (T::AccountId, u128)>;

	#[pallet::storage]
	#[pallet::getter(fn next_loan_id)]
	pub type NextLoanId<T: Config> = StorageValue<_, T::LoanId, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A loan was created
		LoanCreated { loan_id: T::LoanId, borrower: T::AccountId, amount: u128 },
		/// A loan was repaid
		LoanRepaid { loan_id: T::LoanId, borrower: T::AccountId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Loan not found
		LoanNotFound,
		/// Not the loan owner
		NotLoanOwner,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a simple loan (placeholder implementation)
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_loan())]
		pub fn create_loan(
			origin: OriginFor<T>,
			amount: u128,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			
			let loan_id = NextLoanId::<T>::get();
			Loans::<T>::insert(&loan_id, (&who, amount));
			NextLoanId::<T>::put(loan_id + T::LoanId::from(1u32));
			
			Self::deposit_event(Event::LoanCreated { loan_id, borrower: who, amount });
			Ok(())
		}

		/// Repay a loan (placeholder implementation)
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::repay_loan())]
		pub fn repay_loan(
			origin: OriginFor<T>,
			loan_id: T::LoanId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			
			let (borrower, _amount) = Loans::<T>::get(&loan_id).ok_or(Error::<T>::LoanNotFound)?;
			ensure!(borrower == who, Error::<T>::NotLoanOwner);
			
			Loans::<T>::remove(&loan_id);
			Self::deposit_event(Event::LoanRepaid { loan_id, borrower: who });
			Ok(())
		}
	}

	// Weight trait (placeholder)
	pub trait WeightInfo {
		fn create_loan() -> Weight;
		fn repay_loan() -> Weight;
	}

	// Default weight implementation
	impl WeightInfo for () {
		fn create_loan() -> Weight {
			Weight::from_parts(10_000, 0)
		}
		fn repay_loan() -> Weight {
			Weight::from_parts(10_000, 0)
		}
	}
}
