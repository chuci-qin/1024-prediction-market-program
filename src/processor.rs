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
    PRICE_PRECISION, MIN_PRICE, MAX_PRICE,
};
use crate::utils::{
    check_signer, get_current_timestamp,
    safe_add_u64,
    validate_price, validate_price_pair,
    deserialize_account,
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
        
        // Multi-Outcome Market Instructions
        PredictionMarketInstruction::CreateMultiOutcomeMarket(args) => {
            msg!("Instruction: CreateMultiOutcomeMarket");
            process_create_multi_outcome_market(program_id, accounts, args)
        }
        PredictionMarketInstruction::MintMultiOutcomeCompleteSet(args) => {
            msg!("Instruction: MintMultiOutcomeCompleteSet");
            process_mint_multi_outcome_complete_set(program_id, accounts, args)
        }
        PredictionMarketInstruction::RedeemMultiOutcomeCompleteSet(args) => {
            msg!("Instruction: RedeemMultiOutcomeCompleteSet");
            process_redeem_multi_outcome_complete_set(program_id, accounts, args)
        }
        PredictionMarketInstruction::PlaceMultiOutcomeOrder(args) => {
            msg!("Instruction: PlaceMultiOutcomeOrder");
            process_place_multi_outcome_order(program_id, accounts, args)
        }
        PredictionMarketInstruction::ProposeMultiOutcomeResult(args) => {
            msg!("Instruction: ProposeMultiOutcomeResult");
            process_propose_multi_outcome_result(program_id, accounts, args)
        }
        PredictionMarketInstruction::ClaimMultiOutcomeWinnings(args) => {
            msg!("Instruction: ClaimMultiOutcomeWinnings");
            process_claim_multi_outcome_winnings(program_id, accounts, args)
        }
        
        // === Relayer Instructions ===
        PredictionMarketInstruction::RelayerMintCompleteSet(args) => {
            msg!("Instruction: RelayerMintCompleteSet");
            process_relayer_mint_complete_set(program_id, accounts, args)
        }
        PredictionMarketInstruction::RelayerRedeemCompleteSet(args) => {
            msg!("Instruction: RelayerRedeemCompleteSet");
            process_relayer_redeem_complete_set(program_id, accounts, args)
        }
        PredictionMarketInstruction::RelayerPlaceOrder(args) => {
            msg!("Instruction: RelayerPlaceOrder");
            process_relayer_place_order(program_id, accounts, args)
        }
        PredictionMarketInstruction::RelayerCancelOrder(args) => {
            msg!("Instruction: RelayerCancelOrder");
            process_relayer_cancel_order(program_id, accounts, args)
        }
        PredictionMarketInstruction::RelayerClaimWinnings(args) => {
            msg!("Instruction: RelayerClaimWinnings");
            process_relayer_claim_winnings(program_id, accounts, args)
        }
        PredictionMarketInstruction::RelayerRefundCancelledMarket(args) => {
            msg!("Instruction: RelayerRefundCancelledMarket");
            process_relayer_refund_cancelled_market(program_id, accounts, args)
        }
        PredictionMarketInstruction::RelayerMintMultiOutcomeCompleteSet(args) => {
            msg!("Instruction: RelayerMintMultiOutcomeCompleteSet");
            process_relayer_mint_multi_outcome_complete_set(program_id, accounts, args)
        }
        PredictionMarketInstruction::RelayerRedeemMultiOutcomeCompleteSet(args) => {
            msg!("Instruction: RelayerRedeemMultiOutcomeCompleteSet");
            process_relayer_redeem_multi_outcome_complete_set(program_id, accounts, args)
        }
        PredictionMarketInstruction::RelayerPlaceMultiOutcomeOrder(args) => {
            msg!("Instruction: RelayerPlaceMultiOutcomeOrder");
            process_relayer_place_multi_outcome_order(program_id, accounts, args)
        }
        PredictionMarketInstruction::RelayerClaimMultiOutcomeWinnings(args) => {
            msg!("Instruction: RelayerClaimMultiOutcomeWinnings");
            process_relayer_claim_multi_outcome_winnings(program_id, accounts, args)
        }
        
        // === Multi-Outcome Matching Operations ===
        PredictionMarketInstruction::MatchMintMulti(args) => {
            msg!("Instruction: MatchMintMulti");
            process_match_mint_multi(program_id, accounts, args)
        }
        PredictionMarketInstruction::MatchBurnMulti(args) => {
            msg!("Instruction: MatchBurnMulti");
            process_match_burn_multi(program_id, accounts, args)
        }
        PredictionMarketInstruction::RelayerMatchMintMulti(args) => {
            msg!("Instruction: RelayerMatchMintMulti");
            process_relayer_match_mint_multi(program_id, accounts, args)
        }
        PredictionMarketInstruction::RelayerMatchBurnMulti(args) => {
            msg!("Instruction: RelayerMatchBurnMulti");
            process_relayer_match_burn_multi(program_id, accounts, args)
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
    let mut config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
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
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
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
    let mut yes_order = deserialize_account::<Order>(&yes_order_info.data.borrow())?;
    let mut no_order = deserialize_account::<Order>(&no_order_info.data.borrow())?;
    
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
    validate_price_pair(args.yes_price, args.no_price)?;
    
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
    let mut config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
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
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
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
    let (yes_order_pda, yes_order_bump) = Pubkey::find_program_address(
        &[ORDER_SEED, &market_id_bytes, &yes_order_id_bytes],
        program_id,
    );
    if *yes_order_info.key != yes_order_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let no_order_id_bytes = args.no_order_id.to_le_bytes();
    let (no_order_pda, no_order_bump) = Pubkey::find_program_address(
        &[ORDER_SEED, &market_id_bytes, &no_order_id_bytes],
        program_id,
    );
    if *no_order_info.key != no_order_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load orders
    let mut yes_order = deserialize_account::<Order>(&yes_order_info.data.borrow())?;
    let mut no_order = deserialize_account::<Order>(&no_order_info.data.borrow())?;
    
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
    
    // Verify orders have escrow (sell orders should have escrowed tokens)
    if !yes_order.has_escrow() || !no_order.has_escrow() {
        msg!("Error: Sell orders must have escrowed tokens for MatchBurn");
        return Err(PredictionMarketError::EscrowNotFound.into());
    }
    
    // P5.2.3: Verify escrow balances are sufficient
    use crate::utils::{verify_escrow_balance, verify_escrow_pda};
    
    // Verify YES escrow PDA
    verify_escrow_pda(
        yes_seller_token_info,
        program_id,
        args.market_id,
        args.yes_order_id,
    )?;
    
    // Verify NO escrow PDA
    verify_escrow_pda(
        no_seller_token_info,
        program_id,
        args.market_id,
        args.no_order_id,
    )?;
    
    // Verify YES escrow has enough tokens
    verify_escrow_balance(yes_seller_token_info, match_amount)?;
    
    // Verify NO escrow has enough tokens
    verify_escrow_balance(no_seller_token_info, match_amount)?;
    
    // Calculate PDA seeds for signing
    let market_seeds: &[&[u8]] = &[MARKET_SEED, &market_id_bytes, &[market.bump]];
    let yes_order_seeds: &[&[u8]] = &[ORDER_SEED, &market_id_bytes, &yes_order_id_bytes, &[yes_order_bump]];
    let no_order_seeds: &[&[u8]] = &[ORDER_SEED, &market_id_bytes, &no_order_id_bytes, &[no_order_bump]];
    
    // Burn YES tokens from YES seller's escrow (using order PDA as signer)
    // yes_seller_token_info is now the escrow token account
    invoke_signed(
        &spl_token::instruction::burn(
            token_program_info.key,
            yes_seller_token_info.key,
            yes_mint_info.key,
            yes_order_info.key, // Order PDA is the owner of escrow
            &[],
            match_amount,
        )?,
        &[yes_seller_token_info.clone(), yes_mint_info.clone(), yes_order_info.clone(), token_program_info.clone()],
        &[yes_order_seeds],
    )?;
    
    // Burn NO tokens from NO seller's escrow
    invoke_signed(
        &spl_token::instruction::burn(
            token_program_info.key,
            no_seller_token_info.key,
            no_mint_info.key,
            no_order_info.key, // Order PDA is the owner of escrow
            &[],
            match_amount,
        )?,
        &[no_seller_token_info.clone(), no_mint_info.clone(), no_order_info.clone(), token_program_info.clone()],
        &[no_order_seeds],
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
    
    // Account 5: Seller's Token Account / Escrow (writable)
    let seller_token_info = next_account_info(account_info_iter)?;
    
    // Account 6: Buyer's Token Account (writable)
    let buyer_token_info = next_account_info(account_info_iter)?;
    
    // Account 7: Token Program
    let token_program_info = next_account_info(account_info_iter)?;
    
    // Account 8: Buyer Position PDA (writable) - Phase 2 addition
    let buyer_position_info = next_account_info(account_info_iter)?;
    
    // Account 9: Seller Position PDA (writable) - Phase 2 addition
    let seller_position_info = next_account_info(account_info_iter)?;
    
    // Account 10: System Program - Phase 2 addition (for Position creation)
    let system_program_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let mut config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
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
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if !market.is_tradeable() {
        return Err(PredictionMarketError::MarketNotTradeable.into());
    }
    
    // Verify Order PDAs (taker = buy, maker = sell in this context)
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
    
    // Calculate execution price (use provided price or buyer's price)
    let exec_price = args.price;
    
    // Verify execution price is within bounds
    if exec_price < sell_order.price || exec_price > buy_order.price {
        return Err(PredictionMarketError::InvalidExecutionPrice.into());
    }
    
    // P5.2.4: Verify sell order has escrow with sufficient tokens
    if !sell_order.has_escrow() {
        msg!("Error: Sell order must have escrowed tokens for ExecuteTrade");
        return Err(PredictionMarketError::EscrowNotFound.into());
    }
    
    // Verify escrow PDA and balance
    use crate::utils::{verify_escrow_balance, verify_escrow_pda};
    
    // Note: seller_token_info should be the escrow token account for sell orders
    verify_escrow_pda(
        seller_token_info,
        program_id,
        args.market_id,
        args.maker_order_id,
    )?;
    
    verify_escrow_balance(seller_token_info, match_amount)?;
    
    // Transfer tokens from seller's escrow to buyer's token account
    // Using sell_order PDA as signer
    let sell_order_seeds: &[&[u8]] = &[ORDER_SEED, &market_id_bytes, &maker_order_id_bytes, &[sell_order.bump]];
    
    invoke_signed(
        &spl_token::instruction::transfer(
            token_program_info.key,
            seller_token_info.key,
            buyer_token_info.key,
            sell_order_info.key, // Sell order PDA is the owner of escrow
            &[],
            match_amount,
        )?,
        &[seller_token_info.clone(), buyer_token_info.clone(), sell_order_info.clone(), token_program_info.clone()],
        &[sell_order_seeds],
    )?;
    
    msg!("Transferred {} tokens from escrow to buyer", match_amount);
    
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
    
    // Phase 2: Update Positions
    // Verify Buyer Position PDA
    let buyer_owner = buy_order.owner;
    let (buyer_position_pda, buyer_position_bump) = Pubkey::find_program_address(
        &[POSITION_SEED, &market_id_bytes, buyer_owner.as_ref()],
        program_id,
    );
    if *buyer_position_info.key != buyer_position_pda {
        msg!("Error: Invalid buyer position PDA");
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Verify Seller Position PDA  
    let seller_owner = sell_order.owner;
    let (seller_position_pda, seller_position_bump) = Pubkey::find_program_address(
        &[POSITION_SEED, &market_id_bytes, seller_owner.as_ref()],
        program_id,
    );
    if *seller_position_info.key != seller_position_pda {
        msg!("Error: Invalid seller position PDA");
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Load or create Buyer Position
    let buyer_position_exists = buyer_position_info.data_len() > 0 && 
        buyer_position_info.owner == program_id;
    
    if buyer_position_exists {
        // Update existing buyer position
        let mut buyer_position = deserialize_account::<Position>(&buyer_position_info.data.borrow())?;
        if buyer_position.discriminator != POSITION_DISCRIMINATOR {
            return Err(PredictionMarketError::InvalidAccountData.into());
        }
        
        // Buyer receives tokens at exec_price
        buyer_position.add_tokens(buy_order.outcome, match_amount, exec_price, current_time);
        buyer_position.serialize(&mut *buyer_position_info.data.borrow_mut())?;
        msg!("Updated buyer position: {} tokens added", match_amount);
    } else {
        // Create new buyer position
        let rent = Rent::get()?;
        let space = Position::SIZE;
        let lamports = rent.minimum_balance(space);
        let buyer_position_seeds: &[&[u8]] = &[POSITION_SEED, &market_id_bytes, buyer_owner.as_ref(), &[buyer_position_bump]];
        
        invoke_signed(
            &system_instruction::create_account(
                relayer_info.key,
                buyer_position_info.key,
                lamports,
                space as u64,
                program_id,
            ),
            &[relayer_info.clone(), buyer_position_info.clone(), system_program_info.clone()],
            &[buyer_position_seeds],
        )?;
        
        let mut buyer_position = Position::new(args.market_id, buyer_owner, buyer_position_bump, current_time);
        buyer_position.add_tokens(buy_order.outcome, match_amount, exec_price, current_time);
        buyer_position.serialize(&mut *buyer_position_info.data.borrow_mut())?;
        msg!("Created new buyer position with {} tokens", match_amount);
    }
    
    // Update Seller Position (must exist - they're selling tokens they own)
    if seller_position_info.data_len() > 0 && seller_position_info.owner == program_id {
        let mut seller_position = deserialize_account::<Position>(&seller_position_info.data.borrow())?;
        if seller_position.discriminator != POSITION_DISCRIMINATOR {
            return Err(PredictionMarketError::InvalidAccountData.into());
        }
        
        // Seller removes tokens at exec_price (realize PnL)
        seller_position.remove_tokens(sell_order.outcome, match_amount, exec_price, current_time);
        seller_position.serialize(&mut *seller_position_info.data.borrow_mut())?;
        msg!("Updated seller position: {} tokens removed", match_amount);
    } else {
        // Seller should have a position if they're selling tokens
        // This is an edge case - create an empty position and subtract (will go negative in logic)
        msg!("Warning: Seller position not found, skipping position update");
    }
    
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
    msg!("Taker order {}: {}/{}", args.taker_order_id, buy_order.filled_amount, buy_order.amount);
    msg!("Maker order {}: {}/{}", args.maker_order_id, sell_order.filled_amount, sell_order.amount);
    
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
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
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
    let market = deserialize_account::<Market>(&market_info.data.borrow())?;
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
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
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
    let mut proposal = deserialize_account::<OracleProposal>(&proposal_info.data.borrow())?;
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
    if args.result == proposal.proposed_result {
        msg!("Error: Challenge result must be different from proposal");
        return Err(PredictionMarketError::SameResultAsProposal.into());
    }
    
    // Update proposal with challenge
    proposal.status = ProposalStatus::Disputed;
    proposal.challenger = Some(*challenger_info.key);
    proposal.challenger_result = Some(args.result);
    proposal.challenger_bond = config.proposer_bond_e6;
    proposal.serialize(&mut *proposal_info.data.borrow_mut())?;
    
    msg!("Result challenged successfully");
    msg!("Market ID: {}", args.market_id);
    msg!("Challenger: {}", challenger_info.key);
    msg!("Challenger's Proposed Result: {:?}", args.result);
    
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
    let mut config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Load market
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
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
    let mut proposal = deserialize_account::<OracleProposal>(&proposal_info.data.borrow())?;
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
    let mut config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify admin
    if *admin_info.key != config.admin {
        return Err(PredictionMarketError::Unauthorized.into());
    }
    
    // Load market
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
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
    let mut proposal = deserialize_account::<OracleProposal>(&proposal_info.data.borrow())?;
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
    market.final_result = Some(args.result);
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    // Update config
    config.active_markets = config.active_markets.saturating_sub(1);
    config.serialize(&mut *config_info.data.borrow_mut())?;
    
    msg!("Dispute resolved successfully");
    msg!("Market ID: {}", market.market_id);
    msg!("Final Result: {:?}", args.result);
    
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
    let market = deserialize_account::<Market>(&market_info.data.borrow())?;
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
    let mut position = deserialize_account::<Position>(&position_info.data.borrow())?;
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
    let market = deserialize_account::<Market>(&market_info.data.borrow())?;
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
    let mut position = deserialize_account::<Position>(&position_info.data.borrow())?;
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
    let mut config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
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
    let mut config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
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
    let mut config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
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
    let mut config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
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
    use crate::state::{AuthorizedCallers, AUTHORIZED_CALLERS_SEED, AUTHORIZED_CALLERS_DISCRIMINATOR};
    
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Admin (signer)
    let admin_info = next_account_info(account_info_iter)?;
    check_signer(admin_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: AuthorizedCallers PDA (writable)
    let callers_info = next_account_info(account_info_iter)?;
    
    // Account 3: System Program (for creating if needed)
    let system_program_info = next_account_info(account_info_iter)?;
    
    // Load config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify admin
    if *admin_info.key != config.admin {
        return Err(PredictionMarketError::Unauthorized.into());
    }
    
    // Verify AuthorizedCallers PDA
    let (callers_pda, callers_bump) = Pubkey::find_program_address(
        &[AUTHORIZED_CALLERS_SEED],
        program_id,
    );
    if *callers_info.key != callers_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let current_time = get_current_timestamp()?;
    
    // Check if PDA needs to be created
    if callers_info.data_len() == 0 {
        // Create the AuthorizedCallers account
        let rent = Rent::get()?;
        let space = AuthorizedCallers::SIZE;
        let lamports = rent.minimum_balance(space);
        let callers_seeds: &[&[u8]] = &[AUTHORIZED_CALLERS_SEED, &[callers_bump]];
        
        invoke_signed(
            &system_instruction::create_account(
                admin_info.key,
                callers_info.key,
                lamports,
                space as u64,
                program_id,
            ),
            &[admin_info.clone(), callers_info.clone(), system_program_info.clone()],
            &[callers_seeds],
        )?;
        
        // Initialize with the new caller
        let mut callers = AuthorizedCallers::new(callers_bump, current_time);
        callers.add_caller(args.caller, current_time)
            .map_err(|_| PredictionMarketError::InvalidAccountData)?;
        callers.serialize(&mut *callers_info.data.borrow_mut())?;
        
        msg!("Created AuthorizedCallers registry and added first caller");
    } else {
        // Load existing and add caller
        let mut callers = deserialize_account::<AuthorizedCallers>(&callers_info.data.borrow())?;
        if callers.discriminator != AUTHORIZED_CALLERS_DISCRIMINATOR {
            return Err(PredictionMarketError::InvalidAccountData.into());
        }
        
        callers.add_caller(args.caller, current_time)
            .map_err(|_| {
                msg!("Error: Caller already authorized or list is full");
                PredictionMarketError::InvalidAccountData
            })?;
        
        callers.serialize(&mut *callers_info.data.borrow_mut())?;
    }
    
    msg!("Authorized caller added successfully");
    msg!("Caller: {}", args.caller);
    
    Ok(())
}

fn process_remove_authorized_caller(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RemoveAuthorizedCallerArgs,
) -> ProgramResult {
    use crate::state::{AuthorizedCallers, AUTHORIZED_CALLERS_SEED, AUTHORIZED_CALLERS_DISCRIMINATOR};
    
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Admin (signer)
    let admin_info = next_account_info(account_info_iter)?;
    check_signer(admin_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: AuthorizedCallers PDA (writable)
    let callers_info = next_account_info(account_info_iter)?;
    
    // Load config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify admin
    if *admin_info.key != config.admin {
        return Err(PredictionMarketError::Unauthorized.into());
    }
    
    // Verify AuthorizedCallers PDA
    let (callers_pda, _) = Pubkey::find_program_address(
        &[AUTHORIZED_CALLERS_SEED],
        program_id,
    );
    if *callers_info.key != callers_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Check if PDA exists
    if callers_info.data_len() == 0 {
        msg!("Error: AuthorizedCallers registry not initialized");
        return Err(PredictionMarketError::AccountNotInitialized.into());
    }
    
    // Load and update
    let mut callers = deserialize_account::<AuthorizedCallers>(&callers_info.data.borrow())?;
    if callers.discriminator != AUTHORIZED_CALLERS_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    let current_time = get_current_timestamp()?;
    
    callers.remove_caller(&args.caller, current_time)
        .map_err(|_| {
            msg!("Error: Caller not found in authorized list");
            PredictionMarketError::InvalidAccountData
        })?;
    
    callers.serialize(&mut *callers_info.data.borrow_mut())?;
    
    msg!("Authorized caller removed successfully");
    msg!("Caller: {}", args.caller);
    msg!("Remaining callers: {}", callers.count);
    
    Ok(())
}

// ============================================================================
// Multi-Outcome Market Processors
// ============================================================================

fn process_create_multi_outcome_market(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: CreateMultiOutcomeMarketArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Creator (signer)
    let creator_info = next_account_info(account_info_iter)?;
    check_signer(creator_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market PDA
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: Market Vault PDA
    let market_vault_info = next_account_info(account_info_iter)?;
    
    // Account 4: USDC Mint
    let usdc_mint_info = next_account_info(account_info_iter)?;
    
    // Account 5: Token Program
    let token_program_info = next_account_info(account_info_iter)?;
    
    // Account 6: System Program
    let system_program_info = next_account_info(account_info_iter)?;
    
    // Account 7: Rent Sysvar
    let rent_info = next_account_info(account_info_iter)?;
    
    // Validate num_outcomes
    if args.num_outcomes < 2 || args.num_outcomes > 32 {
        msg!("Number of outcomes must be between 2 and 32");
        return Err(PredictionMarketError::InvalidArgument.into());
    }
    
    if args.outcome_hashes.len() != args.num_outcomes as usize {
        msg!("Outcome hashes count must match num_outcomes");
        return Err(PredictionMarketError::InvalidArgument.into());
    }
    
    // Validate creator fee
    if args.creator_fee_bps > 500 {
        msg!("Creator fee cannot exceed 5%");
        return Err(PredictionMarketError::InvalidArgument.into());
    }
    
    // Load config
    msg!("Loading config from account: {}", config_info.key);
    msg!("Config account data len: {}", config_info.data.borrow().len());
    let mut config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    msg!("Config loaded successfully");
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        msg!("Invalid discriminator");
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    msg!("Discriminator valid");
    
    // Check if paused
    if config.is_paused {
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    msg!("Config not paused");
    
    let market_id = config.next_market_id;
    msg!("Next market ID: {}", market_id);
    let market_id_bytes = market_id.to_le_bytes();
    let current_time = get_current_timestamp()?;
    
    // Verify Market PDA
    let (market_pda, market_bump) = Pubkey::find_program_address(
        &[MARKET_SEED, &market_id_bytes],
        program_id,
    );
    if *market_info.key != market_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Verify Market Vault PDA
    let (market_vault_pda, market_vault_bump) = Pubkey::find_program_address(
        &[MARKET_VAULT_SEED, &market_id_bytes],
        program_id,
    );
    if *market_vault_info.key != market_vault_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    let rent = Rent::get()?;
    let market_seeds: &[&[u8]] = &[MARKET_SEED, &market_id_bytes, &[market_bump]];
    
    // Create Market account
    let market_space = Market::SIZE;
    let market_lamports = rent.minimum_balance(market_space);
    
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
    
    // Create outcome token mints
    let mint_space = spl_token::state::Mint::LEN;
    let mint_lamports = rent.minimum_balance(mint_space);
    
    for outcome_idx in 0..args.num_outcomes {
        let outcome_mint_info = next_account_info(account_info_iter)?;
        
        // Verify outcome mint PDA
        let (outcome_mint_pda, outcome_mint_bump) = Pubkey::find_program_address(
            &[OUTCOME_MINT_SEED, &market_id_bytes, &[outcome_idx]],
            program_id,
        );
        if *outcome_mint_info.key != outcome_mint_pda {
            msg!("Invalid outcome mint PDA for outcome {}", outcome_idx);
            return Err(PredictionMarketError::InvalidPDA.into());
        }
        
        let outcome_mint_seeds: &[&[u8]] = &[OUTCOME_MINT_SEED, &market_id_bytes, &[outcome_idx], &[outcome_mint_bump]];
        
        // Create outcome mint
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
        
        // Initialize outcome mint (authority = Market PDA)
        invoke_signed(
            &spl_token::instruction::initialize_mint(
                token_program_info.key,
                outcome_mint_info.key,
                market_info.key, // mint_authority
                Some(market_info.key), // freeze_authority
                6, // decimals
            )?,
            &[outcome_mint_info.clone(), rent_info.clone()],
            &[market_seeds],
        )?;
        
        msg!("Created outcome {} mint: {}", outcome_idx, outcome_mint_info.key);
    }
    
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
        market_type: MarketType::MultiOutcome,
        num_outcomes: args.num_outcomes,
        creator: *creator_info.key,
        question_hash: args.question_hash,
        resolution_spec_hash: args.resolution_spec_hash,
        yes_mint: Pubkey::default(), // Not used for multi-outcome
        no_mint: Pubkey::default(),  // Not used for multi-outcome
        market_vault: *market_vault_info.key,
        status: MarketStatus::Pending,
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
    
    msg!("Multi-outcome market created successfully");
    msg!("Market ID: {}", market_id);
    msg!("Creator: {}", creator_info.key);
    msg!("Num Outcomes: {}", args.num_outcomes);
    msg!("Market Vault: {}", market_vault_info.key);
    msg!("Resolution Time: {}", args.resolution_time);
    msg!("Creator Fee: {} bps", args.creator_fee_bps);
    
    Ok(())
}

fn process_mint_multi_outcome_complete_set(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: MintMultiOutcomeCompleteSetArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: User (signer)
    let user_info = next_account_info(account_info_iter)?;
    check_signer(user_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: Market Vault
    let market_vault_info = next_account_info(account_info_iter)?;
    
    // Account 4: User USDC Account
    let user_usdc_info = next_account_info(account_info_iter)?;
    
    // Account 5: User Position PDA
    let position_info = next_account_info(account_info_iter)?;
    
    // Account 6: Token Program
    let token_program_info = next_account_info(account_info_iter)?;
    
    // Account 7: System Program
    let system_program_info = next_account_info(account_info_iter)?;
    
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
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    if market.market_type != MarketType::MultiOutcome {
        msg!("This market is not a multi-outcome market");
        return Err(PredictionMarketError::InvalidArgument.into());
    }
    if !market.is_tradeable() {
        return Err(PredictionMarketError::MarketNotActive.into());
    }
    
    // Validate amount
    if args.amount == 0 {
        return Err(PredictionMarketError::InvalidAmount.into());
    }
    
    let market_id_bytes = args.market_id.to_le_bytes();
    let market_seeds: &[&[u8]] = &[MARKET_SEED, &market_id_bytes, &[market.bump]];
    
    // Transfer USDC from user to market vault (1 USDC per complete set)
    invoke(
        &spl_token::instruction::transfer(
            token_program_info.key,
            user_usdc_info.key,
            market_vault_info.key,
            user_info.key,
            &[],
            args.amount,
        )?,
        &[user_usdc_info.clone(), market_vault_info.clone(), user_info.clone()],
    )?;
    
    // Mint 1 token of each outcome to user
    for outcome_idx in 0..market.num_outcomes {
        let outcome_mint_info = next_account_info(account_info_iter)?;
        let user_outcome_token_info = next_account_info(account_info_iter)?;
        
        // Verify outcome mint PDA
        let (outcome_mint_pda, _) = Pubkey::find_program_address(
            &[OUTCOME_MINT_SEED, &market_id_bytes, &[outcome_idx]],
            program_id,
        );
        if *outcome_mint_info.key != outcome_mint_pda {
            msg!("Invalid outcome mint for index {}", outcome_idx);
            return Err(PredictionMarketError::InvalidPDA.into());
        }
        
        // Mint tokens to user
        invoke_signed(
            &spl_token::instruction::mint_to(
                token_program_info.key,
                outcome_mint_info.key,
                user_outcome_token_info.key,
                market_info.key, // mint authority
                &[],
                args.amount,
            )?,
            &[outcome_mint_info.clone(), user_outcome_token_info.clone(), market_info.clone()],
            &[market_seeds],
        )?;
    }
    
    // Update market stats
    market.total_minted += args.amount;
    market.updated_at = get_current_timestamp()?;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    msg!("Multi-outcome complete set minted");
    msg!("Market ID: {}", args.market_id);
    msg!("Amount: {}", args.amount);
    msg!("User: {}", user_info.key);
    
    Ok(())
}

fn process_redeem_multi_outcome_complete_set(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RedeemMultiOutcomeCompleteSetArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: User (signer)
    let user_info = next_account_info(account_info_iter)?;
    check_signer(user_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: Market Vault
    let market_vault_info = next_account_info(account_info_iter)?;
    
    // Account 4: User USDC Account
    let user_usdc_info = next_account_info(account_info_iter)?;
    
    // Account 5: User Position PDA
    let _position_info = next_account_info(account_info_iter)?;
    
    // Account 6: Token Program
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
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    if market.market_type != MarketType::MultiOutcome {
        msg!("This market is not a multi-outcome market");
        return Err(PredictionMarketError::InvalidArgument.into());
    }
    if !market.is_tradeable() {
        return Err(PredictionMarketError::MarketNotActive.into());
    }
    
    // Validate amount
    if args.amount == 0 {
        return Err(PredictionMarketError::InvalidAmount.into());
    }
    
    let market_id_bytes = args.market_id.to_le_bytes();
    let market_seeds: &[&[u8]] = &[MARKET_SEED, &market_id_bytes, &[market.bump]];
    
    // Burn 1 token of each outcome from user
    for outcome_idx in 0..market.num_outcomes {
        let outcome_mint_info = next_account_info(account_info_iter)?;
        let user_outcome_token_info = next_account_info(account_info_iter)?;
        
        // Verify outcome mint PDA
        let (outcome_mint_pda, _) = Pubkey::find_program_address(
            &[OUTCOME_MINT_SEED, &market_id_bytes, &[outcome_idx]],
            program_id,
        );
        if *outcome_mint_info.key != outcome_mint_pda {
            msg!("Invalid outcome mint for index {}", outcome_idx);
            return Err(PredictionMarketError::InvalidPDA.into());
        }
        
        // Burn tokens from user
        invoke(
            &spl_token::instruction::burn(
                token_program_info.key,
                user_outcome_token_info.key,
                outcome_mint_info.key,
                user_info.key,
                &[],
                args.amount,
            )?,
            &[user_outcome_token_info.clone(), outcome_mint_info.clone(), user_info.clone()],
        )?;
    }
    
    // Transfer USDC from market vault to user
    invoke_signed(
        &spl_token::instruction::transfer(
            token_program_info.key,
            market_vault_info.key,
            user_usdc_info.key,
            market_info.key, // vault authority
            &[],
            args.amount,
        )?,
        &[market_vault_info.clone(), user_usdc_info.clone(), market_info.clone()],
        &[market_seeds],
    )?;
    
    // Update market stats
    market.total_minted = market.total_minted.saturating_sub(args.amount);
    market.updated_at = get_current_timestamp()?;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    msg!("Multi-outcome complete set redeemed");
    msg!("Market ID: {}", args.market_id);
    msg!("Amount: {}", args.amount);
    msg!("User: {}", user_info.key);
    
    Ok(())
}

fn process_place_multi_outcome_order(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: PlaceMultiOutcomeOrderArgs,
) -> ProgramResult {
    use crate::state::{Order, OrderSide, Outcome, OrderType, OrderStatus, ORDER_DISCRIMINATOR};
    
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: User (signer)
    let user_info = next_account_info(account_info_iter)?;
    check_signer(user_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: Order PDA
    let order_info = next_account_info(account_info_iter)?;
    
    // Account 4: User Position PDA
    let _position_info = next_account_info(account_info_iter)?;
    
    // Account 5: Outcome Mint
    let _outcome_mint_info = next_account_info(account_info_iter)?;
    
    // Account 6: User Outcome Token Account
    let _user_outcome_token_info = next_account_info(account_info_iter)?;
    
    // Account 7: Token Program
    let _token_program_info = next_account_info(account_info_iter)?;
    
    // Account 8: System Program
    let system_program_info = next_account_info(account_info_iter)?;
    
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
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    if market.market_type != MarketType::MultiOutcome {
        msg!("This market is not a multi-outcome market");
        return Err(PredictionMarketError::InvalidArgument.into());
    }
    if !market.is_tradeable() {
        return Err(PredictionMarketError::MarketNotActive.into());
    }
    
    // Validate outcome index
    if args.outcome_index >= market.num_outcomes {
        msg!("Invalid outcome index: {} >= {}", args.outcome_index, market.num_outcomes);
        return Err(PredictionMarketError::InvalidArgument.into());
    }
    
    // Validate price
    if args.price < MIN_PRICE || args.price > MAX_PRICE {
        return Err(PredictionMarketError::InvalidPrice.into());
    }
    
    // Validate amount
    if args.amount == 0 {
        return Err(PredictionMarketError::InvalidAmount.into());
    }
    
    let order_id = market.next_order_id;
    let market_id_bytes = args.market_id.to_le_bytes();
    let order_id_bytes = order_id.to_le_bytes();
    let current_time = get_current_timestamp()?;
    
    // Verify Order PDA
    let (order_pda, order_bump) = Pubkey::find_program_address(
        &[ORDER_SEED, &market_id_bytes, &order_id_bytes],
        program_id,
    );
    if *order_info.key != order_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Create order account
    let rent = Rent::get()?;
    let order_space = Order::SIZE;
    let order_lamports = rent.minimum_balance(order_space);
    let order_seeds: &[&[u8]] = &[ORDER_SEED, &market_id_bytes, &order_id_bytes, &[order_bump]];
    
    invoke_signed(
        &system_instruction::create_account(
            user_info.key,
            order_info.key,
            order_lamports,
            order_space as u64,
            program_id,
        ),
        &[user_info.clone(), order_info.clone(), system_program_info.clone()],
        &[order_seeds],
    )?;
    
    // Map outcome_index to Outcome enum (for compatibility)
    let outcome = if args.outcome_index == 0 { Outcome::Yes } else { Outcome::No };
    
    // Initialize order
    let order = Order {
        discriminator: ORDER_DISCRIMINATOR,
        order_id,
        market_id: args.market_id,
        owner: *user_info.key,
        side: args.side,
        outcome,
        outcome_index: args.outcome_index,
        price: args.price,
        amount: args.amount,
        filled_amount: 0,
        status: OrderStatus::Open,
        order_type: OrderType::GTC,
        expiration_time: args.expiration_time,
        created_at: current_time,
        updated_at: current_time,
        bump: order_bump,
        escrow_token_account: None,
        reserved: [0u8; 30],
    };
    
    order.serialize(&mut *order_info.data.borrow_mut())?;
    
    // Update market
    market.next_order_id += 1;
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    msg!("Multi-outcome order placed");
    msg!("Market ID: {}", args.market_id);
    msg!("Order ID: {}", order_id);
    msg!("Outcome Index: {}", args.outcome_index);
    msg!("Side: {:?}", args.side);
    msg!("Price: {}", args.price);
    msg!("Amount: {}", args.amount);
    
    Ok(())
}

fn process_propose_multi_outcome_result(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: ProposeMultiOutcomeResultArgs,
) -> ProgramResult {
    use crate::state::{OracleProposal, ProposalStatus, ORACLE_PROPOSAL_DISCRIMINATOR};
    
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Oracle Admin (signer)
    let oracle_admin_info = next_account_info(account_info_iter)?;
    check_signer(oracle_admin_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: Oracle Proposal PDA
    let proposal_info = next_account_info(account_info_iter)?;
    
    // Account 4: System Program
    let system_program_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Verify oracle admin
    if *oracle_admin_info.key != config.oracle_admin {
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
    if market.market_type != MarketType::MultiOutcome {
        msg!("This market is not a multi-outcome market");
        return Err(PredictionMarketError::InvalidArgument.into());
    }
    
    // Validate winning outcome index
    if args.winning_outcome_index >= market.num_outcomes {
        msg!("Invalid winning outcome index: {} >= {}", args.winning_outcome_index, market.num_outcomes);
        return Err(PredictionMarketError::InvalidArgument.into());
    }
    
    let current_time = get_current_timestamp()?;
    
    // Check if market can be resolved
    if !market.can_resolve(current_time) {
        return Err(PredictionMarketError::MarketNotResolvable.into());
    }
    
    let market_id_bytes = args.market_id.to_le_bytes();
    
    // Verify proposal PDA
    let (proposal_pda, proposal_bump) = Pubkey::find_program_address(
        &[ORACLE_PROPOSAL_SEED, &market_id_bytes],
        program_id,
    );
    if *proposal_info.key != proposal_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Create proposal account
    let rent = Rent::get()?;
    let proposal_space = OracleProposal::SIZE;
    let proposal_lamports = rent.minimum_balance(proposal_space);
    let proposal_seeds: &[&[u8]] = &[ORACLE_PROPOSAL_SEED, &market_id_bytes, &[proposal_bump]];
    
    invoke_signed(
        &system_instruction::create_account(
            oracle_admin_info.key,
            proposal_info.key,
            proposal_lamports,
            proposal_space as u64,
            program_id,
        ),
        &[oracle_admin_info.clone(), proposal_info.clone(), system_program_info.clone()],
        &[proposal_seeds],
    )?;
    
    // Initialize proposal (use MarketResult::Yes as placeholder since we're using winning_outcome_index)
    let proposal = OracleProposal {
        discriminator: ORACLE_PROPOSAL_DISCRIMINATOR,
        market_id: args.market_id,
        proposer: *oracle_admin_info.key,
        proposed_result: MarketResult::Yes, // For multi-outcome, use winning_outcome_index instead
        status: ProposalStatus::Pending,
        proposed_at: current_time,
        challenge_deadline: current_time + config.challenge_window_secs,
        bond_amount: config.proposer_bond_e6,
        challenger: None,
        challenger_result: None,
        challenger_bond: 0,
        bump: proposal_bump,
        reserved: [0u8; 32],
    };
    
    proposal.serialize(&mut *proposal_info.data.borrow_mut())?;
    
    // Update market with winning outcome index
    market.winning_outcome_index = Some(args.winning_outcome_index);
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    msg!("Multi-outcome result proposed");
    msg!("Market ID: {}", args.market_id);
    msg!("Winning Outcome Index: {}", args.winning_outcome_index);
    msg!("Challenge Deadline: {}", current_time + config.challenge_window_secs);
    
    Ok(())
}

fn process_claim_multi_outcome_winnings(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: ClaimMultiOutcomeWinningsArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: User (signer)
    let user_info = next_account_info(account_info_iter)?;
    check_signer(user_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: User Position PDA
    let _position_info = next_account_info(account_info_iter)?;
    
    // Account 4: User Winning Outcome Token Account
    let user_winning_token_info = next_account_info(account_info_iter)?;
    
    // Account 5: Winning Outcome Mint
    let winning_mint_info = next_account_info(account_info_iter)?;
    
    // Account 6: Market Vault
    let market_vault_info = next_account_info(account_info_iter)?;
    
    // Account 7: User USDC Account
    let user_usdc_info = next_account_info(account_info_iter)?;
    
    // Account 8: Token Program
    let token_program_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Load and validate market
    let market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    if market.market_type != MarketType::MultiOutcome {
        msg!("This market is not a multi-outcome market");
        return Err(PredictionMarketError::InvalidArgument.into());
    }
    
    // Verify market is resolved
    if market.status != MarketStatus::Resolved {
        return Err(PredictionMarketError::MarketNotResolved.into());
    }
    
    // Get winning outcome
    let winning_outcome_index = market.winning_outcome_index
        .ok_or(PredictionMarketError::MarketNotResolved)?;
    
    let market_id_bytes = args.market_id.to_le_bytes();
    let market_seeds: &[&[u8]] = &[MARKET_SEED, &market_id_bytes, &[market.bump]];
    
    // Verify winning mint PDA
    let (winning_mint_pda, _) = Pubkey::find_program_address(
        &[OUTCOME_MINT_SEED, &market_id_bytes, &[winning_outcome_index]],
        program_id,
    );
    if *winning_mint_info.key != winning_mint_pda {
        msg!("Invalid winning mint PDA");
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Get user's winning token balance
    let user_winning_token_account = spl_token::state::Account::unpack(
        &user_winning_token_info.data.borrow()
    )?;
    let winning_amount = user_winning_token_account.amount;
    
    if winning_amount == 0 {
        msg!("No winning tokens to claim");
        return Err(PredictionMarketError::InvalidAmount.into());
    }
    
    // Burn user's winning tokens
    invoke(
        &spl_token::instruction::burn(
            token_program_info.key,
            user_winning_token_info.key,
            winning_mint_info.key,
            user_info.key,
            &[],
            winning_amount,
        )?,
        &[user_winning_token_info.clone(), winning_mint_info.clone(), user_info.clone()],
    )?;
    
    // Transfer USDC from market vault to user (1 USDC per winning token)
    invoke_signed(
        &spl_token::instruction::transfer(
            token_program_info.key,
            market_vault_info.key,
            user_usdc_info.key,
            market_info.key, // vault authority
            &[],
            winning_amount,
        )?,
        &[market_vault_info.clone(), user_usdc_info.clone(), market_info.clone()],
        &[market_seeds],
    )?;
    
    msg!("Multi-outcome winnings claimed");
    msg!("Market ID: {}", args.market_id);
    msg!("Winning Outcome Index: {}", winning_outcome_index);
    msg!("Amount: {}", winning_amount);
    msg!("User: {}", user_info.key);
    
    Ok(())
}

// ============================================================================
// Relayer Instructions - Admin/Relayer 代替用户签名
// ============================================================================

/// 验证调用者是否为 Admin 或授权的 Relayer
fn verify_relayer(config: &PredictionMarketConfig, relayer: &Pubkey) -> Result<(), ProgramError> {
    // Admin 和 Oracle Admin 都可以作为 Relayer
    if relayer == &config.admin || relayer == &config.oracle_admin {
        return Ok(());
    }
    msg!("Error: Caller {} is not an authorized relayer", relayer);
    Err(PredictionMarketError::Unauthorized.into())
}

/// Relayer 版本的 MintCompleteSet
/// Relayer 代替用户签名，从用户的 Vault 余额扣款，铸造代币给用户
fn process_relayer_mint_complete_set(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RelayerMintCompleteSetArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Account 0: Relayer (signer) - 代替用户签名
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    // Account 1: PredictionMarketConfig
    let config_info = next_account_info(account_info_iter)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    
    // Account 3: Market Vault (writable)
    let market_vault_info = next_account_info(account_info_iter)?;
    
    // Account 4: YES Token Mint (writable)
    let yes_mint_info = next_account_info(account_info_iter)?;
    
    // Account 5: NO Token Mint (writable)
    let no_mint_info = next_account_info(account_info_iter)?;
    
    // Account 6: User's YES Token Account (writable)
    let user_yes_info = next_account_info(account_info_iter)?;
    
    // Account 7: User's NO Token Account (writable)
    let user_no_info = next_account_info(account_info_iter)?;
    
    // Account 8: Position PDA (writable)
    let position_info = next_account_info(account_info_iter)?;
    
    // Account 9: User Vault Account (Vault Program) - 用于扣款
    let user_vault_info = next_account_info(account_info_iter)?;
    
    // Account 10: Vault Config
    let vault_config_info = next_account_info(account_info_iter)?;
    
    // Account 11: Vault Program
    let vault_program_info = next_account_info(account_info_iter)?;
    
    // Account 12: Token Program
    let token_program_info = next_account_info(account_info_iter)?;
    
    // Account 13: System Program
    let system_program_info = next_account_info(account_info_iter)?;
    
    // Load and validate config
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // 验证 Relayer 权限
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
    let market_seeds: &[&[u8]] = &[MARKET_SEED, &market_id_bytes, &[market.bump]];
    
    // TODO: CPI to Vault Program to deduct from user's vault balance
    // For now, we skip the USDC transfer as it will be handled differently
    msg!("⚠️ Relayer mint: Vault CPI deduction to be implemented");
    
    // Mint YES tokens to user
    invoke_signed(
        &spl_token::instruction::mint_to(
            token_program_info.key,
            yes_mint_info.key,
            user_yes_info.key,
            market_info.key,
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
            market_info.key,
            &[],
            args.amount,
        )?,
        &[no_mint_info.clone(), user_no_info.clone(), market_info.clone(), token_program_info.clone()],
        &[market_seeds],
    )?;
    
    // Load or create Position - Relayer pays for account creation
    let (position_pda, position_bump) = Pubkey::find_program_address(
        &[POSITION_SEED, &market_id_bytes, args.user_wallet.as_ref()],
        program_id,
    );
    
    if *position_info.key != position_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    if position_info.data_is_empty() {
        let rent = Rent::get()?;
        let space = Position::SIZE;
        let lamports = rent.minimum_balance(space);
        let position_seeds: &[&[u8]] = &[POSITION_SEED, &market_id_bytes, args.user_wallet.as_ref(), &[position_bump]];
        
        // Relayer pays for account creation
        invoke_signed(
            &system_instruction::create_account(
                relayer_info.key,  // Relayer pays
                position_info.key,
                lamports,
                space as u64,
                program_id,
            ),
            &[relayer_info.clone(), position_info.clone(), system_program_info.clone()],
            &[position_seeds],
        )?;
        
        let position = Position::new(market.market_id, args.user_wallet, position_bump, current_time);
        let mut data = position_info.try_borrow_mut_data()?;
        position.serialize(&mut data.as_mut())?;
    }
    
    // Update market stats
    market.total_minted = safe_add_u64(market.total_minted, args.amount)?;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    msg!("✅ RelayerMintCompleteSet completed");
    msg!("User: {}", args.user_wallet);
    msg!("Market ID: {}", args.market_id);
    msg!("Amount: {}", args.amount);
    
    Ok(())
}

/// Relayer 版本的 RedeemCompleteSet
fn process_relayer_redeem_complete_set(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RelayerRedeemCompleteSetArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    let config_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let market_vault_info = next_account_info(account_info_iter)?;
    let yes_mint_info = next_account_info(account_info_iter)?;
    let no_mint_info = next_account_info(account_info_iter)?;
    let user_yes_info = next_account_info(account_info_iter)?;
    let user_no_info = next_account_info(account_info_iter)?;
    let position_info = next_account_info(account_info_iter)?;
    let user_vault_info = next_account_info(account_info_iter)?;
    let vault_config_info = next_account_info(account_info_iter)?;
    let vault_program_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    verify_relayer(&config, relayer_info.key)?;
    
    if config.is_paused {
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    if args.amount == 0 {
        return Err(PredictionMarketError::InvalidAmount.into());
    }
    
    let market_id_bytes = market.market_id.to_le_bytes();
    let market_seeds: &[&[u8]] = &[MARKET_SEED, &market_id_bytes, &[market.bump]];
    
    // Burn YES tokens from user (user already authorized via Relayer)
    // Note: This requires the token account to have delegated authority to the market
    // For simplicity, we use the market as authority via PDA signing
    invoke_signed(
        &spl_token::instruction::burn(
            token_program_info.key,
            user_yes_info.key,
            yes_mint_info.key,
            market_info.key,  // Market as burn authority
            &[],
            args.amount,
        )?,
        &[user_yes_info.clone(), yes_mint_info.clone(), market_info.clone()],
        &[market_seeds],
    )?;
    
    // Burn NO tokens
    invoke_signed(
        &spl_token::instruction::burn(
            token_program_info.key,
            user_no_info.key,
            no_mint_info.key,
            market_info.key,
            &[],
            args.amount,
        )?,
        &[user_no_info.clone(), no_mint_info.clone(), market_info.clone()],
        &[market_seeds],
    )?;
    
    // TODO: CPI to Vault to credit user's balance
    msg!("⚠️ Relayer redeem: Vault CPI credit to be implemented");
    
    market.total_minted = market.total_minted.saturating_sub(args.amount);
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    msg!("✅ RelayerRedeemCompleteSet completed");
    msg!("User: {}", args.user_wallet);
    msg!("Amount: {}", args.amount);
    
    Ok(())
}

/// Relayer 版本的 PlaceOrder
fn process_relayer_place_order(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RelayerPlaceOrderArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    let config_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let order_info = next_account_info(account_info_iter)?;
    let position_info = next_account_info(account_info_iter)?;
    let user_vault_info = next_account_info(account_info_iter)?;
    let vault_config_info = next_account_info(account_info_iter)?;
    let vault_program_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;
    
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    verify_relayer(&config, relayer_info.key)?;
    
    if config.is_paused {
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    if !market.is_tradeable() {
        return Err(PredictionMarketError::MarketNotTradeable.into());
    }
    
    // Validate price
    validate_price(args.price)?;
    
    let current_time = get_current_timestamp()?;
    let market_id_bytes = args.market_id.to_le_bytes();
    
    // Get next order ID
    let order_id = market.next_order_id;
    market.next_order_id = market.next_order_id.saturating_add(1);
    
    // Derive Order PDA
    let order_id_bytes = order_id.to_le_bytes();
    let (order_pda, order_bump) = Pubkey::find_program_address(
        &[ORDER_SEED, &market_id_bytes, &order_id_bytes],
        program_id,
    );
    
    if *order_info.key != order_pda {
        return Err(PredictionMarketError::InvalidPDA.into());
    }
    
    // Create Order account - Relayer pays
    let rent = Rent::get()?;
    let space = Order::SIZE;
    let lamports = rent.minimum_balance(space);
    let order_seeds: &[&[u8]] = &[ORDER_SEED, &market_id_bytes, &order_id_bytes, &[order_bump]];
    
    invoke_signed(
        &system_instruction::create_account(
            relayer_info.key,
            order_info.key,
            lamports,
            space as u64,
            program_id,
        ),
        &[relayer_info.clone(), order_info.clone(), system_program_info.clone()],
        &[order_seeds],
    )?;
    
    // Derive outcome_index from outcome for binary markets
    let outcome_index = match args.outcome {
        Outcome::Yes => 0u8,
        Outcome::No => 1u8,
    };
    
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
        escrow_token_account: None,
        reserved: [0u8; 30],
    };
    
    order.serialize(&mut *order_info.data.borrow_mut())?;
    
    // Update market stats
    market.open_interest = market.open_interest.saturating_add(1);
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    // TODO: Lock margin via Vault CPI
    msg!("⚠️ Relayer place order: Vault margin lock to be implemented");
    
    msg!("✅ RelayerPlaceOrder completed");
    msg!("User: {}", args.user_wallet);
    msg!("Order ID: {}", order_id);
    msg!("Side: {:?}, Outcome: {:?}", args.side, args.outcome);
    msg!("Price: {}, Amount: {}", args.price, args.amount);
    
    Ok(())
}

/// Relayer 版本的 CancelOrder
fn process_relayer_cancel_order(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RelayerCancelOrderArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    let config_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let order_info = next_account_info(account_info_iter)?;
    let user_vault_info = next_account_info(account_info_iter)?;
    let vault_config_info = next_account_info(account_info_iter)?;
    let vault_program_info = next_account_info(account_info_iter)?;
    
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    verify_relayer(&config, relayer_info.key)?;
    
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    let mut order = deserialize_account::<Order>(&order_info.data.borrow())?;
    
    // Verify order belongs to user
    if order.owner != args.user_wallet {
        msg!("Order does not belong to user");
        return Err(PredictionMarketError::Unauthorized.into());
    }
    
    if order.status != OrderStatus::Open {
        msg!("Order is not open");
        return Err(PredictionMarketError::InvalidAmount.into());
    }
    
    // Cancel order
    order.status = OrderStatus::Cancelled;
    order.updated_at = get_current_timestamp()?;
    order.serialize(&mut *order_info.data.borrow_mut())?;
    
    // Update market stats
    market.open_interest = market.open_interest.saturating_sub(1);
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    // TODO: Release margin via Vault CPI
    msg!("⚠️ Relayer cancel order: Vault margin release to be implemented");
    
    msg!("✅ RelayerCancelOrder completed");
    msg!("User: {}", args.user_wallet);
    msg!("Order ID: {}", args.order_id);
    
    Ok(())
}

/// Relayer 版本的 ClaimWinnings
fn process_relayer_claim_winnings(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RelayerClaimWinningsArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    let config_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let position_info = next_account_info(account_info_iter)?;
    let user_token_info = next_account_info(account_info_iter)?;
    let token_mint_info = next_account_info(account_info_iter)?;
    let market_vault_info = next_account_info(account_info_iter)?;
    let user_vault_info = next_account_info(account_info_iter)?;
    let vault_config_info = next_account_info(account_info_iter)?;
    let vault_program_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    verify_relayer(&config, relayer_info.key)?;
    
    let market = deserialize_account::<Market>(&market_info.data.borrow())?;
    if market.market_id != args.market_id {
        return Err(PredictionMarketError::MarketNotFound.into());
    }
    
    if market.status != MarketStatus::Resolved {
        return Err(PredictionMarketError::MarketNotResolved.into());
    }
    
    // Get user's winning token balance and burn
    let user_token_account = spl_token::state::Account::unpack(&user_token_info.data.borrow())?;
    let winning_amount = user_token_account.amount;
    
    if winning_amount == 0 {
        return Err(PredictionMarketError::InvalidAmount.into());
    }
    
    let market_id_bytes = args.market_id.to_le_bytes();
    let market_seeds: &[&[u8]] = &[MARKET_SEED, &market_id_bytes, &[market.bump]];
    
    // Burn winning tokens (market as authority)
    invoke_signed(
        &spl_token::instruction::burn(
            token_program_info.key,
            user_token_info.key,
            token_mint_info.key,
            market_info.key,
            &[],
            winning_amount,
        )?,
        &[user_token_info.clone(), token_mint_info.clone(), market_info.clone()],
        &[market_seeds],
    )?;
    
    // TODO: Credit user's Vault balance via CPI
    msg!("⚠️ Relayer claim: Vault credit to be implemented");
    
    msg!("✅ RelayerClaimWinnings completed");
    msg!("User: {}", args.user_wallet);
    msg!("Amount: {}", winning_amount);
    
    Ok(())
}

/// Relayer 版本的 RefundCancelledMarket
fn process_relayer_refund_cancelled_market(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RelayerRefundCancelledMarketArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    let config_info = next_account_info(account_info_iter)?;
    
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    verify_relayer(&config, relayer_info.key)?;
    
    // Similar logic to RefundCancelledMarket but with Relayer as signer
    msg!("✅ RelayerRefundCancelledMarket - implementation pending full Vault CPI");
    msg!("User: {}", args.user_wallet);
    msg!("Market ID: {}", args.market_id);
    
    Ok(())
}

/// Relayer 版本的 MintMultiOutcomeCompleteSet
fn process_relayer_mint_multi_outcome_complete_set(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RelayerMintMultiOutcomeCompleteSetArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    let config_info = next_account_info(account_info_iter)?;
    
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    verify_relayer(&config, relayer_info.key)?;
    
    // Similar logic to MintMultiOutcomeCompleteSet
    msg!("✅ RelayerMintMultiOutcomeCompleteSet - implementation pending");
    msg!("User: {}", args.user_wallet);
    msg!("Market ID: {}", args.market_id);
    msg!("Amount: {}", args.amount);
    
    Ok(())
}

/// Relayer 版本的 RedeemMultiOutcomeCompleteSet
fn process_relayer_redeem_multi_outcome_complete_set(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RelayerRedeemMultiOutcomeCompleteSetArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    let config_info = next_account_info(account_info_iter)?;
    
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    verify_relayer(&config, relayer_info.key)?;
    
    msg!("✅ RelayerRedeemMultiOutcomeCompleteSet - implementation pending");
    msg!("User: {}", args.user_wallet);
    msg!("Market ID: {}", args.market_id);
    msg!("Amount: {}", args.amount);
    
    Ok(())
}

/// Relayer 版本的 PlaceMultiOutcomeOrder
fn process_relayer_place_multi_outcome_order(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RelayerPlaceMultiOutcomeOrderArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    let config_info = next_account_info(account_info_iter)?;
    
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    verify_relayer(&config, relayer_info.key)?;
    
    msg!("✅ RelayerPlaceMultiOutcomeOrder - implementation pending");
    msg!("User: {}", args.user_wallet);
    msg!("Market ID: {}", args.market_id);
    msg!("Outcome Index: {}", args.outcome_index);
    
    Ok(())
}

/// Relayer 版本的 ClaimMultiOutcomeWinnings
fn process_relayer_claim_multi_outcome_winnings(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RelayerClaimMultiOutcomeWinningsArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let relayer_info = next_account_info(account_info_iter)?;
    check_signer(relayer_info)?;
    
    let config_info = next_account_info(account_info_iter)?;
    
    let config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    verify_relayer(&config, relayer_info.key)?;
    
    msg!("✅ RelayerClaimMultiOutcomeWinnings - implementation pending");
    msg!("User: {}", args.user_wallet);
    msg!("Market ID: {}", args.market_id);
    
    Ok(())
}

// ============================================================================
// Multi-Outcome Matching Operations
// ============================================================================

/// Process MatchMintMulti instruction
/// 
/// Complete Set Mint for multi-outcome market:
/// When sum of all outcome buy prices <= 1.0, mint one token of each outcome.
/// 
/// Account layout:
/// 0. [signer] Authorized Caller (Relayer/Matcher)
/// 1. [writable] PredictionMarketConfig
/// 2. [writable] Market
/// 3. [] Market Vault
/// 4. [] Token Program
/// 5. [] System Program
/// Dynamic accounts (3 per outcome, for i in 0..num_outcomes):
///   6 + 3*i + 0: [writable] Order PDA
///   6 + 3*i + 1: [writable] Outcome Token Mint
///   6 + 3*i + 2: [writable] Buyer's Token Account
fn process_match_mint_multi(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: MatchMintMultiArgs,
) -> ProgramResult {
    use crate::state::MAX_OUTCOMES_FOR_MATCH;
    
    let account_info_iter = &mut accounts.iter();
    
    // ========== Fixed Accounts (6) ==========
    
    // Account 0: Authorized Caller (signer)
    let caller_info = next_account_info(account_info_iter)?;
    check_signer(caller_info)?;
    
    // Account 1: PredictionMarketConfig (writable)
    let config_info = next_account_info(account_info_iter)?;
    let mut config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if config.is_paused {
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    // Verify authorized caller
    verify_authorized_caller(&config, caller_info.key)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Validate market
    if !market.is_tradeable() {
        return Err(PredictionMarketError::MarketNotTradeable.into());
    }
    
    // Verify market is multi-outcome type
    if market.market_type != MarketType::MultiOutcome {
        msg!("Error: MatchMintMulti requires MultiOutcome market type");
        return Err(PredictionMarketError::InvalidMarketType.into());
    }
    
    // Validate num_outcomes
    if args.num_outcomes < 2 || args.num_outcomes > MAX_OUTCOMES_FOR_MATCH {
        msg!("Invalid num_outcomes: {}, max is {}", args.num_outcomes, MAX_OUTCOMES_FOR_MATCH);
        return Err(PredictionMarketError::InvalidArgument.into());
    }
    
    // Validate num_outcomes matches market
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
    
    // Account 3: Market Vault (not used in mint, but validated)
    let _market_vault_info = next_account_info(account_info_iter)?;
    
    // Account 4: Token Program
    let token_program_info = next_account_info(account_info_iter)?;
    
    // Account 5: System Program
    let _system_program_info = next_account_info(account_info_iter)?;
    
    // ========== Dynamic Accounts (3 per outcome) ==========
    
    let market_id_bytes = args.market_id.to_le_bytes();
    let current_time = get_current_timestamp()?;
    
    // Calculate market PDA seeds for signing (market is mint authority)
    let market_seeds: &[&[u8]] = &[MARKET_SEED, &market_id_bytes, &[market.bump]];
    
    // Track minimum matchable amount across all orders
    let mut min_remaining: u64 = args.amount;
    
    // Collect order and account info for processing
    struct OutcomeData<'a> {
        order_info: AccountInfo<'a>,
        mint_info: AccountInfo<'a>,
        buyer_token_info: AccountInfo<'a>,
        order: Order,
        outcome_index: u8,
        price: u64,
    }
    
    let mut outcome_data: Vec<OutcomeData> = Vec::with_capacity(args.num_outcomes as usize);
    
    // Parse and validate dynamic accounts
    for i in 0..args.num_outcomes as usize {
        let (expected_outcome_idx, order_id, price) = args.orders[i];
        
        // Verify outcome_index is sequential (0, 1, 2, ...)
        if expected_outcome_idx != i as u8 {
            msg!("Error: outcome_index {} at position {} (expected {})", expected_outcome_idx, i, i);
            return Err(PredictionMarketError::InvalidOutcome.into());
        }
        
        // Parse accounts for this outcome
        let order_info = next_account_info(account_info_iter)?;
        let mint_info = next_account_info(account_info_iter)?;
        let buyer_token_info = next_account_info(account_info_iter)?;
        
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
        let order = deserialize_account::<Order>(&order_info.data.borrow())?;
        
        if order.discriminator != ORDER_DISCRIMINATOR {
            return Err(PredictionMarketError::InvalidAccountData.into());
        }
        
        // Verify order is a Buy order
        if order.side != crate::state::OrderSide::Buy {
            msg!("Error: Order {} must be Buy order for MatchMintMulti", order_id);
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
        
        // Verify price is <= order's limit price (for buy, lower is better for matcher)
        if price > order.price {
            msg!("Error: Match price {} exceeds order limit price {}", price, order.price);
            return Err(PredictionMarketError::PriceExceedsLimit.into());
        }
        
        // Verify Outcome Mint PDA
        let (outcome_mint_pda, _) = Pubkey::find_program_address(
            &[OUTCOME_MINT_SEED, &market_id_bytes, &[expected_outcome_idx]],
            program_id,
        );
        if *mint_info.key != outcome_mint_pda {
            msg!("Error: Invalid Outcome Mint PDA for outcome {}", i);
            return Err(PredictionMarketError::InvalidPDA.into());
        }
        
        // Track minimum remaining amount
        let remaining = order.remaining_amount();
        if remaining < min_remaining {
            min_remaining = remaining;
        }
        
        outcome_data.push(OutcomeData {
            order_info: order_info.clone(),
            mint_info: mint_info.clone(),
            buyer_token_info: buyer_token_info.clone(),
            order,
            outcome_index: expected_outcome_idx,
            price,
        });
    }
    
    // Calculate actual match amount
    let match_amount = min_remaining.min(args.amount);
    
    if match_amount == 0 {
        msg!("Error: No matchable amount");
        return Err(PredictionMarketError::NoMatchableAmount.into());
    }
    
    // ========== Execute: Mint tokens to each buyer ==========
    
    for data in outcome_data.iter() {
        // Mint outcome tokens to buyer
        invoke_signed(
            &spl_token::instruction::mint_to(
                token_program_info.key,
                data.mint_info.key,
                data.buyer_token_info.key,
                market_info.key, // Market PDA is the mint authority
                &[],
                match_amount,
            )?,
            &[
                data.mint_info.clone(),
                data.buyer_token_info.clone(),
                market_info.clone(),
                token_program_info.clone(),
            ],
            &[market_seeds],
        )?;
        
        msg!("Minted {} tokens for outcome {} to buyer", match_amount, data.outcome_index);
    }
    
    // ========== Update orders ==========
    
    for data in outcome_data.iter() {
        let mut order = data.order.clone();
        order.filled_amount += match_amount;
        
        if order.filled_amount >= order.amount {
            order.status = OrderStatus::Filled;
        } else {
            order.status = OrderStatus::PartialFilled;
        }
        order.updated_at = current_time;
        order.serialize(&mut *data.order_info.data.borrow_mut())?;
    }
    
    // ========== Update market stats ==========
    
    market.total_minted += match_amount;
    market.total_volume_e6 += match_amount as i64;
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    // ========== Update config stats ==========
    
    config.total_volume_e6 += match_amount as i64;
    config.total_minted_sets += match_amount;
    config.serialize(&mut *config_info.data.borrow_mut())?;
    
    // ========== Log success ==========
    
    msg!("✅ MatchMintMulti executed successfully");
    msg!("   Market ID: {}", args.market_id);
    msg!("   Num outcomes: {}", args.num_outcomes);
    msg!("   Match amount: {}", match_amount);
    msg!("   Total price: {} ({}%)", total_price, total_price * 100 / PRICE_PRECISION);
    
    // Log spread (protocol revenue)
    let spread = PRICE_PRECISION.saturating_sub(total_price);
    if spread > 0 {
        msg!("   Spread (protocol revenue): {} ({}%)", spread, spread * 100 / PRICE_PRECISION);
    }
    
    Ok(())
}

/// Process MatchBurnMulti instruction
/// 
/// Complete Set Burn for multi-outcome market:
/// When sum of all outcome sell prices >= 1.0, burn tokens and return USDC.
/// 
/// Account layout:
/// 0. [signer] Authorized Caller (Relayer/Matcher)
/// 1. [writable] PredictionMarketConfig
/// 2. [writable] Market
/// 3. [writable] Market Vault (USDC vault)
/// 4. [] Token Program
/// 5. [] System Program
/// Dynamic accounts (4 per outcome, for i in 0..num_outcomes):
///   6 + 4*i + 0: [writable] Order PDA
///   6 + 4*i + 1: [writable] Outcome Token Mint
///   6 + 4*i + 2: [writable] Seller's Token Account (Escrow)
///   6 + 4*i + 3: [writable] Seller's USDC Account
fn process_match_burn_multi(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: MatchBurnMultiArgs,
) -> ProgramResult {
    use crate::state::MAX_OUTCOMES_FOR_MATCH;
    use crate::utils::{verify_escrow_balance, verify_escrow_pda};
    
    let account_info_iter = &mut accounts.iter();
    
    // ========== Fixed Accounts (6) ==========
    
    // Account 0: Authorized Caller (signer)
    let caller_info = next_account_info(account_info_iter)?;
    check_signer(caller_info)?;
    
    // Account 1: PredictionMarketConfig (writable)
    let config_info = next_account_info(account_info_iter)?;
    let mut config = deserialize_account::<PredictionMarketConfig>(&config_info.data.borrow())?;
    
    if config.discriminator != PM_CONFIG_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    if config.is_paused {
        return Err(PredictionMarketError::ProgramPaused.into());
    }
    
    // Verify authorized caller
    verify_authorized_caller(&config, caller_info.key)?;
    
    // Account 2: Market (writable)
    let market_info = next_account_info(account_info_iter)?;
    let mut market = deserialize_account::<Market>(&market_info.data.borrow())?;
    
    if market.discriminator != MARKET_DISCRIMINATOR {
        return Err(PredictionMarketError::InvalidAccountData.into());
    }
    
    // Validate market
    if !market.is_tradeable() {
        return Err(PredictionMarketError::MarketNotTradeable.into());
    }
    
    // Verify market is multi-outcome type
    if market.market_type != MarketType::MultiOutcome {
        msg!("Error: MatchBurnMulti requires MultiOutcome market type");
        return Err(PredictionMarketError::InvalidMarketType.into());
    }
    
    // Validate num_outcomes
    if args.num_outcomes < 2 || args.num_outcomes > MAX_OUTCOMES_FOR_MATCH {
        msg!("Invalid num_outcomes: {}, max is {}", args.num_outcomes, MAX_OUTCOMES_FOR_MATCH);
        return Err(PredictionMarketError::InvalidArgument.into());
    }
    
    // Validate num_outcomes matches market
    if args.num_outcomes != market.num_outcomes {
        msg!("num_outcomes {} != market.num_outcomes {}", args.num_outcomes, market.num_outcomes);
        return Err(PredictionMarketError::InvalidArgument.into());
    }
    
    // Validate orders count
    if args.orders.len() != args.num_outcomes as usize {
        msg!("Orders count {} != num_outcomes {}", args.orders.len(), args.num_outcomes);
        return Err(PredictionMarketError::InvalidArgument.into());
    }
    
    // Validate price sum >= 1.0 (price conservation for burning)
    let total_price: u64 = args.orders.iter().map(|(_, _, p)| p).sum();
    if total_price < PRICE_PRECISION {
        msg!("Total price {} < 1.0 ({})", total_price, PRICE_PRECISION);
        return Err(PredictionMarketError::InvalidPricePair.into());
    }
    
    // Account 3: Market Vault (writable, for USDC transfers)
    let market_vault_info = next_account_info(account_info_iter)?;
    
    // Verify market vault
    if *market_vault_info.key != market.market_vault {
        return Err(PredictionMarketError::InvalidMarketVault.into());
    }
    
    // Account 4: Token Program
    let token_program_info = next_account_info(account_info_iter)?;
    
    // Account 5: System Program
    let _system_program_info = next_account_info(account_info_iter)?;
    
    // ========== Dynamic Accounts (4 per outcome) ==========
    
    let market_id_bytes = args.market_id.to_le_bytes();
    let current_time = get_current_timestamp()?;
    
    // Calculate market PDA seeds for signing (market controls vault)
    let market_seeds: &[&[u8]] = &[MARKET_SEED, &market_id_bytes, &[market.bump]];
    
    // Track minimum matchable amount across all orders
    let mut min_remaining: u64 = args.amount;
    
    // Collect order and account info for processing
    struct OutcomeData<'a> {
        order_info: AccountInfo<'a>,
        mint_info: AccountInfo<'a>,
        seller_token_info: AccountInfo<'a>,
        seller_usdc_info: AccountInfo<'a>,
        order: Order,
        order_id: u64,
        order_bump: u8,
        outcome_index: u8,
        price: u64,
    }
    
    let mut outcome_data: Vec<OutcomeData> = Vec::with_capacity(args.num_outcomes as usize);
    
    // Parse and validate dynamic accounts
    for i in 0..args.num_outcomes as usize {
        let (expected_outcome_idx, order_id, price) = args.orders[i];
        
        // Verify outcome_index is sequential (0, 1, 2, ...)
        if expected_outcome_idx != i as u8 {
            msg!("Error: outcome_index {} at position {} (expected {})", expected_outcome_idx, i, i);
            return Err(PredictionMarketError::InvalidOutcome.into());
        }
        
        // Parse accounts for this outcome
        let order_info = next_account_info(account_info_iter)?;
        let mint_info = next_account_info(account_info_iter)?;
        let seller_token_info = next_account_info(account_info_iter)?;
        let seller_usdc_info = next_account_info(account_info_iter)?;
        
        // Verify Order PDA
        let order_id_bytes = order_id.to_le_bytes();
        let (order_pda, order_bump) = Pubkey::find_program_address(
            &[ORDER_SEED, &market_id_bytes, &order_id_bytes],
            program_id,
        );
        if *order_info.key != order_pda {
            msg!("Error: Invalid Order PDA for outcome {}", i);
            return Err(PredictionMarketError::InvalidPDA.into());
        }
        
        // Load and validate order
        let order = deserialize_account::<Order>(&order_info.data.borrow())?;
        
        if order.discriminator != ORDER_DISCRIMINATOR {
            return Err(PredictionMarketError::InvalidAccountData.into());
        }
        
        // Verify order is a Sell order
        if order.side != crate::state::OrderSide::Sell {
            msg!("Error: Order {} must be Sell order for MatchBurnMulti", order_id);
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
        
        // Verify price is >= order's limit price (for sell, higher is better for seller)
        if price < order.price {
            msg!("Error: Match price {} below order limit price {}", price, order.price);
            return Err(PredictionMarketError::PriceBelowLimit.into());
        }
        
        // Verify sell order has escrow
        if !order.has_escrow() {
            msg!("Error: Sell order {} must have escrowed tokens", order_id);
            return Err(PredictionMarketError::EscrowNotFound.into());
        }
        
        // Verify Outcome Mint PDA
        let (outcome_mint_pda, _) = Pubkey::find_program_address(
            &[OUTCOME_MINT_SEED, &market_id_bytes, &[expected_outcome_idx]],
            program_id,
        );
        if *mint_info.key != outcome_mint_pda {
            msg!("Error: Invalid Outcome Mint PDA for outcome {}", i);
            return Err(PredictionMarketError::InvalidPDA.into());
        }
        
        // Verify escrow PDA
        verify_escrow_pda(
            seller_token_info,
            program_id,
            args.market_id,
            order_id,
        )?;
        
        // Track minimum remaining amount
        let remaining = order.remaining_amount();
        if remaining < min_remaining {
            min_remaining = remaining;
        }
        
        outcome_data.push(OutcomeData {
            order_info: order_info.clone(),
            mint_info: mint_info.clone(),
            seller_token_info: seller_token_info.clone(),
            seller_usdc_info: seller_usdc_info.clone(),
            order,
            order_id,
            order_bump,
            outcome_index: expected_outcome_idx,
            price,
        });
    }
    
    // Calculate actual match amount
    let match_amount = min_remaining.min(args.amount);
    
    if match_amount == 0 {
        msg!("Error: No matchable amount");
        return Err(PredictionMarketError::NoMatchableAmount.into());
    }
    
    // Verify all escrows have sufficient balance
    for data in outcome_data.iter() {
        verify_escrow_balance(&data.seller_token_info, match_amount)?;
    }
    
    // ========== Execute: Burn tokens and transfer USDC ==========
    
    for data in outcome_data.iter() {
        // Calculate order PDA seeds for signing (order PDA owns escrow)
        let order_id_bytes = data.order_id.to_le_bytes();
        let order_seeds: &[&[u8]] = &[
            ORDER_SEED,
            &market_id_bytes,
            &order_id_bytes,
            &[data.order_bump],
        ];
        
        // Burn outcome tokens from seller's escrow
        invoke_signed(
            &spl_token::instruction::burn(
                token_program_info.key,
                data.seller_token_info.key,
                data.mint_info.key,
                data.order_info.key, // Order PDA is the owner of escrow
                &[],
                match_amount,
            )?,
            &[
                data.seller_token_info.clone(),
                data.mint_info.clone(),
                data.order_info.clone(),
                token_program_info.clone(),
            ],
            &[order_seeds],
        )?;
        
        // Calculate USDC proceeds for this seller (proportional to price)
        let proceeds = ((match_amount as u128) * (data.price as u128) / (PRICE_PRECISION as u128)) as u64;
        
        // Transfer USDC from vault to seller
        invoke_signed(
            &spl_token::instruction::transfer(
                token_program_info.key,
                market_vault_info.key,
                data.seller_usdc_info.key,
                market_info.key, // Market PDA is the vault authority
                &[],
                proceeds,
            )?,
            &[
                market_vault_info.clone(),
                data.seller_usdc_info.clone(),
                market_info.clone(),
                token_program_info.clone(),
            ],
            &[market_seeds],
        )?;
        
        msg!("Burned {} tokens for outcome {}, paid {} USDC", 
             match_amount, data.outcome_index, proceeds);
    }
    
    // ========== Update orders ==========
    
    for data in outcome_data.iter() {
        let mut order = data.order.clone();
        order.filled_amount += match_amount;
        
        if order.filled_amount >= order.amount {
            order.status = OrderStatus::Filled;
        } else {
            order.status = OrderStatus::PartialFilled;
        }
        order.updated_at = current_time;
        order.serialize(&mut *data.order_info.data.borrow_mut())?;
    }
    
    // ========== Update market stats ==========
    
    market.total_minted = market.total_minted.saturating_sub(match_amount);
    market.total_volume_e6 += match_amount as i64;
    market.updated_at = current_time;
    market.serialize(&mut *market_info.data.borrow_mut())?;
    
    // ========== Update config stats ==========
    
    config.total_volume_e6 += match_amount as i64;
    config.serialize(&mut *config_info.data.borrow_mut())?;
    
    // ========== Log success ==========
    
    msg!("✅ MatchBurnMulti executed successfully");
    msg!("   Market ID: {}", args.market_id);
    msg!("   Num outcomes: {}", args.num_outcomes);
    msg!("   Match amount: {}", match_amount);
    msg!("   Total price: {} ({}%)", total_price, total_price * 100 / PRICE_PRECISION);
    
    // Log spread (protocol revenue from price > 1.0)
    let spread = total_price.saturating_sub(PRICE_PRECISION);
    if spread > 0 {
        msg!("   Spread (protocol revenue): {} ({}%)", spread, spread * 100 / PRICE_PRECISION);
    }
    
    Ok(())
}

/// Relayer version of MatchMintMulti
fn process_relayer_match_mint_multi(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RelayerMatchMintMultiArgs,
) -> ProgramResult {
    // Convert to regular args and call main implementation
    let match_args = MatchMintMultiArgs {
        market_id: args.market_id,
        num_outcomes: args.num_outcomes,
        amount: args.amount,
        orders: args.orders,
    };
    
    msg!("RelayerMatchMintMulti -> delegating to MatchMintMulti");
    process_match_mint_multi(program_id, accounts, match_args)
}

/// Relayer version of MatchBurnMulti
fn process_relayer_match_burn_multi(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: RelayerMatchBurnMultiArgs,
) -> ProgramResult {
    // Convert to regular args and call main implementation
    let match_args = MatchBurnMultiArgs {
        market_id: args.market_id,
        num_outcomes: args.num_outcomes,
        amount: args.amount,
        orders: args.orders,
    };
    
    msg!("RelayerMatchBurnMulti -> delegating to MatchBurnMulti");
    process_match_burn_multi(program_id, accounts, match_args)
}

/// Verify that caller is an authorized matching engine
/// 
/// Checks in order:
/// 1. Is caller the admin?
/// 2. Is caller the oracle_admin?
/// 3. If AuthorizedCallers account is provided, is caller in the list?
fn verify_authorized_caller(config: &PredictionMarketConfig, caller: &Pubkey) -> ProgramResult {
    // Check if caller is admin or oracle_admin (always authorized)
    if caller == &config.admin || caller == &config.oracle_admin {
        return Ok(());
    }
    
    msg!("Unauthorized caller: {}", caller);
    Err(PredictionMarketError::Unauthorized.into())
}

/// Verify that caller is an authorized matching engine with AuthorizedCallers PDA check
/// 
/// This version also checks the AuthorizedCallers registry
fn verify_authorized_caller_with_registry(
    config: &PredictionMarketConfig, 
    caller: &Pubkey,
    callers_info: Option<&AccountInfo>,
) -> ProgramResult {
    use crate::state::{AuthorizedCallers, AUTHORIZED_CALLERS_DISCRIMINATOR};
    
    // Check if caller is admin or oracle_admin (always authorized)
    if caller == &config.admin || caller == &config.oracle_admin {
        return Ok(());
    }
    
    // Check AuthorizedCallers registry if provided
    if let Some(callers_account) = callers_info {
        if callers_account.data_len() > 0 {
            if let Ok(callers) = deserialize_account::<AuthorizedCallers>(&callers_account.data.borrow()) {
                if callers.discriminator == AUTHORIZED_CALLERS_DISCRIMINATOR {
                    if callers.is_authorized(caller) {
                        return Ok(());
                    }
                }
            }
        }
    }
    
    msg!("Unauthorized caller: {}", caller);
    Err(PredictionMarketError::Unauthorized.into())
}

