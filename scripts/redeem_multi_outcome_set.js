/**
 * Redeem Complete Set for Multi-Outcome Market
 * Usage: node redeem_multi_outcome_set.js <market_id> <amount> [num_outcomes]
 * Example: node redeem_multi_outcome_set.js 2 10000000 3
 * (Burns 10 tokens of each outcome, returns 10 USDC)
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
const { 
  getAssociatedTokenAddress, 
  TOKEN_PROGRAM_ID,
} = require('@solana/spl-token');
const fs = require('fs');

// Program IDs
const PROGRAM_ID = new PublicKey('FVtPQkdYvSNdpTA6QXYRcTBhDGgnufw2Enqmo2tQKr58');
const USDC_MINT = new PublicKey('7pCrfxhcAEyTFDhrhKRtRS2iMvEYx2dtNE7NzwuU7SA9');

// Seeds
const PM_CONFIG_SEED = Buffer.from('pm_config');
const MARKET_SEED = Buffer.from('market');
const MARKET_VAULT_SEED = Buffer.from('market_vault');
const POSITION_SEED = Buffer.from('position');
const OUTCOME_MINT_SEED = Buffer.from('outcome_mint');

// Instruction index for RedeemMultiOutcomeCompleteSet = 28
const INSTRUCTION_INDEX = 28;

/**
 * Serialize RedeemMultiOutcomeCompleteSetArgs:
 * - market_id: u64
 * - amount: u64
 */
function serializeRedeemArgs(marketId, amount) {
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
    console.error('Usage: node redeem_multi_outcome_set.js <market_id> <amount> [num_outcomes]');
    console.error('Example: node redeem_multi_outcome_set.js 2 10000000 3');
    process.exit(1);
  }
  
  console.log('='.repeat(60));
  console.log('1024 Prediction Market - Redeem Multi-Outcome Complete Set');
  console.log('='.repeat(60));
  console.log(`Market ID: ${marketId}`);
  console.log(`Amount: ${amount / 1000000} tokens (${amount} e6)`);
  console.log(`Num Outcomes: ${numOutcomes}`);
  console.log('');
  
  const connection = new Connection('http://127.0.0.1:8899', 'confirmed');
  console.log('Connected to local RPC');
  
  // Load user keypair
  const faucetPath = process.env.HOME + '/1024chain-testnet/keys/faucet.json';
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
  
  // Check USDC balance before
  try {
    const usdcBalance = await connection.getTokenAccountBalance(userUsdcAccount);
    console.log(`User USDC Balance Before: ${usdcBalance.value.uiAmount} USDC`);
  } catch (e) {
    console.log('Could not read USDC balance');
  }
  
  // Derive outcome mints and get user token accounts
  const outcomeMints = [];
  const userOutcomeAccounts = [];
  
  console.log('\nOutcome Token Balances Before:');
  for (let i = 0; i < numOutcomes; i++) {
    const [outcomeMintPda] = PublicKey.findProgramAddressSync(
      [OUTCOME_MINT_SEED, marketIdBuffer, Buffer.from([i])],
      PROGRAM_ID
    );
    outcomeMints.push(outcomeMintPda);
    
    const userOutcomeAccount = await getAssociatedTokenAddress(outcomeMintPda, user.publicKey);
    userOutcomeAccounts.push(userOutcomeAccount);
    
    // Check balance
    try {
      const balance = await connection.getTokenAccountBalance(userOutcomeAccount);
      console.log(`  [${i}] ${outcomeMintPda.toBase58().slice(0,8)}...: ${balance.value.uiAmount} tokens`);
    } catch (e) {
      console.log(`  [${i}] ${outcomeMintPda.toBase58().slice(0,8)}...: Account not found`);
    }
  }
  
  // Serialize instruction data
  const instructionData = serializeRedeemArgs(marketId, amount);
  
  // Build accounts list
  // From instruction.rs:
  // 0. `[signer]` User
  // 1. `[]` PredictionMarketConfig
  // 2. `[writable]` Market
  // 3. `[writable]` Market Vault
  // 4. `[writable]` User's USDC Account
  // 5. `[writable]` User Position PDA
  // 6. `[]` Token Program
  // 7..7+n. `[writable]` Outcome Token Mints + User Token Accounts (pairs)
  const accounts = [
    { pubkey: user.publicKey, isSigner: true, isWritable: true },      // 0. User
    { pubkey: configPda, isSigner: false, isWritable: false },         // 1. Config
    { pubkey: marketPda, isSigner: false, isWritable: true },          // 2. Market
    { pubkey: vaultPda, isSigner: false, isWritable: true },           // 3. Vault
    { pubkey: userUsdcAccount, isSigner: false, isWritable: true },    // 4. User USDC
    { pubkey: positionPda, isSigner: false, isWritable: true },        // 5. Position
    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },  // 6. Token Program
  ];
  
  // Add outcome mints and user token accounts (pairs)
  for (let i = 0; i < numOutcomes; i++) {
    accounts.push({ pubkey: outcomeMints[i], isSigner: false, isWritable: true });
    accounts.push({ pubkey: userOutcomeAccounts[i], isSigner: false, isWritable: true });
  }
  
  const redeemInstruction = new TransactionInstruction({
    keys: accounts,
    programId: PROGRAM_ID,
    data: instructionData,
  });
  
  console.log('\nSending transaction...');
  
  try {
    const tx = new Transaction().add(redeemInstruction);
    
    const signature = await sendAndConfirmTransaction(connection, tx, [user], {
      commitment: 'confirmed',
    });
    
    console.log('\n' + '='.repeat(60));
    console.log('✅ Multi-Outcome Complete Set Redeemed Successfully!');
    console.log('='.repeat(60));
    console.log(`Transaction: ${signature}`);
    console.log(`Market ID: ${marketId}`);
    console.log(`Tokens Burned: ${amount / 1000000} of each outcome`);
    console.log(`USDC Received: ${amount / 1000000} USDC`);
    console.log('');
    
    // Check balances after
    console.log('Token Balances After:');
    for (let i = 0; i < numOutcomes; i++) {
      try {
        const balance = await connection.getTokenAccountBalance(userOutcomeAccounts[i]);
        console.log(`  Outcome ${i}: ${balance.value.uiAmount} tokens`);
      } catch (e) {
        console.log(`  Outcome ${i}: Error reading balance`);
      }
    }
    
    // Check USDC balance after
    try {
      const usdcBalance = await connection.getTokenAccountBalance(userUsdcAccount);
      console.log(`\nUser USDC Balance After: ${usdcBalance.value.uiAmount} USDC`);
    } catch (e) {
      console.log('\nCould not read USDC balance');
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
