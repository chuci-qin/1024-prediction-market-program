/**
 * Create Market 11 on 1024Chain Testnet
 * Handles fast block times (4x speed)
 */

const { 
  Connection, 
  Keypair, 
  PublicKey, 
  Transaction, 
  TransactionInstruction,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
} = require('@solana/web3.js');
const crypto = require('crypto');

const RPC_URL = 'https://testnet-rpc.1024chain.com/rpc/';
const PROGRAM_ID = new PublicKey('FVtPQkdYvSNdpTA6QXYRcTBhDGgnufw2Enqmo2tQKr58');
const TOKEN_PROGRAM_ID = new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
const USDC_MINT = new PublicKey('7pCrfxhcAEyTFDhrhKRtRS2iMvEYx2dtNE7NzwuU7SA9');

const PM_CONFIG_SEED = Buffer.from('pm_config');
const MARKET_SEED = Buffer.from('market');
const YES_MINT_SEED = Buffer.from('yes_mint');
const NO_MINT_SEED = Buffer.from('no_mint');
const MARKET_VAULT_SEED = Buffer.from('market_vault');

const RELAYER_KEYPAIR = [9,201,67,159,134,166,247,250,175,67,60,55,49,132,104,141,207,35,62,44,129,223,128,15,8,206,189,184,216,157,244,27,16,42,227,1,241,96,112,131,253,96,7,205,80,14,207,215,38,236,183,121,99,16,116,102,82,186,3,234,3,4,107,113];

function serializeCreateMarketArgs(questionHash, resolutionSpecHash, resolutionTime, finalizationDeadline, creatorFeeBps) {
  const buffer = Buffer.alloc(1 + 32 + 32 + 8 + 8 + 2);
  let offset = 0;
  buffer.writeUInt8(1, offset); offset += 1;
  questionHash.copy(buffer, offset); offset += 32;
  resolutionSpecHash.copy(buffer, offset); offset += 32;
  buffer.writeBigInt64LE(BigInt(resolutionTime), offset); offset += 8;
  buffer.writeBigInt64LE(BigInt(finalizationDeadline), offset); offset += 8;
  buffer.writeUInt16LE(creatorFeeBps, offset);
  return buffer;
}

async function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function main() {
  console.log('='.repeat(60));
  console.log('Creating Market 11 on 1024Chain Testnet');
  console.log('='.repeat(60));
  
  const connection = new Connection(RPC_URL, {
    commitment: 'confirmed',
    confirmTransactionInitialTimeout: 5000, // 5 seconds for fast chain
  });
  
  const admin = Keypair.fromSecretKey(new Uint8Array(RELAYER_KEYPAIR));
  console.log('Admin:', admin.publicKey.toBase58());
  
  const [configPda] = PublicKey.findProgramAddressSync([PM_CONFIG_SEED], PROGRAM_ID);
  
  // Market 11
  const marketId = 11n;
  const marketIdBytes = Buffer.alloc(8);
  marketIdBytes.writeBigUInt64LE(marketId);
  
  const [marketPda] = PublicKey.findProgramAddressSync([MARKET_SEED, marketIdBytes], PROGRAM_ID);
  const [yesMintPda] = PublicKey.findProgramAddressSync([YES_MINT_SEED, marketIdBytes], PROGRAM_ID);
  const [noMintPda] = PublicKey.findProgramAddressSync([NO_MINT_SEED, marketIdBytes], PROGRAM_ID);
  const [vaultPda] = PublicKey.findProgramAddressSync([MARKET_VAULT_SEED, marketIdBytes], PROGRAM_ID);
  
  console.log('\nMarket PDAs:');
  console.log('  Market:', marketPda.toBase58());
  console.log('  YES Mint:', yesMintPda.toBase58());
  console.log('  NO Mint:', noMintPda.toBase58());
  console.log('  Vault:', vaultPda.toBase58());
  
  // Check if market already exists
  const existingMarket = await connection.getAccountInfo(marketPda);
  if (existingMarket) {
    console.log('\n✅ Market 11 already exists!');
    return;
  }
  
  // Market: Will Ethereum reach $5K in 2025?
  const question = 'Will Ethereum reach $5K in 2025?';
  const resolutionSpec = 'This market resolves YES if ETH price exceeds $5,000 USD on major exchanges before December 31, 2025 23:59 UTC.';
  
  const questionHash = crypto.createHash('sha256').update(question).digest();
  const resolutionSpecHash = crypto.createHash('sha256').update(resolutionSpec).digest();
  
  const now = Math.floor(Date.now() / 1000);
  const resolutionTime = now + (30 * 24 * 60 * 60); // 30 days
  const finalizationDeadline = resolutionTime + (3 * 24 * 60 * 60); // +3 days
  const creatorFeeBps = 50;
  
  console.log('\nMarket Details:');
  console.log('  Question:', question);
  console.log('  Resolution:', new Date(resolutionTime * 1000).toISOString());
  
  const instructionData = serializeCreateMarketArgs(
    questionHash, resolutionSpecHash, resolutionTime, finalizationDeadline, creatorFeeBps
  );
  
  const createMarketIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: admin.publicKey, isSigner: true, isWritable: true },
      { pubkey: configPda, isSigner: false, isWritable: true },
      { pubkey: marketPda, isSigner: false, isWritable: true },
      { pubkey: yesMintPda, isSigner: false, isWritable: true },
      { pubkey: noMintPda, isSigner: false, isWritable: true },
      { pubkey: vaultPda, isSigner: false, isWritable: true },
      { pubkey: USDC_MINT, isSigner: false, isWritable: false },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
    ],
    data: instructionData,
  });
  
  console.log('\n📤 Sending transaction (skipPreflight for fast chain)...');
  
  try {
    // Get fresh blockhash right before sending
    const { blockhash } = await connection.getLatestBlockhash('finalized');
    
    const tx = new Transaction();
    tx.add(createMarketIx);
    tx.recentBlockhash = blockhash;
    tx.feePayer = admin.publicKey;
    tx.sign(admin);
    
    // Send with skipPreflight to avoid simulation delays
    const signature = await connection.sendRawTransaction(tx.serialize(), {
      skipPreflight: true,
      maxRetries: 3,
    });
    
    console.log('Transaction sent:', signature);
    console.log('Explorer: https://testnet-scan.1024chain.com/tx/' + signature);
    
    // Poll for confirmation instead of using confirmTransaction
    console.log('\n⏳ Waiting for confirmation...');
    for (let i = 0; i < 30; i++) {
      await sleep(500);
      const status = await connection.getSignatureStatus(signature);
      if (status.value?.confirmationStatus === 'confirmed' || status.value?.confirmationStatus === 'finalized') {
        if (status.value.err) {
          console.error('❌ Transaction failed:', status.value.err);
          return;
        }
        console.log('\n✅ CreateMarket successful!');
        console.log('Market ID: 11');
        console.log('Market PDA:', marketPda.toBase58());
        console.log('YES Mint:', yesMintPda.toBase58());
        console.log('NO Mint:', noMintPda.toBase58());
        return;
      }
      process.stdout.write('.');
    }
    
    // Verify by checking account
    console.log('\n\n🔍 Verifying market account...');
    const marketAccount = await connection.getAccountInfo(marketPda);
    if (marketAccount) {
      console.log('✅ Market 11 created successfully!');
      console.log('  Size:', marketAccount.data.length, 'bytes');
    } else {
      console.log('⚠️  Market account not found, transaction may still be processing');
    }
    
  } catch (error) {
    console.error('\n❌ Error:', error.message);
    if (error.logs) {
      console.error('Logs:');
      error.logs.forEach(log => console.error('  ', log));
    }
  }
  
  console.log('\n' + '='.repeat(60));
}

main().catch(console.error);

