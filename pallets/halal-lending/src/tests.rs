// Tests for halal-lending pallet

use crate::mock::*;
use crate::pallet::{Error, Event, Loans, NextLoanId};
use frame_support::{assert_ok, assert_noop};
use orml_traits::MultiCurrency;
use sp_runtime::traits::BadOrigin;

#[test]
fn test_create_loan_flow() {
    new_test_ext().execute_with(|| {
        // Initial setup (done in mock.rs genesis):
        // - Alice has 10,000 MOCK_VTOKEN
        // - Pallet has 100,000 MOCK_USDC
        
        println!("\n=== INITIAL STATE ===");
        println!("Alice's MOCK_VTOKEN balance: {}", Tokens::free_balance(MOCK_VTOKEN, &ALICE));
        println!("Alice's MOCK_USDC balance: {}", Tokens::free_balance(MOCK_USDC, &ALICE));
        println!("Pallet's MOCK_VTOKEN balance: {}", Tokens::free_balance(MOCK_VTOKEN, &HalalLending::account_id()));
        println!("Pallet's MOCK_USDC balance: {}", Tokens::free_balance(MOCK_USDC, &HalalLending::account_id()));
        
        // Verify initial balances
        assert_eq!(Tokens::free_balance(MOCK_VTOKEN, &ALICE), 10_000);
        assert_eq!(Tokens::free_balance(MOCK_USDC, &ALICE), 0);
        assert_eq!(Tokens::free_balance(MOCK_USDC, &HalalLending::account_id()), 100_000);
        
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
        println!("Alice's MOCK_VTOKEN balance: {}", Tokens::free_balance(MOCK_VTOKEN, &ALICE));
        println!("Alice's MOCK_USDC balance: {}", Tokens::free_balance(MOCK_USDC, &ALICE));
        println!("Pallet's MOCK_VTOKEN balance: {}", Tokens::free_balance(MOCK_VTOKEN, &HalalLending::account_id()));
        println!("Pallet's MOCK_USDC balance: {}", Tokens::free_balance(MOCK_USDC, &HalalLending::account_id()));
        
        // Verify balances after loan creation
        // Alice should have:
        // - 9,000 MOCK_VTOKEN (10,000 - 1,000 collateral)
        // - 500 MOCK_USDC (received as loan)
        assert_eq!(Tokens::free_balance(MOCK_VTOKEN, &ALICE), 9_000);
        assert_eq!(Tokens::free_balance(MOCK_USDC, &ALICE), 500);
        
        // Pallet should have:
        // - 1,000 MOCK_VTOKEN (received as collateral, earning staking rewards!)
        // - 99,500 MOCK_USDC (100,000 - 500 lent out)
        assert_eq!(Tokens::free_balance(MOCK_VTOKEN, &HalalLending::account_id()), 1_000);
        assert_eq!(Tokens::free_balance(MOCK_USDC, &HalalLending::account_id()), 99_500);
        
        // Verify loan was stored correctly
        let loan = Loans::<Test>::get(0).expect("Loan should exist");
        assert_eq!(loan.borrower, ALICE);
        assert_eq!(loan.collateral_vtoken, MOCK_VTOKEN);
        assert_eq!(loan.collateral_amount, collateral_amount);
        assert_eq!(loan.loan_currency, MOCK_USDC);
        assert_eq!(loan.loan_amount, loan_amount);
        assert_eq!(loan.status, crate::pallet::LoanStatus::Active);
        
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
        println!("\n💰 Platform is now earning staking rewards on the 1,000 MOCK_VTOKEN collateral!");
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
        println!("Alice's MOCK_VTOKEN: {}", Tokens::free_balance(MOCK_VTOKEN, &ALICE));
        println!("Alice's MOCK_USDC: {}", Tokens::free_balance(MOCK_USDC, &ALICE));
        
        // Alice repays the loan (exact amount, no interest!)
        assert_ok!(HalalLending::repay_loan(
            RuntimeOrigin::signed(ALICE),
            0 // loan_id
        ));
        
        println!("\n=== AFTER REPAYMENT ===");
        println!("Alice's MOCK_VTOKEN: {}", Tokens::free_balance(MOCK_VTOKEN, &ALICE));
        println!("Alice's MOCK_USDC: {}", Tokens::free_balance(MOCK_USDC, &ALICE));
        
        // Verify balances after repayment
        // Alice should have:
        // - 10,000 MOCK_VTOKEN (got collateral back!)
        // - 0 MOCK_USDC (paid back exactly what she borrowed)
        assert_eq!(Tokens::free_balance(MOCK_VTOKEN, &ALICE), 10_000);
        assert_eq!(Tokens::free_balance(MOCK_USDC, &ALICE), 0);
        
        // Pallet should have:
        // - 0 MOCK_VTOKEN (returned collateral)
        // - 100,000 MOCK_USDC (got repayment back)
        assert_eq!(Tokens::free_balance(MOCK_VTOKEN, &HalalLending::account_id()), 0);
        assert_eq!(Tokens::free_balance(MOCK_USDC, &HalalLending::account_id()), 100_000);
        
        // Verify loan status updated
        let loan = Loans::<Test>::get(0).expect("Loan should exist");
        assert_eq!(loan.status, crate::pallet::LoanStatus::Repaid);
        
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
