/**
 * Initialize 1024 Prediction Market Program on 1024Chain Testnet
 */

const { 
  Connection, 
  Keypair, 
  PublicKey, 
  Transaction, 
  TransactionInstruction,
  SystemProgram,
  sendAndConfirmTransaction 
} = require('@solana/web3.js');

// 1024Chain Testnet RPC
const RPC_URL = 'https://testnet-rpc.1024chain.com/rpc/';

// Program and account IDs
const PROGRAM_ID = new PublicKey('FVtPQkdYvSNdpTA6QXYRcTBhDGgnufw2Enqmo2tQKr58');
const VAULT_PROGRAM = new PublicKey('8n3FHwYxFgQCQc2FNFkwDUf9mcqupxXcCvgfHbApUzYU');
const FUND_PROGRAM = new PublicKey('FPhDzu7yCDC1BBvzGwpM6dHHNQBPpKEv6Y3Ptdc7o3fJ');
const USDC_MINT = new PublicKey('7pCrfxhcAEyTFDhrhKRtRS2iMvEYx2dtNE7NzwuU7SA9');

// PDA Seeds
const PM_CONFIG_SEED = Buffer.from('pm_config');

// Relayer keypair from 1024-core/.env
const RELAYER_KEYPAIR = [9,201,67,159,134,166,247,250,175,67,60,55,49,132,104,141,207,35,62,44,129,223,128,15,8,206,189,184,216,157,244,27,16,42,227,1,241,96,112,131,253,96,7,205,80,14,207,215,38,236,183,121,99,16,116,102,82,186,3,234,3,4,107,113];

/**
 * Serialize InitializeArgs for the program
 */
function serializeInitializeArgs(oracleAdmin, challengeWindowSecs, proposerBondE6) {
  const buffer = Buffer.alloc(1 + 32 + 8 + 8);
  let offset = 0;
  
  // Instruction index = 0 (Initialize)
  buffer.writeUInt8(0, offset);
  offset += 1;
  
  // Oracle admin pubkey (32 bytes)
  oracleAdmin.toBuffer().copy(buffer, offset);
  offset += 32;
  
  // Challenge window seconds (i64 little-endian)
  buffer.writeBigInt64LE(BigInt(challengeWindowSecs), offset);
  offset += 8;
  
  // Proposer bond e6 (u64 little-endian)
  buffer.writeBigUInt64LE(BigInt(proposerBondE6), offset);
  
  return buffer;
}

async function main() {
  console.log('='.repeat(60));
  console.log('1024 Prediction Market Program - Initialize on Testnet');
  console.log('='.repeat(60));
  
  // Connect to 1024Chain Testnet
  const connection = new Connection(RPC_URL, 'confirmed');
  console.log(`Connected to: ${RPC_URL}`);
  
  // Load relayer keypair
  const admin = Keypair.fromSecretKey(new Uint8Array(RELAYER_KEYPAIR));
  console.log(`Admin/Relayer: ${admin.publicKey.toBase58()}`);
  
  // Check balance
  const balance = await connection.getBalance(admin.publicKey);
  console.log(`Balance: ${balance / 1e9} SOL`);
  
  if (balance < 1e9) {
    console.error('❌ Insufficient balance! Need at least 1 SOL');
    return;
  }
  
  // Derive Config PDA
  const [configPda, configBump] = PublicKey.findProgramAddressSync(
    [PM_CONFIG_SEED],
    PROGRAM_ID
  );
  console.log(`Config PDA: ${configPda.toBase58()} (bump: ${configBump})`);
  
  // Check if program is deployed
  const programInfo = await connection.getAccountInfo(PROGRAM_ID);
  if (!programInfo) {
    console.error('❌ Program not deployed at', PROGRAM_ID.toBase58());
    return;
  }
  console.log(`✅ Program deployed (${programInfo.data.length} bytes)`);
  
  // Check if already initialized
  const configAccount = await connection.getAccountInfo(configPda);
  if (configAccount) {
    console.log('\n✅ Program already initialized!');
    console.log(`Account size: ${configAccount.data.length} bytes`);
    console.log(`Owner: ${configAccount.owner.toBase58()}`);
    
    // Parse config
    const data = configAccount.data;
    const adminKey = new PublicKey(data.slice(8, 40));
    const usdcMint = new PublicKey(data.slice(40, 72));
    const nextMarketId = data.readBigUInt64LE(168);
    
    console.log(`\nConfig:`);
    console.log(`  Admin: ${adminKey.toBase58()}`);
    console.log(`  USDC Mint: ${usdcMint.toBase58()}`);
    console.log(`  Next Market ID: ${nextMarketId}`);
    return;
  }
  
  // Create Initialize instruction
  console.log('\n🚀 Creating Initialize instruction...');
  
  const instructionData = serializeInitializeArgs(
    admin.publicKey,        // oracle_admin = admin
    24 * 60 * 60,           // challenge_window = 24 hours
    100_000_000             // proposer_bond = 100 USDC (e6)
  );
  
  console.log(`Instruction data (${instructionData.length} bytes)`);
  
  const initializeIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: admin.publicKey, isSigner: true, isWritable: true },
      { pubkey: configPda, isSigner: false, isWritable: true },
      { pubkey: USDC_MINT, isSigner: false, isWritable: false },
      { pubkey: VAULT_PROGRAM, isSigner: false, isWritable: false },
      { pubkey: FUND_PROGRAM, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: instructionData,
  });
  
  // Send transaction
  console.log('\n📤 Sending transaction...');
  const tx = new Transaction().add(initializeIx);
  
  try {
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.lastValidBlockHeight = lastValidBlockHeight;
    tx.feePayer = admin.publicKey;
    
    const signature = await sendAndConfirmTransaction(connection, tx, [admin], {
      commitment: 'confirmed',
    });
    
    console.log('\n✅ Initialize successful!');
    console.log(`Signature: ${signature}`);
    console.log(`Explorer: https://testnet-scan.1024chain.com/tx/${signature}`);
  } catch (error) {
    console.error('\n❌ Transaction failed:');
    if (error.logs) {
      console.error('Logs:');
      error.logs.forEach(log => console.error('  ', log));
    }
    console.error(error.message || error);
    return;
  }
  
  // Verify
  console.log('\n🔍 Verifying...');
  const finalConfig = await connection.getAccountInfo(configPda);
  if (finalConfig) {
    console.log('✅ Config account created');
    console.log(`  Size: ${finalConfig.data.length} bytes`);
    console.log(`  Owner: ${finalConfig.owner.toBase58()}`);
  }
  
  console.log('\n' + '='.repeat(60));
}

main().catch(console.error);

