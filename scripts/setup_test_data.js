#!/usr/bin/env node
/**
 * Setup Test Data - 创建完整的链上测试数据
 * 
 * 功能：
 * 1. 创建二元市场 (market_id = 1)
 * 2. 激活市场
 * 3. 铸造份额 (50 USDC)
 * 4. 创建买单和卖单
 * 5. 创建多选市场 (market_id = 2)
 * 6. 激活多选市场
 * 7. 铸造多选份额
 * 
 * 用法: node setup_test_data.js
 */

const { execSync, spawn } = require('child_process');
const path = require('path');

const SCRIPT_DIR = __dirname;

// 执行命令并等待完成
function runScript(scriptName, args = []) {
  const cmd = `node ${scriptName} ${args.join(' ')}`;
  console.log(`\n>>> Running: ${cmd}`);
  try {
    execSync(cmd, { 
      stdio: 'inherit', 
      cwd: SCRIPT_DIR,
      timeout: 60000 // 60 秒超时
    });
    return true;
  } catch (error) {
    console.error(`❌ Script failed: ${scriptName}`);
    return false;
  }
}

// 等待一段时间
function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function main() {
  console.log('='.repeat(60));
  console.log('  1024 Prediction Market - Setup Test Data');
  console.log('='.repeat(60));
  console.log(`\nProgram ID: 9hsG1DksmgadjjJTEEX7CdevQKYVkQag3mEratPRZXjv`);
  console.log(`Date: ${new Date().toISOString()}`);
  console.log('');

  const createdMarkets = [];
  let success = true;

  // ============================================================
  // Step 1: 查询当前配置
  // ============================================================
  console.log('\n' + '='.repeat(60));
  console.log('  Step 1: Query Current Config');
  console.log('='.repeat(60));
  
  runScript('query_config.js');

  // ============================================================
  // Step 2: 创建二元市场
  // ============================================================
  console.log('\n' + '='.repeat(60));
  console.log('  Step 2: Create Binary Market');
  console.log('='.repeat(60));
  
  if (runScript('create_market.js')) {
    createdMarkets.push({ id: 1, type: 'binary' });
  } else {
    console.log('⚠️ Binary market may already exist, continuing...');
    createdMarkets.push({ id: 1, type: 'binary', existed: true });
  }
  
  await sleep(3000);

  // ============================================================
  // Step 3: 激活二元市场
  // ============================================================
  console.log('\n' + '='.repeat(60));
  console.log('  Step 3: Activate Binary Market (ID: 1)');
  console.log('='.repeat(60));
  
  runScript('activate_market.js', ['1']);
  await sleep(2000);

  // ============================================================
  // Step 4: 查询二元市场状态
  // ============================================================
  console.log('\n' + '='.repeat(60));
  console.log('  Step 4: Query Binary Market');
  console.log('='.repeat(60));
  
  runScript('query_market.js', ['1']);

  // ============================================================
  // Step 5: 铸造二元市场份额
  // ============================================================
  console.log('\n' + '='.repeat(60));
  console.log('  Step 5: Mint Complete Set for Binary Market (50 USDC)');
  console.log('='.repeat(60));
  
  runScript('mint_complete_set.js', ['1', '50000000']);
  await sleep(2000);

  // ============================================================
  // Step 6: 创建买单
  // ============================================================
  console.log('\n' + '='.repeat(60));
  console.log('  Step 6: Place Buy Order for YES @ 0.55');
  console.log('='.repeat(60));
  
  // place_order.js [market_id] [side] [outcome] [price] [amount]
  // side: buy/sell, outcome: yes/no, price: 0.0-1.0, amount: e6
  runScript('place_order.js', ['1', 'buy', 'yes', '0.55', '10000000']);
  await sleep(2000);

  // ============================================================
  // Step 7: 创建卖单
  // ============================================================
  console.log('\n' + '='.repeat(60));
  console.log('  Step 7: Place Sell Order for NO @ 0.45');
  console.log('='.repeat(60));
  
  runScript('place_order.js', ['1', 'sell', 'no', '0.45', '10000000']);
  await sleep(2000);

  // ============================================================
  // Step 8: 创建多选市场
  // ============================================================
  console.log('\n' + '='.repeat(60));
  console.log('  Step 8: Create Multi-Outcome Market (3 outcomes)');
  console.log('='.repeat(60));
  
  if (runScript('create_multi_outcome_market.js', ['3', '"Test Multi-Outcome Market"'])) {
    createdMarkets.push({ id: 2, type: 'multi-outcome', outcomes: 3 });
  } else {
    console.log('⚠️ Multi-outcome market may already exist, continuing...');
    createdMarkets.push({ id: 2, type: 'multi-outcome', outcomes: 3, existed: true });
  }
  await sleep(3000);

  // ============================================================
  // Step 9: 激活多选市场
  // ============================================================
  console.log('\n' + '='.repeat(60));
  console.log('  Step 9: Activate Multi-Outcome Market (ID: 2)');
  console.log('='.repeat(60));
  
  runScript('activate_market.js', ['2']);
  await sleep(2000);

  // ============================================================
  // Step 10: 铸造多选市场份额
  // ============================================================
  console.log('\n' + '='.repeat(60));
  console.log('  Step 10: Mint Multi-Outcome Complete Set (50 USDC)');
  console.log('='.repeat(60));
  
  runScript('mint_multi_outcome_set.js', ['2', '50000000', '3']);
  await sleep(2000);

  // ============================================================
  // Summary
  // ============================================================
  console.log('\n' + '='.repeat(60));
  console.log('  SETUP COMPLETE - SUMMARY');
  console.log('='.repeat(60));
  console.log('');
  console.log('Created Markets:');
  createdMarkets.forEach(m => {
    const status = m.existed ? '(already existed)' : '(newly created)';
    console.log(`  - Market ID ${m.id}: ${m.type} ${status}`);
  });
  console.log('');
  console.log('Next Steps:');
  console.log('  1. Update test config to use market_id: 1 (binary) and 2 (multi-outcome)');
  console.log('  2. Run Public API tests');
  console.log('');
  console.log('Commands:');
  console.log('  - Query market: node query_market.js 1');
  console.log('  - Query orders: node query_order.js 1 1');
  console.log('');
}

main().catch(console.error);





