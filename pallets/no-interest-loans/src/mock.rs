// Mock runtime for testing no-interest-loans pallet

use crate as pallet_no_interest_loans;
use crate::pallet::{Balance, CurrencyId};
use polkadot_sdk::{
    frame_support::{
        dispatch::DispatchResult,
        parameter_types,
        traits::{ConstU32, Everything, ExistenceRequirement},
        PalletId,
    },
    frame_system as system,
    sp_core::H256,
    sp_runtime::{
        traits::{BlakeTwo256, IdentityLookup},
        BuildStorage, FixedU128, Permill,
    },
};
use std::collections::BTreeMap;

type Block = system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet
polkadot_sdk::frame_support::construct_runtime!(
    pub enum Test
    {
        System: system,
        Balances: polkadot_sdk::pallet_balances,
        NoInterestLoans: pallet_no_interest_loans,
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
    type AccountData = polkadot_sdk::pallet_balances::AccountData<Balance>;
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

// Balances pallet configuration
parameter_types! {
    pub const ExistentialDeposit: Balance = 1;
}

impl polkadot_sdk::pallet_balances::Config for Test {
    type MaxLocks = ConstU32<50>;
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type RuntimeHoldReason = ();
    type RuntimeFreezeReason = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ();
    type DoneSlashHandler = ();
}

// Mock price provider with dynamic price support for liquidation testing

thread_local! {
    static PRICES: std::cell::RefCell<BTreeMap<CurrencyId, FixedU128>> = std::cell::RefCell::new(BTreeMap::new());
}

pub struct MockPriceProvider;

impl MockPriceProvider {
    pub fn set_price(currency_id: CurrencyId, price: FixedU128) {
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
    type Price = FixedU128;

    fn get_price(currency_id: CurrencyId) -> Option<Self::Price> {
        PRICES
            .with(|p| p.borrow().get(&currency_id).copied())
            .or_else(|| {
                // Default price of 1.0 if not set
                Some(FixedU128::from_inner(1_000_000_000_000_000_000))
            })
    }

    fn set_price(currency_id: CurrencyId, price: Self::Price) -> DispatchResult {
        Self::set_price(currency_id, price);
        Ok(())
    }
}

// Simple multi-currency implementation for testing
pub struct MockMultiCurrency;
impl crate::pallet::MultiCurrency<u64> for MockMultiCurrency {
    type CurrencyId = CurrencyId;
    type Balance = Balance;

    fn transfer(
        _currency_id: Self::CurrencyId,
        from: &u64,
        to: &u64,
        amount: Self::Balance,
        _existence_requirement: ExistenceRequirement,
    ) -> DispatchResult {
        // For simplicity, use the native token (Balances pallet) for all currencies
        <Balances as polkadot_sdk::frame_support::traits::fungible::Mutate<u64>>::transfer(
            from,
            to,
            amount,
            polkadot_sdk::frame_support::traits::tokens::Preservation::Expendable,
        )
        .map(|_| ())
    }

    fn free_balance(_currency_id: Self::CurrencyId, who: &u64) -> Self::Balance {
        Balances::free_balance(who)
    }
}

// Halal Lending configuration
parameter_types! {
    pub const MaxLTV: Permill = Permill::from_percent(50); // 50% max LTV for borrowing
    pub const LiquidationThreshold: Permill = Permill::from_percent(75); // 75% triggers liquidation
    pub const LiquidationBonus: Permill = Permill::from_percent(5); // 5% bonus for liquidators
    pub const StakingRewardFee: Permill = Permill::from_percent(30); // 30%
    pub const TreasuryAccount: u64 = 999; // Treasury account ID
    pub const NoInterestLoansPalletId: PalletId = PalletId(*b"hlallend");
}

impl pallet_no_interest_loans::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type MultiCurrency = MockMultiCurrency;
    type PriceProvider = MockPriceProvider;
    type LoanId = u64;
    type MaxLTV = MaxLTV;
    type LiquidationThreshold = LiquidationThreshold;
    type LiquidationBonus = LiquidationBonus;
    type WeightInfo = ();
    type StakingRewardFee = StakingRewardFee;
    type TreasuryAccount = TreasuryAccount;
    type PalletId = NoInterestLoansPalletId;
}

// Mock currency IDs for testing
pub const MOCK_VTOKEN: CurrencyId = 1; // Mock vToken
pub const MOCK_USDC: CurrencyId = 2; // Mock USDC

// Test accounts
pub const ALICE: u64 = 1;
pub const BOB: u64 = 2;
pub const TREASURY: u64 = 999;

// Build genesis storage according to the mock runtime
pub fn new_test_ext() -> polkadot_sdk::sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    // Initialize balances using the Balances pallet
    polkadot_sdk::pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            // Give Alice 10,000 tokens
            (ALICE, 10_000),
            // Give Bob 5,000 tokens
            (BOB, 5_000),
            // Give the pallet 100,000 tokens to lend out
            (NoInterestLoans::account_id(), 100_000),
            // Give treasury some tokens
            (TREASURY, 1_000),
        ],
        dev_accounts: None,
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = polkadot_sdk::sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}
