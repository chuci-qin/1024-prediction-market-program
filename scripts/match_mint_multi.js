/**
 * MatchMintMulti - Match N buy orders to mint complete set for multi-outcome market
 * 
 * Usage: node match_mint_multi.js [market_id] [amount] [order1_id:price] [order2_id:price] ...
 * 
 * Example (3-outcome market):
 *   node match_mint_multi.js 5 10000000 101:0.33 102:0.33 103:0.34
 *   
 * Requires: sum of all prices ≈ 1.0 (within tolerance)
 */

const { 
  Connection, 
  Keypair, 
  PublicKey, 
  Transaction, 
  TransactionInstruction,
} = require('@solana/web3.js');
const { TOKEN_PROGRAM_ID, getAssociatedTokenAddress } = require('@solana/spl-token');
const fs = require('fs');

// 1024Chain Testnet 配置
const RPC_URL = 'https://testnet-rpc.1024chain.com/rpc/';
const PROGRAM_ID = new PublicKey('FVtPQkdYvSNdpTA6QXYRcTBhDGgnufw2Enqmo2tQKr58');

// Seeds
const PM_CONFIG_SEED = Buffer.from('pm_config');
const MARKET_SEED = Buffer.from('market');
const ORDER_SEED = Buffer.from('order');
const OUTCOME_MINT_SEED = Buffer.from('outcome_mint');

// MatchMintMulti = instruction index 42
const MATCH_MINT_MULTI_IX = 42;

const PRICE_PRECISION = 1_000_000;

/**
 * Serialize MatchMintMultiArgs (Borsh format)
 * Layout:
 * - u8 instruction (42)
 * - u64 market_id
 * - u8 num_outcomes
 * - u64 amount
 * - u32 vec_len (Borsh Vec length prefix)
 * - For each order: (u8 outcome_index, u64 order_id, u64 price_e6)
 */
function serializeMatchMintMultiArgs(marketId, numOutcomes, amount, orders) {
  // Calculate buffer size: 1 + 8 + 1 + 8 + 4 + (orders.length * (1 + 8 + 8))
  const orderDataSize = orders.length * (1 + 8 + 8);
  const buffer = Buffer.alloc(1 + 8 + 1 + 8 + 4 + orderDataSize);
  let offset = 0;
  
  // Instruction index
  buffer.writeUInt8(MATCH_MINT_MULTI_IX, offset); offset += 1;
  
  // market_id (u64)
  buffer.writeBigUInt64LE(BigInt(marketId), offset); offset += 8;
  
  // num_outcomes (u8)
  buffer.writeUInt8(numOutcomes, offset); offset += 1;
  
  // amount (u64)
  buffer.writeBigUInt64LE(BigInt(amount), offset); offset += 8;
  
  // Vec length prefix (u32 for Borsh)
  buffer.writeUInt32LE(orders.length, offset); offset += 4;
  
  // Each order: (outcome_index: u8, order_id: u64, price_e6: u64)
  for (const order of orders) {
    buffer.writeUInt8(order.outcomeIndex, offset); offset += 1;
    buffer.writeBigUInt64LE(BigInt(order.orderId), offset); offset += 8;
    buffer.writeBigUInt64LE(BigInt(order.price), offset); offset += 8;
  }
  
  return buffer;
}

/**
 * 快速发送交易 (适配 4x tick 速度链)
 */
async function sendTransactionFast(connection, transaction, signers) {
  const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash('confirmed');
  transaction.recentBlockhash = blockhash;
  transaction.lastValidBlockHeight = lastValidBlockHeight;
  transaction.feePayer = signers[0].publicKey;
  
  transaction.sign(...signers);
  
  const signature = await connection.sendRawTransaction(transaction.serialize(), {
    skipPreflight: false,
    preflightCommitment: 'confirmed',
    maxRetries: 3,
  });
  
  // Quick confirmation check
  for (let i = 0; i < 10; i++) {
    await new Promise(r => setTimeout(r, 500));
    const status = await connection.getSignatureStatus(signature);
    if (status.value?.confirmationStatus === 'confirmed' || status.value?.confirmationStatus === 'finalized') {
      return { signature, confirmed: true };
    }
    if (status.value?.err) {
      throw new Error(`Transaction failed: ${JSON.stringify(status.value.err)}`);
    }
  }
  
  return { signature, confirmed: false };
}

async function main() {
  console.log('='.repeat(60));
  console.log('1024 Prediction Market - MatchMintMulti (Multi-Outcome)');
  console.log('='.repeat(60));
  
  // Parse arguments
  const marketId = process.argv[2] ? parseInt(process.argv[2]) : 1;
  const amount = process.argv[3] ? parseInt(process.argv[3]) : 10_000_000; // 10 tokens
  
  // Parse order:price pairs
  const orders = [];
  for (let i = 4; i < process.argv.length; i++) {
    const [orderId, priceStr] = process.argv[i].split(':');
    const price = Math.floor(parseFloat(priceStr) * PRICE_PRECISION);
    orders.push({
      outcomeIndex: i - 4, // 0-based index
      orderId: parseInt(orderId),
      price: price,
    });
  }
  
  // Default example if no orders provided
  if (orders.length === 0) {
    console.log('\nNo orders provided. Using example (2-outcome market):');
    orders.push({ outcomeIndex: 0, orderId: 1, price: 600_000 }); // YES @ $0.60
    orders.push({ outcomeIndex: 1, orderId: 2, price: 400_000 }); // NO @ $0.40
  }
  
  const numOutcomes = orders.length;
  const totalPrice = orders.reduce((sum, o) => sum + o.price, 0);
  
  console.log(`\nMarket ID: ${marketId}`);
  console.log(`Amount: ${amount / 1_000_000} tokens`);
  console.log(`Number of Outcomes: ${numOutcomes}`);
  console.log(`Orders:`);
  orders.forEach((o, i) => {
    console.log(`  ${i}: Order #${o.orderId} @ $${(o.price / PRICE_PRECISION).toFixed(4)}`);
  });
  console.log(`Total Price: $${(totalPrice / PRICE_PRECISION).toFixed(6)}`);
  
  if (Math.abs(totalPrice - PRICE_PRECISION) > 10000) { // 1% tolerance
    console.log('\n⚠️  Warning: Prices do not sum to 1.0');
  }
  
  const connection = new Connection(RPC_URL, {
    commitment: 'confirmed',
    confirmTransactionInitialTimeout: 15000,
  });
  
  // Load caller keypair
  const keypairPaths = [
    process.env.KEYPAIR_PATH,
    '/Users/chuciqin/Desktop/project1024/1024codebase/1024-chain/keys/faucet.json',
    process.env.HOME + '/1024chain-testnet/keys/faucet.json',
  ].filter(Boolean);
  
  let callerPath;
  for (const p of keypairPaths) {
    if (fs.existsSync(p)) {
      callerPath = p;
      break;
    }
  }
  
  if (!callerPath) {
    console.error('❌ Keypair file not found!');
    return;
  }
  
  const callerData = JSON.parse(fs.readFileSync(callerPath, 'utf-8'));
  const caller = Keypair.fromSecretKey(new Uint8Array(callerData));
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
  
  // Parse market vault from market account
  const marketData = marketAccount.data;
  const marketVault = new PublicKey(marketData.slice(176, 208)); // Adjust offset as needed
  
  console.log(`\nMarket PDA: ${marketPda.toBase58()}`);
  console.log(`Market Vault: ${marketVault.toBase58()}`);
  
  // Build accounts array
  const keys = [
    { pubkey: caller.publicKey, isSigner: true, isWritable: true },
    { pubkey: configPda, isSigner: false, isWritable: false },
    { pubkey: marketPda, isSigner: false, isWritable: true },
    { pubkey: marketVault, isSigner: false, isWritable: true },
    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    { pubkey: new PublicKey('11111111111111111111111111111111'), isSigner: false, isWritable: false }, // System Program
  ];
  
  // Add dynamic accounts for each outcome (Order, Mint, TokenAccount)
  for (const order of orders) {
    // Order PDA
    const orderIdBytes = Buffer.alloc(8);
    orderIdBytes.writeBigUInt64LE(BigInt(order.orderId));
    const [orderPda] = PublicKey.findProgramAddressSync(
      [ORDER_SEED, marketIdBytes, orderIdBytes],
      PROGRAM_ID
    );
    
    // Get order owner from order account
    const orderAccount = await connection.getAccountInfo(orderPda);
    if (!orderAccount) {
      console.error(`❌ Order #${order.orderId} not found!`);
      return;
    }
    const orderOwner = new PublicKey(orderAccount.data.slice(24, 56));
    
    // Outcome Token Mint PDA
    const outcomeIndexByte = Buffer.from([order.outcomeIndex]);
    const [outcomeMintPda] = PublicKey.findProgramAddressSync(
      [OUTCOME_MINT_SEED, marketPda.toBuffer(), outcomeIndexByte],
      PROGRAM_ID
    );
    
    // Buyer's Token Account (ATA)
    const buyerTokenAccount = await getAssociatedTokenAddress(outcomeMintPda, orderOwner);
    
    console.log(`  Outcome ${order.outcomeIndex}:`);
    console.log(`    Order PDA: ${orderPda.toBase58()}`);
    console.log(`    Owner: ${orderOwner.toBase58()}`);
    console.log(`    Mint: ${outcomeMintPda.toBase58()}`);
    console.log(`    Token Account: ${buyerTokenAccount.toBase58()}`);
    
    keys.push({ pubkey: orderPda, isSigner: false, isWritable: true });
    keys.push({ pubkey: outcomeMintPda, isSigner: false, isWritable: true });
    keys.push({ pubkey: buyerTokenAccount, isSigner: false, isWritable: true });
  }
  
  // Serialize instruction
  const instructionData = serializeMatchMintMultiArgs(marketId, numOutcomes, amount, orders);
  console.log(`\nInstruction data (${instructionData.length} bytes): ${instructionData.toString('hex').slice(0, 60)}...`);
  
  const matchMintMultiIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: keys,
    data: instructionData,
  });
  
  const tx = new Transaction().add(matchMintMultiIx);
  
  console.log('\nSending MatchMintMulti transaction...');
  
  try {
    const { signature, confirmed } = await sendTransactionFast(connection, tx, [caller]);
    
    if (confirmed) {
      console.log('\n✅ MatchMintMulti successful!');
    } else {
      console.log('\n⚠️  TX sent, confirmation pending');
    }
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
