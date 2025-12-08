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
}

