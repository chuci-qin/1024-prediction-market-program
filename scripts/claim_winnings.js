/**
 * ClaimWinnings - Claim USDC from winning tokens after market resolution
 * Run on server: node claim_winnings.js [market_id]
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
const { TOKEN_PROGRAM_ID, getAssociatedTokenAddress } = require('@solana/spl-token');
const fs = require('fs');

const PROGRAM_ID = config.PROGRAM_ID;
const USDC_MINT = new PublicKey('7pCrfxhcAEyTFDhrhKRtRS2iMvEYx2dtNE7NzwuU7SA9');

// Seeds
const MARKET_SEED = Buffer.from('market');
const MARKET_VAULT_SEED = Buffer.from('market_vault');
const POSITION_SEED = Buffer.from('position');

// Instruction index (Initialize=0, ..., ClaimWinnings=18)
const CLAIM_WINNINGS_IX = 18;

async function main() {
  const marketId = process.argv[2] ? parseInt(process.argv[2]) : 1;
  
  console.log('='.repeat(60));
  console.log('1024 Prediction Market - Claim Winnings');
  console.log('='.repeat(60));
  console.log(`Market ID: ${marketId}`);
  
  const connection = new Connection(config.RPC_URL, 'confirmed');
  
  const faucetPath = '/Users/patrick/Developer/1024ex/faucet.json';
  const faucetData = JSON.parse(fs.readFileSync(faucetPath, 'utf-8'));
  const user = Keypair.fromSecretKey(new Uint8Array(faucetData));
  console.log(`\nUser: ${user.publicKey.toBase58()}`);
  
  // Derive PDAs
  const marketIdBytes = Buffer.alloc(8);
  marketIdBytes.writeBigUInt64LE(BigInt(marketId));
  const [marketPda] = PublicKey.findProgramAddressSync([MARKET_SEED, marketIdBytes], PROGRAM_ID);
  const [marketVault] = PublicKey.findProgramAddressSync([MARKET_VAULT_SEED, marketIdBytes], PROGRAM_ID);
  const [positionPda] = PublicKey.findProgramAddressSync(
    [POSITION_SEED, marketIdBytes, user.publicKey.toBuffer()],
    PROGRAM_ID
  );
  
  console.log(`  Market PDA: ${marketPda.toBase58()}`);
  console.log(`  Market Vault: ${marketVault.toBase58()}`);
  console.log(`  Position PDA: ${positionPda.toBase58()}`);
  
  // Get market info
  const marketAccount = await connection.getAccountInfo(marketPda);
  if (!marketAccount) {
    console.error('❌ Market not found!');
    return;
  }
  
  const marketData = marketAccount.data;
  const yesMint = new PublicKey(marketData.slice(112, 144));
  const noMint = new PublicKey(marketData.slice(144, 176));
  const status = marketData[208];
  const finalResultTag = marketData[226];
  
  // MarketStatus enum from state.rs: Pending=0, Active=1, Paused=2, Resolved=3, Cancelled=4
  const statusNames = ['Pending', 'Active', 'Paused', 'Resolved', 'Cancelled'];
  const resultNames = ['None', 'Yes', 'No', 'Invalid'];
  
  console.log(`\nMarket Info:`);
  console.log(`  YES Mint: ${yesMint.toBase58()}`);
  console.log(`  NO Mint: ${noMint.toBase58()}`);
  console.log(`  Status: ${statusNames[status] || status}`);
  console.log(`  Final Result: ${resultNames[finalResultTag] || 'Unknown'}`);
  
  if (status !== 3) { // 3 = Resolved
    console.log('\n⚠️  Warning: Market is not resolved yet!');
  } else {
    console.log('\n✅ Market is resolved! Can claim winnings.');
  }
  
  // Derive token accounts
  const userUsdcAta = await getAssociatedTokenAddress(USDC_MINT, user.publicKey);
  const userYesAta = await getAssociatedTokenAddress(yesMint, user.publicKey);
  const userNoAta = await getAssociatedTokenAddress(noMint, user.publicKey);
  
  console.log(`  User USDC: ${userUsdcAta.toBase58()}`);
  console.log(`  User YES: ${userYesAta.toBase58()}`);
  console.log(`  User NO: ${userNoAta.toBase58()}`);
  
  // Check token balances
  try {
    const yesBalance = await connection.getTokenAccountBalance(userYesAta);
    const noBalance = await connection.getTokenAccountBalance(userNoAta);
    console.log(`\nToken Balances:`);
    console.log(`  YES: ${yesBalance.value.uiAmount}`);
    console.log(`  NO: ${noBalance.value.uiAmount}`);
  } catch (e) {
    console.log('\nCould not fetch token balances');
  }
  
  // Serialize instruction (just instruction index, no args)
  const instructionData = Buffer.from([CLAIM_WINNINGS_IX]);
  console.log(`\nInstruction data: ${instructionData.toString('hex')}`);
  
  /**
   * Accounts for ClaimWinnings (from processor.rs):
   * 0. [signer] User
   * 1. [] Market
   * 2. [writable] Position
   * 3. [writable] Market Vault
   * 4. [writable] User's USDC Account
   * 5. [writable] User's YES Token Account
   * 6. [writable] User's NO Token Account
   * 7. [writable] YES Mint
   * 8. [writable] NO Mint
   * 9. [] Token Program
   */
  const claimIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: user.publicKey, isSigner: true, isWritable: true },
      { pubkey: marketPda, isSigner: false, isWritable: false },
      { pubkey: positionPda, isSigner: false, isWritable: true },
      { pubkey: marketVault, isSigner: false, isWritable: true },
      { pubkey: userUsdcAta, isSigner: false, isWritable: true },
      { pubkey: userYesAta, isSigner: false, isWritable: true },
      { pubkey: userNoAta, isSigner: false, isWritable: true },
      { pubkey: yesMint, isSigner: false, isWritable: true },
      { pubkey: noMint, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    data: instructionData,
  });
  
  const tx = new Transaction().add(claimIx);
  
  console.log('\nSending ClaimWinnings transaction...');
  
  try {
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.lastValidBlockHeight = lastValidBlockHeight;
    tx.feePayer = user.publicKey;
    
    const signature = await sendAndConfirmTransaction(connection, tx, [user], {
      commitment: 'confirmed',
    });
    
    console.log('\n✅ ClaimWinnings successful!');
    console.log(`Signature: ${signature}`);
    console.log(`Explorer: https://testnet-scan.1024chain.com/tx/${signature}`);
    
    // Check USDC balance after
    try {
      const usdcBalance = await connection.getTokenAccountBalance(userUsdcAta);
      console.log(`\nUSDC Balance After: ${usdcBalance.value.uiAmount}`);
    } catch (e) {
      console.log('\nCould not fetch USDC balance');
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
