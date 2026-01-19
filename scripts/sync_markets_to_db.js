#!/usr/bin/env node
/**
 * Sync Chain Markets to Database
 * 
 * 将新程序创建的链上市场 (market_id = 1, 2) 同步到数据库
 * 保留已有的旧程序市场数据
 */

const { Connection, PublicKey } = require('@solana/web3.js');
const { Client } = require('pg');
const config = require('./config');

const PROGRAM_ID = new PublicKey(config.PROGRAM_ID);

// 要同步的市场配置
const MARKETS_TO_SYNC = [
  {
    market_id: 1,
    question: 'Will BTC reach $100k by 2025?',
    description: 'Binary prediction market for BTC price target',
    category: 'crypto',
    market_type: 'binary',
    num_outcomes: 2,
  },
  {
    market_id: 2,
    question: 'Who will win the next presidential election?',
    description: 'Multi-outcome prediction market for election',
    category: 'politics',
    market_type: 'multi_outcome',
    num_outcomes: 3, // Candidate A, B, C
  },
];

// PDA 推导函数
function deriveMarketPda(marketId) {
  const [pda, bump] = PublicKey.findProgramAddressSync(
    [Buffer.from('market'), Buffer.from(new BigUint64Array([BigInt(marketId)]).buffer)],
    PROGRAM_ID
  );
  return { pda, bump };
}

function deriveYesMint(marketId) {
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from('yes_mint'), Buffer.from(new BigUint64Array([BigInt(marketId)]).buffer)],
    PROGRAM_ID
  );
  return pda;
}

function deriveNoMint(marketId) {
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from('no_mint'), Buffer.from(new BigUint64Array([BigInt(marketId)]).buffer)],
    PROGRAM_ID
  );
  return pda;
}

function deriveVault(marketId) {
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from('market_vault'), Buffer.from(new BigUint64Array([BigInt(marketId)]).buffer)],
    PROGRAM_ID
  );
  return pda;
}

// 计算 question_hash (简单 SHA256)
const crypto = require('crypto');
function computeQuestionHash(question) {
  return crypto.createHash('sha256').update(question).digest('hex');
}

async function verifyChainAccount(conn, pda) {
  try {
    const account = await conn.getAccountInfo(pda);
    return account !== null;
  } catch (e) {
    return false;
  }
}

async function main() {
  console.log('=== 同步链上市场到数据库 ===\n');
  console.log('Program ID:', config.PROGRAM_ID.toString());
  
  // 连接 Solana
  const conn = new Connection(config.RPC_URL);
  console.log('RPC URL:', config.RPC_URL);
  
  // 连接数据库
  const dbUrl = process.env.DATABASE_URL;
  if (!dbUrl) {
    console.error('❌ DATABASE_URL 环境变量未设置');
    console.log('请先运行: source /Users/patrick/Developer/1024ex/1024-core/.env');
    process.exit(1);
  }
  
  const client = new Client({ connectionString: dbUrl });
  await client.connect();
  console.log('✅ 已连接到数据库\n');
  
  try {
    for (const market of MARKETS_TO_SYNC) {
      console.log(`\n--- 处理 Market ${market.market_id}: ${market.question} ---`);
      
      // 计算 PDA 地址
      const { pda: marketPda, bump } = deriveMarketPda(market.market_id);
      const yesMint = deriveYesMint(market.market_id);
      const noMint = deriveNoMint(market.market_id);
      const vault = deriveVault(market.market_id);
      
      console.log('Market PDA:', marketPda.toBase58());
      console.log('Bump:', bump);
      
      // 验证链上账户存在
      const exists = await verifyChainAccount(conn, marketPda);
      if (!exists) {
        console.log('⚠️ 链上账户不存在，跳过');
        continue;
      }
      console.log('✅ 链上账户验证通过');
      
      // 检查数据库中是否已存在
      const checkResult = await client.query(
        'SELECT id FROM prediction_markets WHERE market_id = $1',
        [market.market_id]
      );
      
      if (checkResult.rows.length > 0) {
        console.log('⚠️ 数据库中已存在，跳过');
        continue;
      }
      
      // 计算时间戳 (未来 30 天)
      const now = new Date();
      const endTime = new Date(now.getTime() + 30 * 24 * 60 * 60 * 1000);
      const resolutionTime = new Date(endTime.getTime() + 1 * 60 * 60 * 1000);
      const finalizationDeadline = new Date(resolutionTime.getTime() + 24 * 60 * 60 * 1000);
      
      // 插入市场记录
      const insertMarketQuery = `
        INSERT INTO prediction_markets (
          market_id, market_pda, market_type, num_outcomes,
          question, question_hash, description,
          category, creator, creator_fee_bps,
          yes_mint, no_mint, vault,
          status, end_time, resolution_time, finalization_deadline,
          bump, slug
        ) VALUES (
          $1, $2, $3, $4,
          $5, $6, $7,
          $8, $9, $10,
          $11, $12, $13,
          $14, $15, $16, $17,
          $18, $19
        )
        ON CONFLICT (market_id) DO NOTHING
        RETURNING id;
      `;
      
      const slug = `market-${market.market_id}-${Date.now()}`;
      const creator = config.ADMIN_KEYPAIR_PATH ? 'admin' : 'system';
      
      const insertResult = await client.query(insertMarketQuery, [
        market.market_id,
        marketPda.toBase58(),
        market.market_type,
        market.num_outcomes,
        market.question,
        computeQuestionHash(market.question),
        market.description,
        market.category,
        creator,
        0, // creator_fee_bps
        yesMint.toBase58(),
        noMint.toBase58(),
        vault.toBase58(),
        'active', // status
        endTime.toISOString(),
        resolutionTime.toISOString(),
        finalizationDeadline.toISOString(),
        bump,
        slug,
      ]);
      
      if (insertResult.rows.length > 0) {
        console.log('✅ 市场记录已插入, UUID:', insertResult.rows[0].id);
        
        // 插入 outcomes
        if (market.market_type === 'binary') {
          // 二元市场: YES (0) 和 NO (1)
          await insertOutcome(client, market.market_id, 0, 'Yes', yesMint.toBase58());
          await insertOutcome(client, market.market_id, 1, 'No', noMint.toBase58());
        } else {
          // 多选市场: 多个选项
          const outcomes = ['Candidate A', 'Candidate B', 'Candidate C'];
          for (let i = 0; i < outcomes.length; i++) {
            await insertOutcome(client, market.market_id, i, outcomes[i], null);
          }
        }
        console.log('✅ Outcomes 已插入');
      } else {
        console.log('⚠️ 市场记录插入失败或已存在');
      }
    }
    
    console.log('\n=== 同步完成 ===\n');
    
    // 显示当前市场列表
    const listResult = await client.query(`
      SELECT market_id, question, status, market_pda 
      FROM prediction_markets 
      WHERE market_id IN (1, 2)
      ORDER BY market_id
    `);
    
    console.log('新同步的市场:');
    for (const row of listResult.rows) {
      console.log(`  Market ${row.market_id}: ${row.question} [${row.status}]`);
      console.log(`    PDA: ${row.market_pda}`);
    }
    
  } finally {
    await client.end();
    console.log('\n✅ 数据库连接已关闭');
  }
}

async function insertOutcome(client, marketId, outcomeIndex, label, outcomeMint) {
  const labelHash = crypto.createHash('sha256').update(label).digest('hex');
  const query = `
    INSERT INTO prediction_market_outcomes (
      market_id, outcome_index, label, label_hash, outcome_mint
    ) VALUES ($1, $2, $3, $4, $5)
    ON CONFLICT (market_id, outcome_index) DO NOTHING;
  `;
  
  await client.query(query, [marketId, outcomeIndex, label, labelHash, outcomeMint || '']);
  console.log(`  - Outcome ${outcomeIndex}: ${label}`);
}

main().catch(err => {
  console.error('❌ 致命错误:', err);
  process.exit(1);
});

