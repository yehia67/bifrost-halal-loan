# Halal Lending Pallet

A truly halal (interest-free) lending protocol for Bifrost. Borrowers repay exactly what they borrowed - no interest, no fees!

## How It Works

1. **Deposit vToken Collateral**: Users deposit vTokens (like vDOT, vKSM) as collateral
2. **Borrow Stablecoins**: Get USDC loans based on collateral value
3. **Repay Exact Amount**: Repay exactly what you borrowed - nothing more!
4. **Platform Revenue**: Platform earns from staking rewards on locked vToken collateral

## Development Status

✅ **Step 2 Complete**: Core pallet structure implemented and compiles successfully!

### What's Working:
- ✅ Pallet compiles without errors
- ✅ Core data structures (LoanPosition, LoanStatus)
- ✅ Storage items (Loans, UserLoans, NextLoanId)
- ✅ Basic extrinsics (create_loan, repay_loan)
- ✅ Events and errors defined

### What's Next:
- ⏳ Add price oracle integration for LTV calculations
- ⏳ Add liquidation logic
- ⏳ Create mock runtime for testing
- ⏳ Write comprehensive tests
- ⏳ Add benchmarking
- ⏳ Integrate with Bifrost runtime

## Testing Your Code

### 1. Compile Check (✅ Working)
```bash
cargo check -p bifrost-halal-lending
```

### 2. Run Tests (Once mock runtime is set up)
```bash
cargo test -p bifrost-halal-lending
```

### 3. Build the Pallet
```bash
cargo build -p bifrost-halal-lending
```

## Quick Function Overview

### `create_loan`
```rust
pub fn create_loan(
    origin: OriginFor<T>,
    vtoken_id: CurrencyId,        // e.g., vDOT, vKSM
    collateral_amount: Balance,    // Amount of vToken to lock
    loan_amount: Balance,          // Amount of USDC to borrow
) -> DispatchResult
```

### `repay_loan`
```rust
pub fn repay_loan(
    origin: OriginFor<T>,
    loan_id: T::LoanId,           // ID of the loan to repay
) -> DispatchResult
// Note: Always repays exact loan_amount - no interest!
```

## Key Features

- **Zero Interest**: Borrowers repay exactly what they borrowed
- **vToken Collateral**: Uses Bifrost's liquid staking tokens
- **Platform Revenue**: Earns from staking rewards on collateral
- **Sharia Compliant**: No riba (interest), truly halal

## Architecture

```
User
  ↓ (deposits vDOT)
Halal Lending Pallet
  ↓ (locks vDOT, earns staking rewards)
  ↓ (lends USDC)
User
  ↓ (repays exact USDC amount)
Halal Lending Pallet
  ↓ (returns vDOT)
User
```

## Configuration

The pallet requires these associated types in your runtime:

```rust
type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
type MultiCurrency: MultiCurrency<Self::AccountId, CurrencyId = CurrencyId, Balance = Balance>;
type PriceProvider: OraclePriceProvider;
type LoanId: Parameter + Member + AtLeast32BitUnsigned + Default + Copy + MaxEncodedLen;
type MaxLTV: Get<Permill>;  // e.g., 50% = Permill::from_percent(50)
type WeightInfo: WeightInfo;
```

## Next Steps

1. **Add LTV Validation**: Implement collateral ratio checks in `create_loan`
2. **Price Oracle Integration**: Use `PriceProvider` to calculate collateral value
3. **Liquidation Logic**: Add `liquidate_loan` extrinsic
4. **Testing**: Create mock runtime and comprehensive tests
5. **Benchmarking**: Add weight calculations
6. **Runtime Integration**: Add to Bifrost runtime

## License

GPL-3.0-or-later WITH Classpath-exception-2.0
