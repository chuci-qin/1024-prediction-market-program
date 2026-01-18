/**
 * Create prediction markets on 1024 Chain Testnet
 */

const { 
  Connection, 
  Keypair, 
  PublicKey, 
  Transaction, 
  TransactionInstruction,
  SystemProgram,
  sendAndConfirmTransaction,
  SYSVAR_RENT_PUBKEY,
} = require('@solana/web3.js');
const config = require('./config');
const crypto = require('crypto');
const fs = require('fs');

// 1024 Chain Testnet RPC
const RPC_URL = config.RPC_URL;

// Program IDs (from 1024 Chain)
const PROGRAM_ID = config.PROGRAM_ID;
const TOKEN_PROGRAM_ID = new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
const USDC_MINT = new PublicKey('7pCrfxhcAEyTFDhrhKRtRS2iMvEYx2dtNE7NzwuU7SA9');

// Seeds
const PM_CONFIG_SEED = Buffer.from('pm_config');
const MARKET_SEED = Buffer.from('market');
const YES_MINT_SEED = Buffer.from('yes_mint');
const NO_MINT_SEED = Buffer.from('no_mint');
const MARKET_VAULT_SEED = Buffer.from('market_vault');

const NEXT_MARKET_ID_OFFSET = 8 + 32 + 32 + 32 + 32 + 32; // = 168

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

async function createMarket(connection, admin, question, resolutionSpec, resolutionTime, finalizationDeadline) {
  const [configPda] = PublicKey.findProgramAddressSync([PM_CONFIG_SEED], PROGRAM_ID);
  
  const configAccount = await connection.getAccountInfo(configPda);
  if (!configAccount) {
    console.error('❌ Config not initialized!');
    return null;
  }
  
  const nextMarketId = configAccount.data.readBigUInt64LE(NEXT_MARKET_ID_OFFSET);
  console.log(`  Next Market ID: ${nextMarketId}`);
  
  const marketIdBytes = Buffer.alloc(8);
  marketIdBytes.writeBigUInt64LE(nextMarketId);
  
  const [marketPda] = PublicKey.findProgramAddressSync([MARKET_SEED, marketIdBytes], PROGRAM_ID);
  const [yesMintPda] = PublicKey.findProgramAddressSync([YES_MINT_SEED, marketIdBytes], PROGRAM_ID);
  const [noMintPda] = PublicKey.findProgramAddressSync([NO_MINT_SEED, marketIdBytes], PROGRAM_ID);
  const [vaultPda] = PublicKey.findProgramAddressSync([MARKET_VAULT_SEED, marketIdBytes], PROGRAM_ID);
  
  const questionHash = crypto.createHash('sha256').update(question).digest();
  const resolutionSpecHash = crypto.createHash('sha256').update(resolutionSpec).digest();
  const creatorFeeBps = 50;
  
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
  
  const tx = new Transaction().add(createMarketIx);
  
  try {
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.lastValidBlockHeight = lastValidBlockHeight;
    tx.feePayer = admin.publicKey;
    
    const signature = await sendAndConfirmTransaction(connection, tx, [admin], {
      commitment: 'confirmed',
    });
    
    console.log(`  ✅ Market created! ID: ${nextMarketId}, PDA: ${marketPda.toBase58()}`);
    console.log(`  Signature: ${signature}`);
    
    return {
      marketId: Number(nextMarketId),
      marketPda: marketPda.toBase58(),
      yesMint: yesMintPda.toBase58(),
      noMint: noMintPda.toBase58(),
      vault: vaultPda.toBase58(),
      questionHash: questionHash.toString('hex'),
      resolutionSpecHash: resolutionSpecHash.toString('hex'),
    };
  } catch (error) {
    console.error('  ❌ Transaction failed:', error.message);
    if (error.logs) error.logs.forEach(log => console.error('    ', log));
    return null;
  }
}

async function main() {
  console.log('='.repeat(60));
  console.log('1024 Prediction Market - Create Markets on Testnet');
  console.log('='.repeat(60));
  
  const connection = new Connection(RPC_URL, 'confirmed');
  console.log('Connected to:', RPC_URL);
  
  // Load faucet keypair
  const faucetPath = '/Users/chuciqin/Desktop/project1024/1024codebase/1024-chain/keys/faucet.json';
  const faucetData = JSON.parse(fs.readFileSync(faucetPath, 'utf-8'));
  const admin = Keypair.fromSecretKey(new Uint8Array(faucetData));
  console.log('Admin:', admin.publicKey.toBase58());
  
  const now = Math.floor(Date.now() / 1000);
  
  // Market 1: Binary - Ethereum 5K
  console.log('\n--- Creating Binary Market: Ethereum 5K ---');
  const binaryResult = await createMarket(
    connection,
    admin,
    'Will Ethereum reach $5K in 2025?',
    'Market resolves YES if ETH/USD price on Binance, Coinbase, or Kraken exceeds $5,000 at any point before resolution time.',
    Math.floor(new Date('2025-12-31T23:59:59Z').getTime() / 1000),
    Math.floor(new Date('2026-01-03T23:59:59Z').getTime() / 1000)
  );
  
  // Market 2: Binary - FIFA (we'll use binary for simplicity, multi-outcome needs different instruction)
  console.log('\n--- Creating Binary Market: FIFA World Cup ---');
  const fifaResult = await createMarket(
    connection,
    admin,
    'Will Spain win the 2026 FIFA World Cup?',
    'This market resolves YES if Spain wins the 2026 FIFA World Cup Final based on official FIFA announcement.',
    Math.floor(new Date('2026-07-19T23:59:59Z').getTime() / 1000),
    Math.floor(new Date('2026-07-22T23:59:59Z').getTime() / 1000)
  );
  
  console.log('\n=== Results ===');
  if (binaryResult) {
    console.log('Binary Market (ETH 5K):');
    console.log(JSON.stringify(binaryResult, null, 2));
  }
  if (fifaResult) {
    console.log('Binary Market (FIFA Spain):');
    console.log(JSON.stringify(fifaResult, null, 2));
  }
  
  console.log('\n⚠️  Now update the database with these values!');
}

main().catch(console.error);
