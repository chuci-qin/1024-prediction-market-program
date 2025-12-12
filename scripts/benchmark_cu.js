/**
 * Compute Unit (CU) Benchmark Script
 * 
 * Measures the CU consumption for various prediction market instructions,
 * especially for multi-outcome matching operations.
 * 
 * Usage: node benchmark_cu.js
 * 
 * Note: This script simulates transactions to measure CU without executing them.
 */

const { 
  Connection, 
  Keypair, 
  PublicKey, 
  Transaction, 
  TransactionInstruction,
  ComputeBudgetProgram,
} = require('@solana/web3.js');
const { TOKEN_PROGRAM_ID } = require('@solana/spl-token');
const fs = require('fs');

// 1024Chain Testnet 配置
const RPC_URL = 'https://testnet-rpc.1024chain.com/rpc/';
const PROGRAM_ID = new PublicKey('FVtPQkdYvSNdpTA6QXYRcTBhDGgnufw2Enqmo2tQKr58');

// Seeds
const PM_CONFIG_SEED = Buffer.from('pm_config');
const MARKET_SEED = Buffer.from('market');
const ORDER_SEED = Buffer.from('order');
const OUTCOME_MINT_SEED = Buffer.from('outcome_mint');

// Instruction indices
const INSTRUCTIONS = {
  MatchMint: 11,
  MatchBurn: 12,
  ExecuteTrade: 13,
  MatchMintMulti: 42,
  MatchBurnMulti: 43,
};

const PRICE_PRECISION = 1_000_000;

/**
 * Create a mock MatchMintMulti instruction for CU simulation
 */
function createMockMatchMintMultiInstruction(numOutcomes, caller, configPda, marketPda, marketVault) {
  const orderDataSize = numOutcomes * (1 + 8 + 8);
  const buffer = Buffer.alloc(1 + 8 + 1 + 8 + 4 + orderDataSize);
  let offset = 0;
  
  buffer.writeUInt8(INSTRUCTIONS.MatchMintMulti, offset); offset += 1;
  buffer.writeBigUInt64LE(BigInt(1), offset); offset += 8; // market_id
  buffer.writeUInt8(numOutcomes, offset); offset += 1;
  buffer.writeBigUInt64LE(BigInt(1_000_000), offset); offset += 8; // amount
  buffer.writeUInt32LE(numOutcomes, offset); offset += 4;
  
  const pricePerOutcome = Math.floor(PRICE_PRECISION / numOutcomes);
  for (let i = 0; i < numOutcomes; i++) {
    buffer.writeUInt8(i, offset); offset += 1;
    buffer.writeBigUInt64LE(BigInt(i + 1), offset); offset += 8;
    buffer.writeBigUInt64LE(BigInt(pricePerOutcome), offset); offset += 8;
  }
  
  // Build mock account list
  const keys = [
    { pubkey: caller.publicKey, isSigner: true, isWritable: true },
    { pubkey: configPda, isSigner: false, isWritable: false },
    { pubkey: marketPda, isSigner: false, isWritable: true },
    { pubkey: marketVault, isSigner: false, isWritable: true },
    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    { pubkey: new PublicKey('11111111111111111111111111111111'), isSigner: false, isWritable: false },
  ];
  
  // Add mock accounts for each outcome (Order, Mint, TokenAccount)
  for (let i = 0; i < numOutcomes; i++) {
    keys.push({ pubkey: Keypair.generate().publicKey, isSigner: false, isWritable: true });
    keys.push({ pubkey: Keypair.generate().publicKey, isSigner: false, isWritable: true });
    keys.push({ pubkey: Keypair.generate().publicKey, isSigner: false, isWritable: true });
  }
  
  return new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: keys,
    data: buffer,
  });
}

/**
 * Simulate transaction and extract CU usage
 */
async function simulateAndGetCU(connection, instruction, caller) {
  const transaction = new Transaction();
  
  // Add compute budget instruction to request max CU
  transaction.add(
    ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 })
  );
  transaction.add(instruction);
  
  const { blockhash } = await connection.getLatestBlockhash('confirmed');
  transaction.recentBlockhash = blockhash;
  transaction.feePayer = caller.publicKey;
  
  try {
    const simulation = await connection.simulateTransaction(transaction, [caller]);
    
    if (simulation.value.err) {
      // Even failed simulations report CU consumed
      const cuMatch = simulation.value.logs?.find(log => log.includes('consumed'));
      if (cuMatch) {
        const match = cuMatch.match(/consumed (\d+) of/);
        if (match) {
          return { 
            cu: parseInt(match[1]), 
            error: simulation.value.err,
            logs: simulation.value.logs 
          };
        }
      }
      return { cu: null, error: simulation.value.err, logs: simulation.value.logs };
    }
    
    return { 
      cu: simulation.value.unitsConsumed || 0, 
      error: null,
      logs: simulation.value.logs 
    };
  } catch (error) {
    return { cu: null, error: error.message, logs: [] };
  }
}

async function main() {
  console.log('='.repeat(70));
  console.log('  1024 Prediction Market - Compute Unit Benchmark');
  console.log('='.repeat(70));
  console.log(`\nProgram ID: ${PROGRAM_ID.toBase58()}`);
  console.log(`RPC: ${RPC_URL}\n`);
  
  const connection = new Connection(RPC_URL, 'confirmed');
  
  // Load caller keypair
  const keypairPath = '/Users/chuciqin/Desktop/project1024/1024codebase/1024-chain/keys/faucet.json';
  if (!fs.existsSync(keypairPath)) {
    console.error('❌ Keypair file not found!');
    return;
  }
  
  const callerData = JSON.parse(fs.readFileSync(keypairPath, 'utf-8'));
  const caller = Keypair.fromSecretKey(new Uint8Array(callerData));
  console.log(`Caller: ${caller.publicKey.toBase58()}\n`);
  
  // Derive PDAs
  const [configPda] = PublicKey.findProgramAddressSync([PM_CONFIG_SEED], PROGRAM_ID);
  
  const marketIdBytes = Buffer.alloc(8);
  marketIdBytes.writeBigUInt64LE(BigInt(1));
  const [marketPda] = PublicKey.findProgramAddressSync([MARKET_SEED, marketIdBytes], PROGRAM_ID);
  
  // Use a mock vault for simulation
  const marketVault = Keypair.generate().publicKey;
  
  console.log('Testing MatchMintMulti CU consumption for different outcome counts:\n');
  console.log('┌──────────────┬─────────────┬──────────────┬─────────────────────┐');
  console.log('│ Outcomes     │ Accounts    │ CU Consumed  │ Recommended Budget  │');
  console.log('├──────────────┼─────────────┼──────────────┼─────────────────────┤');
  
  const results = [];
  const outcomeCounts = [2, 3, 4, 5, 6, 8, 10, 12, 16];
  
  for (const numOutcomes of outcomeCounts) {
    const accountCount = 6 + (numOutcomes * 3); // Base + 3 per outcome
    
    if (accountCount > 64) {
      console.log(`│ ${numOutcomes.toString().padStart(12)} │ ${accountCount.toString().padStart(11)} │ EXCEEDS 64  │ N/A                 │`);
      continue;
    }
    
    const instruction = createMockMatchMintMultiInstruction(
      numOutcomes, caller, configPda, marketPda, marketVault
    );
    
    const result = await simulateAndGetCU(connection, instruction, caller);
    
    let cuStr, recommendedStr;
    if (result.cu !== null) {
      // Add 50% buffer for recommended budget
      const recommended = Math.ceil(result.cu * 1.5);
      cuStr = result.cu.toLocaleString().padStart(12);
      recommendedStr = recommended.toLocaleString().padStart(19);
      results.push({ numOutcomes, accountCount, cu: result.cu, recommended });
    } else {
      cuStr = 'SIM FAILED'.padStart(12);
      recommendedStr = 'N/A'.padStart(19);
    }
    
    console.log(`│ ${numOutcomes.toString().padStart(12)} │ ${accountCount.toString().padStart(11)} │ ${cuStr} │ ${recommendedStr} │`);
  }
  
  console.log('└──────────────┴─────────────┴──────────────┴─────────────────────┘');
  
  // Summary
  console.log('\n' + '='.repeat(70));
  console.log('  Summary & Recommendations');
  console.log('='.repeat(70));
  
  if (results.length > 0) {
    console.log('\n📊 CU Consumption Analysis:\n');
    
    // Group by ranges
    const lowRange = results.filter(r => r.numOutcomes <= 4);
    const midRange = results.filter(r => r.numOutcomes > 4 && r.numOutcomes <= 8);
    const highRange = results.filter(r => r.numOutcomes > 8);
    
    if (lowRange.length > 0) {
      const avgLow = Math.round(lowRange.reduce((s, r) => s + r.cu, 0) / lowRange.length);
      console.log(`  2-4 outcomes:  ~${avgLow.toLocaleString()} CU → Request: 150,000 CU`);
    }
    
    if (midRange.length > 0) {
      const avgMid = Math.round(midRange.reduce((s, r) => s + r.cu, 0) / midRange.length);
      console.log(`  5-8 outcomes:  ~${avgMid.toLocaleString()} CU → Request: 250,000 CU`);
    }
    
    if (highRange.length > 0) {
      const avgHigh = Math.round(highRange.reduce((s, r) => s + r.cu, 0) / highRange.length);
      console.log(`  9-16 outcomes: ~${avgHigh.toLocaleString()} CU → Request: 450,000 CU`);
    }
    
    console.log('\n📝 Notes:');
    console.log('  - Max accounts per transaction: 64');
    console.log('  - Max outcomes for MatchMintMulti: 16 (6 + 16*3 = 54 accounts)');
    console.log('  - Always add ComputeBudgetProgram.setComputeUnitLimit() for multi-outcome ops');
    console.log('  - CU estimates are based on simulation; actual may vary slightly');
  }
  
  console.log('\n' + '='.repeat(70));
}

main().catch(console.error);
