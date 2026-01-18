/**
 * Place Order - Create a buy/sell order for YES or NO tokens
 * Run on server: node place_order.js [market_id] [side] [outcome] [price] [amount]
 * 
 * Example:
 *   node place_order.js 1 buy yes 0.60 50000000  # Buy 50 YES at $0.60
 *   node place_order.js 1 sell no 0.40 30000000  # Sell 30 NO at $0.40
 * 
 * Note: Sell orders now lock tokens in an escrow account!
 */

const { 
  Connection, 
  Keypair, 
  PublicKey, 
  Transaction, 
  TransactionInstruction,
  sendAndConfirmTransaction,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
} = require('@solana/web3.js');
const config = require('./config');
const { TOKEN_PROGRAM_ID, getAssociatedTokenAddress, ASSOCIATED_TOKEN_PROGRAM_ID } = require('@solana/spl-token');
const fs = require('fs');

const PROGRAM_ID = config.PROGRAM_ID;
const RPC_URL = config.RPC_URL;

// Seeds
const PM_CONFIG_SEED = Buffer.from('pm_config');
const MARKET_SEED = Buffer.from('market');
const ORDER_SEED = Buffer.from('order');
const ORDER_ESCROW_SEED = Buffer.from('order_escrow');

// Instruction index
const PLACE_ORDER_IX = 9;

// Constants
const PRICE_PRECISION = 1_000_000; // 1e6

// Enums
const OrderSide = {
  Buy: 0,
  Sell: 1,
};

const Outcome = {
  Yes: 0,
  No: 1,
};

const OrderType = {
  Limit: 0,
  Market: 1,
};

/**
 * Serialize PlaceOrderArgs
 * Layout:
 * - u8 instruction (9)
 * - u64 market_id
 * - u8 side (0=Buy, 1=Sell)
 * - u8 outcome (0=Yes, 1=No)
 * - u64 price (e6)
 * - u64 amount
 * - u8 order_type (0=Limit)
 * - u8 has_expiration (0=None)
 * - (if has_expiration: i64 expiration_time)
 */
function serializePlaceOrderArgs(marketId, side, outcome, price, amount, orderType = 0) {
  const buffer = Buffer.alloc(1 + 8 + 1 + 1 + 8 + 8 + 1 + 1);
  let offset = 0;
  
  buffer.writeUInt8(PLACE_ORDER_IX, offset); offset += 1;
  buffer.writeBigUInt64LE(BigInt(marketId), offset); offset += 8;
  buffer.writeUInt8(side, offset); offset += 1;
  buffer.writeUInt8(outcome, offset); offset += 1;
  buffer.writeBigUInt64LE(BigInt(price), offset); offset += 8;
  buffer.writeBigUInt64LE(BigInt(amount), offset); offset += 8;
  buffer.writeUInt8(orderType, offset); offset += 1;
  buffer.writeUInt8(0, offset); // has_expiration = None
  
  return buffer;
}

async function main() {
  const marketId = process.argv[2] ? parseInt(process.argv[2]) : 1;
  const sideArg = (process.argv[3] || 'buy').toLowerCase();
  const outcomeArg = (process.argv[4] || 'yes').toLowerCase();
  const priceArg = process.argv[5] ? parseFloat(process.argv[5]) : 0.60;
  const amountArg = process.argv[6] ? parseInt(process.argv[6]) : 10_000_000; // 10 tokens
  
  const side = sideArg === 'sell' ? OrderSide.Sell : OrderSide.Buy;
  const outcome = outcomeArg === 'no' ? Outcome.No : Outcome.Yes;
  const priceE6 = Math.floor(priceArg * PRICE_PRECISION);
  
  console.log('='.repeat(60));
  console.log('1024 Prediction Market - Place Order');
  console.log('='.repeat(60));
  console.log(`Market ID: ${marketId}`);
  console.log(`Side: ${sideArg.toUpperCase()}`);
  console.log(`Outcome: ${outcomeArg.toUpperCase()}`);
  console.log(`Price: $${priceArg} (${priceE6} e6)`);
  console.log(`Amount: ${amountArg / 1_000_000} tokens`);
  
  const connection = new Connection(RPC_URL, {
    commitment: 'confirmed',
    confirmTransactionInitialTimeout: 60000,
  });
  
  const faucetPath = process.env.HOME + '/1024chain-testnet/keys/faucet.json';
  const faucetData = JSON.parse(fs.readFileSync(faucetPath, 'utf-8'));
  const user = Keypair.fromSecretKey(new Uint8Array(faucetData));
  console.log(`\nUser: ${user.publicKey.toBase58()}`);
  
  // Derive PDAs
  const [configPda] = PublicKey.findProgramAddressSync([PM_CONFIG_SEED], PROGRAM_ID);
  
  const marketIdBytes = Buffer.alloc(8);
  marketIdBytes.writeBigUInt64LE(BigInt(marketId));
  const [marketPda] = PublicKey.findProgramAddressSync([MARKET_SEED, marketIdBytes], PROGRAM_ID);
  
  // Get market info to find next_order_id
  const marketAccount = await connection.getAccountInfo(marketPda);
  if (!marketAccount) {
    console.error('❌ Market not found!');
    return;
  }
  
  const marketData = marketAccount.data;
  const yesMint = new PublicKey(marketData.slice(112, 144));
  const noMint = new PublicKey(marketData.slice(144, 176));
  
  // Parse next_order_id from market data
  // Market structure: after final_result (variable), we need to find next_order_id
  // Let's calculate the offset based on the structure
  // Offset for next_order_id: after creator_fee_bps (u16) which is after open_interest
  // Actually, let me read it properly from the market layout
  
  console.log(`\nMarket Info:`);
  console.log(`  YES Mint: ${yesMint.toBase58()}`);
  console.log(`  NO Mint: ${noMint.toBase58()}`);
  
  // Read next_order_id - need to calculate proper offset
  // Market structure:
  // discriminator(8) + market_id(8) + creator(32) + question_hash(32) + resolution_spec_hash(32)
  // + yes_mint(32) + no_mint(32) + market_vault(32) + status(1) + review_status(1)
  // + resolution_time(8) + finalization_deadline(8) + final_result(1-2) + created_at(8)
  // + updated_at(8) + total_minted(8) + total_volume_e6(8) + open_interest(8)
  // + creator_fee_bps(2) + next_order_id(8)
  
  // Since final_result is Option<MarketResult> with variable length, let's skip to the end
  // and work backwards from bump which is at a known position
  // Actually, let's just read from the config to get the pattern working
  
  // Get next_order_id from market account
  // Market structure with fixed Option<MarketResult> analysis:
  // After finalization_deadline(8) at offset 218, we have:
  // - final_result: Option<MarketResult> at 226 (1 byte if None, 2 if Some)
  // - If None (0x00), next fields start at 227
  // Let's find next_order_id by counting from status:
  // status(208) + review_status(209) + resolution_time(8) + finalization_deadline(8) + final_result(1-2)
  // + created_at(8) + updated_at(8) + total_minted(8) + total_volume_e6(8) + open_interest(8)
  // + creator_fee_bps(2) + next_order_id(8)
  
  // Check if final_result is None (0) or Some
  const finalResultTag = marketData[226];
  let nextOrderIdOffset;
  if (finalResultTag === 0) {
    // None: 1 byte
    // 227 + created_at(8) + updated_at(8) + total_minted(8) + total_volume_e6(8) + open_interest(8) + creator_fee_bps(2) = 269
    nextOrderIdOffset = 227 + 8 + 8 + 8 + 8 + 8 + 2;
  } else {
    // Some: 2 bytes
    nextOrderIdOffset = 228 + 8 + 8 + 8 + 8 + 8 + 2;
  }
  
  const orderId = marketData.readBigUInt64LE(nextOrderIdOffset);
  console.log(`  next_order_id from market: ${orderId}`);
  
  // Derive Order PDA
  const orderIdBytes = Buffer.alloc(8);
  orderIdBytes.writeBigUInt64LE(orderId);
  const [orderPda] = PublicKey.findProgramAddressSync(
    [ORDER_SEED, marketIdBytes, orderIdBytes],
    PROGRAM_ID
  );
  console.log(`  Order PDA (id=${orderId}): ${orderPda.toBase58()}`);
  
  // Serialize instruction
  const instructionData = serializePlaceOrderArgs(marketId, side, outcome, priceE6, amountArg);
  console.log(`\nInstruction data: ${instructionData.toString('hex')}`);
  
  /**
   * Accounts for PlaceOrder (from processor.rs):
   * 0. [signer] User
   * 1. [] Config
   * 2. [writable] Market
   * 3. [writable] Order PDA
   * 4. [] System Program
   * 
   * Additional for Sell orders:
   * 5. [] Token Mint (YES or NO based on outcome)
   * 6. [writable] User's Token Account
   * 7. [writable] Escrow Token Account (PDA)
   * 8. [] Token Program
   * 9. [] Rent Sysvar
   */
  
  const isSellOrder = side === OrderSide.Sell;
  
  // Base accounts for all orders
  const accounts = [
    { pubkey: user.publicKey, isSigner: true, isWritable: true },
    { pubkey: configPda, isSigner: false, isWritable: false },
    { pubkey: marketPda, isSigner: false, isWritable: true },
    { pubkey: orderPda, isSigner: false, isWritable: true },
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
  ];
  
  // Add additional accounts for Sell orders
  if (isSellOrder) {
    const tokenMint = outcome === Outcome.Yes ? yesMint : noMint;
    const userTokenAccount = await getAssociatedTokenAddress(tokenMint, user.publicKey);
    
    // Derive escrow PDA
    const [escrowPda] = PublicKey.findProgramAddressSync(
      [ORDER_ESCROW_SEED, marketIdBytes, orderIdBytes],
      PROGRAM_ID
    );
    
    console.log(`\nSell Order Escrow Setup:`);
    console.log(`  Token Mint: ${tokenMint.toBase58()}`);
    console.log(`  User Token Account: ${userTokenAccount.toBase58()}`);
    console.log(`  Escrow PDA: ${escrowPda.toBase58()}`);
    
    // Check user has enough tokens
    try {
      const tokenBalance = await connection.getTokenAccountBalance(userTokenAccount);
      console.log(`  User Token Balance: ${tokenBalance.value.uiAmount}`);
      if (BigInt(tokenBalance.value.amount) < BigInt(amountArg)) {
        console.error(`\n❌ Insufficient token balance! Have ${tokenBalance.value.uiAmount}, need ${amountArg / 1_000_000}`);
        return;
      }
    } catch (e) {
      console.error(`\n❌ Could not find user token account: ${userTokenAccount.toBase58()}`);
      console.error('Make sure you have minted tokens first using mint_tokens.js');
      return;
    }
    
    accounts.push(
      { pubkey: tokenMint, isSigner: false, isWritable: false },
      { pubkey: userTokenAccount, isSigner: false, isWritable: true },
      { pubkey: escrowPda, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
    );
  }
  
  const placeOrderIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: accounts,
    data: instructionData,
  });
  
  const tx = new Transaction().add(placeOrderIx);
  
  console.log('\nSending PlaceOrder transaction...');
  
  try {
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.lastValidBlockHeight = lastValidBlockHeight;
    tx.feePayer = user.publicKey;
    
    const signature = await sendAndConfirmTransaction(connection, tx, [user], {
      commitment: 'confirmed',
    });
    
    console.log('\n✅ PlaceOrder successful!');
    console.log(`Signature: ${signature}`);
    console.log(`Order ID: ${orderId}`);
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
