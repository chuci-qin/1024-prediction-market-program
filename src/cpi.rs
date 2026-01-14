//! CPI (Cross-Program Invocation) helpers for the Prediction Market Program
//!
//! This module provides helpers for calling:
//! - Vault Program (lock/release funds, settlements)
//! - SPL Token Program (mint/burn/transfer tokens)
//!
//! NOTE: Fee collection will be implemented in Vault Program layer (V2 architecture)
//! rather than via Fund Program CPI from PM Program.

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
/// If PMUserAccount doesn't exist, it will be auto-initialized (requires payer and system_program).
/// 
/// Vault Instruction Index: 16 (PredictionMarketLock)
pub fn cpi_lock_for_prediction<'a>(
    vault_program: &AccountInfo<'a>,
    vault_config: &AccountInfo<'a>,
    user_account: &AccountInfo<'a>,
    pm_user_account: &AccountInfo<'a>,
    caller_program: &AccountInfo<'a>,
    payer: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    amount: u64,
    signer_seeds: &[&[u8]],
) -> ProgramResult {
    msg!("CPI: Lock {} for prediction market", amount);
    
    // Instruction discriminator for PredictionMarketLock = index 16
    let mut data = vec![16u8];
    data.extend_from_slice(&amount.to_le_bytes());
    
    // Include payer and system_program for auto-init if pm_user_account is empty
    let accounts = vec![
        vault_config.clone(),
        user_account.clone(),
        pm_user_account.clone(),
        caller_program.clone(),
        payer.clone(),          // Payer for auto-init
        system_program.clone(), // System program for auto-init
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
/// 
/// Vault Instruction Index: 17 (PredictionMarketUnlock)
pub fn cpi_release_from_prediction<'a>(
    vault_program: &AccountInfo<'a>,
    vault_config: &AccountInfo<'a>,
    user_account: &AccountInfo<'a>,
    pm_user_account: &AccountInfo<'a>,
    caller_program: &AccountInfo<'a>,
    amount: u64,
    signer_seeds: &[&[u8]],
) -> ProgramResult {
    msg!("CPI: Release {} from prediction market", amount);
    
    // Instruction index for PredictionMarketUnlock = 17
    let mut data = vec![17u8];
    data.extend_from_slice(&amount.to_le_bytes());
    
    let accounts = vec![
        vault_config.clone(),
        user_account.clone(),
        pm_user_account.clone(),
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
/// 
/// Vault Instruction Index: 18 (PredictionMarketSettle)
/// 
/// NOTE: This is the legacy version without auto-init support.
/// Use `cpi_prediction_settle_with_auto_init` for new code.
pub fn cpi_prediction_settle<'a>(
    vault_program: &AccountInfo<'a>,
    vault_config: &AccountInfo<'a>,
    pm_user_account: &AccountInfo<'a>,
    caller_program: &AccountInfo<'a>,
    locked_amount: u64,
    settlement_amount: u64,
    signer_seeds: &[&[u8]],
) -> ProgramResult {
    msg!("CPI: Settle prediction market - locked: {}, settlement: {}", 
         locked_amount, settlement_amount);
    
    // Instruction index for PredictionMarketSettle = 18
    let mut data = vec![18u8];
    data.extend_from_slice(&locked_amount.to_le_bytes());
    data.extend_from_slice(&settlement_amount.to_le_bytes());
    
    let accounts = vec![
        vault_config.clone(),
        pm_user_account.clone(),
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

/// Settle prediction market winnings with auto-init support (CPI to Vault Program)
/// 
/// This version supports automatic creation of PMUserAccount if it doesn't exist.
/// Pass payer, system_program, and user_wallet for auto-init capability.
/// 
/// Vault Instruction Index: 18 (PredictionMarketSettle)
pub fn cpi_prediction_settle_with_auto_init<'a>(
    vault_program: &AccountInfo<'a>,
    vault_config: &AccountInfo<'a>,
    pm_user_account: &AccountInfo<'a>,
    caller_program: &AccountInfo<'a>,
    payer: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    user_wallet: &AccountInfo<'a>,
    locked_amount: u64,
    settlement_amount: u64,
    signer_seeds: &[&[u8]],
) -> ProgramResult {
    msg!("CPI: Settle prediction market (with auto-init) - locked: {}, settlement: {}", 
         locked_amount, settlement_amount);
    
    // Instruction index for PredictionMarketSettle = 18
    let mut data = vec![18u8];
    data.extend_from_slice(&locked_amount.to_le_bytes());
    data.extend_from_slice(&settlement_amount.to_le_bytes());
    
    // Include optional accounts for auto-init
    let accounts = vec![
        vault_config.clone(),
        pm_user_account.clone(),
        caller_program.clone(),
        payer.clone(),           // For paying account creation rent
        system_program.clone(),  // System Program for create_account
        user_wallet.clone(),     // User wallet for PDA derivation
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
// V2 Fee Architecture: Vault CPI with Fee Collection
// ============================================================================

/// Lock user funds for prediction market with fee collection (CPI to Vault Program)
/// 
/// This is the V2 fee architecture version that:
/// 1. Deducts gross_amount from UserAccount.available_balance
/// 2. Reads fee rate from PM Fee Config
/// 3. Calculates and collects minting fee
/// 4. Locks net_amount to PMUserAccount
/// 
/// Vault Instruction Index: 21 (PredictionMarketLockWithFee)
/// 
/// Accounts expected by Vault:
/// 0. `[]` VaultConfig
/// 1. `[writable]` UserAccount
/// 2. `[writable]` PMUserAccount
/// 3. `[]` Caller Program (PM Config PDA)
/// 4. `[writable]` Vault Token Account
/// 5. `[writable]` PM Fee Vault
/// 6. `[writable]` PM Fee Config PDA
/// 7. `[]` Token Program
/// 8. `[signer, writable]` Payer (for auto-init)
/// 9. `[]` System Program (for auto-init)
pub fn cpi_lock_for_prediction_with_fee<'a>(
    vault_program: &AccountInfo<'a>,
    vault_config: &AccountInfo<'a>,
    user_account: &AccountInfo<'a>,
    pm_user_account: &AccountInfo<'a>,
    caller_program: &AccountInfo<'a>,
    vault_token_account: &AccountInfo<'a>,
    pm_fee_vault: &AccountInfo<'a>,
    pm_fee_config: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    payer: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    gross_amount: u64,
    signer_seeds: &[&[u8]],
) -> ProgramResult {
    msg!("CPI: Lock {} with fee for prediction market", gross_amount);
    
    // Instruction index for PredictionMarketLockWithFee = 21
    // (After RelayerDeposit=19, RelayerWithdraw=20)
    let mut data = vec![21u8];
    data.extend_from_slice(&gross_amount.to_le_bytes());
    
    let accounts = vec![
        vault_config.clone(),
        user_account.clone(),
        pm_user_account.clone(),
        caller_program.clone(),
        vault_token_account.clone(),
        pm_fee_vault.clone(),
        pm_fee_config.clone(),
        token_program.clone(),
        payer.clone(),
        system_program.clone(),
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

/// Release user funds from prediction market with fee collection (CPI to Vault Program)
/// 
/// V2 fee architecture version that collects redemption fee.
/// 
/// Vault Instruction Index: 22 (PredictionMarketUnlockWithFee)
/// 
/// Accounts:
/// 0. `[]` VaultConfig
/// 1. `[writable]` UserAccount
/// 2. `[writable]` PMUserAccount
/// 3. `[]` Caller Program
/// 4. `[writable]` Vault Token Account
/// 5. `[writable]` PM Fee Vault
/// 6. `[writable]` PM Fee Config PDA
/// 7. `[]` Token Program
pub fn cpi_release_from_prediction_with_fee<'a>(
    vault_program: &AccountInfo<'a>,
    vault_config: &AccountInfo<'a>,
    user_account: &AccountInfo<'a>,
    pm_user_account: &AccountInfo<'a>,
    caller_program: &AccountInfo<'a>,
    vault_token_account: &AccountInfo<'a>,
    pm_fee_vault: &AccountInfo<'a>,
    pm_fee_config: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    gross_amount: u64,
    signer_seeds: &[&[u8]],
) -> ProgramResult {
    msg!("CPI: Release {} with fee from prediction market", gross_amount);
    
    // Instruction index for PredictionMarketUnlockWithFee = 22
    let mut data = vec![22u8];
    data.extend_from_slice(&gross_amount.to_le_bytes());
    
    let accounts = vec![
        vault_config.clone(),
        user_account.clone(),
        pm_user_account.clone(),
        caller_program.clone(),
        vault_token_account.clone(),
        pm_fee_vault.clone(),
        pm_fee_config.clone(),
        token_program.clone(),
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

/// CPI to Vault.PredictionMarketTradeWithFee (index 23)
/// 
/// Collects trading fee from a trade. Does not modify user balances.
/// 
/// Accounts:
/// 0. VaultConfig
/// 1. Caller Program
/// 2. Vault Token Account
/// 3. PM Fee Vault
/// 4. PM Fee Config
/// 5. Token Program
pub fn cpi_trade_with_fee<'a>(
    vault_program: &AccountInfo<'a>,
    vault_config: &AccountInfo<'a>,
    caller_program: &AccountInfo<'a>,
    vault_token_account: &AccountInfo<'a>,
    pm_fee_vault: &AccountInfo<'a>,
    pm_fee_config: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    trade_amount: u64,
    is_taker: bool,
    signer_seeds: &[&[u8]],
) -> ProgramResult {
    msg!("CPI: Trade fee for amount={}, is_taker={}", trade_amount, is_taker);
    
    // Instruction index for PredictionMarketTradeWithFee = 23
    let mut data = vec![23u8];
    data.extend_from_slice(&trade_amount.to_le_bytes());
    data.push(if is_taker { 1 } else { 0 });
    
    let accounts = vec![
        vault_config.clone(),
        caller_program.clone(),
        vault_token_account.clone(),
        pm_fee_vault.clone(),
        pm_fee_config.clone(),
        token_program.clone(),
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

/// CPI to Vault.PredictionMarketSettleWithFee (index 24)
/// 
/// Settles market position with fee deduction.
/// 
/// Accounts:
/// 0. VaultConfig
/// 1. PredictionMarketUserAccount
/// 2. Caller Program
/// 3. Vault Token Account
/// 4. PM Fee Vault
/// 5. PM Fee Config
/// 6. Token Program
pub fn cpi_settle_with_fee<'a>(
    vault_program: &AccountInfo<'a>,
    vault_config: &AccountInfo<'a>,
    pm_user_account: &AccountInfo<'a>,
    caller_program: &AccountInfo<'a>,
    vault_token_account: &AccountInfo<'a>,
    pm_fee_vault: &AccountInfo<'a>,
    pm_fee_config: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    locked_amount: u64,
    settlement_amount: u64,
    signer_seeds: &[&[u8]],
) -> ProgramResult {
    msg!("CPI: Settle with fee locked={}, settlement={}", locked_amount, settlement_amount);
    
    // Instruction index for PredictionMarketSettleWithFee = 24
    let mut data = vec![24u8];
    data.extend_from_slice(&locked_amount.to_le_bytes());
    data.extend_from_slice(&settlement_amount.to_le_bytes());
    
    let accounts = vec![
        vault_config.clone(),
        pm_user_account.clone(),
        caller_program.clone(),
        vault_token_account.clone(),
        pm_fee_vault.clone(),
        pm_fee_config.clone(),
        token_program.clone(),
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
