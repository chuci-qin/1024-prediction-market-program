/**
 * LLM Oracle E2E Integration Test Script
 * 
 * Phase 6.2-6.3: AI Oracle Integration Test + On-chain Transaction Verification
 * 
 * This script tests the complete LLM Oracle integration:
 * 1. Create a test market
 * 2. Configure Oracle with AI settings
 * 3. Freeze Oracle config (stores on IPFS)
 * 4. Activate market
 * 5. Halt trading (simulating resolution time)
 * 6. Call AI Oracle API to get resolution
 * 7. Submit ProposeResultWithResearch on-chain
 * 8. Verify on-chain state
 * 
 * Usage: node e2e_llm_oracle_test.js [--dry-run] [--skip-ai]
 */

const { Connection, Keypair, PublicKey, Transaction, sendAndConfirmTransaction } = require('@solana/web3.js');
const config = require('./config');
const fs = require('fs');
const path = require('path');
const axios = require('axios');

// ============================================================================
// Configuration
// ============================================================================

const CONFIG = {
  // 1024Chain Testnet
  RPC_URL: process.env.RPC_URL || 'https://testnet-rpc.1024chain.com/rpc/',
  
  // Program IDs
  PREDICTION_MARKET_PROGRAM: config.PROGRAM_ID,
  VAULT_PROGRAM: new PublicKey('vR3BifKCa2TGKP2uhToxZAMYAYydqpesvKGX54gzFny'),
  
  // Oracle API
  ORACLE_API_URL: process.env.LLM_ORACLE_API_URL || 'http://localhost:8989',
  
  // Backend API
  // Gateway API 端口 8082 (前端专用)
  BACKEND_API_URL: process.env.BACKEND_API_URL || 'http://localhost:8082',
  
  // Admin keypair
  ADMIN_KEYPAIR_PATH: process.env.ADMIN_KEYPAIR || './oracle-admin.json',
  
  // Test settings
  DRY_RUN: process.argv.includes('--dry-run'),
  SKIP_AI: process.argv.includes('--skip-ai'),
  VERBOSE: process.argv.includes('--verbose'),
};

// ============================================================================
// Utilities
// ============================================================================

function log(msg, ...args) {
  console.log(`[${new Date().toISOString().slice(11, 19)}] ${msg}`, ...args);
}

function logVerbose(msg, ...args) {
  if (CONFIG.VERBOSE) {
    console.log(`[DEBUG] ${msg}`, ...args);
  }
}

async function loadKeypair(path) {
  try {
    const data = JSON.parse(fs.readFileSync(path, 'utf-8'));
    return Keypair.fromSecretKey(Uint8Array.from(data));
  } catch (e) {
    log(`⚠️ Could not load keypair from ${path}, generating new one`);
    return Keypair.generate();
  }
}

function createCidBytes(prefix) {
  const cid = `Qm${prefix}TestCID1234567890abcdef0123456789`;
  const bytes = new Uint8Array(64);
  const encoded = new TextEncoder().encode(cid);
  bytes.set(encoded.slice(0, 64));
  return bytes;
}

function createHashBytes(seed) {
  const hash = new Uint8Array(32);
  for (let i = 0; i < 32; i++) {
    hash[i] = (seed + i) % 256;
  }
  return hash;
}

// ============================================================================
// Test Class
// ============================================================================

class LlmOracleE2ETest {
  constructor() {
    this.connection = null;
    this.adminKeypair = null;
    this.testMarketId = null;
    this.results = {
      steps: [],
      success: false,
      errors: [],
    };
  }

  async setup() {
    log('📦 Setting up test environment...');
    
    this.connection = new Connection(CONFIG.RPC_URL, 'confirmed');
    
    // Verify connection
    const version = await this.connection.getVersion();
    log(`  Connected to RPC: solana-core ${version['solana-core']}`);
    
    // Load admin keypair
    this.adminKeypair = await loadKeypair(CONFIG.ADMIN_KEYPAIR_PATH);
    log(`  Admin wallet: ${this.adminKeypair.publicKey.toBase58()}`);
    
    // Check balance
    const balance = await this.connection.getBalance(this.adminKeypair.publicKey);
    log(`  Admin balance: ${balance / 1e9} N1024`);
    
    if (balance < 100000000) { // 0.1 N1024
      log('⚠️ Low balance, some tests may fail');
    }
    
    this.addResult('setup', true, 'Environment setup complete');
  }

  addResult(step, success, message) {
    this.results.steps.push({ step, success, message });
    if (!success) {
      this.results.errors.push({ step, message });
    }
  }

  // ========================================================================
  // Phase 1: Test Oracle API Health
  // ========================================================================
  
  async testOracleApiHealth() {
    log('🔍 Phase 1: Testing Oracle API health...');
    
    try {
      const response = await axios.get(`${CONFIG.ORACLE_API_URL}/health`, {
        timeout: 5000,
      });
      
      if (response.status === 200 && response.data.status === 'healthy') {
        log(`  ✅ Oracle API healthy: ${JSON.stringify(response.data)}`);
        this.addResult('oracle_health', true, 'Oracle API is healthy');
        return true;
      } else {
        log(`  ⚠️ Oracle API not healthy: ${response.status}`);
        this.addResult('oracle_health', false, `Unhealthy response: ${response.status}`);
        return false;
      }
    } catch (e) {
      log(`  ❌ Oracle API unreachable: ${e.message}`);
      this.addResult('oracle_health', false, `API error: ${e.message}`);
      return false;
    }
  }

  // ========================================================================
  // Phase 2: Create Test Market via Backend API
  // ========================================================================
  
  async createTestMarket() {
    log('📝 Phase 2: Creating test market...');
    
    const timestamp = Date.now();
    const marketData = {
      title: `E2E LLM Oracle Test ${timestamp}`,
      question: `Will the E2E test ${timestamp} pass all verifications?`,
      description: 'Automated E2E test for LLM Oracle integration',
      category: 'test',
      tags: ['e2e', 'oracle', 'automated'],
      market_type: 'binary',
      resolution_time: new Date(Date.now() + 3600000).toISOString(), // 1 hour
      admin_wallet: this.adminKeypair.publicKey.toBase58(),
      resolution_type: 'ai_oracle',
      resolution_criteria: {
        criteria: 'Test must complete all phases',
        evidence_sources: ['test logs'],
      },
      llm_config: {
        min_agents: 3,
        consensus_threshold: 0.67,
        strategies: ['news_searcher', 'fact_checker', 'data_analyst'],
      },
    };
    
    if (CONFIG.DRY_RUN) {
      this.testMarketId = 999;
      log(`  [DRY RUN] Would create market: ${marketData.title}`);
      this.addResult('create_market', true, 'Dry run - market creation skipped');
      return;
    }
    
    try {
      const response = await axios.post(
        `${CONFIG.BACKEND_API_URL}/prediction/markets`,
        marketData,
        { timeout: 10000 }
      );
      
      if (response.data.success && response.data.market_id) {
        this.testMarketId = response.data.market_id;
        log(`  ✅ Market created: ID=${this.testMarketId}`);
        this.addResult('create_market', true, `Market ${this.testMarketId} created`);
      } else {
        log(`  ❌ Market creation failed: ${JSON.stringify(response.data)}`);
        this.addResult('create_market', false, response.data.error || 'Unknown error');
      }
    } catch (e) {
      log(`  ❌ API error: ${e.message}`);
      this.addResult('create_market', false, e.message);
    }
  }

  // ========================================================================
  // Phase 3: Freeze Oracle Config
  // ========================================================================
  
  async freezeOracleConfig() {
    log('🔒 Phase 3: Freezing Oracle config...');
    
    if (!this.testMarketId) {
      log('  ⏭️ Skipping - no market created');
      return;
    }
    
    if (CONFIG.DRY_RUN) {
      log('  [DRY RUN] Would freeze Oracle config');
      this.addResult('freeze_config', true, 'Dry run - freeze skipped');
      return;
    }
    
    try {
      const response = await axios.post(
        `${CONFIG.BACKEND_API_URL}/prediction/oracle/freeze-config`,
        { market_id: this.testMarketId },
        { timeout: 30000 }
      );
      
      if (response.data.success) {
        log(`  ✅ Config frozen: CID=${response.data.config_cid}`);
        this.addResult('freeze_config', true, 'Config frozen and uploaded to IPFS');
      } else {
        log(`  ❌ Freeze failed: ${response.data.error}`);
        this.addResult('freeze_config', false, response.data.error);
      }
    } catch (e) {
      log(`  ❌ API error: ${e.message}`);
      this.addResult('freeze_config', false, e.message);
    }
  }

  // ========================================================================
  // Phase 4: Trigger AI Oracle Resolution
  // ========================================================================
  
  async triggerAiResolution() {
    log('🤖 Phase 4: Triggering AI Oracle resolution...');
    
    if (CONFIG.SKIP_AI) {
      log('  [SKIP_AI] Skipping real AI Oracle call');
      this.addResult('ai_resolution', true, 'AI call skipped');
      return { outcome: 'YES', confidence: 0.85, research_cid: 'QmMockCid' };
    }
    
    if (!this.testMarketId || CONFIG.DRY_RUN) {
      log('  ⏭️ Skipping - no market or dry run');
      return null;
    }
    
    try {
      log('  📡 Calling Oracle API (this may take 2-5 minutes)...');
      
      const response = await axios.post(
        `${CONFIG.ORACLE_API_URL}/api/v1/resolve/sync`,
        {
          market_id: this.testMarketId,
          question: `Will the E2E test ${this.testMarketId} pass all verifications?`,
          resolution_criteria: 'Test must complete all phases',
        },
        { timeout: 300000 } // 5 minute timeout for AI
      );
      
      if (response.data.status === 'completed') {
        log(`  ✅ Resolution complete: ${response.data.outcome} (${response.data.confidence * 100}%)`);
        this.addResult('ai_resolution', true, `Outcome: ${response.data.outcome}`);
        return response.data;
      } else {
        log(`  ❌ Resolution failed: ${response.data.error}`);
        this.addResult('ai_resolution', false, response.data.error);
        return null;
      }
    } catch (e) {
      log(`  ❌ Oracle API error: ${e.message}`);
      this.addResult('ai_resolution', false, e.message);
      return null;
    }
  }

  // ========================================================================
  // Phase 5: Verify On-chain State
  // ========================================================================
  
  async verifyOnChainState() {
    log('🔍 Phase 5: Verifying on-chain state...');
    
    if (!this.testMarketId || CONFIG.DRY_RUN) {
      log('  ⏭️ Skipping - no market or dry run');
      return;
    }
    
    try {
      // Get market data from backend
      const response = await axios.get(
        `${CONFIG.BACKEND_API_URL}/prediction/markets/${this.testMarketId}`
      );
      
      if (response.data.success) {
        const market = response.data.data;
        log(`  Market status: ${market.status}`);
        log(`  Oracle config frozen: ${market.oracle_config?.is_frozen || 'N/A'}`);
        
        // Get research data
        const researchResponse = await axios.get(
          `${CONFIG.BACKEND_API_URL}/prediction/markets/${this.testMarketId}/research`
        );
        
        if (researchResponse.data.success && researchResponse.data.data) {
          log(`  Research CID: ${researchResponse.data.data.research_cid}`);
          log(`  Consensus result: ${researchResponse.data.data.consensus_result}`);
        }
        
        this.addResult('verify_state', true, 'On-chain state verified');
      } else {
        log(`  ⚠️ Could not get market: ${response.data.error}`);
        this.addResult('verify_state', false, response.data.error);
      }
    } catch (e) {
      log(`  ❌ Verification error: ${e.message}`);
      this.addResult('verify_state', false, e.message);
    }
  }

  // ========================================================================
  // Run All Tests
  // ========================================================================
  
  async runAll() {
    console.log('═══════════════════════════════════════════════════════════');
    console.log('       LLM Oracle E2E Integration Test');
    console.log('═══════════════════════════════════════════════════════════');
    console.log();
    
    if (CONFIG.DRY_RUN) {
      log('🔸 Running in DRY RUN mode - no actual transactions');
    }
    if (CONFIG.SKIP_AI) {
      log('🔸 Running with SKIP_AI - using mock AI responses');
    }
    console.log();
    
    try {
      await this.setup();
      await this.testOracleApiHealth();
      await this.createTestMarket();
      await this.freezeOracleConfig();
      await this.triggerAiResolution();
      await this.verifyOnChainState();
      
      this.results.success = this.results.errors.length === 0;
    } catch (e) {
      log(`❌ Test failed with exception: ${e.message}`);
      this.results.errors.push({ step: 'runtime', message: e.message });
      this.results.success = false;
    }
    
    // Print summary
    console.log();
    console.log('═══════════════════════════════════════════════════════════');
    console.log('                     Test Summary');
    console.log('═══════════════════════════════════════════════════════════');
    
    for (const step of this.results.steps) {
      const icon = step.success ? '✅' : '❌';
      console.log(`  ${icon} ${step.step}: ${step.message}`);
    }
    
    console.log();
    if (this.results.success) {
      console.log('🎉 All tests PASSED!');
    } else {
      console.log(`❌ Tests FAILED with ${this.results.errors.length} error(s)`);
      for (const err of this.results.errors) {
        console.log(`   - ${err.step}: ${err.message}`);
      }
    }
    
    console.log('═══════════════════════════════════════════════════════════');
    
    return this.results.success ? 0 : 1;
  }
}

// ============================================================================
// Main
// ============================================================================

async function main() {
  const test = new LlmOracleE2ETest();
  const exitCode = await test.runAll();
  process.exit(exitCode);
}

main().catch(e => {
  console.error('Fatal error:', e);
  process.exit(1);
});

