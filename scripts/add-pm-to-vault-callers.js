/**
 * Add Prediction Market Program to Vault's authorized callers
 * 
 * This script adds the Prediction Market Program as an authorized CPI caller
 * in the Vault Program's VaultConfig.
 * 
 * Required: Admin keypair with Vault admin privileges
 * 
 * Usage: node add-pm-to-vault-callers.js
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
const path = require('path');
const borsh = require('borsh');

// ============================================================================
// Configuration from environment
// ============================================================================

// Load .env from 1024-core
const envPath = path.join(__dirname, '../1024-core/.env');
if (fs.existsSync(envPath)) {
  const envContent = fs.readFileSync(envPath, 'utf8');
  envContent.split('\n').forEach(line => {
    const [key, ...valueParts] = line.split('=');
    if (key && valueParts.length > 0) {
      const value = valueParts.join('=').trim();
      if (!process.env[key]) {
        process.env[key] = value;
      }
    }
  });
}

const VAULT_PROGRAM_ID = new PublicKey(process.env.VAULT_PROGRAM_ID || 'vR3BifKCa2TGKP2uhToxZAMYAYydqpesvKGX54gzFny');
const VAULT_CONFIG_PDA = new PublicKey(process.env.VAULT_CONFIG_PDA || 'rMLrkwxV4uNLKmL2vmP3CJbYPbKamjZD4wjeKZsCy1g');
const PM_PROGRAM_ID = new PublicKey(process.env.PM_PROGRAM_ID || '9hsG1DksmgadjjJTEEX7CdevQKYVkQag3mEratPRZXjv');
// PM Config PDA - this is what actually makes CPI calls to Vault
const [PM_CONFIG_PDA] = PublicKey.findProgramAddressSync([Buffer.from('pm_config')], PM_PROGRAM_ID);
const RPC_URL = process.env.SOLANA_RPC_URL || 'https://testnet-rpc.1024chain.com/rpc/';

// Vault instruction indices for authorized caller management
// Based on VaultInstruction enum:
// 0=Initialize, 1=InitializeUser, 2=Deposit, 3=Withdraw, 4=LockMargin,
// 5=ReleaseMargin, 6=ClosePositionSettle, 7=LiquidatePosition,
// 8=AddAuthorizedCaller, 9=RemoveAuthorizedCaller
const ADD_AUTHORIZED_CALLER_IX = 8;
const REMOVE_AUTHORIZED_CALLER_IX = 9;

async function main() {
  console.log('='.repeat(60));
  console.log('Add Prediction Market to Vault Authorized Callers');
  console.log('='.repeat(60));
  
  console.log(`\nVault Program:      ${VAULT_PROGRAM_ID.toBase58()}`);
  console.log(`Vault Config PDA:   ${VAULT_CONFIG_PDA.toBase58()}`);
  console.log(`PM Program:         ${PM_PROGRAM_ID.toBase58()}`);
  console.log(`PM Config PDA:      ${PM_CONFIG_PDA.toBase58()} (this is the actual CPI caller)`);
  console.log(`RPC:                ${RPC_URL}`);
  
  // Load admin keypair
  const adminKeypairPaths = [
    path.join(__dirname, '../1024-core/keys/faucet.json'),
    path.join(__dirname, '../faucet.json'),
    process.env.ADMIN_KEYPAIR_PATH,
  ].filter(Boolean);
  
  let adminKeypair = null;
  for (const kpPath of adminKeypairPaths) {
    try {
      if (fs.existsSync(kpPath)) {
        const keypairData = JSON.parse(fs.readFileSync(kpPath, 'utf8'));
        adminKeypair = Keypair.fromSecretKey(Uint8Array.from(keypairData));
        console.log(`Admin keypair:      ${adminKeypair.publicKey.toBase58()} (from ${kpPath})`);
        break;
      }
    } catch (e) {
      console.warn(`Failed to load keypair from ${kpPath}: ${e.message}`);
    }
  }
  
  if (!adminKeypair) {
    console.error('\n❌ Could not find admin keypair!');
    console.error('Tried paths:', adminKeypairPaths);
    process.exit(1);
  }
  
  const connection = new Connection(RPC_URL, {
    commitment: 'confirmed',
    confirmTransactionInitialTimeout: 120000,
    wsEndpoint: undefined,  // Disable WebSocket to avoid 405 errors
  });
  
  // Check VaultConfig exists and get admin
  console.log('\nChecking VaultConfig...');
  const vaultConfigAccount = await connection.getAccountInfo(VAULT_CONFIG_PDA);
  if (!vaultConfigAccount) {
    console.error('❌ VaultConfig account not found!');
    process.exit(1);
  }
  
  const configData = vaultConfigAccount.data;
  // VaultConfig layout: discriminator(8) + admin(32) + ...
  const configAdmin = new PublicKey(configData.slice(8, 40));
  console.log(`VaultConfig admin:  ${configAdmin.toBase58()}`);
  
  if (!configAdmin.equals(adminKeypair.publicKey)) {
    console.error(`\n❌ Admin mismatch!`);
    console.error(`Config admin:  ${configAdmin.toBase58()}`);
    console.error(`Your keypair:  ${adminKeypair.publicKey.toBase58()}`);
    process.exit(1);
  }
  
  console.log('✅ Admin verified!');
  
  // Check if PM is already in authorized_callers
  // VaultConfig layout:
  // discriminator(8) + admin(32) + usdc_mint(32) + vault_token_account(32) +
  // authorized_callers(10 * 32 = 320) + ledger_program(32) + fund_program(32) + ...
  const authorizedCallersOffset = 8 + 32 + 32 + 32;
  console.log('\nChecking existing authorized callers...');
  for (let i = 0; i < 10; i++) {
    const offset = authorizedCallersOffset + (i * 32);
    const caller = new PublicKey(configData.slice(offset, offset + 32));
    if (!caller.equals(PublicKey.default)) {
      console.log(`  Slot ${i}: ${caller.toBase58()}`);
      if (caller.equals(PM_PROGRAM_ID)) {
        console.log(`       ^ This is PM Program (NOT the CPI caller - should be replaced)`);
      }
      if (caller.equals(PM_CONFIG_PDA)) {
        console.log(`       ^ This is PM Config PDA (the actual CPI caller)`);
        console.log(`\n✅ PM Config PDA is ALREADY in authorized callers list!`);
        process.exit(0);
      }
    }
  }
  
  // Check if PM_PROGRAM_ID is in the list (which we need to remove to make space)
  let pmProgramSlot = -1;
  for (let i = 0; i < 10; i++) {
    const offset = authorizedCallersOffset + (i * 32);
    const caller = new PublicKey(configData.slice(offset, offset + 32));
    if (caller.equals(PM_PROGRAM_ID)) {
      pmProgramSlot = i;
      break;
    }
  }
  
  if (pmProgramSlot >= 0) {
    console.log(`\n⚠️  Found PM Program ID in slot ${pmProgramSlot} - will remove it first to make space`);
    console.log('   (PM Program ID is NOT the actual CPI caller, PM Config PDA is)');
    
    // Build RemoveAuthorizedCaller instruction for PM_PROGRAM_ID
    console.log('\nStep 1: Removing PM Program ID...');
    
    const removeData = Buffer.alloc(1 + 32);
    removeData.writeUInt8(REMOVE_AUTHORIZED_CALLER_IX, 0);
    PM_PROGRAM_ID.toBuffer().copy(removeData, 1);
    
    const removeIx = new TransactionInstruction({
      programId: VAULT_PROGRAM_ID,
      keys: [
        { pubkey: adminKeypair.publicKey, isSigner: true, isWritable: true },
        { pubkey: VAULT_CONFIG_PDA, isSigner: false, isWritable: true },
      ],
      data: removeData,
    });
    
    const removeTx = new Transaction().add(removeIx);
    
    try {
      const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
      removeTx.recentBlockhash = blockhash;
      removeTx.feePayer = adminKeypair.publicKey;
      removeTx.lastValidBlockHeight = lastValidBlockHeight;
      
      // Sign and send transaction with polling confirmation
      removeTx.sign(adminKeypair);
      const removeSig = await connection.sendRawTransaction(removeTx.serialize(), {
        skipPreflight: false,
        preflightCommitment: 'confirmed',
      });
      
      // Poll for confirmation instead of websocket
      console.log(`   Sent transaction: ${removeSig}`);
      console.log('   Waiting for confirmation (polling)...');
      let confirmed = false;
      for (let i = 0; i < 60; i++) {
        await new Promise(r => setTimeout(r, 1000));
        const status = await connection.getSignatureStatus(removeSig);
        if (status.value && status.value.confirmationStatus === 'confirmed') {
          confirmed = true;
          break;
        }
        if (status.value && status.value.err) {
          throw new Error(`Transaction failed: ${JSON.stringify(status.value.err)}`);
        }
      }
      if (!confirmed) {
        throw new Error('Transaction confirmation timeout');
      }
      
      console.log(`✅ Removed PM Program ID! Signature: ${removeSig}`);
    } catch (error) {
      console.error('❌ Failed to remove PM Program ID:', error.message);
      if (error.logs) {
        error.logs.forEach(log => console.error('  ', log));
      }
      process.exit(1);
    }
  }
  
  // Build AddAuthorizedCaller instruction
  // NOTE: We need to add PM_CONFIG_PDA, not PM_PROGRAM_ID!
  // The PM program uses pm_config PDA as the CPI signer when calling Vault
  console.log('\nStep 2: Adding PM Config PDA as authorized caller...');
  
  // Instruction data: [instruction_index(1), caller_pubkey(32)]
  const instructionData = Buffer.alloc(1 + 32);
  instructionData.writeUInt8(ADD_AUTHORIZED_CALLER_IX, 0);
  PM_CONFIG_PDA.toBuffer().copy(instructionData, 1);
  
  const ix = new TransactionInstruction({
    programId: VAULT_PROGRAM_ID,
    keys: [
      { pubkey: adminKeypair.publicKey, isSigner: true, isWritable: true },  // admin
      { pubkey: VAULT_CONFIG_PDA, isSigner: false, isWritable: true },       // vault_config
    ],
    data: instructionData,
  });
  
  const tx = new Transaction().add(ix);
  
  console.log('Sending transaction...');
  
  try {
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.feePayer = adminKeypair.publicKey;
    tx.lastValidBlockHeight = lastValidBlockHeight;
    
    // Sign and send transaction with polling confirmation
    tx.sign(adminKeypair);
    const signature = await connection.sendRawTransaction(tx.serialize(), {
      skipPreflight: false,
      preflightCommitment: 'confirmed',
    });
    
    // Poll for confirmation instead of websocket
    console.log(`   Sent transaction: ${signature}`);
    console.log('   Waiting for confirmation (polling)...');
    let confirmed = false;
    for (let i = 0; i < 60; i++) {
      await new Promise(r => setTimeout(r, 1000));
      const status = await connection.getSignatureStatus(signature);
      if (status.value && status.value.confirmationStatus === 'confirmed') {
        confirmed = true;
        break;
      }
      if (status.value && status.value.err) {
        throw new Error(`Transaction failed: ${JSON.stringify(status.value.err)}`);
      }
    }
    if (!confirmed) {
      throw new Error('Transaction confirmation timeout');
    }
    
    console.log(`\n✅ Authorized caller added successfully!`);
    console.log(`Signature: ${signature}`);
    console.log(`Explorer:  https://testnet-scan.1024chain.com/tx/${signature}`);
    
    // Verify
    console.log('\nVerifying...');
    const updatedConfig = await connection.getAccountInfo(VAULT_CONFIG_PDA);
    const updatedData = updatedConfig.data;
    
    let found = false;
    for (let i = 0; i < 10; i++) {
      const offset = authorizedCallersOffset + (i * 32);
      const caller = new PublicKey(updatedData.slice(offset, offset + 32));
      if (caller.equals(PM_CONFIG_PDA)) {
        console.log(`✅ PM Config PDA found in slot ${i}!`);
        found = true;
        break;
      }
    }
    
    if (!found) {
      console.log('⚠️  Warning: PM Program not found in list after update');
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
  console.log('Next steps:');
  console.log('1. Run the prediction market tests again');
  console.log('2. The CPI calls should now succeed');
  console.log('='.repeat(60));
}

main().catch(console.error);

