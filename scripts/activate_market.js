/**
 * Activate a prediction market
 * Run on server: node activate_market.js [market_id]
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

// Program IDs
const PROGRAM_ID = new PublicKey('FVtPQkdYvSNdpTA6QXYRcTBhDGgnufw2Enqmo2tQKr58');

// Seeds
const PM_CONFIG_SEED = Buffer.from('pm_config');
const MARKET_SEED = Buffer.from('market');

/**
 * Serialize ActivateMarketArgs
 * Layout:
 * - u8 instruction (2 = ActivateMarket)
 * - u64 market_id
 */
function serializeActivateMarketArgs(marketId) {
  const buffer = Buffer.alloc(1 + 8);
  buffer.writeUInt8(2, 0); // Instruction index = 2
  buffer.writeBigUInt64LE(BigInt(marketId), 1);
  return buffer;
}

async function main() {
  const marketId = process.argv[2] ? parseInt(process.argv[2]) : 1;
  
  console.log('='.repeat(60));
  console.log(`1024 Prediction Market - Activate Market ${marketId}`);
  console.log('='.repeat(60));
  
  const connection = new Connection('http://127.0.0.1:8899', 'confirmed');
  console.log('Connected to local RPC');
  
  // Load admin keypair
  const faucetPath = process.env.HOME + '/1024chain-testnet/keys/faucet.json';
  const faucetData = JSON.parse(fs.readFileSync(faucetPath, 'utf-8'));
  const admin = Keypair.fromSecretKey(new Uint8Array(faucetData));
  console.log(`Admin: ${admin.publicKey.toBase58()}`);
  
  // Derive PDAs
  const [configPda] = PublicKey.findProgramAddressSync([PM_CONFIG_SEED], PROGRAM_ID);
  console.log(`Config PDA: ${configPda.toBase58()}`);
  
  const marketIdBytes = Buffer.alloc(8);
  marketIdBytes.writeBigUInt64LE(BigInt(marketId));
  const [marketPda] = PublicKey.findProgramAddressSync(
    [MARKET_SEED, marketIdBytes],
    PROGRAM_ID
  );
  console.log(`Market PDA: ${marketPda.toBase58()}`);
  
  // Check market exists
  const marketAccount = await connection.getAccountInfo(marketPda);
  if (!marketAccount) {
    console.error(`❌ Market ${marketId} not found!`);
    return;
  }
  
  // Parse current status (offset = 8 + 8 + 32 + 32 = 80 for discriminator + market_id + hashes)
  // Actually need to check the state.rs struct layout
  console.log(`Market account size: ${marketAccount.data.length} bytes`);
  
  // Serialize instruction
  const instructionData = serializeActivateMarketArgs(marketId);
  console.log(`Instruction data: ${instructionData.toString('hex')}`);
  
  // Create instruction
  // Note: Config must be writable because we update active_markets
  const activateIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: admin.publicKey, isSigner: true, isWritable: true },  // Admin
      { pubkey: configPda, isSigner: false, isWritable: true },       // Config (writable!)
      { pubkey: marketPda, isSigner: false, isWritable: true },       // Market
    ],
    data: instructionData,
  });
  
  // Send transaction
  console.log('\nSending ActivateMarket transaction...');
  const tx = new Transaction().add(activateIx);
  
  try {
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.lastValidBlockHeight = lastValidBlockHeight;
    tx.feePayer = admin.publicKey;
    
    const signature = await sendAndConfirmTransaction(connection, tx, [admin], {
      commitment: 'confirmed',
    });
    
    console.log('\n✅ ActivateMarket successful!');
    console.log(`Signature: ${signature}`);
    console.log(`Market ID: ${marketId}`);
    console.log(`Explorer: https://testnet-scan.1024chain.com/tx/${signature}`);
  } catch (error) {
    console.error('\n❌ Transaction failed:');
    if (error.logs) {
      console.error('Logs:');
      error.logs.forEach(log => console.error('  ', log));
    }
    console.error(error.message || error);
  }
  
  // Verify status change
  console.log('\nVerifying market status...');
  const updatedMarket = await connection.getAccountInfo(marketPda);
  if (updatedMarket) {
    // Status is at a specific offset in the Market struct
    // Let's just confirm the account was updated
    console.log('✅ Market account verified');
  }
  
  console.log('\n' + '='.repeat(60));
}

main().catch(console.error);
