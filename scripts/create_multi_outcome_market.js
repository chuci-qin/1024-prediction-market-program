/**
 * Create a Multi-Outcome Prediction Market
 * Usage: node create_multi_outcome_market.js [num_outcomes] [question]
 * Example: node create_multi_outcome_market.js 3 "Who will win the election?"
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
const MARKET_VAULT_SEED = Buffer.from('market_vault');
const OUTCOME_MINT_SEED = Buffer.from('outcome_mint');

// Instruction index for CreateMultiOutcomeMarket = 26 (27th variant in enum, 0-indexed)
const INSTRUCTION_INDEX = 26;

/**
 * Serialize CreateMultiOutcomeMarketArgs matching the Rust struct:
 * - question_hash: [u8; 32]
 * - resolution_spec_hash: [u8; 32]
 * - num_outcomes: u8
 * - outcome_hashes: Vec<[u8; 32]>
 * - resolution_time: i64
 * - finalization_deadline: i64
 * - creator_fee_bps: u16
 */
function serializeCreateMultiOutcomeMarketArgs(
  questionHash, 
  resolutionSpecHash, 
  numOutcomes,
  outcomeHashes,
  resolutionTime, 
  finalizationDeadline, 
  creatorFeeBps
) {
  // Calculate buffer size:
  // 1 (instruction) + 32 (question_hash) + 32 (resolution_spec_hash) + 1 (num_outcomes)
  // + 4 (Vec length) + numOutcomes*32 (outcome_hashes) + 8 (resolution_time) + 8 (finalization_deadline) + 2 (creator_fee_bps)
  const bufferSize = 1 + 32 + 32 + 1 + 4 + (numOutcomes * 32) + 8 + 8 + 2;
  const buffer = Buffer.alloc(bufferSize);
  let offset = 0;
  
  // Instruction index = 100 (CreateMultiOutcomeMarket)
  buffer.writeUInt8(INSTRUCTION_INDEX, offset);
  offset += 1;
  
  // Question hash (32 bytes)
  questionHash.copy(buffer, offset);
  offset += 32;
  
  // Resolution spec hash (32 bytes)
  resolutionSpecHash.copy(buffer, offset);
  offset += 32;
  
  // Num outcomes (u8)
  buffer.writeUInt8(numOutcomes, offset);
  offset += 1;
  
  // Outcome hashes Vec length (u32 LE)
  buffer.writeUInt32LE(numOutcomes, offset);
  offset += 4;
  
  // Outcome hashes (32 bytes each)
  for (let i = 0; i < numOutcomes; i++) {
    outcomeHashes[i].copy(buffer, offset);
    offset += 32;
  }
  
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
 * Generate outcome labels
 */
function generateOutcomeLabels(numOutcomes) {
  if (numOutcomes === 3) {
    return ['Candidate A', 'Candidate B', 'Candidate C'];
  } else if (numOutcomes === 4) {
    return ['Option 1', 'Option 2', 'Option 3', 'Option 4'];
  } else {
    return Array.from({ length: numOutcomes }, (_, i) => `Outcome ${i + 1}`);
  }
}

// Config offset for next_market_id
const NEXT_MARKET_ID_OFFSET = 8 + 32 + 32 + 32 + 32 + 32; // = 168

async function main() {
  const numOutcomes = parseInt(process.argv[2]) || 3;
  const question = process.argv[3] || `Multi-outcome test market with ${numOutcomes} options`;
  
  if (numOutcomes < 2 || numOutcomes > 32) {
    console.error('Error: num_outcomes must be between 2 and 32');
    process.exit(1);
  }
  
  console.log('='.repeat(60));
  console.log('1024 Prediction Market - Create Multi-Outcome Market');
  console.log('='.repeat(60));
  console.log(`Num Outcomes: ${numOutcomes}`);
  console.log(`Question: ${question}`);
  console.log('');
  
  const connection = new Connection('http://127.0.0.1:8899', 'confirmed');
  console.log('Connected to local RPC');
  
  // Load admin keypair
  const faucetPath = process.env.HOME + '/1024chain-testnet/keys/faucet.json';
  const faucetData = JSON.parse(fs.readFileSync(faucetPath, 'utf-8'));
  const admin = Keypair.fromSecretKey(new Uint8Array(faucetData));
  console.log(`Admin/Creator: ${admin.publicKey.toBase58()}`);
  
  // Derive Config PDA
  const [configPda] = PublicKey.findProgramAddressSync([PM_CONFIG_SEED], PROGRAM_ID);
  console.log(`Config PDA: ${configPda.toBase58()}`);
  
  // Get next market_id from config
  const configInfo = await connection.getAccountInfo(configPda);
  if (!configInfo) {
    console.error('Error: Config account not found. Run init_program.js first.');
    process.exit(1);
  }
  
  const nextMarketId = configInfo.data.readBigUInt64LE(NEXT_MARKET_ID_OFFSET);
  const marketId = Number(nextMarketId);
  console.log(`Next Market ID: ${marketId}`);
  
  // Derive Market PDA
  const marketIdBuffer = Buffer.alloc(8);
  marketIdBuffer.writeBigUInt64LE(BigInt(marketId));
  
  const [marketPda] = PublicKey.findProgramAddressSync(
    [MARKET_SEED, marketIdBuffer],
    PROGRAM_ID
  );
  console.log(`Market PDA: ${marketPda.toBase58()}`);
  
  // Derive Market Vault PDA
  const [vaultPda] = PublicKey.findProgramAddressSync(
    [MARKET_VAULT_SEED, marketIdBuffer],
    PROGRAM_ID
  );
  console.log(`Vault PDA: ${vaultPda.toBase58()}`);
  
  // Derive Outcome Mint PDAs
  const outcomeMints = [];
  const outcomeLabels = generateOutcomeLabels(numOutcomes);
  
  console.log('\nOutcome Mints:');
  for (let i = 0; i < numOutcomes; i++) {
    const [outcomeMintPda] = PublicKey.findProgramAddressSync(
      [OUTCOME_MINT_SEED, marketIdBuffer, Buffer.from([i])],
      PROGRAM_ID
    );
    outcomeMints.push(outcomeMintPda);
    console.log(`  [${i}] ${outcomeLabels[i]}: ${outcomeMintPda.toBase58()}`);
  }
  
  // Create hashes
  const questionHash = crypto.createHash('sha256').update(question).digest();
  const resolutionSpec = `Market resolves based on official results. ${numOutcomes} possible outcomes.`;
  const resolutionSpecHash = crypto.createHash('sha256').update(resolutionSpec).digest();
  
  // Create outcome hashes
  const outcomeHashes = outcomeLabels.map(label => 
    crypto.createHash('sha256').update(label).digest()
  );
  
  // Set resolution time (7 days from now) and deadline (10 days from now)
  const now = Math.floor(Date.now() / 1000);
  const resolutionTime = now + 7 * 24 * 60 * 60;
  const finalizationDeadline = now + 10 * 24 * 60 * 60;
  const creatorFeeBps = 100; // 1%
  
  // Serialize instruction data
  const instructionData = serializeCreateMultiOutcomeMarketArgs(
    questionHash,
    resolutionSpecHash,
    numOutcomes,
    outcomeHashes,
    resolutionTime,
    finalizationDeadline,
    creatorFeeBps
  );
  
  console.log(`\nInstruction data size: ${instructionData.length} bytes`);
  
  // Build accounts list
  const accounts = [
    { pubkey: admin.publicKey, isSigner: true, isWritable: true },    // 0. Creator
    { pubkey: configPda, isSigner: false, isWritable: true },          // 1. Config
    { pubkey: marketPda, isSigner: false, isWritable: true },          // 2. Market
    { pubkey: vaultPda, isSigner: false, isWritable: true },           // 3. Vault
    { pubkey: USDC_MINT, isSigner: false, isWritable: false },         // 4. USDC Mint
    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },  // 5. Token Program
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }, // 6. System Program
    { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false }, // 7. Rent
  ];
  
  // Add outcome mints
  for (const outcomeMint of outcomeMints) {
    accounts.push({ pubkey: outcomeMint, isSigner: false, isWritable: true });
  }
  
  const instruction = new TransactionInstruction({
    keys: accounts,
    programId: PROGRAM_ID,
    data: instructionData,
  });
  
  console.log('\nSending transaction...');
  
  try {
    const tx = new Transaction().add(instruction);
    const signature = await sendAndConfirmTransaction(connection, tx, [admin], {
      commitment: 'confirmed',
    });
    
    console.log('\n' + '='.repeat(60));
    console.log('✅ Multi-Outcome Market Created Successfully!');
    console.log('='.repeat(60));
    console.log(`Transaction: ${signature}`);
    console.log(`Market ID: ${marketId}`);
    console.log(`Market PDA: ${marketPda.toBase58()}`);
    console.log(`Num Outcomes: ${numOutcomes}`);
    console.log(`Resolution Time: ${new Date(resolutionTime * 1000).toISOString()}`);
    console.log(`Creator Fee: ${creatorFeeBps / 100}%`);
    console.log('');
    console.log('Outcome Mints:');
    for (let i = 0; i < numOutcomes; i++) {
      console.log(`  [${i}] ${outcomeLabels[i]}: ${outcomeMints[i].toBase58()}`);
    }
    
  } catch (error) {
    console.error('\n❌ Transaction failed:', error.message);
    if (error.logs) {
      console.error('\nProgram logs:');
      error.logs.forEach(log => console.error('  ', log));
    }
    process.exit(1);
  }
}

main().catch(console.error);
