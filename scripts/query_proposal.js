/**
 * QueryProposal - Query oracle proposal status
 * Run on server: node query_proposal.js [market_id]
 */

const { Connection, PublicKey } = require('@solana/web3.js');
const config = require('./config');

const PROGRAM_ID = config.PROGRAM_ID;
const PROPOSAL_SEED = Buffer.from('oracle_proposal');

async function main() {
  const marketId = process.argv[2] ? parseInt(process.argv[2]) : 2;
  
  console.log('='.repeat(60));
  console.log(`1024 Prediction Market - Query Proposal ${marketId}`);
  console.log('='.repeat(60));
  
  const connection = new Connection('http://127.0.0.1:8899', 'confirmed');
  
  const marketIdBytes = Buffer.alloc(8);
  marketIdBytes.writeBigUInt64LE(BigInt(marketId));
  
  const [proposalPda] = PublicKey.findProgramAddressSync(
    [PROPOSAL_SEED, marketIdBytes],
    PROGRAM_ID
  );
  
  console.log(`\nProposal PDA: ${proposalPda.toBase58()}`);
  
  const account = await connection.getAccountInfo(proposalPda);
  if (!account) {
    console.log('❌ Proposal not found!');
    return;
  }
  
  console.log(`Account size: ${account.data.length} bytes`);
  
  const data = account.data;
  let offset = 0;
  
  // OracleProposal struct layout (from state.rs):
  // discriminator: u64 (8)
  // market_id: u64 (8)
  // proposer: Pubkey (32)
  // proposed_result: u8 (MarketResult enum)
  // status: u8 (ProposalStatus enum)
  // proposed_at: i64 (8)
  // challenge_deadline: i64 (8)
  // bond_amount: u64 (8)
  // challenger: Option<Pubkey> (1 + 32 if Some)
  // challenger_result: Option<u8> (1 + 1 if Some)
  
  const discriminator = data.readBigUInt64LE(offset); offset += 8;
  const readMarketId = data.readBigUInt64LE(offset); offset += 8;
  const proposer = new PublicKey(data.slice(offset, offset + 32)); offset += 32;
  const proposedResult = data.readUInt8(offset); offset += 1;
  const status = data.readUInt8(offset); offset += 1;
  const proposalTime = data.readBigInt64LE(offset); offset += 8;
  const challengeDeadline = data.readBigInt64LE(offset); offset += 8;
  const bondAmount = data.readBigUInt64LE(offset); offset += 8;
  
  // Read challenger (Option<Pubkey>)
  const hasChallenger = data.readUInt8(offset); offset += 1;
  let challenger = null;
  if (hasChallenger === 1) {
    challenger = new PublicKey(data.slice(offset, offset + 32));
    offset += 32;
  }
  
  const resultNames = ['Yes', 'No', 'Invalid'];
  const statusNames = ['Pending', 'Challenged', 'Finalized', 'Disputed'];
  
  console.log('\n=== Proposal Details ===');
  console.log(`market_id: ${readMarketId}`);
  console.log(`proposer: ${proposer.toBase58()}`);
  console.log(`proposed_result: ${resultNames[proposedResult]} (${proposedResult})`);
  console.log(`proposal_time: ${new Date(Number(proposalTime) * 1000).toISOString()}`);
  console.log(`challenge_deadline: ${new Date(Number(challengeDeadline) * 1000).toISOString()}`);
  console.log(`status: ${statusNames[status]} (${status})`);
  
  if (challenger) {
    console.log(`challenger: ${challenger.toBase58()}`);
  }
  
  // Get current blockchain time
  const slot = await connection.getSlot();
  const blockTime = await connection.getBlockTime(slot);
  console.log(`\n--- Time Check ---`);
  console.log(`Current blockchain time: ${new Date(blockTime * 1000).toISOString()}`);
  
  const deadline = Number(challengeDeadline);
  if (blockTime >= deadline) {
    console.log(`✅ Challenge window has expired! Can finalize.`);
  } else {
    const remaining = deadline - blockTime;
    console.log(`⏳ Challenge window expires in: ${remaining} seconds (${(remaining/3600).toFixed(2)} hours)`);
  }
  
  console.log('\n' + '='.repeat(60));
}

main().catch(console.error);
