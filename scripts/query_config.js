/**
 * Query Config - Query the prediction market config
 */

const { Connection, PublicKey } = require('@solana/web3.js');
const config = require('./config');

// 新的 Program ID (V3 - 2025-12-12)
const PROGRAM_ID = config.PROGRAM_ID;
const PM_CONFIG_SEED = Buffer.from('pm_config');
const RPC_URL = config.RPC_URL;

async function main() {
  console.log('='.repeat(60));
  console.log('1024 Prediction Market - Query Config');
  console.log('='.repeat(60));
  console.log(`Program ID: ${PROGRAM_ID.toBase58()}`);
  
  const connection = new Connection(RPC_URL, 'confirmed');
  
  const [configPda] = PublicKey.findProgramAddressSync([PM_CONFIG_SEED], PROGRAM_ID);
  console.log(`Config PDA: ${configPda.toBase58()}`);
  
  const configAccount = await connection.getAccountInfo(configPda);
  if (!configAccount) {
    console.error('❌ Config not found!');
    return;
  }
  
  const data = configAccount.data;
  console.log(`Account size: ${data.length} bytes`);
  
  // Parse config
  // discriminator(8) + admin(32) + usdc_mint(32) + vault_program(32) + fund_program(32) + 
  // oracle_admin(32) + next_market_id(8) + total_markets(8) + active_markets(8) + 
  // total_volume_e6(8) + total_minted_sets(8) + challenge_window_secs(8) + proposer_bond_e6(8) + 
  // is_paused(1) + default_platform_fee_bps(2) + ...
  
  let offset = 8; // skip discriminator
  
  const admin = new PublicKey(data.slice(offset, offset + 32)); offset += 32;
  const usdcMint = new PublicKey(data.slice(offset, offset + 32)); offset += 32;
  const vaultProgram = new PublicKey(data.slice(offset, offset + 32)); offset += 32;
  const fundProgram = new PublicKey(data.slice(offset, offset + 32)); offset += 32;
  const oracleAdmin = new PublicKey(data.slice(offset, offset + 32)); offset += 32;
  
  const nextMarketId = data.readBigUInt64LE(offset); offset += 8;
  const totalMarkets = data.readBigUInt64LE(offset); offset += 8;
  const activeMarkets = data.readBigUInt64LE(offset); offset += 8;
  const totalVolumeE6 = data.readBigInt64LE(offset); offset += 8;
  const totalMintedSets = data.readBigUInt64LE(offset); offset += 8;
  const challengeWindowSecs = data.readBigInt64LE(offset); offset += 8;
  const proposerBondE6 = data.readBigUInt64LE(offset); offset += 8;
  const isPaused = data[offset] === 1; offset += 1;
  const platformFeeBps = data.readUInt16LE(offset);
  
  console.log(`\nConfig Data:`);
  console.log(`  Admin: ${admin.toBase58()}`);
  console.log(`  USDC Mint: ${usdcMint.toBase58()}`);
  console.log(`  Vault Program: ${vaultProgram.toBase58()}`);
  console.log(`  Fund Program: ${fundProgram.toBase58()}`);
  console.log(`  Oracle Admin: ${oracleAdmin.toBase58()}`);
  console.log(`  Next Market ID: ${nextMarketId}`);
  console.log(`  Total Markets: ${totalMarkets}`);
  console.log(`  Active Markets: ${activeMarkets}`);
  console.log(`  Total Volume (e6): ${totalVolumeE6}`);
  console.log(`  Total Minted Sets: ${totalMintedSets}`);
  console.log(`  Challenge Window: ${challengeWindowSecs} seconds`);
  console.log(`  Proposer Bond (e6): ${proposerBondE6}`);
  console.log(`  Is Paused: ${isPaused}`);
  console.log(`  Platform Fee (bps): ${platformFeeBps}`);
  
  console.log('\n' + '='.repeat(60));
}

main().catch(console.error);
