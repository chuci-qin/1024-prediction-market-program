/**
 * MatchBurn - Match Sell YES + Sell NO orders to burn tokens and release USDC
 * Run on server: node match_burn.js [market_id] [yes_order_id] [no_order_id] [amount] [yes_price] [no_price]
 * 
 * Note: Now uses ORDER ESCROW accounts instead of seller token accounts!
 * Tokens are escrowed when sell orders are placed.
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
const PM_CONFIG_SEED = Buffer.from('pm_config');
const MARKET_SEED = Buffer.from('market');
const MARKET_VAULT_SEED = Buffer.from('market_vault');
const ORDER_SEED = Buffer.from('order');
const ORDER_ESCROW_SEED = Buffer.from('order_escrow');

// Instruction index (Initialize=0, ..., MatchBurn=12)
const MATCH_BURN_IX = 12;

const PRICE_PRECISION = 1_000_000;

/**
 * Serialize MatchBurnArgs
 * Layout:
 * - u8 instruction (12)
 * - u64 market_id
 * - u64 yes_order_id
 * - u64 no_order_id
 * - u64 amount
 * - u64 yes_price (e6)
 * - u64 no_price (e6)
 */
function serializeMatchBurnArgs(marketId, yesOrderId, noOrderId, amount, yesPrice, noPrice) {
  const buffer = Buffer.alloc(1 + 8 + 8 + 8 + 8 + 8 + 8);
  let offset = 0;
  
  buffer.writeUInt8(MATCH_BURN_IX, offset); offset += 1;
  buffer.writeBigUInt64LE(BigInt(marketId), offset); offset += 8;
  buffer.writeBigUInt64LE(BigInt(yesOrderId), offset); offset += 8;
  buffer.writeBigUInt64LE(BigInt(noOrderId), offset); offset += 8;
  buffer.writeBigUInt64LE(BigInt(amount), offset); offset += 8;
  buffer.writeBigUInt64LE(BigInt(yesPrice), offset); offset += 8;
  buffer.writeBigUInt64LE(BigInt(noPrice), offset);
  
  return buffer;
}

async function main() {
  const marketId = process.argv[2] ? parseInt(process.argv[2]) : 1;
  const yesOrderId = process.argv[3] ? parseInt(process.argv[3]) : 5;
  const noOrderId = process.argv[4] ? parseInt(process.argv[4]) : 6;
  const amount = process.argv[5] ? parseInt(process.argv[5]) : 10_000_000;
  const yesPriceArg = process.argv[6] ? parseFloat(process.argv[6]) : 0.60;
  const noPriceArg = process.argv[7] ? parseFloat(process.argv[7]) : 0.40;
  
  const yesPrice = Math.floor(yesPriceArg * PRICE_PRECISION);
  const noPrice = Math.floor(noPriceArg * PRICE_PRECISION);
  
  console.log('='.repeat(60));
  console.log('1024 Prediction Market - MatchBurn');
  console.log('='.repeat(60));
  console.log(`Market ID: ${marketId}`);
  console.log(`YES Order ID: ${yesOrderId} (Sell), Price: $${yesPriceArg}`);
  console.log(`NO Order ID: ${noOrderId} (Sell), Price: $${noPriceArg}`);
  console.log(`Amount: ${amount / 1_000_000} tokens`);
  console.log(`Total Price: $${yesPriceArg + noPriceArg}`);
  
  const connection = new Connection(config.RPC_URL, 'confirmed');
  
  const faucetPath = process.env.ADMIN_KEYPAIR || '../faucet.json';
  const faucetData = JSON.parse(fs.readFileSync(faucetPath, 'utf-8'));
  const caller = Keypair.fromSecretKey(new Uint8Array(faucetData));
  console.log(`\nCaller: ${caller.publicKey.toBase58()}`);
  
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
  
  // Derive order PDAs
  const yesOrderIdBytes = Buffer.alloc(8);
  yesOrderIdBytes.writeBigUInt64LE(BigInt(yesOrderId));
  const [yesOrderPda] = PublicKey.findProgramAddressSync(
    [ORDER_SEED, marketIdBytes, yesOrderIdBytes],
    PROGRAM_ID
  );
  
  const noOrderIdBytes = Buffer.alloc(8);
  noOrderIdBytes.writeBigUInt64LE(BigInt(noOrderId));
  const [noOrderPda] = PublicKey.findProgramAddressSync(
    [ORDER_SEED, marketIdBytes, noOrderIdBytes],
    PROGRAM_ID
  );
  
  console.log(`  YES Order PDA: ${yesOrderPda.toBase58()}`);
  console.log(`  NO Order PDA: ${noOrderPda.toBase58()}`);
  
  // Get order owners
  const yesOrderAccount = await connection.getAccountInfo(yesOrderPda);
  const noOrderAccount = await connection.getAccountInfo(noOrderPda);
  
  if (!yesOrderAccount || !noOrderAccount) {
    console.error('❌ One or both orders not found!');
    return;
  }
  
  const yesSellerWallet = new PublicKey(yesOrderAccount.data.slice(24, 56));
  const noSellerWallet = new PublicKey(noOrderAccount.data.slice(24, 56));
  
  console.log(`  YES Seller: ${yesSellerWallet.toBase58()}`);
  console.log(`  NO Seller: ${noSellerWallet.toBase58()}`);
  
  // Derive escrow PDAs for sell orders
  const [yesEscrowPda] = PublicKey.findProgramAddressSync(
    [ORDER_ESCROW_SEED, marketIdBytes, yesOrderIdBytes],
    PROGRAM_ID
  );
  const [noEscrowPda] = PublicKey.findProgramAddressSync(
    [ORDER_ESCROW_SEED, marketIdBytes, noOrderIdBytes],
    PROGRAM_ID
  );
  
  console.log(`  YES Escrow: ${yesEscrowPda.toBase58()}`);
  console.log(`  NO Escrow: ${noEscrowPda.toBase58()}`);
  
  // Verify escrow accounts exist
  const yesEscrowAccount = await connection.getAccountInfo(yesEscrowPda);
  const noEscrowAccount = await connection.getAccountInfo(noEscrowPda);
  
  if (!yesEscrowAccount || !noEscrowAccount) {
    console.error('❌ One or both escrow accounts not found!');
    console.error('   Make sure sell orders were placed with the new escrow system.');
    return;
  }
  
  console.log(`  YES Escrow Balance: ${yesEscrowAccount.lamports / 1e9} N1024`);
  console.log(`  NO Escrow Balance: ${noEscrowAccount.lamports / 1e9} N1024`);
  
  // Get USDC accounts for payouts
  const yesSellerUsdcAta = await getAssociatedTokenAddress(USDC_MINT, yesSellerWallet);
  const noSellerUsdcAta = await getAssociatedTokenAddress(USDC_MINT, noSellerWallet);
  
  // Serialize instruction
  const instructionData = serializeMatchBurnArgs(marketId, yesOrderId, noOrderId, amount, yesPrice, noPrice);
  console.log(`\nInstruction data: ${instructionData.toString('hex')}`);
  
  /**
   * Accounts for MatchBurn (from processor.rs - UPDATED):
   * 0. [signer] Relayer
   * 1. [writable] Config
   * 2. [writable] Market
   * 3. [writable] YES Order
   * 4. [writable] NO Order
   * 5. [writable] YES Mint
   * 6. [writable] NO Mint
   * 7. [writable] YES Order ESCROW Token Account (not user's ATA)
   * 8. [writable] NO Order ESCROW Token Account (not user's ATA)
   * 9. [writable] Market Vault
   * 10. [writable] YES Seller's USDC Account
   * 11. [writable] NO Seller's USDC Account
   * 12. [] Token Program
   */
  const matchBurnIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: caller.publicKey, isSigner: true, isWritable: true },
      { pubkey: configPda, isSigner: false, isWritable: true },
      { pubkey: marketPda, isSigner: false, isWritable: true },
      { pubkey: yesOrderPda, isSigner: false, isWritable: true },
      { pubkey: noOrderPda, isSigner: false, isWritable: true },
      { pubkey: yesMint, isSigner: false, isWritable: true },
      { pubkey: noMint, isSigner: false, isWritable: true },
      { pubkey: yesEscrowPda, isSigner: false, isWritable: true },  // Now escrow, not user ATA
      { pubkey: noEscrowPda, isSigner: false, isWritable: true },   // Now escrow, not user ATA
      { pubkey: marketVault, isSigner: false, isWritable: true },
      { pubkey: yesSellerUsdcAta, isSigner: false, isWritable: true },
      { pubkey: noSellerUsdcAta, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    data: instructionData,
  });
  
  const tx = new Transaction().add(matchBurnIx);
  
  console.log('\nSending MatchBurn transaction...');
  
  try {
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.lastValidBlockHeight = lastValidBlockHeight;
    tx.feePayer = caller.publicKey;
    
    const signature = await sendAndConfirmTransaction(connection, tx, [caller], {
      commitment: 'confirmed',
    });
    
    console.log('\n✅ MatchBurn successful!');
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
