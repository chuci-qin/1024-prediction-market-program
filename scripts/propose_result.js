/**
 * ProposeResult - Oracle proposes a result for a market
 * Run on server: node propose_result.js [market_id] [result]
 * 
 * result: 0 = Yes, 1 = No
 */

const { 
  Connection, 
  Keypair, 
  PublicKey, 
  Transaction, 
  TransactionInstruction,
  sendAndConfirmTransaction,
  SystemProgram,
} = require('@solana/web3.js');
const fs = require('fs');

const PROGRAM_ID = new PublicKey('FnwmQjmUkRTLA1G3i1CmFVE5cySzQGYZRezGAErdLizu');

// Seeds
const PM_CONFIG_SEED = Buffer.from('pm_config');
const MARKET_SEED = Buffer.from('market');
const PROPOSAL_SEED = Buffer.from('oracle_proposal');

// Instruction index (count from enum)
// Initialize=0, CreateMarket=1, Activate=2, Pause=3, Resume=4, Cancel=5, Flag=6
// MintComplete=7, RedeemComplete=8, PlaceOrder=9, CancelOrder=10
// MatchMint=11, MatchBurn=12, ExecuteTrade=13, ProposeResult=14
const PROPOSE_RESULT_IX = 14;

/**
 * Serialize ProposeResultArgs
 * Layout:
 * - u8 instruction (14)
 * - u64 market_id
 * - u8 result (0=Yes, 1=No)
 */
function serializeProposeResultArgs(marketId, result) {
  const buffer = Buffer.alloc(1 + 8 + 1);
  let offset = 0;
  
  buffer.writeUInt8(PROPOSE_RESULT_IX, offset); offset += 1;
  buffer.writeBigUInt64LE(BigInt(marketId), offset); offset += 8;
  buffer.writeUInt8(result, offset);
  
  return buffer;
}

async function main() {
  const marketId = process.argv[2] ? parseInt(process.argv[2]) : 1;
  const result = process.argv[3] ? parseInt(process.argv[3]) : 0; // 0 = Yes
  
  const resultNames = ['Yes', 'No', 'Invalid'];
  
  console.log('='.repeat(60));
  console.log('1024 Prediction Market - Propose Result');
  console.log('='.repeat(60));
  console.log(`Market ID: ${marketId}`);
  console.log(`Proposed Result: ${resultNames[result]} (${result})`);
  
  const connection = new Connection('http://127.0.0.1:8899', 'confirmed');
  
  // Use faucet as oracle admin (since they initialized the program)
  const faucetPath = process.env.HOME + '/1024chain-testnet/keys/faucet.json';
  const faucetData = JSON.parse(fs.readFileSync(faucetPath, 'utf-8'));
  const oracleAdmin = Keypair.fromSecretKey(new Uint8Array(faucetData));
  console.log(`\nOracle Admin: ${oracleAdmin.publicKey.toBase58()}`);
  
  // Derive PDAs
  const [configPda] = PublicKey.findProgramAddressSync([PM_CONFIG_SEED], PROGRAM_ID);
  
  const marketIdBytes = Buffer.alloc(8);
  marketIdBytes.writeBigUInt64LE(BigInt(marketId));
  const [marketPda] = PublicKey.findProgramAddressSync([MARKET_SEED, marketIdBytes], PROGRAM_ID);
  
  // Get market info to check resolution time
  const marketAccount = await connection.getAccountInfo(marketPda);
  if (!marketAccount) {
    console.error('❌ Market not found!');
    return;
  }
  
  const marketData = marketAccount.data;
  const resolutionTime = Number(marketData.readBigInt64LE(210));
  
  // Use blockchain time instead of client time
  const slot = await connection.getSlot();
  const blockTime = await connection.getBlockTime(slot);
  
  console.log(`\nMarket Info:`);
  console.log(`  Resolution Time: ${new Date(resolutionTime * 1000).toISOString()}`);
  console.log(`  Blockchain Time: ${new Date(blockTime * 1000).toISOString()}`);
  console.log(`  Market PDA: ${marketPda.toBase58()}`);
  
  if (blockTime < resolutionTime) {
    console.log(`\n⚠️  Warning: Resolution time has not passed yet!`);
    console.log(`   Wait ${Math.round((resolutionTime - blockTime) / 3600)} hours.`);
  } else {
    console.log(`\n✅ Resolution time has passed! Can propose result.`);
  }
  
  // Derive Proposal PDA
  const [proposalPda] = PublicKey.findProgramAddressSync(
    [PROPOSAL_SEED, marketIdBytes],
    PROGRAM_ID
  );
  console.log(`  Proposal PDA: ${proposalPda.toBase58()}`);
  
  // Serialize instruction
  const instructionData = serializeProposeResultArgs(marketId, result);
  console.log(`\nInstruction data: ${instructionData.toString('hex')}`);
  
  /**
   * Accounts for ProposeResult (from processor.rs):
   * 0. [signer] Oracle Admin
   * 1. [] Config
   * 2. [writable] Market
   * 3. [writable] Proposal PDA
   * 4. [] System Program
   */
  const proposeResultIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: oracleAdmin.publicKey, isSigner: true, isWritable: true },
      { pubkey: configPda, isSigner: false, isWritable: false },
      { pubkey: marketPda, isSigner: false, isWritable: true },
      { pubkey: proposalPda, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: instructionData,
  });
  
  const tx = new Transaction().add(proposeResultIx);
  
  console.log('\nSending ProposeResult transaction...');
  
  try {
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.lastValidBlockHeight = lastValidBlockHeight;
    tx.feePayer = oracleAdmin.publicKey;
    
    const signature = await sendAndConfirmTransaction(connection, tx, [oracleAdmin], {
      commitment: 'confirmed',
    });
    
    console.log('\n✅ ProposeResult successful!');
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
