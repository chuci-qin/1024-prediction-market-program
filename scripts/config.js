/**
 * Unified Configuration for 1024 Prediction Market Program Scripts
 * 
 * All scripts should import from this file for consistent configuration.
 * Supports environment variable overrides for deployment flexibility.
 * 
 * Usage:
 *   const { PROGRAM_ID, VAULT_PROGRAM, RPC_URL } = require('./config');
 * 
 * Environment Variables:
 *   - PM_PROGRAM_ID: Override Prediction Market Program ID
 *   - VAULT_PROGRAM_ID: Override Vault Program ID
 *   - SOLANA_RPC_URL: Override RPC endpoint
 *   - USDC_MINT: Override USDC mint address
 *   - ADMIN_KEYPAIR_PATH: Override admin keypair path
 */

const { PublicKey } = require('@solana/web3.js');

// ============================================================================
// Network Configuration
// ============================================================================

const RPC_URL = process.env.SOLANA_RPC_URL || 'https://testnet-rpc.1024chain.com/rpc/';
const COMMITMENT = 'confirmed';

// ============================================================================
// Program IDs (支持环境变量覆盖)
// ============================================================================

const PROGRAM_ID = new PublicKey(
    process.env.PM_PROGRAM_ID || '9hsG1DksmgadjjJTEEX7CdevQKYVkQag3mEratPRZXjv'
);

const VAULT_PROGRAM = new PublicKey(
    process.env.VAULT_PROGRAM_ID || 'vR3BifKCa2TGKP2uhToxZAMYAYydqpesvKGX54gzFny'
);

const USDC_MINT = new PublicKey(
    process.env.USDC_MINT || '7pCrfxhcAEyTFDhrhKRtRS2iMvEYx2dtNE7NzwuU7SA9'
);

const TOKEN_PROGRAM_ID = new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
const ASSOCIATED_TOKEN_PROGRAM_ID = new PublicKey('ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL');
const SYSTEM_PROGRAM_ID = new PublicKey('11111111111111111111111111111111');

// ============================================================================
// Admin Keypair Path
// ============================================================================

const ADMIN_KEYPAIR_PATH = process.env.ADMIN_KEYPAIR_PATH || 
    '/Users/chuciqin/Desktop/project1024/1024codebase/1024-chain/keys/faucet.json';

// ============================================================================
// Test Accounts
// ============================================================================

const TEST_ACCOUNTS = [
    {
        name: 'User1',
        pubkey: '9ocm9zv5F2QghKaFSLGSjkVg6f8XZf54nVTjfC2M3dG4',
        secret: '65d7pAydmKwgo5mVBwnKQUS7BUP1ZBhisEbeRyfzFnGLez85AGSqcqbZCUbsccogzSyLBqYcoZVgU7x7AARtKMHz'
    },
    {
        name: 'User2',
        pubkey: 'G23icA8QJiAM2UwENf1112rGFxoqHP6JJa3TuwVseVxu',
        secret: '2Rc3q4XFhUeZE5LUQCCzuMDVy4iom7mevWgCFCeMobNWAymrNAGe8UEXKkfVJQHb4af4F81JJL86qQz16a1wnv4y'
    },
    {
        name: 'User3',
        pubkey: '9S55H6Bbh2JCqdmQGcw2MWCdWeBNNQYb9GWiCHL62CUH',
        secret: '5isgvaK7oNcxNEctu6hRyYf7z1xEavfMRKmNGb6h9Ect2iFXtA9qKCFhWFhvxSzPJQBBMePuQ5Sd4VUYEKtd3oaq'
    }
];

// ============================================================================
// PDA Seeds (must match state.rs)
// ============================================================================

const SEEDS = {
    PM_CONFIG: Buffer.from('pm_config'),
    MARKET: Buffer.from('market'),
    ORDER: Buffer.from('order'),
    OUTCOME_MINT: Buffer.from('outcome_mint'),
    YES_MINT: Buffer.from('yes_mint'),
    NO_MINT: Buffer.from('no_mint'),
    POSITION: Buffer.from('position'),
    ORDER_ESCROW: Buffer.from('order_escrow'),
    MARKET_VAULT: Buffer.from('market_vault'),
    AUTHORIZED_CALLERS: Buffer.from('authorized_callers'),
    ORACLE_PROPOSAL: Buffer.from('oracle_proposal'),
    ORACLE_PROPOSAL_DATA: Buffer.from('oracle_proposal_data'),
    MARKET_ORACLE_DATA: Buffer.from('market_oracle_data'),
    MULTI_OUTCOME_POSITION: Buffer.from('multi_outcome_position')
};

// ============================================================================
// Instruction Indices (must match instruction.rs)
// ============================================================================

const INSTRUCTIONS = {
    INITIALIZE_CONFIG: 0,
    CREATE_MARKET: 8,
    REINITIALIZE_CONFIG: 9,
    PLACE_ORDER: 10,
    ACTIVATE_MARKET: 11,
    MATCH_MINT: 12,
    MATCH_BURN: 13,
    CANCEL_ORDER: 14,
    EXECUTE_TRADE: 15,
    MINT_COMPLETE_SET: 16,
    REDEEM_COMPLETE_SET: 17,
    CLAIM_WINNINGS: 18,
    PROPOSE_RESULT: 19,
    FINALIZE_RESULT: 20,
    CHALLENGE_RESULT: 21,
    CANCEL_MARKET: 22,
    PAUSE_MARKET: 23,
    ADD_AUTHORIZED_CALLER: 24,
    RESUME_MARKET: 25,
    FLAG_MARKET: 26,
    CREATE_MULTI_OUTCOME_MARKET: 27,
    MINT_MULTI_OUTCOME_SET: 28,
    REDEEM_MULTI_OUTCOME_SET: 29,
    PLACE_MULTI_OUTCOME_ORDER: 30,
    CANCEL_MULTI_OUTCOME_ORDER: 31,
    CLAIM_MULTI_OUTCOME_WINNINGS: 32,
    UPDATE_ORACLE_CONFIG: 33,
    PROPOSE_RESULT_WITH_RESEARCH: 34,
    PROPOSE_RESULT_MANUAL: 35,
    CHALLENGE_RESULT_WITH_EVIDENCE: 36,
    RELAYER_MINT_COMPLETE_SET_V2: 50,
    RELAYER_REDEEM_COMPLETE_SET_V2: 51,
    RELAYER_PLACE_ORDER_V2: 52,
    RELAYER_CANCEL_ORDER_V2: 53,
    RELAYER_MINT_COMPLETE_SET_V2_WITH_FEE: 57,
    RELAYER_REDEEM_COMPLETE_SET_V2_WITH_FEE: 58,
    MATCH_MINT_MULTI: 43,
    MATCH_BURN_MULTI: 44,
    RELAYER_CHALLENGE_RESULT_V2: 61  // New instruction for relayer-signed challenge
};

// ============================================================================
// Market Status
// ============================================================================

const MARKET_STATUS = {
    PENDING: 0,
    ACTIVE: 1,
    PAUSED: 2,
    RESOLVED: 3,
    FINALIZED: 4,
    CANCELLED: 5,
    RESULT_PROPOSED: 6,
    CHALLENGED: 7
};

// ============================================================================
// Order Status
// ============================================================================

const ORDER_STATUS = {
    OPEN: 0,
    PARTIALLY_FILLED: 1,
    FILLED: 2,
    CANCELLED: 3,
    EXPIRED: 4
};

// ============================================================================
// Order Side
// ============================================================================

const ORDER_SIDE = {
    BUY: 0,
    SELL: 1
};

// ============================================================================
// Timing
// ============================================================================

const TX_WAIT_MS = 2000;
const BLOCK_WAIT_MS = 500;

// ============================================================================
// Exports
// ============================================================================

module.exports = {
    // Network
    RPC_URL,
    COMMITMENT,
    
    // Program IDs
    PROGRAM_ID,
    VAULT_PROGRAM,
    USDC_MINT,
    TOKEN_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID,
    SYSTEM_PROGRAM_ID,
    
    // Paths
    ADMIN_KEYPAIR_PATH,
    
    // Accounts
    TEST_ACCOUNTS,
    
    // PDA Seeds
    SEEDS,
    
    // Instructions
    INSTRUCTIONS,
    
    // Enums
    MARKET_STATUS,
    ORDER_STATUS,
    ORDER_SIDE,
    
    // Timing
    TX_WAIT_MS,
    BLOCK_WAIT_MS
};


