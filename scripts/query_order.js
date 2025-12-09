/**
 * Query Order - Get order details
 * Run on server: node query_order.js [market_id] [order_id]
 */

const { Connection, PublicKey } = require('@solana/web3.js');

const PROGRAM_ID = new PublicKey('FVtPQkdYvSNdpTA6QXYRcTBhDGgnufw2Enqmo2tQKr58');
const ORDER_SEED = Buffer.from('order');
const MARKET_SEED = Buffer.from('market');

const OrderSide = ['Buy', 'Sell'];
const Outcome = ['Yes', 'No'];
const OrderStatus = ['Pending', 'Active', 'PartiallyFilled', 'Filled', 'Cancelled', 'Expired'];
const OrderType = ['Limit', 'Market', 'GTD'];

async function main() {
  const marketId = process.argv[2] ? parseInt(process.argv[2]) : 1;
  const orderId = process.argv[3] ? parseInt(process.argv[3]) : 1;
  
  console.log('='.repeat(60));
  console.log(`1024 Prediction Market - Query Order ${marketId}/${orderId}`);
  console.log('='.repeat(60));
  
  const connection = new Connection('http://127.0.0.1:8899', 'confirmed');
  
  // Derive Order PDA
  const marketIdBytes = Buffer.alloc(8);
  marketIdBytes.writeBigUInt64LE(BigInt(marketId));
  
  const orderIdBytes = Buffer.alloc(8);
  orderIdBytes.writeBigUInt64LE(BigInt(orderId));
  
  const [orderPda] = PublicKey.findProgramAddressSync(
    [ORDER_SEED, marketIdBytes, orderIdBytes],
    PROGRAM_ID
  );
  
  console.log(`\nOrder PDA: ${orderPda.toBase58()}`);
  
  const account = await connection.getAccountInfo(orderPda);
  if (!account) {
    console.log('❌ Order not found');
    return;
  }
  
  const data = account.data;
  console.log(`Account size: ${data.length} bytes`);
  
  // Parse Order data
  // Order structure (from state.rs):
  // discriminator(8) + order_id(8) + market_id(8) + owner(32)
  // + side(1) + outcome(1) + price(8) + amount(8) + filled_amount(8)
  // + status(1) + order_type(1) + expiration_time(1-9) + created_at(8) + updated_at(8)
  // + bump(1) + reserved(32)
  
  let offset = 0;
  
  const discriminator = data.readBigUInt64LE(offset); offset += 8;
  console.log('\n=== Order Details ===');
  console.log('discriminator:', discriminator.toString(16));
  
  const orderIdFromData = data.readBigUInt64LE(offset); offset += 8;
  console.log('order_id:', orderIdFromData.toString());
  
  const marketIdFromData = data.readBigUInt64LE(offset); offset += 8;
  console.log('market_id:', marketIdFromData.toString());
  
  const owner = new PublicKey(data.slice(offset, offset + 32));
  offset += 32;
  console.log('owner:', owner.toBase58());
  
  const side = data[offset]; offset += 1;
  console.log('side:', OrderSide[side] || side);
  
  const outcome = data[offset]; offset += 1;
  console.log('outcome:', Outcome[outcome] || outcome);
  
  const price = data.readBigUInt64LE(offset); offset += 8;
  console.log('price:', price.toString(), `($${Number(price) / 1000000})`);
  
  const amount = data.readBigUInt64LE(offset); offset += 8;
  console.log('amount:', amount.toString(), `(${Number(amount) / 1000000} tokens)`);
  
  const filledAmount = data.readBigUInt64LE(offset); offset += 8;
  console.log('filled_amount:', filledAmount.toString());
  
  const status = data[offset]; offset += 1;
  console.log('status:', OrderStatus[status] || status);
  
  const orderType = data[offset]; offset += 1;
  console.log('order_type:', OrderType[orderType] || orderType);
  
  console.log('\n' + '='.repeat(60));
}

main().catch(console.error);
