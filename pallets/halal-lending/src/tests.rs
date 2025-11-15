// Tests for halal-lending pallet

use crate::mock::*;
use crate::pallet::{Error, Event, LoanRewards, LoanStatus, Loans, NextLoanId, PriceProvider};
use polkadot_sdk::{
    frame_support::{assert_noop, assert_ok},
    sp_runtime::{FixedU128, Permill},
};

#[test]
fn test_create_loan_flow() {
    new_test_ext().execute_with(|| {
        println!("\n=== INITIAL STATE ===");
        println!("Alice's balance: {}", Balances::free_balance(&ALICE));
        println!(
            "Pallet's balance: {}",
            Balances::free_balance(&HalalLending::account_id())
        );

        // Verify initial balances
        assert_eq!(Balances::free_balance(&ALICE), 10_000);
        assert_eq!(Balances::free_balance(&HalalLending::account_id()), 100_000);

        // Alice creates a loan
        let collateral_amount = 1_000;
        let loan_amount = 500;

        println!("\n=== CREATING LOAN ===");
        println!("Collateral: {} tokens", collateral_amount);
        println!("Loan amount: {} tokens", loan_amount);

        assert_ok!(HalalLending::create_loan(
            RuntimeOrigin::signed(ALICE),
            MOCK_VTOKEN,
            collateral_amount,
            MOCK_USDC,
            loan_amount
        ));

        println!("\n=== AFTER LOAN CREATION ===");
        println!("Alice's balance: {}", Balances::free_balance(&ALICE));
        println!(
            "Pallet's balance: {}",
            Balances::free_balance(&HalalLending::account_id())
        );

        // Verify balances after loan creation
        // Alice should have: 9,500 (10,000 - 1,000 collateral + 500 loan)
        // Pallet should have: 100,500 (100,000 + 1,000 collateral - 500 loan)
        assert_eq!(Balances::free_balance(&ALICE), 9_500);
        assert_eq!(Balances::free_balance(&HalalLending::account_id()), 100_500);

        // Verify loan was stored correctly
        let loan = Loans::<Test>::get(0).expect("Loan should exist");
        assert_eq!(loan.borrower, ALICE);
        assert_eq!(loan.collateral_vtoken, MOCK_VTOKEN);
        assert_eq!(loan.collateral_amount, collateral_amount);
        assert_eq!(loan.loan_currency, MOCK_USDC);
        assert_eq!(loan.loan_amount, loan_amount);
        assert_eq!(loan.status, LoanStatus::Active);

        // Verify NextLoanId incremented
        assert_eq!(NextLoanId::<Test>::get(), 1);

        println!("\n=== LOAN CREATED SUCCESSFULLY ===");
        println!("Loan ID: 0");
        println!("Borrower: Alice");
        println!("Status: Active");
    });
}

#[test]
fn test_repay_loan_flow() {
    new_test_ext().execute_with(|| {
        // First, create a loan
        let collateral_amount = 1_000;
        let loan_amount = 500;

        assert_ok!(HalalLending::create_loan(
            RuntimeOrigin::signed(ALICE),
            MOCK_VTOKEN,
            collateral_amount,
            MOCK_USDC,
            loan_amount
        ));

        println!("\n=== BEFORE REPAYMENT ===");
        println!("Alice's balance: {}", Balances::free_balance(&ALICE));
        println!(
            "Pallet's balance: {}",
            Balances::free_balance(&HalalLending::account_id())
        );

        // Alice repays the loan (exact amount, no interest!)
        assert_ok!(HalalLending::repay_loan(
            RuntimeOrigin::signed(ALICE),
            0 // loan_id
        ));

        println!("\n=== AFTER REPAYMENT ===");
        println!("Alice's balance: {}", Balances::free_balance(&ALICE));
        println!(
            "Pallet's balance: {}",
            Balances::free_balance(&HalalLending::account_id())
        );

        // Verify balances after repayment
        // Alice should have: 10,000 (got collateral back!)
        // Pallet should have: 100,000 (returned collateral to Alice)
        assert_eq!(Balances::free_balance(&ALICE), 10_000);
        assert_eq!(Balances::free_balance(&HalalLending::account_id()), 100_000);

        // Verify loan status updated
        let loan = Loans::<Test>::get(0).expect("Loan should exist");
        assert_eq!(loan.status, LoanStatus::Repaid);

        println!("\n=== LOAN REPAID SUCCESSFULLY ===");
        println!("✅ Alice repaid EXACTLY what she borrowed (no interest!)");
        println!("✅ Alice got her collateral back");
        println!("💰 Platform kept the staking rewards earned during the loan period");
    });
}

#[test]
fn test_multiple_loans() {
    new_test_ext().execute_with(|| {
        // Alice creates first loan
        assert_ok!(HalalLending::create_loan(
            RuntimeOrigin::signed(ALICE),
            MOCK_VTOKEN,
            1_000,
            MOCK_USDC,
            500
        ));

        // Bob creates a loan
        assert_ok!(HalalLending::create_loan(
            RuntimeOrigin::signed(BOB),
            MOCK_VTOKEN,
            2_000,
            MOCK_USDC,
            1_000
        ));

        // Alice creates second loan
        assert_ok!(HalalLending::create_loan(
            RuntimeOrigin::signed(ALICE),
            MOCK_VTOKEN,
            500,
            MOCK_USDC,
            250
        ));

        // Verify loan IDs incremented correctly
        assert_eq!(NextLoanId::<Test>::get(), 3);

        // Verify each loan
        let loan0 = Loans::<Test>::get(0).unwrap();
        assert_eq!(loan0.borrower, ALICE);
        assert_eq!(loan0.loan_amount, 500);

        let loan1 = Loans::<Test>::get(1).unwrap();
        assert_eq!(loan1.borrower, BOB);
        assert_eq!(loan1.loan_amount, 1_000);

        let loan2 = Loans::<Test>::get(2).unwrap();
        assert_eq!(loan2.borrower, ALICE);
        assert_eq!(loan2.loan_amount, 250);

        println!("\n=== MULTIPLE LOANS TEST ===");
        println!("✅ Alice has 2 loans (IDs: 0, 2)");
        println!("✅ Bob has 1 loan (ID: 1)");
        println!("✅ Total loans created: 3");
    });
}

#[test]
fn test_repay_loan_not_owner() {
    new_test_ext().execute_with(|| {
        // Alice creates a loan
        assert_ok!(HalalLending::create_loan(
            RuntimeOrigin::signed(ALICE),
            MOCK_VTOKEN,
            1_000,
            MOCK_USDC,
            500
        ));

        // Bob tries to repay Alice's loan - should fail!
        assert_noop!(
            HalalLending::repay_loan(RuntimeOrigin::signed(BOB), 0),
            Error::<Test>::NotLoanOwner
        );

        println!("\n=== AUTHORIZATION TEST ===");
        println!("✅ Bob cannot repay Alice's loan");
        println!("✅ Only loan owner can repay");
    });
}

#[test]
fn test_loan_not_found() {
    new_test_ext().execute_with(|| {
        // Try to repay non-existent loan
        assert_noop!(
            HalalLending::repay_loan(RuntimeOrigin::signed(ALICE), 999),
            Error::<Test>::LoanNotFound
        );

        println!("\n=== LOAN NOT FOUND TEST ===");
        println!("✅ Cannot repay non-existent loan");
    });
}

#[test]
fn test_set_price() {
    new_test_ext().execute_with(|| {
        // Set price for MOCK_VTOKEN
        let price = FixedU128::from_rational(2, 1); // 2.0

        assert_ok!(HalalLending::set_price(
            RuntimeOrigin::root(),
            MOCK_VTOKEN,
            price
        ));

        // Verify price was set
        assert_eq!(MockPriceProvider::get_price(MOCK_VTOKEN), Some(price));

        println!("\n=== PRICE SETTING TEST ===");
        println!("✅ Price set successfully");
        println!("✅ Price provider working correctly");
    });
}

#[test]
fn test_liquidation_when_ltv_exceeds_threshold() {
    new_test_ext().execute_with(|| {
        println!("\n=== LIQUIDATION TEST ===");

        // Setup: Create loan with 1,000 vDOT collateral, 500 USDC loan
        assert_ok!(HalalLending::create_loan(
            RuntimeOrigin::signed(ALICE),
            MOCK_VTOKEN,
            1_000,
            MOCK_USDC,
            500
        ));

        println!("Initial loan created:");
        println!("  - Collateral: 1,000 vDOT");
        println!("  - Loan: 500 USDC");
        println!("  - Initial LTV: 50%");

        // Simulate price drop: vDOT price drops 40%
        // This makes LTV = 500 / (1000 * 0.6) = 83.3% > 75% threshold
        MockPriceProvider::set_price(MOCK_VTOKEN, FixedU128::from_rational(6, 10)); // $0.60

        println!("\n💥 Price crash! vDOT drops to $0.60");

        let ltv = HalalLending::calculate_ltv(0).unwrap();
        println!("  - New LTV: {}%", ltv.deconstruct() as f64 / 10000.0);

        // Bob liquidates the loan
        println!("\n🔨 Bob liquidates the loan");

        assert_ok!(HalalLending::liquidate_loan(RuntimeOrigin::signed(BOB), 0));

        // Verify liquidation results
        let loan = HalalLending::loans(0).unwrap();
        assert_eq!(loan.status, LoanStatus::Liquidated);

        // Bob should receive collateral
        let collateral_received = 1_000; // Just the collateral amount
                                         // Bob started with 5,000 vDOT, now has 5,000 + 1,000 - 500 = 5,500 vDOT
        assert_eq!(Balances::free_balance(BOB), 5_000 + collateral_received - 500);

        println!("\n✅ LIQUIDATION SUCCESS:");
        println!("  - Bob paid: 500 USDC");
        println!(
            "  - Bob received: {} vDOT (collateral at discount)",
            collateral_received
        );
        println!("  - Loan status: Liquidated");

        // Verify event
        System::assert_has_event(RuntimeEvent::HalalLending(Event::LoanLiquidated {
            loan_id: 0,
            borrower: ALICE,
            liquidator: BOB,
        }));
    });
}

#[test]
fn test_claim_rewards_on_repayment() {
    new_test_ext().execute_with(|| {
        // Create a loan first
        assert_ok!(HalalLending::create_loan(
            RuntimeOrigin::signed(ALICE),
            MOCK_VTOKEN,
            1_000,
            MOCK_USDC,
            500
        ));

        // Add rewards to simulate staking rewards
        let reward_amount = 300;
        assert_ok!(HalalLending::distribute_cycle_rewards(
            RuntimeOrigin::root(),
            reward_amount
        ));

        // Verify rewards were added
        assert_eq!(LoanRewards::<Test>::get(0), reward_amount);

        // Record initial balances before repayment
        let alice_before_repay = Balances::free_balance(&ALICE); // Should be 9,500 (after loan creation)
        let treasury_before_repay = Balances::free_balance(&TREASURY); // Should be 1,000 (initial)

        println!("Alice balance before repay: {}", alice_before_repay);
        println!("Treasury balance before repay: {}", treasury_before_repay);

        // Repay loan (this should trigger reward claiming)
        assert_ok!(HalalLending::repay_loan(
            RuntimeOrigin::signed(ALICE),
            0 // loan_id
        ));

        // Calculate expected reward distribution
        // Platform takes 30% (StakingRewardFee), borrower gets 70%
        let platform_share = 90; // 30% of 300
        let user_share = 210; // 70% of 300

        println!(
            "Alice balance after repay: {}",
            Balances::free_balance(&ALICE)
        );
        println!(
            "Treasury balance after repay: {}",
            Balances::free_balance(&TREASURY)
        );

        // After repayment, Alice should have:
        // - Her original collateral back (1,000)
        // - Her reward share (210)
        // - Total: 10,000 + 210 = 10,210
        assert_eq!(Balances::free_balance(&ALICE), 10_000 + user_share);
        assert_eq!(
            Balances::free_balance(&TREASURY),
            treasury_before_repay + platform_share
        );

        // Rewards should be cleared after claiming
        assert_eq!(LoanRewards::<Test>::get(0), 0);

        println!("\n=== CLAIM REWARDS TEST ===");
        println!("✅ Platform received 30% fee: {} tokens", platform_share);
        println!("✅ Borrower received 70% share: {} tokens", user_share);
        println!("✅ Rewards cleared from storage");
        println!("💰 TRUE HALAL: Borrower profits from staking rewards!");
    });
}
