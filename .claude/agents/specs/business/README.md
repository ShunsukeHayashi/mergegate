# Business Agent 仕様

このディレクトリには、ビジネス系Agent（14種類）の仕様を配置します。

## Agent仕様ファイルのフォーマット

各Agentの仕様は以下の構造で記述します：

```markdown
# [Agent名] 仕様

## 概要
Agentの役割とビジネス目標

## 入力
- ユーザー要求（市場情報、目標KPI等）
- 必須パラメータ

## 実行フェーズ
1. Phase 1: データ収集
2. Phase 2: 分析
3. Phase 3: 戦略立案
4. ...

## 出力
- 生成するビジネスドキュメント
- レポート形式

## 品質基準
- 検証項目
- スコアリング基準（100点満点）

## 実行例
\`\`\`bash
miyabi agent run ai-entrepreneur --output business-plan.md
\`\`\`
```

## 📊 Business Agent一覧

### 戦略・企画系
- **AIEntrepreneurAgent** - 包括的ビジネスプラン作成
- **ProductConceptAgent** - 製品コンセプト設計
- **ProductDesignAgent** - サービス詳細設計
- **FunnelDesignAgent** - 導線設計
- **PersonaAgent** - ターゲット顧客ペルソナ
- **SelfAnalysisAgent** - 自己分析

### マーケティング系
- **MarketResearchAgent** - 市場調査
- **MarketingAgent** - マーケティング戦略
- **ContentCreationAgent** - コンテンツ制作
- **SNSStrategyAgent** - SNS戦略
- **YouTubeAgent** - YouTube運用最適化

### 営業・顧客管理系
- **SalesAgent** - セールスプロセス最適化
- **CRMAgent** - 顧客関係管理
- **AnalyticsAgent** - データ分析・PDCA

## 🔗 参考

- [SaaS Business Model Guide](https://github.com/ShunsukeHayashi/Miyabi/blob/main/docs/SAAS_BUSINESS_MODEL.md)
- [Business Agents User Guide](https://github.com/ShunsukeHayashi/Miyabi/blob/main/docs/BUSINESS_AGENTS_USER_GUIDE.md)
