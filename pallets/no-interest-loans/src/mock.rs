// Mock runtime for testing no-interest-loans pallet

use crate as pallet_halal_lending;
use bifrost_primitives::{Balance, CurrencyId};
use frame_support::{
	parameter_types,
	traits::{ConstU32, Everything},
};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage, Permill,
};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Tokens: orml_tokens,
		NoInterestLoans: pallet_halal_lending,
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

impl system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
	type RuntimeTask = ();
	type SingleBlockMigrations = ();
	type MultiBlockMigrator = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = ();
	type ExtensionsWeightInfo = ();
}

// orml-tokens configuration
parameter_types! {
	pub const MaxLocks: u32 = 50;
}

pub struct ExistentialDeposits;
impl orml_traits::GetByKey<CurrencyId, Balance> for ExistentialDeposits {
	fn get(_: &CurrencyId) -> Balance {
		0 // No existential deposit for mock tokens
	}
}

impl orml_tokens::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type Amount = i128;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type CurrencyHooks = ();
	type MaxLocks = MaxLocks;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type DustRemovalWhitelist = Everything;
}

// Mock price provider with dynamic price support for liquidation testing
use std::cell::RefCell;
use std::collections::BTreeMap;

thread_local! {
	static PRICES: RefCell<BTreeMap<CurrencyId, sp_runtime::FixedU128>> = RefCell::new(BTreeMap::new());
}

pub struct MockPriceProvider;

impl MockPriceProvider {
	pub fn set_price(currency_id: CurrencyId, price: sp_runtime::FixedU128) {
		PRICES.with(|p| {
			p.borrow_mut().insert(currency_id, price);
		});
	}

	pub fn reset_prices() {
		PRICES.with(|p| {
			p.borrow_mut().clear();
		});
	}
}

// Implement PriceProvider trait for MockPriceProvider
impl crate::pallet::PriceProvider<CurrencyId> for MockPriceProvider {
	type Price = sp_runtime::FixedU128;

	fn get_price(currency_id: &CurrencyId) -> Option<Self::Price> {
		PRICES
			.with(|p| p.borrow().get(currency_id).copied())
			.or_else(|| {
				// Default price of 1.0 if not set
				Some(sp_runtime::FixedU128::from_inner(1_000_000_000_000_000_000))
			})
	}
}

// No interest Lending configuration
parameter_types! {
	pub const MaxLTV: Permill = Permill::from_percent(50); // 50% max LTV for borrowing
	pub const LiquidationThreshold: Permill = Permill::from_percent(75); // 75% triggers liquidation
	pub const LiquidationBonus: Permill = Permill::from_percent(5); // 5% bonus for liquidators
	pub const StakingRewardFee: Permill = Permill::from_percent(30); // 30%
	pub const TreasuryAccount: u64 = 999; // Treasury account ID
}

impl pallet_halal_lending::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = Tokens;
	type PriceProvider = MockPriceProvider;
	type LoanId = u64;
	type MaxLTV = MaxLTV;
	type LiquidationThreshold = LiquidationThreshold;
	type LiquidationBonus = LiquidationBonus;
	type WeightInfo = ();
	type StakingRewardFee = StakingRewardFee;
	type TreasuryAccount = TreasuryAccount;
}

// Mock currency IDs for testing
pub const MOCK_VTOKEN: CurrencyId = CurrencyId::VToken2(99); // Mock vToken
pub const MOCK_USDC: CurrencyId = CurrencyId::Token2(98); // Mock USDC

// Test accounts
pub const ALICE: u64 = 1;
pub const BOB: u64 = 2;
// Build genesis storage according to the mock runtime
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = system::GenesisConfig::<Test>::default()
		.build_storage()
		.unwrap();

	let pallet_account = NoInterestLoans::account_id();

	orml_tokens::GenesisConfig::<Test> {
		balances: vec![
			// Give Alice 10,000 mock vTokens
			(ALICE, MOCK_VTOKEN, 10_000),
			// Give Bob 5,000 mock vTokens
			(BOB, MOCK_VTOKEN, 5_000),
			// Give the pallet 100,000 mock USDC to lend out
			(pallet_account, MOCK_USDC, 100_000),
			// Give the pallet some vTokens to cover liquidation bonuses
			(pallet_account, MOCK_VTOKEN, 1_000),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

// Add treasury account
pub const TREASURY: u64 = 999;
