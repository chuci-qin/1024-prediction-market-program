/**
 * RedeemCompleteSet - Burn YES + NO tokens to get USDC back
 * Run on server: node redeem_complete_set.js [market_id] [amount]
 * 
 * Example: node redeem_complete_set.js 1 50000000  # Redeem 50 tokens
 */

const { 
  Connection, 
  Keypair, 
  PublicKey, 
  Transaction, 
  TransactionInstruction,
  sendAndConfirmTransaction,
} = require('@solana/web3.js');
const { TOKEN_PROGRAM_ID, getAssociatedTokenAddress } = require('@solana/spl-token');
const fs = require('fs');

const PROGRAM_ID = new PublicKey('FnwmQjmUkRTLA1G3i1CmFVE5cySzQGYZRezGAErdLizu');
const USDC_MINT = new PublicKey('7pCrfxhcAEyTFDhrhKRtRS2iMvEYx2dtNE7NzwuU7SA9');

// Seeds
const PM_CONFIG_SEED = Buffer.from('pm_config');
const MARKET_SEED = Buffer.from('market');
const MARKET_VAULT_SEED = Buffer.from('market_vault');
const POSITION_SEED = Buffer.from('position');

// Instruction index (Initialize=0, CreateMarket=1, ..., MintCompleteSet=7, RedeemCompleteSet=8)
const REDEEM_COMPLETE_SET_IX = 8;

/**
 * Serialize RedeemCompleteSetArgs
 * Layout:
 * - u8 instruction (8)
 * - u64 market_id
 * - u64 amount
 */
function serializeRedeemCompleteSetArgs(marketId, amount) {
  const buffer = Buffer.alloc(1 + 8 + 8);
  let offset = 0;
  
  buffer.writeUInt8(REDEEM_COMPLETE_SET_IX, offset); offset += 1;
  buffer.writeBigUInt64LE(BigInt(marketId), offset); offset += 8;
  buffer.writeBigUInt64LE(BigInt(amount), offset);
  
  return buffer;
}

async function main() {
  const marketId = process.argv[2] ? parseInt(process.argv[2]) : 1;
  const amount = process.argv[3] ? parseInt(process.argv[3]) : 10_000_000; // 10 tokens
  
  console.log('='.repeat(60));
  console.log('1024 Prediction Market - Redeem Complete Set');
  console.log('='.repeat(60));
  console.log(`Market ID: ${marketId}`);
  console.log(`Amount: ${amount / 1_000_000} tokens`);
  
  const connection = new Connection('http://127.0.0.1:8899', 'confirmed');
  
  const faucetPath = process.env.HOME + '/1024chain-testnet/keys/faucet.json';
  const faucetData = JSON.parse(fs.readFileSync(faucetPath, 'utf-8'));
  const user = Keypair.fromSecretKey(new Uint8Array(faucetData));
  console.log(`\nUser: ${user.publicKey.toBase58()}`);
  
  // Derive PDAs
  const [configPda] = PublicKey.findProgramAddressSync([PM_CONFIG_SEED], PROGRAM_ID);
  
  const marketIdBytes = Buffer.alloc(8);
  marketIdBytes.writeBigUInt64LE(BigInt(marketId));
  const [marketPda] = PublicKey.findProgramAddressSync([MARKET_SEED, marketIdBytes], PROGRAM_ID);
  const [marketVault] = PublicKey.findProgramAddressSync([MARKET_VAULT_SEED, marketIdBytes], PROGRAM_ID);
  
  // Get market info
  const marketAccount = await connection.getAccountInfo(marketPda);
  if (!marketAccount) {
    console.error('❌ Market not found!');
    return;
  }
  
  const marketData = marketAccount.data;
  const yesMint = new PublicKey(marketData.slice(112, 144));
  const noMint = new PublicKey(marketData.slice(144, 176));
  
  console.log(`\nMarket Info:`);
  console.log(`  YES Mint: ${yesMint.toBase58()}`);
  console.log(`  NO Mint: ${noMint.toBase58()}`);
  console.log(`  Market Vault: ${marketVault.toBase58()}`);
  
  // Derive token accounts
  const userUsdcAta = await getAssociatedTokenAddress(USDC_MINT, user.publicKey);
  const userYesAta = await getAssociatedTokenAddress(yesMint, user.publicKey);
  const userNoAta = await getAssociatedTokenAddress(noMint, user.publicKey);
  
  console.log(`  User USDC: ${userUsdcAta.toBase58()}`);
  console.log(`  User YES: ${userYesAta.toBase58()}`);
  console.log(`  User NO: ${userNoAta.toBase58()}`);
  
  // Derive Position PDA
  const [positionPda] = PublicKey.findProgramAddressSync(
    [POSITION_SEED, marketIdBytes, user.publicKey.toBuffer()],
    PROGRAM_ID
  );
  console.log(`  Position PDA: ${positionPda.toBase58()}`);
  
  // Check balances before
  try {
    const yesBalance = await connection.getTokenAccountBalance(userYesAta);
    const noBalance = await connection.getTokenAccountBalance(userNoAta);
    console.log(`\nBefore Redeem:`);
    console.log(`  YES Balance: ${yesBalance.value.uiAmount}`);
    console.log(`  NO Balance: ${noBalance.value.uiAmount}`);
  } catch (e) {
    console.log('\nCould not fetch token balances');
  }
  
  // Serialize instruction
  const instructionData = serializeRedeemCompleteSetArgs(marketId, amount);
  console.log(`\nInstruction data: ${instructionData.toString('hex')}`);
  
  /**
   * Accounts for RedeemCompleteSet (from processor.rs):
   * 0. [signer] User
   * 1. [] Config
   * 2. [writable] Market
   * 3. [writable] Market Vault
   * 4. [writable] User's USDC Account
   * 5. [writable] YES Token Mint
   * 6. [writable] NO Token Mint
   * 7. [writable] User's YES Token Account
   * 8. [writable] User's NO Token Account
   * 9. [writable] Position PDA
   * 10. [] Token Program
   */
  const redeemIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: user.publicKey, isSigner: true, isWritable: true },
      { pubkey: configPda, isSigner: false, isWritable: false },
      { pubkey: marketPda, isSigner: false, isWritable: true },
      { pubkey: marketVault, isSigner: false, isWritable: true },
      { pubkey: userUsdcAta, isSigner: false, isWritable: true },
      { pubkey: yesMint, isSigner: false, isWritable: true },
      { pubkey: noMint, isSigner: false, isWritable: true },
      { pubkey: userYesAta, isSigner: false, isWritable: true },
      { pubkey: userNoAta, isSigner: false, isWritable: true },
      { pubkey: positionPda, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    data: instructionData,
  });
  
  const tx = new Transaction().add(redeemIx);
  
  console.log('\nSending RedeemCompleteSet transaction...');
  
  try {
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.lastValidBlockHeight = lastValidBlockHeight;
    tx.feePayer = user.publicKey;
    
    const signature = await sendAndConfirmTransaction(connection, tx, [user], {
      commitment: 'confirmed',
    });
    
    console.log('\n✅ RedeemCompleteSet successful!');
    console.log(`Signature: ${signature}`);
    console.log(`Explorer: https://testnet-scan.1024chain.com/tx/${signature}`);
    
    // Check balances after
    try {
      const yesBalance = await connection.getTokenAccountBalance(userYesAta);
      const noBalance = await connection.getTokenAccountBalance(userNoAta);
      console.log(`\nAfter Redeem:`);
      console.log(`  YES Balance: ${yesBalance.value.uiAmount}`);
      console.log(`  NO Balance: ${noBalance.value.uiAmount}`);
    } catch (e) {
      console.log('\nCould not fetch token balances');
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
