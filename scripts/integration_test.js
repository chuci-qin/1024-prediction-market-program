/**
 * Integration Test - Complete trading flow test
 * Run on server: node integration_test.js
 * 
 * Flow:
 * 1. Create new market
 * 2. Activate market
 * 3. Mint complete set
 * 4. Place orders (buy/sell)
 * 5. Match orders
 * 6. Redeem complete set
 * 7. Query final status
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
const { TOKEN_PROGRAM_ID, getAssociatedTokenAddress, createAssociatedTokenAccountInstruction } = require('@solana/spl-token');
const fs = require('fs');
const crypto = require('crypto');

const PROGRAM_ID = new PublicKey('FVtPQkdYvSNdpTA6QXYRcTBhDGgnufw2Enqmo2tQKr58');
const USDC_MINT = new PublicKey('7pCrfxhcAEyTFDhrhKRtRS2iMvEYx2dtNE7NzwuU7SA9');

// Seeds
const PM_CONFIG_SEED = Buffer.from('pm_config');
const MARKET_SEED = Buffer.from('market');
const YES_MINT_SEED = Buffer.from('yes_mint');
const NO_MINT_SEED = Buffer.from('no_mint');
const MARKET_VAULT_SEED = Buffer.from('market_vault');
const POSITION_SEED = Buffer.from('position');
const ORDER_SEED = Buffer.from('order');

// Instruction indices
const CREATE_MARKET_IX = 1;
const ACTIVATE_MARKET_IX = 2;
const MINT_COMPLETE_SET_IX = 7;
const PLACE_ORDER_IX = 9;
const MATCH_MINT_IX = 11;
const REDEEM_COMPLETE_SET_IX = 8;

const PRICE_PRECISION = 1_000_000;

class IntegrationTest {
  constructor(connection, admin) {
    this.connection = connection;
    this.admin = admin;
    this.marketId = null;
  }
  
  async getNextMarketId() {
    const [configPda] = PublicKey.findProgramAddressSync([PM_CONFIG_SEED], PROGRAM_ID);
    const account = await this.connection.getAccountInfo(configPda);
    return Number(account.data.readBigUInt64LE(168));
  }
  
  async createMarket() {
    console.log('\n>>> Step 1: Create Market');
    
    this.marketId = await this.getNextMarketId();
    console.log(`Market ID: ${this.marketId}`);
    
    const slot = await this.connection.getSlot();
    const blockTime = await this.connection.getBlockTime(slot);
    const resolutionTime = blockTime + 300; // 5 minutes
    const finalizationDeadline = blockTime + 600; // 10 minutes
    
    const questionHash = crypto.createHash('sha256').update(`Integration Test ${Date.now()}`).digest();
    const resolutionSpecHash = crypto.createHash('sha256').update('Test resolution').digest();
    
    const buffer = Buffer.alloc(1 + 32 + 32 + 8 + 8 + 2);
    let offset = 0;
    buffer.writeUInt8(CREATE_MARKET_IX, offset); offset += 1;
    questionHash.copy(buffer, offset); offset += 32;
    resolutionSpecHash.copy(buffer, offset); offset += 32;
    buffer.writeBigInt64LE(BigInt(resolutionTime), offset); offset += 8;
    buffer.writeBigInt64LE(BigInt(finalizationDeadline), offset); offset += 8;
    buffer.writeUInt16LE(100, offset); // 1% fee
    
    const [configPda] = PublicKey.findProgramAddressSync([PM_CONFIG_SEED], PROGRAM_ID);
    const marketIdBytes = Buffer.alloc(8);
    marketIdBytes.writeBigUInt64LE(BigInt(this.marketId));
    
    const [marketPda] = PublicKey.findProgramAddressSync([MARKET_SEED, marketIdBytes], PROGRAM_ID);
    const [yesMintPda] = PublicKey.findProgramAddressSync([YES_MINT_SEED, marketIdBytes], PROGRAM_ID);
    const [noMintPda] = PublicKey.findProgramAddressSync([NO_MINT_SEED, marketIdBytes], PROGRAM_ID);
    const [vaultPda] = PublicKey.findProgramAddressSync([MARKET_VAULT_SEED, marketIdBytes], PROGRAM_ID);
    
    this.marketPda = marketPda;
    this.yesMint = yesMintPda;
    this.noMint = noMintPda;
    this.vault = vaultPda;
    this.configPda = configPda;
    this.marketIdBytes = marketIdBytes;
    
    const ix = new TransactionInstruction({
      programId: PROGRAM_ID,
      keys: [
        { pubkey: this.admin.publicKey, isSigner: true, isWritable: true },
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
      data: buffer,
    });
    
    const tx = new Transaction().add(ix);
    const { blockhash } = await this.connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.feePayer = this.admin.publicKey;
    
    const sig = await sendAndConfirmTransaction(this.connection, tx, [this.admin]);
    console.log(`✅ Market created! Signature: ${sig.slice(0, 20)}...`);
    return true;
  }
  
  async activateMarket() {
    console.log('\n>>> Step 2: Activate Market');
    
    const buffer = Buffer.alloc(1 + 8);
    buffer.writeUInt8(ACTIVATE_MARKET_IX, 0);
    buffer.writeBigUInt64LE(BigInt(this.marketId), 1);
    
    const ix = new TransactionInstruction({
      programId: PROGRAM_ID,
      keys: [
        { pubkey: this.admin.publicKey, isSigner: true, isWritable: true },
        { pubkey: this.configPda, isSigner: false, isWritable: true },
        { pubkey: this.marketPda, isSigner: false, isWritable: true },
      ],
      data: buffer,
    });
    
    const tx = new Transaction().add(ix);
    const { blockhash } = await this.connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.feePayer = this.admin.publicKey;
    
    const sig = await sendAndConfirmTransaction(this.connection, tx, [this.admin]);
    console.log(`✅ Market activated! Signature: ${sig.slice(0, 20)}...`);
    return true;
  }
  
  async setupTokenAccounts() {
    console.log('\n>>> Step 2.5: Setup Token Accounts');
    
    const userYesAta = await getAssociatedTokenAddress(this.yesMint, this.admin.publicKey);
    const userNoAta = await getAssociatedTokenAddress(this.noMint, this.admin.publicKey);
    
    this.userYesAta = userYesAta;
    this.userNoAta = userNoAta;
    this.userUsdcAta = await getAssociatedTokenAddress(USDC_MINT, this.admin.publicKey);
    
    const tx = new Transaction();
    
    // Check if accounts exist
    const yesAccount = await this.connection.getAccountInfo(userYesAta);
    const noAccount = await this.connection.getAccountInfo(userNoAta);
    
    if (!yesAccount) {
      tx.add(createAssociatedTokenAccountInstruction(
        this.admin.publicKey, userYesAta, this.admin.publicKey, this.yesMint
      ));
    }
    
    if (!noAccount) {
      tx.add(createAssociatedTokenAccountInstruction(
        this.admin.publicKey, userNoAta, this.admin.publicKey, this.noMint
      ));
    }
    
    if (tx.instructions.length > 0) {
      const { blockhash } = await this.connection.getLatestBlockhash();
      tx.recentBlockhash = blockhash;
      tx.feePayer = this.admin.publicKey;
      await sendAndConfirmTransaction(this.connection, tx, [this.admin]);
      console.log('✅ Token accounts created');
    } else {
      console.log('✅ Token accounts already exist');
    }
    
    return true;
  }
  
  async mintCompleteSet(amount) {
    console.log(`\n>>> Step 3: Mint Complete Set (${amount / 1_000_000} tokens)`);
    
    const [positionPda] = PublicKey.findProgramAddressSync(
      [POSITION_SEED, this.marketIdBytes, this.admin.publicKey.toBuffer()],
      PROGRAM_ID
    );
    this.positionPda = positionPda;
    
    const buffer = Buffer.alloc(1 + 8 + 8);
    buffer.writeUInt8(MINT_COMPLETE_SET_IX, 0);
    buffer.writeBigUInt64LE(BigInt(this.marketId), 1);
    buffer.writeBigUInt64LE(BigInt(amount), 9);
    
    const ix = new TransactionInstruction({
      programId: PROGRAM_ID,
      keys: [
        { pubkey: this.admin.publicKey, isSigner: true, isWritable: true },
        { pubkey: this.configPda, isSigner: false, isWritable: true },
        { pubkey: this.marketPda, isSigner: false, isWritable: true },
        { pubkey: this.vault, isSigner: false, isWritable: true },
        { pubkey: this.userUsdcAta, isSigner: false, isWritable: true },
        { pubkey: this.yesMint, isSigner: false, isWritable: true },
        { pubkey: this.noMint, isSigner: false, isWritable: true },
        { pubkey: this.userYesAta, isSigner: false, isWritable: true },
        { pubkey: this.userNoAta, isSigner: false, isWritable: true },
        { pubkey: positionPda, isSigner: false, isWritable: true },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data: buffer,
    });
    
    const tx = new Transaction().add(ix);
    const { blockhash } = await this.connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.feePayer = this.admin.publicKey;
    
    const sig = await sendAndConfirmTransaction(this.connection, tx, [this.admin]);
    console.log(`✅ Minted ${amount / 1_000_000} complete sets! Signature: ${sig.slice(0, 20)}...`);
    return true;
  }
  
  async placeOrder(side, outcome, price, amount) {
    const sideNames = ['Buy', 'Sell'];
    const outcomeNames = ['Yes', 'No'];
    console.log(`\n>>> Step 4: Place Order (${sideNames[side]} ${outcomeNames[outcome]} @ $${price / PRICE_PRECISION})`);
    
    // Get next order ID from market
    const marketAccount = await this.connection.getAccountInfo(this.marketPda);
    const finalResultTag = marketAccount.data[226];
    let nextOrderIdOffset = finalResultTag === 0 ? 269 : 270;
    const orderId = marketAccount.data.readBigUInt64LE(nextOrderIdOffset);
    
    const orderIdBytes = Buffer.alloc(8);
    orderIdBytes.writeBigUInt64LE(orderId);
    const [orderPda] = PublicKey.findProgramAddressSync(
      [ORDER_SEED, this.marketIdBytes, orderIdBytes],
      PROGRAM_ID
    );
    
    const buffer = Buffer.alloc(1 + 8 + 1 + 1 + 8 + 8 + 1 + 1);
    let offset = 0;
    buffer.writeUInt8(PLACE_ORDER_IX, offset); offset += 1;
    buffer.writeBigUInt64LE(BigInt(this.marketId), offset); offset += 8;
    buffer.writeUInt8(side, offset); offset += 1;
    buffer.writeUInt8(outcome, offset); offset += 1;
    buffer.writeBigUInt64LE(BigInt(price), offset); offset += 8;
    buffer.writeBigUInt64LE(BigInt(amount), offset); offset += 8;
    buffer.writeUInt8(0, offset); offset += 1; // Limit order
    buffer.writeUInt8(0, offset); // No expiration
    
    const ix = new TransactionInstruction({
      programId: PROGRAM_ID,
      keys: [
        { pubkey: this.admin.publicKey, isSigner: true, isWritable: true },
        { pubkey: this.configPda, isSigner: false, isWritable: false },
        { pubkey: this.marketPda, isSigner: false, isWritable: true },
        { pubkey: orderPda, isSigner: false, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data: buffer,
    });
    
    const tx = new Transaction().add(ix);
    const { blockhash } = await this.connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.feePayer = this.admin.publicKey;
    
    const sig = await sendAndConfirmTransaction(this.connection, tx, [this.admin]);
    console.log(`✅ Order ${orderId} placed! Signature: ${sig.slice(0, 20)}...`);
    return Number(orderId);
  }
  
  async matchMint(yesOrderId, noOrderId, amount, yesPrice, noPrice) {
    console.log(`\n>>> Step 5: Match Mint (Order ${yesOrderId} + Order ${noOrderId})`);
    
    const yesOrderIdBytes = Buffer.alloc(8);
    yesOrderIdBytes.writeBigUInt64LE(BigInt(yesOrderId));
    const [yesOrderPda] = PublicKey.findProgramAddressSync(
      [ORDER_SEED, this.marketIdBytes, yesOrderIdBytes],
      PROGRAM_ID
    );
    
    const noOrderIdBytes = Buffer.alloc(8);
    noOrderIdBytes.writeBigUInt64LE(BigInt(noOrderId));
    const [noOrderPda] = PublicKey.findProgramAddressSync(
      [ORDER_SEED, this.marketIdBytes, noOrderIdBytes],
      PROGRAM_ID
    );
    
    const buffer = Buffer.alloc(1 + 8 + 8 + 8 + 8 + 8 + 8);
    let offset = 0;
    buffer.writeUInt8(MATCH_MINT_IX, offset); offset += 1;
    buffer.writeBigUInt64LE(BigInt(this.marketId), offset); offset += 8;
    buffer.writeBigUInt64LE(BigInt(yesOrderId), offset); offset += 8;
    buffer.writeBigUInt64LE(BigInt(noOrderId), offset); offset += 8;
    buffer.writeBigUInt64LE(BigInt(amount), offset); offset += 8;
    buffer.writeBigUInt64LE(BigInt(yesPrice), offset); offset += 8;
    buffer.writeBigUInt64LE(BigInt(noPrice), offset);
    
    const ix = new TransactionInstruction({
      programId: PROGRAM_ID,
      keys: [
        { pubkey: this.admin.publicKey, isSigner: true, isWritable: true },
        { pubkey: this.configPda, isSigner: false, isWritable: true },
        { pubkey: this.marketPda, isSigner: false, isWritable: true },
        { pubkey: yesOrderPda, isSigner: false, isWritable: true },
        { pubkey: noOrderPda, isSigner: false, isWritable: true },
        { pubkey: this.yesMint, isSigner: false, isWritable: true },
        { pubkey: this.noMint, isSigner: false, isWritable: true },
        { pubkey: this.userYesAta, isSigner: false, isWritable: true },
        { pubkey: this.userNoAta, isSigner: false, isWritable: true },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      ],
      data: buffer,
    });
    
    const tx = new Transaction().add(ix);
    const { blockhash } = await this.connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.feePayer = this.admin.publicKey;
    
    const sig = await sendAndConfirmTransaction(this.connection, tx, [this.admin]);
    console.log(`✅ Orders matched! Signature: ${sig.slice(0, 20)}...`);
    return true;
  }
  
  async redeemCompleteSet(amount) {
    console.log(`\n>>> Step 6: Redeem Complete Set (${amount / 1_000_000} tokens)`);
    
    const buffer = Buffer.alloc(1 + 8 + 8);
    buffer.writeUInt8(REDEEM_COMPLETE_SET_IX, 0);
    buffer.writeBigUInt64LE(BigInt(this.marketId), 1);
    buffer.writeBigUInt64LE(BigInt(amount), 9);
    
    const ix = new TransactionInstruction({
      programId: PROGRAM_ID,
      keys: [
        { pubkey: this.admin.publicKey, isSigner: true, isWritable: true },
        { pubkey: this.configPda, isSigner: false, isWritable: false },
        { pubkey: this.marketPda, isSigner: false, isWritable: true },
        { pubkey: this.vault, isSigner: false, isWritable: true },
        { pubkey: this.userUsdcAta, isSigner: false, isWritable: true },
        { pubkey: this.yesMint, isSigner: false, isWritable: true },
        { pubkey: this.noMint, isSigner: false, isWritable: true },
        { pubkey: this.userYesAta, isSigner: false, isWritable: true },
        { pubkey: this.userNoAta, isSigner: false, isWritable: true },
        { pubkey: this.positionPda, isSigner: false, isWritable: true },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      ],
      data: buffer,
    });
    
    const tx = new Transaction().add(ix);
    const { blockhash } = await this.connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.feePayer = this.admin.publicKey;
    
    const sig = await sendAndConfirmTransaction(this.connection, tx, [this.admin]);
    console.log(`✅ Redeemed ${amount / 1_000_000} complete sets! Signature: ${sig.slice(0, 20)}...`);
    return true;
  }
  
  async checkBalances() {
    console.log('\n>>> Final Balances:');
    try {
      const yesBalance = await this.connection.getTokenAccountBalance(this.userYesAta);
      const noBalance = await this.connection.getTokenAccountBalance(this.userNoAta);
      console.log(`  YES: ${yesBalance.value.uiAmount}`);
      console.log(`  NO: ${noBalance.value.uiAmount}`);
    } catch (e) {
      console.log('  Could not fetch balances');
    }
  }
}

async function main() {
  console.log('='.repeat(60));
  console.log('1024 Prediction Market - Integration Test');
  console.log('='.repeat(60));
  
  const connection = new Connection('http://127.0.0.1:8899', 'confirmed');
  
  const faucetPath = process.env.HOME + '/1024chain-testnet/keys/faucet.json';
  const faucetData = JSON.parse(fs.readFileSync(faucetPath, 'utf-8'));
  const admin = Keypair.fromSecretKey(new Uint8Array(faucetData));
  console.log(`Admin: ${admin.publicKey.toBase58()}`);
  
  const test = new IntegrationTest(connection, admin);
  
  try {
    // Step 1: Create market
    await test.createMarket();
    
    // Step 2: Activate market
    await test.activateMarket();
    
    // Step 2.5: Setup token accounts
    await test.setupTokenAccounts();
    
    // Step 3: Mint complete set
    await test.mintCompleteSet(50_000_000); // 50 tokens
    
    // Step 4: Place orders
    const yesOrderId = await test.placeOrder(0, 0, 600_000, 20_000_000); // Buy YES @ $0.60
    const noOrderId = await test.placeOrder(0, 1, 400_000, 20_000_000); // Buy NO @ $0.40
    
    // Step 5: Match orders
    await test.matchMint(yesOrderId, noOrderId, 20_000_000, 600_000, 400_000);
    
    // Step 6: Redeem complete set
    await test.redeemCompleteSet(10_000_000); // 10 tokens
    
    // Check final balances
    await test.checkBalances();
    
    console.log('\n' + '='.repeat(60));
    console.log('✅ Integration Test PASSED!');
    console.log('='.repeat(60));
    
  } catch (error) {
    console.error('\n❌ Integration Test FAILED:');
    if (error.logs) {
      error.logs.forEach(log => console.error('  ', log));
    }
    console.error(error.message || error);
    console.log('\n' + '='.repeat(60));
  }
}

main().catch(console.error);
