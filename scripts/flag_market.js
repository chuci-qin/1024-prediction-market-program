/**
 * FlagMarket - Flag a market for review (Admin only)
 * Run on server: node flag_market.js [market_id] [review_status]
 * 
 * review_status: 0=UnderReview, 1=Approved, 2=Rejected, 3=Flagged
 */

const { 
  Connection, 
  Keypair, 
  PublicKey, 
  Transaction, 
  TransactionInstruction,
  sendAndConfirmTransaction,
} = require('@solana/web3.js');
const fs = require('fs');

const PROGRAM_ID = new PublicKey('FVtPQkdYvSNdpTA6QXYRcTBhDGgnufw2Enqmo2tQKr58');

// Seeds
const PM_CONFIG_SEED = Buffer.from('pm_config');
const MARKET_SEED = Buffer.from('market');

// Instruction index (Initialize=0, ..., FlagMarket=6)
const FLAG_MARKET_IX = 6;

const ReviewStatus = {
  UnderReview: 0,
  Approved: 1,
  Rejected: 2,
  Flagged: 3,
};

/**
 * Serialize FlagMarketArgs
 * Layout:
 * - u8 instruction (6)
 * - u64 market_id
 * - u8 review_status
 */
function serializeFlagMarketArgs(marketId, reviewStatus) {
  const buffer = Buffer.alloc(1 + 8 + 1);
  let offset = 0;
  
  buffer.writeUInt8(FLAG_MARKET_IX, offset); offset += 1;
  buffer.writeBigUInt64LE(BigInt(marketId), offset); offset += 8;
  buffer.writeUInt8(reviewStatus, offset);
  
  return buffer;
}

async function main() {
  const marketId = process.argv[2] ? parseInt(process.argv[2]) : 1;
  const reviewStatus = process.argv[3] ? parseInt(process.argv[3]) : ReviewStatus.Flagged;
  
  const statusNames = ['UnderReview', 'Approved', 'Rejected', 'Flagged'];
  
  console.log('='.repeat(60));
  console.log('1024 Prediction Market - Flag Market');
  console.log('='.repeat(60));
  console.log(`Market ID: ${marketId}`);
  console.log(`Review Status: ${statusNames[reviewStatus]} (${reviewStatus})`);
  
  const connection = new Connection('http://127.0.0.1:8899', 'confirmed');
  
  const faucetPath = process.env.HOME + '/1024chain-testnet/keys/faucet.json';
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
  const instructionData = serializeFlagMarketArgs(marketId, reviewStatus);
  console.log(`\nInstruction data: ${instructionData.toString('hex')}`);
  
  /**
   * Accounts for FlagMarket (from processor.rs):
   * 0. [signer] Admin
   * 1. [] Config
   * 2. [writable] Market
   */
  const flagMarketIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: admin.publicKey, isSigner: true, isWritable: true },
      { pubkey: configPda, isSigner: false, isWritable: false },
      { pubkey: marketPda, isSigner: false, isWritable: true },
    ],
    data: instructionData,
  });
  
  const tx = new Transaction().add(flagMarketIx);
  
  console.log('\nSending FlagMarket transaction...');
  
  try {
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.lastValidBlockHeight = lastValidBlockHeight;
    tx.feePayer = admin.publicKey;
    
    const signature = await sendAndConfirmTransaction(connection, tx, [admin], {
      commitment: 'confirmed',
    });
    
    console.log('\n✅ FlagMarket successful!');
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
