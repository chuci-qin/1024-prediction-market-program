/**
 * ExecuteTrade - Execute direct trade between buy and sell orders
 * Run on server: node execute_trade.js [market_id] [buy_order_id] [sell_order_id] [amount] [price]
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

const PROGRAM_ID = new PublicKey('FVtPQkdYvSNdpTA6QXYRcTBhDGgnufw2Enqmo2tQKr58');

// Seeds
const PM_CONFIG_SEED = Buffer.from('pm_config');
const MARKET_SEED = Buffer.from('market');
const ORDER_SEED = Buffer.from('order');

// Instruction index (Initialize=0, ..., MatchMint=11, MatchBurn=12, ExecuteTrade=13)
const EXECUTE_TRADE_IX = 13;

const PRICE_PRECISION = 1_000_000;

/**
 * Serialize ExecuteTradeArgs
 * Layout:
 * - u8 instruction (13)
 * - u64 market_id
 * - u64 buy_order_id
 * - u64 sell_order_id
 * - u64 amount
 * - u64 price (e6)
 */
function serializeExecuteTradeArgs(marketId, buyOrderId, sellOrderId, amount, price) {
  const buffer = Buffer.alloc(1 + 8 + 8 + 8 + 8 + 8);
  let offset = 0;
  
  buffer.writeUInt8(EXECUTE_TRADE_IX, offset); offset += 1;
  buffer.writeBigUInt64LE(BigInt(marketId), offset); offset += 8;
  buffer.writeBigUInt64LE(BigInt(buyOrderId), offset); offset += 8;
  buffer.writeBigUInt64LE(BigInt(sellOrderId), offset); offset += 8;
  buffer.writeBigUInt64LE(BigInt(amount), offset); offset += 8;
  buffer.writeBigUInt64LE(BigInt(price), offset);
  
  return buffer;
}

async function main() {
  const marketId = process.argv[2] ? parseInt(process.argv[2]) : 1;
  const buyOrderId = process.argv[3] ? parseInt(process.argv[3]) : 3;
  const sellOrderId = process.argv[4] ? parseInt(process.argv[4]) : 4;
  const amount = process.argv[5] ? parseInt(process.argv[5]) : 5_000_000; // 5 tokens
  const priceArg = process.argv[6] ? parseFloat(process.argv[6]) : 0.55;
  
  // If price looks like e6 format (> 1), use as is; otherwise multiply
  const price = priceArg >= 1 ? Math.floor(priceArg) : Math.floor(priceArg * PRICE_PRECISION);
  
  console.log('='.repeat(60));
  console.log('1024 Prediction Market - Execute Trade');
  console.log('='.repeat(60));
  console.log(`Market ID: ${marketId}`);
  console.log(`Buy Order ID: ${buyOrderId}`);
  console.log(`Sell Order ID: ${sellOrderId}`);
  console.log(`Amount: ${amount / 1_000_000} tokens`);
  console.log(`Price: $${priceArg}`);
  
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
  
  // Derive order PDAs
  const buyOrderIdBytes = Buffer.alloc(8);
  buyOrderIdBytes.writeBigUInt64LE(BigInt(buyOrderId));
  const [buyOrderPda] = PublicKey.findProgramAddressSync(
    [ORDER_SEED, marketIdBytes, buyOrderIdBytes],
    PROGRAM_ID
  );
  
  const sellOrderIdBytes = Buffer.alloc(8);
  sellOrderIdBytes.writeBigUInt64LE(BigInt(sellOrderId));
  const [sellOrderPda] = PublicKey.findProgramAddressSync(
    [ORDER_SEED, marketIdBytes, sellOrderIdBytes],
    PROGRAM_ID
  );
  
  console.log(`  Buy Order PDA: ${buyOrderPda.toBase58()}`);
  console.log(`  Sell Order PDA: ${sellOrderPda.toBase58()}`);
  
  // Get order owners from order accounts
  const buyOrderAccount = await connection.getAccountInfo(buyOrderPda);
  const sellOrderAccount = await connection.getAccountInfo(sellOrderPda);
  
  if (!buyOrderAccount || !sellOrderAccount) {
    console.error('❌ One or both orders not found!');
    return;
  }
  
  // Parse order info (owner at offset 24, outcome at offset 57)
  const buyerWallet = new PublicKey(buyOrderAccount.data.slice(24, 56));
  const sellerWallet = new PublicKey(sellOrderAccount.data.slice(24, 56));
  const outcome = buyOrderAccount.data[57]; // 0=YES, 1=NO
  
  console.log(`  Buyer: ${buyerWallet.toBase58()}`);
  console.log(`  Seller: ${sellerWallet.toBase58()}`);
  console.log(`  Outcome: ${outcome === 0 ? 'YES' : 'NO'}`);
  
  // Get token accounts
  const tokenMint = outcome === 0 ? yesMint : noMint;
  const buyerTokenAta = await getAssociatedTokenAddress(tokenMint, buyerWallet);
  const sellerTokenAta = await getAssociatedTokenAddress(tokenMint, sellerWallet);
  
  console.log(`  Buyer Token ATA: ${buyerTokenAta.toBase58()}`);
  console.log(`  Seller Token ATA: ${sellerTokenAta.toBase58()}`);
  
  // Serialize instruction
  const instructionData = serializeExecuteTradeArgs(marketId, buyOrderId, sellOrderId, amount, price);
  console.log(`\nInstruction data: ${instructionData.toString('hex')}`);
  
  /**
   * Accounts for ExecuteTrade (from processor.rs):
   * 0. [signer] Relayer
   * 1. [writable] Config
   * 2. [writable] Market
   * 3. [writable] Buy Order
   * 4. [writable] Sell Order
   * 5. [writable] Seller's Token Account
   * 6. [writable] Buyer's Token Account
   * 7. [] Token Program
   */
  const executeTradeIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: caller.publicKey, isSigner: true, isWritable: true },
      { pubkey: configPda, isSigner: false, isWritable: true },
      { pubkey: marketPda, isSigner: false, isWritable: true },
      { pubkey: buyOrderPda, isSigner: false, isWritable: true },
      { pubkey: sellOrderPda, isSigner: false, isWritable: true },
      { pubkey: sellerTokenAta, isSigner: false, isWritable: true },
      { pubkey: buyerTokenAta, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    data: instructionData,
  });
  
  const tx = new Transaction().add(executeTradeIx);
  
  console.log('\nSending ExecuteTrade transaction...');
  
  try {
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.lastValidBlockHeight = lastValidBlockHeight;
    tx.feePayer = caller.publicKey;
    
    const signature = await sendAndConfirmTransaction(connection, tx, [caller], {
      commitment: 'confirmed',
    });
    
    console.log('\n✅ ExecuteTrade successful!');
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
