/**
 * Test script for 1024 Prediction Market Program
 * Tests the Initialize instruction
 */

import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
  SystemProgram,
  sendAndConfirmTransaction,
} from '@solana/web3.js';
import * as borsh from 'borsh';
import * as fs from 'fs';

// Constants
const PROGRAM_ID = new PublicKey('FVtPQkdYvSNdpTA6QXYRcTBhDGgnufw2Enqmo2tQKr58');
const RPC_URL = 'https://rpc-testnet.1024chain.com/rpc/';

// Vault Program (existing)
const VAULT_PROGRAM = new PublicKey('vR3BifKCa2TGKP2uhToxZAMYAYydqpesvKGX54gzFny');
// Fund Program (existing)
const FUND_PROGRAM = new PublicKey('FPhDzu7yCDC1BBvzGwpM6dHHNQBPpKEv6Y3Ptdc7o3fJ');
// USDC Mint (existing)
const USDC_MINT = new PublicKey('7pCrfxhcAEyTFDhrhKRtRS2iMvEYx2dtNE7NzwuU7SA9');

// PDA Seeds
const PM_CONFIG_SEED = Buffer.from('pm_config');

// InitializeArgs schema for borsh serialization
class InitializeArgs {
  instruction: number;
  oracle_admin: Uint8Array;
  challenge_window_secs: bigint;
  proposer_bond_e6: bigint;

  constructor(fields: {
    instruction: number;
    oracle_admin: Uint8Array;
    challenge_window_secs: bigint;
    proposer_bond_e6: bigint;
  }) {
    this.instruction = fields.instruction;
    this.oracle_admin = fields.oracle_admin;
    this.challenge_window_secs = fields.challenge_window_secs;
    this.proposer_bond_e6 = fields.proposer_bond_e6;
  }
}

const initializeArgsSchema = new Map([
  [
    InitializeArgs,
    {
      kind: 'struct',
      fields: [
        ['instruction', 'u8'],
        ['oracle_admin', [32]],
        ['challenge_window_secs', 'i64'],
        ['proposer_bond_e6', 'u64'],
      ],
    },
  ],
]);

async function main() {
  console.log('='.repeat(60));
  console.log('1024 Prediction Market Program - Initialize Test');
  console.log('='.repeat(60));
  console.log(`Program ID: ${PROGRAM_ID.toBase58()}`);
  console.log(`RPC URL: ${RPC_URL}`);
  console.log('');

  // Connect to cluster
  const connection = new Connection(RPC_URL, 'confirmed');
  console.log('Connected to 1024Chain Testnet');

  // Load faucet keypair
  const faucetPath = process.env.HOME + '/1024chain-testnet/keys/faucet.json';
  let admin: Keypair;
  
  try {
    const faucetData = JSON.parse(fs.readFileSync(faucetPath, 'utf-8'));
    admin = Keypair.fromSecretKey(new Uint8Array(faucetData));
  } catch (e) {
    // Use local faucet key if on local machine
    const localFaucetPath = '/tmp/faucet.json';
    const faucetData = JSON.parse(fs.readFileSync(localFaucetPath, 'utf-8'));
    admin = Keypair.fromSecretKey(new Uint8Array(faucetData));
  }
  
  console.log(`Admin: ${admin.publicKey.toBase58()}`);

  // Derive PredictionMarketConfig PDA
  const [configPda, configBump] = await PublicKey.findProgramAddress(
    [PM_CONFIG_SEED],
    PROGRAM_ID
  );
  console.log(`Config PDA: ${configPda.toBase58()} (bump: ${configBump})`);

  // Check if already initialized
  const configAccount = await connection.getAccountInfo(configPda);
  if (configAccount) {
    console.log('\n⚠️  Program already initialized!');
    console.log(`Config account size: ${configAccount.data.length} bytes`);
    console.log(`Config account owner: ${configAccount.owner.toBase58()}`);
    return;
  }

  // Create Initialize instruction
  console.log('\nCreating Initialize instruction...');
  
  const initArgs = new InitializeArgs({
    instruction: 0, // Initialize instruction index
    oracle_admin: admin.publicKey.toBytes(),
    challenge_window_secs: BigInt(24 * 60 * 60), // 24 hours
    proposer_bond_e6: BigInt(100_000_000), // 100 USDC
  });

  const data = borsh.serialize(initializeArgsSchema, initArgs);
  console.log(`Instruction data size: ${data.length} bytes`);

  const initializeIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: admin.publicKey, isSigner: true, isWritable: true }, // Admin
      { pubkey: configPda, isSigner: false, isWritable: true }, // Config PDA
      { pubkey: USDC_MINT, isSigner: false, isWritable: false }, // USDC Mint
      { pubkey: VAULT_PROGRAM, isSigner: false, isWritable: false }, // Vault Program
      { pubkey: FUND_PROGRAM, isSigner: false, isWritable: false }, // Fund Program
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }, // System Program
    ],
    data: Buffer.from(data),
  });

  // Send transaction
  console.log('\nSending transaction...');
  const tx = new Transaction().add(initializeIx);
  
  try {
    const signature = await sendAndConfirmTransaction(connection, tx, [admin], {
      commitment: 'confirmed',
    });
    console.log('\n✅ Initialize successful!');
    console.log(`Signature: ${signature}`);
    console.log(`Explorer: https://testnet-scan.1024chain.com/tx/${signature}`);
  } catch (error) {
    console.error('\n❌ Transaction failed:');
    console.error(error);
  }

  // Verify initialization
  console.log('\nVerifying initialization...');
  const finalConfigAccount = await connection.getAccountInfo(configPda);
  if (finalConfigAccount) {
    console.log('✅ Config account created');
    console.log(`  Size: ${finalConfigAccount.data.length} bytes`);
    console.log(`  Owner: ${finalConfigAccount.owner.toBase58()}`);
  } else {
    console.log('❌ Config account not found');
  }

  console.log('\n' + '='.repeat(60));
  console.log('Test completed');
  console.log('='.repeat(60));
}

main().catch(console.error);
