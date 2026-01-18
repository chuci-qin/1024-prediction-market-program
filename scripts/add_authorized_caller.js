/**
 * Add Authorized Caller - Add a matching engine relayer to the authorized callers list
 * 
 * Usage: node add_authorized_caller.js <caller_pubkey>
 * 
 * Run on server: node add_authorized_caller.js <caller_pubkey>
 */

const { 
  Connection, 
  Keypair, 
  PublicKey, 
  Transaction, 
  TransactionInstruction,
  sendAndConfirmTransaction,
  SystemProgram,
} = require('@solana/web3.js');
const config = require('./config');
const fs = require('fs');

// Configuration
const PROGRAM_ID = config.PROGRAM_ID;
const RPC_URL = config.RPC_URL;

// Seeds
const PM_CONFIG_SEED = Buffer.from('pm_config');
const AUTHORIZED_CALLERS_SEED = Buffer.from('authorized_callers');

// Instruction index
const ADD_AUTHORIZED_CALLER_IX = 24;

async function main() {
  console.log('='.repeat(60));
  console.log('1024 Prediction Market - Add Authorized Caller');
  console.log('='.repeat(60));
  
  // Parse command line args
  const callerPubkeyArg = process.argv[2];
  if (!callerPubkeyArg) {
    console.error('Usage: node add_authorized_caller.js <caller_pubkey>');
    console.error('Example: node add_authorized_caller.js 267TEwwHkJUHz42TLNggDCecNhYHFxcRALmR17bPkvU8');
    process.exit(1);
  }
  
  let callerPubkey;
  try {
    callerPubkey = new PublicKey(callerPubkeyArg);
  } catch (e) {
    console.error(`Invalid pubkey: ${callerPubkeyArg}`);
    process.exit(1);
  }
  
  console.log(`\nProgram ID: ${PROGRAM_ID.toBase58()}`);
  console.log(`RPC: ${RPC_URL}`);
  console.log(`Caller to add: ${callerPubkey.toBase58()}`);
  
  // Load admin keypair
  const adminKeypairPath = process.env.ADMIN_KEYPAIR || '/home/ubuntu/1024chain-testnet/keys/faucet.json';
  let adminKeypair;
  try {
    const keypairData = JSON.parse(fs.readFileSync(adminKeypairPath, 'utf8'));
    adminKeypair = Keypair.fromSecretKey(Uint8Array.from(keypairData));
    console.log(`Admin: ${adminKeypair.publicKey.toBase58()}`);
  } catch (e) {
    console.error(`Failed to load admin keypair from ${adminKeypairPath}: ${e.message}`);
    process.exit(1);
  }
  
  const connection = new Connection(RPC_URL, {
    commitment: 'confirmed',
    confirmTransactionInitialTimeout: 60000,
  });
  
  // Derive PDAs
  const [configPda] = PublicKey.findProgramAddressSync([PM_CONFIG_SEED], PROGRAM_ID);
  const [callersPda] = PublicKey.findProgramAddressSync([AUTHORIZED_CALLERS_SEED], PROGRAM_ID);
  
  console.log(`\nConfig PDA: ${configPda.toBase58()}`);
  console.log(`AuthorizedCallers PDA: ${callersPda.toBase58()}`);
  
  // Check if config exists
  const configAccount = await connection.getAccountInfo(configPda);
  if (!configAccount) {
    console.error('\n❌ Config not initialized! Run init_program.js first.');
    process.exit(1);
  }
  
  // Check admin
  const configData = configAccount.data;
  const configAdmin = new PublicKey(configData.slice(8, 40));
  if (!configAdmin.equals(adminKeypair.publicKey)) {
    console.error(`\n❌ Admin mismatch!`);
    console.error(`Config admin: ${configAdmin.toBase58()}`);
    console.error(`Your keypair: ${adminKeypair.publicKey.toBase58()}`);
    process.exit(1);
  }
  
  // Check existing callers
  const callersAccount = await connection.getAccountInfo(callersPda);
  if (callersAccount) {
    console.log(`\nExisting AuthorizedCallers account found (${callersAccount.data.length} bytes)`);
    
    // Parse existing callers
    const callersData = callersAccount.data;
    const count = callersData[8]; // discriminator(8) + count(1)
    console.log(`Current caller count: ${count}`);
    
    // Check if caller already exists
    for (let i = 0; i < count; i++) {
      const offset = 16 + (i * 32); // discriminator(8) + count(8) + i*32
      const existingCaller = new PublicKey(callersData.slice(offset, offset + 32));
      if (existingCaller.equals(callerPubkey)) {
        console.log(`\n⚠️  Caller ${callerPubkey.toBase58()} is already authorized!`);
        process.exit(0);
      }
    }
  } else {
    console.log('\nAuthorizedCallers account will be created.');
  }
  
  // Build instruction
  // AddAuthorizedCallerArgs: caller (32 bytes)
  const buffer = Buffer.alloc(1 + 32);
  buffer.writeUInt8(ADD_AUTHORIZED_CALLER_IX, 0);
  callerPubkey.toBuffer().copy(buffer, 1);
  
  const ix = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: adminKeypair.publicKey, isSigner: true, isWritable: true },
      { pubkey: configPda, isSigner: false, isWritable: true },
      { pubkey: callersPda, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: buffer,
  });
  
  const tx = new Transaction().add(ix);
  
  console.log('\nSending transaction...');
  
  try {
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.feePayer = adminKeypair.publicKey;
    tx.lastValidBlockHeight = lastValidBlockHeight;
    
    const signature = await sendAndConfirmTransaction(connection, tx, [adminKeypair], {
      commitment: 'confirmed',
      maxRetries: 3,
    });
    
    console.log(`\n✅ Authorized caller added successfully!`);
    console.log(`Signature: ${signature}`);
    console.log(`Explorer: https://testnet-scan.1024chain.com/tx/${signature}`);
    
    // Verify
    console.log('\nVerifying...');
    const updatedCallersAccount = await connection.getAccountInfo(callersPda);
    if (updatedCallersAccount) {
      const callersData = updatedCallersAccount.data;
      const count = callersData[8];
      console.log(`Caller count after: ${count}`);
      
      for (let i = 0; i < count; i++) {
        const offset = 16 + (i * 32);
        const caller = new PublicKey(callersData.slice(offset, offset + 32));
        console.log(`  Caller ${i + 1}: ${caller.toBase58()}`);
      }
    }
    
  } catch (error) {
    console.error(`\n❌ Transaction failed:`);
    if (error.logs) {
      console.error('Program logs:');
      error.logs.forEach(log => console.error('  ', log));
    }
    console.error(error.message || error);
    process.exit(1);
  }
  
  console.log('\n' + '='.repeat(60));
}

main().catch(console.error);

