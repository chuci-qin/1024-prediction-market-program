/**
 * CancelMarket - Cancel a market (Admin only)
 * Run on server: node cancel_market.js [market_id] [reason]
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
const MARKET_SEED = Buffer.from('market');

// Instruction index (Initialize=0, ..., CancelMarket=5)
const CANCEL_MARKET_IX = 5;

/**
 * Serialize CancelMarketArgs
 * Layout:
 * - u8 instruction (5)
 * - u64 market_id
 * - u8 reason (enum: 0=AdminDecision, 1=InvalidMarket, 2=MarketExpired, etc.)
 */
function serializeCancelMarketArgs(marketId, reason) {
  const buffer = Buffer.alloc(1 + 8 + 1);
  let offset = 0;
  
  buffer.writeUInt8(CANCEL_MARKET_IX, offset); offset += 1;
  buffer.writeBigUInt64LE(BigInt(marketId), offset); offset += 8;
  buffer.writeUInt8(reason, offset);
  
  return buffer;
}

const CancelReason = {
  AdminDecision: 0,
  InvalidMarket: 1,
  MarketExpired: 2,
  DisputeResolution: 3,
};

async function main() {
  const marketId = process.argv[2] ? parseInt(process.argv[2]) : 1;
  const reason = process.argv[3] ? parseInt(process.argv[3]) : CancelReason.AdminDecision;
  
  const reasonNames = ['AdminDecision', 'InvalidMarket', 'MarketExpired', 'DisputeResolution'];
  
  console.log('='.repeat(60));
  console.log('1024 Prediction Market - Cancel Market');
  console.log('='.repeat(60));
  console.log(`Market ID: ${marketId}`);
  console.log(`Reason: ${reasonNames[reason]} (${reason})`);
  
  const connection = new Connection(config.RPC_URL, 'confirmed');
  
  const faucetPath = '/Users/patrick/Developer/1024ex/faucet.json';
  const faucetData = JSON.parse(fs.readFileSync(faucetPath, 'utf-8'));
  const admin = Keypair.fromSecretKey(new Uint8Array(faucetData));
  console.log(`\nAdmin: ${admin.publicKey.toBase58()}`);
  
  // Derive PDAs
  const [configPda] = PublicKey.findProgramAddressSync([PM_CONFIG_SEED], PROGRAM_ID);
  
  const marketIdBytes = Buffer.alloc(8);
  marketIdBytes.writeBigUInt64LE(BigInt(marketId));
  const [marketPda] = PublicKey.findProgramAddressSync([MARKET_SEED, marketIdBytes], PROGRAM_ID);
  
  console.log(`  Config PDA: ${configPda.toBase58()}`);
  console.log(`  Market PDA: ${marketPda.toBase58()}`);
  
  // Serialize instruction
  const instructionData = serializeCancelMarketArgs(marketId, reason);
  console.log(`\nInstruction data: ${instructionData.toString('hex')}`);
  
  /**
   * Accounts for CancelMarket (from processor.rs):
   * 0. [signer] Admin
   * 1. [writable] Config
   * 2. [writable] Market
   */
  const cancelMarketIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: admin.publicKey, isSigner: true, isWritable: true },
      { pubkey: configPda, isSigner: false, isWritable: true },
      { pubkey: marketPda, isSigner: false, isWritable: true },
    ],
    data: instructionData,
  });
  
  const tx = new Transaction().add(cancelMarketIx);
  
  console.log('\nSending CancelMarket transaction...');
  
  try {
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.lastValidBlockHeight = lastValidBlockHeight;
    tx.feePayer = admin.publicKey;
    
    const signature = await sendAndConfirmTransaction(connection, tx, [admin], {
      commitment: 'confirmed',
    });
    
    console.log('\n✅ CancelMarket successful!');
    console.log(`Signature: ${signature}`);
    console.log(`Explorer: https://testnet-scan.1024chain.com/tx/${signature}`);
    
  } catch (error) {
    console.error('\n❌ Transaction failed:');
    if (error.logs) {
      console.error('Logs:');
      error.logs.forEach(log => console.error('  ', log));
    }
    console.error(error.message || error);
  }
  
  console.log('\n' + '='.repeat(60));
}

main().catch(console.error);
