/**
 * Pause and Resume a prediction market
 * Run on server: node pause_resume_market.js [market_id] [action]
 * action: 'pause' or 'resume'
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
const PM_CONFIG_SEED = Buffer.from('pm_config');
const MARKET_SEED = Buffer.from('market');

// Instruction indices
const PAUSE_MARKET_IX = 3;
const RESUME_MARKET_IX = 4;

function serializeMarketIdArgs(instructionIndex, marketId) {
  const buffer = Buffer.alloc(1 + 8);
  buffer.writeUInt8(instructionIndex, 0);
  buffer.writeBigUInt64LE(BigInt(marketId), 1);
  return buffer;
}

async function main() {
  const marketId = process.argv[2] ? parseInt(process.argv[2]) : 1;
  const action = process.argv[3] || 'pause';
  
  console.log('='.repeat(60));
  console.log(`1024 Prediction Market - ${action.toUpperCase()} Market ${marketId}`);
  console.log('='.repeat(60));
  
  const connection = new Connection('http://127.0.0.1:8899', 'confirmed');
  
  const faucetPath = process.env.HOME + '/1024chain-testnet/keys/faucet.json';
  const faucetData = JSON.parse(fs.readFileSync(faucetPath, 'utf-8'));
  const admin = Keypair.fromSecretKey(new Uint8Array(faucetData));
  console.log(`Admin: ${admin.publicKey.toBase58()}`);
  
  const [configPda] = PublicKey.findProgramAddressSync([PM_CONFIG_SEED], PROGRAM_ID);
  const marketIdBytes = Buffer.alloc(8);
  marketIdBytes.writeBigUInt64LE(BigInt(marketId));
  const [marketPda] = PublicKey.findProgramAddressSync([MARKET_SEED, marketIdBytes], PROGRAM_ID);
  
  console.log(`Market PDA: ${marketPda.toBase58()}`);
  
  const instructionIndex = action === 'pause' ? PAUSE_MARKET_IX : RESUME_MARKET_IX;
  const instructionData = serializeMarketIdArgs(instructionIndex, marketId);
  
  const ix = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: admin.publicKey, isSigner: true, isWritable: true },
      { pubkey: configPda, isSigner: false, isWritable: true },
      { pubkey: marketPda, isSigner: false, isWritable: true },
    ],
    data: instructionData,
  });
  
  const tx = new Transaction().add(ix);
  
  try {
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.lastValidBlockHeight = lastValidBlockHeight;
    tx.feePayer = admin.publicKey;
    
    const signature = await sendAndConfirmTransaction(connection, tx, [admin], {
      commitment: 'confirmed',
    });
    
    console.log(`\n✅ ${action.toUpperCase()} Market successful!`);
    console.log(`Signature: ${signature}`);
  } catch (error) {
    console.error(`\n❌ ${action.toUpperCase()} failed:`, error.message);
    if (error.logs) error.logs.forEach(log => console.error('  ', log));
  }
  
  console.log('='.repeat(60));
}

main().catch(console.error);
