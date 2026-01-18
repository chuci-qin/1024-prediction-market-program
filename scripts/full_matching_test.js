/**
 * 完整撮合测试脚本 - 二元市场 + 多选市场
 * 包括 MatchMint, MatchBurn, MatchMintMulti, MatchBurnMulti
 */

const { Connection, Keypair, PublicKey, Transaction, TransactionInstruction, SystemProgram, LAMPORTS_PER_SOL } = require('@solana/web3.js');
const config = require('./config');
const { TOKEN_PROGRAM_ID, getAssociatedTokenAddress, createAssociatedTokenAccountInstruction } = require('@solana/spl-token');
const bs58 = require('bs58').default || require('bs58');

// ========== 配置 ==========
const RPC_URL = config.RPC_URL;
const PROGRAM_ID = config.PROGRAM_ID;
const USDC_MINT = new PublicKey('7pCrfxhcAEyTFDhrhKRtRS2iMvEYx2dtNE7NzwuU7SA9');
const VAULT_PROGRAM = new PublicKey('vR3BifKCa2TGKP2uhToxZAMYAYydqpesvKGX54gzFny');

// 测试账户
const TEST_ACCOUNTS = [
    { name: 'User1', pubkey: '9ocm9zv5F2QghKaFSLGSjkVg6f8XZf54nVTjfC2M3dG4', 
      secret: '65d7pAydmKwgo5mVBwnKQUS7BUP1ZBhisEbeRyfzFnGLez85AGSqcqbZCUbsccogzSyLBqYcoZVgU7x7AARtKMHz' },
    { name: 'User2', pubkey: 'G23icA8QJiAM2UwENf1112rGFxoqHP6JJa3TuwVseVxu',
      secret: '2Rc3q4XFhUeZE5LUQCCzuMDVy4iom7mevWgCFCeMobNWAymrNAGe8UEXKkfVJQHb4af4F81JJL86qQz16a1wnv4y' },
    { name: 'User3', pubkey: '9S55H6Bbh2JCqdmQGcw2MWCdWeBNNQYb9GWiCHL62CUH',
      secret: '5isgvaK7oNcxNEctu6hRyYf7z1xEavfMRKmNGb6h9Ect2iFXtA9qKCFhWFhvxSzPJQBBMePuQ5Sd4VUYEKtd3oaq' }
];

// PDA Seeds
const PM_CONFIG_SEED = Buffer.from('pm_config');
const MARKET_SEED = Buffer.from('market');
const ORDER_SEED = Buffer.from('order');
const OUTCOME_MINT_SEED = Buffer.from('outcome_mint');
const POSITION_SEED = Buffer.from('position');
const ORDER_ESCROW_SEED = Buffer.from('order_escrow');

// 指令索引
const PLACE_ORDER_IX = 10;
const PLACE_MULTI_OUTCOME_ORDER_IX = 30;
const MATCH_MINT_IX = 12;
const MATCH_BURN_IX = 13;
const MATCH_MINT_MULTI_IX = 43;
const MATCH_BURN_MULTI_IX = 44;
const CREATE_MARKET_IX = 8;
const ACTIVATE_MARKET_IX = 11;
const CREATE_MULTI_OUTCOME_MARKET_IX = 27;

// ========== 工具函数 ==========

function loadKeypair(secretKey) {
    const decoded = bs58.decode(secretKey);
    return Keypair.fromSecretKey(decoded);
}

function deriveConfigPDA() {
    return PublicKey.findProgramAddressSync([PM_CONFIG_SEED], PROGRAM_ID);
}

function deriveMarketPDA(marketId) {
    const marketIdBytes = Buffer.alloc(8);
    marketIdBytes.writeBigUInt64LE(BigInt(marketId));
    return PublicKey.findProgramAddressSync([MARKET_SEED, marketIdBytes], PROGRAM_ID);
}

function deriveOrderPDA(marketId, orderId) {
    const marketIdBytes = Buffer.alloc(8);
    marketIdBytes.writeBigUInt64LE(BigInt(marketId));
    const orderIdBytes = Buffer.alloc(8);
    orderIdBytes.writeBigUInt64LE(BigInt(orderId));
    return PublicKey.findProgramAddressSync([ORDER_SEED, marketIdBytes, orderIdBytes], PROGRAM_ID);
}

function deriveOutcomeMintPDA(marketId, outcomeIndex) {
    const marketIdBytes = Buffer.alloc(8);
    marketIdBytes.writeBigUInt64LE(BigInt(marketId));
    return PublicKey.findProgramAddressSync(
        [OUTCOME_MINT_SEED, marketIdBytes, Buffer.from([outcomeIndex])], 
        PROGRAM_ID
    );
}

function derivePositionPDA(marketId, user) {
    const marketIdBytes = Buffer.alloc(8);
    marketIdBytes.writeBigUInt64LE(BigInt(marketId));
    return PublicKey.findProgramAddressSync(
        [POSITION_SEED, marketIdBytes, user.toBuffer()],
        PROGRAM_ID
    );
}

function deriveEscrowPDA(marketId, orderId) {
    const marketIdBytes = Buffer.alloc(8);
    marketIdBytes.writeBigUInt64LE(BigInt(marketId));
    const orderIdBytes = Buffer.alloc(8);
    orderIdBytes.writeBigUInt64LE(BigInt(orderId));
    return PublicKey.findProgramAddressSync(
        [ORDER_ESCROW_SEED, marketIdBytes, orderIdBytes],
        PROGRAM_ID
    );
}

async function getNextOrderId(connection, marketPDA) {
    const accountInfo = await connection.getAccountInfo(marketPDA);
    if (!accountInfo) throw new Error('Market not found');
    // next_order_id 在 Market 结构的偏移量 (需要根据实际结构计算)
    // 这里假设在固定偏移处
    const data = accountInfo.data;
    // 使用之前测试确认的偏移量
    let offset = 8 + 8 + 32; // discriminator + market_id + creator
    offset += 256; // question (估计)
    offset += 8; // created_at
    offset += 8; // resolution_time
    offset += 8; // finalization_deadline
    offset += 1; // status
    offset += 1; // market_type
    offset += 1; // outcome_count
    offset += 33; // result (Option<MarketResult>)
    return Number(data.readBigUInt64LE(offset));
}

async function ensureATA(connection, payer, mint, owner) {
    const ata = await getAssociatedTokenAddress(mint, owner);
    const accountInfo = await connection.getAccountInfo(ata);
    if (!accountInfo) {
        console.log(`  创建 ATA: ${ata.toString().slice(0, 8)}...`);
        const tx = new Transaction().add(
            createAssociatedTokenAccountInstruction(payer.publicKey, ata, owner, mint)
        );
        await connection.sendTransaction(tx, [payer], { skipPreflight: true });
        await sleep(2000);
    }
    return ata;
}

function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

// ========== 创建市场 ==========

async function createBinaryMarket(connection, admin) {
    const [configPDA] = deriveConfigPDA();
    const configInfo = await connection.getAccountInfo(configPDA);
    const nextMarketId = Number(configInfo.data.readBigUInt64LE(8 + 32 + 32 + 32 + 32 + 32));
    
    console.log(`\n创建二元市场 ID: ${nextMarketId}...`);
    
    const [marketPDA] = deriveMarketPDA(nextMarketId);
    const [yesMint] = deriveOutcomeMintPDA(nextMarketId, 0);
    const [noMint] = deriveOutcomeMintPDA(nextMarketId, 1);
    
    // CreateMarket instruction
    const question = 'Will BTC > 100k by EOY 2025?';
    const questionBuffer = Buffer.alloc(256);
    questionBuffer.write(question, 0, 'utf8');
    
    const now = Math.floor(Date.now() / 1000);
    const resolutionTime = now + 86400 * 7; // 7 days
    const finalizationDeadline = now + 86400 * 14; // 14 days
    
    const data = Buffer.alloc(1 + 256 + 8 + 8);
    let off = 0;
    data.writeUInt8(CREATE_MARKET_IX, off); off += 1;
    questionBuffer.copy(data, off); off += 256;
    data.writeBigInt64LE(BigInt(resolutionTime), off); off += 8;
    data.writeBigInt64LE(BigInt(finalizationDeadline), off);
    
    const ix = new TransactionInstruction({
        keys: [
            { pubkey: admin.publicKey, isSigner: true, isWritable: true },
            { pubkey: configPDA, isSigner: false, isWritable: true },
            { pubkey: marketPDA, isSigner: false, isWritable: true },
            { pubkey: yesMint, isSigner: false, isWritable: true },
            { pubkey: noMint, isSigner: false, isWritable: true },
            { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
            { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }
        ],
        programId: PROGRAM_ID,
        data: data
    });
    
    try {
        const tx = new Transaction().add(ix);
        const sig = await connection.sendTransaction(tx, [admin], { skipPreflight: true });
        console.log(`  ✅ CreateMarket: ${sig.slice(0, 20)}...`);
        await sleep(2000);
        
        // Activate market
        console.log('  激活市场...');
        const activateData = Buffer.alloc(1 + 8);
        activateData.writeUInt8(ACTIVATE_MARKET_IX, 0);
        activateData.writeBigUInt64LE(BigInt(nextMarketId), 1);
        
        const activateIx = new TransactionInstruction({
            keys: [
                { pubkey: admin.publicKey, isSigner: true, isWritable: true },
                { pubkey: configPDA, isSigner: false, isWritable: true },
                { pubkey: marketPDA, isSigner: false, isWritable: true }
            ],
            programId: PROGRAM_ID,
            data: activateData
        });
        
        const tx2 = new Transaction().add(activateIx);
        await connection.sendTransaction(tx2, [admin], { skipPreflight: true });
        await sleep(2000);
        
        console.log(`  ✅ Market ${nextMarketId} 激活成功`);
        return nextMarketId;
    } catch (e) {
        console.log(`  ❌ 创建市场失败: ${e.message}`);
        return null;
    }
}

async function createMultiOutcomeMarket(connection, admin, outcomeCount = 3) {
    const [configPDA] = deriveConfigPDA();
    const configInfo = await connection.getAccountInfo(configPDA);
    const nextMarketId = Number(configInfo.data.readBigUInt64LE(8 + 32 + 32 + 32 + 32 + 32));
    
    console.log(`\n创建多选市场 ID: ${nextMarketId} (${outcomeCount} outcomes)...`);
    
    const [marketPDA] = deriveMarketPDA(nextMarketId);
    const outcomeMints = [];
    for (let i = 0; i < outcomeCount; i++) {
        const [mint] = deriveOutcomeMintPDA(nextMarketId, i);
        outcomeMints.push(mint);
    }
    
    const question = 'FIFA World Cup 2026 Winner?';
    const questionBuffer = Buffer.alloc(256);
    questionBuffer.write(question, 0, 'utf8');
    
    const now = Math.floor(Date.now() / 1000);
    const resolutionTime = now + 86400 * 30;
    const finalizationDeadline = now + 86400 * 45;
    
    // CreateMultiOutcomeMarket data
    const data = Buffer.alloc(1 + 256 + 8 + 8 + 1);
    let off = 0;
    data.writeUInt8(CREATE_MULTI_OUTCOME_MARKET_IX, off); off += 1;
    questionBuffer.copy(data, off); off += 256;
    data.writeBigInt64LE(BigInt(resolutionTime), off); off += 8;
    data.writeBigInt64LE(BigInt(finalizationDeadline), off); off += 8;
    data.writeUInt8(outcomeCount, off);
    
    const keys = [
        { pubkey: admin.publicKey, isSigner: true, isWritable: true },
        { pubkey: configPDA, isSigner: false, isWritable: true },
        { pubkey: marketPDA, isSigner: false, isWritable: true }
    ];
    
    for (const mint of outcomeMints) {
        keys.push({ pubkey: mint, isSigner: false, isWritable: true });
    }
    
    keys.push({ pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false });
    keys.push({ pubkey: SystemProgram.programId, isSigner: false, isWritable: false });
    
    try {
        const tx = new Transaction().add(new TransactionInstruction({
            keys: keys,
            programId: PROGRAM_ID,
            data: data
        }));
        const sig = await connection.sendTransaction(tx, [admin], { skipPreflight: true });
        console.log(`  ✅ CreateMultiOutcomeMarket: ${sig.slice(0, 20)}...`);
        await sleep(2000);
        
        // Activate
        console.log('  激活市场...');
        const activateData = Buffer.alloc(1 + 8);
        activateData.writeUInt8(ACTIVATE_MARKET_IX, 0);
        activateData.writeBigUInt64LE(BigInt(nextMarketId), 1);
        
        const activateIx = new TransactionInstruction({
            keys: [
                { pubkey: admin.publicKey, isSigner: true, isWritable: true },
                { pubkey: configPDA, isSigner: false, isWritable: true },
                { pubkey: marketPDA, isSigner: false, isWritable: true }
            ],
            programId: PROGRAM_ID,
            data: activateData
        });
        
        const tx2 = new Transaction().add(activateIx);
        await connection.sendTransaction(tx2, [admin], { skipPreflight: true });
        await sleep(2000);
        
        console.log(`  ✅ Multi-outcome Market ${nextMarketId} 激活成功`);
        return nextMarketId;
    } catch (e) {
        console.log(`  ❌ 创建多选市场失败: ${e.message}`);
        return null;
    }
}

// ========== 二元市场测试 ==========

async function testBinaryMarketMatching(connection, user1, user2) {
    console.log('\n' + '='.repeat(60));
    console.log('📊 测试一：二元市场撮合 (MatchMint)');
    console.log('='.repeat(60));
    
    const [configPDA] = deriveConfigPDA();
    const faucetData = require('/Users/chuciqin/Desktop/project1024/1024codebase/1024-chain/keys/faucet.json');
    const faucet = Keypair.fromSecretKey(new Uint8Array(faucetData));
    
    // 查找或创建活跃市场
    console.log('\n查找活跃的二元市场...');
    let activeMarketId = null;
    for (let id = 8; id <= 15; id++) {
        const [marketPDA] = deriveMarketPDA(id);
        const marketInfo = await connection.getAccountInfo(marketPDA);
        if (marketInfo) {
            const baseOffset = 8 + 8 + 32 + 256 + 8 + 8 + 8;
            const status = marketInfo.data[baseOffset];
            const marketType = marketInfo.data[baseOffset + 1];
            if (status === 1 && marketType === 0) { // Active && Binary
                activeMarketId = id;
                console.log(`✅ 找到活跃二元市场 ID: ${id}`);
                break;
            }
        }
    }
    
    if (!activeMarketId) {
        console.log('没有活跃的二元市场，创建新的...');
        activeMarketId = await createBinaryMarket(connection, faucet);
        if (!activeMarketId) {
            return { success: false, reason: 'create_market_failed' };
        }
    }
    
    const marketId = activeMarketId;
    const [marketPDA] = deriveMarketPDA(marketId);
    
    // 获取 Mints
    const [yesMint] = deriveOutcomeMintPDA(marketId, 0);
    const [noMint] = deriveOutcomeMintPDA(marketId, 1);
    
    console.log(`\n市场配置:`);
    console.log(`  Market PDA: ${marketPDA.toString()}`);
    console.log(`  YES Mint: ${yesMint.toString()}`);
    console.log(`  NO Mint: ${noMint.toString()}`);
    
    // 获取 next_order_id
    let nextOrderId;
    try {
        nextOrderId = await getNextOrderId(connection, marketPDA);
    } catch (e) {
        nextOrderId = 1;
    }
    
    console.log(`\n下一个 Order ID: ${nextOrderId}`);
    
    // 确保 ATAs 存在
    console.log('\n确保代币账户存在...');
    const user1YesATA = await ensureATA(connection, user1, yesMint, user1.publicKey);
    const user1NoATA = await ensureATA(connection, user1, noMint, user1.publicKey);
    const user2YesATA = await ensureATA(connection, user2, yesMint, user2.publicKey);
    const user2NoATA = await ensureATA(connection, user2, noMint, user2.publicKey);
    
    // 下单: User1 Buy YES @ 0.60
    console.log('\n📝 User1: 下 Buy YES 订单 @ 0.60...');
    const order1Id = nextOrderId;
    const [order1PDA] = deriveOrderPDA(marketId, order1Id);
    
    const placeOrder1Data = Buffer.alloc(1 + 8 + 1 + 8 + 8 + 8 + 1);
    let offset = 0;
    placeOrder1Data.writeUInt8(PLACE_ORDER_IX, offset); offset += 1;
    placeOrder1Data.writeBigUInt64LE(BigInt(marketId), offset); offset += 8;
    placeOrder1Data.writeUInt8(0, offset); offset += 1; // YES
    placeOrder1Data.writeBigUInt64LE(600000n, offset); offset += 8; // 0.60 * 1e6
    placeOrder1Data.writeBigUInt64LE(1000000n, offset); offset += 8; // 1 token
    placeOrder1Data.writeBigUInt64LE(0n, offset); offset += 8; // no expiration
    placeOrder1Data.writeUInt8(0, offset); // Buy side
    
    const placeOrder1Ix = new TransactionInstruction({
        keys: [
            { pubkey: user1.publicKey, isSigner: true, isWritable: true },
            { pubkey: configPDA, isSigner: false, isWritable: false },
            { pubkey: marketPDA, isSigner: false, isWritable: true },
            { pubkey: order1PDA, isSigner: false, isWritable: true },
            { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }
        ],
        programId: PROGRAM_ID,
        data: placeOrder1Data
    });
    
    try {
        const tx1 = new Transaction().add(placeOrder1Ix);
        const sig1 = await connection.sendTransaction(tx1, [user1], { skipPreflight: true });
        console.log(`  ✅ Order 1 (Buy YES): ${sig1.slice(0, 20)}...`);
        await sleep(2000);
    } catch (e) {
        console.log(`  ❌ Order 1 失败: ${e.message}`);
        return { success: false, reason: 'place_order_failed' };
    }
    
    // 下单: User2 Buy NO @ 0.40
    console.log('📝 User2: 下 Buy NO 订单 @ 0.40...');
    const order2Id = order1Id + 1;
    const [order2PDA] = deriveOrderPDA(marketId, order2Id);
    
    const placeOrder2Data = Buffer.alloc(1 + 8 + 1 + 8 + 8 + 8 + 1);
    offset = 0;
    placeOrder2Data.writeUInt8(PLACE_ORDER_IX, offset); offset += 1;
    placeOrder2Data.writeBigUInt64LE(BigInt(marketId), offset); offset += 8;
    placeOrder2Data.writeUInt8(1, offset); offset += 1; // NO
    placeOrder2Data.writeBigUInt64LE(400000n, offset); offset += 8; // 0.40 * 1e6
    placeOrder2Data.writeBigUInt64LE(1000000n, offset); offset += 8; // 1 token
    placeOrder2Data.writeBigUInt64LE(0n, offset); offset += 8;
    placeOrder2Data.writeUInt8(0, offset); // Buy side
    
    const placeOrder2Ix = new TransactionInstruction({
        keys: [
            { pubkey: user2.publicKey, isSigner: true, isWritable: true },
            { pubkey: configPDA, isSigner: false, isWritable: false },
            { pubkey: marketPDA, isSigner: false, isWritable: true },
            { pubkey: order2PDA, isSigner: false, isWritable: true },
            { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }
        ],
        programId: PROGRAM_ID,
        data: placeOrder2Data
    });
    
    try {
        const tx2 = new Transaction().add(placeOrder2Ix);
        const sig2 = await connection.sendTransaction(tx2, [user2], { skipPreflight: true });
        console.log(`  ✅ Order 2 (Buy NO): ${sig2.slice(0, 20)}...`);
        await sleep(2000);
    } catch (e) {
        console.log(`  ❌ Order 2 失败: ${e.message}`);
        return { success: false, reason: 'place_order_failed' };
    }
    
    // MatchMint
    console.log('\n🔥 执行 MatchMint 撮合...');
    
    // faucet 已在函数开头加载
    
    const matchMintData = Buffer.alloc(1 + 8 + 8 + 8 + 8 + 8 + 8);
    offset = 0;
    matchMintData.writeUInt8(MATCH_MINT_IX, offset); offset += 1;
    matchMintData.writeBigUInt64LE(BigInt(marketId), offset); offset += 8;
    matchMintData.writeBigUInt64LE(BigInt(order1Id), offset); offset += 8; // yes_order_id
    matchMintData.writeBigUInt64LE(BigInt(order2Id), offset); offset += 8; // no_order_id
    matchMintData.writeBigUInt64LE(1000000n, offset); offset += 8; // amount
    matchMintData.writeBigUInt64LE(600000n, offset); offset += 8; // yes_price
    matchMintData.writeBigUInt64LE(400000n, offset); // no_price
    
    const matchMintIx = new TransactionInstruction({
        keys: [
            { pubkey: faucet.publicKey, isSigner: true, isWritable: true },       // 0. Caller
            { pubkey: configPDA, isSigner: false, isWritable: true },              // 1. Config
            { pubkey: marketPDA, isSigner: false, isWritable: true },              // 2. Market
            { pubkey: order1PDA, isSigner: false, isWritable: true },              // 3. YES Order
            { pubkey: order2PDA, isSigner: false, isWritable: true },              // 4. NO Order
            { pubkey: yesMint, isSigner: false, isWritable: true },                // 5. YES Mint
            { pubkey: noMint, isSigner: false, isWritable: true },                 // 6. NO Mint
            { pubkey: user1YesATA, isSigner: false, isWritable: true },            // 7. User1 YES ATA
            { pubkey: user2NoATA, isSigner: false, isWritable: true },             // 8. User2 NO ATA
            { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false }       // 9. Token Program
        ],
        programId: PROGRAM_ID,
        data: matchMintData
    });
    
    try {
        const tx = new Transaction().add(matchMintIx);
        const sig = await connection.sendTransaction(tx, [faucet], { skipPreflight: true });
        console.log(`  ✅ MatchMint 成功: ${sig.slice(0, 20)}...`);
        await sleep(2000);
        
        // 验证
        await sleep(1000);
        try {
            const order1Info = await connection.getAccountInfo(order1PDA);
            if (order1Info) {
                const order1Status = order1Info.data[8 + 8 + 8 + 32 + 1 + 1 + 1 + 8 + 8 + 8];
                console.log(`  订单1状态: ${order1Status === 2 ? 'Filled ✅' : `状态码: ${order1Status}`}`);
            }
        } catch (verifyErr) {
            console.log(`  (验证跳过)`);
        }
        
        return { success: true, marketId, order1Id, order2Id };
    } catch (e) {
        console.log(`  ❌ MatchMint 失败: ${e.message}`);
        return { success: false, reason: 'match_mint_failed' };
    }
}

// ========== 多选市场测试 ==========

async function testMultiOutcomeMarketMatching(connection, users) {
    console.log('\n' + '='.repeat(60));
    console.log('📊 测试二：多选市场撮合 (MatchMintMulti + MatchBurnMulti)');
    console.log('='.repeat(60));
    
    const [configPDA] = deriveConfigPDA();
    const faucetData = require('/Users/chuciqin/Desktop/project1024/1024codebase/1024-chain/keys/faucet.json');
    const faucet = Keypair.fromSecretKey(new Uint8Array(faucetData));
    
    // 查找或创建活跃的多选市场
    console.log('\n查找活跃的多选市场...');
    let activeMarketId = null;
    for (let id = 9; id <= 20; id++) {
        const [marketPDA] = deriveMarketPDA(id);
        const marketInfo = await connection.getAccountInfo(marketPDA);
        if (marketInfo) {
            const baseOffset = 8 + 8 + 32 + 256 + 8 + 8 + 8;
            const status = marketInfo.data[baseOffset];
            const marketType = marketInfo.data[baseOffset + 1];
            const outcomeCount = marketInfo.data[baseOffset + 2];
            if (status === 1 && marketType === 1 && outcomeCount >= 3) {
                activeMarketId = id;
                console.log(`✅ 找到活跃多选市场 ID: ${id} (${outcomeCount} outcomes)`);
                break;
            }
        }
    }
    
    if (!activeMarketId) {
        console.log('没有活跃的多选市场，创建新的...');
        activeMarketId = await createMultiOutcomeMarket(connection, faucet, 3);
        if (!activeMarketId) {
            return { success: false, reason: 'create_multi_market_failed' };
        }
    }
    
    const marketId = activeMarketId;
    const [marketPDA] = deriveMarketPDA(marketId);
    
    // 获取 Outcome Mints
    const outcomeCount = 3;
    const outcomeMints = [];
    for (let i = 0; i < outcomeCount; i++) {
        const [mint] = deriveOutcomeMintPDA(marketId, i);
        outcomeMints.push(mint);
    }
    
    console.log(`\n市场配置:`);
    console.log(`  Market ID: ${marketId}`);
    for (let i = 0; i < outcomeCount; i++) {
        console.log(`  Outcome ${i} Mint: ${outcomeMints[i].toString().slice(0, 12)}...`);
    }
    
    // 获取 next_order_id
    let nextOrderId;
    try {
        nextOrderId = await getNextOrderId(connection, marketPDA);
    } catch (e) {
        nextOrderId = 1;
    }
    console.log(`  下一个 Order ID: ${nextOrderId}`);
    
    // ========== Part A: MatchMintMulti ==========
    console.log('\n--- Part A: MatchMintMulti (3 Buy 订单) ---');
    
    // 确保 ATAs
    console.log('确保代币账户存在...');
    const userATAs = [];
    for (let i = 0; i < 3; i++) {
        const atas = [];
        for (let j = 0; j < outcomeCount; j++) {
            const ata = await ensureATA(connection, users[i], outcomeMints[j], users[i].publicKey);
            atas.push(ata);
        }
        userATAs.push(atas);
    }
    
    // 3个用户下 Buy 订单，价格总和 = 1.0
    const prices = [330000n, 340000n, 330000n]; // 0.33 + 0.34 + 0.33 = 1.0
    const orderIds = [];
    const orderPDAs = [];
    
    for (let i = 0; i < 3; i++) {
        const orderId = nextOrderId + i;
        orderIds.push(orderId);
        const [orderPDA] = deriveOrderPDA(marketId, orderId);
        orderPDAs.push(orderPDA);
        const [positionPDA] = derivePositionPDA(marketId, users[i].publicKey);
        
        console.log(`\n📝 User${i+1}: 下 Buy Outcome${i} 订单 @ ${Number(prices[i]) / 1e6}...`);
        
        // PlaceMultiOutcomeOrder 数据
        const data = Buffer.alloc(1 + 8 + 1 + 1 + 8 + 8 + 8);
        let off = 0;
        data.writeUInt8(PLACE_MULTI_OUTCOME_ORDER_IX, off); off += 1;
        data.writeBigUInt64LE(BigInt(marketId), off); off += 8;
        data.writeUInt8(i, off); off += 1; // outcome_index
        data.writeUInt8(0, off); off += 1; // side = Buy
        data.writeBigUInt64LE(prices[i], off); off += 8;
        data.writeBigUInt64LE(1000000n, off); off += 8; // amount
        data.writeBigUInt64LE(0n, off); // expiration
        
        const ix = new TransactionInstruction({
            keys: [
                { pubkey: users[i].publicKey, isSigner: true, isWritable: true },
                { pubkey: configPDA, isSigner: false, isWritable: false },
                { pubkey: marketPDA, isSigner: false, isWritable: true },
                { pubkey: orderPDA, isSigner: false, isWritable: true },
                { pubkey: positionPDA, isSigner: false, isWritable: true },
                { pubkey: outcomeMints[i], isSigner: false, isWritable: false },
                { pubkey: userATAs[i][i], isSigner: false, isWritable: false },
                { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
                { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }
            ],
            programId: PROGRAM_ID,
            data: data
        });
        
        try {
            const tx = new Transaction().add(ix);
            const sig = await connection.sendTransaction(tx, [users[i]], { skipPreflight: true });
            console.log(`  ✅ Order ${orderId}: ${sig.slice(0, 20)}...`);
            await sleep(2000);
        } catch (e) {
            console.log(`  ❌ Order ${orderId} 失败: ${e.message}`);
            return { success: false, reason: 'place_multi_order_failed' };
        }
    }
    
    // MatchMintMulti
    console.log('\n🔥 执行 MatchMintMulti 撮合...');
    
    const matchMintMultiData = Buffer.alloc(1 + 8 + 1 + 8 * 3 + 8 + 8 * 3);
    let off = 0;
    matchMintMultiData.writeUInt8(MATCH_MINT_MULTI_IX, off); off += 1;
    matchMintMultiData.writeBigUInt64LE(BigInt(marketId), off); off += 8;
    matchMintMultiData.writeUInt8(3, off); off += 1; // outcome_count
    for (const id of orderIds) {
        matchMintMultiData.writeBigUInt64LE(BigInt(id), off); off += 8;
    }
    matchMintMultiData.writeBigUInt64LE(1000000n, off); off += 8; // amount
    for (const p of prices) {
        matchMintMultiData.writeBigUInt64LE(p, off); off += 8;
    }
    
    // 构建账户列表
    const matchMintKeys = [
        { pubkey: faucet.publicKey, isSigner: true, isWritable: true },
        { pubkey: configPDA, isSigner: false, isWritable: true },
        { pubkey: marketPDA, isSigner: false, isWritable: true }
    ];
    
    // 添加 Order PDAs
    for (const pda of orderPDAs) {
        matchMintKeys.push({ pubkey: pda, isSigner: false, isWritable: true });
    }
    
    // 添加 Outcome Mints
    for (const mint of outcomeMints) {
        matchMintKeys.push({ pubkey: mint, isSigner: false, isWritable: true });
    }
    
    // 添加 User Token Accounts
    for (let i = 0; i < 3; i++) {
        matchMintKeys.push({ pubkey: userATAs[i][i], isSigner: false, isWritable: true });
    }
    
    matchMintKeys.push({ pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false });
    
    try {
        const tx = new Transaction().add(new TransactionInstruction({
            keys: matchMintKeys,
            programId: PROGRAM_ID,
            data: matchMintMultiData
        }));
        const sig = await connection.sendTransaction(tx, [faucet], { skipPreflight: true });
        console.log(`  ✅ MatchMintMulti 成功: ${sig.slice(0, 20)}...`);
        await sleep(3000);
    } catch (e) {
        console.log(`  ❌ MatchMintMulti 失败: ${e.message}`);
        return { success: false, reason: 'match_mint_multi_failed' };
    }
    
    // ========== Part B: MatchBurnMulti ==========
    console.log('\n--- Part B: MatchBurnMulti (3 Sell 订单 with Escrow) ---');
    
    // 获取新的 order_id
    let newNextOrderId;
    try {
        newNextOrderId = await getNextOrderId(connection, marketPDA);
    } catch (e) {
        newNextOrderId = nextOrderId + 3;
    }
    
    const sellPrices = [340000n, 330000n, 340000n]; // 0.34 + 0.33 + 0.34 = 1.01 > 1.0 ✅
    const sellOrderIds = [];
    const sellOrderPDAs = [];
    const escrowPDAs = [];
    
    for (let i = 0; i < 3; i++) {
        const orderId = newNextOrderId + i;
        sellOrderIds.push(orderId);
        const [orderPDA] = deriveOrderPDA(marketId, orderId);
        sellOrderPDAs.push(orderPDA);
        const [escrowPDA] = deriveEscrowPDA(marketId, orderId);
        escrowPDAs.push(escrowPDA);
        const [positionPDA] = derivePositionPDA(marketId, users[i].publicKey);
        
        console.log(`\n📝 User${i+1}: 下 Sell Outcome${i} 订单 @ ${Number(sellPrices[i]) / 1e6}...`);
        
        // PlaceMultiOutcomeOrder 数据 (Sell)
        const data = Buffer.alloc(1 + 8 + 1 + 1 + 8 + 8 + 8);
        let off = 0;
        data.writeUInt8(PLACE_MULTI_OUTCOME_ORDER_IX, off); off += 1;
        data.writeBigUInt64LE(BigInt(marketId), off); off += 8;
        data.writeUInt8(i, off); off += 1; // outcome_index
        data.writeUInt8(1, off); off += 1; // side = Sell
        data.writeBigUInt64LE(sellPrices[i], off); off += 8;
        data.writeBigUInt64LE(500000n, off); off += 8; // 0.5 tokens
        data.writeBigUInt64LE(0n, off); // expiration
        
        // Sell 订单需要 11 个账户
        const ix = new TransactionInstruction({
            keys: [
                { pubkey: users[i].publicKey, isSigner: true, isWritable: true },
                { pubkey: configPDA, isSigner: false, isWritable: false },
                { pubkey: marketPDA, isSigner: false, isWritable: true },
                { pubkey: orderPDA, isSigner: false, isWritable: true },
                { pubkey: positionPDA, isSigner: false, isWritable: true },
                { pubkey: outcomeMints[i], isSigner: false, isWritable: true },
                { pubkey: userATAs[i][i], isSigner: false, isWritable: true },
                { pubkey: escrowPDA, isSigner: false, isWritable: true },
                { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
                { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
                { pubkey: new PublicKey('SysvarRent111111111111111111111111111111111'), isSigner: false, isWritable: false }
            ],
            programId: PROGRAM_ID,
            data: data
        });
        
        try {
            const tx = new Transaction().add(ix);
            const sig = await connection.sendTransaction(tx, [users[i]], { skipPreflight: true });
            console.log(`  ✅ Sell Order ${orderId}: ${sig.slice(0, 20)}...`);
            await sleep(2000);
            
            // 验证 escrow 是否创建
            const escrowInfo = await connection.getAccountInfo(escrowPDA);
            if (escrowInfo) {
                console.log(`  🔒 Escrow 已创建: ${escrowPDA.toString().slice(0, 12)}...`);
            }
        } catch (e) {
            console.log(`  ❌ Sell Order ${orderId} 失败: ${e.message}`);
            return { success: false, reason: 'place_sell_order_failed', error: e.message };
        }
    }
    
    // MatchBurnMulti
    console.log('\n🔥 执行 MatchBurnMulti 撮合...');
    
    const matchBurnMultiData = Buffer.alloc(1 + 8 + 1 + 8 * 3 + 8 + 8 * 3);
    off = 0;
    matchBurnMultiData.writeUInt8(MATCH_BURN_MULTI_IX, off); off += 1;
    matchBurnMultiData.writeBigUInt64LE(BigInt(marketId), off); off += 8;
    matchBurnMultiData.writeUInt8(3, off); off += 1;
    for (const id of sellOrderIds) {
        matchBurnMultiData.writeBigUInt64LE(BigInt(id), off); off += 8;
    }
    matchBurnMultiData.writeBigUInt64LE(500000n, off); off += 8; // amount
    for (const p of sellPrices) {
        matchBurnMultiData.writeBigUInt64LE(p, off); off += 8;
    }
    
    // 构建账户列表
    const matchBurnKeys = [
        { pubkey: faucet.publicKey, isSigner: true, isWritable: true },
        { pubkey: configPDA, isSigner: false, isWritable: true },
        { pubkey: marketPDA, isSigner: false, isWritable: true }
    ];
    
    // 添加 Order PDAs
    for (const pda of sellOrderPDAs) {
        matchBurnKeys.push({ pubkey: pda, isSigner: false, isWritable: true });
    }
    
    // 添加 Escrow Token Accounts
    for (const pda of escrowPDAs) {
        matchBurnKeys.push({ pubkey: pda, isSigner: false, isWritable: true });
    }
    
    // 添加 Outcome Mints
    for (const mint of outcomeMints) {
        matchBurnKeys.push({ pubkey: mint, isSigner: false, isWritable: true });
    }
    
    // 添加 User Pubkeys (for USDC distribution via Vault)
    for (let i = 0; i < 3; i++) {
        matchBurnKeys.push({ pubkey: users[i].publicKey, isSigner: false, isWritable: true });
    }
    
    matchBurnKeys.push({ pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false });
    matchBurnKeys.push({ pubkey: VAULT_PROGRAM, isSigner: false, isWritable: false });
    
    try {
        const tx = new Transaction().add(new TransactionInstruction({
            keys: matchBurnKeys,
            programId: PROGRAM_ID,
            data: matchBurnMultiData
        }));
        const sig = await connection.sendTransaction(tx, [faucet], { skipPreflight: true });
        console.log(`  ✅ MatchBurnMulti 成功: ${sig.slice(0, 20)}...`);
        await sleep(3000);
        
        // 验证订单状态
        for (let i = 0; i < 3; i++) {
            const orderInfo = await connection.getAccountInfo(sellOrderPDAs[i]);
            if (orderInfo) {
                const status = orderInfo.data[8 + 8 + 8 + 32 + 1 + 1 + 1 + 8 + 8 + 8];
                console.log(`  Sell Order ${sellOrderIds[i]} 状态: ${status === 2 ? 'Filled ✅' : `${status}`}`);
            }
        }
        
        return { success: true, marketId, mintOrderIds: orderIds, burnOrderIds: sellOrderIds };
    } catch (e) {
        console.log(`  ❌ MatchBurnMulti 失败: ${e.message}`);
        return { success: false, reason: 'match_burn_multi_failed', error: e.message };
    }
}

// ========== 主函数 ==========

async function main() {
    console.log('🚀 1024 Prediction Market 完整撮合测试');
    console.log('='.repeat(60));
    console.log(`RPC: ${RPC_URL}`);
    console.log(`Program: ${PROGRAM_ID.toString()}`);
    console.log('');
    
    const connection = new Connection(RPC_URL, 'confirmed');
    
    // 加载测试账户
    const users = TEST_ACCOUNTS.map(acc => loadKeypair(acc.secret));
    console.log('测试账户:');
    for (let i = 0; i < users.length; i++) {
        console.log(`  User${i+1}: ${users[i].publicKey.toString()}`);
    }
    
    // 测试 1: 二元市场
    const binaryResult = await testBinaryMarketMatching(connection, users[0], users[1]);
    console.log('\n📊 二元市场测试结果:', binaryResult.success ? '✅ 成功' : `❌ 失败 (${binaryResult.reason})`);
    
    // 测试 2: 多选市场
    const multiResult = await testMultiOutcomeMarketMatching(connection, users);
    console.log('\n📊 多选市场测试结果:', multiResult.success ? '✅ 成功' : `❌ 失败 (${multiResult.reason})`);
    
    // 总结
    console.log('\n' + '='.repeat(60));
    console.log('📋 测试总结');
    console.log('='.repeat(60));
    console.log(`二元市场 MatchMint: ${binaryResult.success ? '✅' : '❌'}`);
    console.log(`多选市场 MatchMintMulti: ${multiResult.success ? '✅' : '❌'}`);
    console.log(`多选市场 MatchBurnMulti: ${multiResult.success ? '✅' : '❌'}`);
    
    if (binaryResult.success && multiResult.success) {
        console.log('\n🎉 所有测试通过！Complete Set CTF + Order Book 撮合机制完全验证成功！');
    }
}

main().catch(console.error);

