//! Error types for the Prediction Market Program

use num_derive::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    program_error::ProgramError,
};
use thiserror::Error;

/// Errors that may be returned by the Prediction Market Program
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum PredictionMarketError {
    // === General Errors (0-99) ===
    
    #[error("Invalid instruction")]
    InvalidInstruction = 0,
    
    #[error("Invalid account data")]
    InvalidAccountData = 1,
    
    #[error("Account not initialized")]
    AccountNotInitialized = 2,
    
    #[error("Already initialized")]
    AlreadyInitialized = 3,
    
    #[error("Invalid program address")]
    InvalidProgramAddress = 4,
    
    #[error("Invalid signer")]
    InvalidSigner = 5,
    
    #[error("Unauthorized")]
    Unauthorized = 6,
    
    #[error("Arithmetic overflow")]
    ArithmeticOverflow = 7,
    
    #[error("Insufficient funds")]
    InsufficientFunds = 8,
    
    #[error("Program is paused")]
    ProgramPaused = 9,
    
    #[error("Invalid PDA")]
    InvalidPDA = 10,
    
    #[error("Invalid amount")]
    InvalidAmount = 11,
    
    #[error("Invalid price")]
    InvalidPrice = 12,
    
    #[error("Invalid USDC mint")]
    InvalidUSDCMint = 13,
    
    #[error("Invalid argument")]
    InvalidArgument = 14,
    
    // === Market Errors (100-199) ===
    
    #[error("Market not found")]
    MarketNotFound = 100,
    
    #[error("Market already exists")]
    MarketAlreadyExists = 101,
    
    #[error("Invalid market status")]
    InvalidMarketStatus = 102,
    
    #[error("Market is paused")]
    MarketPaused = 103,
    
    #[error("Market not active")]
    MarketNotActive = 104,
    
    #[error("Market already resolved")]
    MarketAlreadyResolved = 105,
    
    #[error("Market not resolved")]
    MarketNotResolved = 106,
    
    #[error("Resolution time not reached")]
    ResolutionTimeNotReached = 107,
    
    #[error("Market cancelled")]
    MarketCancelled = 108,
    
    #[error("Invalid resolution time")]
    InvalidResolutionTime = 109,
    
    #[error("Market under review")]
    MarketUnderReview = 110,
    
    #[error("Market not tradeable")]
    MarketNotTradeable = 111,
    
    #[error("Invalid finalization deadline")]
    InvalidFinalizationDeadline = 112,
    
    #[error("Creator fee too high")]
    CreatorFeeTooHigh = 113,
    
    #[error("Market not cancelled")]
    MarketNotCancelled = 114,
    
    #[error("Market not resolvable")]
    MarketNotResolvable = 115,
    
    #[error("Invalid market vault")]
    InvalidMarketVault = 116,
    
    #[error("Invalid YES mint")]
    InvalidYesMint = 117,
    
    #[error("Invalid NO mint")]
    InvalidNoMint = 118,
    
    #[error("Invalid token mint")]
    InvalidTokenMint = 119,
    
    #[error("Invalid market type")]
    InvalidMarketType = 120,
    
    // === Order Errors (200-299) ===
    
    #[error("Order not found")]
    OrderNotFound = 200,
    
    #[error("Order already filled")]
    OrderAlreadyFilled = 201,
    
    #[error("Order already cancelled")]
    OrderAlreadyCancelled = 202,
    
    #[error("Invalid order price")]
    InvalidOrderPrice = 203,
    
    #[error("Invalid order amount")]
    InvalidOrderAmount = 204,
    
    #[error("Orders not matchable")]
    OrdersNotMatchable = 205,
    
    #[error("Order expired")]
    OrderExpired = 206,
    
    #[error("Invalid order type")]
    InvalidOrderType = 207,
    
    #[error("Order owner mismatch")]
    OrderOwnerMismatch = 208,
    
    #[error("Order not active")]
    OrderNotActive = 209,
    
    #[error("Invalid order side")]
    InvalidOrderSide = 210,
    
    #[error("Invalid outcome")]
    InvalidOutcome = 211,
    
    #[error("Invalid price pair")]
    InvalidPricePair = 212,
    
    #[error("Price exceeds limit")]
    PriceExceedsLimit = 213,
    
    #[error("No matchable amount")]
    NoMatchableAmount = 214,
    
    #[error("Price below limit")]
    PriceBelowLimit = 215,
    
    #[error("Outcome mismatch")]
    OutcomeMismatch = 216,
    
    #[error("Price mismatch")]
    PriceMismatch = 217,
    
    #[error("Invalid execution price")]
    InvalidExecutionPrice = 218,
    
    #[error("Invalid expiration time")]
    InvalidExpirationTime = 219,
    
    #[error("Missing expiration time")]
    MissingExpirationTime = 220,
    
    // === Position Errors (300-399) ===
    
    #[error("Position not found")]
    PositionNotFound = 300,
    
    #[error("Already settled")]
    AlreadySettled = 301,
    
    #[error("Insufficient position")]
    InsufficientPosition = 302,
    
    #[error("Position not empty")]
    PositionNotEmpty = 303,
    
    #[error("Insufficient token balance")]
    InsufficientTokenBalance = 304,
    
    // === Complete Set Errors (400-499) ===
    
    #[error("Insufficient USDC for minting")]
    InsufficientUsdcForMinting = 400,
    
    #[error("Insufficient tokens for redemption")]
    InsufficientTokensForRedemption = 401,
    
    #[error("Redemption amount exceeds available")]
    RedemptionExceedsAvailable = 402,
    
    #[error("Invalid mint amount")]
    InvalidMintAmount = 403,
    
    // === Oracle Errors (500-599) ===
    
    #[error("Oracle result not available")]
    OracleResultNotAvailable = 500,
    
    #[error("Invalid oracle result")]
    InvalidOracleResult = 501,
    
    #[error("Oracle dispute in progress")]
    OracleDisputeInProgress = 502,
    
    #[error("Challenge window expired")]
    ChallengeWindowExpired = 503,
    
    #[error("Challenge window not expired")]
    ChallengeWindowNotExpired = 504,
    
    #[error("Invalid proposer")]
    InvalidProposer = 505,
    
    #[error("Insufficient proposer bond")]
    InsufficientProposerBond = 506,
    
    #[error("Proposal already exists")]
    ProposalAlreadyExists = 507,
    
    #[error("Cannot challenge")]
    CannotChallenge = 508,
    
    #[error("Same result as proposal")]
    SameResultAsProposal = 509,
    
    #[error("Cannot finalize")]
    CannotFinalize = 510,
    
    #[error("Proposal not disputed")]
    ProposalNotDisputed = 511,
    
    // === Token Errors (600-699) ===
    
    // InvalidTokenMint moved to 119 in Market Errors section
    
    #[error("Invalid token account")]
    InvalidTokenAccount = 601,
    
    #[error("Token transfer failed")]
    TokenTransferFailed = 602,
    
    #[error("Token mint failed")]
    TokenMintFailed = 603,
    
    #[error("Token burn failed")]
    TokenBurnFailed = 604,
    
    #[error("Invalid escrow owner")]
    InvalidEscrowOwner = 605,
    
    #[error("Insufficient escrow balance")]
    InsufficientEscrowBalance = 606,
    
    #[error("Escrow account not found")]
    EscrowNotFound = 607,
    
    #[error("Escrow already exists")]
    EscrowAlreadyExists = 608,
    
    // === Multi-Outcome Errors (650-699) ===
    
    #[error("Too many outcomes (max 16 for matching)")]
    TooManyOutcomes = 650,
    
    #[error("Outcomes count mismatch")]
    OutcomesMismatch = 651,
    
    #[error("Price sum exceeds 1.0 (for minting)")]
    PriceSumExceedsOne = 652,
    
    #[error("Price sum below 1.0 (for burning)")]
    PriceSumBelowOne = 653,
    
    #[error("Invalid outcome index")]
    InvalidOutcomeIndex = 654,
    
    #[error("Missing outcome in order set")]
    MissingOutcomeOrder = 655,
    
    // === CPI Errors (700-799) ===
    
    #[error("Invalid CPI caller")]
    InvalidCpiCaller = 700,
    
    #[error("CPI call failed")]
    CpiCallFailed = 701,
    
    #[error("Vault program mismatch")]
    VaultProgramMismatch = 702,
    
    #[error("Fund program mismatch")]
    FundProgramMismatch = 703,
}

impl From<PredictionMarketError> for ProgramError {
    fn from(e: PredictionMarketError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for PredictionMarketError {
    fn type_of() -> &'static str {
        "PredictionMarketError"
    }
}

