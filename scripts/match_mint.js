/**
 * MatchMint - Match YES buy + NO buy orders to mint complete set
 * Run on server: node match_mint.js [market_id] [yes_order_id] [no_order_id] [yes_price] [no_price]
 * 
 * Requires: yes_price + no_price ≈ 1.0 (within tolerance)
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
const { TOKEN_PROGRAM_ID, getAssociatedTokenAddress } = require('@solana/spl-token');
const fs = require('fs');

const PROGRAM_ID = config.PROGRAM_ID;
const USDC_MINT = new PublicKey('7pCrfxhcAEyTFDhrhKRtRS2iMvEYx2dtNE7NzwuU7SA9');

// Seeds
const PM_CONFIG_SEED = Buffer.from('pm_config');
const MARKET_SEED = Buffer.from('market');
const ORDER_SEED = Buffer.from('order');
const POSITION_SEED = Buffer.from('position');

// Instruction index (count from 0 in the enum: Initialize=0, CreateMarket=1, ... MatchMint=11)
const MATCH_MINT_IX = 11;

const PRICE_PRECISION = 1_000_000;

/**
 * Serialize MatchMintArgs
 * Layout:
 * - u8 instruction (12)
 * - u64 market_id
 * - u64 yes_order_id
 * - u64 no_order_id
 * - u64 amount
 * - u64 yes_price (e6)
 * - u64 no_price (e6)
 */
function serializeMatchMintArgs(marketId, yesOrderId, noOrderId, amount, yesPrice, noPrice) {
  const buffer = Buffer.alloc(1 + 8 + 8 + 8 + 8 + 8 + 8);
  let offset = 0;
  
  buffer.writeUInt8(MATCH_MINT_IX, offset); offset += 1;
  buffer.writeBigUInt64LE(BigInt(marketId), offset); offset += 8;
  buffer.writeBigUInt64LE(BigInt(yesOrderId), offset); offset += 8;
  buffer.writeBigUInt64LE(BigInt(noOrderId), offset); offset += 8;
  buffer.writeBigUInt64LE(BigInt(amount), offset); offset += 8;
  buffer.writeBigUInt64LE(BigInt(yesPrice), offset); offset += 8;
  buffer.writeBigUInt64LE(BigInt(noPrice), offset);
  
  return buffer;
}

async function main() {
  const marketId = process.argv[2] ? parseInt(process.argv[2]) : 1;
  const yesOrderId = process.argv[3] ? parseInt(process.argv[3]) : 1;
  const noOrderId = process.argv[4] ? parseInt(process.argv[4]) : 2;
  const amount = process.argv[5] ? parseInt(process.argv[5]) : 10_000_000; // 10 tokens
  const yesPriceArg = process.argv[6] ? parseFloat(process.argv[6]) : 0.60;
  const noPriceArg = process.argv[7] ? parseFloat(process.argv[7]) : 0.40;
  
  const yesPrice = Math.floor(yesPriceArg * PRICE_PRECISION);
  const noPrice = Math.floor(noPriceArg * PRICE_PRECISION);
  
  console.log('='.repeat(60));
  console.log('1024 Prediction Market - MatchMint');
  console.log('='.repeat(60));
  console.log(`Market ID: ${marketId}`);
  console.log(`YES Order ID: ${yesOrderId}, Price: $${yesPriceArg}`);
  console.log(`NO Order ID: ${noOrderId}, Price: $${noPriceArg}`);
  console.log(`Amount: ${amount / 1_000_000} tokens`);
  console.log(`Total Price: $${yesPriceArg + noPriceArg}`);
  
  if (Math.abs(yesPriceArg + noPriceArg - 1.0) > 0.01) {
    console.log('\n⚠️  Warning: Prices do not sum to 1.0');
  }
  
  const connection = new Connection(config.RPC_URL, 'confirmed');
  
  const faucetPath = '/Users/patrick/Developer/1024ex/faucet.json';
  const faucetData = JSON.parse(fs.readFileSync(faucetPath, 'utf-8'));
  const caller = Keypair.fromSecretKey(new Uint8Array(faucetData));
  console.log(`\nCaller: ${caller.publicKey.toBase58()}`);
  
  // Derive PDAs
  const [configPda] = PublicKey.findProgramAddressSync([PM_CONFIG_SEED], PROGRAM_ID);
  
  const marketIdBytes = Buffer.alloc(8);
  marketIdBytes.writeBigUInt64LE(BigInt(marketId));
  const [marketPda] = PublicKey.findProgramAddressSync([MARKET_SEED, marketIdBytes], PROGRAM_ID);
  
  // Get market info
  const marketAccount = await connection.getAccountInfo(marketPda);
  if (!marketAccount) {
    console.error('❌ Market not found!');
    return;
  }
  
  const marketData = marketAccount.data;
  const yesMint = new PublicKey(marketData.slice(112, 144));
  const noMint = new PublicKey(marketData.slice(144, 176));
  const marketVault = new PublicKey(marketData.slice(176, 208));
  
  console.log(`\nMarket Info:`);
  console.log(`  YES Mint: ${yesMint.toBase58()}`);
  console.log(`  NO Mint: ${noMint.toBase58()}`);
  console.log(`  Vault: ${marketVault.toBase58()}`);
  
  // Derive order PDAs
  const yesOrderIdBytes = Buffer.alloc(8);
  yesOrderIdBytes.writeBigUInt64LE(BigInt(yesOrderId));
  const [yesOrderPda] = PublicKey.findProgramAddressSync(
    [ORDER_SEED, marketIdBytes, yesOrderIdBytes],
    PROGRAM_ID
  );
  
  const noOrderIdBytes = Buffer.alloc(8);
  noOrderIdBytes.writeBigUInt64LE(BigInt(noOrderId));
  const [noOrderPda] = PublicKey.findProgramAddressSync(
    [ORDER_SEED, marketIdBytes, noOrderIdBytes],
    PROGRAM_ID
  );
  
  console.log(`  YES Order PDA: ${yesOrderPda.toBase58()}`);
  console.log(`  NO Order PDA: ${noOrderPda.toBase58()}`);
  
  // Get order owners from order accounts
  const yesOrderAccount = await connection.getAccountInfo(yesOrderPda);
  const noOrderAccount = await connection.getAccountInfo(noOrderPda);
  
  if (!yesOrderAccount || !noOrderAccount) {
    console.error('❌ One or both orders not found!');
    return;
  }
  
  // Parse order owners (offset 24 after discriminator + order_id + market_id)
  const yesOrderOwner = new PublicKey(yesOrderAccount.data.slice(24, 56));
  const noOrderOwner = new PublicKey(noOrderAccount.data.slice(24, 56));
  
  console.log(`  YES Order Owner: ${yesOrderOwner.toBase58()}`);
  console.log(`  NO Order Owner: ${noOrderOwner.toBase58()}`);
  
  // Get token accounts
  const yesOwnerYesAta = await getAssociatedTokenAddress(yesMint, yesOrderOwner);
  const noOwnerNoAta = await getAssociatedTokenAddress(noMint, noOrderOwner);
  
  // Derive position PDAs
  const [yesOwnerPositionPda] = PublicKey.findProgramAddressSync(
    [POSITION_SEED, marketIdBytes, yesOrderOwner.toBuffer()],
    PROGRAM_ID
  );
  const [noOwnerPositionPda] = PublicKey.findProgramAddressSync(
    [POSITION_SEED, marketIdBytes, noOrderOwner.toBuffer()],
    PROGRAM_ID
  );
  
  console.log(`  YES Owner Position: ${yesOwnerPositionPda.toBase58()}`);
  console.log(`  NO Owner Position: ${noOwnerPositionPda.toBase58()}`);
  
  // Serialize instruction
  const instructionData = serializeMatchMintArgs(marketId, yesOrderId, noOrderId, amount, yesPrice, noPrice);
  console.log(`\nInstruction data: ${instructionData.toString('hex')}`);
  
  /**
   * Accounts for MatchMint (from processor.rs):
   * 0. [signer] Relayer/Keeper
   * 1. [writable] Config
   * 2. [writable] Market
   * 3. [writable] YES Order
   * 4. [writable] NO Order
   * 5. [writable] YES Mint
   * 6. [writable] NO Mint
   * 7. [writable] YES Buyer's YES Token Account
   * 8. [writable] NO Buyer's NO Token Account
   * 9. [] Token Program
   */
  const matchMintIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: caller.publicKey, isSigner: true, isWritable: true },
      { pubkey: configPda, isSigner: false, isWritable: true },
      { pubkey: marketPda, isSigner: false, isWritable: true },
      { pubkey: yesOrderPda, isSigner: false, isWritable: true },
      { pubkey: noOrderPda, isSigner: false, isWritable: true },
      { pubkey: yesMint, isSigner: false, isWritable: true },
      { pubkey: noMint, isSigner: false, isWritable: true },
      { pubkey: yesOwnerYesAta, isSigner: false, isWritable: true },
      { pubkey: noOwnerNoAta, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    data: instructionData,
  });
  
  const tx = new Transaction().add(matchMintIx);
  
  console.log('\nSending MatchMint transaction...');
  
  try {
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.lastValidBlockHeight = lastValidBlockHeight;
    tx.feePayer = caller.publicKey;
    
    const signature = await sendAndConfirmTransaction(connection, tx, [caller], {
      commitment: 'confirmed',
    });
    
    console.log('\n✅ MatchMint successful!');
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
  
  console.log('\n' + '='.repeat(60));
}

main().catch(console.error);
