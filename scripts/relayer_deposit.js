/**
 * Relayer Deposit - 为用户入金到 Vault
 * 
 * Usage: node relayer_deposit.js <user_wallet> <amount_usdc>
 * Example: node relayer_deposit.js 9ocm9zv5F2QghKaFSLGSjkVg6f8XZf54nVTjfC2M3dG4 1000
 */

const { 
  Connection, 
  Keypair, 
  PublicKey, 
  Transaction, 
  TransactionInstruction,
  SystemProgram,
} = require('@solana/web3.js');
const fs = require('fs');

// Configuration
const RPC_URL = 'https://rpc-testnet.1024chain.com/rpc/';
const VAULT_PROGRAM_ID = new PublicKey('vR3BifKCa2TGKP2uhToxZAMYAYydqpesvKGX54gzFny');
const VAULT_CONFIG_PDA = new PublicKey('rMLrkwxV4uNLKmL2vmP3CJbYPbKamjZD4wjeKZsCy1g');

// RelayerDeposit instruction index
const RELAYER_DEPOSIT_IX = 20;

// UserAccount PDA seed
const USER_SEED = Buffer.from('user');

async function main() {
  console.log('='.repeat(60));
  console.log('💰 Relayer Deposit - Vault Program');
  console.log('='.repeat(60));
  
  // Parse args
  const userWalletArg = process.argv[2];
  const amountArg = process.argv[3];
  
  if (!userWalletArg || !amountArg) {
    console.log('\nUsage: node relayer_deposit.js <user_wallet> <amount_usdc>');
    console.log('Example: node relayer_deposit.js 9ocm9zv5F2QghKaFSLGSjkVg6f8XZf54nVTjfC2M3dG4 1000');
    process.exit(1);
  }
  
  let userWallet;
  try {
    userWallet = new PublicKey(userWalletArg);
  } catch (e) {
    console.error('Invalid wallet address:', userWalletArg);
    process.exit(1);
  }
  
  const amountUsdc = parseFloat(amountArg);
  if (isNaN(amountUsdc) || amountUsdc <= 0) {
    console.error('Invalid amount:', amountArg);
    process.exit(1);
  }
  
  console.log(`\nVault Program: ${VAULT_PROGRAM_ID.toBase58()}`);
  console.log(`VaultConfig: ${VAULT_CONFIG_PDA.toBase58()}`);
  console.log(`User Wallet: ${userWallet.toBase58()}`);
  console.log(`Amount: $${amountUsdc} USDC`);
  
  // Load admin keypair
  const adminPath = process.env.ADMIN_KEYPAIR || '/home/ubuntu/1024chain-testnet/keys/faucet.json';
  let admin;
  try {
    const keypairData = JSON.parse(fs.readFileSync(adminPath, 'utf8'));
    admin = Keypair.fromSecretKey(Uint8Array.from(keypairData));
    console.log(`Admin/Relayer: ${admin.publicKey.toBase58()}`);
  } catch (e) {
    console.error('Failed to load admin keypair from:', adminPath);
    process.exit(1);
  }
  
  const connection = new Connection(RPC_URL, {
    commitment: 'confirmed',
    confirmTransactionInitialTimeout: 60000,
  });
  
  // Derive UserAccount PDA
  const [userAccountPda, bump] = PublicKey.findProgramAddressSync(
    [USER_SEED, userWallet.toBuffer()],
    VAULT_PROGRAM_ID
  );
  console.log(`UserAccount PDA: ${userAccountPda.toBase58()}`);
  
  // Check if UserAccount exists
  const userAccountInfo = await connection.getAccountInfo(userAccountPda);
  if (userAccountInfo) {
    console.log('UserAccount exists, will add to balance');
  } else {
    console.log('UserAccount will be created');
  }
  
  // Convert amount to e6
  const amountE6 = BigInt(Math.floor(amountUsdc * 1_000_000));
  console.log(`Amount (e6): ${amountE6}`);
  
  // Build instruction
  // RelayerDeposit: instruction(1) + user_wallet(32) + amount(8)
  const buffer = Buffer.alloc(1 + 32 + 8);
  let offset = 0;
  buffer.writeUInt8(RELAYER_DEPOSIT_IX, offset);
  offset += 1;
  userWallet.toBuffer().copy(buffer, offset);
  offset += 32;
  buffer.writeBigUInt64LE(amountE6, offset);
  
  const ix = new TransactionInstruction({
    programId: VAULT_PROGRAM_ID,
    keys: [
      { pubkey: admin.publicKey, isSigner: true, isWritable: true },
      { pubkey: userAccountPda, isSigner: false, isWritable: true },
      { pubkey: VAULT_CONFIG_PDA, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: buffer,
  });
  
  const tx = new Transaction().add(ix);
  
  console.log('\n📤 Sending transaction...');
  
  try {
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.feePayer = admin.publicKey;
    
    // Sign and send
    tx.sign(admin);
    const signature = await connection.sendRawTransaction(tx.serialize(), {
      skipPreflight: false,
    });
    
    console.log(`Transaction sent: ${signature}`);
    console.log('Waiting for confirmation...');
    
    // Poll for confirmation
    let confirmed = false;
    for (let i = 0; i < 30; i++) {
      await new Promise(r => setTimeout(r, 1000));
      const status = await connection.getSignatureStatus(signature);
      if (status.value?.confirmationStatus === 'confirmed' || 
          status.value?.confirmationStatus === 'finalized') {
        confirmed = true;
        break;
      }
      if (status.value?.err) {
        throw new Error(`Transaction failed: ${JSON.stringify(status.value.err)}`);
      }
    }
    
    if (!confirmed) {
      throw new Error('Transaction confirmation timeout');
    }
    
    console.log(`\n✅ Deposit successful!`);
    console.log(`Signature: ${signature}`);
    console.log(`Explorer: https://testnet-scan.1024chain.com/tx/${signature}`);
    
    // Query updated balance
    console.log('\n📊 Querying updated balance...');
    const updatedAccount = await connection.getAccountInfo(userAccountPda);
    if (updatedAccount) {
      const data = updatedAccount.data;
      // UserAccount: discriminator(8) + wallet(32) + bump(1) + available_balance_e6(8) + ...
      const availableBalance = data.readBigInt64LE(41);
      const lockedMargin = data.readBigInt64LE(49);
      console.log(`Available balance: $${Number(availableBalance) / 1_000_000}`);
      console.log(`Locked margin: $${Number(lockedMargin) / 1_000_000}`);
    }
    
  } catch (error) {
    console.error('\n❌ Deposit failed:');
    if (error.logs) {
      console.log('Program logs:');
      error.logs.forEach(log => console.log('  ', log));
    }
    console.error(error.message || error);
    process.exit(1);
  }
}

main().catch(console.error);

