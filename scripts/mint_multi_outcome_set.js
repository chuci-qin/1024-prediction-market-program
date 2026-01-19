/**
 * Mint Complete Set for Multi-Outcome Market
 * Usage: node mint_multi_outcome_set.js <market_id> <amount> [num_outcomes]
 * Example: node mint_multi_outcome_set.js 1 100000000 3
 * (Mints 100 USDC worth of complete sets = 100 tokens of each outcome)
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
const { 
  getAssociatedTokenAddress, 
  createAssociatedTokenAccountInstruction,
  TOKEN_PROGRAM_ID,
} = require('@solana/spl-token');
const fs = require('fs');

// Program IDs
const PROGRAM_ID = config.PROGRAM_ID;
const USDC_MINT = new PublicKey('7pCrfxhcAEyTFDhrhKRtRS2iMvEYx2dtNE7NzwuU7SA9');

// Seeds
const PM_CONFIG_SEED = Buffer.from('pm_config');
const MARKET_SEED = Buffer.from('market');
const MARKET_VAULT_SEED = Buffer.from('market_vault');
const POSITION_SEED = Buffer.from('position');
const OUTCOME_MINT_SEED = Buffer.from('outcome_mint');

// Instruction index for MintMultiOutcomeCompleteSet = 27 (28th variant in enum, 0-indexed)
const INSTRUCTION_INDEX = 27;

/**
 * Serialize MintMultiOutcomeCompleteSetArgs:
 * - market_id: u64
 * - amount: u64
 */
function serializeMintArgs(marketId, amount) {
  const buffer = Buffer.alloc(1 + 8 + 8); // 17 bytes
  let offset = 0;
  
  buffer.writeUInt8(INSTRUCTION_INDEX, offset);
  offset += 1;
  
  buffer.writeBigUInt64LE(BigInt(marketId), offset);
  offset += 8;
  
  buffer.writeBigUInt64LE(BigInt(amount), offset);
  
  return buffer;
}

async function main() {
  const marketId = parseInt(process.argv[2]);
  const amount = parseInt(process.argv[3]);
  const numOutcomes = parseInt(process.argv[4]) || 3;
  
  if (!marketId || !amount) {
    console.error('Usage: node mint_multi_outcome_set.js <market_id> <amount> [num_outcomes]');
    console.error('Example: node mint_multi_outcome_set.js 1 100000000 3');
    process.exit(1);
  }
  
  console.log('='.repeat(60));
  console.log('1024 Prediction Market - Mint Multi-Outcome Complete Set');
  console.log('='.repeat(60));
  console.log(`Market ID: ${marketId}`);
  console.log(`Amount: ${amount / 1000000} USDC (${amount} e6)`);
  console.log(`Num Outcomes: ${numOutcomes}`);
  console.log('');
  
  const connection = new Connection(config.RPC_URL, 'confirmed');
  console.log('Connected to local RPC');
  
  // Load user keypair
  const faucetPath = '/Users/patrick/Developer/1024ex/faucet.json';
  const faucetData = JSON.parse(fs.readFileSync(faucetPath, 'utf-8'));
  const user = Keypair.fromSecretKey(new Uint8Array(faucetData));
  console.log(`User: ${user.publicKey.toBase58()}`);
  
  // Derive PDAs
  const marketIdBuffer = Buffer.alloc(8);
  marketIdBuffer.writeBigUInt64LE(BigInt(marketId));
  
  const [configPda] = PublicKey.findProgramAddressSync([PM_CONFIG_SEED], PROGRAM_ID);
  const [marketPda] = PublicKey.findProgramAddressSync([MARKET_SEED, marketIdBuffer], PROGRAM_ID);
  const [vaultPda] = PublicKey.findProgramAddressSync([MARKET_VAULT_SEED, marketIdBuffer], PROGRAM_ID);
  const [positionPda] = PublicKey.findProgramAddressSync(
    [POSITION_SEED, marketIdBuffer, user.publicKey.toBuffer()],
    PROGRAM_ID
  );
  
  console.log(`Config PDA: ${configPda.toBase58()}`);
  console.log(`Market PDA: ${marketPda.toBase58()}`);
  console.log(`Vault PDA: ${vaultPda.toBase58()}`);
  console.log(`Position PDA: ${positionPda.toBase58()}`);
  
  // Get user USDC account
  const userUsdcAccount = await getAssociatedTokenAddress(USDC_MINT, user.publicKey);
  console.log(`User USDC: ${userUsdcAccount.toBase58()}`);
  
  // Check user USDC balance
  try {
    const usdcBalance = await connection.getTokenAccountBalance(userUsdcAccount);
    console.log(`User USDC Balance: ${usdcBalance.value.uiAmount} USDC`);
  } catch (e) {
    console.error('Error: User USDC account not found. Run setup_usdc.js first.');
    process.exit(1);
  }
  
  // Derive outcome mints and get/create user token accounts
  const outcomeMints = [];
  const userOutcomeAccounts = [];
  const setupInstructions = [];
  
  console.log('\nOutcome Token Accounts:');
  for (let i = 0; i < numOutcomes; i++) {
    const [outcomeMintPda] = PublicKey.findProgramAddressSync(
      [OUTCOME_MINT_SEED, marketIdBuffer, Buffer.from([i])],
      PROGRAM_ID
    );
    outcomeMints.push(outcomeMintPda);
    
    const userOutcomeAccount = await getAssociatedTokenAddress(outcomeMintPda, user.publicKey);
    userOutcomeAccounts.push(userOutcomeAccount);
    
    console.log(`  [${i}] Mint: ${outcomeMintPda.toBase58()}`);
    console.log(`      User ATA: ${userOutcomeAccount.toBase58()}`);
    
    // Check if account exists, if not create it
    const accountInfo = await connection.getAccountInfo(userOutcomeAccount);
    if (!accountInfo) {
      console.log(`      Creating ATA for outcome ${i}...`);
      setupInstructions.push(
        createAssociatedTokenAccountInstruction(
          user.publicKey,
          userOutcomeAccount,
          user.publicKey,
          outcomeMintPda
        )
      );
    }
  }
  
  // Serialize instruction data
  const instructionData = serializeMintArgs(marketId, amount);
  
  // Build accounts list
  const accounts = [
    { pubkey: user.publicKey, isSigner: true, isWritable: true },      // 0. User
    { pubkey: configPda, isSigner: false, isWritable: false },         // 1. Config
    { pubkey: marketPda, isSigner: false, isWritable: true },          // 2. Market
    { pubkey: vaultPda, isSigner: false, isWritable: true },           // 3. Vault
    { pubkey: userUsdcAccount, isSigner: false, isWritable: true },    // 4. User USDC
    { pubkey: positionPda, isSigner: false, isWritable: true },        // 5. Position
    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },  // 6. Token Program
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }, // 7. System Program
  ];
  
  // Add outcome mints and user token accounts (pairs)
  for (let i = 0; i < numOutcomes; i++) {
    accounts.push({ pubkey: outcomeMints[i], isSigner: false, isWritable: true });
    accounts.push({ pubkey: userOutcomeAccounts[i], isSigner: false, isWritable: true });
  }
  
  const mintInstruction = new TransactionInstruction({
    keys: accounts,
    programId: PROGRAM_ID,
    data: instructionData,
  });
  
  console.log('\nSending transaction...');
  
  try {
    const tx = new Transaction();
    
    // Add setup instructions if any
    for (const ix of setupInstructions) {
      tx.add(ix);
    }
    
    // Add main instruction
    tx.add(mintInstruction);
    
    const signature = await sendAndConfirmTransaction(connection, tx, [user], {
      commitment: 'confirmed',
    });
    
    console.log('\n' + '='.repeat(60));
    console.log('✅ Multi-Outcome Complete Set Minted Successfully!');
    console.log('='.repeat(60));
    console.log(`Transaction: ${signature}`);
    console.log(`Market ID: ${marketId}`);
    console.log(`USDC Spent: ${amount / 1000000} USDC`);
    console.log(`Tokens Minted: ${amount / 1000000} of each outcome`);
    console.log('');
    
    // Check balances
    console.log('New Token Balances:');
    for (let i = 0; i < numOutcomes; i++) {
      try {
        const balance = await connection.getTokenAccountBalance(userOutcomeAccounts[i]);
        console.log(`  Outcome ${i}: ${balance.value.uiAmount} tokens`);
      } catch (e) {
        console.log(`  Outcome ${i}: Error reading balance`);
      }
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
