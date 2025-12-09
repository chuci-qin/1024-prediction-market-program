/**
 * Claim Winnings from Multi-Outcome Market
 * Usage: node claim_multi_outcome_winnings.js <market_id> [winning_outcome_index]
 * Example: node claim_multi_outcome_winnings.js 2 0
 * (Claims USDC for winning outcome tokens)
 */

const { 
  Connection, 
  Keypair, 
  PublicKey, 
  Transaction, 
  TransactionInstruction,
  sendAndConfirmTransaction,
} = require('@solana/web3.js');
const { 
  getAssociatedTokenAddress, 
  TOKEN_PROGRAM_ID,
} = require('@solana/spl-token');
const fs = require('fs');

// Program IDs
const PROGRAM_ID = new PublicKey('FnwmQjmUkRTLA1G3i1CmFVE5cySzQGYZRezGAErdLizu');
const USDC_MINT = new PublicKey('7pCrfxhcAEyTFDhrhKRtRS2iMvEYx2dtNE7NzwuU7SA9');

// Seeds
const PM_CONFIG_SEED = Buffer.from('pm_config');
const MARKET_SEED = Buffer.from('market');
const MARKET_VAULT_SEED = Buffer.from('market_vault');
const POSITION_SEED = Buffer.from('position');
const OUTCOME_MINT_SEED = Buffer.from('outcome_mint');

// Instruction index for ClaimMultiOutcomeWinnings = 31
const INSTRUCTION_INDEX = 31;

/**
 * Serialize ClaimMultiOutcomeWinningsArgs:
 * - market_id: u64
 */
function serializeClaimArgs(marketId) {
  const buffer = Buffer.alloc(1 + 8); // 9 bytes
  let offset = 0;
  
  buffer.writeUInt8(INSTRUCTION_INDEX, offset);
  offset += 1;
  
  buffer.writeBigUInt64LE(BigInt(marketId), offset);
  
  return buffer;
}

async function main() {
  const marketId = parseInt(process.argv[2]);
  let winningOutcomeIndex = process.argv[3] !== undefined ? parseInt(process.argv[3]) : null;
  
  if (!marketId) {
    console.error('Usage: node claim_multi_outcome_winnings.js <market_id> [winning_outcome_index]');
    console.error('Example: node claim_multi_outcome_winnings.js 2 0');
    process.exit(1);
  }
  
  console.log('='.repeat(60));
  console.log('1024 Prediction Market - Claim Multi-Outcome Winnings');
  console.log('='.repeat(60));
  console.log(`Market ID: ${marketId}`);
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
  
  // Read market to get winning outcome if not provided
  const marketAccount = await connection.getAccountInfo(marketPda);
  if (!marketAccount) {
    console.error('❌ Market not found!');
    process.exit(1);
  }
  
  const marketData = marketAccount.data;
  // winning_outcome_index is stored in Market struct, need to find offset
  // For now, use provided value or default to 0
  if (winningOutcomeIndex === null) {
    // Try to read from market data
    // The winning_outcome_index is near the end of the Market struct
    // Approximate offset based on Market struct layout
    const winningOutcomeOpt = marketData[340]; // Option discriminant
    if (winningOutcomeOpt === 1) {
      winningOutcomeIndex = marketData[341];
      console.log(`Winning outcome index from market: ${winningOutcomeIndex}`);
    } else {
      console.error('❌ Market has not been resolved yet (no winning outcome)');
      console.error('Run propose_multi_outcome_result.js and finalize_result.js first.');
      process.exit(1);
    }
  }
  
  console.log(`Winning Outcome Index: ${winningOutcomeIndex}`);
  
  // Derive winning outcome mint
  const [winningOutcomeMint] = PublicKey.findProgramAddressSync(
    [OUTCOME_MINT_SEED, marketIdBuffer, Buffer.from([winningOutcomeIndex])],
    PROGRAM_ID
  );
  console.log(`Winning Outcome Mint: ${winningOutcomeMint.toBase58()}`);
  
  // Get user's winning outcome token account
  const userWinningOutcomeAccount = await getAssociatedTokenAddress(winningOutcomeMint, user.publicKey);
  console.log(`User Winning Token Account: ${userWinningOutcomeAccount.toBase58()}`);
  
  // Get user's USDC account
  const userUsdcAccount = await getAssociatedTokenAddress(USDC_MINT, user.publicKey);
  console.log(`User USDC Account: ${userUsdcAccount.toBase58()}`);
  
  // Check winning token balance
  try {
    const balance = await connection.getTokenAccountBalance(userWinningOutcomeAccount);
    console.log(`\nWinning Token Balance: ${balance.value.uiAmount} tokens`);
  } catch (e) {
    console.log('\nCould not read winning token balance');
  }
  
  // Check USDC balance before
  try {
    const usdcBalance = await connection.getTokenAccountBalance(userUsdcAccount);
    console.log(`USDC Balance Before: ${usdcBalance.value.uiAmount} USDC`);
  } catch (e) {
    console.log('Could not read USDC balance');
  }
  
  // Serialize instruction data
  const instructionData = serializeClaimArgs(marketId);
  console.log(`\nInstruction Data: ${instructionData.toString('hex')}`);
  
  // Build accounts list
  // From instruction.rs:
  // 0. `[signer]` User
  // 1. `[]` PredictionMarketConfig
  // 2. `[]` Market
  // 3. `[writable]` User Position PDA
  // 4. `[writable]` User's Winning Outcome Token Account
  // 5. `[writable]` Winning Outcome Token Mint
  // 6. `[writable]` Market Vault
  // 7. `[writable]` User's USDC Account
  // 8. `[]` Token Program
  const accounts = [
    { pubkey: user.publicKey, isSigner: true, isWritable: true },         // 0. User
    { pubkey: configPda, isSigner: false, isWritable: false },            // 1. Config
    { pubkey: marketPda, isSigner: false, isWritable: false },            // 2. Market
    { pubkey: positionPda, isSigner: false, isWritable: true },           // 3. Position
    { pubkey: userWinningOutcomeAccount, isSigner: false, isWritable: true }, // 4. User Token
    { pubkey: winningOutcomeMint, isSigner: false, isWritable: true },    // 5. Mint
    { pubkey: vaultPda, isSigner: false, isWritable: true },              // 6. Vault
    { pubkey: userUsdcAccount, isSigner: false, isWritable: true },       // 7. User USDC
    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },     // 8. Token Program
  ];
  
  const claimInstruction = new TransactionInstruction({
    keys: accounts,
    programId: PROGRAM_ID,
    data: instructionData,
  });
  
  console.log('\nSending transaction...');
  
  try {
    const tx = new Transaction().add(claimInstruction);
    
    const signature = await sendAndConfirmTransaction(connection, tx, [user], {
      commitment: 'confirmed',
    });
    
    console.log('\n' + '='.repeat(60));
    console.log('✅ Multi-Outcome Winnings Claimed Successfully!');
    console.log('='.repeat(60));
    console.log(`Transaction: ${signature}`);
    console.log(`Market ID: ${marketId}`);
    console.log(`Winning Outcome: ${winningOutcomeIndex}`);
    console.log('');
    
    // Check winning token balance after
    try {
      const balance = await connection.getTokenAccountBalance(userWinningOutcomeAccount);
      console.log(`Winning Token Balance After: ${balance.value.uiAmount} tokens`);
    } catch (e) {
      console.log('Winning tokens have been burned');
    }
    
    // Check USDC balance after
    try {
      const usdcBalance = await connection.getTokenAccountBalance(userUsdcAccount);
      console.log(`USDC Balance After: ${usdcBalance.value.uiAmount} USDC`);
    } catch (e) {
      console.log('Could not read USDC balance');
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
