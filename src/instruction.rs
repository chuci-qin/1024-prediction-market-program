//! Instruction definitions for the Prediction Market Program

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

use crate::state::{MarketResult, OrderSide, OrderType, Outcome};

/// All instructions supported by the Prediction Market Program
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum PredictionMarketInstruction {
    // =========================================================================
    // Initialization (0-9)
    // =========================================================================
    
    /// Initialize the Prediction Market Program global config
    /// 
    /// Accounts:
    /// 0. `[signer]` Admin
    /// 1. `[writable]` PredictionMarketConfig PDA
    /// 2. `[]` USDC Mint
    /// 3. `[]` Vault Program
    /// 4. `[]` Fund Program
    /// 5. `[]` System Program
    Initialize(InitializeArgs),
    
    /// Reinitialize the config (Admin only, for migration/upgrade)
    /// 
    /// This allows resetting the config data while preserving the account.
    /// Only callable by the current admin.
    /// 
    /// Accounts:
    /// 0. `[signer]` Admin
    /// 1. `[writable]` PredictionMarketConfig PDA
    /// 2. `[]` USDC Mint
    /// 3. `[]` Vault Program
    /// 4. `[]` Fund Program
    ReinitializeConfig(ReinitializeConfigArgs),
    
    // =========================================================================
    // Market Management (10-29)
    // =========================================================================
    
    /// Create a new prediction market
    /// 
    /// Accounts:
    /// 0. `[signer]` Creator
    /// 1. `[writable]` PredictionMarketConfig
    /// 2. `[writable]` Market PDA
    /// 3. `[writable]` YES Token Mint PDA
    /// 4. `[writable]` NO Token Mint PDA
    /// 5. `[writable]` Market Vault PDA
    /// 6. `[]` USDC Mint
    /// 7. `[]` Token Program
    /// 8. `[]` System Program
    /// 9. `[]` Rent Sysvar
    CreateMarket(CreateMarketArgs),
    
    /// Activate a market (Admin only)
    /// 
    /// Accounts:
    /// 0. `[signer]` Admin
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[writable]` Market
    ActivateMarket(ActivateMarketArgs),
    
    /// Pause a market (Admin only)
    /// 
    /// Accounts:
    /// 0. `[signer]` Admin
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[writable]` Market
    PauseMarket(PauseMarketArgs),
    
    /// Resume a paused market (Admin only)
    /// 
    /// Accounts:
    /// 0. `[signer]` Admin
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[writable]` Market
    ResumeMarket(ResumeMarketArgs),
    
    /// Cancel a market (Admin only, refunds will be available)
    /// 
    /// Accounts:
    /// 0. `[signer]` Admin
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[writable]` Market
    CancelMarket(CancelMarketArgs),
    
    /// Flag a market for review
    /// 
    /// Accounts:
    /// 0. `[signer]` Admin
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[writable]` Market
    FlagMarket(FlagMarketArgs),
    
    // =========================================================================
    // Complete Set Operations (30-39)
    // =========================================================================
    
    /// Mint a complete set (1 USDC -> 1 YES + 1 NO)
    /// 
    /// Accounts:
    /// 0. `[signer]` User
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[writable]` Market
    /// 3. `[writable]` Market Vault
    /// 4. `[writable]` User's USDC Account
    /// 5. `[writable]` YES Token Mint
    /// 6. `[writable]` NO Token Mint
    /// 7. `[writable]` User's YES Token Account
    /// 8. `[writable]` User's NO Token Account
    /// 9. `[writable]` User Position PDA
    /// 10. `[writable]` User Vault Account (Vault Program)
    /// 11. `[]` Vault Config
    /// 12. `[]` Vault Program
    /// 13. `[]` Fund Program (for fees)
    /// 14. `[]` Token Program
    /// 15. `[]` System Program
    MintCompleteSet(MintCompleteSetArgs),
    
    /// Redeem a complete set (1 YES + 1 NO -> 1 USDC)
    /// 
    /// Accounts:
    /// 0. `[signer]` User
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[writable]` Market
    /// 3. `[writable]` Market Vault
    /// 4. `[writable]` User's USDC Account
    /// 5. `[writable]` YES Token Mint
    /// 6. `[writable]` NO Token Mint
    /// 7. `[writable]` User's YES Token Account
    /// 8. `[writable]` User's NO Token Account
    /// 9. `[writable]` User Position PDA
    /// 10. `[writable]` User Vault Account (Vault Program)
    /// 11. `[]` Vault Config
    /// 12. `[]` Vault Program
    /// 13. `[]` Fund Program (for fees)
    /// 14. `[]` Token Program
    RedeemCompleteSet(RedeemCompleteSetArgs),
    
    // =========================================================================
    // Order Operations (40-59)
    // =========================================================================
    
    /// Place a new order
    /// 
    /// Accounts:
    /// 0. `[signer]` User
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[writable]` Market
    /// 3. `[writable]` Order PDA
    /// 4. `[writable]` User Position PDA
    /// 5. `[writable]` User Vault Account (for margin lock)
    /// 6. `[]` Vault Config
    /// 7. `[]` Vault Program
    /// 8. `[]` System Program
    PlaceOrder(PlaceOrderArgs),
    
    /// Cancel an existing order
    /// 
    /// Accounts:
    /// 0. `[signer]` User
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[]` Market
    /// 3. `[writable]` Order PDA
    /// 4. `[writable]` User Vault Account (for margin release)
    /// 5. `[]` Vault Config
    /// 6. `[]` Vault Program
    CancelOrder(CancelOrderArgs),
    
    /// Match two orders via minting (Buy YES + Buy NO = Mint)
    /// 
    /// Called by off-chain matching engine (authorized caller)
    /// 
    /// Accounts:
    /// 0. `[signer]` Authorized Caller (matching engine)
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[writable]` Market
    /// 3. `[writable]` YES Buy Order
    /// 4. `[writable]` NO Buy Order
    /// 5. `[writable]` YES Buyer Position
    /// 6. `[writable]` NO Buyer Position
    /// 7. `[writable]` Market Vault
    /// 8. `[writable]` YES Token Mint
    /// 9. `[writable]` NO Token Mint
    /// 10. `[writable]` YES Buyer's Token Account
    /// 11. `[writable]` NO Buyer's Token Account
    /// 12. `[writable]` YES Buyer Vault Account
    /// 13. `[writable]` NO Buyer Vault Account
    /// 14. `[]` Vault Config
    /// 15. `[]` Vault Program
    /// 16. `[]` Fund Program (for fees)
    /// 17. `[]` Token Program
    MatchMint(MatchMintArgs),
    
    /// Match two orders via burning (Sell YES + Sell NO = Burn)
    /// 
    /// Called by off-chain matching engine (authorized caller)
    /// 
    /// Accounts:
    /// 0. `[signer]` Authorized Caller (matching engine)
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[writable]` Market
    /// 3. `[writable]` YES Sell Order
    /// 4. `[writable]` NO Sell Order
    /// 5. `[writable]` YES Seller Position
    /// 6. `[writable]` NO Seller Position
    /// 7. `[writable]` Market Vault
    /// 8. `[writable]` YES Token Mint
    /// 9. `[writable]` NO Token Mint
    /// 10. `[writable]` YES Seller's Token Account
    /// 11. `[writable]` NO Seller's Token Account
    /// 12. `[writable]` YES Seller Vault Account
    /// 13. `[writable]` NO Seller Vault Account
    /// 14. `[]` Vault Config
    /// 15. `[]` Vault Program
    /// 16. `[]` Fund Program (for fees)
    /// 17. `[]` Token Program
    MatchBurn(MatchBurnArgs),
    
    /// Execute a direct trade (Buy against existing Sell or vice versa)
    /// 
    /// Accounts:
    /// 0. `[signer]` Authorized Caller
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[writable]` Market
    /// 3. `[writable]` Taker Order
    /// 4. `[writable]` Maker Order
    /// 5. `[writable]` Taker Position
    /// 6. `[writable]` Maker Position
    /// 7. `[writable]` Taker's Token Account
    /// 8. `[writable]` Maker's Token Account
    /// 9. `[writable]` Taker Vault Account
    /// 10. `[writable]` Maker Vault Account
    /// 11. `[]` Vault Config
    /// 12. `[]` Vault Program
    /// 13. `[]` Fund Program
    /// 14. `[]` Token Program
    ExecuteTrade(ExecuteTradeArgs),
    
    // =========================================================================
    // Oracle / Resolution (60-79)
    // =========================================================================
    
    /// Submit a result proposal (Oracle)
    /// 
    /// Accounts:
    /// 0. `[signer]` Proposer (Oracle or authorized)
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[writable]` Market
    /// 3. `[writable]` OracleProposal PDA
    /// 4. `[writable]` Proposer's Vault Account (for bond)
    /// 5. `[]` Vault Config
    /// 6. `[]` Vault Program
    /// 7. `[]` System Program
    ProposeResult(ProposeResultArgs),
    
    /// Challenge a proposed result
    /// 
    /// Accounts:
    /// 0. `[signer]` Challenger
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[]` Market
    /// 3. `[writable]` OracleProposal
    /// 4. `[writable]` Challenger's Vault Account (for bond)
    /// 5. `[]` Vault Config
    /// 6. `[]` Vault Program
    ChallengeResult(ChallengeResultArgs),
    
    /// Finalize a result after challenge window
    /// 
    /// Accounts:
    /// 0. `[signer]` Anyone (permissionless)
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[writable]` Market
    /// 3. `[writable]` OracleProposal
    /// 4. `[writable]` Proposer's Vault Account (for bond return)
    /// 5. `[]` Vault Config
    /// 6. `[]` Vault Program
    FinalizeResult,
    
    /// Resolve a disputed proposal (Committee only)
    /// 
    /// Accounts:
    /// 0. `[signer]` Committee member
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[writable]` Market
    /// 3. `[writable]` OracleProposal
    /// 4. `[writable]` Winner's Vault Account (bond return)
    /// 5. `[writable]` Loser's Vault Account (bond forfeiture)
    /// 6. `[]` Vault Config
    /// 7. `[]` Vault Program
    ResolveDispute(ResolveDisputeArgs),
    
    // =========================================================================
    // Settlement (80-89)
    // =========================================================================
    
    /// Claim winnings from a resolved market
    /// 
    /// Accounts:
    /// 0. `[signer]` User
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[]` Market
    /// 3. `[writable]` User Position
    /// 4. `[writable]` User's Token Account (YES or NO, will be burned)
    /// 5. `[writable]` Token Mint (YES or NO)
    /// 6. `[writable]` Market Vault
    /// 7. `[writable]` User Vault Account
    /// 8. `[]` Vault Config
    /// 9. `[]` Vault Program
    /// 10. `[]` Token Program
    ClaimWinnings,
    
    /// Refund from a cancelled market
    /// 
    /// Accounts:
    /// 0. `[signer]` User
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[]` Market
    /// 3. `[writable]` User Position
    /// 4. `[writable]` User's YES Token Account
    /// 5. `[writable]` User's NO Token Account
    /// 6. `[writable]` YES Token Mint
    /// 7. `[writable]` NO Token Mint
    /// 8. `[writable]` Market Vault
    /// 9. `[writable]` User Vault Account
    /// 10. `[]` Vault Config
    /// 11. `[]` Vault Program
    /// 12. `[]` Token Program
    RefundCancelledMarket,
    
    // =========================================================================
    // Admin Operations (90-99)
    // =========================================================================
    
    /// Update program admin
    /// 
    /// Accounts:
    /// 0. `[signer]` Current Admin
    /// 1. `[writable]` PredictionMarketConfig
    UpdateAdmin(UpdateAdminArgs),
    
    /// Update oracle admin
    /// 
    /// Accounts:
    /// 0. `[signer]` Admin
    /// 1. `[writable]` PredictionMarketConfig
    UpdateOracleAdmin(UpdateOracleAdminArgs),
    
    /// Set program paused state
    /// 
    /// Accounts:
    /// 0. `[signer]` Admin
    /// 1. `[writable]` PredictionMarketConfig
    SetPaused(SetPausedArgs),
    
    /// Update oracle configuration
    /// 
    /// Accounts:
    /// 0. `[signer]` Admin
    /// 1. `[writable]` PredictionMarketConfig
    UpdateOracleConfig(UpdateOracleConfigArgs),
    
    /// Add authorized caller (matching engine)
    /// 
    /// Accounts:
    /// 0. `[signer]` Admin
    /// 1. `[writable]` PredictionMarketConfig
    AddAuthorizedCaller(AddAuthorizedCallerArgs),
    
    /// Remove authorized caller
    /// 
    /// Accounts:
    /// 0. `[signer]` Admin
    /// 1. `[writable]` PredictionMarketConfig
    RemoveAuthorizedCaller(RemoveAuthorizedCallerArgs),
    
    // =========================================================================
    // Multi-Outcome Market Operations (100-119)
    // =========================================================================
    
    /// Create a multi-outcome market (e.g., election with multiple candidates)
    /// 
    /// Accounts:
    /// 0. `[signer]` Creator
    /// 1. `[writable]` PredictionMarketConfig
    /// 2. `[writable]` Market PDA
    /// 3. `[writable]` Market Vault PDA
    /// 4. `[]` USDC Mint
    /// 5. `[]` Token Program
    /// 6. `[]` System Program
    /// 7. `[]` Rent Sysvar
    /// 8..8+n. `[writable]` Outcome Token Mints (n outcomes)
    CreateMultiOutcomeMarket(CreateMultiOutcomeMarketArgs),
    
    /// Mint a complete set for multi-outcome market
    /// (1 USDC -> 1 token of each outcome)
    /// 
    /// Accounts:
    /// 0. `[signer]` User
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[writable]` Market
    /// 3. `[writable]` Market Vault
    /// 4. `[writable]` User's USDC Account
    /// 5. `[writable]` User Position PDA
    /// 6. `[]` Token Program
    /// 7. `[]` System Program
    /// 8..8+n. `[writable]` Outcome Token Mints + User Token Accounts (pairs)
    MintMultiOutcomeCompleteSet(MintMultiOutcomeCompleteSetArgs),
    
    /// Redeem a complete set for multi-outcome market
    /// (1 of each outcome token -> 1 USDC)
    /// 
    /// Accounts:
    /// 0. `[signer]` User
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[writable]` Market
    /// 3. `[writable]` Market Vault
    /// 4. `[writable]` User's USDC Account
    /// 5. `[writable]` User Position PDA
    /// 6. `[]` Token Program
    /// 7..7+n. `[writable]` Outcome Token Mints + User Token Accounts (pairs)
    RedeemMultiOutcomeCompleteSet(RedeemMultiOutcomeCompleteSetArgs),
    
    /// Place an order for a specific outcome in multi-outcome market
    /// 
    /// Account Layout (unified for Buy and Sell):
    /// 
    /// | Index | Account                    | Buy Order | Sell Order |
    /// |-------|----------------------------|-----------|------------|
    /// | 0     | `[signer]` User            | Required  | Required   |
    /// | 1     | `[]` PredictionMarketConfig| Required  | Required   |
    /// | 2     | `[writable]` Market        | Required  | Required   |
    /// | 3     | `[writable]` Order PDA     | Required  | Required   |
    /// | 4     | `[writable]` User Position | Required  | Required   |
    /// | 5     | `[]` Outcome Token Mint    | Required  | `[writable]` |
    /// | 6     | `[]` User Token Account    | Required  | `[writable]` (source) |
    /// | 7     | `[]` Token Program         | Required  | - |
    /// | 8     | `[]` System Program        | Required  | - |
    /// | 7     | `[writable]` Escrow PDA    | -         | Required (destination) |
    /// | 8     | `[]` Token Program         | -         | Required   |
    /// | 9     | `[]` System Program        | -         | Required   |
    /// | 10    | `[]` Rent Sysvar           | -         | Required   |
    /// 
    /// For Sell orders, tokens are transferred from User Token Account to Escrow.
    /// Escrow PDA = [b"order_escrow", market_id, order_id]
    PlaceMultiOutcomeOrder(PlaceMultiOutcomeOrderArgs),
    
    /// Propose result for multi-outcome market
    /// 
    /// Accounts:
    /// 0. `[signer]` Oracle Admin
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[writable]` Market
    /// 3. `[writable]` Oracle Proposal PDA
    /// 4. `[]` System Program
    ProposeMultiOutcomeResult(ProposeMultiOutcomeResultArgs),
    
    /// Claim winnings from multi-outcome market
    /// 
    /// Accounts:
    /// 0. `[signer]` User
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[]` Market
    /// 3. `[writable]` User Position PDA
    /// 4. `[writable]` User's Winning Outcome Token Account
    /// 5. `[writable]` Winning Outcome Token Mint
    /// 6. `[writable]` Market Vault
    /// 7. `[writable]` User's USDC Account
    /// 8. `[]` Token Program
    ClaimMultiOutcomeWinnings(ClaimMultiOutcomeWinningsArgs),
    
    // =========================================================================
    // Relayer Instructions (200-249) - Admin/Relayer 代替用户签名
    // =========================================================================
    
    /// Relayer 版本的 MintCompleteSet
    /// 
    /// Accounts:
    /// 0. `[signer]` Admin/Relayer
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[writable]` Market
    /// 3. `[writable]` Market Vault
    /// 4. `[writable]` User's Vault Account (Vault Program)
    /// 5. `[writable]` YES Token Mint
    /// 6. `[writable]` NO Token Mint
    /// 7. `[writable]` User's YES Token Account
    /// 8. `[writable]` User's NO Token Account
    /// 9. `[writable]` Position PDA
    /// 10. `[]` VaultConfig
    /// 11. `[]` Vault Program
    /// 12. `[]` Token Program
    /// 13. `[]` System Program
    RelayerMintCompleteSet(RelayerMintCompleteSetArgs),
    
    /// Relayer 版本的 RedeemCompleteSet
    /// 
    /// Accounts: (类似于 RelayerMintCompleteSet)
    RelayerRedeemCompleteSet(RelayerRedeemCompleteSetArgs),
    
    /// Relayer 版本的 PlaceOrder
    /// 
    /// Accounts:
    /// 0. `[signer]` Admin/Relayer
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[writable]` Market
    /// 3. `[writable]` Order PDA
    /// 4. `[writable]` User Position PDA
    /// 5. `[writable]` User Vault Account (for margin lock)
    /// 6. `[]` Vault Config
    /// 7. `[]` Vault Program
    /// 8. `[]` System Program
    RelayerPlaceOrder(RelayerPlaceOrderArgs),
    
    /// Relayer 版本的 CancelOrder
    RelayerCancelOrder(RelayerCancelOrderArgs),
    
    /// Relayer 版本的 ClaimWinnings
    RelayerClaimWinnings(RelayerClaimWinningsArgs),
    
    /// Relayer 版本的 RefundCancelledMarket
    RelayerRefundCancelledMarket(RelayerRefundCancelledMarketArgs),
    
    /// Relayer 版本的 MintMultiOutcomeCompleteSet
    RelayerMintMultiOutcomeCompleteSet(RelayerMintMultiOutcomeCompleteSetArgs),
    
    /// Relayer 版本的 RedeemMultiOutcomeCompleteSet
    RelayerRedeemMultiOutcomeCompleteSet(RelayerRedeemMultiOutcomeCompleteSetArgs),
    
    /// Relayer 版本的 PlaceMultiOutcomeOrder
    RelayerPlaceMultiOutcomeOrder(RelayerPlaceMultiOutcomeOrderArgs),
    
    /// Relayer 版本的 ClaimMultiOutcomeWinnings
    RelayerClaimMultiOutcomeWinnings(RelayerClaimMultiOutcomeWinningsArgs),
    
    // =========================================================================
    // Multi-Outcome Matching Operations (250-259)
    // =========================================================================
    
    /// Match multiple buy orders via minting for multi-outcome market
    /// 
    /// Called by off-chain matching engine (authorized caller)
    /// When sum of all outcome buy prices <= 1.0, mint one token of each outcome.
    /// 
    /// Accounts:
    /// 0. `[signer]` Authorized Caller (matching engine)
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[writable]` Market
    /// 3. `[writable]` Market Vault
    /// 4. `[]` Token Program
    /// 5. `[]` System Program
    /// 6..6+3*N: Dynamic accounts for each outcome
    MatchMintMulti(MatchMintMultiArgs),
    
    /// Match multiple sell orders via burning for multi-outcome market
    /// 
    /// Called by off-chain matching engine (authorized caller)
    /// When sum of all outcome sell prices >= 1.0, burn tokens and return USDC.
    /// 
    /// Accounts: (same structure as MatchMintMulti)
    MatchBurnMulti(MatchBurnMultiArgs),
    
    /// Relayer version of MatchMintMulti
    RelayerMatchMintMulti(RelayerMatchMintMultiArgs),
    
    /// Relayer version of MatchBurnMulti
    RelayerMatchBurnMulti(RelayerMatchBurnMultiArgs),
    
    // =========================================================================
    // V2 Instructions - Pure Vault Mode (260-279)
    // NO SPL Token (YES/NO Mints), positions tracked in Position PDA
    // =========================================================================
    
    /// V2: RelayerMintCompleteSet (Vault CPI, no SPL Token)
    /// Uses Vault.PredictionMarketLock instead of SPL Token minting
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
    RelayerMintCompleteSetV2(RelayerMintCompleteSetArgs),
    
    /// V2: RelayerRedeemCompleteSet (Vault CPI, no SPL Token)
    /// Uses Vault.PredictionMarketUnlock instead of SPL Token burning
    /// 
    /// Accounts: (same as RelayerMintCompleteSetV2)
    RelayerRedeemCompleteSetV2(RelayerRedeemCompleteSetArgs),
    
    /// V2: MatchMint (Vault CPI, no SPL Token)
    /// 
    /// Accounts:
    /// 0. `[signer]` Relayer/Matcher
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[writable]` Market
    /// 3. `[writable]` YES Buy Order
    /// 4. `[writable]` NO Buy Order
    /// 5. `[writable]` YES Buyer Position
    /// 6. `[writable]` NO Buyer Position
    /// 7. `[writable]` YES Buyer Vault Account
    /// 8. `[writable]` YES Buyer PM User Account
    /// 9. `[writable]` NO Buyer Vault Account
    /// 10. `[writable]` NO Buyer PM User Account
    /// 11. `[]` Vault Config
    /// 12. `[]` Vault Program
    /// 13. `[]` System Program
    MatchMintV2(MatchMintArgs),
    
    /// V2: MatchBurn (Vault CPI, no SPL Token)
    /// 
    /// Accounts: (same as MatchMintV2)
    MatchBurnV2(MatchBurnArgs),
    
    /// V2: RelayerClaimWinnings (Vault CPI, no SPL Token)
    /// Uses Vault.PredictionMarketSettle for settlement
    /// 
    /// Accounts:
    /// 0. `[signer]` Relayer
    /// 1. `[]` PredictionMarketConfig
    /// 2. `[]` Market
    /// 3. `[writable]` Position PDA
    /// 4. `[writable]` PM User Account
    /// 5. `[]` Vault Config
    /// 6. `[]` Vault Program
    RelayerClaimWinningsV2(RelayerClaimWinningsArgs),
}

// ============================================================================
// Argument Structs
// ============================================================================

// === Initialization ===

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct InitializeArgs {
    /// Oracle admin pubkey
    pub oracle_admin: Pubkey,
    /// Challenge window in seconds (default: 24 hours)
    pub challenge_window_secs: i64,
    /// Proposer bond amount (e6)
    pub proposer_bond_e6: u64,
}

/// Arguments for ReinitializeConfig
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ReinitializeConfigArgs {
    /// New oracle admin pubkey
    pub oracle_admin: Pubkey,
    /// Challenge window in seconds
    pub challenge_window_secs: i64,
    /// Proposer bond amount (e6)
    pub proposer_bond_e6: u64,
    /// Reset market counters (if true, resets next_market_id, total_markets, etc.)
    pub reset_counters: bool,
}

// === Market Management ===

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct CreateMarketArgs {
    /// Question hash (SHA256 of IPFS CID or question text)
    pub question_hash: [u8; 32],
    /// Resolution specification hash
    pub resolution_spec_hash: [u8; 32],
    /// Earliest resolution time (Unix timestamp)
    pub resolution_time: i64,
    /// Latest finalization deadline (Unix timestamp)
    pub finalization_deadline: i64,
    /// Creator fee in basis points (max 500 = 5%)
    pub creator_fee_bps: u16,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ActivateMarketArgs {
    /// Market ID
    pub market_id: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct PauseMarketArgs {
    /// Market ID
    pub market_id: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ResumeMarketArgs {
    /// Market ID
    pub market_id: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct CancelMarketArgs {
    /// Market ID
    pub market_id: u64,
    /// Reason for cancellation
    pub reason: u8,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct FlagMarketArgs {
    /// Market ID
    pub market_id: u64,
    /// Flag reason
    pub reason: u8,
}

// === Complete Set Operations ===

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct MintCompleteSetArgs {
    /// Market ID
    pub market_id: u64,
    /// Amount to mint (in tokens, 1:1 with USDC)
    pub amount: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct RedeemCompleteSetArgs {
    /// Market ID
    pub market_id: u64,
    /// Amount to redeem (in tokens)
    pub amount: u64,
}

// === Order Operations ===

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct PlaceOrderArgs {
    /// Market ID
    pub market_id: u64,
    /// Order side (Buy/Sell)
    pub side: OrderSide,
    /// Outcome (YES/NO)
    pub outcome: Outcome,
    /// Price (e6, e.g., 650000 = $0.65)
    pub price: u64,
    /// Amount in tokens
    pub amount: u64,
    /// Order type
    pub order_type: OrderType,
    /// Expiration time (for GTD orders)
    pub expiration_time: Option<i64>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct CancelOrderArgs {
    /// Market ID
    pub market_id: u64,
    /// Order ID
    pub order_id: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct MatchMintArgs {
    /// Market ID
    pub market_id: u64,
    /// YES buy order ID
    pub yes_order_id: u64,
    /// NO buy order ID
    pub no_order_id: u64,
    /// Amount to match
    pub amount: u64,
    /// Match price for YES (e6)
    pub yes_price: u64,
    /// Match price for NO (e6)
    pub no_price: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct MatchBurnArgs {
    /// Market ID
    pub market_id: u64,
    /// YES sell order ID
    pub yes_order_id: u64,
    /// NO sell order ID
    pub no_order_id: u64,
    /// Amount to match
    pub amount: u64,
    /// Match price for YES (e6)
    pub yes_price: u64,
    /// Match price for NO (e6)
    pub no_price: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ExecuteTradeArgs {
    /// Market ID
    pub market_id: u64,
    /// Taker order ID
    pub taker_order_id: u64,
    /// Maker order ID
    pub maker_order_id: u64,
    /// Amount to trade
    pub amount: u64,
    /// Execution price (e6)
    pub price: u64,
}

// === Multi-Outcome Matching Operations ===

/// Order info for multi-outcome matching: (outcome_index, order_id, price_e6)
pub type MultiOutcomeOrderInfo = (u8, u64, u64);

/// Arguments for MatchMintMulti instruction (Multi-Outcome Market)
/// 
/// Complete Set Mint for multi-outcome market:
/// When sum of all outcome buy prices <= 1.0, mint one token of each outcome.
/// 
/// Accounts:
/// 0. `[signer]` Authorized Caller (Matching Engine)
/// 1. `[]` PredictionMarketConfig
/// 2. `[writable]` Market
/// 3. `[writable]` Market Vault (receives USDC)
/// 4. `[]` Token Program
/// 5. `[]` System Program
/// 6..6+3*N: For each outcome i (i = 0..N-1):
///    - `[writable]` Order PDA
///    - `[writable]` Outcome Token Mint
///    - `[writable]` Buyer's Token Account
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct MatchMintMultiArgs {
    /// Market ID
    pub market_id: u64,
    /// Number of outcomes (2-16, limited to avoid account limit)
    pub num_outcomes: u8,
    /// Amount to match/mint
    pub amount: u64,
    /// Order info for each outcome: Vec<(outcome_index, order_id, price_e6)>
    /// Must contain exactly num_outcomes entries
    /// Sum of all prices must be <= 1_000_000 (1.0 USDC)
    pub orders: Vec<MultiOutcomeOrderInfo>,
}

/// Arguments for MatchBurnMulti instruction (Multi-Outcome Market)
/// 
/// Complete Set Burn for multi-outcome market:
/// When sum of all outcome sell prices >= 1.0, burn tokens and return USDC.
/// 
/// Accounts:
/// 0. `[signer]` Authorized Caller (Matching Engine)
/// 1. `[]` PredictionMarketConfig
/// 2. `[writable]` Market
/// 3. `[writable]` Market Vault (releases USDC)
/// 4. `[]` Token Program
/// 5. `[]` System Program
/// 6..6+3*N: For each outcome i (i = 0..N-1):
///    - `[writable]` Order PDA
///    - `[writable]` Outcome Token Mint
///    - `[writable]` Seller's Token Account
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct MatchBurnMultiArgs {
    /// Market ID
    pub market_id: u64,
    /// Number of outcomes (2-16)
    pub num_outcomes: u8,
    /// Amount to match/burn
    pub amount: u64,
    /// Order info for each outcome: Vec<(outcome_index, order_id, price_e6)>
    /// Must contain exactly num_outcomes entries
    /// Sum of all prices must be >= 1_000_000 (1.0 USDC)
    pub orders: Vec<MultiOutcomeOrderInfo>,
}

// === Oracle / Resolution ===

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ProposeResultArgs {
    /// Market ID
    pub market_id: u64,
    /// Proposed result
    pub result: MarketResult,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ChallengeResultArgs {
    /// Market ID
    pub market_id: u64,
    /// Challenger's proposed result
    pub result: MarketResult,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ResolveDisputeArgs {
    /// Market ID
    pub market_id: u64,
    /// Final result decided by committee
    pub result: MarketResult,
}

// === Admin Operations ===

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct UpdateAdminArgs {
    /// New admin pubkey
    pub new_admin: Pubkey,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct UpdateOracleAdminArgs {
    /// New oracle admin pubkey
    pub new_oracle_admin: Pubkey,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct SetPausedArgs {
    /// Paused state
    pub paused: bool,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct UpdateOracleConfigArgs {
    /// New challenge window (seconds)
    pub challenge_window_secs: Option<i64>,
    /// New proposer bond (e6)
    pub proposer_bond_e6: Option<u64>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct AddAuthorizedCallerArgs {
    /// Caller to authorize
    pub caller: Pubkey,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct RemoveAuthorizedCallerArgs {
    /// Caller to remove
    pub caller: Pubkey,
}

// === Multi-Outcome Market Operations ===

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct CreateMultiOutcomeMarketArgs {
    /// Question hash (SHA256 of IPFS CID or question text)
    pub question_hash: [u8; 32],
    /// Resolution specification hash
    pub resolution_spec_hash: [u8; 32],
    /// Number of outcomes (2-32)
    pub num_outcomes: u8,
    /// Outcome label hashes (SHA256 of each outcome label)
    /// Length must match num_outcomes
    pub outcome_hashes: Vec<[u8; 32]>,
    /// Earliest resolution time (Unix timestamp)
    pub resolution_time: i64,
    /// Latest finalization deadline (Unix timestamp)
    pub finalization_deadline: i64,
    /// Creator fee in basis points (max 500 = 5%)
    pub creator_fee_bps: u16,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct MintMultiOutcomeCompleteSetArgs {
    /// Market ID
    pub market_id: u64,
    /// Amount to mint (1 USDC -> 1 of each outcome token)
    pub amount: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct RedeemMultiOutcomeCompleteSetArgs {
    /// Market ID
    pub market_id: u64,
    /// Amount to redeem (1 of each outcome token -> 1 USDC)
    pub amount: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct PlaceMultiOutcomeOrderArgs {
    /// Market ID
    pub market_id: u64,
    /// Order side (Buy/Sell)
    pub side: OrderSide,
    /// Outcome index (0-based)
    pub outcome_index: u8,
    /// Price (e6, e.g., 250000 = $0.25 for 4-outcome market)
    pub price: u64,
    /// Amount in tokens
    pub amount: u64,
    /// Order type
    pub order_type: OrderType,
    /// Expiration time (for GTD orders)
    pub expiration_time: Option<i64>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ProposeMultiOutcomeResultArgs {
    /// Market ID
    pub market_id: u64,
    /// Winning outcome index (0-based)
    pub winning_outcome_index: u8,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ClaimMultiOutcomeWinningsArgs {
    /// Market ID
    pub market_id: u64,
}

// ============================================================================
// Relayer Instructions (200-249) - User does NOT sign
// ============================================================================

/// Relayer版本的MintCompleteSet
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct RelayerMintCompleteSetArgs {
    /// 用户钱包地址
    pub user_wallet: Pubkey,
    /// Market ID
    pub market_id: u64,
    /// Amount to mint
    pub amount: u64,
}

/// Relayer版本的RedeemCompleteSet
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct RelayerRedeemCompleteSetArgs {
    /// 用户钱包地址
    pub user_wallet: Pubkey,
    /// Market ID
    pub market_id: u64,
    /// Amount to redeem
    pub amount: u64,
}

/// Relayer版本的PlaceOrder
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct RelayerPlaceOrderArgs {
    /// 用户钱包地址
    pub user_wallet: Pubkey,
    /// Market ID
    pub market_id: u64,
    /// Order side (Buy/Sell)
    pub side: OrderSide,
    /// Outcome (YES/NO)
    pub outcome: Outcome,
    /// Price (e6)
    pub price: u64,
    /// Amount in tokens
    pub amount: u64,
    /// Order type
    pub order_type: OrderType,
    /// Expiration time (for GTD orders)
    pub expiration_time: Option<i64>,
}

/// Relayer版本的CancelOrder
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct RelayerCancelOrderArgs {
    /// 用户钱包地址
    pub user_wallet: Pubkey,
    /// Market ID
    pub market_id: u64,
    /// Order ID
    pub order_id: u64,
}

/// Relayer版本的ClaimWinnings
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct RelayerClaimWinningsArgs {
    /// 用户钱包地址
    pub user_wallet: Pubkey,
    /// Market ID
    pub market_id: u64,
}

/// Relayer版本的RefundCancelledMarket
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct RelayerRefundCancelledMarketArgs {
    /// 用户钱包地址
    pub user_wallet: Pubkey,
    /// Market ID
    pub market_id: u64,
}

/// Relayer版本的MintMultiOutcomeCompleteSet
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct RelayerMintMultiOutcomeCompleteSetArgs {
    /// 用户钱包地址
    pub user_wallet: Pubkey,
    /// Market ID
    pub market_id: u64,
    /// Amount to mint
    pub amount: u64,
}

/// Relayer版本的RedeemMultiOutcomeCompleteSet
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct RelayerRedeemMultiOutcomeCompleteSetArgs {
    /// 用户钱包地址
    pub user_wallet: Pubkey,
    /// Market ID
    pub market_id: u64,
    /// Amount to redeem
    pub amount: u64,
}

/// Relayer版本的PlaceMultiOutcomeOrder
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct RelayerPlaceMultiOutcomeOrderArgs {
    /// 用户钱包地址
    pub user_wallet: Pubkey,
    /// Market ID
    pub market_id: u64,
    /// Order side (Buy/Sell)
    pub side: OrderSide,
    /// Outcome index (0-based)
    pub outcome_index: u8,
    /// Price (e6)
    pub price: u64,
    /// Amount in tokens
    pub amount: u64,
    /// Order type
    pub order_type: OrderType,
    /// Expiration time (for GTD orders)
    pub expiration_time: Option<i64>,
}

/// Relayer版本的ClaimMultiOutcomeWinnings
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct RelayerClaimMultiOutcomeWinningsArgs {
    /// 用户钱包地址
    pub user_wallet: Pubkey,
    /// Market ID
    pub market_id: u64,
}

// === Multi-Outcome Matching (Relayer Versions) ===

/// Relayer版本的MatchMintMulti
/// 
/// Relayer/Admin signs instead of individual users
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct RelayerMatchMintMultiArgs {
    /// Market ID
    pub market_id: u64,
    /// Number of outcomes (2-16)
    pub num_outcomes: u8,
    /// Amount to match/mint
    pub amount: u64,
    /// Order info for each outcome: Vec<(outcome_index, order_id, price_e6)>
    pub orders: Vec<MultiOutcomeOrderInfo>,
}

/// Relayer版本的MatchBurnMulti
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct RelayerMatchBurnMultiArgs {
    /// Market ID
    pub market_id: u64,
    /// Number of outcomes (2-16)
    pub num_outcomes: u8,
    /// Amount to match/burn
    pub amount: u64,
    /// Order info for each outcome: Vec<(outcome_index, order_id, price_e6)>
    pub orders: Vec<MultiOutcomeOrderInfo>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_serialization() {
        let args = CreateMarketArgs {
            question_hash: [0u8; 32],
            resolution_spec_hash: [0u8; 32],
            resolution_time: 1700000000,
            finalization_deadline: 1701000000,
            creator_fee_bps: 100,
        };
        let ix = PredictionMarketInstruction::CreateMarket(args);
        let serialized = ix.try_to_vec().unwrap();
        assert!(!serialized.is_empty());
        
        let deserialized: PredictionMarketInstruction = 
            BorshDeserialize::try_from_slice(&serialized).unwrap();
        match deserialized {
            PredictionMarketInstruction::CreateMarket(a) => {
                assert_eq!(a.creator_fee_bps, 100);
            }
            _ => panic!("Wrong instruction type"),
        }
    }

    #[test]
    fn test_place_order_serialization() {
        let args = PlaceOrderArgs {
            market_id: 1,
            side: OrderSide::Buy,
            outcome: Outcome::Yes,
            price: 650_000,
            amount: 100,
            order_type: OrderType::GTC,
            expiration_time: None,
        };
        let ix = PredictionMarketInstruction::PlaceOrder(args);
        let serialized = ix.try_to_vec().unwrap();
        
        let deserialized: PredictionMarketInstruction = 
            BorshDeserialize::try_from_slice(&serialized).unwrap();
        match deserialized {
            PredictionMarketInstruction::PlaceOrder(a) => {
                assert_eq!(a.market_id, 1);
                assert_eq!(a.price, 650_000);
            }
            _ => panic!("Wrong instruction type"),
        }
    }

    #[test]
    fn test_match_mint_multi_serialization() {
        // 3-outcome market: sum of prices = 0.33 + 0.33 + 0.34 = 1.00
        let args = MatchMintMultiArgs {
            market_id: 1,
            num_outcomes: 3,
            amount: 100,
            orders: vec![
                (0, 101, 330_000), // outcome 0, order 101, price 0.33
                (1, 102, 330_000), // outcome 1, order 102, price 0.33
                (2, 103, 340_000), // outcome 2, order 103, price 0.34
            ],
        };
        let ix = PredictionMarketInstruction::MatchMintMulti(args);
        let serialized = ix.try_to_vec().unwrap();
        assert!(!serialized.is_empty());
        
        let deserialized: PredictionMarketInstruction = 
            BorshDeserialize::try_from_slice(&serialized).unwrap();
        match deserialized {
            PredictionMarketInstruction::MatchMintMulti(a) => {
                assert_eq!(a.market_id, 1);
                assert_eq!(a.num_outcomes, 3);
                assert_eq!(a.amount, 100);
                assert_eq!(a.orders.len(), 3);
                
                // Verify total price sum
                let total_price: u64 = a.orders.iter().map(|(_, _, p)| p).sum();
                assert_eq!(total_price, 1_000_000); // Exactly 1.0 USDC
            }
            _ => panic!("Wrong instruction type"),
        }
    }

    #[test]
    fn test_match_burn_multi_serialization() {
        // 4-outcome market: sum of prices = 0.30 + 0.30 + 0.20 + 0.25 = 1.05 >= 1.0
        let args = MatchBurnMultiArgs {
            market_id: 2,
            num_outcomes: 4,
            amount: 50,
            orders: vec![
                (0, 201, 300_000), // outcome 0, order 201, price 0.30
                (1, 202, 300_000), // outcome 1, order 202, price 0.30
                (2, 203, 200_000), // outcome 2, order 203, price 0.20
                (3, 204, 250_000), // outcome 3, order 204, price 0.25
            ],
        };
        let ix = PredictionMarketInstruction::MatchBurnMulti(args);
        let serialized = ix.try_to_vec().unwrap();
        
        let deserialized: PredictionMarketInstruction = 
            BorshDeserialize::try_from_slice(&serialized).unwrap();
        match deserialized {
            PredictionMarketInstruction::MatchBurnMulti(a) => {
                assert_eq!(a.market_id, 2);
                assert_eq!(a.num_outcomes, 4);
                assert_eq!(a.amount, 50);
                assert_eq!(a.orders.len(), 4);
                
                // Verify total price sum >= 1.0
                let total_price: u64 = a.orders.iter().map(|(_, _, p)| p).sum();
                assert!(total_price >= 1_000_000); // Should be >= 1.0 USDC
            }
            _ => panic!("Wrong instruction type"),
        }
    }

    #[test]
    fn test_relayer_match_mint_multi_serialization() {
        let args = RelayerMatchMintMultiArgs {
            market_id: 3,
            num_outcomes: 2,
            amount: 200,
            orders: vec![
                (0, 301, 500_000), // YES, order 301, price 0.50
                (1, 302, 500_000), // NO, order 302, price 0.50
            ],
        };
        let ix = PredictionMarketInstruction::RelayerMatchMintMulti(args);
        let serialized = ix.try_to_vec().unwrap();
        
        let deserialized: PredictionMarketInstruction = 
            BorshDeserialize::try_from_slice(&serialized).unwrap();
        match deserialized {
            PredictionMarketInstruction::RelayerMatchMintMulti(a) => {
                assert_eq!(a.market_id, 3);
                assert_eq!(a.num_outcomes, 2);
                assert_eq!(a.orders.len(), 2);
            }
            _ => panic!("Wrong instruction type"),
        }
    }
}

