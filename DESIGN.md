# 1024 Prediction Market Program - 设计文档

> 版本: v0.1  
> 创建日期: 2025-12-08

---

## 1. 概述

### 1.1 Program 架构

预测市场涉及三个 onchain program 的协作：

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Prediction Market Program 架构                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                     ┌─────────────────────────────────────┐                 │
│                     │   1024-prediction-market-program     │                 │
│                     │   (新建 - 核心业务逻辑)               │                 │
│                     ├─────────────────────────────────────┤                 │
│                     │ · Market 账户管理                     │                 │
│                     │ · Order 账户管理                      │                 │
│                     │ · Position 账户管理                   │                 │
│                     │ · YES/NO Token Mint                  │                 │
│                     │ · 撮合执行 (match_mint/match_burn)    │                 │
│                     │ · Oracle 结果接入                     │                 │
│                     │ · 结算逻辑                            │                 │
│                     └───────────────┬─────────────────────┘                 │
│                                     │                                       │
│                          CPI 调用   │                                       │
│                    ┌────────────────┼────────────────┐                     │
│                    │                │                │                     │
│                    ▼                ▼                ▼                     │
│         ┌─────────────────┐  ┌─────────────┐  ┌─────────────┐             │
│         │ 1024-vault-program│  │1024-fund-program│  │ SPL Token │             │
│         │ (修改 - 资金托管) │  │(修改 - 手续费) │  │  Program  │             │
│         ├─────────────────┤  ├─────────────┤  └─────────────┘             │
│         │ · 用户 USDC 托管  │  │ · 预测市场费池 │                            │
│         │ · 预测市场锁定    │  │ · 手续费分配   │                            │
│         │ · 结算出金        │  │ · 做市商奖励   │                            │
│         └─────────────────┘  └─────────────┘                              │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 1.2 与现有程序的集成

| Program | 当前用途 | 预测市场新增 |
|---------|---------|-------------|
| `1024-vault-program` | Perp 用户资金托管 | + 预测市场资金锁定/释放 |
| `1024-fund-program` | Perp 保险基金/手续费 | + 预测市场手续费池 |
| `1024-prediction-market-program` | (新建) | 预测市场核心逻辑 |

---

## 2. 1024-vault-program 修改

### 2.1 UserAccount 扩展

现有 `UserAccount` 需要添加预测市场专用字段：

```rust
/// 用户账户 (PDA)
/// Seeds: ["user", wallet.key()]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct UserAccount {
    // === 现有字段 (Perp) ===
    pub discriminator: u64,
    pub wallet: Pubkey,
    pub bump: u8,
    pub available_balance_e6: i64,
    pub locked_margin_e6: i64,           // Perp 锁定保证金
    pub unrealized_pnl_e6: i64,
    pub total_deposited_e6: i64,
    pub total_withdrawn_e6: i64,
    pub last_update_ts: i64,
    
    // === 新增字段 (Prediction Market) ===
    /// 预测市场锁定的 USDC (用于购买 YES/NO Token)
    pub pm_locked_e6: i64,
    /// 预测市场未结算收益 (已结算市场等待领取的 USDC)
    pub pm_pending_settlement_e6: i64,
    
    pub reserved: [u8; 48],  // 从 64 减少到 48 (新增了 16 bytes)
}

impl UserAccount {
    /// 计算总权益 (包含 Perp + Prediction Market)
    pub fn total_equity(&self) -> i64 {
        self.available_balance_e6 
            + self.locked_margin_e6 
            + self.unrealized_pnl_e6
            + self.pm_locked_e6
            + self.pm_pending_settlement_e6
    }
}
```

### 2.2 新增指令

```rust
/// === 预测市场相关指令 ===

/// 锁定 USDC 用于预测市场 (CPI only - 由 Prediction Market Program 调用)
/// 
/// 用户购买 YES/NO Token 时，从 available_balance 扣除并锁定
/// 
/// Accounts:
/// 0. `[]` VaultConfig
/// 1. `[writable]` UserAccount
/// 2. `[]` Caller Program (验证白名单)
LockForPrediction {
    amount: u64,
},

/// 释放预测市场锁定 (CPI only)
/// 
/// 用户卖出 YES/NO Token 或赎回完整集时释放
/// 
/// Accounts:
/// 0. `[]` VaultConfig
/// 1. `[writable]` UserAccount
/// 2. `[]` Caller Program
ReleaseFromPrediction {
    amount: u64,
},

/// 预测市场结算 (CPI only)
/// 
/// 市场结算后，将用户应得 USDC 记入 pm_pending_settlement
/// 
/// Accounts:
/// 0. `[]` VaultConfig
/// 1. `[writable]` UserAccount
/// 2. `[]` Caller Program
PredictionSettle {
    /// 用户原锁定金额 (将从 pm_locked 扣除)
    locked_amount: u64,
    /// 结算应得金额 (记入 pm_pending_settlement)
    settlement_amount: u64,
},

/// 领取预测市场结算收益
/// 
/// 将 pm_pending_settlement 转为 available_balance
/// 
/// Accounts:
/// 0. `[signer]` User
/// 1. `[writable]` UserAccount
ClaimPredictionSettlement,
```

---

## 3. 1024-fund-program 修改

### 3.1 新增 PredictionMarketFeeConfig

```rust
/// Discriminator for PredictionMarketFeeConfig account
pub const PM_FEE_CONFIG_DISCRIMINATOR: u64 = 0x504D5F464545_4346; // "PM_FEE_CF"

/// PDA Seed
pub const PM_FEE_CONFIG_SEED: &[u8] = b"pm_fee_config";

/// 预测市场手续费配置
/// 
/// PDA Seeds: ["pm_fee_config"]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct PredictionMarketFeeConfig {
    pub discriminator: u64,
    
    /// 关联的 Fund 账户 (手续费资金池)
    pub fee_fund: Pubkey,
    
    /// PDA bump
    pub bump: u8,
    
    // === 费率配置 (basis points) ===
    
    /// 铸造费率 (默认 10 = 0.1%)
    pub minting_fee_bps: u16,
    
    /// 赎回费率 (默认 10 = 0.1%)
    pub redemption_fee_bps: u16,
    
    /// 交易费率 (Taker, 默认 10 = 0.1%)
    pub trading_fee_taker_bps: u16,
    
    /// 交易费率 (Maker, 默认 0 = 0%)
    pub trading_fee_maker_bps: u16,
    
    /// 结算费率 (默认 0 = 0%)
    pub settlement_fee_bps: u16,
    
    // === 费用分配 (basis points, 总计 10000) ===
    
    /// 协议收入占比 (默认 7000 = 70%)
    pub protocol_share_bps: u16,
    
    /// 做市商奖励占比 (默认 2000 = 20%)
    pub maker_reward_share_bps: u16,
    
    /// 市场创建者占比 (默认 1000 = 10%)
    pub creator_share_bps: u16,
    
    // === 统计 ===
    
    /// 累计铸造费收入 (e6)
    pub total_minting_fee_e6: i64,
    
    /// 累计赎回费收入 (e6)
    pub total_redemption_fee_e6: i64,
    
    /// 累计交易费收入 (e6)
    pub total_trading_fee_e6: i64,
    
    /// 累计做市商奖励发放 (e6)
    pub total_maker_rewards_e6: i64,
    
    /// 累计市场创建者分成 (e6)
    pub total_creator_rewards_e6: i64,
    
    // === 授权 ===
    
    /// 授权调用方 (Prediction Market Program)
    pub authorized_caller: Pubkey,
    
    /// 最后更新时间戳
    pub last_update_ts: i64,
    
    pub reserved: [u8; 64],
}

impl PredictionMarketFeeConfig {
    pub const SIZE: usize = 8   // discriminator
        + 32  // fee_fund
        + 1   // bump
        + 2   // minting_fee_bps
        + 2   // redemption_fee_bps
        + 2   // trading_fee_taker_bps
        + 2   // trading_fee_maker_bps
        + 2   // settlement_fee_bps
        + 2   // protocol_share_bps
        + 2   // maker_reward_share_bps
        + 2   // creator_share_bps
        + 8   // total_minting_fee_e6
        + 8   // total_redemption_fee_e6
        + 8   // total_trading_fee_e6
        + 8   // total_maker_rewards_e6
        + 8   // total_creator_rewards_e6
        + 32  // authorized_caller
        + 8   // last_update_ts
        + 64; // reserved
}
```

### 3.2 新增指令

```rust
/// === 预测市场手续费相关指令 ===

/// 初始化预测市场手续费配置
/// 
/// Accounts:
/// 0. `[signer]` Authority
/// 1. `[writable]` PredictionMarketFeeConfig PDA
/// 2. `[writable]` Fee Fund PDA
/// 3. `[]` System Program
InitializePMFeeConfig(InitializePMFeeConfigArgs),

/// 收取预测市场手续费 (CPI from Prediction Market Program)
/// 
/// Accounts:
/// 0. `[signer]` Caller Program
/// 1. `[writable]` PredictionMarketFeeConfig
/// 2. `[writable]` Fee Fund vault
/// 3. `[writable]` Source token account
/// 4. `[]` Token Program
CollectPMFee(CollectPMFeeArgs),

/// 发放做市商奖励 (CPI or Admin)
/// 
/// Accounts:
/// 0. `[signer]` Authority or Caller
/// 1. `[writable]` PredictionMarketFeeConfig
/// 2. `[writable]` Fee Fund vault
/// 3. `[writable]` Maker's token account
/// 4. `[]` Token Program
DistributeMakerReward(DistributeMakerRewardArgs),

/// 发放市场创建者分成 (CPI)
/// 
/// Accounts:
/// 0. `[signer]` Caller Program
/// 1. `[writable]` PredictionMarketFeeConfig
/// 2. `[writable]` Fee Fund vault
/// 3. `[writable]` Creator's token account
/// 4. `[]` Token Program
DistributeCreatorReward(DistributeCreatorRewardArgs),

/// 更新预测市场手续费配置
/// 
/// Accounts:
/// 0. `[signer]` Authority
/// 1. `[writable]` PredictionMarketFeeConfig
UpdatePMFeeConfig(UpdatePMFeeConfigArgs),
```

---

## 4. 1024-prediction-market-program 设计

### 4.1 账户结构

#### 4.1.1 PredictionMarketConfig (全局配置)

```rust
/// PDA Seeds: ["pm_config"]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct PredictionMarketConfig {
    pub discriminator: u64,
    
    /// 管理员
    pub admin: Pubkey,
    
    /// USDC Mint
    pub usdc_mint: Pubkey,
    
    /// Vault Program ID
    pub vault_program: Pubkey,
    
    /// Fund Program ID
    pub fund_program: Pubkey,
    
    /// 下一个市场 ID
    pub next_market_id: u64,
    
    /// 总市场数
    pub total_markets: u64,
    
    /// 活跃市场数
    pub active_markets: u64,
    
    /// 总交易量 (e6)
    pub total_volume_e6: i64,
    
    /// 是否暂停
    pub is_paused: bool,
    
    /// PDA bump
    pub bump: u8,
    
    pub reserved: [u8; 64],
}
```

#### 4.1.2 Market (单个市场)

```rust
/// PDA Seeds: ["market", market_id.to_le_bytes()]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Market {
    pub discriminator: u64,
    
    /// 市场唯一 ID
    pub market_id: u64,
    
    /// 市场创建者
    pub creator: Pubkey,
    
    /// 问题描述哈希 (IPFS CID 的哈希)
    pub question_hash: [u8; 32],
    
    /// Resolution Spec 哈希
    pub resolution_spec_hash: [u8; 32],
    
    /// YES Token Mint
    pub yes_mint: Pubkey,
    
    /// NO Token Mint
    pub no_mint: Pubkey,
    
    /// USDC Vault (市场专用金库)
    pub market_vault: Pubkey,
    
    /// 市场状态
    pub status: MarketStatus,
    
    /// 最早可结算时间
    pub resolution_time: i64,
    
    /// 最迟结算截止时间
    pub finalization_deadline: i64,
    
    /// 最终结果
    pub final_result: Option<MarketResult>,
    
    /// 创建时间
    pub created_at: i64,
    
    /// 总铸造完整集数量
    pub total_minted: u64,
    
    /// 总交易量 (e6)
    pub total_volume_e6: i64,
    
    /// 创建者费率 (bps)
    pub creator_fee_bps: u16,
    
    /// 审查状态
    pub review_status: ReviewStatus,
    
    /// PDA bump
    pub bump: u8,
    
    pub reserved: [u8; 64],
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketStatus {
    Pending = 0,
    Active = 1,
    Paused = 2,
    Resolved = 3,
    Cancelled = 4,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketResult {
    Yes = 0,
    No = 1,
    Invalid = 2,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewStatus {
    None = 0,
    Flagged = 1,
    CancelledInvalid = 2,
    CancelledRegulatory = 3,
}
```

#### 4.1.3 Order (订单)

```rust
/// PDA Seeds: ["order", market_id.to_le_bytes(), order_id.to_le_bytes()]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Order {
    pub discriminator: u64,
    
    /// 订单 ID
    pub order_id: u64,
    
    /// 所属市场
    pub market_id: u64,
    
    /// 订单创建者
    pub owner: Pubkey,
    
    /// 订单方向
    pub side: OrderSide,
    
    /// 结果类型
    pub outcome: Outcome,
    
    /// 价格 (e6, 1 USDC = 1_000_000)
    pub price: u64,
    
    /// 数量
    pub amount: u64,
    
    /// 已成交数量
    pub filled_amount: u64,
    
    /// 订单状态
    pub status: OrderStatus,
    
    /// 订单类型
    pub order_type: OrderType,
    
    /// 过期时间 (None = 永不过期)
    pub expiration_time: Option<i64>,
    
    /// 创建时间
    pub created_at: i64,
    
    /// 最后更新时间
    pub updated_at: i64,
    
    /// PDA bump
    pub bump: u8,
    
    pub reserved: [u8; 32],
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    Buy = 0,
    Sell = 1,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Yes = 0,
    No = 1,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderStatus {
    Open = 0,
    PartialFilled = 1,
    Filled = 2,
    Cancelled = 3,
    Expired = 4,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderType {
    GTC = 0,  // Good Till Cancel
    GTD = 1,  // Good Till Date
    IOC = 2,  // Immediate Or Cancel
    FOK = 3,  // Fill Or Kill
}
```

#### 4.1.4 Position (用户持仓)

```rust
/// PDA Seeds: ["position", market_id.to_le_bytes(), owner.key()]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Position {
    pub discriminator: u64,
    
    /// 所属市场
    pub market_id: u64,
    
    /// 持仓所有者
    pub owner: Pubkey,
    
    /// YES Token 数量
    pub yes_amount: u64,
    
    /// NO Token 数量
    pub no_amount: u64,
    
    /// YES 平均成本 (e6)
    pub yes_avg_cost: u64,
    
    /// NO 平均成本 (e6)
    pub no_avg_cost: u64,
    
    /// 已实现盈亏 (e6)
    pub realized_pnl: i64,
    
    /// 是否已结算
    pub settled: bool,
    
    /// 结算金额 (e6)
    pub settlement_amount: u64,
    
    /// 创建时间
    pub created_at: i64,
    
    /// 最后更新时间
    pub updated_at: i64,
    
    /// PDA bump
    pub bump: u8,
    
    pub reserved: [u8; 32],
}
```

### 4.2 核心指令

```rust
pub enum PredictionMarketInstruction {
    // === 初始化 (0-9) ===
    
    /// 初始化全局配置
    Initialize(InitializeArgs),
    
    // === 市场管理 (10-29) ===
    
    /// 创建市场
    CreateMarket(CreateMarketArgs),
    
    /// 激活市场
    ActivateMarket,
    
    /// 暂停市场
    PauseMarket,
    
    /// 取消市场 (退款)
    CancelMarket,
    
    // === 完整集操作 (30-39) ===
    
    /// 铸造完整集 (存入 N USDC，获得 N YES + N NO)
    MintCompleteSet(MintCompleteSetArgs),
    
    /// 赎回完整集 (销毁 N YES + N NO，取回 N USDC)
    RedeemCompleteSet(RedeemCompleteSetArgs),
    
    // === 订单操作 (40-59) ===
    
    /// 提交订单
    PlaceOrder(PlaceOrderArgs),
    
    /// 取消订单
    CancelOrder(CancelOrderArgs),
    
    /// 撮合铸造 (买 YES + 买 NO = 铸造)
    MatchMint(MatchMintArgs),
    
    /// 撮合销毁 (卖 YES + 卖 NO = 销毁)
    MatchBurn(MatchBurnArgs),
    
    // === 结算操作 (60-79) ===
    
    /// 提交结果提案 (预言机)
    ProposeResult(ProposeResultArgs),
    
    /// 挑战结果
    ChallengeResult(ChallengeResultArgs),
    
    /// 确认结果 (无争议后)
    FinalizeResult,
    
    /// 结算市场
    ResolveMarket,
    
    /// 用户领取收益
    ClaimWinnings,
    
    /// 退款 (市场取消时)
    RefundCancelledMarket,
    
    // === 管理操作 (80-99) ===
    
    /// 更新管理员
    UpdateAdmin(UpdateAdminArgs),
    
    /// 设置暂停
    SetPaused(SetPausedArgs),
    
    /// 添加授权调用方
    AddAuthorizedCaller(AddAuthorizedCallerArgs),
}
```

---

## 5. CPI 调用流程

### 5.1 购买 YES Token (完整流程)

```
用户                   PM Program              Vault Program          Fund Program
  │                        │                        │                      │
  │ PlaceOrder(Buy YES)    │                        │                      │
  ├───────────────────────►│                        │                      │
  │                        │ CPI: LockForPrediction │                      │
  │                        ├───────────────────────►│                      │
  │                        │◄───────────────────────┤                      │
  │                        │                        │                      │
  │                        │ (等待撮合)               │                      │
  │                        │                        │                      │
  
链下撮合引擎
  │                        │                        │                      │
  │ MatchMint(YES,NO)      │                        │                      │
  ├───────────────────────►│                        │                      │
  │                        │ CPI: CollectPMFee      │                      │
  │                        ├───────────────────────────────────────────────►│
  │                        │◄───────────────────────────────────────────────┤
  │                        │                        │                      │
  │                        │ Mint YES Token to User │                      │
  │                        │ Mint NO Token to Other │                      │
  │                        │                        │                      │
  │◄───────────────────────┤                        │                      │
```

### 5.2 结算流程

```
预言机                  PM Program              Vault Program          Fund Program
  │                        │                        │                      │
  │ FinalizeResult(YES)    │                        │                      │
  ├───────────────────────►│                        │                      │
  │                        │ 更新 Market.final_result                      │
  │                        │                        │                      │
  
用户
  │                        │                        │                      │
  │ ClaimWinnings          │                        │                      │
  ├───────────────────────►│                        │                      │
  │                        │ 读取 Position.yes_amount                      │
  │                        │ CPI: PredictionSettle  │                      │
  │                        ├───────────────────────►│                      │
  │                        │◄───────────────────────┤                      │
  │                        │                        │                      │
  │                        │ 销毁 YES Token (可选)    │                      │
  │                        │                        │                      │
  │◄───────────────────────┤                        │                      │
```

---

## 6. 实现计划

### Phase 1: 基础结构 (Week 1)

- [ ] 创建 `1024-prediction-market-program` 项目结构
- [ ] 定义所有账户结构 (state.rs)
- [ ] 定义所有指令 (instruction.rs)
- [ ] 实现 `Initialize` 指令

### Phase 2: Vault/Fund 修改 (Week 1-2)

- [ ] 修改 `1024-vault-program/state.rs` - UserAccount 扩展
- [ ] 修改 `1024-vault-program/instruction.rs` - 新增指令
- [ ] 修改 `1024-vault-program/processor.rs` - 实现新指令
- [ ] 修改 `1024-fund-program/state.rs` - PredictionMarketFeeConfig
- [ ] 修改 `1024-fund-program/instruction.rs` - 新增指令
- [ ] 修改 `1024-fund-program/processor.rs` - 实现新指令

### Phase 3: 核心功能 (Week 2-3)

- [ ] 实现 `CreateMarket`
- [ ] 实现 `MintCompleteSet` / `RedeemCompleteSet`
- [ ] 实现 `PlaceOrder` / `CancelOrder`
- [ ] 实现 `MatchMint` / `MatchBurn`

### Phase 4: 结算功能 (Week 3-4)

- [ ] 实现 Oracle 接口
- [ ] 实现 `ProposeResult` / `ChallengeResult` / `FinalizeResult`
- [ ] 实现 `ResolveMarket` / `ClaimWinnings`
- [ ] 实现 `RefundCancelledMarket`

### Phase 5: 测试 (Week 4)

- [ ] 单元测试
- [ ] 集成测试
- [ ] Devnet 部署测试

---

## 7. 参考

- 现有程序: `1024-vault-program`, `1024-fund-program`
- 设计文档: `1024-docs/prediction-market/design.md`

