# 1024 Prediction Market Program

> 去中心化预测市场程序 - 二元/多结果市场、链下撮合、Oracle 结算

---

## 📋 目录

- [概述](#概述)
- [架构设计](#架构设计)
- [市场类型](#市场类型)
- [账户结构](#账户结构)
- [指令详解](#指令详解)
- [市场生命周期](#市场生命周期)
- [交易机制](#交易机制)
- [Oracle 系统](#oracle-系统)
- [PDA 地址推导](#pda-地址推导)
- [CPI 集成](#cpi-集成)
- [构建与部署](#构建与部署)
- [测试脚本](#测试脚本)
- [错误代码](#错误代码)

---

## 概述

### 程序职责

1024 Prediction Market Program 是去中心化预测市场的核心，支持：

| 功能 | 说明 |
|------|------|
| **二元市场** | YES/NO 双结果预测 (体育、政治、加密价格) |
| **多结果市场** | 2-32 个结果 (选举、比赛名次) |
| **完整集铸造/赎回** | 1 USDC ↔ 1 YES + 1 NO (恒定和) |
| **链下撮合** | 高性能订单簿撮合 + 链上结算 |
| **Oracle 机制** | 提案 → 挑战 → 仲裁 → 最终确定 |
| **Relayer 支持** | 跨链入金场景无需用户链上签名 |

### 部署信息

| 网络 | Program ID |
|------|-----------|
| 1024Chain Testnet | `FnwmQjmUkRTLA1G3i1CmFVE5cySzQGYZRezGAErdLizu` |
| 1024Chain Mainnet | TBD |

**⚠️ 多选市场限制**: 最大支持 **16 个 outcomes** 用于撮合操作 (MatchMintMulti/MatchBurnMulti)，以避免超过 Solana 64 账户限制

### 系统架构

```
┌─────────────────────────────────────────────────────────────────┐
│                1024-prediction-market-program                    │
│                     (Core Business Logic)                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │ Market          │  │ Order Book   │  │ Settlement       │   │
│  │ Management      │  │ Operations   │  │ & Claims         │   │
│  ├─────────────────┤  ├──────────────┤  ├──────────────────┤   │
│  │ CreateMarket    │  │ PlaceOrder   │  │ ClaimWinnings    │   │
│  │ ActivateMarket  │  │ CancelOrder  │  │ RefundCancelled  │   │
│  │ CancelMarket    │  │ MatchMint    │  │                  │   │
│  │ PauseMarket     │  │ MatchBurn    │  │                  │   │
│  └─────────────────┘  │ ExecuteTrade │  └──────────────────┘   │
│                       └──────────────┘                          │
│  ┌─────────────────┐  ┌──────────────┐                         │
│  │ Complete Sets   │  │ Oracle       │                         │
│  ├─────────────────┤  ├──────────────┤                         │
│  │ MintCompleteSet │  │ ProposeResult│                         │
│  │ RedeemComplete  │  │ Challenge    │                         │
│  │ MultiOutcome    │  │ Finalize     │                         │
│  └─────────────────┘  │ Resolve      │                         │
│                       └──────────────┘                          │
└───────────────┬─────────────────────┬───────────────────────────┘
                │                     │
           CPI  │                     │ CPI
                ▼                     ▼
    ┌─────────────────────┐ ┌─────────────────────┐
    │ 1024-vault-program  │ │ 1024-fund-program   │
    │ (User Fund Custody) │ │ (Fee Management)    │
    │                     │ │                     │
    │ - PredictionMarket  │ │ - CollectPMFee      │
    │   Lock/Unlock       │ │ - DistributeMaker   │
    │ - Settlement        │ │ - DistributeCreator │
    └─────────────────────┘ └─────────────────────┘
```

---

## 架构设计

### 交易流程

```
┌─────────────────────────────────────────────────────────────────┐
│                    交易撮合流程                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   用户 A (买 YES @ $0.60)          用户 B (买 NO @ $0.40)       │
│         │                                  │                    │
│         │  PlaceOrder                      │  PlaceOrder        │
│         ▼                                  ▼                    │
│   ┌─────────────┐                 ┌─────────────┐              │
│   │ YES Order   │                 │ NO Order    │              │
│   │ $0.60 x 100 │                 │ $0.40 x 100 │              │
│   └──────┬──────┘                 └──────┬──────┘              │
│          │                               │                      │
│          │     MatchMint (Off-chain)     │                      │
│          └───────────────┬───────────────┘                      │
│                          ▼                                      │
│            ┌──────────────────────────┐                        │
│            │  Matching Engine (链下)   │                        │
│            │  YES $0.60 + NO $0.40    │                        │
│            │  = $1.00 (完整集)         │                        │
│            └────────────┬─────────────┘                        │
│                         │                                       │
│                         ▼ MatchMint (链上)                      │
│            ┌──────────────────────────┐                        │
│            │  铸造完整集:              │                        │
│            │  - 收取 $1 USDC          │                        │
│            │  - 铸造 100 YES 给用户 A │                        │
│            │  - 铸造 100 NO 给用户 B  │                        │
│            └──────────────────────────┘                        │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 代币经济学

```
完整集恒等式: 1 USDC = 1 YES + 1 NO

价格关系: YES_price + NO_price = $1.00

示例:
- YES @ $0.65, NO @ $0.35 → 市场认为 YES 有 65% 概率
- 结算时: 获胜代币 = $1, 失败代币 = $0
```

---

## 市场类型

### 1. 二元市场 (Binary Market)

| 属性 | 说明 |
|------|------|
| 结果数 | 2 (YES / NO) |
| 代币 | YES Token + NO Token |
| 结算 | 获胜方 = $1, 失败方 = $0 |
| 用例 | 体育赛事、价格预测、政治选举 |

**示例:** "BTC 在 2025-01-01 是否超过 $100,000?"

### 2. 多结果市场 (Multi-Outcome Market)

| 属性 | 说明 |
|------|------|
| 结果数 | 2-32 个 |
| 代币 | N 个 Outcome Token |
| 结算 | 获胜方 = $1, 其余 = $0 |
| 用例 | 选举多候选人、比赛名次 |

**示例:** "2024 美国大选获胜者? (Trump / Biden / Other)"

---

## 账户结构

### 1. PredictionMarketConfig (全局配置)

**PDA Seeds:** `["pm_config"]`

```rust
pub struct PredictionMarketConfig {
    pub discriminator: u64,
    pub admin: Pubkey,                      // 程序管理员
    pub oracle_admin: Pubkey,               // Oracle 管理员
    pub usdc_mint: Pubkey,                  // USDC Mint
    pub vault_program: Pubkey,              // Vault Program ID
    pub fund_program: Pubkey,               // Fund Program ID
    
    // 授权调用方 (Matching Engine)
    pub authorized_callers: Vec<Pubkey>,
    
    // Oracle 配置
    pub challenge_window_secs: i64,         // 挑战窗口 (默认 24h)
    pub proposer_bond_e6: u64,              // 提案保证金
    
    // 统计
    pub total_markets: u64,
    pub active_markets: u64,
    pub total_volume_e6: u64,
    
    pub is_paused: bool,
    pub bump: u8,
    pub reserved: [u8; 64],
}
```

### 2. Market (市场账户)

**PDA Seeds:** `["market", market_id.to_le_bytes()]`

```rust
pub struct Market {
    pub discriminator: u64,
    pub market_id: u64,                     // 唯一市场 ID
    pub creator: Pubkey,                    // 市场创建者
    
    // 市场元数据 (链下存储细节)
    pub question_hash: [u8; 32],            // 问题哈希 (IPFS CID)
    pub resolution_spec_hash: [u8; 32],     // 结算规则哈希
    
    // 代币
    pub yes_mint: Pubkey,                   // YES Token Mint
    pub no_mint: Pubkey,                    // NO Token Mint
    pub market_vault: Pubkey,               // USDC 金库
    
    // 时间
    pub resolution_time: i64,               // 最早结算时间
    pub finalization_deadline: i64,         // 最晚最终确定时间
    pub created_at: i64,
    
    // 状态
    pub status: MarketStatus,               // Created/Active/Paused/Resolved/Cancelled
    pub result: MarketResult,               // None/Yes/No/Invalid
    
    // 费用
    pub creator_fee_bps: u16,               // 创建者费率 (max 5%)
    
    // 统计
    pub total_yes_minted: u64,
    pub total_no_minted: u64,
    pub total_volume_e6: u64,
    
    pub bump: u8,
    pub reserved: [u8; 64],
}

pub enum MarketStatus {
    Created = 0,    // 已创建，等待激活
    Active = 1,     // 交易中
    Paused = 2,     // 已暂停
    Resolved = 3,   // 已结算
    Cancelled = 4,  // 已取消
    Flagged = 5,    // 被标记审核
}

pub enum MarketResult {
    None = 0,       // 未确定
    Yes = 1,        // YES 获胜
    No = 2,         // NO 获胜
    Invalid = 3,    // 无效 (退款)
}
```

### 3. Order (订单账户)

**PDA Seeds:** `["order", market_id.to_le_bytes(), order_id.to_le_bytes()]`

```rust
pub struct Order {
    pub discriminator: u64,
    pub market_id: u64,
    pub order_id: u64,
    pub owner: Pubkey,
    
    pub side: OrderSide,                    // Buy / Sell
    pub outcome: Outcome,                   // Yes / No
    pub order_type: OrderType,              // GTC / IOC / FOK / GTD
    
    pub price: u64,                         // 价格 (e6, 650000 = $0.65)
    pub original_amount: u64,               // 原始数量
    pub filled_amount: u64,                 // 已成交数量
    pub remaining_amount: u64,              // 剩余数量
    
    pub status: OrderStatus,                // Open / Filled / Cancelled / Expired
    pub created_at: i64,
    pub expiration_time: Option<i64>,       // GTD 订单过期时间
    
    pub bump: u8,
    pub reserved: [u8; 32],
}

pub enum OrderSide {
    Buy = 0,
    Sell = 1,
}

pub enum Outcome {
    Yes = 0,
    No = 1,
}

pub enum OrderType {
    GTC = 0,    // Good Till Cancel
    IOC = 1,    // Immediate or Cancel
    FOK = 2,    // Fill or Kill
    GTD = 3,    // Good Till Date
}
```

### 4. Position (用户持仓)

**PDA Seeds:** `["position", market_id.to_le_bytes(), owner_pubkey]`

```rust
pub struct Position {
    pub discriminator: u64,
    pub market_id: u64,
    pub owner: Pubkey,
    
    pub yes_tokens: u64,                    // YES Token 持仓
    pub no_tokens: u64,                     // NO Token 持仓
    
    pub total_deposited_e6: i64,            // 累计存入
    pub total_withdrawn_e6: i64,            // 累计提取
    pub realized_pnl_e6: i64,               // 已实现盈亏
    
    pub created_at: i64,
    pub last_update_ts: i64,
    
    pub bump: u8,
    pub reserved: [u8; 32],
}
```

### 5. OracleProposal (结果提案)

**PDA Seeds:** `["oracle_proposal", market_id.to_le_bytes()]`

```rust
pub struct OracleProposal {
    pub discriminator: u64,
    pub market_id: u64,
    pub proposer: Pubkey,                   // 提案者
    
    pub proposed_result: MarketResult,      // 提议的结果
    pub proposed_at: i64,                   // 提案时间
    pub challenge_deadline: i64,            // 挑战截止时间
    
    pub is_challenged: bool,                // 是否被挑战
    pub challenger: Option<Pubkey>,         // 挑战者
    pub challenger_result: Option<MarketResult>, // 挑战者提议
    
    pub proposer_bond_e6: u64,              // 提案者保证金
    pub challenger_bond_e6: u64,            // 挑战者保证金
    
    pub is_finalized: bool,                 // 是否最终确定
    pub final_result: Option<MarketResult>, // 最终结果
    
    pub bump: u8,
    pub reserved: [u8; 32],
}
```

### 6. AuthorizedCallers (授权调用方注册)

**PDA Seeds:** `["authorized_callers"]`

```rust
/// 最多 10 个授权调用方
pub const MAX_AUTHORIZED_CALLERS: usize = 10;

pub struct AuthorizedCallers {
    pub discriminator: u64,
    pub count: u8,                          // 当前数量
    pub callers: [Pubkey; 10],              // 授权 pubkey 列表
    pub created_at: i64,
    pub updated_at: i64,
    pub bump: u8,
    pub reserved: [u8; 32],
}

impl AuthorizedCallers {
    /// 检查 pubkey 是否授权
    pub fn is_authorized(&self, caller: &Pubkey) -> bool;
    
    /// 添加授权调用方
    pub fn add_caller(&mut self, caller: Pubkey) -> Result<(), ()>;
    
    /// 移除授权调用方
    pub fn remove_caller(&mut self, caller: &Pubkey) -> Result<(), ()>;
}
```

**用途:** 存储授权的撮合引擎 (Matching Engine) 公钥，用于验证 MatchMint/MatchBurn/ExecuteTrade 等指令的调用方。

---

## 指令详解

### 初始化指令

```rust
/// Initialize the Prediction Market Program
Initialize(InitializeArgs)

pub struct InitializeArgs {
    pub oracle_admin: Pubkey,
    pub challenge_window_secs: i64,    // 默认 86400 (24h)
    pub proposer_bond_e6: u64,         // 默认 100_000_000 ($100)
}
```

### 市场管理指令

| 指令 | 说明 | 调用者 |
|------|------|--------|
| `CreateMarket` | 创建新市场 | 任何人 |
| `ActivateMarket` | 激活市场 | Admin |
| `PauseMarket` | 暂停交易 | Admin |
| `ResumeMarket` | 恢复交易 | Admin |
| `CancelMarket` | 取消市场 | Admin |
| `FlagMarket` | 标记审核 | Admin |
| `CreateMultiOutcomeMarket` | 创建多结果市场 | 任何人 |

**CreateMarket 参数:**

```rust
pub struct CreateMarketArgs {
    pub question_hash: [u8; 32],       // IPFS CID 哈希
    pub resolution_spec_hash: [u8; 32], // 结算规则哈希
    pub resolution_time: i64,          // 最早结算时间
    pub finalization_deadline: i64,    // 最晚最终确定
    pub creator_fee_bps: u16,          // 创建者费率 (max 500 = 5%)
}
```

### 完整集操作

| 指令 | 说明 | 账户数 |
|------|------|--------|
| `MintCompleteSet` | 1 USDC → 1 YES + 1 NO | 16 |
| `RedeemCompleteSet` | 1 YES + 1 NO → 1 USDC | 15 |
| `MintMultiOutcomeCompleteSet` | 1 USDC → N 个 Outcome Token | 8+2N |
| `RedeemMultiOutcomeCompleteSet` | N 个 Outcome Token → 1 USDC | 7+2N |

### 订单操作

| 指令 | 说明 |
|------|------|
| `PlaceOrder` | 挂单 |
| `CancelOrder` | 取消订单 |
| `MatchMint` | 撮合买单 (YES Buy + NO Buy = Mint) |
| `MatchBurn` | 撮合卖单 (YES Sell + NO Sell = Burn) |
| `MatchMintMulti` | 多选市场撮合铸造 (N 个 Buy 订单 = Mint N tokens) |
| `MatchBurnMulti` | 多选市场撮合销毁 (N 个 Sell 订单 = Burn N tokens) |
| `ExecuteTrade` | 直接成交 (Taker vs Maker) |

**MatchMintMulti 参数 (新增):**

```rust
pub struct MatchMintMultiArgs {
    pub market_id: u64,
    pub num_outcomes: u8,    // 2-16 (受账户数限制)
    pub amount: u64,         // 撮合数量
    pub orders: Vec<(u8, u64, u64)>,  // (outcome_index, order_id, price_e6)
}

// 账户列表 (6 + 3*N 个账户):
// 0. [signer] Authorized Caller (Matching Engine)
// 1. [] PredictionMarketConfig
// 2. [writable] Market
// 3. [writable] Market Vault
// 4. [] Token Program
// 5. [] System Program
// 对于每个 outcome i (0..N-1):
//   6 + 3*i + 0: [writable] Order PDA
//   6 + 3*i + 1: [writable] Outcome Token Mint
//   6 + 3*i + 2: [writable] Buyer's Token Account
```

**MatchBurnMulti 参数 (新增):**

```rust
pub struct MatchBurnMultiArgs {
    pub market_id: u64,
    pub num_outcomes: u8,
    pub amount: u64,
    pub orders: Vec<(u8, u64, u64)>,  // Sell 订单信息
}
```

**Compute Budget 建议:**

| Outcomes 数量 | 预估 CU | 建议请求 |
|--------------|---------|---------|
| 2-4 | ~80,000 | 150,000 |
| 5-8 | ~150,000 | 250,000 |
| 9-16 | ~300,000 | 450,000 |

**PlaceOrder 参数:**

```rust
pub struct PlaceOrderArgs {
    pub market_id: u64,
    pub side: OrderSide,               // Buy / Sell
    pub outcome: Outcome,              // Yes / No
    pub price: u64,                    // 价格 (e6)
    pub amount: u64,                   // 数量
    pub order_type: OrderType,         // GTC / IOC / FOK / GTD
    pub expiration_time: Option<i64>,  // GTD 过期时间
}
```

### Oracle 指令

| 指令 | 说明 | 调用者 |
|------|------|--------|
| `ProposeResult` | 提交结果提案 | Oracle / 授权者 |
| `ChallengeResult` | 挑战提案 | 任何人 |
| `FinalizeResult` | 最终确定结果 | 任何人 (挑战窗口后) |
| `ResolveDispute` | 仲裁争议 | Committee |

### 结算指令

| 指令 | 说明 |
|------|------|
| `ClaimWinnings` | 领取获胜代币收益 |
| `RefundCancelledMarket` | 取消市场退款 |
| `ClaimMultiOutcomeWinnings` | 多结果市场领取 |

### Relayer 指令

| 指令 | 说明 |
|------|------|
| `RelayerMintCompleteSet` | Relayer 代理铸造 |
| `RelayerRedeemCompleteSet` | Relayer 代理赎回 |
| `RelayerPlaceOrder` | Relayer 代理挂单 |
| `RelayerCancelOrder` | Relayer 代理取消 |
| `RelayerClaimWinnings` | Relayer 代理领取 |
| `RelayerRefundCancelledMarket` | Relayer 代理退款 |
| `RelayerMintMultiOutcomeCompleteSet` | 多结果铸造 |
| `RelayerRedeemMultiOutcomeCompleteSet` | 多结果赎回 |
| `RelayerPlaceMultiOutcomeOrder` | 多结果挂单 |
| `RelayerClaimMultiOutcomeWinnings` | 多结果领取 |

### 管理指令

| 指令 | 说明 |
|------|------|
| `UpdateAdmin` | 更新管理员 |
| `UpdateOracleAdmin` | 更新 Oracle 管理员 |
| `SetPaused` | 暂停/恢复程序 |
| `UpdateOracleConfig` | 更新 Oracle 配置 |
| `AddAuthorizedCaller` | 添加授权撮合引擎 |
| `RemoveAuthorizedCaller` | 移除授权撮合引擎 |

---

## 市场生命周期

```
┌─────────────────────────────────────────────────────────────────┐
│                     市场生命周期                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   1. 创建 (Created)                                             │
│      ├─ 任何人调用 CreateMarket                                  │
│      ├─ 设置问题、结算规则、时间参数                              │
│      └─ 等待 Admin 审核激活                                      │
│              │                                                  │
│              ▼                                                  │
│   2. 激活 (Active)                                              │
│      ├─ Admin 调用 ActivateMarket                               │
│      ├─ 开放 MintCompleteSet / PlaceOrder                       │
│      ├─ 链下撮合引擎开始工作                                     │
│      └─ 可能被 Pause / Flag                                     │
│              │                                                  │
│              ▼ (resolution_time 到达)                           │
│   3. 等待结算 (Resolution Period)                               │
│      ├─ 禁止新订单                                              │
│      ├─ Oracle 提交 ProposeResult                               │
│      └─ 24h 挑战窗口                                            │
│              │                                                  │
│      ┌───────┴───────┐                                         │
│      │               │                                         │
│      ▼               ▼                                         │
│   4a. 无挑战          4b. 有挑战                                │
│   FinalizeResult     ResolveDispute                            │
│      │                    │                                    │
│      └───────┬────────────┘                                    │
│              │                                                  │
│              ▼                                                  │
│   5. 已结算 (Resolved)                                          │
│      ├─ 最终结果确定                                            │
│      ├─ 获胜代币可兑换 $1                                        │
│      └─ 用户调用 ClaimWinnings                                  │
│                                                                 │
│   [取消分支]                                                    │
│   CancelMarket (任何阶段)                                       │
│      └─ 进入 Cancelled 状态                                     │
│         └─ 用户调用 RefundCancelledMarket                       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 交易机制

### MatchMint (买单撮合)

当 YES 买单和 NO 买单可以形成完整集时：

```
YES Buy @ $0.60 (100 tokens) + NO Buy @ $0.40 (100 tokens)
= $100 USDC locked
= Mint 100 YES to User A
= Mint 100 NO to User B
```

### MatchBurn (卖单撮合)

当 YES 卖单和 NO 卖单可以销毁完整集时：

```
YES Sell @ $0.70 (100 tokens) + NO Sell @ $0.30 (100 tokens)
= Burn 100 YES from User A
= Burn 100 NO from User B
= Release $100 USDC (distribute by price)
```

### ExecuteTrade (直接交易)

Taker 订单直接与 Maker 订单成交：

```
Taker: Buy YES @ $0.65 (100 tokens)
Maker: Sell YES @ $0.60 (100 tokens)
= Transfer 100 YES from Maker to Taker
= Transfer $60 USDC from Taker to Maker
```

**ExecuteTrade 账户列表 (更新):**

```rust
/// ExecuteTrade 账户 (11 个):
/// 0. [signer] Authorized Caller
/// 1. [writable] PredictionMarketConfig
/// 2. [writable] Market
/// 3. [writable] Buy Order (Taker)
/// 4. [writable] Sell Order (Maker)
/// 5. [writable] Seller's Token Account / Escrow
/// 6. [writable] Buyer's Token Account
/// 7. [] Token Program
/// 8. [writable] Buyer Position PDA      // 自动创建/更新
/// 9. [writable] Seller Position PDA     // 更新
/// 10. [] System Program                  // 用于创建 Position
```

**Position 更新逻辑:**
- Buyer Position: 调用 `add_tokens()` 增加代币持仓
- Seller Position: 调用 `remove_tokens()` 减少代币持仓并记录已实现盈亏

---

## Oracle 系统

### 结算流程

```
┌─────────────────────────────────────────────────────────────────┐
│                     Oracle 结算流程                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   1. ProposeResult                                              │
│      ├─ Oracle Admin 提交结果提案                                │
│      ├─ 锁定 $100 USDC 保证金                                   │
│      └─ 开始 24h 挑战窗口                                        │
│              │                                                  │
│      ┌───────┴───────────────────┐                             │
│      │                           │                             │
│      ▼                           ▼                             │
│   2a. 无挑战                    2b. ChallengeResult            │
│   (24h 后)                      ├─ 挑战者提交不同结果           │
│      │                          ├─ 锁定 $100 保证金             │
│      │                          └─ 进入仲裁流程                 │
│      ▼                               │                         │
│   3a. FinalizeResult                 ▼                         │
│   ├─ 任何人可调用               3b. ResolveDispute             │
│   ├─ 确认提案结果               ├─ Committee 投票决定           │
│   └─ 返还保证金                 ├─ 获胜方获得双方保证金         │
│      │                          └─ 确定最终结果                 │
│      │                               │                         │
│      └───────────────────────────────┘                         │
│                      │                                         │
│                      ▼                                         │
│   4. Market.status = Resolved                                  │
│      Market.result = Yes / No / Invalid                        │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 保证金机制

| 角色 | 保证金 | 成功 | 失败 |
|------|--------|------|------|
| 提案者 | $100 | 返还 | 没收给挑战者 |
| 挑战者 | $100 | 获得提案者保证金 | 没收给提案者 |

---

## PDA 地址推导

### TypeScript 示例

```typescript
const PM_PROGRAM_ID = new PublicKey('PMrkT1yH6Bna4JKBMKjS1NU7qAcM2y7V9Q3VNUa8PRA');

// PredictionMarketConfig PDA
const [pmConfigPDA] = await PublicKey.findProgramAddress(
    [Buffer.from("pm_config")],
    PM_PROGRAM_ID
);

// Market PDA
const marketId = 1n;
const [marketPDA] = await PublicKey.findProgramAddress(
    [
        Buffer.from("market"),
        Buffer.from(marketId.toString(16).padStart(16, '0'), 'hex'),
    ],
    PM_PROGRAM_ID
);

// YES Token Mint PDA
const [yesMintPDA] = await PublicKey.findProgramAddress(
    [Buffer.from("yes_mint"), marketPDA.toBuffer()],
    PM_PROGRAM_ID
);

// NO Token Mint PDA
const [noMintPDA] = await PublicKey.findProgramAddress(
    [Buffer.from("no_mint"), marketPDA.toBuffer()],
    PM_PROGRAM_ID
);

// Market Vault PDA
const [marketVaultPDA] = await PublicKey.findProgramAddress(
    [Buffer.from("market_vault"), marketPDA.toBuffer()],
    PM_PROGRAM_ID
);

// Order PDA
const orderId = 1n;
const [orderPDA] = await PublicKey.findProgramAddress(
    [
        Buffer.from("order"),
        Buffer.from(marketId.toString(16).padStart(16, '0'), 'hex'),
        Buffer.from(orderId.toString(16).padStart(16, '0'), 'hex'),
    ],
    PM_PROGRAM_ID
);

// Position PDA
const [positionPDA] = await PublicKey.findProgramAddress(
    [
        Buffer.from("position"),
        Buffer.from(marketId.toString(16).padStart(16, '0'), 'hex'),
        owner.toBuffer(),
    ],
    PM_PROGRAM_ID
);

// OracleProposal PDA
const [proposalPDA] = await PublicKey.findProgramAddress(
    [
        Buffer.from("oracle_proposal"),
        Buffer.from(marketId.toString(16).padStart(16, '0'), 'hex'),
    ],
    PM_PROGRAM_ID
);
```

---

## CPI 集成

### 调用 Vault Program

```rust
// 锁定资金用于预测市场
cpi::prediction_market_lock(
    vault_program,
    vault_config,
    user_account,
    pm_user_account,
    amount,
)?;

// 释放锁定资金
cpi::prediction_market_unlock(
    vault_program,
    vault_config,
    user_account,
    pm_user_account,
    amount,
)?;

// 结算
cpi::prediction_market_settle(
    vault_program,
    vault_config,
    pm_user_account,
    locked_amount,
    settlement_amount,
)?;
```

### 调用 Fund Program

```rust
// 收取铸造费
cpi::collect_pm_minting_fee(
    fund_program,
    pm_fee_config,
    pm_fee_vault,
    user_token_account,
    minting_amount,
)?;

// 发放创建者分成
cpi::distribute_pm_creator_reward(
    fund_program,
    pm_fee_config,
    pm_fee_vault,
    creator_token_account,
    reward_amount,
    market_id,
)?;
```

---

## 构建与部署

### 构建

```bash
cd 1024-prediction-market-program

# 编译检查
cargo check

# 运行测试
cargo test --lib

# 构建 BPF 程序
cargo build-sbf
```

### 部署

```bash
# 部署到 1024Chain Testnet
solana program deploy target/deploy/prediction_market_program.so \
    --url https://testnet-rpc.1024chain.com/rpc/ \
    --program-id PMrkT1yH6Bna4JKBMKjS1NU7qAcM2y7V9Q3VNUa8PRA \
    --use-rpc
```

---

## 测试脚本

`scripts/` 目录包含完整的测试脚本：

| 脚本 | 说明 |
|------|------|
| `init_program.js` | 初始化程序 |
| `create_market.js` | 创建市场 |
| `activate_market.js` | 激活市场 |
| `mint_complete_set.js` | 铸造完整集 |
| `redeem_complete_set.js` | 赎回完整集 |
| `place_order.js` | 挂单 |
| `cancel_order.js` | 取消订单 |
| `match_mint.js` | 撮合铸造 |
| `match_burn.js` | 撮合销毁 |
| `execute_trade.js` | 直接交易 |
| `propose_result.js` | 提交结果 |
| `finalize_result.js` | 最终确定 |
| `claim_winnings.js` | 领取收益 |
| `full_lifecycle_test.sh` | 完整生命周期测试 |

### 运行测试

```bash
# 单个测试
node scripts/create_market.js

# 完整生命周期
./scripts/full_lifecycle_test.sh

# 多结果市场测试
./scripts/test_multi_outcome_market.sh
```

---

## 错误代码

| 错误 | Code | 说明 |
|------|------|------|
| `MarketNotActive` | 0 | 市场未激活 |
| `MarketPaused` | 1 | 市场已暂停 |
| `MarketResolved` | 2 | 市场已结算 |
| `MarketCancelled` | 3 | 市场已取消 |
| `InvalidOutcome` | 4 | 无效的结果 |
| `InsufficientBalance` | 5 | 余额不足 |
| `InsufficientTokens` | 6 | 代币不足 |
| `OrderNotFound` | 7 | 订单不存在 |
| `OrderNotOpen` | 8 | 订单已关闭 |
| `InvalidPrice` | 9 | 无效价格 (必须 < $1) |
| `PricesDoNotSumToOne` | 10 | YES + NO ≠ $1 |
| `UnauthorizedCaller` | 11 | 未授权的撮合引擎 |
| `ResolutionTimeNotReached` | 12 | 结算时间未到 |
| `ChallengeWindowOpen` | 13 | 挑战窗口未结束 |
| `AlreadyChallenged` | 14 | 已被挑战 |
| `AlreadyFinalized` | 15 | 已最终确定 |
| `InvalidBond` | 16 | 保证金不足 |
| `NotOracleAdmin` | 17 | 非 Oracle 管理员 |
| `MarketNotResolved` | 18 | 市场未结算 |
| `NoWinningTokens` | 19 | 无获胜代币 |
| `Overflow` | 20 | 数值溢出 |

---

## 文件结构

```
1024-prediction-market-program/
├── Cargo.toml
├── README.md
├── rust-toolchain.toml
├── program-keypair.json
├── prediction_market_v2.json           # IDL
├── src/
│   ├── lib.rs                          # 程序入口
│   ├── entrypoint.rs                   # Entrypoint
│   ├── state.rs                        # 账户结构
│   ├── instruction.rs                  # 指令定义
│   ├── processor.rs                    # 处理逻辑
│   ├── error.rs                        # 错误类型
│   ├── utils.rs                        # 工具函数
│   └── cpi.rs                          # CPI Helpers
├── scripts/                            # JavaScript 测试脚本
│   ├── init_program.js
│   ├── create_market.js
│   ├── ...
│   └── full_lifecycle_test.sh
└── tests/
    ├── package.json
    └── test_initialize.ts
```

---

## License

MIT

---

*Last Updated: 2025-12-08*

---

## 更新日志 (v2.0.0)

### 新增功能

1. **多选市场撮合指令**
   - `MatchMintMulti` - N 个买单撮合铸造
   - `MatchBurnMulti` - N 个卖单撮合销毁
   - 支持 2-16 个 outcomes

2. **AuthorizedCallers PDA**
   - 独立的授权调用方注册表
   - 最多 10 个授权地址
   - `AddAuthorizedCaller` / `RemoveAuthorizedCaller` 指令

3. **ExecuteTrade Position 更新**
   - 自动创建/更新 Buyer Position
   - 自动更新 Seller Position
   - 记录已实现盈亏

4. **Escrow 验证增强**
   - `verify_escrow_pda()` - PDA 验证
   - `verify_escrow_balance()` - 余额验证
   - Sell 订单的代币托管验证

5. **Order 结构统一**
   - 新增 `outcome_index` 字段 (0-based)
   - 二元/多选市场统一接口

6. **CPI 集成同步**
   - 更新 Vault Program 指令索引
   - `PredictionMarketLock` = 16
   - `PredictionMarketUnlock` = 17
   - `PredictionMarketSettle` = 18

7. **新增错误码**
   - `TooManyOutcomes` (650)
   - `OutcomesMismatch` (651)
   - `PriceSumExceedsOne` (652)
   - `PriceSumBelowOne` (653)
