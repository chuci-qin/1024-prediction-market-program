/**
 * E2E Test Suite Configuration
 * 1024 Prediction Market Program
 */

const { PublicKey } = require('@solana/web3.js');

module.exports = {
    // Network Configuration
    RPC_URL: 'https://testnet-rpc.1024chain.com/rpc/',
    COMMITMENT: 'confirmed',
    
    // Program IDs
    PROGRAM_ID: new PublicKey('FVtPQkdYvSNdpTA6QXYRcTBhDGgnufw2Enqmo2tQKr58'),
    VAULT_PROGRAM: new PublicKey('vR3BifKCa2TGKP2uhToxZAMYAYydqpesvKGX54gzFny'),
    USDC_MINT: new PublicKey('7pCrfxhcAEyTFDhrhKRtRS2iMvEYx2dtNE7NzwuU7SA9'),
    
    // Admin Keypair Path
    ADMIN_KEYPAIR_PATH: '/Users/chuciqin/Desktop/project1024/1024codebase/1024-chain/keys/faucet.json',
    
    // Test Accounts
    TEST_ACCOUNTS: [
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
    ],
    
    // PDA Seeds
    SEEDS: {
        PM_CONFIG: Buffer.from('pm_config'),
        MARKET: Buffer.from('market'),
        ORDER: Buffer.from('order'),
        OUTCOME_MINT: Buffer.from('outcome_mint'),
        POSITION: Buffer.from('position'),
        ORDER_ESCROW: Buffer.from('order_escrow'),
        AUTHORIZED_CALLERS: Buffer.from('authorized_callers')
    },
    
    // Instruction Indices
    INSTRUCTIONS: {
        INITIALIZE_CONFIG: 0,
        CREATE_MARKET: 8,
        REINITIALIZE_CONFIG: 9,
        PLACE_ORDER: 10,
        ACTIVATE_MARKET: 11,
        MATCH_MINT: 12,
        MATCH_BURN: 13,
        CANCEL_ORDER: 14,
        ADD_AUTHORIZED_CALLER: 24,
        CREATE_MULTI_OUTCOME_MARKET: 27,
        PLACE_MULTI_OUTCOME_ORDER: 30,
        MATCH_MINT_MULTI: 43,
        MATCH_BURN_MULTI: 44
    },
    
    // Market Status
    MARKET_STATUS: {
        PENDING: 0,
        ACTIVE: 1,
        PAUSED: 2,
        RESOLVED: 3,
        FINALIZED: 4,
        CANCELLED: 5
    },
    
    // Order Status
    ORDER_STATUS: {
        OPEN: 0,
        PARTIALLY_FILLED: 1,
        FILLED: 2,
        CANCELLED: 3,
        EXPIRED: 4
    },
    
    // Order Side
    ORDER_SIDE: {
        BUY: 0,
        SELL: 1
    },
    
    // Timing
    TX_WAIT_MS: 2000,
    BLOCK_WAIT_MS: 500
};

