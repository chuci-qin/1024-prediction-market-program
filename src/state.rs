//! State definitions for the Prediction Market Program
//!
//! All account structures used by the program.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

// ============================================================================
// Discriminators
// ============================================================================

pub const PM_CONFIG_DISCRIMINATOR: u64 = 0x504D5F434F4E4649; // "PM_CONFI"
pub const MARKET_DISCRIMINATOR: u64 = 0x4D41524B45545F5F; // "MARKET__"
pub const ORDER_DISCRIMINATOR: u64 = 0x4F524445525F5F5F; // "ORDER___"
pub const POSITION_DISCRIMINATOR: u64 = 0x504F534954494F4E; // "POSITION"
pub const ORACLE_PROPOSAL_DISCRIMINATOR: u64 = 0x4F5241434C455F50; // "ORACLE_P"

// ============================================================================
// PDA Seeds
// ============================================================================

pub const PM_CONFIG_SEED: &[u8] = b"pm_config";
pub const MARKET_SEED: &[u8] = b"market";
pub const ORDER_SEED: &[u8] = b"order";
pub const ORDER_ESCROW_SEED: &[u8] = b"order_escrow";
pub const POSITION_SEED: &[u8] = b"position";
pub const MARKET_VAULT_SEED: &[u8] = b"market_vault";
pub const YES_MINT_SEED: &[u8] = b"yes_mint";
pub const NO_MINT_SEED: &[u8] = b"no_mint";
pub const ORACLE_PROPOSAL_SEED: &[u8] = b"oracle_proposal";
pub const OUTCOME_MINT_SEED: &[u8] = b"outcome_mint"; // For multi-outcome markets
pub const AUTHORIZED_CALLERS_SEED: &[u8] = b"authorized_callers"; // For matching engine callers

// ============================================================================
// Constants
// ============================================================================

/// Maximum number of outcomes for multi-outcome markets
pub const MAX_OUTCOMES: usize = 32;

/// Maximum outcomes for matching operations (MatchMintMulti/MatchBurnMulti)
/// Limited to 16 to avoid exceeding Solana's 64 account limit per transaction
/// Formula: 6 fixed accounts + 3 * num_outcomes = 54 accounts for 16 outcomes
pub const MAX_OUTCOMES_FOR_MATCH: u8 = 16;

/// Maximum length of market question (bytes)
pub const MAX_QUESTION_LEN: usize = 256;

/// Maximum length of resolution spec (bytes)
pub const MAX_RESOLUTION_SPEC_LEN: usize = 512;

/// Price precision (1 USDC = 1_000_000)
pub const PRICE_PRECISION: u64 = 1_000_000;

/// Minimum price (0.01 = 1%)
pub const MIN_PRICE: u64 = 10_000;

/// Maximum price (0.99 = 99%)
pub const MAX_PRICE: u64 = 990_000;

/// Default challenge window (24 hours)
pub const DEFAULT_CHALLENGE_WINDOW_SECS: i64 = 24 * 60 * 60;

/// Default proposer bond (100 USDC)
pub const DEFAULT_PROPOSER_BOND: u64 = 100_000_000;

// ============================================================================
// Enums
// ============================================================================

/// Market type (binary or multi-outcome)
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketType {
    /// Binary market (YES/NO)
    Binary = 0,
    /// Multi-outcome market (e.g., election with multiple candidates)
    MultiOutcome = 1,
}

impl Default for MarketType {
    fn default() -> Self {
        MarketType::Binary
    }
}

/// Market lifecycle status
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketStatus {
    /// Pending review/activation
    Pending = 0,
    /// Active and tradeable
    Active = 1,
    /// Temporarily paused
    Paused = 2,
    /// Resolved (result finalized)
    Resolved = 3,
    /// Cancelled (refunds available)
    Cancelled = 4,
}

impl Default for MarketStatus {
    fn default() -> Self {
        MarketStatus::Pending
    }
}

/// Market resolution result
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketResult {
    /// YES wins (YES holders get 1 USDC per token)
    Yes = 0,
    /// NO wins (NO holders get 1 USDC per token)
    No = 1,
    /// Invalid/cancelled (all holders refunded)
    Invalid = 2,
}

/// Market review status (moderation)
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewStatus {
    /// No issues
    None = 0,
    /// Flagged for review
    Flagged = 1,
    /// Cancelled - Invalid market
    CancelledInvalid = 2,
    /// Cancelled - Regulatory reasons
    CancelledRegulatory = 3,
}

impl Default for ReviewStatus {
    fn default() -> Self {
        ReviewStatus::None
    }
}

/// Order side (buy/sell)
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    Buy = 0,
    Sell = 1,
}

/// Outcome type (YES/NO)
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Yes = 0,
    No = 1,
}

/// Order status
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderStatus {
    /// Open and waiting to be matched
    Open = 0,
    /// Partially filled
    PartialFilled = 1,
    /// Fully filled
    Filled = 2,
    /// Cancelled by user
    Cancelled = 3,
    /// Expired (GTD orders)
    Expired = 4,
}

impl Default for OrderStatus {
    fn default() -> Self {
        OrderStatus::Open
    }
}

/// Order type (time in force)
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderType {
    /// Good Till Cancel - remains until filled or cancelled
    GTC = 0,
    /// Good Till Date - expires at specified time
    GTD = 1,
    /// Immediate Or Cancel - fill what's possible, cancel rest
    IOC = 2,
    /// Fill Or Kill - fill completely or cancel entirely
    FOK = 3,
}

impl Default for OrderType {
    fn default() -> Self {
        OrderType::GTC
    }
}

/// Oracle proposal status
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalStatus {
    /// Proposal submitted, waiting for challenge window
    Pending = 0,
    /// Disputed, waiting for committee resolution
    Disputed = 1,
    /// Finalized and accepted
    Finalized = 2,
    /// Rejected after dispute
    Rejected = 3,
}

impl Default for ProposalStatus {
    fn default() -> Self {
        ProposalStatus::Pending
    }
}

// ============================================================================
// Account Structures
// ============================================================================

/// Global configuration for the Prediction Market Program
/// 
/// PDA Seeds: ["pm_config"]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct PredictionMarketConfig {
    /// Account discriminator
    pub discriminator: u64,
    
    /// Program administrator
    pub admin: Pubkey,
    
    /// USDC Mint address
    pub usdc_mint: Pubkey,
    
    /// Vault Program ID (for CPI)
    pub vault_program: Pubkey,
    
    /// Fund Program ID (for CPI)
    pub fund_program: Pubkey,
    
    /// Oracle admin (can submit results)
    pub oracle_admin: Pubkey,
    
    /// Next market ID
    pub next_market_id: u64,
    
    /// Total markets created
    pub total_markets: u64,
    
    /// Currently active markets
    pub active_markets: u64,
    
    /// Total trading volume (e6)
    pub total_volume_e6: i64,
    
    /// Total minted complete sets
    pub total_minted_sets: u64,
    
    /// Challenge window duration (seconds)
    pub challenge_window_secs: i64,
    
    /// Proposer bond amount (e6)
    pub proposer_bond_e6: u64,
    
    /// Is the program paused?
    pub is_paused: bool,
    
    /// PDA bump
    pub bump: u8,
    
    /// Reserved for future use
    /// Note: 64 bytes to match existing on-chain data size (290 total)
    pub reserved: [u8; 64],
}

impl PredictionMarketConfig {
    pub const SIZE: usize = 8   // discriminator
        + 32  // admin
        + 32  // usdc_mint
        + 32  // vault_program
        + 32  // fund_program
        + 32  // oracle_admin
        + 8   // next_market_id
        + 8   // total_markets
        + 8   // active_markets
        + 8   // total_volume_e6
        + 8   // total_minted_sets
        + 8   // challenge_window_secs
        + 8   // proposer_bond_e6
        + 1   // is_paused
        + 1   // bump
        + 64; // reserved (= 290 total)
    
    /// PDA seeds
    pub fn seeds() -> Vec<Vec<u8>> {
        vec![PM_CONFIG_SEED.to_vec()]
    }
    
    /// Create new config
    pub fn new(
        admin: Pubkey,
        usdc_mint: Pubkey,
        vault_program: Pubkey,
        fund_program: Pubkey,
        oracle_admin: Pubkey,
        bump: u8,
    ) -> Self {
        Self {
            discriminator: PM_CONFIG_DISCRIMINATOR,
            admin,
            usdc_mint,
            vault_program,
            fund_program,
            oracle_admin,
            next_market_id: 1,
            total_markets: 0,
            active_markets: 0,
            total_volume_e6: 0,
            total_minted_sets: 0,
            challenge_window_secs: DEFAULT_CHALLENGE_WINDOW_SECS,
            proposer_bond_e6: DEFAULT_PROPOSER_BOND,
            is_paused: false,
            bump,
            reserved: [0u8; 64],
        }
    }
}

/// A single prediction market
/// 
/// PDA Seeds: ["market", market_id.to_le_bytes()]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Market {
    /// Account discriminator
    pub discriminator: u64,
    
    /// Unique market ID
    pub market_id: u64,
    
    /// Market type (Binary or MultiOutcome)
    pub market_type: MarketType,
    
    /// Number of outcomes (2 for binary, up to MAX_OUTCOMES for multi)
    pub num_outcomes: u8,
    
    /// Market creator
    pub creator: Pubkey,
    
    /// Question hash (SHA256 of IPFS CID or question text)
    pub question_hash: [u8; 32],
    
    /// Resolution specification hash
    pub resolution_spec_hash: [u8; 32],
    
    /// YES Token Mint (for binary markets)
    pub yes_mint: Pubkey,
    
    /// NO Token Mint (for binary markets)
    pub no_mint: Pubkey,
    
    /// Market USDC Vault
    pub market_vault: Pubkey,
    
    /// Current market status
    pub status: MarketStatus,
    
    /// Review status (moderation)
    pub review_status: ReviewStatus,
    
    /// Earliest resolution time (Unix timestamp)
    pub resolution_time: i64,
    
    /// Latest finalization deadline (Unix timestamp)
    pub finalization_deadline: i64,
    
    /// Final result (set after resolution) - for binary markets
    pub final_result: Option<MarketResult>,
    
    /// Winning outcome index (for multi-outcome markets)
    pub winning_outcome_index: Option<u8>,
    
    /// Market creation timestamp
    pub created_at: i64,
    
    /// Last update timestamp
    pub updated_at: i64,
    
    /// Total complete sets minted
    pub total_minted: u64,
    
    /// Total trading volume (e6)
    pub total_volume_e6: i64,
    
    /// Total open interest (active positions)
    pub open_interest: u64,
    
    /// Creator fee rate (basis points, e.g., 100 = 1%)
    pub creator_fee_bps: u16,
    
    /// Next order ID for this market
    pub next_order_id: u64,
    
    /// PDA bump
    pub bump: u8,
    
    /// Reserved for future use
    pub reserved: [u8; 60],
}

impl Market {
    pub const SIZE: usize = 8   // discriminator
        + 8   // market_id
        + 1   // market_type
        + 1   // num_outcomes
        + 32  // creator
        + 32  // question_hash
        + 32  // resolution_spec_hash
        + 32  // yes_mint
        + 32  // no_mint
        + 32  // market_vault
        + 1   // status
        + 1   // review_status
        + 8   // resolution_time
        + 8   // finalization_deadline
        + 1 + 1 // final_result (Option<MarketResult>)
        + 1 + 1 // winning_outcome_index (Option<u8>)
        + 8   // created_at
        + 8   // updated_at
        + 8   // total_minted
        + 8   // total_volume_e6
        + 8   // open_interest
        + 2   // creator_fee_bps
        + 8   // next_order_id
        + 1   // bump
        + 60; // reserved (reduced by 4)
    
    /// PDA seeds
    pub fn seeds(market_id: u64) -> Vec<Vec<u8>> {
        vec![
            MARKET_SEED.to_vec(),
            market_id.to_le_bytes().to_vec(),
        ]
    }
    
    /// Check if market is tradeable
    pub fn is_tradeable(&self) -> bool {
        self.status == MarketStatus::Active && self.review_status == ReviewStatus::None
    }
    
    /// Check if market can be resolved
    pub fn can_resolve(&self, current_time: i64) -> bool {
        self.status == MarketStatus::Active && current_time >= self.resolution_time
    }
    
    /// Check if market is resolved with a result
    pub fn is_resolved(&self) -> bool {
        match self.market_type {
            MarketType::Binary => self.status == MarketStatus::Resolved && self.final_result.is_some(),
            MarketType::MultiOutcome => self.status == MarketStatus::Resolved && self.winning_outcome_index.is_some(),
        }
    }
    
    /// Check if this is a binary market
    pub fn is_binary(&self) -> bool {
        self.market_type == MarketType::Binary
    }
    
    /// Check if this is a multi-outcome market
    pub fn is_multi_outcome(&self) -> bool {
        self.market_type == MarketType::MultiOutcome
    }
}

// ============================================================================
// Multi-Outcome Market Support
// ============================================================================

/// Outcome metadata for multi-outcome markets
/// 
/// Stored off-chain (IPFS), with only the hash stored on-chain.
/// Each outcome has a separate token mint derived from:
/// PDA Seeds: ["outcome_mint", market_id.to_le_bytes(), outcome_index]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct OutcomeMetadata {
    /// Outcome index (0-based)
    pub index: u8,
    /// Label hash (SHA256 of outcome label)
    pub label_hash: [u8; 32],
    /// Token mint address
    pub mint: Pubkey,
}

impl OutcomeMetadata {
    pub const SIZE: usize = 1 + 32 + 32; // index + label_hash + mint
    
    /// Derive outcome mint PDA seeds
    pub fn mint_seeds(market_id: u64, outcome_index: u8) -> Vec<Vec<u8>> {
        vec![
            OUTCOME_MINT_SEED.to_vec(),
            market_id.to_le_bytes().to_vec(),
            vec![outcome_index],
        ]
    }
}

/// Multi-outcome position
/// 
/// Tracks a user's holdings across all outcomes in a multi-outcome market.
/// Uses a fixed-size array for up to MAX_OUTCOMES outcomes.
/// 
/// PDA Seeds: ["position", market_id.to_le_bytes(), owner.key()]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct MultiOutcomePosition {
    /// Account discriminator
    pub discriminator: u64,
    
    /// Market ID
    pub market_id: u64,
    
    /// Number of outcomes
    pub num_outcomes: u8,
    
    /// Position owner
    pub owner: Pubkey,
    
    /// Holdings for each outcome (up to MAX_OUTCOMES)
    /// Each element is the token amount for that outcome index
    pub holdings: [u64; MAX_OUTCOMES],
    
    /// Average cost for each outcome (e6)
    pub avg_costs: [u64; MAX_OUTCOMES],
    
    /// Realized PnL (e6, can be negative)
    pub realized_pnl: i64,
    
    /// Total USDC spent
    pub total_cost_e6: u64,
    
    /// Has this position been settled?
    pub settled: bool,
    
    /// Settlement amount received (e6)
    pub settlement_amount: u64,
    
    /// Creation timestamp
    pub created_at: i64,
    
    /// Last update timestamp
    pub updated_at: i64,
    
    /// PDA bump
    pub bump: u8,
    
    /// Reserved for future use
    pub reserved: [u8; 32],
}

impl MultiOutcomePosition {
    // holdings: 32 * 8 = 256 bytes, avg_costs: 32 * 8 = 256 bytes
    pub const SIZE: usize = 8   // discriminator
        + 8   // market_id
        + 1   // num_outcomes
        + 32  // owner
        + (MAX_OUTCOMES * 8)  // holdings
        + (MAX_OUTCOMES * 8)  // avg_costs
        + 8   // realized_pnl
        + 8   // total_cost_e6
        + 1   // settled
        + 8   // settlement_amount
        + 8   // created_at
        + 8   // updated_at
        + 1   // bump
        + 32; // reserved
    
    /// Create a new empty multi-outcome position
    pub fn new(market_id: u64, num_outcomes: u8, owner: Pubkey, bump: u8, created_at: i64) -> Self {
        Self {
            discriminator: POSITION_DISCRIMINATOR,
            market_id,
            num_outcomes,
            owner,
            holdings: [0u64; MAX_OUTCOMES],
            avg_costs: [0u64; MAX_OUTCOMES],
            realized_pnl: 0,
            total_cost_e6: 0,
            settled: false,
            settlement_amount: 0,
            created_at,
            updated_at: created_at,
            bump,
            reserved: [0u8; 32],
        }
    }
    
    /// Check if position is empty (no tokens in any outcome)
    pub fn is_empty(&self) -> bool {
        for i in 0..self.num_outcomes as usize {
            if self.holdings[i] > 0 {
                return false;
            }
        }
        true
    }
    
    /// Get holdings for a specific outcome
    pub fn get_holding(&self, outcome_index: u8) -> u64 {
        if (outcome_index as usize) < MAX_OUTCOMES {
            self.holdings[outcome_index as usize]
        } else {
            0
        }
    }
    
    /// Add tokens for a specific outcome
    pub fn add_tokens(&mut self, outcome_index: u8, amount: u64, price: u64, current_time: i64) {
        let idx = outcome_index as usize;
        if idx >= MAX_OUTCOMES {
            return;
        }
        
        // Update weighted average cost
        let total_prev = self.holdings[idx] * self.avg_costs[idx];
        let total_new = amount * price;
        let new_total_amount = self.holdings[idx] + amount;
        if new_total_amount > 0 {
            self.avg_costs[idx] = (total_prev + total_new) / new_total_amount;
        }
        self.holdings[idx] += amount;
        
        let cost = ((amount as u128) * (price as u128) / (PRICE_PRECISION as u128)) as u64;
        self.total_cost_e6 += cost;
        self.updated_at = current_time;
    }
    
    /// Calculate settlement value based on winning outcome
    pub fn calculate_settlement(&self, winning_index: u8) -> u64 {
        if (winning_index as usize) < MAX_OUTCOMES {
            self.holdings[winning_index as usize]
        } else {
            0
        }
    }
}

/// An order in the order book
/// 
/// PDA Seeds: ["order", market_id.to_le_bytes(), order_id.to_le_bytes()]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Order {
    /// Account discriminator
    pub discriminator: u64,
    
    /// Order ID (unique within market)
    pub order_id: u64,
    
    /// Market ID
    pub market_id: u64,
    
    /// Order owner
    pub owner: Pubkey,
    
    /// Order side (Buy/Sell)
    pub side: OrderSide,
    
    /// Outcome type (YES/NO) - for binary markets backward compatibility
    pub outcome: Outcome,
    
    /// Outcome index (0-based) - unified field for all market types
    /// Binary markets: 0 = YES, 1 = NO (synced with outcome field)
    /// Multi-outcome markets: 0..N-1
    pub outcome_index: u8,
    
    /// Order price (e6, e.g., 650000 = $0.65)
    pub price: u64,
    
    /// Total order amount (in tokens)
    pub amount: u64,
    
    /// Filled amount
    pub filled_amount: u64,
    
    /// Order status
    pub status: OrderStatus,
    
    /// Order type (GTC, GTD, IOC, FOK)
    pub order_type: OrderType,
    
    /// Expiration time (for GTD orders)
    pub expiration_time: Option<i64>,
    
    /// Creation timestamp
    pub created_at: i64,
    
    /// Last update timestamp
    pub updated_at: i64,
    
    /// PDA bump
    pub bump: u8,
    
    /// Escrow token account (for sell orders)
    /// This holds the tokens that the seller is offering
    pub escrow_token_account: Option<Pubkey>,
    
    /// Reserved for future use (reduced by 1 byte for outcome_index)
    pub reserved: [u8; 30],
}

impl Order {
    pub const SIZE: usize = 8   // discriminator
        + 8   // order_id
        + 8   // market_id
        + 32  // owner
        + 1   // side
        + 1   // outcome
        + 1   // outcome_index (NEW)
        + 8   // price
        + 8   // amount
        + 8   // filled_amount
        + 1   // status
        + 1   // order_type
        + 1 + 8 // expiration_time (Option<i64>)
        + 8   // created_at
        + 8   // updated_at
        + 1   // bump
        + 1 + 32 // escrow_token_account (Option<Pubkey>)
        + 30; // reserved (reduced by 1 for outcome_index)
    
    /// PDA seeds
    pub fn seeds(market_id: u64, order_id: u64) -> Vec<Vec<u8>> {
        vec![
            ORDER_SEED.to_vec(),
            market_id.to_le_bytes().to_vec(),
            order_id.to_le_bytes().to_vec(),
        ]
    }
    
    /// Escrow token account PDA seeds
    /// For sell orders, tokens are locked in this escrow
    pub fn escrow_seeds(market_id: u64, order_id: u64) -> Vec<Vec<u8>> {
        vec![
            ORDER_ESCROW_SEED.to_vec(),
            market_id.to_le_bytes().to_vec(),
            order_id.to_le_bytes().to_vec(),
        ]
    }
    
    /// Check if this is a sell order with escrowed tokens
    pub fn has_escrow(&self) -> bool {
        self.side == OrderSide::Sell && self.escrow_token_account.is_some()
    }
    
    /// Remaining unfilled amount
    pub fn remaining_amount(&self) -> u64 {
        self.amount.saturating_sub(self.filled_amount)
    }
    
    /// Check if order is still active
    pub fn is_active(&self) -> bool {
        matches!(self.status, OrderStatus::Open | OrderStatus::PartialFilled)
    }
    
    /// Check if order is expired
    pub fn is_expired(&self, current_time: i64) -> bool {
        if let Some(exp_time) = self.expiration_time {
            current_time >= exp_time
        } else {
            false
        }
    }
    
    /// Calculate USDC cost for buying tokens at this order's price
    pub fn calculate_cost(&self, token_amount: u64) -> u64 {
        // cost = amount * price / PRICE_PRECISION
        ((token_amount as u128) * (self.price as u128) / (PRICE_PRECISION as u128)) as u64
    }
    
    /// Calculate USDC proceeds for selling tokens at this order's price
    pub fn calculate_proceeds(&self, token_amount: u64) -> u64 {
        self.calculate_cost(token_amount)
    }
    
    /// Get outcome index (unified interface for both binary and multi-outcome markets)
    /// For binary markets: returns 0 for YES, 1 for NO
    /// For multi-outcome markets: returns the outcome_index directly
    pub fn get_outcome_index(&self) -> u8 {
        self.outcome_index
    }
    
    /// Check if this order is for a binary market (YES/NO)
    pub fn is_binary_market_order(&self) -> bool {
        self.outcome_index <= 1
    }
}

/// User's position in a market
/// 
/// PDA Seeds: ["position", market_id.to_le_bytes(), owner.key()]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Position {
    /// Account discriminator
    pub discriminator: u64,
    
    /// Market ID
    pub market_id: u64,
    
    /// Position owner
    pub owner: Pubkey,
    
    /// YES token holdings
    pub yes_amount: u64,
    
    /// NO token holdings
    pub no_amount: u64,
    
    /// Average cost basis for YES (e6)
    pub yes_avg_cost: u64,
    
    /// Average cost basis for NO (e6)
    pub no_avg_cost: u64,
    
    /// Realized PnL (e6, can be negative)
    pub realized_pnl: i64,
    
    /// Total USDC spent on this position
    pub total_cost_e6: u64,
    
    /// Has this position been settled?
    pub settled: bool,
    
    /// Settlement amount received (e6)
    pub settlement_amount: u64,
    
    /// Creation timestamp
    pub created_at: i64,
    
    /// Last update timestamp
    pub updated_at: i64,
    
    /// PDA bump
    pub bump: u8,
    
    /// Reserved for future use
    pub reserved: [u8; 32],
}

impl Position {
    pub const SIZE: usize = 8   // discriminator
        + 8   // market_id
        + 32  // owner
        + 8   // yes_amount
        + 8   // no_amount
        + 8   // yes_avg_cost
        + 8   // no_avg_cost
        + 8   // realized_pnl
        + 8   // total_cost_e6
        + 1   // settled
        + 8   // settlement_amount
        + 8   // created_at
        + 8   // updated_at
        + 1   // bump
        + 32; // reserved
    
    /// PDA seeds
    pub fn seeds(market_id: u64, owner: &Pubkey) -> Vec<Vec<u8>> {
        vec![
            POSITION_SEED.to_vec(),
            market_id.to_le_bytes().to_vec(),
            owner.to_bytes().to_vec(),
        ]
    }
    
    /// Create a new empty position
    pub fn new(market_id: u64, owner: Pubkey, bump: u8, created_at: i64) -> Self {
        Self {
            discriminator: POSITION_DISCRIMINATOR,
            market_id,
            owner,
            yes_amount: 0,
            no_amount: 0,
            yes_avg_cost: 0,
            no_avg_cost: 0,
            realized_pnl: 0,
            total_cost_e6: 0,
            settled: false,
            settlement_amount: 0,
            created_at,
            updated_at: created_at,
            bump,
            reserved: [0u8; 32],
        }
    }
    
    /// Check if position is empty (no tokens)
    pub fn is_empty(&self) -> bool {
        self.yes_amount == 0 && self.no_amount == 0
    }
    
    /// Calculate unrealized PnL at given prices
    pub fn unrealized_pnl(&self, yes_price: u64, no_price: u64) -> i64 {
        let yes_value = (self.yes_amount as u128) * (yes_price as u128) / (PRICE_PRECISION as u128);
        let no_value = (self.no_amount as u128) * (no_price as u128) / (PRICE_PRECISION as u128);
        let total_value = (yes_value + no_value) as i64;
        total_value - (self.total_cost_e6 as i64)
    }
    
    /// Calculate settlement value based on market result
    pub fn calculate_settlement(&self, result: MarketResult) -> u64 {
        match result {
            MarketResult::Yes => self.yes_amount,
            MarketResult::No => self.no_amount,
            MarketResult::Invalid => {
                // Return original cost basis (simplified)
                self.total_cost_e6
            }
        }
    }
    
    /// Update position after adding tokens
    pub fn add_tokens(
        &mut self,
        outcome: Outcome,
        amount: u64,
        price: u64,
        current_time: i64,
    ) {
        match outcome {
            Outcome::Yes => {
                // Update weighted average cost
                let total_prev = self.yes_amount * self.yes_avg_cost;
                let total_new = amount * price;
                let new_total_amount = self.yes_amount + amount;
                if new_total_amount > 0 {
                    self.yes_avg_cost = (total_prev + total_new) / new_total_amount;
                }
                self.yes_amount += amount;
            }
            Outcome::No => {
                let total_prev = self.no_amount * self.no_avg_cost;
                let total_new = amount * price;
                let new_total_amount = self.no_amount + amount;
                if new_total_amount > 0 {
                    self.no_avg_cost = (total_prev + total_new) / new_total_amount;
                }
                self.no_amount += amount;
            }
        }
        
        let cost = ((amount as u128) * (price as u128) / (PRICE_PRECISION as u128)) as u64;
        self.total_cost_e6 += cost;
        self.updated_at = current_time;
    }
    
    /// Update position after removing tokens
    pub fn remove_tokens(
        &mut self,
        outcome: Outcome,
        amount: u64,
        price: u64,
        current_time: i64,
    ) {
        let cost_basis = match outcome {
            Outcome::Yes => {
                self.yes_amount = self.yes_amount.saturating_sub(amount);
                self.yes_avg_cost
            }
            Outcome::No => {
                self.no_amount = self.no_amount.saturating_sub(amount);
                self.no_avg_cost
            }
        };
        
        // Calculate realized PnL
        let proceeds = ((amount as u128) * (price as u128) / (PRICE_PRECISION as u128)) as i64;
        let cost = ((amount as u128) * (cost_basis as u128) / (PRICE_PRECISION as u128)) as i64;
        self.realized_pnl += proceeds - cost;
        
        self.updated_at = current_time;
    }
}

/// Oracle result proposal
/// 
/// PDA Seeds: ["oracle_proposal", market_id.to_le_bytes()]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct OracleProposal {
    /// Account discriminator
    pub discriminator: u64,
    
    /// Market ID
    pub market_id: u64,
    
    /// Proposer address
    pub proposer: Pubkey,
    
    /// Proposed result
    pub proposed_result: MarketResult,
    
    /// Proposal status
    pub status: ProposalStatus,
    
    /// Proposal timestamp
    pub proposed_at: i64,
    
    /// Challenge deadline
    pub challenge_deadline: i64,
    
    /// Bond amount (e6)
    pub bond_amount: u64,
    
    /// Challenger address (if disputed)
    pub challenger: Option<Pubkey>,
    
    /// Challenger's proposed result (if disputed)
    pub challenger_result: Option<MarketResult>,
    
    /// Challenger's bond
    pub challenger_bond: u64,
    
    /// PDA bump
    pub bump: u8,
    
    /// Reserved for future use
    pub reserved: [u8; 32],
}

impl OracleProposal {
    pub const SIZE: usize = 8   // discriminator
        + 8   // market_id
        + 32  // proposer
        + 1   // proposed_result
        + 1   // status
        + 8   // proposed_at
        + 8   // challenge_deadline
        + 8   // bond_amount
        + 1 + 32 // challenger (Option<Pubkey>)
        + 1 + 1 // challenger_result (Option<MarketResult>)
        + 8   // challenger_bond
        + 1   // bump
        + 32; // reserved
    
    /// PDA seeds
    pub fn seeds(market_id: u64) -> Vec<Vec<u8>> {
        vec![
            ORACLE_PROPOSAL_SEED.to_vec(),
            market_id.to_le_bytes().to_vec(),
        ]
    }
    
    /// Check if challenge window has expired
    pub fn can_finalize(&self, current_time: i64) -> bool {
        self.status == ProposalStatus::Pending && current_time >= self.challenge_deadline
    }
    
    /// Check if proposal can be challenged
    pub fn can_challenge(&self, current_time: i64) -> bool {
        self.status == ProposalStatus::Pending && current_time < self.challenge_deadline
    }
}

// ============================================================================
// Authorized Callers Registry
// ============================================================================

/// Maximum number of authorized callers (matching engines, keepers)
pub const MAX_AUTHORIZED_CALLERS: usize = 10;

/// Discriminator for AuthorizedCallers PDA
pub const AUTHORIZED_CALLERS_DISCRIMINATOR: u64 = 0x3141_5926_5358_9793;

/// Authorized callers registry for matching engine access control
/// 
/// PDA Seeds: ["authorized_callers"]
/// 
/// This structure stores the list of pubkeys authorized to call
/// matching instructions (MatchMint, MatchBurn, ExecuteTrade, etc.)
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct AuthorizedCallers {
    /// Account discriminator
    pub discriminator: u64,
    
    /// Number of active callers
    pub count: u8,
    
    /// List of authorized caller pubkeys (fixed size array)
    pub callers: [Pubkey; MAX_AUTHORIZED_CALLERS],
    
    /// Creation timestamp
    pub created_at: i64,
    
    /// Last update timestamp
    pub updated_at: i64,
    
    /// PDA bump
    pub bump: u8,
    
    /// Reserved for future use
    pub reserved: [u8; 32],
}

impl AuthorizedCallers {
    /// Calculate size: 8 + 1 + (32 * 10) + 8 + 8 + 1 + 32 = 378 bytes
    pub const SIZE: usize = 8   // discriminator
        + 1   // count
        + 32 * MAX_AUTHORIZED_CALLERS  // callers array (320 bytes)
        + 8   // created_at
        + 8   // updated_at
        + 1   // bump
        + 32; // reserved
    
    /// PDA seeds
    pub fn seeds() -> Vec<Vec<u8>> {
        vec![AUTHORIZED_CALLERS_SEED.to_vec()]
    }
    
    /// Create a new empty AuthorizedCallers registry
    pub fn new(bump: u8, created_at: i64) -> Self {
        Self {
            discriminator: AUTHORIZED_CALLERS_DISCRIMINATOR,
            count: 0,
            callers: [Pubkey::default(); MAX_AUTHORIZED_CALLERS],
            created_at,
            updated_at: created_at,
            bump,
            reserved: [0u8; 32],
        }
    }
    
    /// Check if a pubkey is authorized
    pub fn is_authorized(&self, caller: &Pubkey) -> bool {
        for i in 0..(self.count as usize) {
            if self.callers[i] == *caller {
                return true;
            }
        }
        false
    }
    
    /// Add a caller to the list
    /// Returns Ok(()) if added, Err if already exists or list is full
    pub fn add_caller(&mut self, caller: Pubkey, current_time: i64) -> Result<(), ()> {
        // Check if already exists
        if self.is_authorized(&caller) {
            return Err(()); // Already authorized
        }
        
        // Check if list is full
        if (self.count as usize) >= MAX_AUTHORIZED_CALLERS {
            return Err(()); // List full
        }
        
        // Add to list
        self.callers[self.count as usize] = caller;
        self.count += 1;
        self.updated_at = current_time;
        
        Ok(())
    }
    
    /// Remove a caller from the list
    /// Returns Ok(()) if removed, Err if not found
    pub fn remove_caller(&mut self, caller: &Pubkey, current_time: i64) -> Result<(), ()> {
        for i in 0..(self.count as usize) {
            if self.callers[i] == *caller {
                // Swap with last element and decrement count
                let last_idx = (self.count - 1) as usize;
                self.callers[i] = self.callers[last_idx];
                self.callers[last_idx] = Pubkey::default();
                self.count -= 1;
                self.updated_at = current_time;
                return Ok(());
            }
        }
        Err(()) // Not found
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::pubkey::Pubkey;

    #[test]
    fn test_config_size() {
        assert!(PredictionMarketConfig::SIZE > 0);
        println!("PredictionMarketConfig SIZE: {}", PredictionMarketConfig::SIZE);
    }

    #[test]
    fn test_market_size() {
        assert!(Market::SIZE > 0);
        println!("Market SIZE: {}", Market::SIZE);
    }

    #[test]
    fn test_order_size() {
        assert!(Order::SIZE > 0);
        println!("Order SIZE: {}", Order::SIZE);
    }

    #[test]
    fn test_position_size() {
        assert!(Position::SIZE > 0);
        println!("Position SIZE: {}", Position::SIZE);
    }

    #[test]
    fn test_oracle_proposal_size() {
        assert!(OracleProposal::SIZE > 0);
        println!("OracleProposal SIZE: {}", OracleProposal::SIZE);
    }

    #[test]
    fn test_authorized_callers_size() {
        assert!(AuthorizedCallers::SIZE > 0);
        println!("AuthorizedCallers SIZE: {}", AuthorizedCallers::SIZE);
        // Expected: 8 + 1 + 320 + 8 + 8 + 1 + 32 = 378 bytes
        assert_eq!(AuthorizedCallers::SIZE, 378);
    }

    #[test]
    fn test_authorized_callers_operations() {
        let mut callers = AuthorizedCallers::new(255, 1000);
        
        let caller1 = Pubkey::new_unique();
        let caller2 = Pubkey::new_unique();
        let caller3 = Pubkey::new_unique();
        
        // Test add
        assert!(callers.add_caller(caller1, 1001).is_ok());
        assert_eq!(callers.count, 1);
        assert!(callers.is_authorized(&caller1));
        assert!(!callers.is_authorized(&caller2));
        
        // Test add duplicate (should fail)
        assert!(callers.add_caller(caller1, 1002).is_err());
        assert_eq!(callers.count, 1);
        
        // Test add second
        assert!(callers.add_caller(caller2, 1003).is_ok());
        assert_eq!(callers.count, 2);
        assert!(callers.is_authorized(&caller1));
        assert!(callers.is_authorized(&caller2));
        
        // Test remove
        assert!(callers.remove_caller(&caller1, 1004).is_ok());
        assert_eq!(callers.count, 1);
        assert!(!callers.is_authorized(&caller1));
        assert!(callers.is_authorized(&caller2));
        
        // Test remove non-existent (should fail)
        assert!(callers.remove_caller(&caller3, 1005).is_err());
    }

    #[test]
    fn test_position_add_tokens() {
        let mut position = Position::new(1, Pubkey::new_unique(), 255, 1000);
        
        // Add 100 YES tokens at $0.50
        position.add_tokens(Outcome::Yes, 100, 500_000, 1001);
        assert_eq!(position.yes_amount, 100);
        assert_eq!(position.yes_avg_cost, 500_000);
        
        // Add 50 more YES tokens at $0.60
        position.add_tokens(Outcome::Yes, 50, 600_000, 1002);
        assert_eq!(position.yes_amount, 150);
        // Weighted average: (100 * 0.5 + 50 * 0.6) / 150 = 0.533...
        assert!(position.yes_avg_cost > 500_000 && position.yes_avg_cost < 600_000);
    }

    #[test]
    fn test_order_calculate_cost() {
        let order = Order {
            discriminator: ORDER_DISCRIMINATOR,
            order_id: 1,
            market_id: 1,
            owner: Pubkey::new_unique(),
            side: OrderSide::Buy,
            outcome: Outcome::Yes,
            outcome_index: 0,  // YES = 0
            price: 650_000, // $0.65
            amount: 100,
            filled_amount: 0,
            status: OrderStatus::Open,
            order_type: OrderType::GTC,
            expiration_time: None,
            created_at: 1000,
            updated_at: 1000,
            bump: 255,
            escrow_token_account: None,
            reserved: [0u8; 30],
        };
        
        // Cost of 100 tokens at $0.65 = $65 USDC
        // Formula: 100 * 650_000 / 1_000_000 = 65 USDC tokens
        let cost = order.calculate_cost(100);
        assert_eq!(cost, 65);  // 65 USDC (not e6 format)
    }

    #[test]
    fn test_position_settlement() {
        let mut position = Position::new(1, Pubkey::new_unique(), 255, 1000);
        position.yes_amount = 100;
        position.no_amount = 50;
        
        // YES wins
        let settlement = position.calculate_settlement(MarketResult::Yes);
        assert_eq!(settlement, 100); // 100 USDC for 100 YES tokens
        
        // NO wins
        let settlement = position.calculate_settlement(MarketResult::No);
        assert_eq!(settlement, 50); // 50 USDC for 50 NO tokens
    }
}

