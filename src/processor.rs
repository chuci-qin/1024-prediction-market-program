//! Instruction processor for the Prediction Market Program

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::error::PredictionMarketError;
use crate::instruction::PredictionMarketInstruction;
use crate::state::{
    PredictionMarketConfig, Market, Order, Position, OracleProposal,
    MarketStatus, MarketResult, ReviewStatus, OrderStatus, ProposalStatus,
    PM_CONFIG_SEED, MARKET_SEED, ORDER_SEED, POSITION_SEED, 
    MARKET_VAULT_SEED, YES_MINT_SEED, NO_MINT_SEED, ORACLE_PROPOSAL_SEED,
    PM_CONFIG_DISCRIMINATOR, MARKET_DISCRIMINATOR, ORDER_DISCRIMINATOR, 
    POSITION_DISCRIMINATOR, ORACLE_PROPOSAL_DISCRIMINATOR,
    PRICE_PRECISION,
};
use crate::utils::{
    check_signer, verify_pda, get_current_timestamp, create_pda_account,
    safe_add_i64, safe_sub_i64, safe_mul_u64, safe_div_u64,
    calculate_fee, validate_price, validate_price_pair,
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
        PredictionMarketInstruction::MatchMint(args) => {
            msg!("Instruction: MatchMint");
            process_match_mint(program_id, accounts, args)
        }
        PredictionMarketInstruction::MatchBurn(args) => {
            msg!("Instruction: MatchBurn");
            process_match_burn(program_id, accounts, args)
        }
        PredictionMarketInstruction::ExecuteTrade(args) => {
            msg!("Instruction: ExecuteTrade");
            process_execute_trade(program_id, accounts, args)
        }
        
        // === Oracle / Resolution ===
        PredictionMarketInstruction::ProposeResult(args) => {
            msg!("Instruction: ProposeResult");
            process_propose_result(program_id, accounts, args)
        }
        PredictionMarketInstruction::ChallengeResult(args) => {
            msg!("Instruction: ChallengeResult");
            process_challenge_result(program_id, accounts, args)
        }
        PredictionMarketInstruction::FinalizeResult => {
            msg!("Instruction: FinalizeResult");
            process_finalize_result(program_id, accounts)
        }
        PredictionMarketInstruction::ResolveDispute(args) => {
            msg!("Instruction: ResolveDispute");
            process_resolve_dispute(program_id, accounts, args)
        }
        
        // === Settlement ===
        PredictionMarketInstruction::ClaimWinnings => {
            msg!("Instruction: ClaimWinnings");
            process_claim_winnings(program_id, accounts)
        }
        PredictionMarketInstruction::RefundCancelledMarket => {
            msg!("Instruction: RefundCancelledMarket");
            process_refund_cancelled_market(program_id, accounts)
        }
        
        // === Admin Operations ===
        PredictionMarketInstruction::UpdateAdmin(args) => {
            msg!("Instruction: UpdateAdmin");
            process_update_admin(program_id, accounts, args)
        }
        PredictionMarketInstruction::UpdateOracleAdmin(args) => {
            msg!("Instruction: UpdateOracleAdmin");
            process_update_oracle_admin(program_id, accounts, args)
        }
        PredictionMarketInstruction::SetPaused(args) => {
            msg!("Instruction: SetPaused");
            process_set_paused(program_id, accounts, args)
        }
        PredictionMarketInstruction::UpdateOracleConfig(args) => {
            msg!("Instruction: UpdateOracleConfig");
            process_update_oracle_config(program_id, accounts, args)
        }
        PredictionMarketInstruction::AddAuthorizedCaller(args) => {
            msg!("Instruction: AddAuthorizedCaller");
            process_add_authorized_caller(program_id, accounts, args)
        }
        PredictionMarketInstruction::RemoveAuthorizedCaller(args) => {
            msg!("Instruction: RemoveAuthorizedCaller");
            process_remove_authorized_caller(program_id, accounts, args)
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
    let mut config = PredictionMarketConfig::try_from_slice(&config_info.data.borrow())?;
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
        created_at: current_time,
        updated_at: current_time,
        total_minted: 0,
        total_volume_e6: 0,
        open_interest: 0,
        creator_fee_bps: args.creator_fee_bps,
        next_order_id: 1,
        bump: market_bump,
        reserved: [0u8; 64],
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
    let mut config = PredictionMarketConfig::try_from_slice(&config_info.data.borrow())?;
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
    let mut market = Market::try_from_slice(&market_info.data.borrow())?;
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
    let mut config = PredictionMarketConfig::try_from_slice(&config_info.data.borrow())?;
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
    let mut market = Market::try_from_slice(&market_info.data.borrow())?;
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
    let mut config = PredictionMarketConfig::try_from_slice(&config_info.data.borrow())?;
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
    let mut market = Market::try_from_slice(&market_info.data.borrow())?;
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
    let mut config = PredictionMarketConfig::try_from_slice(&config_info.data.borrow())?;
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
    let mut market = Market::try_from_slice(&market_info.data.borrow())?;
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
    market.review_status = args.reason;
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    // Update config if was active
    if was_active {
        config.active_markets = config.active_markets.saturating_sub(1);
        config.serialize(&mut *config_info.data.borrow_mut())?;
    }
    
    msg!("Market {} cancelled successfully. Reason: {:?}", args.market_id, args.reason);
    
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
    let config = PredictionMarketConfig::try_from_slice(&config_info.data.borrow())?;
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
    let mut market = Market::try_from_slice(&market_info.data.borrow())?;
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
    let mut config = PredictionMarketConfig::try_from_slice(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if config.is_paused {
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    // Load and validate market
    let mut market = Market::try_from_slice(&market_info.data.borrow())?;
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
    
    if position_info.data_is_empty() {
        // Create new position
        let rent = Rent::get()?;
        let space = Position::SIZE;
        let lamports = rent.minimum_balance(space);
        let position_seeds: &[&[u8]] = &[POSITION_SEED, &market_id_bytes, user_info.key.as_ref(), &[position_bump]];
        
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
        
        let position = Position::new(market.market_id, *user_info.key, position_bump, current_time);
        position.serialize(&mut *position_info.data.borrow_mut())?;
    }
    
    // Update position
    let mut position = Position::try_from_slice(&position_info.data.borrow())?;
    
    // For complete set, cost is at $0.50 each (1 USDC total for YES + NO)
    let half_price = PRICE_PRECISION / 2; // 500_000
    position.add_tokens(crate::state::Outcome::Yes, args.amount, half_price, current_time);
    position.add_tokens(crate::state::Outcome::No, args.amount, half_price, current_time);
    
    position.serialize(&mut *position_info.data.borrow_mut())?;
    
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
    let config = PredictionMarketConfig::try_from_slice(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if config.is_paused {
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    // Load and validate market
    let mut market = Market::try_from_slice(&market_info.data.borrow())?;
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
    let mut position = Position::try_from_slice(&position_info.data.borrow())?;
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
    msg!("Amount: {} (YES + NO)", args.amount);
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
    
    // Load and validate config
    let config = PredictionMarketConfig::try_from_slice(&config_info.data.borrow())?;
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
    let mut market = Market::try_from_slice(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if !market.is_tradeable() {
        msg!("Error: Market is not tradeable");
        return Err(PredictionMarketError::MarketNotTradeable.into());
    }
    
    // Validate order parameters
    if !validate_price(args.price) {
        msg!("Error: Price must be between 0.01 and 0.99");
        return Err(PredictionMarketError::InvalidPrice.into());
    }
    
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
    
    // Initialize order
    let order = Order {
        discriminator: ORDER_DISCRIMINATOR,
        order_id,
        market_id: args.market_id,
        owner: *user_info.key,
        side: args.side,
        outcome: args.outcome,
        price: args.price,
        amount: args.amount,
        filled_amount: 0,
        status: OrderStatus::Open,
        order_type: args.order_type,
        expiration_time: args.expiration_time,
        created_at: current_time,
        updated_at: current_time,
        bump: order_bump,
        reserved: [0u8; 32],
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
    let (order_pda, _) = Pubkey::find_program_address(
        &[ORDER_SEED, &market_id_bytes, &order_id_bytes],
        program_id,
    );
    if *order_info.key != order_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load order
    let mut order = Order::try_from_slice(&order_info.data.borrow())?;
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
    
    // Cancel order
    order.status = OrderStatus::Cancelled;
    order.updated_at = current_time;
    order.serialize(&mut *order_info.data.borrow_mut())?;
    
    msg!("Order cancelled successfully");
    msg!("Order ID: {}", args.order_id);
    msg!("Market ID: {}", args.market_id);
    msg!("Unfilled amount: {}", order.remaining_amount());
    
    Ok(())
}

fn process_match_mint(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: MatchMintArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Relayer/Keeper (signer)
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    // Account 1: PredictionMarketConfig (writable)
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: YES Order (writable)
    let yes_order_info = next_account_info(account_info_iter)?;
    
    // Account 4: NO Order (writable)
    let no_order_info = next_account_info(account_info_iter)?;
    
    // Account 5: YES Token Mint (writable)
    let yes_mint_info = next_account_info(account_info_iter)?;
    
    // Account 6: NO Token Mint (writable)
    let no_mint_info = next_account_info(account_info_iter)?;
    
    // Account 7: YES Buyer's YES Token Account (writable)
    let yes_buyer_token_info = next_account_info(account_info_iter)?;
    
    // Account 8: NO Buyer's NO Token Account (writable)
    let no_buyer_token_info = next_account_info(account_info_iter)?;
    
    // Account 9: Token Program
    let token_program_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let mut config = PredictionMarketConfig::try_from_slice(&config_info.data.borrow())?;
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
    
    // Load market
    let mut market = Market::try_from_slice(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if !market.is_tradeable() {
        return Err(PredictionMarketError::MarketNotTradeable.into());
    }
    
    // Verify mints
    if *yes_mint_info.key != market.yes_mint {
        return Err(PredictionMarketError::InvalidYesMint.into());
    }
    if *no_mint_info.key != market.no_mint {
        return Err(PredictionMarketError::InvalidNoMint.into());
    }
    
    // Verify Order PDAs
    let yes_order_id_bytes = args.yes_order_id.to_le_bytes();
    let (yes_order_pda, _) = Pubkey::find_program_address(
        &[ORDER_SEED, &market_id_bytes, &yes_order_id_bytes],
        program_id,
    );
    if *yes_order_info.key != yes_order_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let no_order_id_bytes = args.no_order_id.to_le_bytes();
    let (no_order_pda, _) = Pubkey::find_program_address(
        &[ORDER_SEED, &market_id_bytes, &no_order_id_bytes],
        program_id,
    );
    if *no_order_info.key != no_order_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load orders
    let mut yes_order = Order::try_from_slice(&yes_order_info.data.borrow())?;
    let mut no_order = Order::try_from_slice(&no_order_info.data.borrow())?;
    
    // Validate orders
    if yes_order.discriminator != ORDER_DISCRIMINATOR || no_order.discriminator != ORDER_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify both are Buy orders
    if yes_order.side != crate::state::OrderSide::Buy || no_order.side != crate::state::OrderSide::Buy {
        msg!("Error: Both orders must be Buy orders for MatchMint");
        return Err(PredictionMarketError::InvalidOrderSide.into());
    }
    
    // Verify outcomes
    if yes_order.outcome != crate::state::Outcome::Yes {
        msg!("Error: First order must be for YES outcome");
        return Err(PredictionMarketError::InvalidOutcome.into());
    }
    if no_order.outcome != crate::state::Outcome::No {
        msg!("Error: Second order must be for NO outcome");
        return Err(PredictionMarketError::InvalidOutcome.into());
    }
    
    // Verify orders are active
    if !yes_order.is_active() || !no_order.is_active() {
        return Err(PredictionMarketError::OrderNotActive.into());
    }
    
    // Validate prices are complementary (should sum to >= 1.0)
    if !validate_price_pair(args.yes_price, args.no_price) {
        msg!("Error: YES price + NO price must equal 1.0 for minting");
        return Err(PredictionMarketError::InvalidPricePair.into());
    }
    
    // Verify prices match or are better than limit prices
    if args.yes_price > yes_order.price {
        msg!("Error: Match price exceeds YES order limit price");
        return Err(PredictionMarketError::PriceExceedsLimit.into());
    }
    if args.no_price > no_order.price {
        msg!("Error: Match price exceeds NO order limit price");
        return Err(PredictionMarketError::PriceExceedsLimit.into());
    }
    
    // Calculate matchable amount
    let yes_remaining = yes_order.remaining_amount();
    let no_remaining = no_order.remaining_amount();
    let match_amount = args.amount.min(yes_remaining).min(no_remaining);
    
    if match_amount == 0 {
        msg!("Error: No matchable amount");
        return Err(PredictionMarketError::NoMatchableAmount.into());
    }
    
    let current_time = get_current_timestamp()?;
    
    // Calculate market PDA seeds for signing
    let market_seeds: &[&[u8]] = &[MARKET_SEED, &market_id_bytes, &[market.bump]];
    
    // Mint YES tokens to YES buyer
    invoke_signed(
        &spl_token::instruction::mint_to(
            token_program_info.key,
            yes_mint_info.key,
            yes_buyer_token_info.key,
            market_info.key,
            &[],
            match_amount,
        )?,
        &[yes_mint_info.clone(), yes_buyer_token_info.clone(), market_info.clone(), token_program_info.clone()],
        &[market_seeds],
    )?;
    
    // Mint NO tokens to NO buyer
    invoke_signed(
        &spl_token::instruction::mint_to(
            token_program_info.key,
            no_mint_info.key,
            no_buyer_token_info.key,
            market_info.key,
            &[],
            match_amount,
        )?,
        &[no_mint_info.clone(), no_buyer_token_info.clone(), market_info.clone(), token_program_info.clone()],
        &[market_seeds],
    )?;
    
    // Update orders
    yes_order.filled_amount += match_amount;
    if yes_order.filled_amount >= yes_order.amount {
        yes_order.status = OrderStatus::Filled;
    } else {
        yes_order.status = OrderStatus::PartialFilled;
    }
    yes_order.updated_at = current_time;
    yes_order.serialize(&mut *yes_order_info.data.borrow_mut())?;
    
    no_order.filled_amount += match_amount;
    if no_order.filled_amount >= no_order.amount {
        no_order.status = OrderStatus::Filled;
    } else {
        no_order.status = OrderStatus::PartialFilled;
    }
    no_order.updated_at = current_time;
    no_order.serialize(&mut *no_order_info.data.borrow_mut())?;
    
    // Calculate trade volume (in USDC terms)
    let trade_volume = match_amount; // 1 complete set = 1 USDC
    
    // Update market stats
    market.total_minted += match_amount;
    market.total_volume_e6 += trade_volume as i64;
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    // Update config stats
    config.total_volume_e6 += trade_volume as i64;
    config.total_minted_sets += match_amount;
    config.serialize(&mut *config_info.data.borrow_mut())?;
    
    msg!("MatchMint executed successfully");
    msg!("Match amount: {}", match_amount);
    msg!("YES price: {} (e6), NO price: {} (e6)", args.yes_price, args.no_price);
    msg!("YES order {} filled: {}/{}", args.yes_order_id, yes_order.filled_amount, yes_order.amount);
    msg!("NO order {} filled: {}/{}", args.no_order_id, no_order.filled_amount, no_order.amount);
    
    Ok(())
}

fn process_match_burn(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: MatchBurnArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Relayer/Keeper (signer)
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    // Account 1: PredictionMarketConfig (writable)
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: YES Order (writable)
    let yes_order_info = next_account_info(account_info_iter)?;
    
    // Account 4: NO Order (writable)
    let no_order_info = next_account_info(account_info_iter)?;
    
    // Account 5: YES Token Mint (writable)
    let yes_mint_info = next_account_info(account_info_iter)?;
    
    // Account 6: NO Token Mint (writable)
    let no_mint_info = next_account_info(account_info_iter)?;
    
    // Account 7: YES Seller's YES Token Account (writable)
    let yes_seller_token_info = next_account_info(account_info_iter)?;
    
    // Account 8: NO Seller's NO Token Account (writable)
    let no_seller_token_info = next_account_info(account_info_iter)?;
    
    // Account 9: Market Vault (writable)
    let market_vault_info = next_account_info(account_info_iter)?;
    
    // Account 10: YES Seller's USDC Account (writable)
    let yes_seller_usdc_info = next_account_info(account_info_iter)?;
    
    // Account 11: NO Seller's USDC Account (writable)
    let no_seller_usdc_info = next_account_info(account_info_iter)?;
    
    // Account 12: Token Program
    let token_program_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let mut config = PredictionMarketConfig::try_from_slice(&config_info.data.borrow())?;
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
    
    // Load market
    let mut market = Market::try_from_slice(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if !market.is_tradeable() {
        return Err(PredictionMarketError::MarketNotTradeable.into());
    }
    
    // Verify addresses
    if *yes_mint_info.key != market.yes_mint {
        return Err(PredictionMarketError::InvalidYesMint.into());
    }
    if *no_mint_info.key != market.no_mint {
        return Err(PredictionMarketError::InvalidNoMint.into());
    }
    if *market_vault_info.key != market.market_vault {
        return Err(PredictionMarketError::InvalidMarketVault.into());
    }
    
    // Verify Order PDAs
    let yes_order_id_bytes = args.yes_order_id.to_le_bytes();
    let (yes_order_pda, _) = Pubkey::find_program_address(
        &[ORDER_SEED, &market_id_bytes, &yes_order_id_bytes],
        program_id,
    );
    if *yes_order_info.key != yes_order_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let no_order_id_bytes = args.no_order_id.to_le_bytes();
    let (no_order_pda, _) = Pubkey::find_program_address(
        &[ORDER_SEED, &market_id_bytes, &no_order_id_bytes],
        program_id,
    );
    if *no_order_info.key != no_order_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load orders
    let mut yes_order = Order::try_from_slice(&yes_order_info.data.borrow())?;
    let mut no_order = Order::try_from_slice(&no_order_info.data.borrow())?;
    
    // Validate orders
    if yes_order.discriminator != ORDER_DISCRIMINATOR || no_order.discriminator != ORDER_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify both are Sell orders
    if yes_order.side != crate::state::OrderSide::Sell || no_order.side != crate::state::OrderSide::Sell {
        msg!("Error: Both orders must be Sell orders for MatchBurn");
        return Err(PredictionMarketError::InvalidOrderSide.into());
    }
    
    // Verify outcomes
    if yes_order.outcome != crate::state::Outcome::Yes {
        return Err(PredictionMarketError::InvalidOutcome.into());
    }
    if no_order.outcome != crate::state::Outcome::No {
        return Err(PredictionMarketError::InvalidOutcome.into());
    }
    
    // Verify orders are active
    if !yes_order.is_active() || !no_order.is_active() {
        return Err(PredictionMarketError::OrderNotActive.into());
    }
    
    // Validate prices are complementary (should sum to <= 1.0 for selling)
    let total_price = args.yes_price + args.no_price;
    if total_price > PRICE_PRECISION {
        msg!("Error: YES price + NO price cannot exceed 1.0 for burning");
        return Err(PredictionMarketError::InvalidPricePair.into());
    }
    
    // Verify prices match or are better than limit prices (for sell, higher is better)
    if args.yes_price < yes_order.price {
        msg!("Error: Match price below YES order limit price");
        return Err(PredictionMarketError::PriceBelowLimit.into());
    }
    if args.no_price < no_order.price {
        msg!("Error: Match price below NO order limit price");
        return Err(PredictionMarketError::PriceBelowLimit.into());
    }
    
    // Calculate matchable amount
    let yes_remaining = yes_order.remaining_amount();
    let no_remaining = no_order.remaining_amount();
    let match_amount = args.amount.min(yes_remaining).min(no_remaining);
    
    if match_amount == 0 {
        return Err(PredictionMarketError::NoMatchableAmount.into());
    }
    
    let current_time = get_current_timestamp()?;
    
    // Calculate market PDA seeds for signing
    let market_seeds: &[&[u8]] = &[MARKET_SEED, &market_id_bytes, &[market.bump]];
    
    // Burn YES tokens from YES seller (requires seller signature - simplified here)
    // Note: In production, tokens should be transferred to escrow first when order is placed
    invoke_signed(
        &spl_token::instruction::burn(
            token_program_info.key,
            yes_seller_token_info.key,
            yes_mint_info.key,
            market_info.key, // We assume tokens were escrowed to market
            &[],
            match_amount,
        )?,
        &[yes_seller_token_info.clone(), yes_mint_info.clone(), market_info.clone(), token_program_info.clone()],
        &[market_seeds],
    )?;
    
    // Burn NO tokens from NO seller
    invoke_signed(
        &spl_token::instruction::burn(
            token_program_info.key,
            no_seller_token_info.key,
            no_mint_info.key,
            market_info.key,
            &[],
            match_amount,
        )?,
        &[no_seller_token_info.clone(), no_mint_info.clone(), market_info.clone(), token_program_info.clone()],
        &[market_seeds],
    )?;
    
    // Calculate proceeds for each seller
    let yes_proceeds = ((match_amount as u128) * (args.yes_price as u128) / (PRICE_PRECISION as u128)) as u64;
    let no_proceeds = ((match_amount as u128) * (args.no_price as u128) / (PRICE_PRECISION as u128)) as u64;
    
    // Transfer USDC from vault to YES seller
    invoke_signed(
        &spl_token::instruction::transfer(
            token_program_info.key,
            market_vault_info.key,
            yes_seller_usdc_info.key,
            market_info.key,
            &[],
            yes_proceeds,
        )?,
        &[market_vault_info.clone(), yes_seller_usdc_info.clone(), market_info.clone(), token_program_info.clone()],
        &[market_seeds],
    )?;
    
    // Transfer USDC from vault to NO seller
    invoke_signed(
        &spl_token::instruction::transfer(
            token_program_info.key,
            market_vault_info.key,
            no_seller_usdc_info.key,
            market_info.key,
            &[],
            no_proceeds,
        )?,
        &[market_vault_info.clone(), no_seller_usdc_info.clone(), market_info.clone(), token_program_info.clone()],
        &[market_seeds],
    )?;
    
    // Update orders
    yes_order.filled_amount += match_amount;
    if yes_order.filled_amount >= yes_order.amount {
        yes_order.status = OrderStatus::Filled;
    } else {
        yes_order.status = OrderStatus::PartialFilled;
    }
    yes_order.updated_at = current_time;
    yes_order.serialize(&mut *yes_order_info.data.borrow_mut())?;
    
    no_order.filled_amount += match_amount;
    if no_order.filled_amount >= no_order.amount {
        no_order.status = OrderStatus::Filled;
    } else {
        no_order.status = OrderStatus::PartialFilled;
    }
    no_order.updated_at = current_time;
    no_order.serialize(&mut *no_order_info.data.borrow_mut())?;
    
    // Update market stats
    market.total_minted = market.total_minted.saturating_sub(match_amount);
    market.total_volume_e6 += match_amount as i64;
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    // Update config stats
    config.total_volume_e6 += match_amount as i64;
    config.serialize(&mut *config_info.data.borrow_mut())?;
    
    msg!("MatchBurn executed successfully");
    msg!("Match amount: {}", match_amount);
    msg!("YES seller proceeds: {} USDC (e6)", yes_proceeds);
    msg!("NO seller proceeds: {} USDC (e6)", no_proceeds);
    
    Ok(())
}

fn process_execute_trade(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: ExecuteTradeArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Relayer/Keeper (signer)
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    // Account 1: PredictionMarketConfig (writable)
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: Buy Order (writable)
    let buy_order_info = next_account_info(account_info_iter)?;
    
    // Account 4: Sell Order (writable)
    let sell_order_info = next_account_info(account_info_iter)?;
    
    // Account 5: Seller's Token Account (writable)
    let seller_token_info = next_account_info(account_info_iter)?;
    
    // Account 6: Buyer's Token Account (writable)
    let buyer_token_info = next_account_info(account_info_iter)?;
    
    // Account 7: Token Program
    let token_program_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let mut config = PredictionMarketConfig::try_from_slice(&config_info.data.borrow())?;
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
    
    // Load market
    let mut market = Market::try_from_slice(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if !market.is_tradeable() {
        return Err(PredictionMarketError::MarketNotTradeable.into());
    }
    
    // Verify Order PDAs
    let buy_order_id_bytes = args.buy_order_id.to_le_bytes();
    let (buy_order_pda, _) = Pubkey::find_program_address(
        &[ORDER_SEED, &market_id_bytes, &buy_order_id_bytes],
        program_id,
    );
    if *buy_order_info.key != buy_order_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let sell_order_id_bytes = args.sell_order_id.to_le_bytes();
    let (sell_order_pda, _) = Pubkey::find_program_address(
        &[ORDER_SEED, &market_id_bytes, &sell_order_id_bytes],
        program_id,
    );
    if *sell_order_info.key != sell_order_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load orders
    let mut buy_order = Order::try_from_slice(&buy_order_info.data.borrow())?;
    let mut sell_order = Order::try_from_slice(&sell_order_info.data.borrow())?;
    
    // Validate orders
    if buy_order.discriminator != ORDER_DISCRIMINATOR || sell_order.discriminator != ORDER_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify order sides
    if buy_order.side != crate::state::OrderSide::Buy {
        return Err(PredictionMarketError::InvalidOrderSide.into());
    }
    if sell_order.side != crate::state::OrderSide::Sell {
        return Err(PredictionMarketError::InvalidOrderSide.into());
    }
    
    // Verify same outcome
    if buy_order.outcome != sell_order.outcome {
        msg!("Error: Orders must be for the same outcome");
        return Err(PredictionMarketError::OutcomeMismatch.into());
    }
    
    // Verify orders are active
    if !buy_order.is_active() || !sell_order.is_active() {
        return Err(PredictionMarketError::OrderNotActive.into());
    }
    
    // Verify price compatibility (buy price >= sell price)
    if buy_order.price < sell_order.price {
        msg!("Error: Buy price must be >= sell price");
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
    
    // Calculate execution price (mid-price or buyer's price)
    let exec_price = args.execution_price.unwrap_or(buy_order.price);
    
    // Verify execution price is within bounds
    if exec_price < sell_order.price || exec_price > buy_order.price {
        return Err(PredictionMarketError::InvalidExecutionPrice.into());
    }
    
    // Note: In a full implementation, we would:
    // 1. Transfer tokens from seller's escrowed account to buyer
    // 2. Transfer USDC from buyer's locked funds to seller
    // For now, this is a simplified version that assumes tokens are already escrowed
    
    // Update orders
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
    
    // Calculate trade value
    let trade_value = ((match_amount as u128) * (exec_price as u128) / (PRICE_PRECISION as u128)) as i64;
    
    // Update market stats
    market.total_volume_e6 += trade_value;
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    // Update config stats
    config.total_volume_e6 += trade_value;
    config.serialize(&mut *config_info.data.borrow_mut())?;
    
    msg!("Trade executed successfully");
    msg!("Match amount: {}", match_amount);
    msg!("Execution price: {} (e6)", exec_price);
    msg!("Trade value: {} USDC (e6)", trade_value);
    msg!("Buy order {}: {}/{}", args.buy_order_id, buy_order.filled_amount, buy_order.amount);
    msg!("Sell order {}: {}/{}", args.sell_order_id, sell_order.filled_amount, sell_order.amount);
    
    Ok(())
}

fn process_propose_result(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: ProposeResultArgs,
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
    
    // Account 4: System Program
    let system_program_info = next_account_info(account_info_iter)?;
    
    // Load config
    let config = PredictionMarketConfig::try_from_slice(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify oracle admin
    if *oracle_admin_info.key != config.oracle_admin {
        msg!("Error: Only oracle admin can propose results");
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
    let market = Market::try_from_slice(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    let current_time = get_current_timestamp()?;
    
    // Verify market can be resolved
    if !market.can_resolve(current_time) {
        msg!("Error: Market cannot be resolved yet");
        return Err(PredictionMarketError::MarketNotResolvable.into());
    }
    
    // Verify OracleProposal PDA
    let (proposal_pda, proposal_bump) = Pubkey::find_program_address(
        &[ORACLE_PROPOSAL_SEED, &market_id_bytes],
        program_id,
    );
    if *proposal_info.key != proposal_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Check if proposal already exists
    if !proposal_info.data_is_empty() {
        msg!("Error: Proposal already exists for this market");
        return Err(PredictionMarketError::ProposalAlreadyExists.into());
    }
    
    // Create proposal account
    let rent = Rent::get()?;
    let space = OracleProposal::SIZE;
    let lamports = rent.minimum_balance(space);
    let proposal_seeds: &[&[u8]] = &[ORACLE_PROPOSAL_SEED, &market_id_bytes, &[proposal_bump]];
    
    invoke_signed(
        &system_instruction::create_account(
            oracle_admin_info.key,
            proposal_info.key,
            lamports,
            space as u64,
            program_id,
        ),
        &[oracle_admin_info.clone(), proposal_info.clone(), system_program_info.clone()],
        &[proposal_seeds],
    )?;
    
    // Initialize proposal
    let challenge_deadline = current_time + config.challenge_window_secs;
    let proposal = OracleProposal {
        discriminator: ORACLE_PROPOSAL_DISCRIMINATOR,
        market_id: args.market_id,
        proposer: *oracle_admin_info.key,
        proposed_result: args.result,
        status: ProposalStatus::Pending,
        proposed_at: current_time,
        challenge_deadline,
        bond_amount: config.proposer_bond_e6,
        challenger: None,
        challenger_result: None,
        challenger_bond: 0,
        bump: proposal_bump,
        reserved: [0u8; 32],
    };
    
    proposal.serialize(&mut *proposal_info.data.borrow_mut())?;
    
    msg!("Result proposed successfully");
    msg!("Market ID: {}", args.market_id);
    msg!("Proposed Result: {:?}", args.result);
    msg!("Challenge Deadline: {}", challenge_deadline);
    
    Ok(())
}

fn process_challenge_result(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: ChallengeResultArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Challenger (signer)
    let challenger_info = next_account_info(account_info_iter)?;
    check_signer(challenger_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: OracleProposal (writable)
    let proposal_info = next_account_info(account_info_iter)?;
    
    // Load config
    let config = PredictionMarketConfig::try_from_slice(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify OracleProposal PDA
    let market_id_bytes = args.market_id.to_le_bytes();
    let (proposal_pda, _) = Pubkey::find_program_address(
        &[ORACLE_PROPOSAL_SEED, &market_id_bytes],
        program_id,
    );
    if *proposal_info.key != proposal_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load proposal
    let mut proposal = OracleProposal::try_from_slice(&proposal_info.data.borrow())?;
    if proposal.discriminator != ORACLE_PROPOSAL_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    let current_time = get_current_timestamp()?;
    
    // Verify can challenge
    if !proposal.can_challenge(current_time) {
        msg!("Error: Cannot challenge - either already disputed or window closed");
        return Err(PredictionMarketError::CannotChallenge.into());
    }
    
    // Verify different result proposed
    if args.proposed_result == proposal.proposed_result {
        msg!("Error: Challenge result must be different from proposal");
        return Err(PredictionMarketError::SameResultAsProposal.into());
    }
    
    // Update proposal with challenge
    proposal.status = ProposalStatus::Disputed;
    proposal.challenger = Some(*challenger_info.key);
    proposal.challenger_result = Some(args.proposed_result);
    proposal.challenger_bond = config.proposer_bond_e6;
    proposal.serialize(&mut *proposal_info.data.borrow_mut())?;
    
    msg!("Result challenged successfully");
    msg!("Market ID: {}", args.market_id);
    msg!("Challenger: {}", challenger_info.key);
    msg!("Challenger's Proposed Result: {:?}", args.proposed_result);
    
    Ok(())
}

fn process_finalize_result(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Anyone can call (signer)
    let caller_info = next_account_info(account_info_iter)?;
    check_signer(caller_info)?;
    
    // Account 1: PredictionMarketConfig (writable)
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: OracleProposal (writable)
    let proposal_info = next_account_info(account_info_iter)?;
    
    // Load config
    let mut config = PredictionMarketConfig::try_from_slice(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Load market
    let mut market = Market::try_from_slice(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify Market PDA
    let market_id_bytes = market.market_id.to_le_bytes();
    let (market_pda, _) = Pubkey::find_program_address(
        &[MARKET_SEED, &market_id_bytes],
        program_id,
    );
    if *market_info.key != market_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load proposal
    let mut proposal = OracleProposal::try_from_slice(&proposal_info.data.borrow())?;
    if proposal.discriminator != ORACLE_PROPOSAL_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    let current_time = get_current_timestamp()?;
    
    // Verify can finalize (challenge window passed and not disputed)
    if !proposal.can_finalize(current_time) {
        msg!("Error: Cannot finalize - either disputed or challenge window not expired");
        return Err(PredictionMarketError::CannotFinalize.into());
    }
    
    // Finalize proposal
    proposal.status = ProposalStatus::Finalized;
    proposal.serialize(&mut *proposal_info.data.borrow_mut())?;
    
    // Update market with final result
    market.status = MarketStatus::Resolved;
    market.final_result = Some(proposal.proposed_result);
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    // Update config
    config.active_markets = config.active_markets.saturating_sub(1);
    config.serialize(&mut *config_info.data.borrow_mut())?;
    
    msg!("Result finalized successfully");
    msg!("Market ID: {}", market.market_id);
    msg!("Final Result: {:?}", proposal.proposed_result);
    
    Ok(())
}

fn process_resolve_dispute(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: ResolveDisputeArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Admin (signer)
    let admin_info = next_account_info(account_info_iter)?;
    check_signer(admin_info)?;
    
    // Account 1: PredictionMarketConfig (writable)
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: OracleProposal (writable)
    let proposal_info = next_account_info(account_info_iter)?;
    
    // Load config
    let mut config = PredictionMarketConfig::try_from_slice(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify admin
    if *admin_info.key != config.admin {
        return Err(PredictionMarketError::Unauthorized.into());
    }
    
    // Load market
    let mut market = Market::try_from_slice(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify Market PDA
    let market_id_bytes = market.market_id.to_le_bytes();
    let (market_pda, _) = Pubkey::find_program_address(
        &[MARKET_SEED, &market_id_bytes],
        program_id,
    );
    if *market_info.key != market_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load proposal
    let mut proposal = OracleProposal::try_from_slice(&proposal_info.data.borrow())?;
    if proposal.discriminator != ORACLE_PROPOSAL_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify proposal is disputed
    if proposal.status != ProposalStatus::Disputed {
        msg!("Error: Proposal is not disputed");
        return Err(PredictionMarketError::ProposalNotDisputed.into());
    }
    
    let current_time = get_current_timestamp()?;
    
    // Finalize with admin decision
    proposal.status = ProposalStatus::Finalized;
    proposal.serialize(&mut *proposal_info.data.borrow_mut())?;
    
    // Update market with final result
    market.status = MarketStatus::Resolved;
    market.final_result = Some(args.final_result);
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    // Update config
    config.active_markets = config.active_markets.saturating_sub(1);
    config.serialize(&mut *config_info.data.borrow_mut())?;
    
    msg!("Dispute resolved successfully");
    msg!("Market ID: {}", market.market_id);
    msg!("Final Result: {:?}", args.final_result);
    
    Ok(())
}

fn process_claim_winnings(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: User (signer)
    let user_info = next_account_info(account_info_iter)?;
    check_signer(user_info)?;
    
    // Account 1: Market
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 2: Position (writable)
    let position_info = next_account_info(account_info_iter)?;
    
    // Account 3: Market Vault (writable)
    let market_vault_info = next_account_info(account_info_iter)?;
    
    // Account 4: User's USDC Account (writable)
    let user_usdc_info = next_account_info(account_info_iter)?;
    
    // Account 5: User's YES Token Account (writable, for burning)
    let user_yes_info = next_account_info(account_info_iter)?;
    
    // Account 6: User's NO Token Account (writable, for burning)
    let user_no_info = next_account_info(account_info_iter)?;
    
    // Account 7: YES Mint (writable)
    let yes_mint_info = next_account_info(account_info_iter)?;
    
    // Account 8: NO Mint (writable)
    let no_mint_info = next_account_info(account_info_iter)?;
    
    // Account 9: Token Program
    let token_program_info = next_account_info(account_info_iter)?;
    
    // Load market
    let market = Market::try_from_slice(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify market is resolved
    if !market.is_resolved() {
        msg!("Error: Market is not resolved yet");
        return Err(PredictionMarketError::MarketNotResolved.into());
    }
    
    let final_result = market.final_result.ok_or(PredictionMarketError::MarketNotResolved)?;
    
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
    let mut position = Position::try_from_slice(&position_info.data.borrow())?;
    if position.discriminator != POSITION_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify not already settled
    if position.settled {
        msg!("Error: Position already settled");
        return Err(PredictionMarketError::AlreadySettled.into());
    }
    
    // Calculate settlement amount based on result
    let settlement_amount = position.calculate_settlement(final_result);
    
    let current_time = get_current_timestamp()?;
    let market_seeds: &[&[u8]] = &[MARKET_SEED, &market_id_bytes, &[market.bump]];
    
    // Burn winning tokens and transfer USDC
    match final_result {
        MarketResult::Yes => {
            // Burn YES tokens
            if position.yes_amount > 0 {
                invoke(
                    &spl_token::instruction::burn(
                        token_program_info.key,
                        user_yes_info.key,
                        yes_mint_info.key,
                        user_info.key,
                        &[],
                        position.yes_amount,
                    )?,
                    &[user_yes_info.clone(), yes_mint_info.clone(), user_info.clone(), token_program_info.clone()],
                )?;
            }
            
            // Transfer winnings (1 USDC per YES token)
            if settlement_amount > 0 {
                invoke_signed(
                    &spl_token::instruction::transfer(
                        token_program_info.key,
                        market_vault_info.key,
                        user_usdc_info.key,
                        market_info.key,
                        &[],
                        settlement_amount,
                    )?,
                    &[market_vault_info.clone(), user_usdc_info.clone(), market_info.clone(), token_program_info.clone()],
                    &[market_seeds],
                )?;
            }
            
            // Burn worthless NO tokens (if any)
            if position.no_amount > 0 {
                invoke(
                    &spl_token::instruction::burn(
                        token_program_info.key,
                        user_no_info.key,
                        no_mint_info.key,
                        user_info.key,
                        &[],
                        position.no_amount,
                    )?,
                    &[user_no_info.clone(), no_mint_info.clone(), user_info.clone(), token_program_info.clone()],
                )?;
            }
        }
        MarketResult::No => {
            // Burn NO tokens
            if position.no_amount > 0 {
                invoke(
                    &spl_token::instruction::burn(
                        token_program_info.key,
                        user_no_info.key,
                        no_mint_info.key,
                        user_info.key,
                        &[],
                        position.no_amount,
                    )?,
                    &[user_no_info.clone(), no_mint_info.clone(), user_info.clone(), token_program_info.clone()],
                )?;
            }
            
            // Transfer winnings (1 USDC per NO token)
            if settlement_amount > 0 {
                invoke_signed(
                    &spl_token::instruction::transfer(
                        token_program_info.key,
                        market_vault_info.key,
                        user_usdc_info.key,
                        market_info.key,
                        &[],
                        settlement_amount,
                    )?,
                    &[market_vault_info.clone(), user_usdc_info.clone(), market_info.clone(), token_program_info.clone()],
                    &[market_seeds],
                )?;
            }
            
            // Burn worthless YES tokens (if any)
            if position.yes_amount > 0 {
                invoke(
                    &spl_token::instruction::burn(
                        token_program_info.key,
                        user_yes_info.key,
                        yes_mint_info.key,
                        user_info.key,
                        &[],
                        position.yes_amount,
                    )?,
                    &[user_yes_info.clone(), yes_mint_info.clone(), user_info.clone(), token_program_info.clone()],
                )?;
            }
        }
        MarketResult::Invalid => {
            // Refund original cost - burn all tokens
            if position.yes_amount > 0 {
                invoke(
                    &spl_token::instruction::burn(
                        token_program_info.key,
                        user_yes_info.key,
                        yes_mint_info.key,
                        user_info.key,
                        &[],
                        position.yes_amount,
                    )?,
                    &[user_yes_info.clone(), yes_mint_info.clone(), user_info.clone(), token_program_info.clone()],
                )?;
            }
            if position.no_amount > 0 {
                invoke(
                    &spl_token::instruction::burn(
                        token_program_info.key,
                        user_no_info.key,
                        no_mint_info.key,
                        user_info.key,
                        &[],
                        position.no_amount,
                    )?,
                    &[user_no_info.clone(), no_mint_info.clone(), user_info.clone(), token_program_info.clone()],
                )?;
            }
            
            // Refund total cost
            if settlement_amount > 0 {
                invoke_signed(
                    &spl_token::instruction::transfer(
                        token_program_info.key,
                        market_vault_info.key,
                        user_usdc_info.key,
                        market_info.key,
                        &[],
                        settlement_amount,
                    )?,
                    &[market_vault_info.clone(), user_usdc_info.clone(), market_info.clone(), token_program_info.clone()],
                    &[market_seeds],
                )?;
            }
        }
    }
    
    // Update position
    position.settled = true;
    position.settlement_amount = settlement_amount;
    position.updated_at = current_time;
    position.serialize(&mut *position_info.data.borrow_mut())?;
    
    msg!("Winnings claimed successfully");
    msg!("User: {}", user_info.key);
    msg!("Market ID: {}", market.market_id);
    msg!("Final Result: {:?}", final_result);
    msg!("Settlement Amount: {} USDC (e6)", settlement_amount);
    
    Ok(())
}

fn process_refund_cancelled_market(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: User (signer)
    let user_info = next_account_info(account_info_iter)?;
    check_signer(user_info)?;
    
    // Account 1: Market
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 2: Position (writable)
    let position_info = next_account_info(account_info_iter)?;
    
    // Account 3: Market Vault (writable)
    let market_vault_info = next_account_info(account_info_iter)?;
    
    // Account 4: User's USDC Account (writable)
    let user_usdc_info = next_account_info(account_info_iter)?;
    
    // Account 5: User's YES Token Account (writable)
    let user_yes_info = next_account_info(account_info_iter)?;
    
    // Account 6: User's NO Token Account (writable)
    let user_no_info = next_account_info(account_info_iter)?;
    
    // Account 7: YES Mint (writable)
    let yes_mint_info = next_account_info(account_info_iter)?;
    
    // Account 8: NO Mint (writable)
    let no_mint_info = next_account_info(account_info_iter)?;
    
    // Account 9: Token Program
    let token_program_info = next_account_info(account_info_iter)?;
    
    // Load market
    let market = Market::try_from_slice(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify market is cancelled
    if market.status != MarketStatus::Cancelled {
        msg!("Error: Market is not cancelled");
        return Err(PredictionMarketError::MarketNotCancelled.into());
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
    let mut position = Position::try_from_slice(&position_info.data.borrow())?;
    if position.discriminator != POSITION_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if position.settled {
        msg!("Error: Position already settled");
        return Err(PredictionMarketError::AlreadySettled.into());
    }
    
    let current_time = get_current_timestamp()?;
    let market_seeds: &[&[u8]] = &[MARKET_SEED, &market_id_bytes, &[market.bump]];
    
    // Calculate refund amount (total cost basis)
    let refund_amount = position.total_cost_e6;
    
    // Burn all tokens
    if position.yes_amount > 0 {
        invoke(
            &spl_token::instruction::burn(
                token_program_info.key,
                user_yes_info.key,
                yes_mint_info.key,
                user_info.key,
                &[],
                position.yes_amount,
            )?,
            &[user_yes_info.clone(), yes_mint_info.clone(), user_info.clone(), token_program_info.clone()],
        )?;
    }
    
    if position.no_amount > 0 {
        invoke(
            &spl_token::instruction::burn(
                token_program_info.key,
                user_no_info.key,
                no_mint_info.key,
                user_info.key,
                &[],
                position.no_amount,
            )?,
            &[user_no_info.clone(), no_mint_info.clone(), user_info.clone(), token_program_info.clone()],
        )?;
    }
    
    // Transfer refund from vault to user
    if refund_amount > 0 {
        invoke_signed(
            &spl_token::instruction::transfer(
                token_program_info.key,
                market_vault_info.key,
                user_usdc_info.key,
                market_info.key,
                &[],
                refund_amount,
            )?,
            &[market_vault_info.clone(), user_usdc_info.clone(), market_info.clone(), token_program_info.clone()],
            &[market_seeds],
        )?;
    }
    
    // Update position
    position.settled = true;
    position.settlement_amount = refund_amount;
    position.updated_at = current_time;
    position.serialize(&mut *position_info.data.borrow_mut())?;
    
    msg!("Refund for cancelled market processed");
    msg!("User: {}", user_info.key);
    msg!("Market ID: {}", market.market_id);
    msg!("Refund Amount: {} USDC (e6)", refund_amount);
    
    Ok(())
}

fn process_update_admin(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: UpdateAdminArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Current Admin (signer)
    let admin_info = next_account_info(account_info_iter)?;
    check_signer(admin_info)?;
    
    // Account 1: PredictionMarketConfig (writable)
    let config_info = next_account_info(account_info_iter)?;
    
    // Load config
    let mut config = PredictionMarketConfig::try_from_slice(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify current admin
    if *admin_info.key != config.admin {
        return Err(PredictionMarketError::Unauthorized.into());
    }
    
    // Update admin
    config.admin = args.new_admin;
    config.serialize(&mut *config_info.data.borrow_mut())?;
    
    msg!("Admin updated successfully");
    msg!("Old Admin: {}", admin_info.key);
    msg!("New Admin: {}", args.new_admin);
    
    Ok(())
}

fn process_update_oracle_admin(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: UpdateOracleAdminArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Admin (signer)
    let admin_info = next_account_info(account_info_iter)?;
    check_signer(admin_info)?;
    
    // Account 1: PredictionMarketConfig (writable)
    let config_info = next_account_info(account_info_iter)?;
    
    // Load config
    let mut config = PredictionMarketConfig::try_from_slice(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify admin
    if *admin_info.key != config.admin {
        return Err(PredictionMarketError::Unauthorized.into());
    }
    
    // Update oracle admin
    let old_oracle_admin = config.oracle_admin;
    config.oracle_admin = args.new_oracle_admin;
    config.serialize(&mut *config_info.data.borrow_mut())?;
    
    msg!("Oracle admin updated successfully");
    msg!("Old Oracle Admin: {}", old_oracle_admin);
    msg!("New Oracle Admin: {}", args.new_oracle_admin);
    
    Ok(())
}

fn process_set_paused(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: SetPausedArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Admin (signer)
    let admin_info = next_account_info(account_info_iter)?;
    check_signer(admin_info)?;
    
    // Account 1: PredictionMarketConfig (writable)
    let config_info = next_account_info(account_info_iter)?;
    
    // Load config
    let mut config = PredictionMarketConfig::try_from_slice(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify admin
    if *admin_info.key != config.admin {
        return Err(PredictionMarketError::Unauthorized.into());
    }
    
    // Update paused state
    config.is_paused = args.paused;
    config.serialize(&mut *config_info.data.borrow_mut())?;
    
    msg!("Program paused state updated");
    msg!("Paused: {}", args.paused);
    
    Ok(())
}

fn process_update_oracle_config(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: UpdateOracleConfigArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Admin (signer)
    let admin_info = next_account_info(account_info_iter)?;
    check_signer(admin_info)?;
    
    // Account 1: PredictionMarketConfig (writable)
    let config_info = next_account_info(account_info_iter)?;
    
    // Load config
    let mut config = PredictionMarketConfig::try_from_slice(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify admin
    if *admin_info.key != config.admin {
        return Err(PredictionMarketError::Unauthorized.into());
    }
    
    // Update oracle config
    if let Some(challenge_window) = args.challenge_window_secs {
        config.challenge_window_secs = challenge_window;
    }
    if let Some(proposer_bond) = args.proposer_bond_e6 {
        config.proposer_bond_e6 = proposer_bond;
    }
    config.serialize(&mut *config_info.data.borrow_mut())?;
    
    msg!("Oracle config updated successfully");
    msg!("Challenge Window: {} seconds", config.challenge_window_secs);
    msg!("Proposer Bond: {} (e6)", config.proposer_bond_e6);
    
    Ok(())
}

fn process_add_authorized_caller(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: AddAuthorizedCallerArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Admin (signer)
    let admin_info = next_account_info(account_info_iter)?;
    check_signer(admin_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Load config
    let config = PredictionMarketConfig::try_from_slice(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify admin
    if *admin_info.key != config.admin {
        return Err(PredictionMarketError::Unauthorized.into());
    }
    
    // Note: In a full implementation, we would maintain a list of authorized callers
    // For now, this is a placeholder
    msg!("Authorized caller added (placeholder)");
    msg!("Caller: {}", args.caller);
    
    Ok(())
}

fn process_remove_authorized_caller(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RemoveAuthorizedCallerArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Admin (signer)
    let admin_info = next_account_info(account_info_iter)?;
    check_signer(admin_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Load config
    let config = PredictionMarketConfig::try_from_slice(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify admin
    if *admin_info.key != config.admin {
        return Err(PredictionMarketError::Unauthorized.into());
    }
    
    // Note: In a full implementation, we would maintain a list of authorized callers
    // For now, this is a placeholder
    msg!("Authorized caller removed (placeholder)");
    msg!("Caller: {}", args.caller);
    
    Ok(())
}

