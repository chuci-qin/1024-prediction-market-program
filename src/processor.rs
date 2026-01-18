//! Instruction processor for the Prediction Market Program

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::error::PredictionMarketError;
use crate::instruction::PredictionMarketInstruction;
use crate::state::{
    PredictionMarketConfig, Market, Order, Position, OracleProposal,
    MarketType, MarketStatus, MarketResult, ReviewStatus, OrderStatus, ProposalStatus, Outcome,
    PM_CONFIG_SEED, MARKET_SEED, ORDER_SEED, ORDER_ESCROW_SEED, POSITION_SEED, 
    MARKET_VAULT_SEED, YES_MINT_SEED, NO_MINT_SEED, ORACLE_PROPOSAL_SEED, OUTCOME_MINT_SEED,
    PM_CONFIG_DISCRIMINATOR, MARKET_DISCRIMINATOR, ORDER_DISCRIMINATOR, 
    POSITION_DISCRIMINATOR, ORACLE_PROPOSAL_DISCRIMINATOR,
    PRICE_PRECISION, MIN_PRICE, MAX_PRICE, MAX_OUTCOMES,
};
use crate::utils::{
    check_signer, get_current_timestamp,
    safe_add_u64,
    validate_price, validate_price_pair,
    deserialize_account,
};
use crate::cpi::{
    cpi_lock_for_prediction,
    cpi_release_from_prediction,
    cpi_prediction_settle,
    cpi_prediction_settle_with_auto_init,
    cpi_lock_for_prediction_with_fee,
    cpi_release_from_prediction_with_fee,
    cpi_trade_with_fee,
    cpi_settle_with_fee,
};

/// Process an instruction
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = PredictionMarketInstruction::try_from_slice(instruction_data)?;
    
    match instruction {
        // === Initialization ===
        PredictionMarketInstruction::Initialize(args) => {
            msg!("Instruction: Initialize");
            process_initialize(program_id, accounts, args)
        }
        PredictionMarketInstruction::ReinitializeConfig(args) => {
            msg!("Instruction: ReinitializeConfig");
            process_reinitialize_config(program_id, accounts, args)
        }
        
        // === Market Management ===
        PredictionMarketInstruction::CreateMarket(args) => {
            msg!("Instruction: CreateMarket");
            process_create_market(program_id, accounts, args)
        }
        PredictionMarketInstruction::ActivateMarket(args) => {
            msg!("Instruction: ActivateMarket");
            process_activate_market(program_id, accounts, args)
        }
        PredictionMarketInstruction::PauseMarket(args) => {
            msg!("Instruction: PauseMarket");
            process_pause_market(program_id, accounts, args)
        }
        PredictionMarketInstruction::ResumeMarket(args) => {
            msg!("Instruction: ResumeMarket");
            process_resume_market(program_id, accounts, args)
        }
        PredictionMarketInstruction::CancelMarket(args) => {
            msg!("Instruction: CancelMarket");
            process_cancel_market(program_id, accounts, args)
        }
        PredictionMarketInstruction::FlagMarket(args) => {
            msg!("Instruction: FlagMarket");
            process_flag_market(program_id, accounts, args)
        }
        
        // === Complete Set Operations ===
        PredictionMarketInstruction::MintCompleteSet(args) => {
            msg!("Instruction: MintCompleteSet");
            process_mint_complete_set(program_id, accounts, args)
        }
        PredictionMarketInstruction::RedeemCompleteSet(args) => {
            msg!("Instruction: RedeemCompleteSet");
            process_redeem_complete_set(program_id, accounts, args)
        }
        
        // === Order Operations ===
        PredictionMarketInstruction::PlaceOrder(args) => {
            msg!("Instruction: PlaceOrder");
            process_place_order(program_id, accounts, args)
        }
        PredictionMarketInstruction::CancelOrder(args) => {
            msg!("Instruction: CancelOrder");
            process_cancel_order(program_id, accounts, args)
        }
        // V1 指令已弃用 (2025-12-15) - 请使用 V2 版本
        PredictionMarketInstruction::MatchMint(_) => {
            msg!("❌ MatchMint V1 DEPRECATED - Use MatchMintV2");
            Err(PredictionMarketError::InstructionDeprecated.into())
        }
        PredictionMarketInstruction::MatchBurn(_) => {
            msg!("❌ MatchBurn V1 DEPRECATED - Use MatchBurnV2");
            Err(PredictionMarketError::InstructionDeprecated.into())
        }
        PredictionMarketInstruction::ExecuteTrade(_) => {
            msg!("❌ ExecuteTrade V1 DEPRECATED - Use ExecuteTradeV2");
            Err(PredictionMarketError::InstructionDeprecated.into())
        }
        
        // === Oracle / Resolution ===
        // 注意：这些功能需要从链上 V7 程序调用，本地代码被意外删除
        PredictionMarketInstruction::ProposeResult(_) => {
            msg!("⚠️ ProposeResult: Use deployed V7 program");
            Err(ProgramError::InvalidInstructionData)
        }
        PredictionMarketInstruction::ChallengeResult(_) => {
            msg!("⚠️ ChallengeResult: Use deployed V7 program");
            Err(ProgramError::InvalidInstructionData)
        }
        PredictionMarketInstruction::FinalizeResult => {
            msg!("⚠️ FinalizeResult: Use deployed V7 program");
            Err(ProgramError::InvalidInstructionData)
        }
        PredictionMarketInstruction::ResolveDispute(_) => {
            msg!("⚠️ ResolveDispute: Use deployed V7 program");
            Err(ProgramError::InvalidInstructionData)
        }
        
        // === Settlement ===
        PredictionMarketInstruction::ClaimWinnings => {
            msg!("⚠️ ClaimWinnings V1: Use RelayerClaimWinningsV2");
            Err(PredictionMarketError::InstructionDeprecated.into())
        }
        PredictionMarketInstruction::RefundCancelledMarket => {
            msg!("⚠️ RefundCancelledMarket: Use deployed V7 program");
            Err(ProgramError::InvalidInstructionData)
        }
        
        // === Admin Operations ===
        PredictionMarketInstruction::UpdateAdmin(_) => {
            msg!("⚠️ UpdateAdmin: Use deployed V7 program");
            Err(ProgramError::InvalidInstructionData)
        }
        PredictionMarketInstruction::UpdateOracleAdmin(_) => {
            msg!("⚠️ UpdateOracleAdmin: Use deployed V7 program");
            Err(ProgramError::InvalidInstructionData)
        }
        PredictionMarketInstruction::SetPaused(_) => {
            msg!("⚠️ SetPaused: Use deployed V7 program");
            Err(ProgramError::InvalidInstructionData)
        }
        PredictionMarketInstruction::UpdateOracleConfig(_) => {
            msg!("⚠️ UpdateOracleConfig: Use deployed V7 program");
            Err(ProgramError::InvalidInstructionData)
        }
        PredictionMarketInstruction::AddAuthorizedCaller(_) => {
            msg!("⚠️ AddAuthorizedCaller: Use deployed V7 program");
            Err(ProgramError::InvalidInstructionData)
        }
        PredictionMarketInstruction::RemoveAuthorizedCaller(_) => {
            msg!("⚠️ RemoveAuthorizedCaller: Use deployed V7 program");
            Err(ProgramError::InvalidInstructionData)
        }
        
        // Multi-Outcome Market Instructions
        PredictionMarketInstruction::CreateMultiOutcomeMarket(args) => {
            msg!("Instruction: CreateMultiOutcomeMarket");
            process_create_multi_outcome_market(program_id, accounts, args)
        }
        PredictionMarketInstruction::MintMultiOutcomeCompleteSet(_) => {
            msg!("⚠️ MintMultiOutcomeCompleteSet: Use deployed V7 program");
            Err(ProgramError::InvalidInstructionData)
        }
        PredictionMarketInstruction::RedeemMultiOutcomeCompleteSet(_) => {
            msg!("⚠️ RedeemMultiOutcomeCompleteSet: Use deployed V7 program");
            Err(ProgramError::InvalidInstructionData)
        }
        PredictionMarketInstruction::PlaceMultiOutcomeOrder(_) => {
            msg!("⚠️ PlaceMultiOutcomeOrder: Use deployed V7 program");
            Err(ProgramError::InvalidInstructionData)
        }
        PredictionMarketInstruction::ProposeMultiOutcomeResult(_) => {
            msg!("⚠️ ProposeMultiOutcomeResult: Use deployed V7 program");
            Err(ProgramError::InvalidInstructionData)
        }
        PredictionMarketInstruction::ClaimMultiOutcomeWinnings(_) => {
            msg!("⚠️ ClaimMultiOutcomeWinnings: Use deployed V7 program");
            Err(ProgramError::InvalidInstructionData)
        }
        
        // === Relayer Instructions ===
        // V1 Relayer 指令已弃用 (2025-12-15) - 请使用 V2 版本
        PredictionMarketInstruction::RelayerMintCompleteSet(_) => {
            msg!("❌ RelayerMintCompleteSet V1 DEPRECATED - Use RelayerMintCompleteSetV2");
            Err(PredictionMarketError::InstructionDeprecated.into())
        }
        PredictionMarketInstruction::RelayerRedeemCompleteSet(_) => {
            msg!("❌ RelayerRedeemCompleteSet V1 DEPRECATED - Use RelayerRedeemCompleteSetV2");
            Err(PredictionMarketError::InstructionDeprecated.into())
        }
        PredictionMarketInstruction::RelayerPlaceOrder(_) => {
            msg!("❌ RelayerPlaceOrder V1 DEPRECATED - Use RelayerPlaceOrderV2");
            Err(PredictionMarketError::InstructionDeprecated.into())
        }
        PredictionMarketInstruction::RelayerCancelOrder(_) => {
            msg!("❌ RelayerCancelOrder V1 DEPRECATED - Use RelayerCancelOrderV2");
            Err(PredictionMarketError::InstructionDeprecated.into())
        }
        PredictionMarketInstruction::RelayerClaimWinnings(_) => {
            msg!("❌ RelayerClaimWinnings V1 DEPRECATED - Use RelayerClaimWinningsV2");
            Err(PredictionMarketError::InstructionDeprecated.into())
        }
        PredictionMarketInstruction::RelayerRefundCancelledMarket(_) => {
            msg!("⚠️ RelayerRefundCancelledMarket: Use deployed V7 program");
            Err(ProgramError::InvalidInstructionData)
        }
        PredictionMarketInstruction::RelayerMintMultiOutcomeCompleteSet(_) => {
            msg!("⚠️ RelayerMintMultiOutcomeCompleteSet: Use deployed V7 program");
            Err(ProgramError::InvalidInstructionData)
        }
        PredictionMarketInstruction::RelayerRedeemMultiOutcomeCompleteSet(_) => {
            msg!("⚠️ RelayerRedeemMultiOutcomeCompleteSet: Use deployed V7 program");
            Err(ProgramError::InvalidInstructionData)
        }
        PredictionMarketInstruction::RelayerPlaceMultiOutcomeOrder(_) => {
            msg!("⚠️ RelayerPlaceMultiOutcomeOrder: Use deployed V7 program");
            Err(ProgramError::InvalidInstructionData)
        }
        PredictionMarketInstruction::RelayerClaimMultiOutcomeWinnings(_) => {
            msg!("⚠️ RelayerClaimMultiOutcomeWinnings: Use deployed V7 program");
            Err(ProgramError::InvalidInstructionData)
        }
        
        // === Multi-Outcome V1 指令已弃用 (2025-12-15) - 请使用 V2 版本 ===
        PredictionMarketInstruction::MatchMintMulti(_) => {
            msg!("❌ MatchMintMulti V1 DEPRECATED - Use MatchMintMultiV2");
            Err(PredictionMarketError::InstructionDeprecated.into())
        }
        PredictionMarketInstruction::MatchBurnMulti(_) => {
            msg!("❌ MatchBurnMulti V1 DEPRECATED - Use MatchBurnMultiV2");
            Err(PredictionMarketError::InstructionDeprecated.into())
        }
        PredictionMarketInstruction::RelayerMatchMintMulti(_) => {
            msg!("❌ RelayerMatchMintMulti V1 DEPRECATED - Use MatchMintMultiV2");
            Err(PredictionMarketError::InstructionDeprecated.into())
        }
        PredictionMarketInstruction::RelayerMatchBurnMulti(_) => {
            msg!("❌ RelayerMatchBurnMulti V1 DEPRECATED - Use MatchBurnMultiV2");
            Err(PredictionMarketError::InstructionDeprecated.into())
        }
        
        // === V2 Instructions (Pure Vault Mode) ===
        PredictionMarketInstruction::RelayerMintCompleteSetV2(args) => {
            msg!("Instruction: RelayerMintCompleteSetV2");
            process_relayer_mint_complete_set_v2(program_id, accounts, args)
        }
        PredictionMarketInstruction::RelayerRedeemCompleteSetV2(args) => {
            msg!("Instruction: RelayerRedeemCompleteSetV2");
            process_relayer_redeem_complete_set_v2(program_id, accounts, args)
        }
        PredictionMarketInstruction::MatchMintV2(args) => {
            msg!("Instruction: MatchMintV2");
            process_match_mint_v2(program_id, accounts, args)
        }
        PredictionMarketInstruction::MatchBurnV2(args) => {
            msg!("Instruction: MatchBurnV2");
            process_match_burn_v2(program_id, accounts, args)
        }
        PredictionMarketInstruction::RelayerClaimWinningsV2(args) => {
            msg!("Instruction: RelayerClaimWinningsV2");
            process_relayer_claim_winnings_v2(program_id, accounts, args)
        }
        PredictionMarketInstruction::ExecuteTradeV2(args) => {
            msg!("Instruction: ExecuteTradeV2");
            process_execute_trade_v2(program_id, accounts, args)
        }
        PredictionMarketInstruction::MatchMintMultiV2(args) => {
            msg!("Instruction: MatchMintMultiV2");
            process_match_mint_multi_v2(program_id, accounts, args)
        }
        PredictionMarketInstruction::MatchBurnMultiV2(args) => {
            msg!("Instruction: MatchBurnMultiV2");
            process_match_burn_multi_v2(program_id, accounts, args)
        }
        PredictionMarketInstruction::RelayerPlaceOrderV2(args) => {
            msg!("Instruction: RelayerPlaceOrderV2");
            process_relayer_place_order_v2(program_id, accounts, args)
        }
        PredictionMarketInstruction::RelayerCancelOrderV2(args) => {
            msg!("Instruction: RelayerCancelOrderV2");
            process_relayer_cancel_order_v2(program_id, accounts, args)
        }
        
        // V2 Multi-Outcome Instructions
        PredictionMarketInstruction::RelayerPlaceMultiOutcomeOrderV2(args) => {
            msg!("Instruction: RelayerPlaceMultiOutcomeOrderV2");
            process_relayer_place_multi_outcome_order_v2(program_id, accounts, args)
        }
        PredictionMarketInstruction::RelayerCancelMultiOutcomeOrderV2(args) => {
            msg!("Instruction: RelayerCancelMultiOutcomeOrderV2");
            process_relayer_cancel_multi_outcome_order_v2(program_id, accounts, args)
        }
        
        // V2 WithFee Instructions
        PredictionMarketInstruction::RelayerMintCompleteSetV2WithFee(args) => {
            msg!("Instruction: RelayerMintCompleteSetV2WithFee");
            process_relayer_mint_complete_set_v2_with_fee(program_id, accounts, args)
        }
        PredictionMarketInstruction::RelayerRedeemCompleteSetV2WithFee(args) => {
            msg!("Instruction: RelayerRedeemCompleteSetV2WithFee");
            process_relayer_redeem_complete_set_v2_with_fee(program_id, accounts, args)
        }
        
        // === LLM Oracle Instructions (Phase 4.5) ===
        PredictionMarketInstruction::InitializeMarketOracleData(args) => {
            msg!("Instruction: InitializeMarketOracleData");
            process_initialize_market_oracle_data(program_id, accounts, args)
        }
        PredictionMarketInstruction::SetCreationData(args) => {
            msg!("Instruction: SetCreationData");
            process_set_creation_data(program_id, accounts, args)
        }
        PredictionMarketInstruction::FreezeOracleConfig(args) => {
            msg!("Instruction: FreezeOracleConfig");
            process_freeze_oracle_config(program_id, accounts, args)
        }
        PredictionMarketInstruction::HaltTrading(args) => {
            msg!("Instruction: HaltTrading");
            process_halt_trading(program_id, accounts, args)
        }
        PredictionMarketInstruction::ProposeResultWithResearch(args) => {
            msg!("Instruction: ProposeResultWithResearch");
            process_propose_result_with_research(program_id, accounts, args)
        }
        PredictionMarketInstruction::ProposeResultManual(args) => {
            msg!("Instruction: ProposeResultManual");
            process_propose_result_manual(program_id, accounts, args)
        }
        PredictionMarketInstruction::ChallengeResultWithEvidence(args) => {
            msg!("Instruction: ChallengeResultWithEvidence");
            process_challenge_result_with_evidence(program_id, accounts, args)
        }
        
        // === Multi-Outcome V2 Instructions ===
        PredictionMarketInstruction::RelayerMintMultiOutcomeCompleteSetV2(args) => {
            msg!("Instruction: RelayerMintMultiOutcomeCompleteSetV2");
            process_relayer_mint_multi_outcome_complete_set_v2(program_id, accounts, args)
        }
        PredictionMarketInstruction::RelayerRedeemMultiOutcomeCompleteSetV2(args) => {
            msg!("Instruction: RelayerRedeemMultiOutcomeCompleteSetV2");
            process_relayer_redeem_multi_outcome_complete_set_v2(program_id, accounts, args)
        }
        PredictionMarketInstruction::RelayerClaimMultiOutcomeWinningsV2(args) => {
            msg!("Instruction: RelayerClaimMultiOutcomeWinningsV2");
            process_relayer_claim_multi_outcome_winnings_v2(program_id, accounts, args)
        }
        
        // === Oracle V2 Instructions (V15.1) ===
        PredictionMarketInstruction::FinalizeResultV2(args) => {
            msg!("Instruction: FinalizeResultV2");
            process_finalize_result_v2(program_id, accounts, args)
        }
        
        // === Relayer Oracle V2 Instructions ===
        PredictionMarketInstruction::RelayerChallengeResultV2(args) => {
            msg!("Instruction: RelayerChallengeResultV2");
            process_relayer_challenge_result_v2(program_id, accounts, args)
        }
    }
}

// ============================================================================
// Processor Implementations (Stubs - to be implemented)
// ============================================================================

use crate::instruction::*;

fn process_initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: InitializeArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Admin (signer)
    let admin_info = next_account_info(account_info_iter)?;
    check_signer(admin_info)?;
    
    // Account 1: PredictionMarketConfig PDA (writable)
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: USDC Mint
    let usdc_mint_info = next_account_info(account_info_iter)?;
    
    // Account 3: Vault Program
    let vault_program_info = next_account_info(account_info_iter)?;
    
    // Account 4: Fund Program
    let fund_program_info = next_account_info(account_info_iter)?;
    
    // Account 5: System Program
    let system_program_info = next_account_info(account_info_iter)?;
    
    // Verify System Program
    if *system_program_info.key != solana_program::system_program::ID {
        msg!("Error: Invalid System Program");
        return Err(ProgramError::IncorrectProgramId);
    }
    
    // Calculate PDA and verify
    let (config_pda, config_bump) = Pubkey::find_program_address(
        &[PM_CONFIG_SEED],
        program_id,
    );
    
    if *config_info.key != config_pda {
        msg!("Error: Invalid PredictionMarketConfig PDA");
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Check if already initialized
    if !config_info.data_is_empty() {
        msg!("Error: PredictionMarketConfig already initialized");
        return Err(PredictionMarketError::AlreadyInitialized.into());
    }
    
    // Create config account
    let rent = Rent::get()?;
    let space = PredictionMarketConfig::SIZE;
    let lamports = rent.minimum_balance(space);
    
    let signer_seeds: &[&[u8]] = &[PM_CONFIG_SEED, &[config_bump]];
    
    invoke_signed(
        &system_instruction::create_account(
            admin_info.key,
            config_info.key,
            lamports,
            space as u64,
            program_id,
        ),
        &[admin_info.clone(), config_info.clone(), system_program_info.clone()],
        &[signer_seeds],
    )?;
    
    // Initialize config data
    let config = PredictionMarketConfig::new(
        *admin_info.key,
        *usdc_mint_info.key,
        *vault_program_info.key,
        *fund_program_info.key,
        args.oracle_admin,
        config_bump,
    );
    
    // Apply custom settings from args
    let mut config = config;
    config.challenge_window_secs = args.challenge_window_secs;
    config.proposer_bond_e6 = args.proposer_bond_e6;
    
    // Serialize and save
    config.serialize(&mut *config_info.data.borrow_mut())?;
    
    msg!("PredictionMarketConfig initialized successfully");
    msg!("Admin: {}", admin_info.key);
    msg!("USDC Mint: {}", usdc_mint_info.key);
    msg!("Vault Program: {}", vault_program_info.key);
    msg!("Fund Program: {}", fund_program_info.key);
    msg!("Oracle Admin: {}", args.oracle_admin);
    msg!("Challenge Window: {} seconds", args.challenge_window_secs);
    msg!("Proposer Bond: {} (e6)", args.proposer_bond_e6);
    
    Ok(())
}

/// Reinitialize config - allows admin to reset config data for migration/upgrade
fn process_reinitialize_config(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: ReinitializeConfigArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Admin (signer)
    let admin_info = next_account_info(account_info_iter)?;
    check_signer(admin_info)?;
    
    // Account 1: PredictionMarketConfig PDA (writable)
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: USDC Mint
    let usdc_mint_info = next_account_info(account_info_iter)?;
    
    // Account 3: Vault Program
    let vault_program_info = next_account_info(account_info_iter)?;
    
    // Account 4: Fund Program
    let fund_program_info = next_account_info(account_info_iter)?;
    
    // Verify config PDA
    let (config_pda, config_bump) = Pubkey::find_program_address(
        &[PM_CONFIG_SEED],
        program_id,
    );
    
    if *config_info.key != config_pda {
        msg!("Error: Invalid PredictionMarketConfig PDA");
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Config must exist (this is reinitialize, not initialize)
    if config_info.data_is_empty() {
        msg!("Error: Config not initialized, use Initialize instead");
        return Err(PredictionMarketError::AccountNotInitialized.into());
    }
    
    // Load existing config to verify admin
    let existing_config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    
    // Verify caller is current admin
    if existing_config.admin != *admin_info.key {
        msg!("Error: Only admin can reinitialize config");
        return Err(PredictionMarketError::Unauthorized.into());
    }
    
    // Create new config data
    let mut new_config = PredictionMarketConfig::new(
        *admin_info.key,
        *usdc_mint_info.key,
        *vault_program_info.key,
        *fund_program_info.key,
        args.oracle_admin,
        config_bump,
    );
    
    // Apply custom settings
    new_config.challenge_window_secs = args.challenge_window_secs;
    new_config.proposer_bond_e6 = args.proposer_bond_e6;
    
    // Preserve or reset counters based on args
    if !args.reset_counters {
        new_config.next_market_id = existing_config.next_market_id;
        new_config.total_markets = existing_config.total_markets;
        new_config.active_markets = existing_config.active_markets;
        new_config.total_volume_e6 = existing_config.total_volume_e6;
        new_config.total_minted_sets = existing_config.total_minted_sets;
    }
    
    // Serialize and save
    new_config.serialize(&mut *config_info.data.borrow_mut())?;
    
    msg!("PredictionMarketConfig reinitialized successfully");
    msg!("Admin: {}", admin_info.key);
    msg!("USDC Mint: {}", usdc_mint_info.key);
    msg!("Vault Program: {}", vault_program_info.key);
    msg!("Fund Program: {}", fund_program_info.key);
    msg!("Oracle Admin: {}", args.oracle_admin);
    msg!("Reset Counters: {}", args.reset_counters);
    
    Ok(())
}

fn process_create_market(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: CreateMarketArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Creator (signer)
    let creator_info = next_account_info(account_info_iter)?;
    check_signer(creator_info)?;
    
    // Account 1: PredictionMarketConfig (writable)
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market PDA (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: YES Token Mint PDA (writable)
    let yes_mint_info = next_account_info(account_info_iter)?;
    
    // Account 4: NO Token Mint PDA (writable)
    let no_mint_info = next_account_info(account_info_iter)?;
    
    // Account 5: Market Vault PDA (writable)
    let market_vault_info = next_account_info(account_info_iter)?;
    
    // Account 6: USDC Mint
    let usdc_mint_info = next_account_info(account_info_iter)?;
    
    // Account 7: Token Program
    let token_program_info = next_account_info(account_info_iter)?;
    
    // Account 8: System Program
    let system_program_info = next_account_info(account_info_iter)?;
    
    // Account 9: Rent Sysvar
    let rent_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let mut config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        msg!("Error: Invalid PredictionMarketConfig discriminator");
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if config.is_paused {
        msg!("Error: Program is paused");
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    // Validate USDC mint matches config
    if *usdc_mint_info.key != config.usdc_mint {
        msg!("Error: USDC Mint mismatch");
        return Err(PredictionMarketError::InvalidUSDCMint.into());
    }
    
    // Validate market parameters
    let current_time = get_current_timestamp()?;
    if args.resolution_time <= current_time {
        msg!("Error: Resolution time must be in the future");
        return Err(PredictionMarketError::InvalidResolutionTime.into());
    }
    
    if args.finalization_deadline <= args.resolution_time {
        msg!("Error: Finalization deadline must be after resolution time");
        return Err(PredictionMarketError::InvalidFinalizationDeadline.into());
    }
    
    if args.creator_fee_bps > 500 {
        msg!("Error: Creator fee cannot exceed 5%");
        return Err(PredictionMarketError::CreatorFeeTooHigh.into());
    }
    
    // Allocate market_id
    let market_id = config.next_market_id;
    let market_id_bytes = market_id.to_le_bytes();
    
    // Verify Market PDA
    let (market_pda, market_bump) = Pubkey::find_program_address(
        &[MARKET_SEED, &market_id_bytes],
        program_id,
    );
    if *market_info.key != market_pda {
        msg!("Error: Invalid Market PDA");
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Verify YES Mint PDA
    let (yes_mint_pda, yes_mint_bump) = Pubkey::find_program_address(
        &[YES_MINT_SEED, &market_id_bytes],
        program_id,
    );
    if *yes_mint_info.key != yes_mint_pda {
        msg!("Error: Invalid YES Mint PDA");
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Verify NO Mint PDA
    let (no_mint_pda, no_mint_bump) = Pubkey::find_program_address(
        &[NO_MINT_SEED, &market_id_bytes],
        program_id,
    );
    if *no_mint_info.key != no_mint_pda {
        msg!("Error: Invalid NO Mint PDA");
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Verify Market Vault PDA
    let (market_vault_pda, market_vault_bump) = Pubkey::find_program_address(
        &[MARKET_VAULT_SEED, &market_id_bytes],
        program_id,
    );
    if *market_vault_info.key != market_vault_pda {
        msg!("Error: Invalid Market Vault PDA");
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let rent = Rent::get()?;
    
    // Create Market account
    let market_space = Market::SIZE;
    let market_lamports = rent.minimum_balance(market_space);
    let market_seeds: &[&[u8]] = &[MARKET_SEED, &market_id_bytes, &[market_bump]];
    
    invoke_signed(
        &system_instruction::create_account(
            creator_info.key,
            market_info.key,
            market_lamports,
            market_space as u64,
            program_id,
        ),
        &[creator_info.clone(), market_info.clone(), system_program_info.clone()],
        &[market_seeds],
    )?;
    
    // Create YES Token Mint (using SPL Token)
    let mint_space = spl_token::state::Mint::LEN;
    let mint_lamports = rent.minimum_balance(mint_space);
    let yes_mint_seeds: &[&[u8]] = &[YES_MINT_SEED, &market_id_bytes, &[yes_mint_bump]];
    
    invoke_signed(
        &system_instruction::create_account(
            creator_info.key,
            yes_mint_info.key,
            mint_lamports,
            mint_space as u64,
            token_program_info.key,
        ),
        &[creator_info.clone(), yes_mint_info.clone(), system_program_info.clone()],
        &[yes_mint_seeds],
    )?;
    
    // Initialize YES Mint (authority = Market PDA)
    invoke_signed(
        &spl_token::instruction::initialize_mint(
            token_program_info.key,
            yes_mint_info.key,
            market_info.key, // mint_authority
            Some(market_info.key), // freeze_authority
            6, // decimals
        )?,
        &[yes_mint_info.clone(), rent_info.clone()],
        &[market_seeds],
    )?;
    
    // Create NO Token Mint
    let no_mint_seeds: &[&[u8]] = &[NO_MINT_SEED, &market_id_bytes, &[no_mint_bump]];
    
    invoke_signed(
        &system_instruction::create_account(
            creator_info.key,
            no_mint_info.key,
            mint_lamports,
            mint_space as u64,
            token_program_info.key,
        ),
        &[creator_info.clone(), no_mint_info.clone(), system_program_info.clone()],
        &[no_mint_seeds],
    )?;
    
    // Initialize NO Mint (authority = Market PDA)
    invoke_signed(
        &spl_token::instruction::initialize_mint(
            token_program_info.key,
            no_mint_info.key,
            market_info.key, // mint_authority
            Some(market_info.key), // freeze_authority
            6, // decimals
        )?,
        &[no_mint_info.clone(), rent_info.clone()],
        &[market_seeds],
    )?;
    
    // Create Market Vault (USDC Token Account)
    let vault_space = spl_token::state::Account::LEN;
    let vault_lamports = rent.minimum_balance(vault_space);
    let market_vault_seeds: &[&[u8]] = &[MARKET_VAULT_SEED, &market_id_bytes, &[market_vault_bump]];
    
    invoke_signed(
        &system_instruction::create_account(
            creator_info.key,
            market_vault_info.key,
            vault_lamports,
            vault_space as u64,
            token_program_info.key,
        ),
        &[creator_info.clone(), market_vault_info.clone(), system_program_info.clone()],
        &[market_vault_seeds],
    )?;
    
    // Initialize Market Vault (owner = Market PDA)
    invoke_signed(
        &spl_token::instruction::initialize_account(
            token_program_info.key,
            market_vault_info.key,
            usdc_mint_info.key,
            market_info.key, // owner
        )?,
        &[market_vault_info.clone(), usdc_mint_info.clone(), market_info.clone(), rent_info.clone()],
        &[market_seeds],
    )?;
    
    // Initialize Market data
    let market = Market {
        discriminator: MARKET_DISCRIMINATOR,
        market_id,
        market_type: MarketType::Binary, // Binary market
        num_outcomes: 2, // YES/NO
        creator: *creator_info.key,
        question_hash: args.question_hash,
        resolution_spec_hash: args.resolution_spec_hash,
        yes_mint: *yes_mint_info.key,
        no_mint: *no_mint_info.key,
        market_vault: *market_vault_info.key,
        status: MarketStatus::Pending, // Starts as Pending, admin needs to activate
        review_status: ReviewStatus::None,
        resolution_time: args.resolution_time,
        finalization_deadline: args.finalization_deadline,
        final_result: None,
        winning_outcome_index: None, // For multi-outcome markets
        created_at: current_time,
        updated_at: current_time,
        total_minted: 0,
        total_volume_e6: 0,
        open_interest: 0,
        creator_fee_bps: args.creator_fee_bps,
        next_order_id: 1,
        bump: market_bump,
        reserved: [0u8; 60],
    };
    
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    // Update config
    config.next_market_id += 1;
    config.total_markets += 1;
    config.serialize(&mut *config_info.data.borrow_mut())?;
    
    msg!("Market created successfully");
    msg!("Market ID: {}", market_id);
    msg!("Creator: {}", creator_info.key);
    msg!("YES Mint: {}", yes_mint_info.key);
    msg!("NO Mint: {}", no_mint_info.key);
    msg!("Market Vault: {}", market_vault_info.key);
    msg!("Resolution Time: {}", args.resolution_time);
    msg!("Creator Fee: {} bps", args.creator_fee_bps);
    
    Ok(())
}

/// Create a multi-outcome prediction market
/// 
/// Account layout:
/// 0. `[signer]` Creator
/// 1. `[writable]` PredictionMarketConfig
/// 2. `[writable]` Market PDA
/// 3. `[writable]` Market Vault PDA
/// 4. `[]` USDC Mint
/// 5. `[]` Token Program
/// 6. `[]` System Program
/// 7. `[]` Rent Sysvar
/// 8..8+n. `[writable]` Outcome Token Mints (n outcomes)
fn process_create_multi_outcome_market(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: CreateMultiOutcomeMarketArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Creator (signer)
    let creator_info = next_account_info(account_info_iter)?;
    check_signer(creator_info)?;
    
    // Account 1: PredictionMarketConfig (writable)
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market PDA (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: Market Vault PDA (writable)
    let market_vault_info = next_account_info(account_info_iter)?;
    
    // Account 4: USDC Mint
    let usdc_mint_info = next_account_info(account_info_iter)?;
    
    // Account 5: Token Program
    let token_program_info = next_account_info(account_info_iter)?;
    
    // Account 6: System Program
    let system_program_info = next_account_info(account_info_iter)?;
    
    // Account 7: Rent Sysvar
    let rent_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let mut config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        msg!("Error: Invalid PredictionMarketConfig discriminator");
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if config.is_paused {
        msg!("Error: Program is paused");
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    // Validate USDC mint matches config
    if *usdc_mint_info.key != config.usdc_mint {
        msg!("Error: USDC Mint mismatch");
        return Err(PredictionMarketError::InvalidUSDCMint.into());
    }
    
    // Validate num_outcomes (2-32)
    if args.num_outcomes < 2 || args.num_outcomes as usize > MAX_OUTCOMES {
        msg!("Error: num_outcomes must be between 2 and {}", MAX_OUTCOMES);
        return Err(PredictionMarketError::InvalidArgument.into());
    }
    
    // Validate outcome_hashes length matches num_outcomes
    if args.outcome_hashes.len() != args.num_outcomes as usize {
        msg!("Error: outcome_hashes length ({}) != num_outcomes ({})", 
             args.outcome_hashes.len(), args.num_outcomes);
        return Err(PredictionMarketError::InvalidArgument.into());
    }
    
    // Validate market parameters
    let current_time = get_current_timestamp()?;
    if args.resolution_time <= current_time {
        msg!("Error: Resolution time must be in the future");
        return Err(PredictionMarketError::InvalidResolutionTime.into());
    }
    
    if args.finalization_deadline <= args.resolution_time {
        msg!("Error: Finalization deadline must be after resolution time");
        return Err(PredictionMarketError::InvalidFinalizationDeadline.into());
    }
    
    if args.creator_fee_bps > 500 {
        msg!("Error: Creator fee cannot exceed 5%");
        return Err(PredictionMarketError::CreatorFeeTooHigh.into());
    }
    
    // Allocate market_id
    let market_id = config.next_market_id;
    let market_id_bytes = market_id.to_le_bytes();
    
    // Verify Market PDA
    let (market_pda, market_bump) = Pubkey::find_program_address(
        &[MARKET_SEED, &market_id_bytes],
        program_id,
    );
    if *market_info.key != market_pda {
        msg!("Error: Invalid Market PDA");
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Verify Market Vault PDA
    let (market_vault_pda, market_vault_bump) = Pubkey::find_program_address(
        &[MARKET_VAULT_SEED, &market_id_bytes],
        program_id,
    );
    if *market_vault_info.key != market_vault_pda {
        msg!("Error: Invalid Market Vault PDA");
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let rent = Rent::get()?;
    
    // Create Market account
    let market_space = Market::SIZE;
    let market_lamports = rent.minimum_balance(market_space);
    let market_seeds: &[&[u8]] = &[MARKET_SEED, &market_id_bytes, &[market_bump]];
    
    invoke_signed(
        &system_instruction::create_account(
            creator_info.key,
            market_info.key,
            market_lamports,
            market_space as u64,
            program_id,
        ),
        &[creator_info.clone(), market_info.clone(), system_program_info.clone()],
        &[market_seeds],
    )?;
    
    // Create Market Vault (USDC Token Account)
    let vault_space = spl_token::state::Account::LEN;
    let vault_lamports = rent.minimum_balance(vault_space);
    let vault_seeds: &[&[u8]] = &[MARKET_VAULT_SEED, &market_id_bytes, &[market_vault_bump]];
    
    invoke_signed(
        &system_instruction::create_account(
            creator_info.key,
            market_vault_info.key,
            vault_lamports,
            vault_space as u64,
            token_program_info.key,
        ),
        &[creator_info.clone(), market_vault_info.clone(), system_program_info.clone()],
        &[vault_seeds],
    )?;
    
    // Initialize Market Vault (owner = Market PDA)
    invoke_signed(
        &spl_token::instruction::initialize_account(
            token_program_info.key,
            market_vault_info.key,
            usdc_mint_info.key,
            market_info.key, // owner = Market PDA
        )?,
        &[
            market_vault_info.clone(),
            usdc_mint_info.clone(),
            market_info.clone(),
            rent_info.clone(),
        ],
        &[vault_seeds],
    )?;
    
    // Create and initialize outcome token mints
    let mint_space = spl_token::state::Mint::LEN;
    let mint_lamports = rent.minimum_balance(mint_space);
    
    // We'll collect the first outcome mint as yes_mint and second as no_mint for compatibility
    // (even though multi-outcome markets don't really use yes/no terminology)
    let mut first_outcome_mint = Pubkey::default();
    let mut second_outcome_mint = Pubkey::default();
    
    for outcome_index in 0..args.num_outcomes {
        let outcome_index_bytes = [outcome_index];
        
        // Derive Outcome Mint PDA
        let (outcome_mint_pda, outcome_mint_bump) = Pubkey::find_program_address(
            &[OUTCOME_MINT_SEED, &market_id_bytes, &outcome_index_bytes],
            program_id,
        );
        
        // Get the account info for this outcome mint (from remaining accounts)
        let outcome_mint_info = next_account_info(account_info_iter)?;
        if *outcome_mint_info.key != outcome_mint_pda {
            msg!("Error: Invalid Outcome Mint PDA for outcome {}", outcome_index);
            return Err(PredictionMarketError::InvalidPDA.into());
        }
        
        // Create Outcome Mint account
        let outcome_mint_seeds: &[&[u8]] = &[
            OUTCOME_MINT_SEED, 
            &market_id_bytes, 
            &outcome_index_bytes, 
            &[outcome_mint_bump]
        ];
        
        invoke_signed(
            &system_instruction::create_account(
                creator_info.key,
                outcome_mint_info.key,
                mint_lamports,
                mint_space as u64,
                token_program_info.key,
            ),
            &[creator_info.clone(), outcome_mint_info.clone(), system_program_info.clone()],
            &[outcome_mint_seeds],
        )?;
        
        // Initialize Outcome Mint (authority = Market PDA)
        invoke_signed(
            &spl_token::instruction::initialize_mint(
                token_program_info.key,
                outcome_mint_info.key,
                market_info.key, // mint_authority
                Some(market_info.key), // freeze_authority
                6, // decimals (same as USDC)
            )?,
            &[
                outcome_mint_info.clone(),
                rent_info.clone(),
            ],
            &[outcome_mint_seeds],
        )?;
        
        // Store first two outcome mints for compatibility with binary market fields
        if outcome_index == 0 {
            first_outcome_mint = *outcome_mint_info.key;
        } else if outcome_index == 1 {
            second_outcome_mint = *outcome_mint_info.key;
        }
        
        msg!("Created Outcome {} Mint: {}", outcome_index, outcome_mint_info.key);
    }
    
    // Initialize market data
    let market = Market {
        discriminator: MARKET_DISCRIMINATOR,
        market_id,
        market_type: MarketType::MultiOutcome, // Multi-outcome market
        num_outcomes: args.num_outcomes,
        creator: *creator_info.key,
        question_hash: args.question_hash,
        resolution_spec_hash: args.resolution_spec_hash,
        yes_mint: first_outcome_mint, // First outcome mint for compatibility
        no_mint: second_outcome_mint, // Second outcome mint for compatibility
        market_vault: *market_vault_info.key,
        status: MarketStatus::Pending, // Starts as Pending, admin needs to activate
        review_status: ReviewStatus::None,
        resolution_time: args.resolution_time,
        finalization_deadline: args.finalization_deadline,
        final_result: None,
        winning_outcome_index: None,
        created_at: current_time,
        updated_at: current_time,
        total_minted: 0,
        total_volume_e6: 0,
        open_interest: 0,
        creator_fee_bps: args.creator_fee_bps,
        next_order_id: 1,
        bump: market_bump,
        reserved: [0u8; 60],
    };
    
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    // Update config
    config.next_market_id += 1;
    config.total_markets += 1;
    config.serialize(&mut *config_info.data.borrow_mut())?;
    
    msg!("✅ Multi-outcome market created successfully");
    msg!("Market ID: {}", market_id);
    msg!("Creator: {}", creator_info.key);
    msg!("Num Outcomes: {}", args.num_outcomes);
    msg!("Market Vault: {}", market_vault_info.key);
    msg!("Resolution Time: {}", args.resolution_time);
    msg!("Creator Fee: {} bps", args.creator_fee_bps);
    
    Ok(())
}

fn process_activate_market(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: ActivateMarketArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Admin (signer)
    let admin_info = next_account_info(account_info_iter)?;
    check_signer(admin_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let config_data = config_info.data.borrow();
    let mut config = PredictionMarketConfig::deserialize(&mut &config_data[..])?;
    drop(config_data);
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify admin
    if *admin_info.key != config.admin {
        msg!("Error: Only admin can activate markets");
        return Err(PredictionMarketError::Unauthorized.into());
    }
    
    // Verify Market PDA
    let market_id_bytes = args.market_id.to_le_bytes();
    let (market_pda, _) = Pubkey::find_program_address(
        &[MARKET_SEED, &market_id_bytes],
        program_id,
    );
    if *market_info.key != market_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load market
    let market_data = market_info.data.borrow();
    let mut market = Market::deserialize(&mut &market_data[..])?;
    drop(market_data);
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify market is in Pending status
    if market.status != MarketStatus::Pending {
        msg!("Error: Market must be in Pending status to activate");
        return Err(PredictionMarketError::InvalidMarketStatus.into());
    }
    
    // Activate market
    let current_time = get_current_timestamp()?;
    market.status = MarketStatus::Active;
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    // Update config
    config.active_markets += 1;
    config.serialize(&mut *config_info.data.borrow_mut())?;
    
    msg!("Market {} activated successfully", args.market_id);
    
    Ok(())
}

fn process_pause_market(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: PauseMarketArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let admin_info = next_account_info(account_info_iter)?;
    check_signer(admin_info)?;
    
    let config_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let mut config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify admin
    if *admin_info.key != config.admin {
        msg!("Error: Only admin can pause markets");
        return Err(PredictionMarketError::Unauthorized.into());
    }
    
    // Verify Market PDA
    let market_id_bytes = args.market_id.to_le_bytes();
    let (market_pda, _) = Pubkey::find_program_address(
        &[MARKET_SEED, &market_id_bytes],
        program_id,
    );
    if *market_info.key != market_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load market
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify market is Active
    if market.status != MarketStatus::Active {
        msg!("Error: Can only pause active markets");
        return Err(PredictionMarketError::InvalidMarketStatus.into());
    }
    
    // Pause market
    let current_time = get_current_timestamp()?;
    market.status = MarketStatus::Paused;
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    // Update config
    config.active_markets = config.active_markets.saturating_sub(1);
    config.serialize(&mut *config_info.data.borrow_mut())?;
    
    msg!("Market {} paused successfully", args.market_id);
    
    Ok(())
}

fn process_resume_market(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: ResumeMarketArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let admin_info = next_account_info(account_info_iter)?;
    check_signer(admin_info)?;
    
    let config_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let mut config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify admin
    if *admin_info.key != config.admin {
        return Err(PredictionMarketError::Unauthorized.into());
    }
    
    // Verify Market PDA
    let market_id_bytes = args.market_id.to_le_bytes();
    let (market_pda, _) = Pubkey::find_program_address(
        &[MARKET_SEED, &market_id_bytes],
        program_id,
    );
    if *market_info.key != market_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load market
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify market is Paused
    if market.status != MarketStatus::Paused {
        msg!("Error: Can only resume paused markets");
        return Err(PredictionMarketError::InvalidMarketStatus.into());
    }
    
    // Resume market
    let current_time = get_current_timestamp()?;
    market.status = MarketStatus::Active;
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    // Update config
    config.active_markets += 1;
    config.serialize(&mut *config_info.data.borrow_mut())?;
    
    msg!("Market {} resumed successfully", args.market_id);
    
    Ok(())
}

fn process_cancel_market(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: CancelMarketArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let admin_info = next_account_info(account_info_iter)?;
    check_signer(admin_info)?;
    
    let config_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let mut config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify admin
    if *admin_info.key != config.admin {
        return Err(PredictionMarketError::Unauthorized.into());
    }
    
    // Verify Market PDA
    let market_id_bytes = args.market_id.to_le_bytes();
    let (market_pda, _) = Pubkey::find_program_address(
        &[MARKET_SEED, &market_id_bytes],
        program_id,
    );
    if *market_info.key != market_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load market
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify market is not already resolved or cancelled
    if market.status == MarketStatus::Resolved || market.status == MarketStatus::Cancelled {
        msg!("Error: Cannot cancel resolved or already cancelled markets");
        return Err(PredictionMarketError::InvalidMarketStatus.into());
    }
    
    let was_active = market.status == MarketStatus::Active;
    
    // Cancel market
    let current_time = get_current_timestamp()?;
    market.status = MarketStatus::Cancelled;
    // Convert reason u8 to ReviewStatus
    market.review_status = match args.reason {
        1 => ReviewStatus::Flagged,
        2 => ReviewStatus::CancelledInvalid,
        3 => ReviewStatus::CancelledRegulatory,
        _ => ReviewStatus::None,
    };
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    // Update config if was active
    if was_active {
        config.active_markets = config.active_markets.saturating_sub(1);
        config.serialize(&mut *config_info.data.borrow_mut())?;
    }
    
    msg!("Market {} cancelled successfully. Reason: {}", args.market_id, args.reason);
    
    Ok(())
}

fn process_flag_market(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: FlagMarketArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let admin_info = next_account_info(account_info_iter)?;
    check_signer(admin_info)?;
    
    let config_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify admin
    if *admin_info.key != config.admin {
        return Err(PredictionMarketError::Unauthorized.into());
    }
    
    // Verify Market PDA
    let market_id_bytes = args.market_id.to_le_bytes();
    let (market_pda, _) = Pubkey::find_program_address(
        &[MARKET_SEED, &market_id_bytes],
        program_id,
    );
    if *market_info.key != market_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load market
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Flag market
    let current_time = get_current_timestamp()?;
    market.review_status = ReviewStatus::Flagged;
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    msg!("Market {} flagged for review", args.market_id);
    
    Ok(())
}

fn process_mint_complete_set(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: MintCompleteSetArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: User (signer)
    let user_info = next_account_info(account_info_iter)?;
    check_signer(user_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: Market Vault (writable)
    let market_vault_info = next_account_info(account_info_iter)?;
    
    // Account 4: User's USDC Account (writable)
    let user_usdc_info = next_account_info(account_info_iter)?;
    
    // Account 5: YES Token Mint (writable)
    let yes_mint_info = next_account_info(account_info_iter)?;
    
    // Account 6: NO Token Mint (writable)
    let no_mint_info = next_account_info(account_info_iter)?;
    
    // Account 7: User's YES Token Account (writable)
    let user_yes_info = next_account_info(account_info_iter)?;
    
    // Account 8: User's NO Token Account (writable)
    let user_no_info = next_account_info(account_info_iter)?;
    
    // Account 9: Position PDA (writable)
    let position_info = next_account_info(account_info_iter)?;
    
    // Account 10: Token Program
    let token_program_info = next_account_info(account_info_iter)?;
    
    // Account 11: System Program
    let system_program_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let mut config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if config.is_paused {
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    // Load and validate market
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify market is tradeable
    if !market.is_tradeable() {
        msg!("Error: Market is not tradeable");
        return Err(PredictionMarketError::MarketNotTradeable.into());
    }
    
    // Verify market vault
    if *market_vault_info.key != market.market_vault {
        return Err(PredictionMarketError::InvalidMarketVault.into());
    }
    
    // Verify mints
    if *yes_mint_info.key != market.yes_mint {
        return Err(PredictionMarketError::InvalidYesMint.into());
    }
    if *no_mint_info.key != market.no_mint {
        return Err(PredictionMarketError::InvalidNoMint.into());
    }
    
    // Validate amount
    if args.amount == 0 {
        return Err(PredictionMarketError::InvalidAmount.into());
    }
    
    let current_time = get_current_timestamp()?;
    
    // Calculate market PDA seeds for signing
    let market_id_bytes = market.market_id.to_le_bytes();
    let market_seeds: &[&[u8]] = &[MARKET_SEED, &market_id_bytes, &[market.bump]];
    
    // NOTE: Fee collection will be implemented in Vault Program layer (V2 architecture)
    // This V1 instruction does not collect fees
    
    // Transfer USDC from user to market vault
    invoke(
        &spl_token::instruction::transfer(
            token_program_info.key,
            user_usdc_info.key,
            market_vault_info.key,
            user_info.key,
            &[],
            args.amount,
        )?,
        &[user_usdc_info.clone(), market_vault_info.clone(), user_info.clone(), token_program_info.clone()],
    )?;
    
    // Mint YES tokens to user
    invoke_signed(
        &spl_token::instruction::mint_to(
            token_program_info.key,
            yes_mint_info.key,
            user_yes_info.key,
            market_info.key, // mint authority
            &[],
            args.amount,
        )?,
        &[yes_mint_info.clone(), user_yes_info.clone(), market_info.clone(), token_program_info.clone()],
        &[market_seeds],
    )?;
    
    // Mint NO tokens to user
    invoke_signed(
        &spl_token::instruction::mint_to(
            token_program_info.key,
            no_mint_info.key,
            user_no_info.key,
            market_info.key, // mint authority
            &[],
            args.amount,
        )?,
        &[no_mint_info.clone(), user_no_info.clone(), market_info.clone(), token_program_info.clone()],
        &[market_seeds],
    )?;
    
    // Load or create Position
    let (position_pda, position_bump) = Pubkey::find_program_address(
        &[POSITION_SEED, &market_id_bytes, user_info.key.as_ref()],
        program_id,
    );
    
    if *position_info.key != position_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    msg!("DEBUG: Position PDA verified, is_empty={}", position_info.data_is_empty());
    
    if position_info.data_is_empty() {
        // Create new position using create_pda_account helper
        let rent = Rent::get()?;
        let space = Position::SIZE;
        let lamports = rent.minimum_balance(space);
        let position_seeds: &[&[u8]] = &[POSITION_SEED, &market_id_bytes, user_info.key.as_ref(), &[position_bump]];
        
        msg!("Creating position account, space={}", space);
        
        // Use invoke_signed to create account  
        invoke_signed(
            &system_instruction::create_account(
                user_info.key,
                position_info.key,
                lamports,
                space as u64,
                program_id,
            ),
            &[user_info.clone(), position_info.clone(), system_program_info.clone()],
            &[position_seeds],
        )?;
        
        // After CPI, the position_info.data should be updated
        // But we need to use try_borrow_mut to access the newly allocated data
        msg!("Position account created, data len after CPI = {}", position_info.data_len());
        
        // Initialize position data
        let position = Position::new(market.market_id, *user_info.key, position_bump, current_time);
        
        // Serialize using the data_len() which should reflect the new size
        let mut data = position_info.try_borrow_mut_data()?;
        position.serialize(&mut data.as_mut())?;
        drop(data);
        
        msg!("Position initialized successfully");
    }
    
    // Update position - use try_borrow_data to ensure we get the latest data
    let position_data = position_info.try_borrow_data()?;
    let mut position = Position::deserialize(&mut &position_data[..])?;
    drop(position_data);
    
    if position.discriminator != POSITION_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // For complete set, cost is at $0.50 each (1 USDC total for YES + NO)
    let half_price = PRICE_PRECISION / 2; // 500_000
    position.add_tokens(crate::state::Outcome::Yes, args.amount, half_price, current_time);
    position.add_tokens(crate::state::Outcome::No, args.amount, half_price, current_time);
    
    // Serialize position back to account
    let mut position_data = position_info.try_borrow_mut_data()?;
    position.serialize(&mut position_data.as_mut())?;
    drop(position_data);
    
    // Update market stats
    market.total_minted += args.amount;
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    // Update config stats
    config.total_minted_sets += args.amount;
    config.serialize(&mut *config_info.data.borrow_mut())?;
    
    msg!("Minted complete set successfully");
    msg!("Amount: {} (YES + NO)", args.amount);
    msg!("User: {}", user_info.key);
    msg!("Market ID: {}", market.market_id);
    
    Ok(())
}

fn process_redeem_complete_set(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RedeemCompleteSetArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: User (signer)
    let user_info = next_account_info(account_info_iter)?;
    check_signer(user_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: Market Vault (writable)
    let market_vault_info = next_account_info(account_info_iter)?;
    
    // Account 4: User's USDC Account (writable)
    let user_usdc_info = next_account_info(account_info_iter)?;
    
    // Account 5: YES Token Mint (writable)
    let yes_mint_info = next_account_info(account_info_iter)?;
    
    // Account 6: NO Token Mint (writable)
    let no_mint_info = next_account_info(account_info_iter)?;
    
    // Account 7: User's YES Token Account (writable)
    let user_yes_info = next_account_info(account_info_iter)?;
    
    // Account 8: User's NO Token Account (writable)
    let user_no_info = next_account_info(account_info_iter)?;
    
    // Account 9: Position PDA (writable)
    let position_info = next_account_info(account_info_iter)?;
    
    // Account 10: Token Program
    let token_program_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if config.is_paused {
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    // Load and validate market
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify market is tradeable
    if !market.is_tradeable() {
        msg!("Error: Market is not tradeable");
        return Err(PredictionMarketError::MarketNotTradeable.into());
    }
    
    // Verify addresses
    if *market_vault_info.key != market.market_vault {
        return Err(PredictionMarketError::InvalidMarketVault.into());
    }
    if *yes_mint_info.key != market.yes_mint {
        return Err(PredictionMarketError::InvalidYesMint.into());
    }
    if *no_mint_info.key != market.no_mint {
        return Err(PredictionMarketError::InvalidNoMint.into());
    }
    
    // Validate amount
    if args.amount == 0 {
        return Err(PredictionMarketError::InvalidAmount.into());
    }
    
    // Verify Position PDA
    let market_id_bytes = market.market_id.to_le_bytes();
    let (position_pda, _) = Pubkey::find_program_address(
        &[POSITION_SEED, &market_id_bytes, user_info.key.as_ref()],
        program_id,
    );
    if *position_info.key != position_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load position
    let mut position = deserialize_account::<Position>(&position_info.data.borrow())?;
    if position.discriminator != POSITION_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify user has enough tokens
    if position.yes_amount < args.amount || position.no_amount < args.amount {
        msg!("Error: Insufficient token balance for redemption");
        return Err(PredictionMarketError::InsufficientTokenBalance.into());
    }
    
    let current_time = get_current_timestamp()?;
    
    // Calculate market PDA seeds for signing
    let market_seeds: &[&[u8]] = &[MARKET_SEED, &market_id_bytes, &[market.bump]];
    
    // Burn YES tokens from user
    invoke(
        &spl_token::instruction::burn(
            token_program_info.key,
            user_yes_info.key,
            yes_mint_info.key,
            user_info.key,
            &[],
            args.amount,
        )?,
        &[user_yes_info.clone(), yes_mint_info.clone(), user_info.clone(), token_program_info.clone()],
    )?;
    
    // Burn NO tokens from user
    invoke(
        &spl_token::instruction::burn(
            token_program_info.key,
            user_no_info.key,
            no_mint_info.key,
            user_info.key,
            &[],
            args.amount,
        )?,
        &[user_no_info.clone(), no_mint_info.clone(), user_info.clone(), token_program_info.clone()],
    )?;
    
    // NOTE: Fee collection will be implemented in Vault Program layer (V2 architecture)
    // This V1 instruction does not collect fees
    
    // Transfer USDC from market vault to user
    invoke_signed(
        &spl_token::instruction::transfer(
            token_program_info.key,
            market_vault_info.key,
            user_usdc_info.key,
            market_info.key, // owner
            &[],
            args.amount,
        )?,
        &[market_vault_info.clone(), user_usdc_info.clone(), market_info.clone(), token_program_info.clone()],
        &[market_seeds],
    )?;
    
    // Update position
    let half_price = PRICE_PRECISION / 2;
    position.remove_tokens(crate::state::Outcome::Yes, args.amount, half_price, current_time);
    position.remove_tokens(crate::state::Outcome::No, args.amount, half_price, current_time);
    position.serialize(&mut *position_info.data.borrow_mut())?;
    
    // Update market stats
    market.total_minted = market.total_minted.saturating_sub(args.amount);
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    msg!("Redeemed complete set successfully");
    msg!("Amount: {}", args.amount);
    msg!("User: {}", user_info.key);
    msg!("Market ID: {}", market.market_id);
    
    Ok(())
}

fn process_place_order(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: PlaceOrderArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: User (signer)
    let user_info = next_account_info(account_info_iter)?;
    check_signer(user_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: Order PDA (writable)
    let order_info = next_account_info(account_info_iter)?;
    
    // Account 4: System Program
    let system_program_info = next_account_info(account_info_iter)?;
    
    // Additional accounts for Sell orders (optional):
    // Account 5: Token Mint (YES or NO based on outcome)
    // Account 6: User's Token Account (for the token being sold)
    // Account 7: Escrow Token Account (writable, PDA)
    // Account 8: Token Program
    // Account 9: Rent Sysvar
    
    let is_sell_order = args.side == crate::state::OrderSide::Sell;
    
    // Load and validate config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if config.is_paused {
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    // Verify Market PDA
    let market_id_bytes = args.market_id.to_le_bytes();
    let (market_pda, _) = Pubkey::find_program_address(
        &[MARKET_SEED, &market_id_bytes],
        program_id,
    );
    if *market_info.key != market_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load and validate market
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if !market.is_tradeable() {
        msg!("Error: Market is not tradeable");
        return Err(PredictionMarketError::MarketNotTradeable.into());
    }
    
    // Validate order parameters
    validate_price(args.price)?;
    
    if args.amount == 0 {
        return Err(PredictionMarketError::InvalidAmount.into());
    }
    
    let current_time = get_current_timestamp()?;
    
    // Validate expiration for GTD orders
    if args.order_type == crate::state::OrderType::GTD {
        if let Some(exp_time) = args.expiration_time {
            if exp_time <= current_time {
                msg!("Error: Expiration time must be in the future");
                return Err(PredictionMarketError::InvalidExpirationTime.into());
            }
        } else {
            msg!("Error: GTD orders require expiration time");
            return Err(PredictionMarketError::MissingExpirationTime.into());
        }
    }
    
    // Log IOC/FOK order type for off-chain matching engine reference
    // IOC (Immediate Or Cancel): Matching engine should match what's possible, 
    //     then call CancelOrder on remaining amount
    // FOK (Fill Or Kill): Matching engine should only match if entire order 
    //     can be filled, otherwise reject the order entirely
    match args.order_type {
        crate::state::OrderType::IOC => {
            msg!("📝 IOC order: Will be partially filled or cancelled by matching engine");
        }
        crate::state::OrderType::FOK => {
            msg!("📝 FOK order: Must be completely filled or will be rejected");
        }
        crate::state::OrderType::GTD => {
            msg!("📝 GTD order: Valid until {:?}", args.expiration_time);
        }
        crate::state::OrderType::GTC => {
            msg!("📝 GTC order: Good till cancelled");
        }
    }
    
    // Allocate order_id
    let order_id = market.next_order_id;
    let order_id_bytes = order_id.to_le_bytes();
    
    // Verify Order PDA
    let (order_pda, order_bump) = Pubkey::find_program_address(
        &[ORDER_SEED, &market_id_bytes, &order_id_bytes],
        program_id,
    );
    if *order_info.key != order_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Create Order account
    let rent = Rent::get()?;
    let space = Order::SIZE;
    let lamports = rent.minimum_balance(space);
    let order_seeds: &[&[u8]] = &[ORDER_SEED, &market_id_bytes, &order_id_bytes, &[order_bump]];
    
    invoke_signed(
        &system_instruction::create_account(
            user_info.key,
            order_info.key,
            lamports,
            space as u64,
            program_id,
        ),
        &[user_info.clone(), order_info.clone(), system_program_info.clone()],
        &[order_seeds],
    )?;
    
    // For sell orders, create escrow and lock tokens
    let escrow_token_account: Option<Pubkey> = if is_sell_order {
        // Get additional accounts for sell order
        let token_mint_info = next_account_info(account_info_iter)?;
        let user_token_info = next_account_info(account_info_iter)?;
        let escrow_token_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let rent_sysvar_info = next_account_info(account_info_iter)?;
        
        // Verify token mint matches the outcome
        let expected_mint = match args.outcome {
            crate::state::Outcome::Yes => market.yes_mint,
            crate::state::Outcome::No => market.no_mint,
        };
        if *token_mint_info.key != expected_mint {
            msg!("Error: Token mint does not match outcome");
            return Err(PredictionMarketError::InvalidTokenMint.into());
        }
        
        // Derive escrow PDA
        let (escrow_pda, escrow_bump) = Pubkey::find_program_address(
            &[ORDER_ESCROW_SEED, &market_id_bytes, &order_id_bytes],
            program_id,
        );
        if *escrow_token_info.key != escrow_pda {
            return Err(PredictionMarketError::InvalidPDA.into());
        }
        
        // Create escrow token account (owned by order PDA)
        let escrow_seeds: &[&[u8]] = &[ORDER_ESCROW_SEED, &market_id_bytes, &order_id_bytes, &[escrow_bump]];
        
        let rent = Rent::from_account_info(rent_sysvar_info)?;
        let space = spl_token::state::Account::LEN;
        let lamports = rent.minimum_balance(space);
        
        // Create the escrow account
        invoke_signed(
            &system_instruction::create_account(
                user_info.key,
                escrow_token_info.key,
                lamports,
                space as u64,
                &spl_token::id(),
            ),
            &[user_info.clone(), escrow_token_info.clone(), system_program_info.clone()],
            &[escrow_seeds],
        )?;
        
        // Initialize the escrow token account with order PDA as owner
        invoke(
            &spl_token::instruction::initialize_account3(
                token_program_info.key,
                escrow_token_info.key,
                token_mint_info.key,
                order_info.key, // Order PDA is the owner
            )?,
            &[escrow_token_info.clone(), token_mint_info.clone()],
        )?;
        
        // Transfer tokens from user to escrow
        invoke(
            &spl_token::instruction::transfer(
                token_program_info.key,
                user_token_info.key,
                escrow_token_info.key,
                user_info.key,
                &[],
                args.amount,
            )?,
            &[user_token_info.clone(), escrow_token_info.clone(), user_info.clone(), token_program_info.clone()],
        )?;
        
        msg!("Tokens locked in escrow: {}", args.amount);
        Some(escrow_pda)
    } else {
        None
    };
    
    // Initialize order
    // Derive outcome_index from outcome for binary markets
    let outcome_index = match args.outcome {
        Outcome::Yes => 0u8,
        Outcome::No => 1u8,
    };
    
    let order = Order {
        discriminator: ORDER_DISCRIMINATOR,
        order_id,
        market_id: args.market_id,
        owner: *user_info.key,
        side: args.side,
        outcome: args.outcome,
        outcome_index,
        price: args.price,
        amount: args.amount,
        filled_amount: 0,
        status: OrderStatus::Open,
        order_type: args.order_type,
        expiration_time: args.expiration_time,
        created_at: current_time,
        updated_at: current_time,
        bump: order_bump,
        escrow_token_account,
        reserved: [0u8; 30],
    };
    
    order.serialize(&mut *order_info.data.borrow_mut())?;
    
    // Update market
    market.next_order_id += 1;
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    msg!("Order placed successfully");
    msg!("Order ID: {}", order_id);
    msg!("Market ID: {}", args.market_id);
    msg!("Side: {:?}, Outcome: {:?}", args.side, args.outcome);
    msg!("Price: {} (e6), Amount: {}", args.price, args.amount);
    
    Ok(())
}

fn process_cancel_order(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: CancelOrderArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: User (signer)
    let user_info = next_account_info(account_info_iter)?;
    check_signer(user_info)?;
    
    // Account 1: Market
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 2: Order PDA (writable)
    let order_info = next_account_info(account_info_iter)?;
    
    // Verify Market PDA
    let market_id_bytes = args.market_id.to_le_bytes();
    let (market_pda, _) = Pubkey::find_program_address(
        &[MARKET_SEED, &market_id_bytes],
        program_id,
    );
    if *market_info.key != market_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Verify Order PDA
    let order_id_bytes = args.order_id.to_le_bytes();
    let (order_pda, order_bump) = Pubkey::find_program_address(
        &[ORDER_SEED, &market_id_bytes, &order_id_bytes],
        program_id,
    );
    if *order_info.key != order_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load order
    let mut order = deserialize_account::<Order>(&order_info.data.borrow())?;
    if order.discriminator != ORDER_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify owner
    if order.owner != *user_info.key {
        msg!("Error: Only order owner can cancel");
        return Err(PredictionMarketError::Unauthorized.into());
    }
    
    // Verify order is active
    if !order.is_active() {
        msg!("Error: Order is not active");
        return Err(PredictionMarketError::OrderNotActive.into());
    }
    
    let current_time = get_current_timestamp()?;
    let remaining_amount = order.remaining_amount();
    
    // If sell order with escrow, return tokens to user
    if order.has_escrow() {
        // Additional accounts for returning escrowed tokens:
        // Account 3: User's Token Account (writable)
        // Account 4: Escrow Token Account (writable)
        // Account 5: Token Program
        let user_token_info = next_account_info(account_info_iter)?;
        let escrow_token_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        
        // Verify escrow PDA
        let (escrow_pda, _) = Pubkey::find_program_address(
            &[ORDER_ESCROW_SEED, &market_id_bytes, &order_id_bytes],
            program_id,
        );
        if *escrow_token_info.key != escrow_pda {
            return Err(PredictionMarketError::InvalidPDA.into());
        }
        
        // Transfer remaining tokens back to user (using order PDA as signer)
        let order_seeds: &[&[u8]] = &[ORDER_SEED, &market_id_bytes, &order_id_bytes, &[order_bump]];
        
        if remaining_amount > 0 {
            invoke_signed(
                &spl_token::instruction::transfer(
                    token_program_info.key,
                    escrow_token_info.key,
                    user_token_info.key,
                    order_info.key, // Order PDA is the owner
                    &[],
                    remaining_amount,
                )?,
                &[escrow_token_info.clone(), user_token_info.clone(), order_info.clone(), token_program_info.clone()],
                &[order_seeds],
            )?;
            
            msg!("Returned {} tokens from escrow", remaining_amount);
        }
        
        // Close escrow account and return lamports to user
        invoke_signed(
            &spl_token::instruction::close_account(
                token_program_info.key,
                escrow_token_info.key,
                user_info.key,
                order_info.key,
                &[],
            )?,
            &[escrow_token_info.clone(), user_info.clone(), order_info.clone(), token_program_info.clone()],
            &[order_seeds],
        )?;
        
        msg!("Closed escrow token account");
    }
    
    // Cancel order
    order.status = OrderStatus::Cancelled;
    order.updated_at = current_time;
    order.serialize(&mut *order_info.data.borrow_mut())?;
    
    msg!("Order cancelled successfully");
    msg!("Order ID: {}", args.order_id);
    msg!("Market ID: {}", args.market_id);
    msg!("Returned amount: {}", remaining_amount);
    
    Ok(())
}

fn process_relayer_mint_complete_set_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RelayerMintCompleteSetArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Relayer (signer)
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: Position PDA (writable)
    let position_info = next_account_info(account_info_iter)?;
    
    // Account 4: User Vault Account (writable)
    let user_vault_info = next_account_info(account_info_iter)?;
    
    // Account 5: PM User Account (writable)
    let pm_user_account_info = next_account_info(account_info_iter)?;
    
    // Account 6: Vault Config
    let vault_config_info = next_account_info(account_info_iter)?;
    
    // Account 7: Vault Program
    let vault_program_info = next_account_info(account_info_iter)?;
    
    // Account 8: System Program
    let system_program_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify Relayer authority
    verify_relayer(&config, relayer_info.key)?;
    
    if config.is_paused {
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    // Load and validate market
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    if !market.is_tradeable() {
        return Err(PredictionMarketError::MarketNotTradeable.into());
    }
    
    // Validate amount
    if args.amount == 0 {
        return Err(PredictionMarketError::InvalidAmount.into());
    }
    
    let current_time = get_current_timestamp()?;
    let market_id_bytes = market.market_id.to_le_bytes();
    
    // Derive Config PDA for CPI signing
    let (config_pda, config_bump) = Pubkey::find_program_address(
        &[PM_CONFIG_SEED],
        program_id,
    );
    
    if *config_info.key != config_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let config_seeds: &[&[u8]] = &[PM_CONFIG_SEED, &[config_bump]];
    
    // Step 1: CPI to Vault - PredictionMarketLock
    // Also passes relayer (payer) and system_program for auto-init of PMUserAccount
    msg!("CPI: Vault.PredictionMarketLock amount={}", args.amount);
    cpi_lock_for_prediction(
        vault_program_info,
        vault_config_info,
        user_vault_info,
        pm_user_account_info,
        config_info,  // PM Config as caller program marker
        relayer_info, // Payer for auto-init
        system_program_info, // System program for auto-init
        args.amount,
        config_seeds,
    )?;
    
    // Step 2: Create or update Position PDA
    let (position_pda, position_bump) = Pubkey::find_program_address(
        &[POSITION_SEED, &market_id_bytes, args.user_wallet.as_ref()],
        program_id,
    );
    
    if *position_info.key != position_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let is_new_position = position_info.data_is_empty();
    
    if is_new_position {
        // Create new Position account
        let rent = Rent::get()?;
        let space = Position::SIZE;
        let lamports = rent.minimum_balance(space);
        let position_seeds: &[&[u8]] = &[
            POSITION_SEED, 
            &market_id_bytes, 
            args.user_wallet.as_ref(), 
            &[position_bump]
        ];
        
        invoke_signed(
            &system_instruction::create_account(
                relayer_info.key,
                position_info.key,
                lamports,
                space as u64,
                program_id,
            ),
            &[relayer_info.clone(), position_info.clone(), system_program_info.clone()],
            &[position_seeds],
        )?;
    }
    
    // Get mutable access to position data
    // For newly created accounts, we initialize; for existing, we update
    let mut position_data = position_info.try_borrow_mut_data()?;
    
    let mut position = if is_new_position {
        // Initialize new position
        Position::new(market.market_id, args.user_wallet, position_bump, current_time)
    } else {
        // Deserialize existing position
        let pos = Position::deserialize(&mut &position_data[..])?;
        if pos.discriminator != POSITION_DISCRIMINATOR {
            return Err(PredictionMarketError::InvalidAccountData.into());
        }
        pos
    };
    
    // MintCompleteSet: add same amount to both YES and NO
    // avg_cost = 500_000 (0.5 USDC) because 1 USDC = 1 YES + 1 NO
    position.yes_amount = safe_add_u64(position.yes_amount, args.amount)?;
    position.no_amount = safe_add_u64(position.no_amount, args.amount)?;
    position.yes_avg_cost = 500_000;  // 0.5 USDC per token
    position.no_avg_cost = 500_000;   // 0.5 USDC per token
    position.total_cost_e6 = safe_add_u64(position.total_cost_e6, args.amount)?;  // Total USDC spent
    position.updated_at = current_time;
    
    // Serialize directly to the account data slice
    position.serialize(&mut position_data.as_mut())?;
    drop(position_data); // Release mutable borrow
    
    // Step 3: Update Market
    market.total_minted = safe_add_u64(market.total_minted, args.amount)?;
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    msg!("✅ RelayerMintCompleteSetV2 completed");
    msg!("User: {}", args.user_wallet);
    msg!("Amount: {}", args.amount);
    msg!("Position YES: {}, NO: {}", position.yes_amount, position.no_amount);
    msg!("Total Minted: {}", market.total_minted);
    
    Ok(())
}

/// V2: RelayerRedeemCompleteSet using Vault CPI (no SPL Token)
/// 
/// This function:
/// 1. Validates relayer, market, and position
/// 2. Verifies user has sufficient YES and NO virtual tokens
/// 3. Calls Vault.PredictionMarketUnlock to move funds from pm_locked to available_balance
/// 4. Updates Position PDA by reducing YES/NO amounts
/// 5. Updates Market.total_minted
fn process_relayer_redeem_complete_set_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RelayerRedeemCompleteSetArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Relayer (signer)
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: Position PDA (writable)
    let position_info = next_account_info(account_info_iter)?;
    
    // Account 4: User Vault Account (writable)
    let user_vault_info = next_account_info(account_info_iter)?;
    
    // Account 5: PM User Account (writable)
    let pm_user_account_info = next_account_info(account_info_iter)?;
    
    // Account 6: Vault Config
    let vault_config_info = next_account_info(account_info_iter)?;
    
    // Account 7: Vault Program
    let vault_program_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    verify_relayer(&config, relayer_info.key)?;
    
    if config.is_paused {
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    // Load and validate market
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    // Validate amount
    if args.amount == 0 {
        return Err(PredictionMarketError::InvalidAmount.into());
    }
    
    let current_time = get_current_timestamp()?;
    let market_id_bytes = market.market_id.to_le_bytes();
    
    // Verify Position PDA
    let (position_pda, _position_bump) = Pubkey::find_program_address(
        &[POSITION_SEED, &market_id_bytes, args.user_wallet.as_ref()],
        program_id,
    );
    
    if *position_info.key != position_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load and validate Position
    let mut position = deserialize_account::<Position>(&position_info.data.borrow())?;
    if position.discriminator != POSITION_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify user has sufficient tokens
    if position.yes_amount < args.amount || position.no_amount < args.amount {
        msg!("Insufficient position: YES={}, NO={}, requested={}", 
             position.yes_amount, position.no_amount, args.amount);
        return Err(PredictionMarketError::InsufficientPosition.into());
    }
    
    // Derive Config PDA for CPI signing
    let (config_pda, config_bump) = Pubkey::find_program_address(
        &[PM_CONFIG_SEED],
        program_id,
    );
    
    if *config_info.key != config_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let config_seeds: &[&[u8]] = &[PM_CONFIG_SEED, &[config_bump]];
    
    // Step 1: CPI to Vault - PredictionMarketUnlock
    msg!("CPI: Vault.PredictionMarketUnlock amount={}", args.amount);
    cpi_release_from_prediction(
        vault_program_info,
        vault_config_info,
        user_vault_info,
        pm_user_account_info,
        config_info,
        args.amount,
        config_seeds,
    )?;
    
    // Step 2: Update Position - reduce YES and NO amounts
    position.yes_amount = position.yes_amount.saturating_sub(args.amount);
    position.no_amount = position.no_amount.saturating_sub(args.amount);
    position.updated_at = current_time;
    
    position.serialize(&mut *position_info.data.borrow_mut())?;
    
    // Step 3: Update Market
    market.total_minted = market.total_minted.saturating_sub(args.amount);
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    msg!("✅ RelayerRedeemCompleteSetV2 completed");
    msg!("User: {}", args.user_wallet);
    msg!("Amount: {}", args.amount);
    msg!("Position YES: {}, NO: {}", position.yes_amount, position.no_amount);
    msg!("Total Minted: {}", market.total_minted);
    
    Ok(())
}

/// V2: MatchMint using Vault CPI (no SPL Token)
/// 
/// Matches a YES buy order with a NO buy order via minting.
/// Both buyers lock funds, and receive virtual tokens in their positions.
fn process_match_mint_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: MatchMintArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Relayer/Matcher (signer)
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: YES Buy Order (writable)
    let yes_order_info = next_account_info(account_info_iter)?;
    
    // Account 4: NO Buy Order (writable)
    let no_order_info = next_account_info(account_info_iter)?;
    
    // Account 5: YES Buyer Position (writable)
    let yes_position_info = next_account_info(account_info_iter)?;
    
    // Account 6: NO Buyer Position (writable)
    let no_position_info = next_account_info(account_info_iter)?;
    
    // Account 7: YES Buyer Vault Account (writable)
    let yes_vault_info = next_account_info(account_info_iter)?;
    
    // Account 8: YES Buyer PM User Account (writable)
    let yes_pm_user_info = next_account_info(account_info_iter)?;
    
    // Account 9: NO Buyer Vault Account (writable)
    let no_vault_info = next_account_info(account_info_iter)?;
    
    // Account 10: NO Buyer PM User Account (writable)
    let no_pm_user_info = next_account_info(account_info_iter)?;
    
    // Account 11: Vault Config
    let vault_config_info = next_account_info(account_info_iter)?;
    
    // Account 12: Vault Program
    let vault_program_info = next_account_info(account_info_iter)?;
    
    // Account 13: System Program (for auto-init PMUserAccount)
    let system_program_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    verify_relayer(&config, relayer_info.key)?;
    
    if config.is_paused {
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    // Load and validate market
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    if !market.is_tradeable() {
        return Err(PredictionMarketError::MarketNotTradeable.into());
    }
    
    // Validate price pair for minting: yes_price + no_price <= 1.0
    if args.yes_price + args.no_price > PRICE_PRECISION {
        msg!("Price sum {} + {} > 1.0, not valid for minting", args.yes_price, args.no_price);
        return Err(PredictionMarketError::InvalidPricePair.into());
    }
    
    let current_time = get_current_timestamp()?;
    
    // Load orders and validate
    let mut yes_order = deserialize_account::<Order>(&yes_order_info.data.borrow())?;
    let mut no_order = deserialize_account::<Order>(&no_order_info.data.borrow())?;
    
    if yes_order.discriminator != ORDER_DISCRIMINATOR || no_order.discriminator != ORDER_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify orders are Buy orders
    if yes_order.side != crate::state::OrderSide::Buy || no_order.side != crate::state::OrderSide::Buy {
        return Err(PredictionMarketError::InvalidOrderSide.into());
    }
    
    // Verify outcomes
    if yes_order.outcome != Outcome::Yes || no_order.outcome != Outcome::No {
        return Err(PredictionMarketError::InvalidOutcome.into());
    }
    
    // Verify orders are active
    if !yes_order.is_active() || !no_order.is_active() {
        return Err(PredictionMarketError::OrderNotActive.into());
    }
    
    // Calculate match amount
    let yes_remaining = yes_order.remaining_amount();
    let no_remaining = no_order.remaining_amount();
    let match_amount = args.amount.min(yes_remaining).min(no_remaining);
    
    if match_amount == 0 {
        return Err(PredictionMarketError::NoMatchableAmount.into());
    }
    
    // Calculate costs
    let yes_cost = (match_amount as u128 * args.yes_price as u128 / PRICE_PRECISION as u128) as u64;
    let no_cost = (match_amount as u128 * args.no_price as u128 / PRICE_PRECISION as u128) as u64;
    
    // Derive Config PDA for CPI signing
    let (config_pda, config_bump) = Pubkey::find_program_address(
        &[PM_CONFIG_SEED],
        program_id,
    );
    
    if *config_info.key != config_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let config_seeds: &[&[u8]] = &[PM_CONFIG_SEED, &[config_bump]];
    
    // Step 1: Lock funds for YES buyer
    msg!("CPI: Lock {} for YES buyer", yes_cost);
    cpi_lock_for_prediction(
        vault_program_info,
        vault_config_info,
        yes_vault_info,
        yes_pm_user_info,
        config_info,
        relayer_info,       // Payer for auto-init
        system_program_info, // System program for auto-init
        yes_cost,
        config_seeds,
    )?;
    
    // Step 2: Lock funds for NO buyer
    msg!("CPI: Lock {} for NO buyer", no_cost);
    cpi_lock_for_prediction(
        vault_program_info,
        vault_config_info,
        no_vault_info,
        no_pm_user_info,
        config_info,
        relayer_info,       // Payer for auto-init
        system_program_info, // System program for auto-init
        no_cost,
        config_seeds,
    )?;
    
    // Step 3: Create or update YES buyer position (Auto-init if needed)
    let market_id_bytes = args.market_id.to_le_bytes();
    let yes_buyer = yes_order.owner;
    let (yes_position_pda, yes_position_bump) = Pubkey::find_program_address(
        &[POSITION_SEED, &market_id_bytes, yes_buyer.as_ref()],
        program_id,
    );
    
    if *yes_position_info.key != yes_position_pda {
        msg!("Error: Invalid YES Position PDA");
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let yes_is_new = yes_position_info.data_is_empty();
    
    if yes_is_new {
        // Create new YES Position account
        let rent = Rent::get()?;
        let space = Position::SIZE;
        let lamports = rent.minimum_balance(space);
        let position_seeds: &[&[u8]] = &[
            POSITION_SEED, 
            &market_id_bytes, 
            yes_buyer.as_ref(), 
            &[yes_position_bump]
        ];
        
        msg!("Creating YES buyer Position account");
        invoke_signed(
            &system_instruction::create_account(
                relayer_info.key,
                yes_position_info.key,
                lamports,
                space as u64,
                program_id,
            ),
            &[relayer_info.clone(), yes_position_info.clone(), system_program_info.clone()],
            &[position_seeds],
        )?;
    }
    
    // Update YES position
    {
        let mut yes_position_data = yes_position_info.try_borrow_mut_data()?;
        let mut yes_position = if yes_is_new {
            Position::new(market.market_id, yes_buyer, yes_position_bump, current_time)
        } else {
            let pos = Position::deserialize(&mut &yes_position_data[..])?;
            if pos.discriminator != POSITION_DISCRIMINATOR {
                return Err(PredictionMarketError::InvalidAccountData.into());
            }
            pos
        };
        yes_position.add_tokens(Outcome::Yes, match_amount, args.yes_price, current_time);
        yes_position.serialize(&mut yes_position_data.as_mut())?;
    }
    
    // Step 4: Create or update NO buyer position (Auto-init if needed)
    let no_buyer = no_order.owner;
    let (no_position_pda, no_position_bump) = Pubkey::find_program_address(
        &[POSITION_SEED, &market_id_bytes, no_buyer.as_ref()],
        program_id,
    );
    
    if *no_position_info.key != no_position_pda {
        msg!("Error: Invalid NO Position PDA");
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let no_is_new = no_position_info.data_is_empty();
    
    if no_is_new {
        // Create new NO Position account
        let rent = Rent::get()?;
        let space = Position::SIZE;
        let lamports = rent.minimum_balance(space);
        let position_seeds: &[&[u8]] = &[
            POSITION_SEED, 
            &market_id_bytes, 
            no_buyer.as_ref(), 
            &[no_position_bump]
        ];
        
        msg!("Creating NO buyer Position account");
        invoke_signed(
            &system_instruction::create_account(
                relayer_info.key,
                no_position_info.key,
                lamports,
                space as u64,
                program_id,
            ),
            &[relayer_info.clone(), no_position_info.clone(), system_program_info.clone()],
            &[position_seeds],
        )?;
    }
    
    // Update NO position
    {
        let mut no_position_data = no_position_info.try_borrow_mut_data()?;
        let mut no_position = if no_is_new {
            Position::new(market.market_id, no_buyer, no_position_bump, current_time)
        } else {
            let pos = Position::deserialize(&mut &no_position_data[..])?;
            if pos.discriminator != POSITION_DISCRIMINATOR {
                return Err(PredictionMarketError::InvalidAccountData.into());
            }
            pos
        };
        no_position.add_tokens(Outcome::No, match_amount, args.no_price, current_time);
        no_position.serialize(&mut no_position_data.as_mut())?;
    }
    
    // Step 5: Update orders
    yes_order.filled_amount = safe_add_u64(yes_order.filled_amount, match_amount)?;
    if yes_order.filled_amount >= yes_order.amount {
        yes_order.status = OrderStatus::Filled;
    } else {
        yes_order.status = OrderStatus::PartialFilled;
    }
    yes_order.updated_at = current_time;
    yes_order.serialize(&mut *yes_order_info.data.borrow_mut())?;
    
    no_order.filled_amount = safe_add_u64(no_order.filled_amount, match_amount)?;
    if no_order.filled_amount >= no_order.amount {
        no_order.status = OrderStatus::Filled;
    } else {
        no_order.status = OrderStatus::PartialFilled;
    }
    no_order.updated_at = current_time;
    no_order.serialize(&mut *no_order_info.data.borrow_mut())?;
    
    // Step 6: Update market
    market.total_minted = safe_add_u64(market.total_minted, match_amount)?;
    market.total_volume_e6 = market.total_volume_e6.saturating_add((yes_cost + no_cost) as i64);
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    // Step 7: Optional Fee Collection (V2.1 architecture)
    // If fee accounts are provided, collect trading fees
    // Account 14: Vault Token Account (optional)
    // Account 15: PM Fee Vault (optional)
    // Account 16: PM Fee Config (optional)
    // Account 17: Token Program (optional)
    let vault_token_account = next_account_info(account_info_iter).ok();
    let pm_fee_vault = next_account_info(account_info_iter).ok();
    let pm_fee_config = next_account_info(account_info_iter).ok();
    let token_program = next_account_info(account_info_iter).ok();
    
    if let (Some(vta), Some(pfv), Some(pfc), Some(tp)) = (vault_token_account, pm_fee_vault, pm_fee_config, token_program) {
        msg!("Fee accounts detected, collecting trading fees...");
        
        let (config_pda, config_bump) = Pubkey::find_program_address(
            &[PM_CONFIG_SEED],
            program_id,
        );
        let config_seeds: &[&[u8]] = &[PM_CONFIG_SEED, &[config_bump]];
        
        // Collect Taker fee from YES buyer
        let _ = cpi_trade_with_fee(
            vault_program_info,
            vault_config_info,
            config_info,
            vta,
            pfv,
            pfc,
            tp,
            yes_cost,
            true, // is_taker
            config_seeds,
        );
        
        // Collect Taker fee from NO buyer  
        let _ = cpi_trade_with_fee(
            vault_program_info,
            vault_config_info,
            config_info,
            vta,
            pfv,
            pfc,
            tp,
            no_cost,
            true, // is_taker
            config_seeds,
        );
        
        msg!("✅ Trading fees collected for MatchMintV2");
    } else {
        msg!("Fee accounts not provided, skipping fee collection");
    }
    
    msg!("✅ MatchMintV2 completed");
    msg!("Amount: {}", match_amount);
    msg!("YES cost: {}, NO cost: {}", yes_cost, no_cost);
    msg!("Total Minted: {}", market.total_minted);
    
    Ok(())
}

/// V2: MatchBurn using Vault CPI (no SPL Token)
/// 
/// Matches a YES sell order with a NO sell order via burning.
/// Both sellers receive funds from their locked amounts.
fn process_match_burn_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: MatchBurnArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Relayer/Matcher (signer)
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: YES Sell Order (writable)
    let yes_order_info = next_account_info(account_info_iter)?;
    
    // Account 4: NO Sell Order (writable)
    let no_order_info = next_account_info(account_info_iter)?;
    
    // Account 5: YES Seller Position (writable)
    let yes_position_info = next_account_info(account_info_iter)?;
    
    // Account 6: NO Seller Position (writable)
    let no_position_info = next_account_info(account_info_iter)?;
    
    // Account 7: YES Seller Vault Account (writable)
    let yes_vault_info = next_account_info(account_info_iter)?;
    
    // Account 8: YES Seller PM User Account (writable)
    let yes_pm_user_info = next_account_info(account_info_iter)?;
    
    // Account 9: NO Seller Vault Account (writable)
    let no_vault_info = next_account_info(account_info_iter)?;
    
    // Account 10: NO Seller PM User Account (writable)
    let no_pm_user_info = next_account_info(account_info_iter)?;
    
    // Account 11: Vault Config
    let vault_config_info = next_account_info(account_info_iter)?;
    
    // Account 12: Vault Program
    let vault_program_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    verify_relayer(&config, relayer_info.key)?;
    
    if config.is_paused {
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    // Load and validate market
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    // Validate price pair for burning: yes_price + no_price >= 1.0
    if args.yes_price + args.no_price < PRICE_PRECISION {
        msg!("Price sum {} + {} < 1.0, not valid for burning", args.yes_price, args.no_price);
        return Err(PredictionMarketError::InvalidPricePair.into());
    }
    
    let current_time = get_current_timestamp()?;
    
    // Load orders
    let mut yes_order = deserialize_account::<Order>(&yes_order_info.data.borrow())?;
    let mut no_order = deserialize_account::<Order>(&no_order_info.data.borrow())?;
    
    // Verify orders are Sell orders
    if yes_order.side != crate::state::OrderSide::Sell || no_order.side != crate::state::OrderSide::Sell {
        return Err(PredictionMarketError::InvalidOrderSide.into());
    }
    
    // Verify outcomes
    if yes_order.outcome != Outcome::Yes || no_order.outcome != Outcome::No {
        return Err(PredictionMarketError::InvalidOutcome.into());
    }
    
    // Verify orders are active
    if !yes_order.is_active() || !no_order.is_active() {
        return Err(PredictionMarketError::OrderNotActive.into());
    }
    
    // Calculate match amount
    let yes_remaining = yes_order.remaining_amount();
    let no_remaining = no_order.remaining_amount();
    let match_amount = args.amount.min(yes_remaining).min(no_remaining);
    
    if match_amount == 0 {
        return Err(PredictionMarketError::NoMatchableAmount.into());
    }
    
    // Calculate proceeds
    let yes_proceeds = (match_amount as u128 * args.yes_price as u128 / PRICE_PRECISION as u128) as u64;
    let no_proceeds = (match_amount as u128 * args.no_price as u128 / PRICE_PRECISION as u128) as u64;
    
    // Load positions
    let mut yes_position = deserialize_account::<Position>(&yes_position_info.data.borrow())?;
    let mut no_position = deserialize_account::<Position>(&no_position_info.data.borrow())?;
    
    // Verify sellers have sufficient LOCKED shares (locked when Sell order was placed)
    if yes_position.yes_locked < match_amount {
        msg!("Error: YES seller has insufficient locked shares: {} < {}", 
             yes_position.yes_locked, match_amount);
        return Err(PredictionMarketError::InsufficientPosition.into());
    }
    if no_position.no_locked < match_amount {
        msg!("Error: NO seller has insufficient locked shares: {} < {}", 
             no_position.no_locked, match_amount);
        return Err(PredictionMarketError::InsufficientPosition.into());
    }
    
    // Derive Config PDA for CPI signing
    let (config_pda, config_bump) = Pubkey::find_program_address(
        &[PM_CONFIG_SEED],
        program_id,
    );
    
    if *config_info.key != config_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let config_seeds: &[&[u8]] = &[PM_CONFIG_SEED, &[config_bump]];
    
    // Step 1: Release funds for YES seller
    msg!("CPI: Release {} for YES seller", yes_proceeds);
    cpi_release_from_prediction(
        vault_program_info,
        vault_config_info,
        yes_vault_info,
        yes_pm_user_info,
        config_info,
        yes_proceeds,
        config_seeds,
    )?;
    
    // Step 2: Release funds for NO seller
    msg!("CPI: Release {} for NO seller", no_proceeds);
    cpi_release_from_prediction(
        vault_program_info,
        vault_config_info,
        no_vault_info,
        no_pm_user_info,
        config_info,
        no_proceeds,
        config_seeds,
    )?;
    
    // Step 3: Update positions - consume locked shares (unlock + remove)
    yes_position.consume_locked_shares(Outcome::Yes, match_amount, args.yes_price, current_time)
        .map_err(|_| {
            msg!("Error: Failed to consume YES locked shares");
            PredictionMarketError::InsufficientPosition
        })?;
    yes_position.serialize(&mut *yes_position_info.data.borrow_mut())?;
    
    no_position.consume_locked_shares(Outcome::No, match_amount, args.no_price, current_time)
        .map_err(|_| {
            msg!("Error: Failed to consume NO locked shares");
            PredictionMarketError::InsufficientPosition
        })?;
    no_position.serialize(&mut *no_position_info.data.borrow_mut())?;
    
    msg!("📊 Burned {} complete sets (YES + NO)", match_amount);
    
    // Step 4: Update orders
    yes_order.filled_amount = safe_add_u64(yes_order.filled_amount, match_amount)?;
    if yes_order.filled_amount >= yes_order.amount {
        yes_order.status = OrderStatus::Filled;
    } else {
        yes_order.status = OrderStatus::PartialFilled;
    }
    yes_order.updated_at = current_time;
    yes_order.serialize(&mut *yes_order_info.data.borrow_mut())?;
    
    no_order.filled_amount = safe_add_u64(no_order.filled_amount, match_amount)?;
    if no_order.filled_amount >= no_order.amount {
        no_order.status = OrderStatus::Filled;
    } else {
        no_order.status = OrderStatus::PartialFilled;
    }
    no_order.updated_at = current_time;
    no_order.serialize(&mut *no_order_info.data.borrow_mut())?;
    
    // Step 5: Update market
    market.total_minted = market.total_minted.saturating_sub(match_amount);
    market.total_volume_e6 = market.total_volume_e6.saturating_add((yes_proceeds + no_proceeds) as i64);
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    // Step 6: Optional Fee Collection (V2.1 architecture)
    // If fee accounts are provided, collect trading fees
    let vault_token_account = next_account_info(account_info_iter).ok();
    let pm_fee_vault = next_account_info(account_info_iter).ok();
    let pm_fee_config = next_account_info(account_info_iter).ok();
    let token_program = next_account_info(account_info_iter).ok();
    
    if let (Some(vta), Some(pfv), Some(pfc), Some(tp)) = (vault_token_account, pm_fee_vault, pm_fee_config, token_program) {
        msg!("Fee accounts detected, collecting trading fees...");
        
        let (config_pda, config_bump) = Pubkey::find_program_address(
            &[PM_CONFIG_SEED],
            program_id,
        );
        let config_seeds: &[&[u8]] = &[PM_CONFIG_SEED, &[config_bump]];
        
        // Collect Maker fee from YES seller
        let _ = cpi_trade_with_fee(
            vault_program_info,
            vault_config_info,
            config_info,
            vta,
            pfv,
            pfc,
            tp,
            yes_proceeds,
            false, // is_maker
            config_seeds,
        );
        
        // Collect Maker fee from NO seller
        let _ = cpi_trade_with_fee(
            vault_program_info,
            vault_config_info,
            config_info,
            vta,
            pfv,
            pfc,
            tp,
            no_proceeds,
            false, // is_maker
            config_seeds,
        );
        
        msg!("✅ Trading fees collected for MatchBurnV2");
    } else {
        msg!("Fee accounts not provided, skipping fee collection");
    }
    
    msg!("✅ MatchBurnV2 completed");
    msg!("Amount: {}", match_amount);
    msg!("YES proceeds: {}, NO proceeds: {}", yes_proceeds, no_proceeds);
    msg!("Total Minted: {}", market.total_minted);
    
    Ok(())
}

/// V2: RelayerClaimWinnings using Vault CPI (no SPL Token)
/// 
/// This function:
/// 1. Validates market is resolved
/// 2. Calculates settlement based on winning outcome and position
/// 3. Calls Vault.PredictionMarketSettle to settle funds
/// 4. Marks position as settled
fn process_relayer_claim_winnings_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RelayerClaimWinningsArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Relayer (signer)
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: Position PDA (writable)
    let position_info = next_account_info(account_info_iter)?;
    
    // Account 4: PM User Account (writable)
    let pm_user_account_info = next_account_info(account_info_iter)?;
    
    // Account 5: Vault Config
    let vault_config_info = next_account_info(account_info_iter)?;
    
    // Account 6: Vault Program
    let vault_program_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    verify_relayer(&config, relayer_info.key)?;
    
    // Load and validate market
    let market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    if market.status != MarketStatus::Resolved {
        return Err(PredictionMarketError::MarketNotResolved.into());
    }
    
    let final_result = market.final_result.ok_or(PredictionMarketError::MarketNotResolved)?;
    
    let market_id_bytes = market.market_id.to_le_bytes();
    let current_time = get_current_timestamp()?;
    
    // Verify Position PDA
    let (position_pda, _position_bump) = Pubkey::find_program_address(
        &[POSITION_SEED, &market_id_bytes, args.user_wallet.as_ref()],
        program_id,
    );
    
    if *position_info.key != position_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load and validate Position
    let mut position = deserialize_account::<Position>(&position_info.data.borrow())?;
    if position.discriminator != POSITION_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if position.settled {
        return Err(PredictionMarketError::AlreadySettled.into());
    }
    
    // Calculate settlement amount based on result
    let (winning_amount, locked_amount) = match final_result {
        MarketResult::Yes => (position.yes_amount, position.total_cost_e6),
        MarketResult::No => (position.no_amount, position.total_cost_e6),
        MarketResult::Invalid => {
            // Refund original cost
            (0, position.total_cost_e6)
        }
    };
    
    let settlement_amount = if final_result == MarketResult::Invalid {
        // Full refund on invalid market
        locked_amount
    } else {
        // Winning tokens pay out 1:1
        winning_amount
    };
    
    if settlement_amount == 0 && winning_amount == 0 {
        msg!("No winnings to claim for user {}", args.user_wallet);
        return Err(PredictionMarketError::InvalidAmount.into());
    }
    
    // Derive Config PDA for CPI signing
    let (config_pda, config_bump) = Pubkey::find_program_address(
        &[PM_CONFIG_SEED],
        program_id,
    );
    
    if *config_info.key != config_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let config_seeds: &[&[u8]] = &[PM_CONFIG_SEED, &[config_bump]];
    
    // Check for optional fee accounts
    // Account 8: Vault Token Account (optional)
    // Account 9: PM Fee Vault (optional)
    // Account 10: PM Fee Config (optional)
    // Account 11: Token Program (optional)
    let vault_token_account = next_account_info(account_info_iter).ok();
    let pm_fee_vault = next_account_info(account_info_iter).ok();
    let pm_fee_config = next_account_info(account_info_iter).ok();
    let token_program = next_account_info(account_info_iter).ok();
    
    let use_fee_settlement = vault_token_account.is_some() 
        && pm_fee_vault.is_some() 
        && pm_fee_config.is_some() 
        && token_program.is_some();
    
    if use_fee_settlement {
        // Use settle with fee CPI
        let vta = vault_token_account.unwrap();
        let pfv = pm_fee_vault.unwrap();
        let pfc = pm_fee_config.unwrap();
        let tp = token_program.unwrap();
        
        msg!("CPI: Vault.PredictionMarketSettleWithFee locked={}, settlement={}", 
             locked_amount, settlement_amount);
        cpi_settle_with_fee(
            vault_program_info,
            vault_config_info,
            pm_user_account_info,
            config_info,
            vta,
            pfv,
            pfc,
            tp,
            locked_amount,
            settlement_amount,
            config_seeds,
        )?;
        msg!("✅ Settlement with fee collection completed");
    } else {
        // Use regular settle CPI (no fee)
        msg!("CPI: Vault.PredictionMarketSettle locked={}, settlement={}", 
             locked_amount, settlement_amount);
        cpi_prediction_settle(
            vault_program_info,
            vault_config_info,
            pm_user_account_info,
            config_info,
            locked_amount,
            settlement_amount,
            config_seeds,
        )?;
    }
    
    // Update Position
    let pnl = (settlement_amount as i64) - (locked_amount as i64);
    position.realized_pnl = position.realized_pnl.saturating_add(pnl);
    position.settlement_amount = settlement_amount;
    position.settled = true;
    position.yes_amount = 0;
    position.no_amount = 0;
    position.updated_at = current_time;
    
    position.serialize(&mut *position_info.data.borrow_mut())?;
    
    msg!("✅ RelayerClaimWinningsV2 completed");
    msg!("User: {}", args.user_wallet);
    msg!("Result: {:?}", final_result);
    msg!("Settlement: {}, PnL: {}", settlement_amount, pnl);
    
    Ok(())
}

/// V2: ExecuteTrade using Vault CPI (no SPL Token)
/// 
/// Direct trade between buyer and seller:
/// - Buyer has USDC locked in pm_locked (from RelayerPlaceOrder)
/// - Seller has virtual shares in Position PDA
/// - Trade transfers USDC (buyer → seller) and shares (seller → buyer)
/// 
/// Flow:
/// 1. Validate orders (same outcome, price compatible, sufficient amounts)
/// 2. Validate seller has sufficient Position shares
/// 3. CPI: Settle buyer (locked=cost, settlement=0) - deduct from buyer's pm_locked
/// 4. CPI: Settle seller (locked=0, settlement=cost) - add to seller's pending_settlement  
/// 5. Update Positions: transfer shares from seller to buyer
/// 6. Update Orders: mark filled/partial_filled
fn process_execute_trade_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: ExecuteTradeArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Relayer/Keeper (signer)
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: Buy Order (writable)
    let buy_order_info = next_account_info(account_info_iter)?;
    
    // Account 4: Sell Order (writable)
    let sell_order_info = next_account_info(account_info_iter)?;
    
    // Account 5: Buyer Position PDA (writable)
    let buyer_position_info = next_account_info(account_info_iter)?;
    
    // Account 6: Seller Position PDA (writable)
    let seller_position_info = next_account_info(account_info_iter)?;
    
    // Account 7: Buyer UserAccount (Vault, writable) - not used in Settle, but for validation
    let _buyer_vault_info = next_account_info(account_info_iter)?;
    
    // Account 8: Buyer PMUserAccount (Vault, writable)
    let buyer_pm_user_info = next_account_info(account_info_iter)?;
    
    // Account 9: Seller UserAccount (Vault, writable) - not used in Settle
    let _seller_vault_info = next_account_info(account_info_iter)?;
    
    // Account 10: Seller PMUserAccount (Vault, writable)
    let seller_pm_user_info = next_account_info(account_info_iter)?;
    
    // Account 11: VaultConfig
    let vault_config_info = next_account_info(account_info_iter)?;
    
    // Account 12: Vault Program
    let vault_program_info = next_account_info(account_info_iter)?;
    
    // Account 13: System Program
    let system_program_info = next_account_info(account_info_iter)?;
    
    // Account 14: Buyer Wallet (用于 CPI 自动创建 PMUserAccount)
    let buyer_wallet_info = next_account_info(account_info_iter)?;
    
    // Account 15: Seller Wallet (用于 CPI 自动创建 PMUserAccount)
    let seller_wallet_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    verify_relayer(&config, relayer_info.key)?;
    
    if config.is_paused {
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    // Verify Market PDA
    let market_id_bytes = args.market_id.to_le_bytes();
    let (market_pda, _) = Pubkey::find_program_address(
        &[MARKET_SEED, &market_id_bytes],
        program_id,
    );
    if *market_info.key != market_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load market
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if !market.is_tradeable() {
        return Err(PredictionMarketError::MarketNotTradeable.into());
    }
    
    // Verify Order PDAs
    let taker_order_id_bytes = args.taker_order_id.to_le_bytes();
    let (buy_order_pda, _) = Pubkey::find_program_address(
        &[ORDER_SEED, &market_id_bytes, &taker_order_id_bytes],
        program_id,
    );
    if *buy_order_info.key != buy_order_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let maker_order_id_bytes = args.maker_order_id.to_le_bytes();
    let (sell_order_pda, _) = Pubkey::find_program_address(
        &[ORDER_SEED, &market_id_bytes, &maker_order_id_bytes],
        program_id,
    );
    if *sell_order_info.key != sell_order_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load orders
    let mut buy_order = deserialize_account::<Order>(&buy_order_info.data.borrow())?;
    let mut sell_order = deserialize_account::<Order>(&sell_order_info.data.borrow())?;
    
    // Validate orders
    if buy_order.discriminator != ORDER_DISCRIMINATOR || sell_order.discriminator != ORDER_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify order sides
    if buy_order.side != crate::state::OrderSide::Buy {
        msg!("Error: Order {} is not a buy order", args.taker_order_id);
        return Err(PredictionMarketError::InvalidOrderSide.into());
    }
    if sell_order.side != crate::state::OrderSide::Sell {
        msg!("Error: Order {} is not a sell order", args.maker_order_id);
        return Err(PredictionMarketError::InvalidOrderSide.into());
    }
    
    // Verify same outcome
    if buy_order.outcome != sell_order.outcome {
        msg!("Error: Orders must be for the same outcome");
        return Err(PredictionMarketError::OutcomeMismatch.into());
    }
    
    let outcome = buy_order.outcome;
    
    // Verify orders are active
    if !buy_order.is_active() || !sell_order.is_active() {
        msg!("Error: One or both orders are not active");
        return Err(PredictionMarketError::OrderNotActive.into());
    }
    
    // Verify price compatibility (buy price >= sell price)
    if buy_order.price < sell_order.price {
        msg!("Error: Buy price {} must be >= sell price {}", buy_order.price, sell_order.price);
        return Err(PredictionMarketError::PriceMismatch.into());
    }
    
    // Calculate matchable amount
    let buy_remaining = buy_order.remaining_amount();
    let sell_remaining = sell_order.remaining_amount();
    let match_amount = args.amount.min(buy_remaining).min(sell_remaining);
    
    if match_amount == 0 {
        return Err(PredictionMarketError::NoMatchableAmount.into());
    }
    
    let current_time = get_current_timestamp()?;
    
    // Execution price (use provided price, should be <= buy_price and >= sell_price)
    let exec_price = args.price;
    if exec_price < sell_order.price || exec_price > buy_order.price {
        msg!("Error: Execution price {} out of bounds [{}, {}]", 
             exec_price, sell_order.price, buy_order.price);
        return Err(PredictionMarketError::InvalidExecutionPrice.into());
    }
    
    // Calculate trade cost: cost = amount * price / PRICE_PRECISION
    let trade_cost = (match_amount as u128)
        .checked_mul(exec_price as u128)
        .ok_or(PredictionMarketError::ArithmeticOverflow)?
        .checked_div(PRICE_PRECISION as u128)
        .ok_or(PredictionMarketError::ArithmeticOverflow)? as u64;
    
    msg!("V2 Direct Trade: amount={}, price={}, cost={}", match_amount, exec_price, trade_cost);
    
    // Verify Position PDAs
    let (buyer_position_pda, _) = Pubkey::find_program_address(
        &[POSITION_SEED, &market_id_bytes, buy_order.owner.as_ref()],
        program_id,
    );
    if *buyer_position_info.key != buyer_position_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let (seller_position_pda, _) = Pubkey::find_program_address(
        &[POSITION_SEED, &market_id_bytes, sell_order.owner.as_ref()],
        program_id,
    );
    if *seller_position_info.key != seller_position_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load seller position to verify sufficient LOCKED shares
    // In V2, shares are locked when placing a Sell order via RelayerPlaceOrderV2
    let mut seller_position = deserialize_account::<Position>(&seller_position_info.data.borrow())?;
    if seller_position.discriminator != POSITION_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Check seller has sufficient LOCKED shares for this trade
    // The shares should have been locked when the Sell order was placed
    let seller_locked = seller_position.locked(outcome);
    
    if seller_locked < match_amount {
        msg!("Error: Seller has insufficient locked shares: {} < {} (total: {}, locked: {})", 
             seller_locked, match_amount,
             match outcome {
                 Outcome::Yes => seller_position.yes_amount,
                 Outcome::No => seller_position.no_amount,
             },
             seller_locked);
        return Err(PredictionMarketError::InsufficientPosition.into());
    }
    
    // Derive Config PDA for CPI signing
    let (config_pda, config_bump) = Pubkey::find_program_address(
        &[PM_CONFIG_SEED],
        program_id,
    );
    
    if *config_info.key != config_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let config_seeds: &[&[u8]] = &[PM_CONFIG_SEED, &[config_bump]];
    
    // Step 1: CPI - Settle buyer (deduct from pm_locked)
    // locked=trade_cost, settlement=0
    // 使用支持自动创建 PMUserAccount 的版本
    msg!("CPI: Settle buyer - deduct {} from pm_locked", trade_cost);
    cpi_prediction_settle_with_auto_init(
        vault_program_info,
        vault_config_info,
        buyer_pm_user_info,
        config_info,
        relayer_info,           // payer for auto-init
        system_program_info,    // system program for create_account
        buyer_wallet_info,      // buyer wallet for PDA derivation
        trade_cost,             // locked_amount to deduct
        0,                      // settlement_amount (none for buyer in trade)
        config_seeds,
    )?;
    
    // Step 2: CPI - Settle seller (add to pending_settlement)
    // locked=0, settlement=trade_cost
    // 使用支持自动创建 PMUserAccount 的版本
    msg!("CPI: Settle seller - add {} to pending_settlement", trade_cost);
    cpi_prediction_settle_with_auto_init(
        vault_program_info,
        vault_config_info,
        seller_pm_user_info,
        config_info,
        relayer_info,           // payer for auto-init
        system_program_info,    // system program for create_account
        seller_wallet_info,     // seller wallet for PDA derivation
        0,                      // locked_amount (seller didn't lock for sell order in V2)
        trade_cost,             // settlement_amount
        config_seeds,
    )?;
    
    // Step 3: Update Positions - transfer shares (seller → buyer)
    // Load or create buyer position (auto-init if empty)
    let (_, buyer_position_bump) = Pubkey::find_program_address(
        &[POSITION_SEED, &market_id_bytes, buy_order.owner.as_ref()],
        program_id,
    );
    
    let mut buyer_position = if buyer_position_info.data_is_empty() {
        // Auto-create buyer Position PDA (like MintCompleteSet does)
        msg!("Creating buyer Position PDA (auto-init for DirectTrade)");
        
        let rent = Rent::get()?;
        let space = Position::SIZE;
        let lamports = rent.minimum_balance(space);
        let position_seeds: &[&[u8]] = &[
            POSITION_SEED,
            &market_id_bytes,
            buy_order.owner.as_ref(),
            &[buyer_position_bump]
        ];
        
        invoke_signed(
            &system_instruction::create_account(
                relayer_info.key,
                buyer_position_info.key,
                lamports,
                space as u64,
                program_id,
            ),
            &[relayer_info.clone(), buyer_position_info.clone(), system_program_info.clone()],
            &[position_seeds],
        )?;
        
        // Initialize new position
        let position = Position::new(market.market_id, buy_order.owner, buyer_position_bump, current_time);
        position.serialize(&mut *buyer_position_info.data.borrow_mut())?;
        
        msg!("✅ Buyer Position PDA created: {}", buyer_position_info.key);
        position
    } else {
        deserialize_account::<Position>(&buyer_position_info.data.borrow())?
    };
    
    // Consume locked shares from seller (this unlocks and removes in one step)
    // Note: For Direct Trade, seller doesn't receive USDC here (handled by CPI above)
    // We use exec_price for PnL calculation
    seller_position.consume_locked_shares(outcome, match_amount, exec_price, current_time)
        .map_err(|_| {
            msg!("Error: Failed to consume locked shares from seller");
            PredictionMarketError::InsufficientPosition
        })?;
    
    // Add shares to buyer
    buyer_position.add_tokens(outcome, match_amount, exec_price, current_time);
    
    // Migrate seller Position if needed (old 146 bytes → new 154 bytes)
    if seller_position_info.data_len() < Position::SIZE {
        msg!("📦 Migrating seller Position: {} bytes → {} bytes", 
             seller_position_info.data_len(), Position::SIZE);
        seller_position_info.realloc(Position::SIZE, false)?;
        
        // Transfer lamports for rent-exemption if needed
        let rent = Rent::get()?;
        let required_lamports = rent.minimum_balance(Position::SIZE);
        let current_lamports = seller_position_info.lamports();
        if current_lamports < required_lamports {
            let diff = required_lamports - current_lamports;
            // Relayer pays for the realloc
            **relayer_info.try_borrow_mut_lamports()? -= diff;
            **seller_position_info.try_borrow_mut_lamports()? += diff;
            msg!("💰 Transferred {} lamports for rent", diff);
        }
    }
    
    // Migrate buyer Position if needed (shouldn't happen since we just created it, but just in case)
    if buyer_position_info.data_len() < Position::SIZE {
        msg!("📦 Migrating buyer Position: {} bytes → {} bytes", 
             buyer_position_info.data_len(), Position::SIZE);
        buyer_position_info.realloc(Position::SIZE, false)?;
        
        let rent = Rent::get()?;
        let required_lamports = rent.minimum_balance(Position::SIZE);
        let current_lamports = buyer_position_info.lamports();
        if current_lamports < required_lamports {
            let diff = required_lamports - current_lamports;
            **relayer_info.try_borrow_mut_lamports()? -= diff;
            **buyer_position_info.try_borrow_mut_lamports()? += diff;
            msg!("💰 Transferred {} lamports for rent", diff);
        }
    }
    
    seller_position.serialize(&mut *seller_position_info.data.borrow_mut())?;
    buyer_position.serialize(&mut *buyer_position_info.data.borrow_mut())?;
    
    msg!("📊 Shares transferred: {} {:?} from seller to buyer", match_amount, outcome);
    
    // Step 4: Update Orders
    buy_order.filled_amount += match_amount;
    if buy_order.filled_amount >= buy_order.amount {
        buy_order.status = OrderStatus::Filled;
    } else {
        buy_order.status = OrderStatus::PartialFilled;
    }
    buy_order.updated_at = current_time;
    buy_order.serialize(&mut *buy_order_info.data.borrow_mut())?;
    
    sell_order.filled_amount += match_amount;
    if sell_order.filled_amount >= sell_order.amount {
        sell_order.status = OrderStatus::Filled;
    } else {
        sell_order.status = OrderStatus::PartialFilled;
    }
    sell_order.updated_at = current_time;
    sell_order.serialize(&mut *sell_order_info.data.borrow_mut())?;
    
    // Step 5: Update Market stats
    market.total_volume_e6 = market.total_volume_e6.saturating_add(trade_cost as i64);
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    // Step 6: Optional Fee Collection (V2.1 architecture)
    let vault_token_account = next_account_info(account_info_iter).ok();
    let pm_fee_vault = next_account_info(account_info_iter).ok();
    let pm_fee_config = next_account_info(account_info_iter).ok();
    let token_program = next_account_info(account_info_iter).ok();
    
    if let (Some(vta), Some(pfv), Some(pfc), Some(tp)) = (vault_token_account, pm_fee_vault, pm_fee_config, token_program) {
        msg!("Fee accounts detected, collecting trading fees...");
        
        let (config_pda, config_bump) = Pubkey::find_program_address(
            &[PM_CONFIG_SEED],
            program_id,
        );
        let config_seeds: &[&[u8]] = &[PM_CONFIG_SEED, &[config_bump]];
        
        // Collect Taker fee from buyer
        let _ = cpi_trade_with_fee(
            vault_program_info,
            vault_config_info,
            config_info,
            vta,
            pfv,
            pfc,
            tp,
            trade_cost,
            true, // Taker (buyer)
            config_seeds,
        );
        
        // Collect Maker fee from seller
        let _ = cpi_trade_with_fee(
            vault_program_info,
            vault_config_info,
            config_info,
            vta,
            pfv,
            pfc,
            tp,
            trade_cost,
            false, // Maker (seller)
            config_seeds,
        );
        
        msg!("✅ Trading fees collected for ExecuteTradeV2");
    }
    
    // Emit success log
    msg!("✅ ExecuteTradeV2 completed");
    msg!("Market: {}, Outcome: {:?}", args.market_id, outcome);
    msg!("Buy Order: {}, Sell Order: {}", args.taker_order_id, args.maker_order_id);
    msg!("Amount: {}, Price: {}, Cost: {}", match_amount, exec_price, trade_cost);
    msg!("Buyer: {}", buy_order.owner);
    msg!("Seller: {}", sell_order.owner);
    
    Ok(())
}

// ============================================================================
// Multi-Outcome V2 Instructions (Pure Vault Mode)
// ============================================================================

/// V2: MatchMintMulti using Vault CPI (no SPL Token)
/// 
/// Multi-outcome Complete Set Mint:
/// When sum of all outcome buy prices <= 1.0, lock buyer funds via Vault CPI
/// and record virtual token holdings in MultiOutcomePosition PDA.
/// 
/// Account layout:
/// 0. [signer] Relayer/Matcher
/// 1. [] PredictionMarketConfig
/// 2. [writable] Market
/// 3. [] VaultConfig
/// 4. [] Vault Program
/// 5. [] System Program
/// Dynamic accounts (4 per outcome, for i in 0..num_outcomes):
///   6 + 4*i + 0: [writable] Order PDA
///   6 + 4*i + 1: [writable] Buyer MultiOutcomePosition PDA
///   6 + 4*i + 2: [writable] Buyer UserAccount (Vault)
///   6 + 4*i + 3: [writable] Buyer PMUserAccount (Vault)
fn process_match_mint_multi_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: MatchMintMultiV2Args,
) -> ProgramResult {
    use crate::state::{MAX_OUTCOMES_FOR_MATCH, MultiOutcomePosition, MULTI_OUTCOME_POSITION_DISCRIMINATOR};
    
    let account_info_iter = &mut accounts.iter();
    
    // ========== Fixed Accounts (6) ==========
    
    // Account 0: Relayer (signer)
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if config.is_paused {
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    // Verify relayer authorization
    verify_relayer(&config, relayer_info.key)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    if !market.is_tradeable() {
        return Err(PredictionMarketError::MarketNotTradeable.into());
    }
    
    // Verify market is multi-outcome type
    if market.market_type != MarketType::MultiOutcome {
        msg!("Error: MatchMintMultiV2 requires MultiOutcome market type");
        return Err(PredictionMarketError::InvalidMarketType.into());
    }
    
    // Validate num_outcomes
    if args.num_outcomes < 2 || args.num_outcomes > MAX_OUTCOMES_FOR_MATCH {
        msg!("Invalid num_outcomes: {}, max is {}", args.num_outcomes, MAX_OUTCOMES_FOR_MATCH);
        return Err(PredictionMarketError::InvalidArgument.into());
    }
    
    if args.num_outcomes != market.num_outcomes {
        msg!("num_outcomes {} != market.num_outcomes {}", args.num_outcomes, market.num_outcomes);
        return Err(PredictionMarketError::InvalidArgument.into());
    }
    
    // Validate orders count matches num_outcomes
    if args.orders.len() != args.num_outcomes as usize {
        msg!("Orders count {} != num_outcomes {}", args.orders.len(), args.num_outcomes);
        return Err(PredictionMarketError::InvalidArgument.into());
    }
    
    // Validate price sum <= 1.0 (price conservation for minting)
    let total_price: u64 = args.orders.iter().map(|(_, _, p)| p).sum();
    if total_price > PRICE_PRECISION {
        msg!("Total price {} > 1.0 ({})", total_price, PRICE_PRECISION);
        return Err(PredictionMarketError::InvalidPricePair.into());
    }
    
    // Account 3: VaultConfig
    let vault_config_info = next_account_info(account_info_iter)?;
    
    // Account 4: Vault Program
    let vault_program_info = next_account_info(account_info_iter)?;
    
    // Account 5: System Program
    let system_program_info = next_account_info(account_info_iter)?;
    
    // ========== Dynamic Accounts (4 per outcome) ==========
    
    let market_id_bytes = args.market_id.to_le_bytes();
    let current_time = get_current_timestamp()?;
    let match_amount = args.amount;
    
    // Derive Config PDA for CPI signing
    let (config_pda, config_bump) = Pubkey::find_program_address(
        &[PM_CONFIG_SEED],
        program_id,
    );
    
    if *config_info.key != config_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let config_seeds: &[&[u8]] = &[PM_CONFIG_SEED, &[config_bump]];
    
    // Process each outcome
    for i in 0..args.num_outcomes as usize {
        let (expected_outcome_idx, order_id, price) = args.orders[i];
        
        // Verify outcome_index is sequential
        if expected_outcome_idx != i as u8 {
            msg!("Error: outcome_index {} at position {} (expected {})", expected_outcome_idx, i, i);
            return Err(PredictionMarketError::InvalidOutcome.into());
        }
        
        // Parse accounts for this outcome
        let order_info = next_account_info(account_info_iter)?;
        let position_info = next_account_info(account_info_iter)?;
        let user_account_info = next_account_info(account_info_iter)?;
        let pm_user_account_info = next_account_info(account_info_iter)?;
        
        // Verify Order PDA
        let order_id_bytes = order_id.to_le_bytes();
        let (order_pda, _) = Pubkey::find_program_address(
            &[ORDER_SEED, &market_id_bytes, &order_id_bytes],
            program_id,
        );
        if *order_info.key != order_pda {
            msg!("Error: Invalid Order PDA for outcome {}", i);
            return Err(PredictionMarketError::InvalidPDA.into());
        }
        
        // Load and validate order
        let mut order = deserialize_account::<Order>(&order_info.data.borrow())?;
        
        if order.discriminator != ORDER_DISCRIMINATOR {
            return Err(PredictionMarketError::InvalidAccountData.into());
        }
        
        // Verify order is a Buy order
        if order.side != crate::state::OrderSide::Buy {
            msg!("Error: Order {} must be Buy order for MatchMintMultiV2", order_id);
            return Err(PredictionMarketError::InvalidOrderSide.into());
        }
        
        // Verify outcome_index matches
        if order.outcome_index != expected_outcome_idx {
            msg!("Error: Order outcome_index {} != expected {}", order.outcome_index, expected_outcome_idx);
            return Err(PredictionMarketError::InvalidOutcome.into());
        }
        
        // Verify order is active
        if !order.is_active() {
            msg!("Error: Order {} is not active", order_id);
            return Err(PredictionMarketError::OrderNotActive.into());
        }
        
        // Verify remaining amount
        let remaining = order.remaining_amount();
        if remaining < match_amount {
            msg!("Error: Order {} remaining {} < match_amount {}", order_id, remaining, match_amount);
            return Err(PredictionMarketError::InvalidAmount.into());
        }
        
        // Calculate buyer cost: cost = amount * price / 1_000_000
        let buyer_cost = (match_amount as u128)
            .checked_mul(price as u128)
            .ok_or(PredictionMarketError::ArithmeticOverflow)?
            .checked_div(PRICE_PRECISION as u128)
            .ok_or(PredictionMarketError::ArithmeticOverflow)? as u64;
        
        // CPI: Lock buyer funds via Vault
        msg!("CPI: Lock {} for outcome {} buyer", buyer_cost, expected_outcome_idx);
        cpi_lock_for_prediction(
            vault_program_info,
            vault_config_info,
            user_account_info,
            pm_user_account_info,
            config_info,
            relayer_info,
            system_program_info,
            buyer_cost,
            config_seeds,
        )?;
        
        // Update MultiOutcomePosition: add holdings
        // Note: Position should be initialized beforehand
        // If not, initialize a new one
        let mut position = if position_info.data_len() > 0 && position_info.data.borrow()[0] != 0 {
            deserialize_account::<MultiOutcomePosition>(&position_info.data.borrow())?
        } else {
            // Initialize new position using constructor
            MultiOutcomePosition::new(
                args.market_id,
                args.num_outcomes,
                order.owner,
                0, // bump will be calculated if needed
                current_time,
            )
        };
        
        // Add to holdings for this outcome
        let holding_idx = expected_outcome_idx as usize;
        if holding_idx >= position.holdings.len() {
            return Err(PredictionMarketError::InvalidOutcome.into());
        }
        position.holdings[holding_idx] = position.holdings[holding_idx].saturating_add(match_amount);
        position.total_cost_e6 = position.total_cost_e6.saturating_add(buyer_cost);
        position.updated_at = current_time;
        position.serialize(&mut *position_info.data.borrow_mut())?;
        
        // Update order
        order.filled_amount = order.filled_amount.saturating_add(match_amount);
        if order.filled_amount >= order.amount {
            order.status = OrderStatus::Filled;
        } else {
            order.status = OrderStatus::PartialFilled;
        }
        order.updated_at = current_time;
        order.serialize(&mut *order_info.data.borrow_mut())?;
        
        msg!("Outcome {}: order={}, cost={}, new_holding={}", 
             expected_outcome_idx, order_id, buyer_cost, position.holdings[holding_idx]);
    }
    
    // Update market stats
    market.total_minted = market.total_minted.saturating_add(match_amount);
    market.total_volume_e6 = market.total_volume_e6.saturating_add((match_amount as i64) * (total_price as i64) / 1_000_000);
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    // NOTE: Fee collection will be implemented in Vault Program layer (V2 architecture)
    
    msg!("✅ MatchMintMultiV2 completed");
    msg!("Market: {}, Outcomes: {}", args.market_id, args.num_outcomes);
    msg!("Amount: {}, Total Price: {}", match_amount, total_price);
    msg!("Total Minted: {}", market.total_minted);
    
    Ok(())
}

/// V2: MatchBurnMulti using Vault CPI (no SPL Token)
/// 
/// Multi-outcome Complete Set Burn:
/// When sum of all outcome sell prices >= 1.0, settle seller funds via Vault CPI
/// and reduce virtual token holdings in MultiOutcomePosition PDA.
fn process_match_burn_multi_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: MatchBurnMultiV2Args,
) -> ProgramResult {
    use crate::state::{MAX_OUTCOMES_FOR_MATCH, MultiOutcomePosition, MULTI_OUTCOME_POSITION_DISCRIMINATOR};
    
    let account_info_iter = &mut accounts.iter();
    
    // ========== Fixed Accounts (6) ==========
    
    // Account 0: Relayer (signer)
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if config.is_paused {
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    verify_relayer(&config, relayer_info.key)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    if !market.is_tradeable() {
        return Err(PredictionMarketError::MarketNotTradeable.into());
    }
    
    if market.market_type != MarketType::MultiOutcome {
        msg!("Error: MatchBurnMultiV2 requires MultiOutcome market type");
        return Err(PredictionMarketError::InvalidMarketType.into());
    }
    
    if args.num_outcomes < 2 || args.num_outcomes > MAX_OUTCOMES_FOR_MATCH {
        msg!("Invalid num_outcomes: {}", args.num_outcomes);
        return Err(PredictionMarketError::InvalidArgument.into());
    }
    
    if args.num_outcomes != market.num_outcomes {
        return Err(PredictionMarketError::InvalidArgument.into());
    }
    
    if args.orders.len() != args.num_outcomes as usize {
        return Err(PredictionMarketError::InvalidArgument.into());
    }
    
    // Validate price sum >= 1.0 (price conservation for burning)
    let total_price: u64 = args.orders.iter().map(|(_, _, p)| p).sum();
    if total_price < PRICE_PRECISION {
        msg!("Total price {} < 1.0 ({})", total_price, PRICE_PRECISION);
        return Err(PredictionMarketError::InvalidPricePair.into());
    }
    
    // Account 3: VaultConfig
    let vault_config_info = next_account_info(account_info_iter)?;
    
    // Account 4: Vault Program
    let vault_program_info = next_account_info(account_info_iter)?;
    
    // Account 5: System Program
    let _system_program_info = next_account_info(account_info_iter)?;
    
    // ========== Dynamic Accounts (4 per outcome) ==========
    
    let market_id_bytes = args.market_id.to_le_bytes();
    let current_time = get_current_timestamp()?;
    let match_amount = args.amount;
    
    // Derive Config PDA for CPI signing
    let (config_pda, config_bump) = Pubkey::find_program_address(
        &[PM_CONFIG_SEED],
        program_id,
    );
    
    if *config_info.key != config_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let config_seeds: &[&[u8]] = &[PM_CONFIG_SEED, &[config_bump]];
    
    // Process each outcome
    for i in 0..args.num_outcomes as usize {
        let (expected_outcome_idx, order_id, price) = args.orders[i];
        
        if expected_outcome_idx != i as u8 {
            return Err(PredictionMarketError::InvalidOutcome.into());
        }
        
        // Parse accounts for this outcome
        let order_info = next_account_info(account_info_iter)?;
        let position_info = next_account_info(account_info_iter)?;
        let _user_account_info = next_account_info(account_info_iter)?;
        let pm_user_account_info = next_account_info(account_info_iter)?;
        
        // Verify Order PDA
        let order_id_bytes = order_id.to_le_bytes();
        let (order_pda, _) = Pubkey::find_program_address(
            &[ORDER_SEED, &market_id_bytes, &order_id_bytes],
            program_id,
        );
        if *order_info.key != order_pda {
            return Err(PredictionMarketError::InvalidPDA.into());
        }
        
        // Load and validate order
        let mut order = deserialize_account::<Order>(&order_info.data.borrow())?;
        
        if order.discriminator != ORDER_DISCRIMINATOR {
            return Err(PredictionMarketError::InvalidAccountData.into());
        }
        
        // Verify order is a Sell order
        if order.side != crate::state::OrderSide::Sell {
            msg!("Error: Order {} must be Sell order for MatchBurnMultiV2", order_id);
            return Err(PredictionMarketError::InvalidOrderSide.into());
        }
        
        if order.outcome_index != expected_outcome_idx {
            return Err(PredictionMarketError::InvalidOutcome.into());
        }
        
        if !order.is_active() {
            return Err(PredictionMarketError::OrderNotActive.into());
        }
        
        let remaining = order.remaining_amount();
        if remaining < match_amount {
            msg!("Error: Order remaining {} < match_amount {}", remaining, match_amount);
            return Err(PredictionMarketError::InvalidAmount.into());
        }
        
        // Load and validate position
        let mut position = deserialize_account::<MultiOutcomePosition>(&position_info.data.borrow())?;
        
        if position.discriminator != MULTI_OUTCOME_POSITION_DISCRIMINATOR {
            return Err(PredictionMarketError::InvalidAccountData.into());
        }
        
        // Verify seller has sufficient LOCKED holdings (locked when Sell order was placed)
        let holding_idx = expected_outcome_idx as usize;
        if holding_idx >= position.holdings.len() {
            return Err(PredictionMarketError::InvalidOutcome.into());
        }
        
        if position.locked[holding_idx] < match_amount {
            msg!("Error: Seller has insufficient locked holdings: {} < {} (total: {})", 
                 position.locked[holding_idx], match_amount, position.holdings[holding_idx]);
            return Err(PredictionMarketError::InsufficientPosition.into());
        }
        
        // Calculate seller proceeds: proceeds = amount * price / 1_000_000
        let seller_proceeds = (match_amount as u128)
            .checked_mul(price as u128)
            .ok_or(PredictionMarketError::ArithmeticOverflow)?
            .checked_div(PRICE_PRECISION as u128)
            .ok_or(PredictionMarketError::ArithmeticOverflow)? as u64;
        
        // CPI: Settle seller funds via Vault (locked=0, settlement=proceeds)
        msg!("CPI: Settle {} for outcome {} seller", seller_proceeds, expected_outcome_idx);
        cpi_prediction_settle(
            vault_program_info,
            vault_config_info,
            pm_user_account_info,
            config_info,
            0,              // locked_amount: seller didn't lock for sell
            seller_proceeds, // settlement_amount
            config_seeds,
        )?;
        
        // Update position: consume locked shares (unlock + reduce holdings)
        position.consume_locked_shares(expected_outcome_idx, match_amount, price, current_time)
            .map_err(|_| {
                msg!("Error: Failed to consume locked shares for outcome {}", expected_outcome_idx);
                PredictionMarketError::InsufficientPosition
            })?;
        position.serialize(&mut *position_info.data.borrow_mut())?;
        
        // Update order
        order.filled_amount = order.filled_amount.saturating_add(match_amount);
        if order.filled_amount >= order.amount {
            order.status = OrderStatus::Filled;
        } else {
            order.status = OrderStatus::PartialFilled;
        }
        order.updated_at = current_time;
        order.serialize(&mut *order_info.data.borrow_mut())?;
        
        msg!("Outcome {}: order={}, proceeds={}, remaining_holding={}", 
             expected_outcome_idx, order_id, seller_proceeds, position.holdings[holding_idx]);
    }
    
    // Update market stats
    market.total_minted = market.total_minted.saturating_sub(match_amount);
    market.total_volume_e6 = market.total_volume_e6.saturating_add((match_amount as i64) * (total_price as i64) / 1_000_000);
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    // NOTE: Fee collection will be implemented in Vault Program layer (V2 architecture)
    
    msg!("✅ MatchBurnMultiV2 completed");
    msg!("Market: {}, Outcomes: {}", args.market_id, args.num_outcomes);
    msg!("Amount: {}, Total Price: {}", match_amount, total_price);
    msg!("Total Minted: {}", market.total_minted);
    
    Ok(())
}

// ============================================================================
// V2 Relayer Order Instructions
// ============================================================================

/// V2: RelayerPlaceOrder with Vault CPI for margin lock
/// 
/// Places order on behalf of user and locks margin via Vault CPI.
/// Buy orders lock funds, Sell orders require Position holdings.
fn process_relayer_place_order_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RelayerPlaceOrderV2Args,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Relayer (signer)
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if config.is_paused {
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    verify_relayer(&config, relayer_info.key)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    if !market.is_tradeable() {
        return Err(PredictionMarketError::MarketNotTradeable.into());
    }
    
    // Account 3: Order PDA (writable, new)
    let order_info = next_account_info(account_info_iter)?;
    
    // Account 4: Position PDA
    let position_info = next_account_info(account_info_iter)?;
    
    // Account 5: User Vault Account
    let user_vault_info = next_account_info(account_info_iter)?;
    
    // Account 6: PM User Account
    let pm_user_info = next_account_info(account_info_iter)?;
    
    // Account 7: Vault Config
    let vault_config_info = next_account_info(account_info_iter)?;
    
    // Account 8: Vault Program
    let vault_program_info = next_account_info(account_info_iter)?;
    
    // Account 9: System Program
    let system_program_info = next_account_info(account_info_iter)?;
    
    // Derive and verify Order PDA
    let order_id = market.next_order_id;
    let market_id_bytes = args.market_id.to_le_bytes();
    let order_id_bytes = order_id.to_le_bytes();
    let (order_pda, order_bump) = Pubkey::find_program_address(
        &[ORDER_SEED, &market_id_bytes, &order_id_bytes],
        program_id,
    );
    
    if *order_info.key != order_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Calculate margin requirement (in e6 precision)
    // margin_e6 = amount * price_e6
    // Example: 100 contracts × 500,000 (50%) = 50,000,000 e6 = $50 USDC
    // NOTE: Do NOT divide by PRICE_PRECISION! price is already in e6 format.
    let margin = (args.amount as u128)
        .checked_mul(args.price as u128)
        .ok_or(PredictionMarketError::ArithmeticOverflow)? as u64;
    
    let current_time = get_current_timestamp()?;
    
    // Derive Config PDA for CPI signing
    let (config_pda, config_bump) = Pubkey::find_program_address(
        &[PM_CONFIG_SEED],
        program_id,
    );
    
    if *config_info.key != config_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let config_seeds: &[&[u8]] = &[PM_CONFIG_SEED, &[config_bump]];
    
    // For Buy orders: Lock margin in Vault
    if args.side == crate::state::OrderSide::Buy {
        msg!("CPI: Lock margin {} for Buy order", margin);
        cpi_lock_for_prediction(
            vault_program_info,
            vault_config_info,
            user_vault_info,
            pm_user_info,
            config_info,
            relayer_info,
            system_program_info,
            margin,
            config_seeds,
        )?;
    } else {
        // For Sell orders: Verify Position has sufficient AVAILABLE holdings and LOCK them
        let mut position = deserialize_account::<Position>(&position_info.data.borrow())?;
        if position.discriminator != POSITION_DISCRIMINATOR {
            return Err(PredictionMarketError::InvalidAccountData.into());
        }
        
        // Check available (total - locked), not just total
        let available = position.available(args.outcome);
        
        if available < args.amount {
            msg!("Error: Insufficient available holdings: {} < {} (total: {}, locked: {})", 
                 available, args.amount,
                 match args.outcome {
                     Outcome::Yes => position.yes_amount,
                     Outcome::No => position.no_amount,
                 },
                 position.locked(args.outcome));
            return Err(PredictionMarketError::InsufficientPosition.into());
        }
        
        // Lock shares for this Sell order
        position.lock_shares(args.outcome, args.amount)
            .map_err(|_| PredictionMarketError::InsufficientPosition)?;
        
        position.updated_at = current_time;
        position.serialize(&mut *position_info.data.borrow_mut())?;
        
        msg!("📊 Position locked: {} {:?} shares", args.amount, args.outcome);
    }
    
    // Get outcome index
    let outcome_index = match args.outcome {
        Outcome::Yes => 0,
        Outcome::No => 1,
    };
    
    // Create Order
    let order_space = Order::SIZE;
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(order_space);
    
    // Create account via CPI
    let order_seeds: &[&[u8]] = &[ORDER_SEED, &market_id_bytes, &order_id_bytes, &[order_bump]];
    
    invoke_signed(
        &system_instruction::create_account(
            relayer_info.key,
            order_info.key,
            lamports,
            order_space as u64,
            program_id,
        ),
        &[relayer_info.clone(), order_info.clone(), system_program_info.clone()],
        &[order_seeds],
    )?;
    
    // Initialize Order
    let order = Order {
        discriminator: ORDER_DISCRIMINATOR,
        order_id,
        market_id: args.market_id,
        owner: args.user_wallet,
        side: args.side,
        outcome: args.outcome,
        outcome_index,
        price: args.price,
        amount: args.amount,
        filled_amount: 0,
        status: OrderStatus::Open,
        order_type: args.order_type,
        expiration_time: args.expiration_time,
        created_at: current_time,
        updated_at: current_time,
        bump: order_bump,
        escrow_token_account: None, // V2: No SPL token escrow
        reserved: [0u8; 30],
    };
    order.serialize(&mut *order_info.data.borrow_mut())?;
    
    // Update market
    market.next_order_id = market.next_order_id.saturating_add(1);
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    msg!("✅ RelayerPlaceOrderV2 completed");
    msg!("User: {}", args.user_wallet);
    msg!("Order ID: {}, Market: {}", order_id, args.market_id);
    msg!("Side: {:?}, Outcome: {:?}", args.side, args.outcome);
    msg!("Price: {}, Amount: {}, Margin: {}", args.price, args.amount, margin);
    
    Ok(())
}

/// V2: RelayerCancelOrder with Vault CPI for margin unlock
/// 
/// Cancels order and unlocks remaining margin via Vault CPI.
fn process_relayer_cancel_order_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RelayerCancelOrderV2Args,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Relayer (signer)
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if config.is_paused {
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    verify_relayer(&config, relayer_info.key)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    // Account 3: Order PDA (writable)
    let order_info = next_account_info(account_info_iter)?;
    let mut order = deserialize_account::<Order>(&order_info.data.borrow())?;
    
    if order.discriminator != ORDER_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify Order PDA
    let market_id_bytes = args.market_id.to_le_bytes();
    let order_id_bytes = args.order_id.to_le_bytes();
    let (order_pda, _) = Pubkey::find_program_address(
        &[ORDER_SEED, &market_id_bytes, &order_id_bytes],
        program_id,
    );
    
    if *order_info.key != order_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Verify order owner
    if order.owner != args.user_wallet {
        return Err(PredictionMarketError::Unauthorized.into());
    }
    
    // Verify order is cancellable
    if !order.is_active() {
        return Err(PredictionMarketError::OrderNotActive.into());
    }
    
    // Account 4: Position PDA (for Sell order share unlock)
    let position_info = next_account_info(account_info_iter)?;
    
    // Account 5: User Vault Account
    let user_vault_info = next_account_info(account_info_iter)?;
    
    // Account 6: PM User Account
    let pm_user_info = next_account_info(account_info_iter)?;
    
    // Account 7: Vault Config
    let vault_config_info = next_account_info(account_info_iter)?;
    
    // Account 8: Vault Program
    let vault_program_info = next_account_info(account_info_iter)?;
    
    // Account 9: System Program
    let _system_program_info = next_account_info(account_info_iter)?;
    
    // Calculate remaining margin to unlock (in e6 precision)
    // remaining_margin_e6 = remaining_amount * price_e6
    // NOTE: Do NOT divide by PRICE_PRECISION! price is already in e6 format.
    let remaining = order.remaining_amount();
    let remaining_margin = (remaining as u128)
        .checked_mul(order.price as u128)
        .ok_or(PredictionMarketError::ArithmeticOverflow)? as u64;
    
    let current_time = get_current_timestamp()?;
    
    // Derive Config PDA for CPI signing
    let (config_pda, config_bump) = Pubkey::find_program_address(
        &[PM_CONFIG_SEED],
        program_id,
    );
    
    if *config_info.key != config_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let config_seeds: &[&[u8]] = &[PM_CONFIG_SEED, &[config_bump]];
    
    // Handle order cancellation based on side
    if order.side == crate::state::OrderSide::Buy {
        // For Buy orders: Unlock remaining margin from Vault
        if remaining_margin > 0 {
            msg!("CPI: Unlock remaining margin {} for cancelled Buy order", remaining_margin);
            cpi_release_from_prediction(
                vault_program_info,
                vault_config_info,
                user_vault_info,
                pm_user_info,
                config_info,
                remaining_margin,
                config_seeds,
            )?;
        }
    } else {
        // For Sell orders: Unlock remaining shares from Position
        if remaining > 0 {
            // Verify Position PDA
            let (position_pda, _) = Pubkey::find_program_address(
                &[POSITION_SEED, &market_id_bytes, order.owner.as_ref()],
                program_id,
            );
            
            if *position_info.key != position_pda {
                msg!("Error: Invalid Position PDA for Sell order cancellation");
                return Err(PredictionMarketError::InvalidPDA.into());
            }
            
            let mut position = deserialize_account::<Position>(&position_info.data.borrow())?;
            if position.discriminator != POSITION_DISCRIMINATOR {
                return Err(PredictionMarketError::InvalidAccountData.into());
            }
            
            // Unlock the remaining locked shares
            position.unlock_shares(order.outcome, remaining)
                .map_err(|_| {
                    msg!("Error: Failed to unlock shares - locked amount mismatch");
                    PredictionMarketError::InsufficientPosition
                })?;
            
            position.updated_at = current_time;
            position.serialize(&mut *position_info.data.borrow_mut())?;
            
            msg!("📊 Position unlocked: {} {:?} shares for cancelled Sell order", remaining, order.outcome);
        }
    }
    
    // Update order status
    order.status = OrderStatus::Cancelled;
    order.updated_at = current_time;
    order.serialize(&mut *order_info.data.borrow_mut())?;
    
    // Update market stats
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    msg!("✅ RelayerCancelOrderV2 completed");
    msg!("User: {}", args.user_wallet);
    msg!("Order ID: {}, Market: {}", args.order_id, args.market_id);
    msg!("Remaining amount: {}, Unlocked margin: {}", remaining, remaining_margin);
    
    Ok(())
}

// ============================================================================
// V2 WithFee Instructions
// ============================================================================

/// Process RelayerMintCompleteSetV2WithFee
/// 
/// Same as RelayerMintCompleteSetV2 but uses Vault.PredictionMarketLockWithFee
/// to collect minting fee during the lock operation.
/// 
/// Accounts:
/// 0. `[signer]` Relayer
/// 1. `[]` PredictionMarketConfig
/// 2. `[writable]` Market
/// 3. `[writable]` Position PDA
/// 4. `[writable]` User Vault Account
/// 5. `[writable]` PM User Account
/// 6. `[]` Vault Config
/// 7. `[]` Vault Program
/// 8. `[]` System Program
/// 9. `[writable]` Vault Token Account
/// 10. `[writable]` PM Fee Vault
/// 11. `[writable]` PM Fee Config PDA
/// 12. `[]` Token Program
fn process_relayer_mint_complete_set_v2_with_fee(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RelayerMintCompleteSetArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Relayer (signer)
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: Position PDA (writable)
    let position_info = next_account_info(account_info_iter)?;
    
    // Account 4: User Vault Account (writable)
    let user_vault_info = next_account_info(account_info_iter)?;
    
    // Account 5: PM User Account (writable)
    let pm_user_account_info = next_account_info(account_info_iter)?;
    
    // Account 6: Vault Config
    let vault_config_info = next_account_info(account_info_iter)?;
    
    // Account 7: Vault Program
    let vault_program_info = next_account_info(account_info_iter)?;
    
    // Account 8: System Program
    let system_program_info = next_account_info(account_info_iter)?;
    
    // Account 9: Vault Token Account (for fee transfer)
    let vault_token_account_info = next_account_info(account_info_iter)?;
    
    // Account 10: PM Fee Vault
    let pm_fee_vault_info = next_account_info(account_info_iter)?;
    
    // Account 11: PM Fee Config PDA
    let pm_fee_config_info = next_account_info(account_info_iter)?;
    
    // Account 12: Token Program
    let token_program_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify Relayer authority
    verify_relayer(&config, relayer_info.key)?;
    
    if config.is_paused {
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    // Load and validate market
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    if !market.is_tradeable() {
        return Err(PredictionMarketError::MarketNotTradeable.into());
    }
    
    // Validate amount
    if args.amount == 0 {
        return Err(PredictionMarketError::InvalidAmount.into());
    }
    
    let current_time = get_current_timestamp()?;
    let market_id_bytes = market.market_id.to_le_bytes();
    
    // Derive Config PDA for CPI signing
    let (config_pda, config_bump) = Pubkey::find_program_address(
        &[PM_CONFIG_SEED],
        program_id,
    );
    
    if *config_info.key != config_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let config_seeds: &[&[u8]] = &[PM_CONFIG_SEED, &[config_bump]];
    
    // Read PM Fee Config to calculate net_amount
    // PM Fee Config offsets (matching Fund Program state.rs):
    // - offset 41: minting_fee_bps (u16)
    const PM_FEE_MINTING_BPS_OFFSET: usize = 41;
    let pm_fee_config_data = pm_fee_config_info.try_borrow_data()?;
    if pm_fee_config_data.len() < 50 {
        msg!("❌ PM Fee Config not initialized");
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    let minting_fee_bps = u16::from_le_bytes([
        pm_fee_config_data[PM_FEE_MINTING_BPS_OFFSET],
        pm_fee_config_data[PM_FEE_MINTING_BPS_OFFSET + 1],
    ]);
    drop(pm_fee_config_data);
    
    // Calculate fee and net_amount
    let fee_amount = ((args.amount as u128) * (minting_fee_bps as u128) / 10000) as u64;
    let net_amount = args.amount.saturating_sub(fee_amount);
    
    msg!("Fee calculation: gross={}, fee_bps={}, fee={}, net={}", 
         args.amount, minting_fee_bps, fee_amount, net_amount);
    
    // Step 1: CPI to Vault - PredictionMarketLockWithFee
    // This locks the funds AND collects the minting fee
    msg!("CPI: Vault.PredictionMarketLockWithFee gross_amount={}", args.amount);
    cpi_lock_for_prediction_with_fee(
        vault_program_info,
        vault_config_info,
        user_vault_info,
        pm_user_account_info,
        config_info,  // PM Config as caller program marker
        vault_token_account_info,
        pm_fee_vault_info,
        pm_fee_config_info,
        token_program_info,
        relayer_info, // Payer for auto-init
        system_program_info,
        args.amount,
        config_seeds,
    )?;
    
    // Step 2: Create or update Position PDA
    let (position_pda, position_bump) = Pubkey::find_program_address(
        &[POSITION_SEED, &market_id_bytes, args.user_wallet.as_ref()],
        program_id,
    );
    
    if *position_info.key != position_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let is_new_position = position_info.data_is_empty();
    
    if is_new_position {
        // Create new Position account
        let rent = Rent::get()?;
        let space = Position::SIZE;
        let lamports = rent.minimum_balance(space);
        let position_seeds: &[&[u8]] = &[
            POSITION_SEED, 
            &market_id_bytes, 
            args.user_wallet.as_ref(), 
            &[position_bump]
        ];
        
        invoke_signed(
            &system_instruction::create_account(
                relayer_info.key,
                position_info.key,
                lamports,
                space as u64,
                program_id,
            ),
            &[relayer_info.clone(), position_info.clone(), system_program_info.clone()],
            &[position_seeds],
        )?;
        
        let position = Position {
            discriminator: POSITION_DISCRIMINATOR,
            market_id: args.market_id,
            owner: args.user_wallet,
            yes_amount: net_amount,  // Use net_amount after fee
            no_amount: net_amount,   // Use net_amount after fee
            yes_locked: 0,
            no_locked: 0,
            yes_avg_cost: PRICE_PRECISION / 2, // 0.5 for complete set
            no_avg_cost: PRICE_PRECISION / 2,
            realized_pnl: 0,
            total_cost_e6: args.amount,  // Record gross amount as cost basis
            settled: false,
            settlement_amount: 0,
            created_at: current_time,
            updated_at: current_time,
            bump: position_bump,
            reserved: [0u8; 16],
        };
        position.serialize(&mut &mut position_info.data.borrow_mut()[..])?;
        
        msg!("Created new Position PDA for user {} in market {}", 
             args.user_wallet, args.market_id);
    } else {
        // Update existing Position
        let mut position = deserialize_account::<Position>(&position_info.data.borrow())?;
        
        if position.discriminator != POSITION_DISCRIMINATOR {
            return Err(PredictionMarketError::InvalidAccountData.into());
        }
        
        if position.owner != args.user_wallet || position.market_id != args.market_id {
            return Err(PredictionMarketError::PositionNotFound.into());
        }
        
        position.yes_amount = safe_add_u64(position.yes_amount, net_amount)?;
        position.no_amount = safe_add_u64(position.no_amount, net_amount)?;
        position.total_cost_e6 = safe_add_u64(position.total_cost_e6, args.amount)?;
        position.updated_at = current_time;
        
        position.serialize(&mut &mut position_info.data.borrow_mut()[..])?;
        
        msg!("Updated Position: +{} YES, +{} NO shares (net after fee)", net_amount, net_amount);
    }
    
    // Step 3: Update market stats (use net_amount for shares)
    market.total_minted = safe_add_u64(market.total_minted, net_amount)?;
    market.updated_at = current_time;
    market.serialize(&mut &mut market_info.data.borrow_mut()[..])?;
    
    msg!("✅ RelayerMintCompleteSetV2WithFee completed");
    msg!("User: {}, Market: {}", args.user_wallet, args.market_id);
    msg!("Gross: {}, Fee: {}, Net shares: {}", args.amount, fee_amount, net_amount);
    
    Ok(())
}

/// Process RelayerRedeemCompleteSetV2WithFee
/// 
/// Same as RelayerRedeemCompleteSetV2 but uses Vault.PredictionMarketUnlockWithFee
/// to collect redemption fee during the unlock operation.
/// 
/// Accounts:
/// 0. `[signer]` Relayer
/// 1. `[]` PredictionMarketConfig
/// 2. `[writable]` Market
/// 3. `[writable]` Position PDA
/// 4. `[writable]` User Vault Account
/// 5. `[writable]` PM User Account
/// 6. `[]` Vault Config
/// 7. `[]` Vault Program
/// 8. `[writable]` Vault Token Account
/// 9. `[writable]` PM Fee Vault
/// 10. `[writable]` PM Fee Config PDA
/// 11. `[]` Token Program
fn process_relayer_redeem_complete_set_v2_with_fee(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RelayerRedeemCompleteSetArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Relayer (signer)
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: Position PDA (writable)
    let position_info = next_account_info(account_info_iter)?;
    
    // Account 4: User Vault Account (writable)
    let user_vault_info = next_account_info(account_info_iter)?;
    
    // Account 5: PM User Account (writable)
    let pm_user_account_info = next_account_info(account_info_iter)?;
    
    // Account 6: Vault Config
    let vault_config_info = next_account_info(account_info_iter)?;
    
    // Account 7: Vault Program
    let vault_program_info = next_account_info(account_info_iter)?;
    
    // Account 8: Vault Token Account
    let vault_token_account_info = next_account_info(account_info_iter)?;
    
    // Account 9: PM Fee Vault
    let pm_fee_vault_info = next_account_info(account_info_iter)?;
    
    // Account 10: PM Fee Config PDA
    let pm_fee_config_info = next_account_info(account_info_iter)?;
    
    // Account 11: Token Program
    let token_program_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify Relayer authority
    verify_relayer(&config, relayer_info.key)?;
    
    if config.is_paused {
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    // Load and validate market
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    // For redemption, we only need the market to exist and not be resolved
    // Users should be able to redeem even from paused markets
    if market.status == MarketStatus::Resolved {
        return Err(PredictionMarketError::MarketAlreadyResolved.into());
    }
    
    // Validate amount
    if args.amount == 0 {
        return Err(PredictionMarketError::InvalidAmount.into());
    }
    
    let current_time = get_current_timestamp()?;
    let market_id_bytes = market.market_id.to_le_bytes();
    
    // Derive Config PDA for CPI signing
    let (config_pda, config_bump) = Pubkey::find_program_address(
        &[PM_CONFIG_SEED],
        program_id,
    );
    
    if *config_info.key != config_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let config_seeds: &[&[u8]] = &[PM_CONFIG_SEED, &[config_bump]];
    
    // Validate and update Position
    let (position_pda, _position_bump) = Pubkey::find_program_address(
        &[POSITION_SEED, &market_id_bytes, args.user_wallet.as_ref()],
        program_id,
    );
    
    if *position_info.key != position_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let mut position = deserialize_account::<Position>(&position_info.data.borrow())?;
    
    if position.discriminator != POSITION_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if position.owner != args.user_wallet || position.market_id != args.market_id {
        return Err(PredictionMarketError::PositionNotFound.into());
    }
    
    // Check user has enough shares to redeem
    let available_yes = position.yes_amount.saturating_sub(position.yes_locked);
    let available_no = position.no_amount.saturating_sub(position.no_locked);
    
    if available_yes < args.amount || available_no < args.amount {
        msg!("Insufficient shares: need {}, have YES={}, NO={}", 
             args.amount, available_yes, available_no);
        return Err(PredictionMarketError::InsufficientPosition.into());
    }
    
    // Burn virtual shares
    position.yes_amount = position.yes_amount.saturating_sub(args.amount);
    position.no_amount = position.no_amount.saturating_sub(args.amount);
    position.updated_at = current_time;
    position.serialize(&mut &mut position_info.data.borrow_mut()[..])?;
    
    // Step 2: CPI to Vault - PredictionMarketUnlockWithFee
    // This releases funds AND collects redemption fee
    msg!("CPI: Vault.PredictionMarketUnlockWithFee gross_amount={}", args.amount);
    cpi_release_from_prediction_with_fee(
        vault_program_info,
        vault_config_info,
        user_vault_info,
        pm_user_account_info,
        config_info,
        vault_token_account_info,
        pm_fee_vault_info,
        pm_fee_config_info,
        token_program_info,
        args.amount,
        config_seeds,
    )?;
    
    // Step 3: Update market stats
    market.total_minted = market.total_minted.saturating_sub(args.amount);
    market.updated_at = current_time;
    market.serialize(&mut &mut market_info.data.borrow_mut()[..])?;
    
    msg!("✅ RelayerRedeemCompleteSetV2WithFee completed");
    msg!("User: {}, Market: {}", args.user_wallet, args.market_id);
    msg!("Gross amount: {} (fee collected by Vault)", args.amount);
    
    Ok(())
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Verify that the caller is an authorized relayer
/// 
/// V2: Only admin can act as relayer (simplified model)
fn verify_relayer(config: &PredictionMarketConfig, relayer: &Pubkey) -> ProgramResult {
    // Check if the relayer is the admin or oracle_admin
    if *relayer == config.admin || *relayer == config.oracle_admin {
        return Ok(());
    }
    
    msg!("Unauthorized relayer: {}", relayer);
    Err(PredictionMarketError::Unauthorized.into())
}

// ============================================================================
// LLM Oracle Processors (Phase 4.6)
// ============================================================================

use crate::state::{
    MarketOracleData, OracleProposalData, ProposalType,
    MARKET_ORACLE_DATA_SEED, ORACLE_PROPOSAL_DATA_SEED,
    MARKET_ORACLE_DATA_DISCRIMINATOR, ORACLE_PROPOSAL_DATA_DISCRIMINATOR,
};

/// Task 4.6.1: Initialize market oracle data account
fn process_initialize_market_oracle_data(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: InitializeMarketOracleDataArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Admin (signer)
    let admin_info = next_account_info(account_info_iter)?;
    check_signer(admin_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: MarketOracleData PDA (writable, to be created)
    let oracle_data_info = next_account_info(account_info_iter)?;
    
    // Account 4: System Program
    let system_program_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Task 4.6.2: Verify admin authority
    if *admin_info.key != config.admin && *admin_info.key != config.oracle_admin {
        msg!("Unauthorized: {} is not admin", admin_info.key);
        return Err(PredictionMarketError::Unauthorized.into());
    }
    
    // Load and validate market
    let market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    // Derive and validate oracle data PDA
    let market_id_bytes = args.market_id.to_le_bytes();
    let (oracle_data_pda, oracle_data_bump) = Pubkey::find_program_address(
        &[MARKET_ORACLE_DATA_SEED, &market_id_bytes],
        program_id,
    );
    
    if *oracle_data_info.key != oracle_data_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Create the oracle data account
    let rent = Rent::get()?;
    let space = MarketOracleData::SIZE;
    let lamports = rent.minimum_balance(space);
    
    let create_account_ix = system_instruction::create_account(
        admin_info.key,
        oracle_data_info.key,
        lamports,
        space as u64,
        program_id,
    );
    
    let seeds: &[&[u8]] = &[MARKET_ORACLE_DATA_SEED, &market_id_bytes, &[oracle_data_bump]];
    
    invoke_signed(
        &create_account_ix,
        &[admin_info.clone(), oracle_data_info.clone(), system_program_info.clone()],
        &[seeds],
    )?;
    
    // Initialize the account data
    let current_time = get_current_timestamp()?;
    let oracle_data = MarketOracleData::new(args.market_id, oracle_data_bump, current_time, args.challenge_duration_secs);
    oracle_data.serialize(&mut &mut oracle_data_info.data.borrow_mut()[..])?;
    
    msg!("✅ Initialized MarketOracleData for market {}", args.market_id);
    
    Ok(())
}

/// Task 4.6.1-4.6.3: Set creation data on market oracle data
fn process_set_creation_data(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: SetCreationDataArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Admin (signer)
    let admin_info = next_account_info(account_info_iter)?;
    check_signer(admin_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: MarketOracleData (writable)
    let oracle_data_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify admin authority
    if *admin_info.key != config.admin && *admin_info.key != config.oracle_admin {
        return Err(PredictionMarketError::Unauthorized.into());
    }
    
    // Load and validate market - Task 4.6.3: only Pending status
    let market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    if market.status != MarketStatus::Pending {
        msg!("Market status must be Pending, got {:?}", market.status);
        return Err(PredictionMarketError::InvalidMarketStatus.into());
    }
    
    // Load and update oracle data
    let mut oracle_data = deserialize_account::<MarketOracleData>(&oracle_data_info.data.borrow())?;
    if oracle_data.discriminator != MARKET_ORACLE_DATA_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if oracle_data.market_id != args.market_id {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    let current_time = get_current_timestamp()?;
    oracle_data.set_creation_data(args.creation_data_cid, args.creation_data_hash, current_time);
    oracle_data.serialize(&mut &mut oracle_data_info.data.borrow_mut()[..])?;
    
    msg!("✅ Set creation data for market {}", args.market_id);
    
    Ok(())
}

/// Task 4.6.4-4.6.6: Freeze oracle config
fn process_freeze_oracle_config(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: FreezeOracleConfigArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Admin (signer)
    let admin_info = next_account_info(account_info_iter)?;
    check_signer(admin_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: MarketOracleData (writable)
    let oracle_data_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify admin authority
    if *admin_info.key != config.admin && *admin_info.key != config.oracle_admin {
        return Err(PredictionMarketError::Unauthorized.into());
    }
    
    // Load and update market - Task 4.6.6: transition Pending -> Active
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    // Load and update oracle data
    let mut oracle_data = deserialize_account::<MarketOracleData>(&oracle_data_info.data.borrow())?;
    if oracle_data.discriminator != MARKET_ORACLE_DATA_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if oracle_data.market_id != args.market_id {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Require creation data to be set first
    if !oracle_data.is_creation_data_set {
        msg!("Creation data must be set before freezing config");
        return Err(PredictionMarketError::InvalidMarketStatus.into());
    }
    
    let current_time = get_current_timestamp()?;
    oracle_data.freeze_config(args.oracle_config_cid, args.oracle_config_hash, current_time);
    oracle_data.serialize(&mut &mut oracle_data_info.data.borrow_mut()[..])?;
    
    // Transition market to Active if ready
    if market.status == MarketStatus::Pending && oracle_data.is_ready_for_trading() {
        market.status = MarketStatus::Active;
        market.updated_at = current_time;
        market.serialize(&mut &mut market_info.data.borrow_mut()[..])?;
        msg!("Market {} activated (config frozen)", args.market_id);
    }
    
    msg!("✅ Frozen oracle config for market {}", args.market_id);
    
    Ok(())
}

/// Task 4.6.7-4.6.8: Halt trading on market (end time reached)
fn process_halt_trading(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: HaltTradingArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Anyone (signer) - permissionless
    let caller_info = next_account_info(account_info_iter)?;
    check_signer(caller_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Load and update market
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    // Task 4.6.8: Time-based check - resolution time must have passed
    let current_time = get_current_timestamp()?;
    if current_time < market.resolution_time {
        msg!("Resolution time not reached: current={}, resolution={}", 
             current_time, market.resolution_time);
        return Err(PredictionMarketError::ResolutionTimeNotReached.into());
    }
    
    // Only Active markets can be halted
    if market.status != MarketStatus::Active {
        msg!("Market status must be Active, got {:?}", market.status);
        return Err(PredictionMarketError::InvalidMarketStatus.into());
    }
    
    // Transition to TradingHalted
    market.status = MarketStatus::TradingHalted;
    market.updated_at = current_time;
    market.serialize(&mut &mut market_info.data.borrow_mut()[..])?;
    
    msg!("✅ Halted trading for market {} (resolution time: {})", 
         args.market_id, market.resolution_time);
    
    Ok(())
}

/// Task 4.6.9-4.6.12: Propose result with research data
fn process_propose_result_with_research(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: ProposeResultWithResearchArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Oracle Admin (signer)
    let oracle_admin_info = next_account_info(account_info_iter)?;
    check_signer(oracle_admin_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: OracleProposal PDA (writable)
    let proposal_info = next_account_info(account_info_iter)?;
    
    // Account 4: OracleProposalData PDA (writable)
    let proposal_data_info = next_account_info(account_info_iter)?;
    
    // Account 5: MarketOracleData (for config hash verification)
    let oracle_data_info = next_account_info(account_info_iter)?;
    
    // Account 6+: Vault accounts for bond (skipped for now)
    let _vault_accounts = account_info_iter;
    
    // Load and validate config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify oracle admin authority
    if *oracle_admin_info.key != config.oracle_admin {
        msg!("Unauthorized: {} is not oracle_admin", oracle_admin_info.key);
        return Err(PredictionMarketError::Unauthorized.into());
    }
    
    // Load and update market
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    // Task 4.6.10: Verify oracle config hash matches frozen config
    let oracle_data = deserialize_account::<MarketOracleData>(&oracle_data_info.data.borrow())?;
    if oracle_data.discriminator != MARKET_ORACLE_DATA_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if !oracle_data.verify_config_hash(&args.oracle_config_hash) {
        msg!("Oracle config hash mismatch");
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Market must be TradingHalted or AwaitingResult
    if !matches!(market.status, MarketStatus::TradingHalted | MarketStatus::AwaitingResult) {
        msg!("Market status must be TradingHalted or AwaitingResult, got {:?}", market.status);
        return Err(PredictionMarketError::InvalidMarketStatus.into());
    }
    
    let current_time = get_current_timestamp()?;
    let market_id_bytes = args.market_id.to_le_bytes();
    
    // Create OracleProposal account (simplified - actual implementation would handle PDA creation)
    let (proposal_pda, _proposal_bump) = Pubkey::find_program_address(
        &[ORACLE_PROPOSAL_SEED, &market_id_bytes],
        program_id,
    );
    
    if *proposal_info.key != proposal_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Create OracleProposalData account (simplified)
    let (proposal_data_pda, proposal_data_bump) = Pubkey::find_program_address(
        &[ORACLE_PROPOSAL_DATA_SEED, &market_id_bytes],
        program_id,
    );
    
    if *proposal_data_info.key != proposal_data_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Task 4.6.11-4.6.12: Store research data in OracleProposalData
    let proposal_data = OracleProposalData::new_llm(
        args.market_id,
        args.research_data_cid,
        args.research_data_hash,
        args.outcome_index,
        args.confidence_score,
        args.requires_manual_review,
        proposal_data_bump,
        current_time,
    );
    proposal_data.serialize(&mut &mut proposal_data_info.data.borrow_mut()[..])?;
    
    // Update market status
    market.status = MarketStatus::ResultProposed;
    market.updated_at = current_time;
    market.serialize(&mut &mut market_info.data.borrow_mut()[..])?;
    
    msg!("✅ Proposed result for market {}: outcome={}, confidence={}", 
         args.market_id, args.outcome_index, args.confidence_score);
    
    Ok(())
}

/// Process manual result proposal (Admin override for UNDETERMINED cases)
/// 
/// Task 4.6.13-4.6.16: Manual proposal with evidence
/// 
/// Accounts:
/// 0. `[signer]` Oracle Admin
/// 1. `[]` PredictionMarketConfig
/// 2. `[writable]` Market
/// 3. `[writable]` OracleProposal PDA
/// 4. `[writable]` OracleProposalData PDA
/// 5. `[]` MarketOracleData (for original research reference)
/// 6. `[writable]` Admin's Vault Account (for bond)
/// 7. `[]` Vault Config
/// 8. `[]` Vault Program
/// 9. `[]` System Program
fn process_propose_result_manual(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: ProposeResultManualArgs,
) -> ProgramResult {
    msg!("ProposeResultManual: market={}, outcome={}", args.market_id, args.outcome_index);
    
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Oracle Admin (signer)
    let oracle_admin_info = next_account_info(account_info_iter)?;
    check_signer(oracle_admin_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: OracleProposal PDA (writable)
    let proposal_info = next_account_info(account_info_iter)?;
    
    // Account 4: OracleProposalData PDA (writable)
    let proposal_data_info = next_account_info(account_info_iter)?;
    
    // Account 5: MarketOracleData (for original research reference)
    let oracle_data_info = next_account_info(account_info_iter)?;
    
    // Account 6+: Vault accounts for bond (optional, skipped for now)
    let _remaining_accounts = account_info_iter;
    
    // Load and validate config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify oracle admin authority
    if *oracle_admin_info.key != config.oracle_admin {
        msg!("Unauthorized: {} is not oracle_admin", oracle_admin_info.key);
        return Err(PredictionMarketError::Unauthorized.into());
    }
    
    // Load and validate market
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    // Load MarketOracleData to get original research reference
    let oracle_data = deserialize_account::<MarketOracleData>(&oracle_data_info.data.borrow())?;
    if oracle_data.discriminator != MARKET_ORACLE_DATA_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Market must be TradingHalted, AwaitingResult, or ResultProposed (to override UNDETERMINED)
    if !matches!(
        market.status, 
        MarketStatus::TradingHalted | MarketStatus::AwaitingResult | MarketStatus::ResultProposed
    ) {
        msg!("Market status must be TradingHalted, AwaitingResult, or ResultProposed for manual override, got {:?}", 
             market.status);
        return Err(PredictionMarketError::InvalidMarketStatus.into());
    }
    
    let current_time = get_current_timestamp()?;
    let market_id_bytes = args.market_id.to_le_bytes();
    
    // Validate OracleProposal PDA
    let (proposal_pda, _proposal_bump) = Pubkey::find_program_address(
        &[ORACLE_PROPOSAL_SEED, &market_id_bytes],
        program_id,
    );
    
    if *proposal_info.key != proposal_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Validate OracleProposalData PDA
    let (proposal_data_pda, proposal_data_bump) = Pubkey::find_program_address(
        &[ORACLE_PROPOSAL_DATA_SEED, &market_id_bytes],
        program_id,
    );
    
    if *proposal_data_info.key != proposal_data_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Task 4.6.14-4.6.15: Create manual proposal data with evidence
    // Use research_data from original LLM attempt (if any)
    let research_cid = oracle_data.oracle_config_cid; // Reference to original config/research
    let research_hash = oracle_data.oracle_config_hash;
    
    let proposal_data = OracleProposalData::new_manual(
        args.market_id,
        research_cid,                    // Original research reference
        research_hash,                   // Original research hash
        args.manual_proposal_cid,        // Manual judgment IPFS CID
        args.manual_reasoning_hash,      // Manual reasoning hash
        args.outcome_index,              // Admin's determined outcome
        proposal_data_bump,
        current_time,
    );
    
    // Serialize proposal data to account
    proposal_data.serialize(&mut &mut proposal_data_info.data.borrow_mut()[..])?;
    
    // Update market status to ResultProposed
    market.status = MarketStatus::ResultProposed;
    market.updated_at = current_time;
    market.serialize(&mut &mut market_info.data.borrow_mut()[..])?;
    
    msg!("✅ Manual proposal for market {}: outcome={}, manual_cid={:?}", 
         args.market_id, 
         args.outcome_index,
         String::from_utf8_lossy(&args.manual_proposal_cid[0..20]));
    
    Ok(())
}

/// Process challenge with evidence (Task 4.6.17-4.6.20)
/// 
/// Allows any user to challenge a proposed result by posting a counter-bond
/// and providing evidence (IPFS CID + hash) supporting their alternative outcome.
/// 
/// Accounts:
/// 0. `[signer]` Challenger
/// 1. `[]` PredictionMarketConfig
/// 2. `[writable]` Market
/// 3. `[writable]` OracleProposal PDA
/// 4. `[writable]` OracleProposalData PDA (to record challenger's outcome)
/// 5. `[writable]` Challenger's Vault Account (for bond)
/// 6. `[writable]` Market Vault (to receive bond)
/// 7. `[]` Vault Config
/// 8. `[]` Vault Program
fn process_challenge_result_with_evidence(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: ChallengeResultWithEvidenceArgs,
) -> ProgramResult {
    msg!("ChallengeResultWithEvidence: market={}, challenger_outcome={}", 
         args.market_id, args.challenger_outcome_index);
    
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Challenger (signer)
    let challenger_info = next_account_info(account_info_iter)?;
    check_signer(challenger_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: OracleProposal PDA (writable)
    let proposal_info = next_account_info(account_info_iter)?;
    
    // Account 4: OracleProposalData PDA (writable)
    let proposal_data_info = next_account_info(account_info_iter)?;
    
    // Account 5+: Vault accounts for bond transfer (handled separately)
    let _remaining_accounts = account_info_iter;
    
    // Load and validate config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Load and validate market
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    // Market must be in ResultProposed state
    if market.status != MarketStatus::ResultProposed {
        msg!("Market must be in ResultProposed state to challenge, got {:?}", market.status);
        return Err(PredictionMarketError::InvalidMarketStatus.into());
    }
    
    let current_time = get_current_timestamp()?;
    let market_id_bytes = args.market_id.to_le_bytes();
    
    // Validate OracleProposal PDA
    let (proposal_pda, _proposal_bump) = Pubkey::find_program_address(
        &[ORACLE_PROPOSAL_SEED, &market_id_bytes],
        program_id,
    );
    
    if *proposal_info.key != proposal_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load and validate OracleProposal to check challenge window
    let proposal = deserialize_account::<OracleProposal>(&proposal_info.data.borrow())?;
    if proposal.discriminator != ORACLE_PROPOSAL_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Task 4.6.18: Verify within challenge window
    let challenge_deadline = proposal.proposed_at + config.challenge_window_secs;
    if current_time > challenge_deadline {
        msg!("Challenge window has expired: current={}, deadline={}", current_time, challenge_deadline);
        return Err(PredictionMarketError::ChallengeWindowExpired.into());
    }
    
    // Validate OracleProposalData PDA
    let (proposal_data_pda, _proposal_data_bump) = Pubkey::find_program_address(
        &[ORACLE_PROPOSAL_DATA_SEED, &market_id_bytes],
        program_id,
    );
    
    if *proposal_data_info.key != proposal_data_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load and update OracleProposalData with challenger's outcome
    let mut proposal_data = deserialize_account::<OracleProposalData>(&proposal_data_info.data.borrow())?;
    if proposal_data.discriminator != ORACLE_PROPOSAL_DATA_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Task 4.6.19: Challenger's outcome must differ from proposed outcome
    if args.challenger_outcome_index == proposal_data.proposed_outcome_index {
        msg!("Challenger outcome must differ from proposed outcome");
        return Err(PredictionMarketError::InvalidOutcome.into());
    }
    
    // Task 4.6.20: Record challenger's outcome and evidence hash
    proposal_data.set_challenger(args.challenger_outcome_index, current_time);
    
    // Update market status to Challenged
    market.status = MarketStatus::Challenged;
    market.updated_at = current_time;
    
    // Serialize updated accounts
    proposal_data.serialize(&mut &mut proposal_data_info.data.borrow_mut()[..])?;
    market.serialize(&mut &mut market_info.data.borrow_mut()[..])?;
    
    // TODO: Transfer challenger bond from challenger's vault to market vault
    // This requires CPI to Vault Program (skipped for now, handled by relayer)
    
    msg!("✅ Challenge submitted for market {}: challenger={}, outcome={}, evidence_hash={:?}", 
         args.market_id,
         challenger_info.key,
         args.challenger_outcome_index,
         &args.evidence_hash[0..8]);
    
    Ok(())
}

// ============================================================================
// V2 Multi-Outcome Order Instructions (Pure Vault Mode)
// ============================================================================

/// V2: Place order for multi-outcome market with Vault CPI
/// Similar to RelayerPlaceOrderV2 but uses outcome_index instead of Outcome enum
fn process_relayer_place_multi_outcome_order_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RelayerPlaceMultiOutcomeOrderV2Args,
) -> ProgramResult {
    use crate::state::{MultiOutcomePosition, MULTI_OUTCOME_POSITION_DISCRIMINATOR, MAX_OUTCOMES};
    
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Relayer (signer)
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if config.is_paused {
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    verify_relayer(&config, relayer_info.key)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    // Verify this is a multi-outcome market
    if market.market_type != MarketType::MultiOutcome {
        msg!("Error: RelayerPlaceMultiOutcomeOrderV2 requires MultiOutcome market type");
        return Err(PredictionMarketError::InvalidMarketType.into());
    }
    
    // Validate outcome_index
    if args.outcome_index >= market.num_outcomes {
        msg!("Error: outcome_index {} >= num_outcomes {}", args.outcome_index, market.num_outcomes);
        return Err(PredictionMarketError::InvalidOutcome.into());
    }
    
    if !market.is_tradeable() {
        return Err(PredictionMarketError::MarketNotTradeable.into());
    }
    
    // Account 3: Order PDA (writable, new)
    let order_info = next_account_info(account_info_iter)?;
    
    // Account 4: MultiOutcomePosition PDA
    let position_info = next_account_info(account_info_iter)?;
    
    // Account 5: User Vault Account
    let user_vault_info = next_account_info(account_info_iter)?;
    
    // Account 6: PM User Account
    let pm_user_info = next_account_info(account_info_iter)?;
    
    // Account 7: Vault Config
    let vault_config_info = next_account_info(account_info_iter)?;
    
    // Account 8: Vault Program
    let vault_program_info = next_account_info(account_info_iter)?;
    
    // Account 9: System Program
    let system_program_info = next_account_info(account_info_iter)?;
    
    // Derive and verify Order PDA
    let order_id = market.next_order_id;
    let market_id_bytes = args.market_id.to_le_bytes();
    let order_id_bytes = order_id.to_le_bytes();
    let (order_pda, order_bump) = Pubkey::find_program_address(
        &[ORDER_SEED, &market_id_bytes, &order_id_bytes],
        program_id,
    );
    
    if *order_info.key != order_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Calculate margin requirement (in e6 precision)
    let margin = (args.amount as u128)
        .checked_mul(args.price as u128)
        .ok_or(PredictionMarketError::ArithmeticOverflow)? as u64;
    
    let current_time = get_current_timestamp()?;
    
    // Derive Config PDA for CPI signing
    let (config_pda, config_bump) = Pubkey::find_program_address(
        &[PM_CONFIG_SEED],
        program_id,
    );
    
    if *config_info.key != config_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let config_seeds: &[&[u8]] = &[PM_CONFIG_SEED, &[config_bump]];
    
    // For Buy orders: Lock margin in Vault
    if args.side == crate::state::OrderSide::Buy {
        msg!("CPI: Lock margin {} for Buy order", margin);
        cpi_lock_for_prediction(
            vault_program_info,
            vault_config_info,
            user_vault_info,
            pm_user_info,
            config_info,
            relayer_info,
            system_program_info,
            margin,
            config_seeds,
        )?;
    } else {
        // For Sell orders: Verify MultiOutcomePosition has sufficient AVAILABLE holdings and LOCK them
        let mut position = deserialize_account::<MultiOutcomePosition>(&position_info.data.borrow())?;
        if position.discriminator != MULTI_OUTCOME_POSITION_DISCRIMINATOR {
            return Err(PredictionMarketError::InvalidAccountData.into());
        }
        
        let idx = args.outcome_index as usize;
        if idx >= MAX_OUTCOMES {
            return Err(PredictionMarketError::InvalidOutcome.into());
        }
        
        // Check available (total - locked)
        let total = position.holdings[idx];
        let locked = position.locked[idx];
        let available = total.saturating_sub(locked);
        
        if available < args.amount {
            msg!("Error: Insufficient available holdings: {} < {} (total: {}, locked: {})", 
                 available, args.amount, total, locked);
            return Err(PredictionMarketError::InsufficientPosition.into());
        }
        
        // Lock shares for this Sell order
        position.locked[idx] = position.locked[idx].saturating_add(args.amount);
        position.updated_at = current_time;
        position.serialize(&mut *position_info.data.borrow_mut())?;
        
        msg!("📊 MultiOutcome Position locked: {} shares for outcome {}", args.amount, args.outcome_index);
    }
    
    // Create Order
    let order_space = Order::SIZE;
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(order_space);
    
    // Create account via CPI
    let order_seeds: &[&[u8]] = &[ORDER_SEED, &market_id_bytes, &order_id_bytes, &[order_bump]];
    
    invoke_signed(
        &system_instruction::create_account(
            relayer_info.key,
            order_info.key,
            lamports,
            order_space as u64,
            program_id,
        ),
        &[relayer_info.clone(), order_info.clone(), system_program_info.clone()],
        &[order_seeds],
    )?;
    
    // Initialize Order - use outcome_index for multi-outcome
    // Note: We use Outcome::Yes as placeholder since Order struct uses Outcome enum
    // The actual outcome is stored in outcome_index field
    let order = Order {
        discriminator: ORDER_DISCRIMINATOR,
        order_id,
        market_id: args.market_id,
        owner: args.user_wallet,
        side: args.side,
        outcome: Outcome::Yes, // Placeholder for multi-outcome
        outcome_index: args.outcome_index,
        price: args.price,
        amount: args.amount,
        filled_amount: 0,
        status: OrderStatus::Open,
        order_type: args.order_type,
        expiration_time: args.expiration_time,
        created_at: current_time,
        updated_at: current_time,
        bump: order_bump,
        escrow_token_account: None, // V2: No SPL token escrow
        reserved: [0u8; 30],
    };
    order.serialize(&mut *order_info.data.borrow_mut())?;
    
    // Update market
    market.next_order_id = market.next_order_id.saturating_add(1);
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    msg!("✅ RelayerPlaceMultiOutcomeOrderV2 completed");
    msg!("User: {}", args.user_wallet);
    msg!("Order ID: {}, Market: {}", order_id, args.market_id);
    msg!("Side: {:?}, Outcome Index: {}", args.side, args.outcome_index);
    msg!("Price: {}, Amount: {}, Margin: {}", args.price, args.amount, margin);
    
    Ok(())
}

/// V2: Cancel order for multi-outcome market with Vault CPI
fn process_relayer_cancel_multi_outcome_order_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RelayerCancelMultiOutcomeOrderV2Args,
) -> ProgramResult {
    use crate::state::{MultiOutcomePosition, MULTI_OUTCOME_POSITION_DISCRIMINATOR, MAX_OUTCOMES};
    
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Relayer (signer)
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    verify_relayer(&config, relayer_info.key)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    // Verify this is a multi-outcome market
    if market.market_type != MarketType::MultiOutcome {
        msg!("Error: RelayerCancelMultiOutcomeOrderV2 requires MultiOutcome market type");
        return Err(PredictionMarketError::InvalidMarketType.into());
    }
    
    // Account 3: Order PDA (writable)
    let order_info = next_account_info(account_info_iter)?;
    let mut order = deserialize_account::<Order>(&order_info.data.borrow())?;
    
    if order.discriminator != ORDER_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if order.order_id != args.order_id || order.market_id != args.market_id {
        return Err(PredictionMarketError::OrderNotFound.into());
    }
    
    if order.owner != args.user_wallet {
        return Err(PredictionMarketError::OrderOwnerMismatch.into());
    }
    
    if order.status != OrderStatus::Open && order.status != OrderStatus::PartialFilled {
        return Err(PredictionMarketError::OrderNotActive.into());
    }
    
    // Account 4: MultiOutcomePosition PDA
    let position_info = next_account_info(account_info_iter)?;
    
    // Account 5: User Vault Account
    let user_vault_info = next_account_info(account_info_iter)?;
    
    // Account 6: PM User Account
    let pm_user_info = next_account_info(account_info_iter)?;
    
    // Account 7: Vault Config
    let vault_config_info = next_account_info(account_info_iter)?;
    
    // Account 8: Vault Program
    let vault_program_info = next_account_info(account_info_iter)?;
    
    // Account 9: System Program
    let _system_program_info = next_account_info(account_info_iter)?;
    
    // Calculate remaining amount and margin
    let remaining = order.amount.saturating_sub(order.filled_amount);
    let remaining_margin = (remaining as u128)
        .checked_mul(order.price as u128)
        .ok_or(PredictionMarketError::ArithmeticOverflow)? as u64;
    
    let current_time = get_current_timestamp()?;
    
    // Derive Config PDA for CPI signing
    let (config_pda, config_bump) = Pubkey::find_program_address(
        &[PM_CONFIG_SEED],
        program_id,
    );
    
    if *config_info.key != config_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let config_seeds: &[&[u8]] = &[PM_CONFIG_SEED, &[config_bump]];
    
    // For Buy orders: Unlock margin from Vault
    if order.side == crate::state::OrderSide::Buy {
        msg!("CPI: Release margin {} for cancelled Buy order", remaining_margin);
        cpi_release_from_prediction(
            vault_program_info,
            vault_config_info,
            user_vault_info,
            pm_user_info,
            config_info,
            remaining_margin,
            config_seeds,
        )?;
    } else {
        // For Sell orders: Unlock shares from MultiOutcomePosition
        let mut position = deserialize_account::<MultiOutcomePosition>(&position_info.data.borrow())?;
        if position.discriminator != MULTI_OUTCOME_POSITION_DISCRIMINATOR {
            return Err(PredictionMarketError::InvalidAccountData.into());
        }
        
        let idx = args.outcome_index as usize;
        if idx >= MAX_OUTCOMES {
            return Err(PredictionMarketError::InvalidOutcome.into());
        }
        
        // Unlock shares
        position.locked[idx] = position.locked[idx].saturating_sub(remaining);
        position.updated_at = current_time;
        position.serialize(&mut *position_info.data.borrow_mut())?;
        
        msg!("📊 MultiOutcome Position unlocked: {} shares for outcome {}", remaining, args.outcome_index);
    }
    
    // Update order status
    order.status = OrderStatus::Cancelled;
    order.updated_at = current_time;
    order.serialize(&mut *order_info.data.borrow_mut())?;
    
    // Update market
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    msg!("✅ RelayerCancelMultiOutcomeOrderV2 completed");
    msg!("User: {}", args.user_wallet);
    msg!("Order ID: {}, Market: {}", args.order_id, args.market_id);
    msg!("Remaining amount: {}, Unlocked margin/shares: {}", remaining, remaining_margin);
    
    Ok(())
}

// ============================================================================
// Multi-Outcome V2 Instructions (Vault CPI Mode)
// ============================================================================

/// V2: RelayerMintMultiOutcomeCompleteSet using Vault CPI (no SPL Token)
/// 
/// Mints a complete set of all outcome tokens for a multi-outcome market.
/// 1 complete set = 1 token of each outcome
/// Cost = amount * 1.0 USDC (locked in Vault)
fn process_relayer_mint_multi_outcome_complete_set_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RelayerMintMultiOutcomeCompleteSetArgs,
) -> ProgramResult {
    use crate::state::{MultiOutcomePosition, MULTI_OUTCOME_POSITION_DISCRIMINATOR, 
                       MULTI_OUTCOME_POSITION_SEED, MAX_OUTCOMES, MarketType};
    
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Relayer (signer)
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: MultiOutcomePosition PDA (writable)
    let position_info = next_account_info(account_info_iter)?;
    
    // Account 4: UserAccount (Vault, writable)
    let user_account_info = next_account_info(account_info_iter)?;
    
    // Account 5: PMUserAccount (Vault, writable)
    let pm_user_account_info = next_account_info(account_info_iter)?;
    
    // Account 6: VaultConfig
    let vault_config_info = next_account_info(account_info_iter)?;
    
    // Account 7: Vault Program
    let vault_program_info = next_account_info(account_info_iter)?;
    
    // Account 8: System Program
    let system_program_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    verify_relayer(&config, relayer_info.key)?;
    
    if config.is_paused {
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    // Load and validate market
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    // Verify this is a multi-outcome market
    if market.market_type != MarketType::MultiOutcome {
        msg!("❌ Expected MultiOutcome market, got {:?}", market.market_type);
        return Err(PredictionMarketError::InvalidMarketType.into());
    }
    
    if !market.is_tradeable() {
        return Err(PredictionMarketError::MarketNotTradeable.into());
    }
    
    if args.amount == 0 {
        return Err(PredictionMarketError::InvalidAmount.into());
    }
    
    let current_time = get_current_timestamp()?;
    let market_id_bytes = market.market_id.to_le_bytes();
    
    // Derive Config PDA for CPI signing
    let (config_pda, config_bump) = Pubkey::find_program_address(
        &[PM_CONFIG_SEED],
        program_id,
    );
    
    if *config_info.key != config_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let config_seeds: &[&[u8]] = &[PM_CONFIG_SEED, &[config_bump]];
    
    // Verify MultiOutcomePosition PDA
    let (position_pda, position_bump) = Pubkey::find_program_address(
        &[MULTI_OUTCOME_POSITION_SEED, &market_id_bytes, args.user_wallet.as_ref()],
        program_id,
    );
    
    if *position_info.key != position_pda {
        msg!("❌ Invalid MultiOutcomePosition PDA");
        msg!("Expected: {}, Got: {}", position_pda, position_info.key);
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Step 1: CPI to Vault - PredictionMarketLock
    msg!("CPI: Vault.PredictionMarketLock amount={}", args.amount);
    cpi_lock_for_prediction(
        vault_program_info,
        vault_config_info,
        user_account_info,
        pm_user_account_info,
        config_info,
        relayer_info,
        system_program_info,
        args.amount,
        config_seeds,
    )?;
    
    // Step 2: Create or update MultiOutcomePosition
    let is_new_position = position_info.data_is_empty();
    
    if is_new_position {
        // Create new MultiOutcomePosition account
        let rent = Rent::get()?;
        let space = MultiOutcomePosition::SIZE;
        let lamports = rent.minimum_balance(space);
        let position_seeds: &[&[u8]] = &[
            MULTI_OUTCOME_POSITION_SEED,
            &market_id_bytes,
            args.user_wallet.as_ref(),
            &[position_bump],
        ];
        
        invoke_signed(
            &system_instruction::create_account(
                relayer_info.key,
                position_info.key,
                lamports,
                space as u64,
                program_id,
            ),
            &[
                relayer_info.clone(),
                position_info.clone(),
                system_program_info.clone(),
            ],
            &[position_seeds],
        )?;
        
        // Initialize MultiOutcomePosition
        let mut position = MultiOutcomePosition::new(
            market.market_id,
            market.num_outcomes,
            args.user_wallet,
            position_bump,
            current_time,
        );
        
        // Add to all outcome holdings
        let num_outcomes = market.num_outcomes as usize;
        for i in 0..num_outcomes {
            position.holdings[i] = args.amount;
        }
        position.total_cost_e6 = args.amount;
        
        position.serialize(&mut *position_info.data.borrow_mut())?;
        msg!("✅ Created new MultiOutcomePosition");
    } else {
        // Update existing position
        let mut position = deserialize_account::<MultiOutcomePosition>(&position_info.data.borrow())?;
        
        if position.discriminator != MULTI_OUTCOME_POSITION_DISCRIMINATOR {
            return Err(PredictionMarketError::InvalidAccountData.into());
        }
        
        let num_outcomes = market.num_outcomes as usize;
        for i in 0..num_outcomes {
            position.holdings[i] = position.holdings[i].saturating_add(args.amount);
        }
        position.total_cost_e6 = position.total_cost_e6.saturating_add(args.amount);
        position.updated_at = current_time;
        
        position.serialize(&mut *position_info.data.borrow_mut())?;
        msg!("✅ Updated existing MultiOutcomePosition");
    }
    
    // Step 3: Update Market
    market.total_minted = market.total_minted.saturating_add(args.amount);
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    msg!("✅ RelayerMintMultiOutcomeCompleteSetV2 completed");
    msg!("User: {}", args.user_wallet);
    msg!("Market: {}", market.market_id);
    msg!("Amount: {}", args.amount);
    msg!("Total Minted: {}", market.total_minted);
    
    Ok(())
}

/// V2: RelayerRedeemMultiOutcomeCompleteSet using Vault CPI (no SPL Token)
/// 
/// Redeems a complete set of all outcome tokens for multi-outcome market.
/// User must have >= amount of ALL outcome tokens.
/// Returns 1 USDC per complete set.
fn process_relayer_redeem_multi_outcome_complete_set_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RelayerRedeemMultiOutcomeCompleteSetArgs,
) -> ProgramResult {
    use crate::state::{MultiOutcomePosition, MULTI_OUTCOME_POSITION_DISCRIMINATOR, 
                       MULTI_OUTCOME_POSITION_SEED, MarketType};
    
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Relayer (signer)
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: MultiOutcomePosition PDA (writable)
    let position_info = next_account_info(account_info_iter)?;
    
    // Account 4: UserAccount (Vault, writable)
    let user_account_info = next_account_info(account_info_iter)?;
    
    // Account 5: PMUserAccount (Vault, writable)
    let pm_user_account_info = next_account_info(account_info_iter)?;
    
    // Account 6: VaultConfig
    let vault_config_info = next_account_info(account_info_iter)?;
    
    // Account 7: Vault Program
    let vault_program_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    verify_relayer(&config, relayer_info.key)?;
    
    if config.is_paused {
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    // Load and validate market
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    if market.market_type != MarketType::MultiOutcome {
        msg!("❌ Expected MultiOutcome market, got {:?}", market.market_type);
        return Err(PredictionMarketError::InvalidMarketType.into());
    }
    
    if args.amount == 0 {
        return Err(PredictionMarketError::InvalidAmount.into());
    }
    
    let current_time = get_current_timestamp()?;
    let market_id_bytes = market.market_id.to_le_bytes();
    
    // Verify MultiOutcomePosition PDA
    let (position_pda, _) = Pubkey::find_program_address(
        &[MULTI_OUTCOME_POSITION_SEED, &market_id_bytes, args.user_wallet.as_ref()],
        program_id,
    );
    
    if *position_info.key != position_pda {
        msg!("❌ Invalid MultiOutcomePosition PDA");
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load position
    let mut position = deserialize_account::<MultiOutcomePosition>(&position_info.data.borrow())?;
    
    if position.discriminator != MULTI_OUTCOME_POSITION_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify user has sufficient AVAILABLE amounts of ALL outcomes
    let num_outcomes = market.num_outcomes as usize;
    for i in 0..num_outcomes {
        let available = position.holdings[i].saturating_sub(position.locked[i]);
        if available < args.amount {
            msg!("❌ Insufficient available outcome {} tokens: available {}, need {}", 
                 i, available, args.amount);
            return Err(PredictionMarketError::InsufficientPosition.into());
        }
    }
    
    // Derive Config PDA for CPI signing
    let (config_pda, config_bump) = Pubkey::find_program_address(
        &[PM_CONFIG_SEED],
        program_id,
    );
    
    if *config_info.key != config_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let config_seeds: &[&[u8]] = &[PM_CONFIG_SEED, &[config_bump]];
    
    // Step 1: Vault CPI - Unlock funds
    msg!("CPI: Vault.PredictionMarketUnlock amount={}", args.amount);
    cpi_release_from_prediction(
        vault_program_info,
        vault_config_info,
        user_account_info,
        pm_user_account_info,
        config_info,
        args.amount,
        config_seeds,
    )?;
    
    // Step 2: Update MultiOutcomePosition - reduce all holdings
    for i in 0..num_outcomes {
        position.holdings[i] = position.holdings[i].saturating_sub(args.amount);
    }
    position.total_cost_e6 = position.total_cost_e6.saturating_sub(args.amount);
    position.updated_at = current_time;
    
    position.serialize(&mut *position_info.data.borrow_mut())?;
    
    // Step 3: Update Market
    market.total_minted = market.total_minted.saturating_sub(args.amount);
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    msg!("✅ RelayerRedeemMultiOutcomeCompleteSetV2 completed");
    msg!("User: {}", args.user_wallet);
    msg!("Market: {}", market.market_id);
    msg!("Amount: {}", args.amount);
    msg!("Total Minted: {}", market.total_minted);
    
    Ok(())
}

/// V2: RelayerClaimMultiOutcomeWinnings using Vault CPI (no SPL Token)
/// 
/// Claims winnings after market resolution for multi-outcome market.
/// Settlement = amount of winning outcome tokens * 1 USDC
fn process_relayer_claim_multi_outcome_winnings_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RelayerClaimMultiOutcomeWinningsArgs,
) -> ProgramResult {
    use crate::state::{MultiOutcomePosition, MULTI_OUTCOME_POSITION_DISCRIMINATOR, 
                       MULTI_OUTCOME_POSITION_SEED, MAX_OUTCOMES, MarketType, MarketStatus};
    
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Relayer (signer)
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: MultiOutcomePosition PDA (writable)
    let position_info = next_account_info(account_info_iter)?;
    
    // Account 4: PMUserAccount (Vault, writable)
    let pm_user_account_info = next_account_info(account_info_iter)?;
    
    // Account 5: VaultConfig
    let vault_config_info = next_account_info(account_info_iter)?;
    
    // Account 6: Vault Program
    let vault_program_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    verify_relayer(&config, relayer_info.key)?;
    
    // Load and validate market
    let market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    if market.market_type != MarketType::MultiOutcome {
        return Err(PredictionMarketError::InvalidMarketType.into());
    }
    
    // Market must be Resolved or Cancelled
    if market.status != MarketStatus::Resolved && market.status != MarketStatus::Cancelled {
        msg!("❌ Market status must be Resolved or Cancelled, got {:?}", market.status);
        return Err(PredictionMarketError::MarketNotResolved.into());
    }
    
    let market_id_bytes = market.market_id.to_le_bytes();
    let current_time = get_current_timestamp()?;
    
    // Verify MultiOutcomePosition PDA
    let (position_pda, _) = Pubkey::find_program_address(
        &[MULTI_OUTCOME_POSITION_SEED, &market_id_bytes, args.user_wallet.as_ref()],
        program_id,
    );
    
    if *position_info.key != position_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load position
    let mut position = deserialize_account::<MultiOutcomePosition>(&position_info.data.borrow())?;
    
    if position.discriminator != MULTI_OUTCOME_POSITION_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if position.settled {
        msg!("Position already settled");
        return Err(PredictionMarketError::AlreadySettled.into());
    }
    
    // Calculate settlement
    let locked_amount = position.total_cost_e6;
    
    let settlement_amount = if market.status == MarketStatus::Cancelled {
        // Full refund on cancelled market
        locked_amount
    } else {
        // Get winning outcome index
        let winning_outcome_index = market.winning_outcome_index
            .ok_or(PredictionMarketError::MarketNotResolved)?;
        
        // Winning tokens pay out 1:1
        position.holdings[winning_outcome_index as usize]
    };
    
    if settlement_amount == 0 && locked_amount == 0 {
        msg!("No position to claim for user {}", args.user_wallet);
        return Err(PredictionMarketError::InvalidAmount.into());
    }
    
    // Derive Config PDA for CPI signing
    let (config_pda, config_bump) = Pubkey::find_program_address(
        &[PM_CONFIG_SEED],
        program_id,
    );
    
    if *config_info.key != config_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let config_seeds: &[&[u8]] = &[PM_CONFIG_SEED, &[config_bump]];
    
    // Step 1: Vault CPI - Settle
    msg!("CPI: Vault.PredictionMarketSettle locked={}, settlement={}", 
         locked_amount, settlement_amount);
    cpi_prediction_settle(
        vault_program_info,
        vault_config_info,
        pm_user_account_info,
        config_info,
        locked_amount,
        settlement_amount,
        config_seeds,
    )?;
    
    // Step 2: Update position
    let pnl = (settlement_amount as i64) - (locked_amount as i64);
    position.realized_pnl = position.realized_pnl.saturating_add(pnl);
    position.settlement_amount = settlement_amount;
    position.settled = true;
    // Clear all holdings
    for i in 0..MAX_OUTCOMES {
        position.holdings[i] = 0;
        position.locked[i] = 0;
    }
    position.updated_at = current_time;
    
    position.serialize(&mut *position_info.data.borrow_mut())?;
    
    msg!("✅ RelayerClaimMultiOutcomeWinningsV2 completed");
    msg!("User: {}", args.user_wallet);
    msg!("Market: {}", market.market_id);
    msg!("Settlement: {}, PnL: {}", settlement_amount, pnl);
    
    Ok(())
}

// ============================================================================
// V15.1: FinalizeResultV2 - Finalize result after challenge window
// ============================================================================

/// Process FinalizeResultV2 instruction
/// 
/// Transitions market from ResultProposed to Resolved after challenge window expires.
/// This is permissionless - anyone can call it after the deadline.
/// The proposer's bond is returned via Vault CPI.
fn process_finalize_result_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: FinalizeResultV2Args,
) -> ProgramResult {
    use crate::state::{OracleProposal, OracleProposalData, ORACLE_PROPOSAL_DISCRIMINATOR, 
                       ORACLE_PROPOSAL_SEED, ORACLE_PROPOSAL_DATA_DISCRIMINATOR,
                       ORACLE_PROPOSAL_DATA_SEED, MarketStatus, ProposalStatus};
    
    msg!("FinalizeResultV2: market={}", args.market_id);
    
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Caller (signer) - permissionless
    let caller_info = next_account_info(account_info_iter)?;
    check_signer(caller_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: OracleProposal PDA (writable)
    let proposal_info = next_account_info(account_info_iter)?;
    
    // Account 4: OracleProposalData PDA
    let proposal_data_info = next_account_info(account_info_iter)?;
    
    // Account 5: Proposer's PMUserAccount (Vault, writable) - for bond return
    let proposer_pm_account_info = next_account_info(account_info_iter)?;
    
    // Account 6: VaultConfig
    let vault_config_info = next_account_info(account_info_iter)?;
    
    // Account 7: Vault Program
    let vault_program_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    let config_bump = config.bump;
    
    // Load and validate market
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    // Market must be in ResultProposed state
    if market.status != MarketStatus::ResultProposed {
        msg!("❌ Market must be in ResultProposed state, got {:?}", market.status);
        return Err(PredictionMarketError::InvalidMarketStatus.into());
    }
    
    let market_id_bytes = args.market_id.to_le_bytes();
    let current_time = get_current_timestamp()?;
    
    // Verify OracleProposal PDA
    let (proposal_pda, _) = Pubkey::find_program_address(
        &[ORACLE_PROPOSAL_SEED, &market_id_bytes],
        program_id,
    );
    
    if *proposal_info.key != proposal_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load and validate proposal
    let mut proposal = deserialize_account::<OracleProposal>(&proposal_info.data.borrow())?;
    if proposal.discriminator != ORACLE_PROPOSAL_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify OracleProposalData PDA
    let (proposal_data_pda, _) = Pubkey::find_program_address(
        &[ORACLE_PROPOSAL_DATA_SEED, &market_id_bytes],
        program_id,
    );
    
    if *proposal_data_info.key != proposal_data_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load proposal data
    let proposal_data = deserialize_account::<OracleProposalData>(&proposal_data_info.data.borrow())?;
    if proposal_data.discriminator != ORACLE_PROPOSAL_DATA_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Check if challenge window has expired (use proposal.challenge_deadline)
    if current_time < proposal.challenge_deadline {
        msg!("❌ Challenge window has not expired yet: current={}, deadline={}", 
             current_time, proposal.challenge_deadline);
        return Err(PredictionMarketError::ChallengeWindowNotExpired.into());
    }
    
    // Proposal must not be disputed (check status)
    if proposal.status == ProposalStatus::Disputed {
        msg!("❌ Cannot finalize: proposal has been disputed");
        return Err(PredictionMarketError::OracleDisputeInProgress.into());
    }
    
    // Proposal must be in Pending status
    if proposal.status != ProposalStatus::Pending {
        msg!("❌ Proposal is not in Pending status, got {:?}", proposal.status);
        return Err(PredictionMarketError::CannotFinalize.into());
    }
    
    // Return proposer's bond via Vault CPI
    // Bond was locked when proposal was created, now we release it
    let bond_amount = proposal.bond_amount;
    
    if bond_amount > 0 {
        msg!("📤 Returning proposer bond: {} e6", bond_amount);
        
        let config_seeds = &[
            PM_CONFIG_SEED,
            &[config_bump],
        ];
        
        // Use settlement with locked=bond, settlement=bond (full return)
        cpi_prediction_settle(
            vault_program_info,
            vault_config_info,
            proposer_pm_account_info,
            config_info,
            bond_amount,  // locked_amount = bond
            bond_amount,  // settlement_amount = bond (full return, no loss)
            config_seeds,
        )?;
    }
    
    // Update market to Resolved
    market.status = MarketStatus::Resolved;
    market.final_result = Some(proposal.proposed_result);
    market.winning_outcome_index = Some(proposal_data.proposed_outcome_index);
    market.updated_at = current_time;
    
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    // Update proposal status to Finalized
    proposal.status = ProposalStatus::Finalized;
    
    proposal.serialize(&mut *proposal_info.data.borrow_mut())?;
    
    msg!("✅ FinalizeResultV2 completed");
    msg!("Market {} resolved with result {:?}, outcome index {}", 
         market.market_id, market.final_result, proposal_data.proposed_outcome_index);
    msg!("Bond returned: {} e6", bond_amount);
    
    Ok(())
}

// ============================================================================
// V15.2: RelayerChallengeResultV2 - Relayer-signed challenge for Public API
// ============================================================================

/// Process RelayerChallengeResultV2 instruction
/// 
/// Allows relayer to submit a challenge on behalf of a user.
/// The challenger's bond is deducted from their Vault account via CPI.
/// This enables Public API to submit challenges without requiring user signature.
/// 
/// Accounts:
/// 0. `[signer]` Relayer
/// 1. `[]` PredictionMarketConfig
/// 2. `[writable]` Market
/// 3. `[writable]` OracleProposal PDA
/// 4. `[writable]` OracleProposalData PDA
/// 5. `[writable]` Challenger's UserAccount (Vault)
/// 6. `[writable]` Challenger's PMUserAccount (Vault) - for bond deduction
/// 7. `[]` VaultConfig
/// 8. `[]` Vault Program
/// 9. `[]` System Program
fn process_relayer_challenge_result_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: crate::instruction::RelayerChallengeResultV2Args,
) -> ProgramResult {
    use crate::state::{OracleProposal, OracleProposalData, ORACLE_PROPOSAL_DISCRIMINATOR, 
                       ORACLE_PROPOSAL_SEED, ORACLE_PROPOSAL_DATA_DISCRIMINATOR,
                       ORACLE_PROPOSAL_DATA_SEED, MarketStatus};
    
    msg!("RelayerChallengeResultV2: market={}, challenger={}, outcome={}", 
         args.market_id, args.user_wallet, args.challenger_outcome_index);
    
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Relayer (signer)
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if config.is_paused {
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    // Verify relayer is authorized
    verify_relayer(&config, relayer_info.key)?;
    
    let config_bump = config.bump;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    // Market must be in ResultProposed state
    if market.status != MarketStatus::ResultProposed {
        msg!("Market must be in ResultProposed state to challenge, got {:?}", market.status);
        return Err(PredictionMarketError::InvalidMarketStatus.into());
    }
    
    let current_time = get_current_timestamp()?;
    let market_id_bytes = args.market_id.to_le_bytes();
    
    // Account 3: OracleProposal PDA (writable)
    let proposal_info = next_account_info(account_info_iter)?;
    
    // Validate OracleProposal PDA
    let (proposal_pda, _proposal_bump) = Pubkey::find_program_address(
        &[ORACLE_PROPOSAL_SEED, &market_id_bytes],
        program_id,
    );
    
    if *proposal_info.key != proposal_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load and validate OracleProposal to check challenge window
    let proposal = deserialize_account::<OracleProposal>(&proposal_info.data.borrow())?;
    if proposal.discriminator != ORACLE_PROPOSAL_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify within challenge window
    let challenge_deadline = proposal.proposed_at + config.challenge_window_secs;
    if current_time > challenge_deadline {
        msg!("Challenge window has expired: current={}, deadline={}", current_time, challenge_deadline);
        return Err(PredictionMarketError::ChallengeWindowExpired.into());
    }
    
    // Account 4: OracleProposalData PDA (writable)
    let proposal_data_info = next_account_info(account_info_iter)?;
    
    // Validate OracleProposalData PDA
    let (proposal_data_pda, _proposal_data_bump) = Pubkey::find_program_address(
        &[ORACLE_PROPOSAL_DATA_SEED, &market_id_bytes],
        program_id,
    );
    
    if *proposal_data_info.key != proposal_data_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load and update OracleProposalData with challenger's outcome
    let mut proposal_data = deserialize_account::<OracleProposalData>(&proposal_data_info.data.borrow())?;
    if proposal_data.discriminator != ORACLE_PROPOSAL_DATA_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Challenger's outcome must differ from proposed outcome
    if args.challenger_outcome_index == proposal_data.proposed_outcome_index {
        msg!("Challenger outcome must differ from proposed outcome");
        return Err(PredictionMarketError::InvalidOutcome.into());
    }
    
    // Account 5: Challenger's UserAccount (Vault) - for bond lock
    let challenger_vault_info = next_account_info(account_info_iter)?;
    
    // Account 6: Challenger's PMUserAccount (Vault)
    let challenger_pm_account_info = next_account_info(account_info_iter)?;
    
    // Account 7: VaultConfig
    let vault_config_info = next_account_info(account_info_iter)?;
    
    // Account 8: Vault Program
    let vault_program_info = next_account_info(account_info_iter)?;
    
    // Account 9: System Program (for auto-init)
    let system_program_info = next_account_info(account_info_iter)?;
    
    // Lock challenger's bond via Vault CPI
    // Use the same bond amount as the proposer
    let bond_amount = config.proposer_bond_e6;
    
    if bond_amount > 0 {
        msg!("📥 Locking challenger bond: {} e6 for user {}", bond_amount, args.user_wallet);
        
        let config_seeds = &[
            PM_CONFIG_SEED,
            &[config_bump],
        ];
        
        // Lock the bond from challenger's vault
        cpi_lock_for_prediction(
            vault_program_info,
            vault_config_info,
            challenger_vault_info,
            challenger_pm_account_info,
            config_info,
            relayer_info,  // payer for auto-init
            system_program_info,
            bond_amount,
            config_seeds,
        )?;
    }
    
    // Record challenger's outcome and evidence hash
    proposal_data.set_challenger(args.challenger_outcome_index, current_time);
    
    // Store evidence hash (stored in proposal_data for reference)
    // Note: evidence_hash is stored off-chain, we just log it here
    msg!("Challenge evidence_hash: {:?}", &args.evidence_hash[0..8]);
    
    // Update market status to Challenged
    market.status = MarketStatus::Challenged;
    market.updated_at = current_time;
    
    // Serialize updated accounts
    proposal_data.serialize(&mut &mut proposal_data_info.data.borrow_mut()[..])?;
    market.serialize(&mut &mut market_info.data.borrow_mut()[..])?;
    
    msg!("✅ RelayerChallengeResultV2 completed");
    msg!("Market {} challenged by {} (via relayer), outcome={}", 
         args.market_id, args.user_wallet, args.challenger_outcome_index);
    msg!("Bond locked: {} e6", bond_amount);
    
    Ok(())
}