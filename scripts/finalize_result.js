/**
 * FinalizeResult - Finalize market result after challenge period
 * Run on server: node finalize_result.js [market_id]
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
const ORACLE_PROPOSAL_SEED = Buffer.from('oracle_proposal');

// Instruction index (Initialize=0, ..., ProposeResult=14, ChallengeResult=15, FinalizeResult=16)
const FINALIZE_RESULT_IX = 16;

async function main() {
  const marketId = process.argv[2] ? parseInt(process.argv[2]) : 1;
  
  console.log('='.repeat(60));
  console.log('1024 Prediction Market - Finalize Result');
  console.log('='.repeat(60));
  console.log(`Market ID: ${marketId}`);
  
  const connection = new Connection('http://127.0.0.1:8899', 'confirmed');
  
  const faucetPath = process.env.HOME + '/1024chain-testnet/keys/faucet.json';
  const faucetData = JSON.parse(fs.readFileSync(faucetPath, 'utf-8'));
  const caller = Keypair.fromSecretKey(new Uint8Array(faucetData));
  console.log(`\nCaller: ${caller.publicKey.toBase58()}`);
  
  // Derive PDAs
  const [configPda] = PublicKey.findProgramAddressSync([PM_CONFIG_SEED], PROGRAM_ID);
  
  const marketIdBytes = Buffer.alloc(8);
  marketIdBytes.writeBigUInt64LE(BigInt(marketId));
  const [marketPda] = PublicKey.findProgramAddressSync([MARKET_SEED, marketIdBytes], PROGRAM_ID);
  const [proposalPda] = PublicKey.findProgramAddressSync([ORACLE_PROPOSAL_SEED, marketIdBytes], PROGRAM_ID);
  
  console.log(`  Config PDA: ${configPda.toBase58()}`);
  console.log(`  Market PDA: ${marketPda.toBase58()}`);
  console.log(`  Proposal PDA: ${proposalPda.toBase58()}`);
  
  // Serialize instruction (just instruction index, no args)
  const instructionData = Buffer.from([FINALIZE_RESULT_IX]);
  console.log(`\nInstruction data: ${instructionData.toString('hex')}`);
  
  /**
   * Accounts for FinalizeResult (from processor.rs):
   * 0. [signer] Caller
   * 1. [writable] Config
   * 2. [writable] Market
   * 3. [writable] OracleProposal
   */
  const finalizeIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: caller.publicKey, isSigner: true, isWritable: true },
      { pubkey: configPda, isSigner: false, isWritable: true },
      { pubkey: marketPda, isSigner: false, isWritable: true },
      { pubkey: proposalPda, isSigner: false, isWritable: true },
    ],
    data: instructionData,
  });
  
  const tx = new Transaction().add(finalizeIx);
  
  console.log('\nSending FinalizeResult transaction...');
  
  try {
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.lastValidBlockHeight = lastValidBlockHeight;
    tx.feePayer = caller.publicKey;
    
    const signature = await sendAndConfirmTransaction(connection, tx, [caller], {
      commitment: 'confirmed',
    });
    
    console.log('\n✅ FinalizeResult successful!');
    console.log(`Signature: ${signature}`);
    console.log(`Explorer: https://testnet-scan.1024chain.com/tx/${signature}`);
    
    // Query market status after
    const marketAccount = await connection.getAccountInfo(marketPda);
    if (marketAccount) {
      const status = marketAccount.data[208];
      const finalResult = marketAccount.data[226];
      const statusNames = ['Pending', 'Active', 'Paused', 'Resolved', 'Cancelled'];
      const resultNames = ['Yes', 'No', 'Invalid'];
      console.log(`\nMarket Status: ${statusNames[status] || status}`);
      if (finalResult > 0) {
        console.log(`Final Result: ${resultNames[finalResult - 1] || finalResult}`);
      }
    }
    
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
