/**
 * Query market status and details
 * Run on server: node query_market.js [market_id]
 */

const { Connection, PublicKey } = require('@solana/web3.js');

const PROGRAM_ID = new PublicKey('FVtPQkdYvSNdpTA6QXYRcTBhDGgnufw2Enqmo2tQKr58');
const RPC_URL = 'https://testnet-rpc.1024chain.com/rpc/';
const PM_CONFIG_SEED = Buffer.from('pm_config');
const MARKET_SEED = Buffer.from('market');

// MarketStatus enum from state.rs: Pending=0, Active=1, Paused=2, Resolved=3, Cancelled=4
const MarketStatus = ['Pending', 'Active', 'Paused', 'Resolved', 'Cancelled'];

async function main() {
  const marketId = process.argv[2] ? parseInt(process.argv[2]) : 1;
  
  console.log('='.repeat(60));
  console.log(`1024 Prediction Market - Query Market ${marketId}`);
  console.log('='.repeat(60));
  console.log(`Program ID: ${PROGRAM_ID.toBase58()}`);
  
  const connection = new Connection(RPC_URL, 'confirmed');
  
  // Query Config
  const [configPda] = PublicKey.findProgramAddressSync([PM_CONFIG_SEED], PROGRAM_ID);
  const config = await connection.getAccountInfo(configPda);
  
  console.log('\n=== Config ===');
  console.log('next_market_id:', config.data.readBigUInt64LE(168).toString());
  console.log('total_markets:', config.data.readBigUInt64LE(176).toString());
  console.log('active_markets:', config.data.readBigUInt64LE(184).toString());
  
  // Query Market
  const marketIdBytes = Buffer.alloc(8);
  marketIdBytes.writeBigUInt64LE(BigInt(marketId));
  const [marketPda] = PublicKey.findProgramAddressSync([MARKET_SEED, marketIdBytes], PROGRAM_ID);
  
  const market = await connection.getAccountInfo(marketPda);
  if (!market) {
    console.log(`\n❌ Market ${marketId} not found`);
    return;
  }
  
  const data = market.data;
  console.log('\n=== Market ===');
  console.log('market_id:', data.readBigUInt64LE(8).toString());
  console.log('creator:', new PublicKey(data.slice(16, 48)).toBase58());
  console.log('yes_mint:', new PublicKey(data.slice(112, 144)).toBase58());
  console.log('no_mint:', new PublicKey(data.slice(144, 176)).toBase58());
  console.log('vault:', new PublicKey(data.slice(176, 208)).toBase58());
  console.log('status:', MarketStatus[data[208]] || data[208]);
  console.log('resolution_time:', new Date(Number(data.readBigInt64LE(210)) * 1000).toISOString());
  console.log('finalization_deadline:', new Date(Number(data.readBigInt64LE(218)) * 1000).toISOString());
  
  // Parse remaining fields after Option<MarketResult>
  // final_result starts at 226, it's Option<u8>: 0 = None, 1+ = Some(value-1)
  const finalResultTag = data[226];
  if (finalResultTag === 0) {
    console.log('final_result: None');
  } else {
    const resultValue = data[227];
    const results = ['Yes', 'No', 'Invalid', 'Cancelled'];
    console.log('final_result:', results[resultValue] || resultValue);
  }
  
  console.log('\n' + '='.repeat(60));
}

main().catch(console.error);
