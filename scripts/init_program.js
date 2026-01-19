/**
 * Initialize 1024 Prediction Market Program
 * Run on server: node init_program.js
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
const config = require('./config');
const fs = require('fs');

// Program and account IDs
const PROGRAM_ID = config.PROGRAM_ID;
const VAULT_PROGRAM = new PublicKey('vR3BifKCa2TGKP2uhToxZAMYAYydqpesvKGX54gzFny');
const FUND_PROGRAM = new PublicKey('FPhDzu7yCDC1BBvzGwpM6dHHNQBPpKEv6Y3Ptdc7o3fJ');
const USDC_MINT = new PublicKey('7pCrfxhcAEyTFDhrhKRtRS2iMvEYx2dtNE7NzwuU7SA9');

// PDA Seeds
const PM_CONFIG_SEED = Buffer.from('pm_config');

/**
 * Serialize InitializeArgs for the program
 * Layout:
 * - u8 instruction (0 = Initialize)
 * - [u8; 32] oracle_admin
 * - i64 challenge_window_secs
 * - u64 proposer_bond_e6
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
  console.log('1024 Prediction Market Program - Initialize');
  console.log('='.repeat(60));
  
  // Connect to RPC (use config or env var)
  const connection = new Connection(config.RPC_URL, 'confirmed');
  console.log(`Connected to RPC: ${config.RPC_URL}`);
  
  // Load faucet keypair (check multiple paths)
  const keypairPaths = [
    process.env.ADMIN_KEYPAIR_PATH,
    process.env.HOME + '/Developer/1024ex/faucet.json',
    '/Users/patrick/Developer/1024ex/faucet.json',
    '/Users/chuciqin/Desktop/project1024/1024codebase/1024-chain/keys/faucet.json',
  ].filter(Boolean);
  
  let faucetData;
  let faucetPath;
  for (const path of keypairPaths) {
    try {
      faucetData = JSON.parse(fs.readFileSync(path, 'utf-8'));
      faucetPath = path;
      break;
    } catch (e) {
      continue;
    }
  }
  
  if (!faucetData) {
    throw new Error('Could not find faucet keypair in any known location');
  }
  console.log(`Loaded keypair from: ${faucetPath}`);
  const admin = Keypair.fromSecretKey(new Uint8Array(faucetData));
  console.log(`Admin: ${admin.publicKey.toBase58()}`);
  
  // Derive Config PDA
  const [configPda, configBump] = PublicKey.findProgramAddressSync(
    [PM_CONFIG_SEED],
    PROGRAM_ID
  );
  console.log(`Config PDA: ${configPda.toBase58()} (bump: ${configBump})`);
  
  // Check if already initialized
  const configAccount = await connection.getAccountInfo(configPda);
  if (configAccount) {
    console.log('\n⚠️  Program already initialized!');
    console.log(`Account size: ${configAccount.data.length} bytes`);
    console.log(`Owner: ${configAccount.owner.toBase58()}`);
    
    // Parse and display config
    const data = configAccount.data;
    console.log('\nConfig data (hex):');
    console.log(data.slice(0, 64).toString('hex'));
    return;
  }
  
  // Create Initialize instruction
  console.log('\nCreating Initialize instruction...');
  
  const instructionData = serializeInitializeArgs(
    admin.publicKey,        // oracle_admin = admin
    24 * 60 * 60,           // challenge_window = 24 hours
    100_000_000             // proposer_bond = 100 USDC (e6)
  );
  
  console.log(`Instruction data (${instructionData.length} bytes): ${instructionData.toString('hex')}`);
  
  const initializeIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: admin.publicKey, isSigner: true, isWritable: true },  // Admin
      { pubkey: configPda, isSigner: false, isWritable: true },       // Config PDA
      { pubkey: USDC_MINT, isSigner: false, isWritable: false },      // USDC Mint
      { pubkey: VAULT_PROGRAM, isSigner: false, isWritable: false },  // Vault Program
      { pubkey: FUND_PROGRAM, isSigner: false, isWritable: false },   // Fund Program
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }, // System
    ],
    data: instructionData,
  });
  
  // Send transaction
  console.log('\nSending transaction...');
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
  }
  
  // Verify
  console.log('\nVerifying...');
  const finalConfig = await connection.getAccountInfo(configPda);
  if (finalConfig) {
    console.log('✅ Config account created');
    console.log(`  Size: ${finalConfig.data.length} bytes`);
    console.log(`  Owner: ${finalConfig.owner.toBase58()}`);
  }
  
  console.log('\n' + '='.repeat(60));
}

main().catch(console.error);
