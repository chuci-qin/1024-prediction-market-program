/**
 * Create a market with past resolution time for Oracle testing
 */

const { 
  Connection, 
  Keypair, 
  PublicKey, 
  Transaction, 
  TransactionInstruction,
  sendAndConfirmTransaction,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
} = require('@solana/web3.js');
const config = require('./config');
const { TOKEN_PROGRAM_ID } = require('@solana/spl-token');
const fs = require('fs');
const crypto = require('crypto');

const PROGRAM_ID = config.PROGRAM_ID;
const USDC_MINT = new PublicKey('7pCrfxhcAEyTFDhrhKRtRS2iMvEYx2dtNE7NzwuU7SA9');

// Seeds
const PM_CONFIG_SEED = Buffer.from('pm_config');
const MARKET_SEED = Buffer.from('market');
const YES_MINT_SEED = Buffer.from('yes_mint');
const NO_MINT_SEED = Buffer.from('no_mint');
const VAULT_SEED = Buffer.from('market_vault');

const CREATE_MARKET_IX = 1;

function serializeCreateMarketArgs(resolutionTime, finalizationDeadline, creatorFeeBps) {
  // QuestionHash: 32 bytes
  const questionHash = crypto.createHash('sha256')
    .update('Oracle Test Market - Resolvable Now')
    .digest();
  
  // ResolutionSpecHash: 32 bytes
  const resolutionSpecHash = crypto.createHash('sha256')
    .update('Immediate resolution test')
    .digest();
  
  const buffer = Buffer.alloc(1 + 32 + 32 + 8 + 8 + 2);
  let offset = 0;
  
  buffer.writeUInt8(CREATE_MARKET_IX, offset); offset += 1;
  questionHash.copy(buffer, offset); offset += 32;
  resolutionSpecHash.copy(buffer, offset); offset += 32;
  buffer.writeBigInt64LE(BigInt(resolutionTime), offset); offset += 8;
  buffer.writeBigInt64LE(BigInt(finalizationDeadline), offset); offset += 8;
  buffer.writeUInt16LE(creatorFeeBps, offset);
  
  return buffer;
}

async function main() {
  console.log('='.repeat(60));
  console.log('1024 Prediction Market - Create Market for Oracle Test');
  console.log('='.repeat(60));
  
  const connection = new Connection(config.RPC_URL, 'confirmed');
  
  const faucetPath = '/Users/patrick/Developer/1024ex/faucet.json';
  const faucetData = JSON.parse(fs.readFileSync(faucetPath, 'utf-8'));
  const creator = Keypair.fromSecretKey(new Uint8Array(faucetData));
  console.log(`Creator: ${creator.publicKey.toBase58()}`);
  
  // Get config to find next market ID
  // Config layout: discriminator(8) + admin(32) + usdc_mint(32) + vault_program(32) 
  //              + fund_program(32) + oracle_admin(32) + next_market_id(8) = offset 168
  const [configPda] = PublicKey.findProgramAddressSync([PM_CONFIG_SEED], PROGRAM_ID);
  const configAccount = await connection.getAccountInfo(configPda);
  const nextMarketId = Number(configAccount.data.readBigUInt64LE(168));
  console.log(`Next Market ID: ${nextMarketId}`);
  
  // Get blockchain slot time
  const slot = await connection.getSlot();
  const blockTime = await connection.getBlockTime(slot);
  
  // Set resolution time to 2 minutes from now (soon resolvable)
  const resolutionTime = blockTime + 120; // 2 minutes from now
  const finalizationDeadline = blockTime + 3600 * 24 * 3; // 3 days from now
  const creatorFeeBps = 100; // 1%
  
  console.log(`\nMarket Settings:`);
  console.log(`  Resolution Time: ${new Date(resolutionTime * 1000).toISOString()} (PAST)`);
  console.log(`  Finalization: ${new Date(finalizationDeadline * 1000).toISOString()}`);
  
  // Derive PDAs
  const marketIdBytes = Buffer.alloc(8);
  marketIdBytes.writeBigUInt64LE(BigInt(nextMarketId));
  
  const [marketPda] = PublicKey.findProgramAddressSync([MARKET_SEED, marketIdBytes], PROGRAM_ID);
  const [yesMintPda] = PublicKey.findProgramAddressSync([YES_MINT_SEED, marketIdBytes], PROGRAM_ID);
  const [noMintPda] = PublicKey.findProgramAddressSync([NO_MINT_SEED, marketIdBytes], PROGRAM_ID);
  const [vaultPda] = PublicKey.findProgramAddressSync([VAULT_SEED, marketIdBytes], PROGRAM_ID);
  
  console.log(`\nPDAs:`);
  console.log(`  Market: ${marketPda.toBase58()}`);
  console.log(`  YES Mint: ${yesMintPda.toBase58()}`);
  console.log(`  NO Mint: ${noMintPda.toBase58()}`);
  console.log(`  Vault: ${vaultPda.toBase58()}`);
  
  const instructionData = serializeCreateMarketArgs(resolutionTime, finalizationDeadline, creatorFeeBps);
  
  const createMarketIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: creator.publicKey, isSigner: true, isWritable: true },
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
  
  console.log('\nSending CreateMarket transaction...');
  
  try {
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.lastValidBlockHeight = lastValidBlockHeight;
    tx.feePayer = creator.publicKey;
    
    const signature = await sendAndConfirmTransaction(connection, tx, [creator], {
      commitment: 'confirmed',
    });
    
    console.log('\n✅ CreateMarket successful!');
    console.log(`Market ID: ${nextMarketId}`);
    console.log(`Signature: ${signature}`);
    
  } catch (error) {
    console.error('\n❌ Transaction failed:');
    if (error.logs) {
      error.logs.forEach(log => console.error('  ', log));
    }
    console.error(error.message || error);
  }
  
  console.log('\n' + '='.repeat(60));
}

main().catch(console.error);
