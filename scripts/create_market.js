/**
 * Create a test prediction market
 * Run on server: node create_market.js
 */

const { 
  Connection, 
  Keypair, 
  PublicKey, 
  Transaction, 
  TransactionInstruction,
  SystemProgram,
  sendAndConfirmTransaction,
  SYSVAR_RENT_PUBKEY,
} = require('@solana/web3.js');
const config = require('./config');
const crypto = require('crypto');
const fs = require('fs');

// Program IDs
const PROGRAM_ID = config.PROGRAM_ID;
const TOKEN_PROGRAM_ID = new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
const USDC_MINT = new PublicKey('7pCrfxhcAEyTFDhrhKRtRS2iMvEYx2dtNE7NzwuU7SA9');

// Seeds (must match state.rs)
const PM_CONFIG_SEED = Buffer.from('pm_config');
const MARKET_SEED = Buffer.from('market');
const YES_MINT_SEED = Buffer.from('yes_mint');
const NO_MINT_SEED = Buffer.from('no_mint');
const MARKET_VAULT_SEED = Buffer.from('market_vault');

/**
 * Serialize CreateMarketArgs matching the Rust struct:
 * - question_hash: [u8; 32]
 * - resolution_spec_hash: [u8; 32]
 * - resolution_time: i64
 * - finalization_deadline: i64
 * - creator_fee_bps: u16
 */
function serializeCreateMarketArgs(questionHash, resolutionSpecHash, resolutionTime, finalizationDeadline, creatorFeeBps) {
  const buffer = Buffer.alloc(1 + 32 + 32 + 8 + 8 + 2); // 83 bytes
  let offset = 0;
  
  // Instruction index = 2 (CreateMarket is the 3rd variant in the enum)
  // 0 = Initialize, 1 = ReinitializeConfig, 2 = CreateMarket
  buffer.writeUInt8(2, offset);
  offset += 1;
  
  // Question hash (32 bytes)
  questionHash.copy(buffer, offset);
  offset += 32;
  
  // Resolution spec hash (32 bytes)
  resolutionSpecHash.copy(buffer, offset);
  offset += 32;
  
  // Resolution time (i64)
  buffer.writeBigInt64LE(BigInt(resolutionTime), offset);
  offset += 8;
  
  // Finalization deadline (i64)
  buffer.writeBigInt64LE(BigInt(finalizationDeadline), offset);
  offset += 8;
  
  // Creator fee bps (u16)
  buffer.writeUInt16LE(creatorFeeBps, offset);
  
  return buffer;
}

/**
 * Calculate PredictionMarketConfig offset for next_market_id
 * Layout:
 * - discriminator: u64 (8)
 * - admin: Pubkey (32)
 * - usdc_mint: Pubkey (32)
 * - vault_program: Pubkey (32)
 * - fund_program: Pubkey (32)
 * - oracle_admin: Pubkey (32)
 * - next_market_id: u64 (8) <-- offset = 168
 */
const NEXT_MARKET_ID_OFFSET = 8 + 32 + 32 + 32 + 32 + 32; // = 168

async function main() {
  console.log('='.repeat(60));
  console.log('1024 Prediction Market - Create Market Test');
  console.log('='.repeat(60));
  
  const connection = new Connection(config.RPC_URL, 'confirmed');
  console.log('Connected to local RPC');
  
  // Load admin keypair
  const faucetPath = process.env.ADMIN_KEYPAIR || '../faucet.json';
  const faucetData = JSON.parse(fs.readFileSync(faucetPath, 'utf-8'));
  const admin = Keypair.fromSecretKey(new Uint8Array(faucetData));
  console.log(`Admin/Creator: ${admin.publicKey.toBase58()}`);
  
  // Derive PDAs
  const [configPda] = PublicKey.findProgramAddressSync([PM_CONFIG_SEED], PROGRAM_ID);
  console.log(`Config PDA: ${configPda.toBase58()}`);
  
  // Get next market_id from config
  const configAccount = await connection.getAccountInfo(configPda);
  if (!configAccount) {
    console.error('❌ Config not initialized! Run init_program.js first.');
    return;
  }
  
  // Parse next_market_id from config at correct offset
  const nextMarketId = configAccount.data.readBigUInt64LE(NEXT_MARKET_ID_OFFSET);
  console.log(`Next Market ID: ${nextMarketId}`);
  
  const marketIdBytes = Buffer.alloc(8);
  marketIdBytes.writeBigUInt64LE(nextMarketId);
  
  const [marketPda] = PublicKey.findProgramAddressSync(
    [MARKET_SEED, marketIdBytes],
    PROGRAM_ID
  );
  console.log(`Market PDA: ${marketPda.toBase58()}`);
  
  // Derive YES/NO mint PDAs (seeds must match state.rs)
  const [yesMintPda] = PublicKey.findProgramAddressSync(
    [YES_MINT_SEED, marketIdBytes],
    PROGRAM_ID
  );
  const [noMintPda] = PublicKey.findProgramAddressSync(
    [NO_MINT_SEED, marketIdBytes],
    PROGRAM_ID
  );
  const [vaultPda] = PublicKey.findProgramAddressSync(
    [MARKET_VAULT_SEED, marketIdBytes],
    PROGRAM_ID
  );
  
  console.log(`YES Mint PDA: ${yesMintPda.toBase58()}`);
  console.log(`NO Mint PDA: ${noMintPda.toBase58()}`);
  console.log(`Vault PDA: ${vaultPda.toBase58()}`);
  
  // Create market parameters
  const now = Math.floor(Date.now() / 1000);
  const resolutionTime = now + (7 * 24 * 60 * 60); // 7 days from now
  const finalizationDeadline = resolutionTime + (3 * 24 * 60 * 60); // 3 days after resolution
  
  // Create hashes for question and resolution spec
  const question = "Will BTC price exceed $150,000 by end of 2025?";
  const resolutionSpec = "This market resolves YES if Bitcoin's price on any major exchange (Binance, Coinbase, Kraken) exceeds $150,000 USD before December 31, 2025 23:59 UTC.";
  
  const questionHash = crypto.createHash('sha256').update(question).digest();
  const resolutionSpecHash = crypto.createHash('sha256').update(resolutionSpec).digest();
  
  const creatorFeeBps = 50; // 0.5%
  
  console.log('\nMarket Details:');
  console.log(`  Question: ${question}`);
  console.log(`  Question Hash: ${questionHash.toString('hex').slice(0, 16)}...`);
  console.log(`  Resolution Time: ${new Date(resolutionTime * 1000).toISOString()}`);
  console.log(`  Finalization Deadline: ${new Date(finalizationDeadline * 1000).toISOString()}`);
  console.log(`  Creator Fee: ${creatorFeeBps / 100}%`);
  
  // Serialize instruction data
  const instructionData = serializeCreateMarketArgs(
    questionHash,
    resolutionSpecHash,
    resolutionTime,
    finalizationDeadline,
    creatorFeeBps
  );
  
  console.log(`\nInstruction data size: ${instructionData.length} bytes`);
  console.log(`Instruction data (hex): ${instructionData.toString('hex')}`);
  
  // Create instruction
  const createMarketIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: admin.publicKey, isSigner: true, isWritable: true },  // Creator/payer
      { pubkey: configPda, isSigner: false, isWritable: true },       // Config
      { pubkey: marketPda, isSigner: false, isWritable: true },       // Market
      { pubkey: yesMintPda, isSigner: false, isWritable: true },      // YES Mint
      { pubkey: noMintPda, isSigner: false, isWritable: true },       // NO Mint
      { pubkey: vaultPda, isSigner: false, isWritable: true },        // Market Vault
      { pubkey: USDC_MINT, isSigner: false, isWritable: false },      // USDC Mint
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false }, // Token Program
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }, // System
      { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false }, // Rent
    ],
    data: instructionData,
  });
  
  // Send transaction
  console.log('\nSending transaction...');
  const tx = new Transaction().add(createMarketIx);
  
  try {
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.lastValidBlockHeight = lastValidBlockHeight;
    tx.feePayer = admin.publicKey;
    
    const signature = await sendAndConfirmTransaction(connection, tx, [admin], {
      commitment: 'confirmed',
    });
    
    console.log('\n✅ CreateMarket successful!');
    console.log(`Signature: ${signature}`);
    console.log(`Market ID: ${nextMarketId}`);
    console.log(`Market PDA: ${marketPda.toBase58()}`);
    console.log(`Explorer: https://testnet-scan.1024chain.com/tx/${signature}`);
  } catch (error) {
    console.error('\n❌ Transaction failed:');
    if (error.logs) {
      console.error('Logs:');
      error.logs.forEach(log => console.error('  ', log));
    }
    console.error(error.message || error);
  }
  
  // Verify market creation
  console.log('\nVerifying market...');
  const marketAccount = await connection.getAccountInfo(marketPda);
  if (marketAccount) {
    console.log('✅ Market account created');
    console.log(`  Size: ${marketAccount.data.length} bytes`);
    console.log(`  Owner: ${marketAccount.owner.toBase58()}`);
  }
  
  console.log('\n' + '='.repeat(60));
}

main().catch(console.error);
