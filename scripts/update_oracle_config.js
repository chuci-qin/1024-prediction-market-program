/**
 * UpdateOracleConfig - Update oracle configuration
 * Run on server: node update_oracle_config.js [challenge_window_secs]
 */

const { 
  Connection, 
  Keypair, 
  PublicKey, 
  Transaction, 
  TransactionInstruction,
  sendAndConfirmTransaction,
} = require('@solana/web3.js');
const config = require('./config');
const fs = require('fs');

const PROGRAM_ID = config.PROGRAM_ID;

// Seeds
const PM_CONFIG_SEED = Buffer.from('pm_config');

// Instruction index - UpdateOracleConfig is near the end
// Count: Initialize=0, CreateMarket=1, Activate=2, Pause=3, Resume=4, Cancel=5, Flag=6,
// MintComplete=7, RedeemComplete=8, PlaceOrder=9, CancelOrder=10, MatchMint=11, MatchBurn=12,
// ExecuteTrade=13, ProposeResult=14, ChallengeResult=15, FinalizeResult=16, ResolveDispute=17,
// ClaimWinnings=18, RefundCancelled=19, SetPaused=20, UpdateAdmin=21, UpdateOracleAdmin=22,
// UpdateOracleConfig=23, AddAuthorizedCaller=24, RemoveAuthorizedCaller=25
const UPDATE_ORACLE_CONFIG_IX = 23;

/**
 * Serialize UpdateOracleConfigArgs
 * Layout:
 * - u8 instruction (23)
 * - Option<i64> challenge_window_secs (1 byte tag + 8 bytes if Some)
 * - Option<u64> proposer_bond_e6 (1 byte tag + 8 bytes if Some)
 */
function serializeUpdateOracleConfigArgs(challengeWindowSecs, proposerBondE6) {
  let size = 1;
  if (challengeWindowSecs !== null) size += 1 + 8;
  else size += 1;
  
  if (proposerBondE6 !== null) size += 1 + 8;
  else size += 1;
  
  const buffer = Buffer.alloc(size);
  let offset = 0;
  
  buffer.writeUInt8(UPDATE_ORACLE_CONFIG_IX, offset); offset += 1;
  
  if (challengeWindowSecs !== null) {
    buffer.writeUInt8(1, offset); offset += 1; // Some
    buffer.writeBigInt64LE(BigInt(challengeWindowSecs), offset); offset += 8;
  } else {
    buffer.writeUInt8(0, offset); offset += 1; // None
  }
  
  if (proposerBondE6 !== null) {
    buffer.writeUInt8(1, offset); offset += 1; // Some
    buffer.writeBigUInt64LE(BigInt(proposerBondE6), offset);
  } else {
    buffer.writeUInt8(0, offset); offset += 1; // None
  }
  
  return buffer;
}

async function main() {
  const challengeWindowSecs = process.argv[2] ? parseInt(process.argv[2]) : 60; // Default: 1 minute
  
  console.log('='.repeat(60));
  console.log('1024 Prediction Market - Update Oracle Config');
  console.log('='.repeat(60));
  console.log(`New Challenge Window: ${challengeWindowSecs} seconds`);
  
  const connection = new Connection('http://127.0.0.1:8899', 'confirmed');
  
  const faucetPath = process.env.HOME + '/1024chain-testnet/keys/faucet.json';
  const faucetData = JSON.parse(fs.readFileSync(faucetPath, 'utf-8'));
  const admin = Keypair.fromSecretKey(new Uint8Array(faucetData));
  console.log(`\nAdmin: ${admin.publicKey.toBase58()}`);
  
  const [configPda] = PublicKey.findProgramAddressSync([PM_CONFIG_SEED], PROGRAM_ID);
  console.log(`Config PDA: ${configPda.toBase58()}`);
  
  // Query current config
  const configAccount = await connection.getAccountInfo(configPda);
  if (configAccount) {
    // challenge_window_secs is at offset 176 (after next_market_id at 168)
    const currentChallenge = Number(configAccount.data.readBigInt64LE(176));
    console.log(`Current Challenge Window: ${currentChallenge} seconds (${currentChallenge/3600} hours)`);
  }
  
  const instructionData = serializeUpdateOracleConfigArgs(challengeWindowSecs, null);
  console.log(`\nInstruction data: ${instructionData.toString('hex')}`);
  
  /**
   * Accounts for UpdateOracleConfig:
   * 0. [signer] Admin
   * 1. [writable] Config
   */
  const ix = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: admin.publicKey, isSigner: true, isWritable: true },
      { pubkey: configPda, isSigner: false, isWritable: true },
    ],
    data: instructionData,
  });
  
  const tx = new Transaction().add(ix);
  
  console.log('\nSending UpdateOracleConfig transaction...');
  
  try {
    const { blockhash } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.feePayer = admin.publicKey;
    
    const signature = await sendAndConfirmTransaction(connection, tx, [admin], {
      commitment: 'confirmed',
    });
    
    console.log('\n✅ UpdateOracleConfig successful!');
    console.log(`Signature: ${signature}`);
    
    // Verify change
    const newConfig = await connection.getAccountInfo(configPda);
    const newChallenge = Number(newConfig.data.readBigInt64LE(176));
    console.log(`New Challenge Window: ${newChallenge} seconds`);
    
  } catch (error) {
    console.error('\n❌ Transaction failed:');
    if (error.logs) {
      error.logs.forEach(log => console.error('  ', log));
    }
    console.error(error.message || error);
  }
  
  console.log('\n' + '='.repeat(60));
}

main().catch(console.error);
