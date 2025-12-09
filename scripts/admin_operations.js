/**
 * Admin Operations - Test all admin instructions
 * Run on server: node admin_operations.js [operation]
 * 
 * Operations:
 *   set_paused     - Pause/unpause the program
 *   update_admin   - Update program admin
 *   update_oracle  - Update oracle admin
 */

const { 
  Connection, 
  Keypair, 
  PublicKey, 
  Transaction, 
  TransactionInstruction,
  sendAndConfirmTransaction,
} = require('@solana/web3.js');
const fs = require('fs');

const PROGRAM_ID = new PublicKey('FnwmQjmUkRTLA1G3i1CmFVE5cySzQGYZRezGAErdLizu');

// Seeds
const PM_CONFIG_SEED = Buffer.from('pm_config');

// Instruction indices based on enum order
// SetPaused, UpdateAdmin, UpdateOracleAdmin are typically at the end
// Let's check the instruction enum
const SET_PAUSED_IX = 19;  // Adjust based on actual enum position
const UPDATE_ADMIN_IX = 20;
const UPDATE_ORACLE_ADMIN_IX = 21;

async function testSetPaused(connection, admin, configPda, paused) {
  console.log(`\n>>> Test SetPaused (paused=${paused})`);
  
  // SetPaused: instruction(1) + bool(1)
  const buffer = Buffer.alloc(2);
  buffer.writeUInt8(SET_PAUSED_IX, 0);
  buffer.writeUInt8(paused ? 1 : 0, 1);
  
  const ix = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: admin.publicKey, isSigner: true, isWritable: true },
      { pubkey: configPda, isSigner: false, isWritable: true },
    ],
    data: buffer,
  });
  
  const tx = new Transaction().add(ix);
  
  try {
    const { blockhash } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.feePayer = admin.publicKey;
    
    const signature = await sendAndConfirmTransaction(connection, tx, [admin], {
      commitment: 'confirmed',
    });
    
    console.log(`✅ SetPaused(${paused}) successful!`);
    console.log(`Signature: ${signature}`);
    return true;
  } catch (error) {
    console.error(`❌ SetPaused failed:`);
    if (error.logs) {
      error.logs.forEach(log => console.error('  ', log));
    }
    console.error(error.message || error);
    return false;
  }
}

async function testUpdateAdmin(connection, admin, configPda, newAdmin) {
  console.log(`\n>>> Test UpdateAdmin`);
  console.log(`New Admin: ${newAdmin.toBase58()}`);
  
  // UpdateAdmin: instruction(1) + pubkey(32)
  const buffer = Buffer.alloc(1 + 32);
  buffer.writeUInt8(UPDATE_ADMIN_IX, 0);
  newAdmin.toBuffer().copy(buffer, 1);
  
  const ix = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: admin.publicKey, isSigner: true, isWritable: true },
      { pubkey: configPda, isSigner: false, isWritable: true },
    ],
    data: buffer,
  });
  
  const tx = new Transaction().add(ix);
  
  try {
    const { blockhash } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.feePayer = admin.publicKey;
    
    const signature = await sendAndConfirmTransaction(connection, tx, [admin], {
      commitment: 'confirmed',
    });
    
    console.log(`✅ UpdateAdmin successful!`);
    console.log(`Signature: ${signature}`);
    return true;
  } catch (error) {
    console.error(`❌ UpdateAdmin failed:`);
    if (error.logs) {
      error.logs.forEach(log => console.error('  ', log));
    }
    console.error(error.message || error);
    return false;
  }
}

async function testUpdateOracleAdmin(connection, admin, configPda, newOracleAdmin) {
  console.log(`\n>>> Test UpdateOracleAdmin`);
  console.log(`New Oracle Admin: ${newOracleAdmin.toBase58()}`);
  
  // UpdateOracleAdmin: instruction(1) + pubkey(32)
  const buffer = Buffer.alloc(1 + 32);
  buffer.writeUInt8(UPDATE_ORACLE_ADMIN_IX, 0);
  newOracleAdmin.toBuffer().copy(buffer, 1);
  
  const ix = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: admin.publicKey, isSigner: true, isWritable: true },
      { pubkey: configPda, isSigner: false, isWritable: true },
    ],
    data: buffer,
  });
  
  const tx = new Transaction().add(ix);
  
  try {
    const { blockhash } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.feePayer = admin.publicKey;
    
    const signature = await sendAndConfirmTransaction(connection, tx, [admin], {
      commitment: 'confirmed',
    });
    
    console.log(`✅ UpdateOracleAdmin successful!`);
    console.log(`Signature: ${signature}`);
    return true;
  } catch (error) {
    console.error(`❌ UpdateOracleAdmin failed:`);
    if (error.logs) {
      error.logs.forEach(log => console.error('  ', log));
    }
    console.error(error.message || error);
    return false;
  }
}

async function queryConfig(connection, configPda) {
  const account = await connection.getAccountInfo(configPda);
  if (!account) {
    console.log('Config not found!');
    return;
  }
  
  const data = account.data;
  // Parse config: discriminator(8) + admin(32) + usdc_mint(32) + vault_program(32) 
  //              + fund_program(32) + oracle_admin(32) + next_market_id(8) + ...
  const admin = new PublicKey(data.slice(8, 40));
  const oracleAdmin = new PublicKey(data.slice(136, 168));
  
  // is_paused is at a specific offset - let's find it
  // After all the base fields, check config structure
  // For now, let's print what we can
  console.log('\n=== Config Status ===');
  console.log(`Admin: ${admin.toBase58()}`);
  console.log(`Oracle Admin: ${oracleAdmin.toBase58()}`);
}

async function main() {
  const operation = process.argv[2] || 'query';
  
  console.log('='.repeat(60));
  console.log('1024 Prediction Market - Admin Operations');
  console.log('='.repeat(60));
  console.log(`Operation: ${operation}`);
  
  const connection = new Connection('http://127.0.0.1:8899', 'confirmed');
  
  const faucetPath = process.env.HOME + '/1024chain-testnet/keys/faucet.json';
  const faucetData = JSON.parse(fs.readFileSync(faucetPath, 'utf-8'));
  const admin = Keypair.fromSecretKey(new Uint8Array(faucetData));
  console.log(`\nAdmin: ${admin.publicKey.toBase58()}`);
  
  const [configPda] = PublicKey.findProgramAddressSync([PM_CONFIG_SEED], PROGRAM_ID);
  console.log(`Config PDA: ${configPda.toBase58()}`);
  
  // Query current config first
  await queryConfig(connection, configPda);
  
  switch (operation) {
    case 'pause':
      await testSetPaused(connection, admin, configPda, true);
      break;
    case 'unpause':
      await testSetPaused(connection, admin, configPda, false);
      break;
    case 'update_admin':
      // For testing, update to self (no change)
      await testUpdateAdmin(connection, admin, configPda, admin.publicKey);
      break;
    case 'update_oracle':
      // For testing, update to self (no change)
      await testUpdateOracleAdmin(connection, admin, configPda, admin.publicKey);
      break;
    case 'query':
    default:
      console.log('\nAvailable operations: pause, unpause, update_admin, update_oracle');
      break;
  }
  
  console.log('\n' + '='.repeat(60));
}

main().catch(console.error);
