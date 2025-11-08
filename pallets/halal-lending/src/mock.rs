// Mock runtime for testing halal-lending pallet

use crate as pallet_halal_lending;
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
use bifrost_primitives::{CurrencyId, Balance, MockOraclePriceProvider};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet
frame_support::construct_runtime!(
    pub enum Test
    {
        System: frame_system,
        Tokens: orml_tokens,
        HalalLending: pallet_halal_lending,
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

// Halal Lending configuration
parameter_types! {
    pub const MaxLTV: Permill = Permill::from_percent(50);
}

impl pallet_halal_lending::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type MultiCurrency = Tokens;
    type PriceProvider = MockOraclePriceProvider;
    type LoanId = u64;
    type MaxLTV = MaxLTV;
    type WeightInfo = ();
}

// Mock currency IDs for testing
pub const MOCK_VTOKEN: CurrencyId = CurrencyId::VToken2(99); // Mock vToken
pub const MOCK_USDC: CurrencyId = CurrencyId::Token2(98);    // Mock USDC

// Test accounts
pub const ALICE: u64 = 1;
pub const BOB: u64 = 2;
pub const PALLET_ACCOUNT: u64 = 100;

// Build genesis storage according to the mock runtime
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default().build_storage().unwrap();

    let pallet_account = HalalLending::account_id();
    
    orml_tokens::GenesisConfig::<Test> {
        balances: vec![
            // Give Alice 10,000 mock vTokens
            (ALICE, MOCK_VTOKEN, 10_000),
            // Give Bob 5,000 mock vTokens
            (BOB, MOCK_VTOKEN, 5_000),
            // Give the pallet 100,000 mock USDC to lend out
            (pallet_account, MOCK_USDC, 100_000),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}
