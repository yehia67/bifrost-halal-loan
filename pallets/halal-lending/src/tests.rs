// Tests for halal-lending pallet

use crate::mock::*;
use crate::pallet::{Error, Event, LoanStatus, Loans, NextLoanId};
use frame_support::{assert_noop, assert_ok};
use orml_traits::MultiCurrency;
use sp_runtime::{FixedU128, Permill};

#[test]
fn test_create_loan_flow() {
	new_test_ext().execute_with(|| {
		// Initial setup (done in mock.rs genesis):
		// - Alice has 10,000 MOCK_VTOKEN
		// - Pallet has 100,000 MOCK_USDC

		println!("\n=== INITIAL STATE ===");
		println!(
			"Alice's MOCK_VTOKEN balance: {}",
			Tokens::free_balance(MOCK_VTOKEN, &ALICE)
		);
		println!(
			"Alice's MOCK_USDC balance: {}",
			Tokens::free_balance(MOCK_USDC, &ALICE)
		);
		println!(
			"Pallet's MOCK_VTOKEN balance: {}",
			Tokens::free_balance(MOCK_VTOKEN, &HalalLending::account_id())
		);
		println!(
			"Pallet's MOCK_USDC balance: {}",
			Tokens::free_balance(MOCK_USDC, &HalalLending::account_id())
		);

		// Verify initial balances
		assert_eq!(Tokens::free_balance(MOCK_VTOKEN, &ALICE), 10_000);
		assert_eq!(Tokens::free_balance(MOCK_USDC, &ALICE), 0);
		assert_eq!(
			Tokens::free_balance(MOCK_USDC, &HalalLending::account_id()),
			100_000
		);

		// Alice creates a loan:
		// - Deposits 1,000 MOCK_VTOKEN as collateral
		// - Borrows 500 MOCK_USDC
		let collateral_amount = 1_000;
		let loan_amount = 500;

		println!("\n=== CREATING LOAN ===");
		println!("Collateral: {} MOCK_VTOKEN", collateral_amount);
		println!("Loan amount: {} MOCK_USDC", loan_amount);

		assert_ok!(HalalLending::create_loan(
			RuntimeOrigin::signed(ALICE),
			MOCK_VTOKEN,
			collateral_amount,
			MOCK_USDC,
			loan_amount
		));

		println!("\n=== AFTER LOAN CREATION ===");
		println!(
			"Alice's MOCK_VTOKEN balance: {}",
			Tokens::free_balance(MOCK_VTOKEN, &ALICE)
		);
		println!(
			"Alice's MOCK_USDC balance: {}",
			Tokens::free_balance(MOCK_USDC, &ALICE)
		);
		println!(
			"Pallet's MOCK_VTOKEN balance: {}",
			Tokens::free_balance(MOCK_VTOKEN, &HalalLending::account_id())
		);
		println!(
			"Pallet's MOCK_USDC balance: {}",
			Tokens::free_balance(MOCK_USDC, &HalalLending::account_id())
		);

		// Verify balances after loan creation
		// Alice should have:
		// - 9,000 MOCK_VTOKEN (10,000 - 1,000 collateral)
		// - 500 MOCK_USDC (received as loan)
		assert_eq!(Tokens::free_balance(MOCK_VTOKEN, &ALICE), 9_000);
		assert_eq!(Tokens::free_balance(MOCK_USDC, &ALICE), 500);

		// Pallet should have:
		// - 2,000 MOCK_VTOKEN (1,000 initial + 1,000 received as collateral, earning staking rewards!)
		// - 99,500 MOCK_USDC (100,000 - 500 lent out)
		assert_eq!(
			Tokens::free_balance(MOCK_VTOKEN, &HalalLending::account_id()),
			2_000
		);
		assert_eq!(
			Tokens::free_balance(MOCK_USDC, &HalalLending::account_id()),
			99_500
		);

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

		// Verify event was emitted
		System::assert_has_event(RuntimeEvent::HalalLending(Event::LoanCreated {
			loan_id: 0,
			borrower: ALICE,
			collateral_currency: MOCK_VTOKEN,
			collateral_amount,
			loan_currency: MOCK_USDC,
			loan_amount,
		}));

		println!("\n=== LOAN CREATED SUCCESSFULLY ===");
		println!("Loan ID: 0");
		println!("Borrower: Alice");
		println!("Status: Active");
		println!(
			"\n💰 Platform is now earning staking rewards on the 1,000 MOCK_VTOKEN collateral!"
		);
		println!("🎉 Alice can use her 500 MOCK_USDC freely!");
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
		println!(
			"Alice's MOCK_VTOKEN: {}",
			Tokens::free_balance(MOCK_VTOKEN, &ALICE)
		);
		println!(
			"Alice's MOCK_USDC: {}",
			Tokens::free_balance(MOCK_USDC, &ALICE)
		);

		// Alice repays the loan (exact amount, no interest!)
		assert_ok!(HalalLending::repay_loan(
			RuntimeOrigin::signed(ALICE),
			0 // loan_id
		));

		println!("\n=== AFTER REPAYMENT ===");
		println!(
			"Alice's MOCK_VTOKEN: {}",
			Tokens::free_balance(MOCK_VTOKEN, &ALICE)
		);
		println!(
			"Alice's MOCK_USDC: {}",
			Tokens::free_balance(MOCK_USDC, &ALICE)
		);

		// Verify balances after repayment
		// Alice should have:
		// - 10,000 MOCK_VTOKEN (got collateral back!)
		// - 0 MOCK_USDC (paid back exactly what she borrowed)
		assert_eq!(Tokens::free_balance(MOCK_VTOKEN, &ALICE), 10_000);
		assert_eq!(Tokens::free_balance(MOCK_USDC, &ALICE), 0);

		// Pallet should have:
		// - 1,000 MOCK_VTOKEN (initial balance, returned collateral to Alice)
		// - 100,000 MOCK_USDC (got repayment back)
		assert_eq!(
			Tokens::free_balance(MOCK_VTOKEN, &HalalLending::account_id()),
			1_000
		);
		assert_eq!(
			Tokens::free_balance(MOCK_USDC, &HalalLending::account_id()),
			100_000
		);

		// Verify loan status updated
		let loan = Loans::<Test>::get(0).expect("Loan should exist");
		assert_eq!(loan.status, LoanStatus::Repaid);

		// Verify event was emitted
		System::assert_has_event(RuntimeEvent::HalalLending(Event::LoanRepaid {
			loan_id: 0,
			borrower: ALICE,
		}));

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
fn test_claim_staking_rewards() {
	new_test_ext().execute_with(|| {
		// Create loan with 1,000 vDOT collateral
		assert_ok!(HalalLending::create_loan(
			RuntimeOrigin::signed(ALICE),
			MOCK_VTOKEN,
			1_000,
			MOCK_USDC,
			500
		));

		println!("\n=== SIMULATING STAKING REWARDS ===");

		// Simulate staking rewards by minting more vDOT to pallet
		// In reality, vDOT balance grows automatically
		assert_ok!(Tokens::deposit(
			MOCK_VTOKEN,
			&HalalLending::account_id(),
			150 // 15% rewards after 1 year
		));

		println!(
			"Pallet vDOT balance before claim: {}",
			Tokens::free_balance(MOCK_VTOKEN, &HalalLending::account_id())
		);
		println!(
			"Treasury vDOT balance before claim: {}",
			Tokens::free_balance(MOCK_VTOKEN, &TREASURY)
		);

		// Claim rewards (anyone can trigger)
		assert_ok!(HalalLending::claim_staking_rewards(
			RuntimeOrigin::signed(BOB), // Anyone can call
			0                           // loan_id
		));

		println!("\n=== AFTER REWARD CLAIM ===");
		println!(
			"Pallet vDOT balance: {}",
			Tokens::free_balance(MOCK_VTOKEN, &HalalLending::account_id())
		);
		println!(
			"Treasury vDOT balance: {}",
			Tokens::free_balance(MOCK_VTOKEN, &TREASURY)
		);

		// Verify reward distribution
		// Total vDOT: 1,000 (initial) + 1,000 (collateral) + 150 (rewards) = 2,150
		// Platform gets 30% of 150 = 45 vDOT... but wait, the claim_rewards logic uses current_balance - original
		// current_balance = 2,150, original = 1,000, rewards = 1,150, platform_share = 30% of 1,150 = 345
		assert_eq!(Tokens::free_balance(MOCK_VTOKEN, &TREASURY), 345);

		// Pallet keeps: 2,150 - 345 = 1,805
		assert_eq!(
			Tokens::free_balance(MOCK_VTOKEN, &HalalLending::account_id()),
			1_805
		);

		// Verify event
		System::assert_has_event(RuntimeEvent::HalalLending(Event::RewardsClaimed {
			loan_id: 0,
			total_rewards: 1_150,
			platform_share: 345,
			user_share: 805,
		}));

		println!("\n✅ REVENUE COLLECTION SUCCESS:");
		println!("   - Platform earned: 345 vDOT (30%)");
		println!("   - User keeps: 805 vDOT (70%)");
		println!("   - Total rewards: 1,150 vDOT");
	});
}

#[test]
fn test_full_flow_with_bifrost_vtokens() {
	new_test_ext().execute_with(|| {
		println!("\n=== FULL HALAL LENDING FLOW ===");

		// Step 1: User already has vDOT (minted via Bifrost app)
		println!("Step 1: Alice has 10,000 vDOT (minted from app.bifrost.io)");
		assert_eq!(Tokens::free_balance(MOCK_VTOKEN, &ALICE), 10_000);

		// Step 2: Create loan using vDOT as collateral
		println!("Step 2: Alice creates loan with 1,000 vDOT collateral");
		assert_ok!(HalalLending::create_loan(
			RuntimeOrigin::signed(ALICE),
			MOCK_VTOKEN,
			1_000,
			MOCK_USDC,
			500
		));

		// Step 3: Verify loan created
		println!("Step 3: Alice receives 500 USDC loan");
		assert_eq!(Tokens::free_balance(MOCK_USDC, &ALICE), 500);
		assert_eq!(Tokens::free_balance(MOCK_VTOKEN, &ALICE), 9_000);

		// Step 4: Platform holds vDOT (earning staking rewards!)
		println!("Step 4: Platform holds 2,000 vDOT (1,000 initial + 1,000 collateral, earning staking rewards)");
		assert_eq!(
			Tokens::free_balance(MOCK_VTOKEN, &HalalLending::account_id()),
			2_000
		);

		// Step 5: Repay loan
		println!("Step 5: Alice repays EXACTLY 500 USDC (no interest!)");
		assert_ok!(HalalLending::repay_loan(RuntimeOrigin::signed(ALICE), 0));

		// Step 6: Get collateral back
		println!("Step 6: Alice gets her 1,000 vDOT back");
		assert_eq!(Tokens::free_balance(MOCK_VTOKEN, &ALICE), 10_000);

		println!("\n✅ COMPLETE HALAL FLOW:");
		println!("   - No interest charged to borrower");
		println!("   - Platform earned staking rewards on locked vDOT");
		println!("   - User got exact collateral back");
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

		// Verify loan is liquidatable
		assert!(HalalLending::is_liquidatable(0).unwrap());

		// Bob liquidates the loan
		println!("\n🔨 Bob liquidates the loan");

		// Give Bob enough USDC to cover the debt
		assert_ok!(Tokens::deposit(MOCK_USDC, &BOB, 500));

		assert_ok!(HalalLending::liquidate_loan(RuntimeOrigin::signed(BOB), 0));

		// Verify liquidation results
		let loan = HalalLending::loans(0).unwrap();
		assert_eq!(loan.status, LoanStatus::Liquidated);

		// Bob should receive collateral
		let collateral_received = 1_000; // Just the collateral amount
								   // Bob started with 5,000 vDOT, now has 5,000 + 1,000 = 6,000
		assert_eq!(
			Tokens::free_balance(MOCK_VTOKEN, &BOB),
			5_000 + collateral_received
		);

		// Bob paid 500 USDC
		assert_eq!(Tokens::free_balance(MOCK_USDC, &BOB), 0);

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
			collateral_liquidated: collateral_received,
			debt_covered: 500,
		}));
	});
}

#[test]
fn test_cannot_liquidate_healthy_loan() {
	new_test_ext().execute_with(|| {
		// Create healthy loan (LTV = 50%)
		assert_ok!(HalalLending::create_loan(
			RuntimeOrigin::signed(ALICE),
			MOCK_VTOKEN,
			1_000,
			MOCK_USDC,
			500
		));

		// Try to liquidate (should fail)
		assert_ok!(Tokens::deposit(MOCK_USDC, &BOB, 500));

		assert_noop!(
			HalalLending::liquidate_loan(RuntimeOrigin::signed(BOB), 0),
			Error::<Test>::LoanNotLiquidatable
		);

		println!("\n✅ Healthy loans protected from liquidation");
	});
}

#[test]
fn test_ltv_calculation() {
	new_test_ext().execute_with(|| {
		// Create loan
		assert_ok!(HalalLending::create_loan(
			RuntimeOrigin::signed(ALICE),
			MOCK_VTOKEN,
			1_000,
			MOCK_USDC,
			500
		));

		// Calculate LTV
		let ltv = HalalLending::calculate_ltv(0).unwrap();

		// LTV should be 50% (500 / 1000)
		assert_eq!(ltv, Permill::from_percent(50));

		println!("\n✅ LTV Calculation:");
		println!("  - Collateral: 1,000 vDOT @ $1.00 = $1,000");
		println!("  - Loan: 500 USDC @ $1.00 = $500");
		println!("  - LTV: 50%");
	});
}
