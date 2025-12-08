//! CPI (Cross-Program Invocation) helpers for the Prediction Market Program
//!
//! This module provides helpers for calling:
//! - Vault Program (lock/release funds, settlements)
//! - Fund Program (fee collection, maker rewards)
//! - SPL Token Program (mint/burn/transfer tokens)

use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::error::PredictionMarketError;

// ============================================================================
// Vault Program CPI
// ============================================================================

/// Lock user funds for prediction market (CPI to Vault Program)
/// 
/// This moves USDC from available_balance to pm_locked in the user's Vault account.
pub fn cpi_lock_for_prediction<'a>(
    vault_program: &AccountInfo<'a>,
    vault_config: &AccountInfo<'a>,
    user_account: &AccountInfo<'a>,
    caller_program: &AccountInfo<'a>,
    amount: u64,
    signer_seeds: &[&[u8]],
) -> ProgramResult {
    msg!("CPI: Lock {} for prediction market", amount);
    
    // Instruction discriminator for LockForPrediction (to be defined in Vault Program)
    // This is a placeholder - actual implementation depends on Vault Program instruction encoding
    let mut data = vec![20u8]; // Instruction index (placeholder)
    data.extend_from_slice(&amount.to_le_bytes());
    
    let accounts = vec![
        vault_config.clone(),
        user_account.clone(),
        caller_program.clone(),
    ];
    
    let ix = solana_program::instruction::Instruction {
        program_id: *vault_program.key,
        accounts: accounts.iter().map(|a| {
            solana_program::instruction::AccountMeta {
                pubkey: *a.key,
                is_signer: a.is_signer,
                is_writable: a.is_writable,
            }
        }).collect(),
        data,
    };
    
    invoke_signed(&ix, &accounts, &[signer_seeds])?;
    
    Ok(())
}

/// Release user funds from prediction market (CPI to Vault Program)
/// 
/// This moves USDC from pm_locked back to available_balance.
pub fn cpi_release_from_prediction<'a>(
    vault_program: &AccountInfo<'a>,
    vault_config: &AccountInfo<'a>,
    user_account: &AccountInfo<'a>,
    caller_program: &AccountInfo<'a>,
    amount: u64,
    signer_seeds: &[&[u8]],
) -> ProgramResult {
    msg!("CPI: Release {} from prediction market", amount);
    
    let mut data = vec![21u8]; // Instruction index (placeholder)
    data.extend_from_slice(&amount.to_le_bytes());
    
    let accounts = vec![
        vault_config.clone(),
        user_account.clone(),
        caller_program.clone(),
    ];
    
    let ix = solana_program::instruction::Instruction {
        program_id: *vault_program.key,
        accounts: accounts.iter().map(|a| {
            solana_program::instruction::AccountMeta {
                pubkey: *a.key,
                is_signer: a.is_signer,
                is_writable: a.is_writable,
            }
        }).collect(),
        data,
    };
    
    invoke_signed(&ix, &accounts, &[signer_seeds])?;
    
    Ok(())
}

/// Settle prediction market winnings (CPI to Vault Program)
/// 
/// This releases locked funds and adds settlement amount to pm_pending_settlement.
pub fn cpi_prediction_settle<'a>(
    vault_program: &AccountInfo<'a>,
    vault_config: &AccountInfo<'a>,
    user_account: &AccountInfo<'a>,
    caller_program: &AccountInfo<'a>,
    locked_amount: u64,
    settlement_amount: u64,
    signer_seeds: &[&[u8]],
) -> ProgramResult {
    msg!("CPI: Settle prediction market - locked: {}, settlement: {}", 
         locked_amount, settlement_amount);
    
    let mut data = vec![22u8]; // Instruction index (placeholder)
    data.extend_from_slice(&locked_amount.to_le_bytes());
    data.extend_from_slice(&settlement_amount.to_le_bytes());
    
    let accounts = vec![
        vault_config.clone(),
        user_account.clone(),
        caller_program.clone(),
    ];
    
    let ix = solana_program::instruction::Instruction {
        program_id: *vault_program.key,
        accounts: accounts.iter().map(|a| {
            solana_program::instruction::AccountMeta {
                pubkey: *a.key,
                is_signer: a.is_signer,
                is_writable: a.is_writable,
            }
        }).collect(),
        data,
    };
    
    invoke_signed(&ix, &accounts, &[signer_seeds])?;
    
    Ok(())
}

// ============================================================================
// Fund Program CPI
// ============================================================================

/// Collect prediction market fee (CPI to Fund Program)
pub fn cpi_collect_pm_fee<'a>(
    fund_program: &AccountInfo<'a>,
    pm_fee_config: &AccountInfo<'a>,
    fee_fund_vault: &AccountInfo<'a>,
    source_token_account: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    fee_type: u8, // 0: minting, 1: redemption, 2: trading
    amount: u64,
    signer_seeds: &[&[u8]],
) -> ProgramResult {
    msg!("CPI: Collect PM fee - type: {}, amount: {}", fee_type, amount);
    
    let mut data = vec![90u8]; // Instruction index for CollectPMFee (placeholder)
    data.push(fee_type);
    data.extend_from_slice(&amount.to_le_bytes());
    
    let accounts = vec![
        pm_fee_config.clone(),
        fee_fund_vault.clone(),
        source_token_account.clone(),
        token_program.clone(),
    ];
    
    let ix = solana_program::instruction::Instruction {
        program_id: *fund_program.key,
        accounts: accounts.iter().map(|a| {
            solana_program::instruction::AccountMeta {
                pubkey: *a.key,
                is_signer: a.is_signer,
                is_writable: a.is_writable,
            }
        }).collect(),
        data,
    };
    
    invoke_signed(&ix, &accounts, &[signer_seeds])?;
    
    Ok(())
}

/// Distribute maker reward (CPI to Fund Program)
pub fn cpi_distribute_maker_reward<'a>(
    fund_program: &AccountInfo<'a>,
    pm_fee_config: &AccountInfo<'a>,
    fee_fund_vault: &AccountInfo<'a>,
    maker_token_account: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    amount: u64,
    signer_seeds: &[&[u8]],
) -> ProgramResult {
    msg!("CPI: Distribute maker reward: {}", amount);
    
    let mut data = vec![91u8]; // Instruction index (placeholder)
    data.extend_from_slice(&amount.to_le_bytes());
    
    let accounts = vec![
        pm_fee_config.clone(),
        fee_fund_vault.clone(),
        maker_token_account.clone(),
        token_program.clone(),
    ];
    
    let ix = solana_program::instruction::Instruction {
        program_id: *fund_program.key,
        accounts: accounts.iter().map(|a| {
            solana_program::instruction::AccountMeta {
                pubkey: *a.key,
                is_signer: a.is_signer,
                is_writable: a.is_writable,
            }
        }).collect(),
        data,
    };
    
    invoke_signed(&ix, &accounts, &[signer_seeds])?;
    
    Ok(())
}

// ============================================================================
// SPL Token Program CPI
// ============================================================================

/// Mint tokens to a user account
pub fn cpi_token_mint_to<'a>(
    token_program: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    destination: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    amount: u64,
    signer_seeds: &[&[u8]],
) -> ProgramResult {
    msg!("CPI: Mint {} tokens", amount);
    
    let ix = spl_token::instruction::mint_to(
        token_program.key,
        mint.key,
        destination.key,
        authority.key,
        &[],
        amount,
    )?;
    
    invoke_signed(
        &ix,
        &[mint.clone(), destination.clone(), authority.clone()],
        &[signer_seeds],
    )?;
    
    Ok(())
}

/// Burn tokens from a user account
pub fn cpi_token_burn<'a>(
    token_program: &AccountInfo<'a>,
    source: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    amount: u64,
    signer_seeds: &[&[u8]],
) -> ProgramResult {
    msg!("CPI: Burn {} tokens", amount);
    
    let ix = spl_token::instruction::burn(
        token_program.key,
        source.key,
        mint.key,
        authority.key,
        &[],
        amount,
    )?;
    
    invoke_signed(
        &ix,
        &[source.clone(), mint.clone(), authority.clone()],
        &[signer_seeds],
    )?;
    
    Ok(())
}

/// Transfer tokens between accounts
pub fn cpi_token_transfer<'a>(
    token_program: &AccountInfo<'a>,
    source: &AccountInfo<'a>,
    destination: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    amount: u64,
    signer_seeds: &[&[u8]],
) -> ProgramResult {
    msg!("CPI: Transfer {} tokens", amount);
    
    let ix = spl_token::instruction::transfer(
        token_program.key,
        source.key,
        destination.key,
        authority.key,
        &[],
        amount,
    )?;
    
    invoke_signed(
        &ix,
        &[source.clone(), destination.clone(), authority.clone()],
        &[signer_seeds],
    )?;
    
    Ok(())
}

/// Transfer tokens (user-signed, no PDA)
pub fn cpi_token_transfer_user<'a>(
    token_program: &AccountInfo<'a>,
    source: &AccountInfo<'a>,
    destination: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    amount: u64,
) -> ProgramResult {
    msg!("CPI: Transfer {} tokens (user signed)", amount);
    
    let ix = spl_token::instruction::transfer(
        token_program.key,
        source.key,
        destination.key,
        authority.key,
        &[],
        amount,
    )?;
    
    invoke(&ix, &[source.clone(), destination.clone(), authority.clone()])?;
    
    Ok(())
}

// ============================================================================
// Validation Helpers
// ============================================================================

/// Verify that the provided program ID matches expected Vault Program
pub fn verify_vault_program(
    provided: &Pubkey,
    expected: &Pubkey,
) -> ProgramResult {
    if provided != expected {
        msg!("Vault program mismatch: expected {}, got {}", expected, provided);
        return Err(PredictionMarketError::VaultProgramMismatch.into());
    }
    Ok(())
}

/// Verify that the provided program ID matches expected Fund Program
pub fn verify_fund_program(
    provided: &Pubkey,
    expected: &Pubkey,
) -> ProgramResult {
    if provided != expected {
        msg!("Fund program mismatch: expected {}, got {}", expected, provided);
        return Err(PredictionMarketError::FundProgramMismatch.into());
    }
    Ok(())
}

/// Verify SPL Token Program
pub fn verify_token_program(provided: &Pubkey) -> ProgramResult {
    if provided != &spl_token::id() {
        msg!("Token program mismatch: expected {}, got {}", spl_token::id(), provided);
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_token_program() {
        assert!(verify_token_program(&spl_token::id()).is_ok());
        assert!(verify_token_program(&Pubkey::new_unique()).is_err());
    }
}

