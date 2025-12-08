//! 1024 Prediction Market Program
//! 
//! Core logic for prediction market trading on Solana.
//!
//! ## Architecture
//!
//! This program works in conjunction with:
//! - `1024-vault-program`: User fund custody (deposits, withdrawals, locked margin)
//! - `1024-fund-program`: Fee pools and maker rewards
//!
//! ## Key Features
//!
//! - Complete Set minting/redemption (1 USDC = 1 YES + 1 NO)
//! - Order book based trading (off-chain matching, on-chain settlement)
//! - Oracle integration for result resolution
//! - Market creation and lifecycle management

pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;
pub mod utils;
pub mod cpi;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

// Re-export commonly used items
pub use error::PredictionMarketError;
pub use instruction::PredictionMarketInstruction;
pub use state::*;

// Program ID - will be updated after deployment
solana_program::declare_id!("PMkt1111111111111111111111111111111111111111");

