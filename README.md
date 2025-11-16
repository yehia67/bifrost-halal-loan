# Halal Lending Pallet

> Zero-interest lending pallet for Substrate/Polkadot

## 🚀 Quick Start - Local Development

### Prerequisites
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Polkadot SDK tools
cargo install --git https://github.com/paritytech/polkadot-sdk polkadot-omni-node
cargo install --git https://github.com/paritytech/polkadot-sdk staging-chain-spec-builder
```

### Run Local Node (No TestNet Required)
```bash
# 1. Clone and build
git clone <this-repo>
cd parachain-template
cargo build --release

# 2. Generate local chain spec
chain-spec-builder create -t development \
--relay-chain paseo \
--para-id 1000 \
--runtime ./target/release/wbuild/parachain-template-runtime/parachain_template_runtime.compact.compressed.wasm \
named-preset development


# 3. Start local development node
polkadot-omni-node --chain plain_chain_spec.json \
--dev \
--rpc-port 8845 \
--dev-block-time 3000
```

### Connect & Test
**Polkadot.js Apps**: https://polkadot.js.org/apps/?rpc=ws://localhost:8845

## Network Info
- **RPC**: `ws://localhost:8845`
- **Mode**: Development (local)
- **Decimals**: 12
- **Symbol**: UNIT

## 🏗️ no-interest-loans Functions

### User Functions
- `createLoan(collateralToken, collateralAmount, loanToken, loanAmount)`
- `repayLoan(loanId)` - Automatically claims rewards
- `liquidateLoan(loanId)`

### Admin Functions (Sudo only)
- `distributeCycleRewards(totalRewards)` ⭐ **Main reward distribution**
- `addRewards(loanId, amount)`
- `setPrice(currencyId, price)`

### How It Works
1. **Create loan** → Lock collateral, get loan
2. **Earn rewards** → Platform stakes your collateral
3. **Repay loan** → Get 70% of staking rewards + collateral back
4. **Platform keeps** → 30% of rewards as fee

## 🧪 Test the Functions

### Via Polkadot.js Apps
1. Go to: https://polkadot.js.org/apps/?rpc=ws://localhost:8845
2. Navigate to: **Developer** → **Extrinsics** → **noInterestLoans**

### Test Example
```
1. createLoan:
   - collateralVtoken: 1
   - collateralAmount: 1000000000000 (1 token)
   - loanCurrency: 2
   - loanAmount: 500000000000 (0.5 tokens)

2. distributeCycleRewards (admin):
   - cycleRewards: 300000000000 (0.3 tokens)

3. repayLoan:
   - loanId: 0
```

### Expected Result
- Borrower gets: 70% of rewards (210 tokens) + collateral back
- Treasury gets: 30% of rewards (90 tokens)

## 🧪 Run Unit Tests

```bash
# Test the no-interest-loans pallet
cargo test --package pallet-no-interest-loans

# Test with output
cargo test --package pallet-no-interest-loans -- --nocapture

# Test specific function
cargo test --package pallet-no-interest-loans test_claim_rewards_on_repayment
```

## 💻 Frontend Integration

### Connect to API
```javascript
import { ApiPromise, WsProvider } from '@polkadot/api';

const api = await ApiPromise.create({ 
  provider: new WsProvider('ws://localhost:8845') 
});
```

### Call Functions
```javascript
// Create loan
await api.tx.noInterestLoans.createLoan(1, amount, 2, loanAmount)
  .signAndSend(account);

// Distribute rewards (admin)
await api.tx.noInterestLoans.distributeCycleRewards(rewardAmount)
  .signAndSend(sudoAccount);

// Query data
const loan = await api.query.noInterestLoans.loans(loanId);
const rewards = await api.query.noInterestLoans.loanRewards(loanId);
```

---

**� Zero interest, only halal profit sharing**
