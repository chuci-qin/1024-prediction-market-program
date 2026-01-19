/**
 * Propose Result for Multi-Outcome Market
 * Usage: node propose_multi_outcome_result.js <market_id> <winning_outcome_index>
 * Example: node propose_multi_outcome_result.js 2 0
 * (Proposes outcome 0 as the winner for market 2)
 */

const { 
  Connection, 
  Keypair, 
  PublicKey, 
  Transaction, 
  TransactionInstruction,
  SystemProgram,
  sendAndConfirmTransaction,
} = require('@solana/web3.js');
const config = require('./config');
const fs = require('fs');

// Program IDs
const PROGRAM_ID = config.PROGRAM_ID;

// Seeds
const PM_CONFIG_SEED = Buffer.from('pm_config');
const MARKET_SEED = Buffer.from('market');
const ORACLE_PROPOSAL_SEED = Buffer.from('oracle_proposal');

// Instruction index for ProposeMultiOutcomeResult = 30
const INSTRUCTION_INDEX = 30;

/**
 * Serialize ProposeMultiOutcomeResultArgs:
 * - market_id: u64
 * - winning_outcome_index: u8
 */
function serializeProposeArgs(marketId, winningOutcomeIndex) {
  const buffer = Buffer.alloc(1 + 8 + 1); // 10 bytes
  let offset = 0;
  
  buffer.writeUInt8(INSTRUCTION_INDEX, offset);
  offset += 1;
  
  buffer.writeBigUInt64LE(BigInt(marketId), offset);
  offset += 8;
  
  buffer.writeUInt8(winningOutcomeIndex, offset);
  
  return buffer;
}

async function main() {
  const marketId = parseInt(process.argv[2]);
  const winningOutcomeIndex = parseInt(process.argv[3]);
  
  if (marketId === undefined || winningOutcomeIndex === undefined) {
    console.error('Usage: node propose_multi_outcome_result.js <market_id> <winning_outcome_index>');
    console.error('Example: node propose_multi_outcome_result.js 2 0');
    process.exit(1);
  }
  
  console.log('='.repeat(60));
  console.log('1024 Prediction Market - Propose Multi-Outcome Result');
  console.log('='.repeat(60));
  console.log(`Market ID: ${marketId}`);
  console.log(`Winning Outcome Index: ${winningOutcomeIndex}`);
  console.log('');
  
  const connection = new Connection(config.RPC_URL, 'confirmed');
  console.log('Connected to local RPC');
  
  // Load oracle admin keypair (same as faucet/admin)
  const faucetPath = '/Users/patrick/Developer/1024ex/faucet.json';
  const faucetData = JSON.parse(fs.readFileSync(faucetPath, 'utf-8'));
  const oracleAdmin = Keypair.fromSecretKey(new Uint8Array(faucetData));
  console.log(`Oracle Admin: ${oracleAdmin.publicKey.toBase58()}`);
  
  // Derive PDAs
  const marketIdBuffer = Buffer.alloc(8);
  marketIdBuffer.writeBigUInt64LE(BigInt(marketId));
  
  const [configPda] = PublicKey.findProgramAddressSync([PM_CONFIG_SEED], PROGRAM_ID);
  const [marketPda] = PublicKey.findProgramAddressSync([MARKET_SEED, marketIdBuffer], PROGRAM_ID);
  const [proposalPda] = PublicKey.findProgramAddressSync(
    [ORACLE_PROPOSAL_SEED, marketIdBuffer],
    PROGRAM_ID
  );
  
  console.log(`Config PDA: ${configPda.toBase58()}`);
  console.log(`Market PDA: ${marketPda.toBase58()}`);
  console.log(`Proposal PDA: ${proposalPda.toBase58()}`);
  
  // Check market status
  const marketAccount = await connection.getAccountInfo(marketPda);
  if (!marketAccount) {
    console.error('❌ Market not found!');
    process.exit(1);
  }
  
  const marketData = marketAccount.data;
  const marketStatus = marketData[74]; // status byte offset (approximate)
  console.log(`Market Status Byte: ${marketStatus}`);
  
  // Serialize instruction data
  const instructionData = serializeProposeArgs(marketId, winningOutcomeIndex);
  console.log(`Instruction Data: ${instructionData.toString('hex')}`);
  
  // Build accounts list
  // From instruction.rs:
  // 0. `[signer]` Oracle Admin
  // 1. `[]` PredictionMarketConfig
  // 2. `[writable]` Market
  // 3. `[writable]` Oracle Proposal PDA
  // 4. `[]` System Program
  const accounts = [
    { pubkey: oracleAdmin.publicKey, isSigner: true, isWritable: true },  // 0. Oracle Admin
    { pubkey: configPda, isSigner: false, isWritable: false },            // 1. Config
    { pubkey: marketPda, isSigner: false, isWritable: true },             // 2. Market
    { pubkey: proposalPda, isSigner: false, isWritable: true },           // 3. Proposal
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }, // 4. System Program
  ];
  
  const proposeInstruction = new TransactionInstruction({
    keys: accounts,
    programId: PROGRAM_ID,
    data: instructionData,
  });
  
  console.log('\nSending transaction...');
  
  try {
    const tx = new Transaction().add(proposeInstruction);
    
    const signature = await sendAndConfirmTransaction(connection, tx, [oracleAdmin], {
      commitment: 'confirmed',
    });
    
    console.log('\n' + '='.repeat(60));
    console.log('✅ Multi-Outcome Result Proposed Successfully!');
    console.log('='.repeat(60));
    console.log(`Transaction: ${signature}`);
    console.log(`Market ID: ${marketId}`);
    console.log(`Winning Outcome: ${winningOutcomeIndex}`);
    console.log('');
    console.log('Note: Challenge window is now active.');
    console.log('Run finalize_result.js after challenge window expires.');
    
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
