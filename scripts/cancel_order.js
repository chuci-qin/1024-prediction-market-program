/**
 * Cancel Order - Cancel an existing order
 * Run on server: node cancel_order.js [market_id] [order_id]
 * 
 * Note: For Sell orders with escrow, tokens will be returned to the user!
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

// Seeds
const PM_CONFIG_SEED = Buffer.from('pm_config');
const MARKET_SEED = Buffer.from('market');
const ORDER_SEED = Buffer.from('order');
const ORDER_ESCROW_SEED = Buffer.from('order_escrow');

// Order Side enum
const OrderSide = {
  Buy: 0,
  Sell: 1,
};

// Outcome enum
const Outcome = {
  Yes: 0,
  No: 1,
};

// Instruction index (Initialize=0, CreateMarket=1, ... CancelOrder=10)
const CANCEL_ORDER_IX = 10;

/**
 * Serialize CancelOrderArgs
 * Layout:
 * - u8 instruction (10)
 * - u64 market_id
 * - u64 order_id
 */
function serializeCancelOrderArgs(marketId, orderId) {
  const buffer = Buffer.alloc(1 + 8 + 8);
  let offset = 0;
  
  buffer.writeUInt8(CANCEL_ORDER_IX, offset); offset += 1;
  buffer.writeBigUInt64LE(BigInt(marketId), offset); offset += 8;
  buffer.writeBigUInt64LE(BigInt(orderId), offset);
  
  return buffer;
}

async function main() {
  const marketId = process.argv[2] ? parseInt(process.argv[2]) : 1;
  const orderId = process.argv[3] ? parseInt(process.argv[3]) : 2;
  
  console.log('='.repeat(60));
  console.log('1024 Prediction Market - Cancel Order');
  console.log('='.repeat(60));
  console.log(`Market ID: ${marketId}`);
  console.log(`Order ID: ${orderId}`);
  
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
  
  const orderIdBytes = Buffer.alloc(8);
  orderIdBytes.writeBigUInt64LE(BigInt(orderId));
  const [orderPda] = PublicKey.findProgramAddressSync(
    [ORDER_SEED, marketIdBytes, orderIdBytes],
    PROGRAM_ID
  );
  
  console.log(`  Order PDA: ${orderPda.toBase58()}`);
  
  // Get market info for token mints
  const marketAccount = await connection.getAccountInfo(marketPda);
  if (!marketAccount) {
    console.error('❌ Market not found!');
    return;
  }
  
  const marketData = marketAccount.data;
  const yesMint = new PublicKey(marketData.slice(112, 144));
  const noMint = new PublicKey(marketData.slice(144, 176));
  
  // Verify order exists and belongs to user
  const orderAccount = await connection.getAccountInfo(orderPda);
  if (!orderAccount) {
    console.error('❌ Order not found!');
    return;
  }
  
  const orderData = orderAccount.data;
  const orderOwner = new PublicKey(orderData.slice(24, 56));
  console.log(`  Order Owner: ${orderOwner.toBase58()}`);
  
  if (!orderOwner.equals(user.publicKey)) {
    console.error('❌ You are not the owner of this order!');
    return;
  }
  
  // Parse order side and outcome
  // Order struct: discriminator(8) + order_id(8) + market_id(8) + owner(32) + side(1) + outcome(1)
  const orderSide = orderData[56]; // offset 56 = 8+8+8+32
  const orderOutcome = orderData[57]; // offset 57
  
  const isSellOrder = orderSide === OrderSide.Sell;
  console.log(`  Order Side: ${isSellOrder ? 'Sell' : 'Buy'}`);
  console.log(`  Order Outcome: ${orderOutcome === Outcome.Yes ? 'YES' : 'NO'}`);
  
  // Check if order has escrow (parse escrow_token_account Option<Pubkey>)
  // Order struct: ... bump(1) + escrow_token_account(1+32 if Some)
  // bump is at offset: 56 + 1 + 1 + 8 + 8 + 8 + 1 + 1 + 1 + 8 + 8 + 8 + 8 = 116 (approximate)
  // Let's check the escrow by deriving the PDA and seeing if it exists
  
  // Derive escrow PDA
  const [escrowPda] = PublicKey.findProgramAddressSync(
    [ORDER_ESCROW_SEED, marketIdBytes, orderIdBytes],
    PROGRAM_ID
  );
  
  // Check if escrow exists
  const escrowAccount = await connection.getAccountInfo(escrowPda);
  const hasEscrow = escrowAccount !== null;
  
  console.log(`  Has Escrow: ${hasEscrow}`);
  if (hasEscrow) {
    console.log(`  Escrow PDA: ${escrowPda.toBase58()}`);
  }
  
  // Serialize instruction
  const instructionData = serializeCancelOrderArgs(marketId, orderId);
  console.log(`\nInstruction data: ${instructionData.toString('hex')}`);
  
  /**
   * Accounts for CancelOrder (from processor.rs):
   * 0. [signer] User
   * 1. [] Market
   * 2. [writable] Order PDA
   * 
   * Additional for Sell orders with escrow:
   * 3. [writable] User's Token Account
   * 4. [writable] Escrow Token Account
   * 5. [] Token Program
   */
  
  const accounts = [
    { pubkey: user.publicKey, isSigner: true, isWritable: true },
    { pubkey: marketPda, isSigner: false, isWritable: false },
    { pubkey: orderPda, isSigner: false, isWritable: true },
  ];
  
  // Add escrow accounts if this is a sell order with escrow
  if (hasEscrow) {
    const tokenMint = orderOutcome === Outcome.Yes ? yesMint : noMint;
    const userTokenAccount = await getAssociatedTokenAddress(tokenMint, user.publicKey);
    
    console.log(`\nEscrow Return Setup:`);
    console.log(`  Token Mint: ${tokenMint.toBase58()}`);
    console.log(`  User Token Account: ${userTokenAccount.toBase58()}`);
    console.log(`  Tokens will be returned to user!`);
    
    accounts.push(
      { pubkey: userTokenAccount, isSigner: false, isWritable: true },
      { pubkey: escrowPda, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    );
  }
  
  const cancelOrderIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: accounts,
    data: instructionData,
  });
  
  const tx = new Transaction().add(cancelOrderIx);
  
  console.log('\nSending CancelOrder transaction...');
  
  try {
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.lastValidBlockHeight = lastValidBlockHeight;
    tx.feePayer = user.publicKey;
    
    const signature = await sendAndConfirmTransaction(connection, tx, [user], {
      commitment: 'confirmed',
    });
    
    console.log('\n✅ CancelOrder successful!');
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
