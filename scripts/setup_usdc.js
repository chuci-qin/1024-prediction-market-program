/**
 * Setup USDC for testing
 * Creates USDC token account and mints USDC (if you have mint authority)
 */

const { 
  Connection, 
  Keypair, 
  PublicKey, 
  Transaction, 
  sendAndConfirmTransaction,
} = require('@solana/web3.js');
const { 
  TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddress,
  createMintToInstruction,
  getMint,
} = require('@solana/spl-token');
const fs = require('fs');

const USDC_MINT = new PublicKey('7pCrfxhcAEyTFDhrhKRtRS2iMvEYx2dtNE7NzwuU7SA9');

async function main() {
  console.log('='.repeat(60));
  console.log('Setup USDC for Prediction Market Testing');
  console.log('='.repeat(60));
  
  const connection = new Connection(config.RPC_URL, 'confirmed');
  
  // Load faucet keypair
  const faucetPath = process.env.ADMIN_KEYPAIR || '../faucet.json';
  const faucetData = JSON.parse(fs.readFileSync(faucetPath, 'utf-8'));
  const user = Keypair.fromSecretKey(new Uint8Array(faucetData));
  console.log(`User: ${user.publicKey.toBase58()}`);
  
  // Get USDC mint info
  console.log('\n=== USDC Mint Info ===');
  try {
    const mintInfo = await getMint(connection, USDC_MINT);
    console.log(`Mint: ${USDC_MINT.toBase58()}`);
    console.log(`Decimals: ${mintInfo.decimals}`);
    console.log(`Supply: ${Number(mintInfo.supply) / 1_000_000} USDC`);
    console.log(`Mint Authority: ${mintInfo.mintAuthority?.toBase58() || 'None'}`);
    console.log(`Freeze Authority: ${mintInfo.freezeAuthority?.toBase58() || 'None'}`);
    
    // Check if user is mint authority
    const isMintAuthority = mintInfo.mintAuthority && mintInfo.mintAuthority.equals(user.publicKey);
    console.log(`User is Mint Authority: ${isMintAuthority}`);
    
    // Get or create user USDC account
    const userUsdcAta = await getAssociatedTokenAddress(USDC_MINT, user.publicKey);
    console.log(`\n=== User USDC Account ===`);
    console.log(`ATA: ${userUsdcAta.toBase58()}`);
    
    const tx = new Transaction();
    
    // Check if account exists
    const ataInfo = await connection.getAccountInfo(userUsdcAta);
    if (!ataInfo) {
      console.log('Creating USDC token account...');
      tx.add(
        createAssociatedTokenAccountInstruction(
          user.publicKey,
          userUsdcAta,
          user.publicKey,
          USDC_MINT
        )
      );
    }
    
    // Mint USDC if user is mint authority
    if (isMintAuthority) {
      const mintAmount = 1000_000_000; // 1000 USDC (6 decimals)
      console.log(`Minting ${mintAmount / 1_000_000} USDC...`);
      tx.add(
        createMintToInstruction(
          USDC_MINT,
          userUsdcAta,
          user.publicKey,
          mintAmount
        )
      );
    }
    
    if (tx.instructions.length > 0) {
      const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
      tx.recentBlockhash = blockhash;
      tx.lastValidBlockHeight = lastValidBlockHeight;
      tx.feePayer = user.publicKey;
      
      const signature = await sendAndConfirmTransaction(connection, tx, [user], {
        commitment: 'confirmed',
      });
      console.log(`\n✅ Transaction successful!`);
      console.log(`Signature: ${signature}`);
    }
    
    // Check balance
    try {
      const balance = await connection.getTokenAccountBalance(userUsdcAta);
      console.log(`\n=== Final Balance ===`);
      console.log(`USDC Balance: ${balance.value.uiAmount} USDC`);
    } catch (e) {
      console.log(`USDC Balance: 0`);
    }
    
  } catch (error) {
    console.error('Error:', error.message);
  }
  
  console.log('\n' + '='.repeat(60));
}

main().catch(console.error);
