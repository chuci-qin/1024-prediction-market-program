//! Utility functions for the Prediction Market Program

use borsh::BorshDeserialize;
use solana_program::{
    account_info::AccountInfo,
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::error::PredictionMarketError;
use crate::state::PRICE_PRECISION;

/// Safely deserialize account data using BorshDeserialize::deserialize
/// This does NOT require the slice to be fully consumed, which is important
/// when the account has padding bytes at the end.
pub fn deserialize_account<T: BorshDeserialize>(data: &[u8]) -> Result<T, ProgramError> {
    T::deserialize(&mut &data[..])
        .map_err(|_| ProgramError::InvalidAccountData)
}

/// Check if a signer is authorized
pub fn check_signer(account: &AccountInfo) -> ProgramResult {
    if !account.is_signer {
        return Err(PredictionMarketError::InvalidSigner.into());
    }
    Ok(())
}

/// Verify PDA derivation
pub fn verify_pda(
    expected: &Pubkey,
    program_id: &Pubkey,
    seeds: &[&[u8]],
) -> Result<u8, ProgramError> {
    let (pda, bump) = Pubkey::find_program_address(seeds, program_id);
    if pda != *expected {
        msg!("PDA mismatch: expected {}, got {}", expected, pda);
        return Err(PredictionMarketError::InvalidProgramAddress.into());
    }
    Ok(bump)
}

/// Get current timestamp from Clock sysvar
pub fn get_current_timestamp() -> Result<i64, ProgramError> {
    let clock = Clock::get()?;
    Ok(clock.unix_timestamp)
}

/// Create a PDA account
pub fn create_pda_account<'a>(
    payer: &AccountInfo<'a>,
    pda: &AccountInfo<'a>,
    space: usize,
    owner: &Pubkey,
    system_program: &AccountInfo<'a>,
    seeds: &[&[u8]],
) -> ProgramResult {
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(space);
    
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            pda.key,
            lamports,
            space as u64,
            owner,
        ),
        &[payer.clone(), pda.clone(), system_program.clone()],
        &[seeds],
    )?;
    
    Ok(())
}

/// Transfer SOL from a PDA
pub fn transfer_lamports<'a>(
    from: &AccountInfo<'a>,
    to: &AccountInfo<'a>,
    amount: u64,
) -> ProgramResult {
    **from.try_borrow_mut_lamports()? -= amount;
    **to.try_borrow_mut_lamports()? += amount;
    Ok(())
}

/// Safe addition for i64
pub fn safe_add_i64(a: i64, b: i64) -> Result<i64, ProgramError> {
    a.checked_add(b)
        .ok_or_else(|| PredictionMarketError::ArithmeticOverflow.into())
}

/// Safe subtraction for i64
pub fn safe_sub_i64(a: i64, b: i64) -> Result<i64, ProgramError> {
    a.checked_sub(b)
        .ok_or_else(|| PredictionMarketError::ArithmeticOverflow.into())
}

/// Safe addition for u64
pub fn safe_add_u64(a: u64, b: u64) -> Result<u64, ProgramError> {
    a.checked_add(b)
        .ok_or_else(|| PredictionMarketError::ArithmeticOverflow.into())
}

/// Safe subtraction for u64
pub fn safe_sub_u64(a: u64, b: u64) -> Result<u64, ProgramError> {
    a.checked_sub(b)
        .ok_or_else(|| PredictionMarketError::ArithmeticOverflow.into())
}

/// Safe multiplication for u64
pub fn safe_mul_u64(a: u64, b: u64) -> Result<u64, ProgramError> {
    a.checked_mul(b)
        .ok_or_else(|| PredictionMarketError::ArithmeticOverflow.into())
}

/// Safe division for u64
pub fn safe_div_u64(a: u64, b: u64) -> Result<u64, ProgramError> {
    if b == 0 {
        return Err(PredictionMarketError::ArithmeticOverflow.into());
    }
    Ok(a / b)
}

/// Calculate fee amount from total and basis points
pub fn calculate_fee(amount: u64, fee_bps: u16) -> u64 {
    ((amount as u128) * (fee_bps as u128) / 10000) as u64
}

/// Calculate amount after fee deduction
pub fn amount_after_fee(amount: u64, fee_bps: u16) -> u64 {
    amount.saturating_sub(calculate_fee(amount, fee_bps))
}

/// Validate price is within acceptable range
pub fn validate_price(price: u64) -> ProgramResult {
    if price < crate::state::MIN_PRICE || price > crate::state::MAX_PRICE {
        msg!("Invalid price: {} (min: {}, max: {})", 
             price, crate::state::MIN_PRICE, crate::state::MAX_PRICE);
        return Err(PredictionMarketError::InvalidOrderPrice.into());
    }
    Ok(())
}

/// Check if YES + NO prices sum to approximately 1 USDC
/// Allows for small spread (up to 5%)
pub fn validate_price_pair(yes_price: u64, no_price: u64) -> ProgramResult {
    let sum = yes_price.saturating_add(no_price);
    let min_sum = PRICE_PRECISION * 95 / 100;  // 0.95
    let max_sum = PRICE_PRECISION * 105 / 100; // 1.05
    
    if sum < min_sum || sum > max_sum {
        msg!("Invalid price pair: YES={}, NO={}, sum={} (expected ~{})", 
             yes_price, no_price, sum, PRICE_PRECISION);
        return Err(PredictionMarketError::InvalidOrderPrice.into());
    }
    Ok(())
}

/// Calculate USDC cost for buying tokens
pub fn calculate_buy_cost(amount: u64, price: u64) -> u64 {
    ((amount as u128) * (price as u128) / (PRICE_PRECISION as u128)) as u64
}

/// Calculate USDC proceeds for selling tokens
pub fn calculate_sell_proceeds(amount: u64, price: u64) -> u64 {
    calculate_buy_cost(amount, price)
}

/// Calculate tokens receivable for USDC amount
pub fn calculate_tokens_for_usdc(usdc_amount: u64, price: u64) -> u64 {
    if price == 0 {
        return 0;
    }
    ((usdc_amount as u128) * (PRICE_PRECISION as u128) / (price as u128)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_fee() {
        // 100 USDC with 1% fee = 1 USDC
        assert_eq!(calculate_fee(100_000_000, 100), 1_000_000);
        
        // 100 USDC with 0.1% fee = 0.1 USDC
        assert_eq!(calculate_fee(100_000_000, 10), 100_000);
        
        // 1000 USDC with 0.5% fee = 5 USDC
        assert_eq!(calculate_fee(1000_000_000, 50), 5_000_000);
    }

    #[test]
    fn test_amount_after_fee() {
        // 100 USDC with 1% fee = 99 USDC remaining
        assert_eq!(amount_after_fee(100_000_000, 100), 99_000_000);
    }

    #[test]
    fn test_calculate_buy_cost() {
        // Buy 100 tokens at $0.65 = $65
        assert_eq!(calculate_buy_cost(100, 650_000), 65);
        
        // Buy 1000 tokens at $0.50 = $500
        assert_eq!(calculate_buy_cost(1000, 500_000), 500);
    }

    #[test]
    fn test_calculate_tokens_for_usdc() {
        // $65 at $0.65 = 100 tokens
        assert_eq!(calculate_tokens_for_usdc(65, 650_000), 100);
        
        // $100 at $0.50 = 200 tokens
        assert_eq!(calculate_tokens_for_usdc(100, 500_000), 200);
    }

    #[test]
    fn test_validate_price() {
        // Valid prices
        assert!(validate_price(500_000).is_ok());  // $0.50
        assert!(validate_price(100_000).is_ok());  // $0.10
        assert!(validate_price(900_000).is_ok());  // $0.90
        
        // Invalid prices (too low or too high)
        assert!(validate_price(1_000).is_err());   // $0.001
        assert!(validate_price(999_000).is_err()); // $0.999
    }

    #[test]
    fn test_validate_price_pair() {
        // Valid pair (sum = 1.0)
        assert!(validate_price_pair(500_000, 500_000).is_ok());
        
        // Valid pair with small spread
        assert!(validate_price_pair(520_000, 490_000).is_ok());
        
        // Invalid pair (sum too low)
        assert!(validate_price_pair(400_000, 400_000).is_err());
        
        // Invalid pair (sum too high)
        assert!(validate_price_pair(600_000, 600_000).is_err());
    }

    #[test]
    fn test_safe_arithmetic() {
        // Safe add
        assert_eq!(safe_add_u64(100, 50).unwrap(), 150);
        assert!(safe_add_u64(u64::MAX, 1).is_err());
        
        // Safe sub
        assert_eq!(safe_sub_u64(100, 50).unwrap(), 50);
        assert!(safe_sub_u64(50, 100).is_err());
        
        // Safe mul
        assert_eq!(safe_mul_u64(100, 5).unwrap(), 500);
        assert!(safe_mul_u64(u64::MAX, 2).is_err());
        
        // Safe div
        assert_eq!(safe_div_u64(100, 5).unwrap(), 20);
        assert!(safe_div_u64(100, 0).is_err());
    }
}

