# Miyabi Agents

このディレクトリには、Miyabiプロジェクトで使用するAgent仕様とプロンプトを配置します。

## 📁 ディレクトリ構造

```
agents/
├── specs/           # Agent仕様定義
│   ├── coding/     # コーディング系Agent（7種類）
│   └── business/   # ビジネス系Agent（14種類）
└── prompts/        # 実行プロンプト
    ├── coding/     # コーディング系Agentプロンプト
    └── business/   # ビジネス系Agentプロンプト
```

## 🤖 Coding Agents（7種類）

1. **CoordinatorAgent** - タスク統括・DAG分解
2. **CodeGenAgent** - AI駆動コード生成
3. **ReviewAgent** - コード品質レビュー
4. **IssueAgent** - Issue分析・ラベリング
5. **PRAgent** - Pull Request自動作成
6. **DeploymentAgent** - CI/CDデプロイ自動化
7. **RefresherAgent** - Issue状態監視・更新

## 💼 Business Agents（14種類）

### 戦略・企画系（6種類）
- AIEntrepreneurAgent, ProductConceptAgent, ProductDesignAgent
- FunnelDesignAgent, PersonaAgent, SelfAnalysisAgent

### マーケティング系（5種類）
- MarketResearchAgent, MarketingAgent, ContentCreationAgent
- SNSStrategyAgent, YouTubeAgent

### 営業・顧客管理系（3種類）
- SalesAgent, CRMAgent, AnalyticsAgent

## 📝 Agent仕様の書き方

詳細は各ディレクトリのREADME.mdを参照してください：
- [specs/coding/README.md](specs/coding/README.md)
- [specs/business/README.md](specs/business/README.md)

## 🚀 Agent実行方法

```bash
# CoordinatorAgentでIssue処理
miyabi agent run coordinator --issue 123

# 複数Issue並列処理
miyabi agent run coordinator --issues 123,124,125 --concurrency 3
```

## 🔗 参考リンク

- [Miyabi Agent SDK](https://docs.rs/miyabi-agents)
- [CLAUDE.md](../../CLAUDE.md) - プロジェクトコンテキスト
