/**
 * Mint Complete Set - Deposit USDC, get YES + NO tokens
 * Run on server: node mint_complete_set.js [market_id] [amount]
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
const config = require('./config');
const { 
  TOKEN_PROGRAM_ID,
  getOrCreateAssociatedTokenAccount,
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddress,
} = require('@solana/spl-token');
const fs = require('fs');

const PROGRAM_ID = config.PROGRAM_ID;
const USDC_MINT = new PublicKey('7pCrfxhcAEyTFDhrhKRtRS2iMvEYx2dtNE7NzwuU7SA9');

// Seeds
const PM_CONFIG_SEED = Buffer.from('pm_config');
const MARKET_SEED = Buffer.from('market');
const POSITION_SEED = Buffer.from('position');

// Instruction index for MintCompleteSet = 7
const MINT_COMPLETE_SET_IX = 7;

/**
 * Serialize MintCompleteSetArgs
 * Layout:
 * - u8 instruction (7)
 * - u64 market_id
 * - u64 amount
 */
function serializeMintCompleteSetArgs(marketId, amount) {
  const buffer = Buffer.alloc(1 + 8 + 8);
  buffer.writeUInt8(MINT_COMPLETE_SET_IX, 0);
  buffer.writeBigUInt64LE(BigInt(marketId), 1);
  buffer.writeBigUInt64LE(BigInt(amount), 9);
  return buffer;
}

async function main() {
  const marketId = process.argv[2] ? parseInt(process.argv[2]) : 1;
  const amount = process.argv[3] ? parseInt(process.argv[3]) : 1_000_000; // 1 USDC = 1M units (6 decimals)
  
  console.log('='.repeat(60));
  console.log(`1024 Prediction Market - Mint Complete Set`);
  console.log('='.repeat(60));
  console.log(`Market ID: ${marketId}`);
  console.log(`Amount: ${amount} (${amount / 1_000_000} USDC)`);
  
  const connection = new Connection(config.RPC_URL, 'confirmed');
  
  const faucetPath = '/Users/patrick/Developer/1024ex/faucet.json';
  const faucetData = JSON.parse(fs.readFileSync(faucetPath, 'utf-8'));
  const user = Keypair.fromSecretKey(new Uint8Array(faucetData));
  console.log(`User: ${user.publicKey.toBase58()}`);
  
  // Derive PDAs
  const [configPda] = PublicKey.findProgramAddressSync([PM_CONFIG_SEED], PROGRAM_ID);
  
  const marketIdBytes = Buffer.alloc(8);
  marketIdBytes.writeBigUInt64LE(BigInt(marketId));
  const [marketPda] = PublicKey.findProgramAddressSync([MARKET_SEED, marketIdBytes], PROGRAM_ID);
  
  // Get market account to find YES/NO mints and vault
  const marketAccount = await connection.getAccountInfo(marketPda);
  if (!marketAccount) {
    console.error('❌ Market not found!');
    return;
  }
  
  const marketData = marketAccount.data;
  const yesMint = new PublicKey(marketData.slice(112, 144));
  const noMint = new PublicKey(marketData.slice(144, 176));
  const marketVault = new PublicKey(marketData.slice(176, 208));
  
  console.log(`\nMarket Info:`);
  console.log(`  YES Mint: ${yesMint.toBase58()}`);
  console.log(`  NO Mint: ${noMint.toBase58()}`);
  console.log(`  Vault: ${marketVault.toBase58()}`);
  
  // Derive position PDA
  const [positionPda] = PublicKey.findProgramAddressSync(
    [POSITION_SEED, marketIdBytes, user.publicKey.toBuffer()],
    PROGRAM_ID
  );
  console.log(`  Position PDA: ${positionPda.toBase58()}`);
  
  // Get or create user token accounts
  console.log('\nPreparing token accounts...');
  
  const userUsdcAta = await getAssociatedTokenAddress(USDC_MINT, user.publicKey);
  const userYesAta = await getAssociatedTokenAddress(yesMint, user.publicKey);
  const userNoAta = await getAssociatedTokenAddress(noMint, user.publicKey);
  
  console.log(`  User USDC ATA: ${userUsdcAta.toBase58()}`);
  console.log(`  User YES ATA: ${userYesAta.toBase58()}`);
  console.log(`  User NO ATA: ${userNoAta.toBase58()}`);
  
  // Check USDC balance
  try {
    const usdcBalance = await connection.getTokenAccountBalance(userUsdcAta);
    console.log(`  USDC Balance: ${usdcBalance.value.uiAmount}`);
  } catch (e) {
    console.log(`  USDC Balance: 0 (account not found)`);
  }
  
  // Build transaction
  const tx = new Transaction();
  
  // Check YES/NO token accounts exist, create if not
  const yesAtaInfo = await connection.getAccountInfo(userYesAta);
  const noAtaInfo = await connection.getAccountInfo(userNoAta);
  
  if (!yesAtaInfo) {
    console.log('  Creating YES token account...');
    tx.add(
      createAssociatedTokenAccountInstruction(
        user.publicKey, userYesAta, user.publicKey, yesMint
      )
    );
  }
  
  if (!noAtaInfo) {
    console.log('  Creating NO token account...');
    tx.add(
      createAssociatedTokenAccountInstruction(
        user.publicKey, userNoAta, user.publicKey, noMint
      )
    );
  }
  
  console.log('  ✅ Token accounts ready');
  
  // Check if position account exists
  const positionInfo = await connection.getAccountInfo(positionPda);
  const positionExists = positionInfo !== null;
  console.log(`  Position exists: ${positionExists}`);
  
  // Create MintCompleteSet instruction
  const instructionData = serializeMintCompleteSetArgs(marketId, amount);
  console.log(`\nInstruction data: ${instructionData.toString('hex')}`);
  
  /**
   * Accounts for MintCompleteSet (from processor.rs):
   * 0. [signer] User
   * 1. [] Config
   * 2. [writable] Market
   * 3. [writable] Market Vault
   * 4. [writable] User USDC Account
   * 5. [writable] YES Mint
   * 6. [writable] NO Mint
   * 7. [writable] User YES Token Account
   * 8. [writable] User NO Token Account
   * 9. [writable] Position PDA
   * 10. [] Token Program
   * 11. [] System Program
   */
  const mintIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: user.publicKey, isSigner: true, isWritable: true },
      { pubkey: configPda, isSigner: false, isWritable: true },  // Writable for total_minted_sets
      { pubkey: marketPda, isSigner: false, isWritable: true },
      { pubkey: marketVault, isSigner: false, isWritable: true },   // 3. Market Vault
      { pubkey: userUsdcAta, isSigner: false, isWritable: true },   // 4. User USDC
      { pubkey: yesMint, isSigner: false, isWritable: true },
      { pubkey: noMint, isSigner: false, isWritable: true },
      { pubkey: userYesAta, isSigner: false, isWritable: true },
      { pubkey: userNoAta, isSigner: false, isWritable: true },
      { pubkey: positionPda, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: instructionData,
  });
  
  tx.add(mintIx);
  
  console.log('\nSending MintCompleteSet transaction...');
  
  try {
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.lastValidBlockHeight = lastValidBlockHeight;
    tx.feePayer = user.publicKey;
    
    const signature = await sendAndConfirmTransaction(connection, tx, [user], {
      commitment: 'confirmed',
    });
    
    console.log('\n✅ MintCompleteSet successful!');
    console.log(`Signature: ${signature}`);
    console.log(`Explorer: https://testnet-scan.1024chain.com/tx/${signature}`);
    
    // Check balances after
    console.log('\n=== Balances After ===');
    try {
      const yesBalance = await connection.getTokenAccountBalance(userYesAta);
      console.log(`YES Balance: ${yesBalance.value.uiAmount}`);
    } catch (e) {
      console.log('YES Balance: 0');
    }
    try {
      const noBalance = await connection.getTokenAccountBalance(userNoAta);
      console.log(`NO Balance: ${noBalance.value.uiAmount}`);
    } catch (e) {
      console.log('NO Balance: 0');
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
