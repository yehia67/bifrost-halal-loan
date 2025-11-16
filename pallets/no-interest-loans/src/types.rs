use bifrost_primitives::{Balance, CurrencyId};
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

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
    pub loan_reward: Balance,
	pub created_at: BlockNumber,
	pub status: LoanStatus,
}

// Type alias for cleaner code
pub type LoanPositionOf<T> =
	LoanPosition<<T as frame_system::Config>::AccountId, BlockNumberFor<T>>;
pub trait PriceProvider<CurrencyId> {
	type Price;
	fn get_price(currency_id: &CurrencyId) -> Option<Self::Price>;
}

/// Weight functions trait (can use default weights for POC)
pub trait WeightInfo {
	fn create_loan() -> Weight;
	fn repay_loan() -> Weight;
	fn distribute_cycle_rewards() -> Weight;
	fn liquidate_loan() -> Weight;
}

/// Default weight implementation for POC
impl WeightInfo for () {
	fn create_loan() -> Weight {
		Weight::from_parts(10_000, 0)
	}
	fn repay_loan() -> Weight {
		Weight::from_parts(10_000, 0)
	}
	fn distribute_cycle_rewards() -> Weight {
		Weight::from_parts(10_000, 0)
	}
	fn liquidate_loan() -> Weight {
		Weight::from_parts(10_000, 0)
	}
}
