# Sprint Planning - MergeGate

**Project**: mergegate
**Total Sprints**: 4
**Sprint Duration**: 5 days (1 week)
**Start Date**: 2025-11-25

---

## 📊 Overview

| Sprint | Period | Milestone | Goal |
|--------|--------|-----------|------|
| 1 | Nov 25-29 | v0.2.0 | Foundation |
| 2 | Dec 2-6 | v0.2.0 | Parser & State |
| 3 | Dec 9-13 | v0.3.0/v1.0.0 | Integration |
| 4 | Dec 16-20 | v1.0.0 | Production Ready |

---

# Sprint 1: Foundation

**Period**: November 25-29, 2025
**Goal**: Core structures for all streams

## 🎯 Sprint Goal

> 全3ストリーム（TUI、LLM、Tools）の基盤構造を確立する

## 📋 Sprint Backlog

| Issue | Task | Priority | Points | Agent |
|-------|------|----------|--------|-------|
| #10 | MarkdownStream core structure | P0 | 3 | CodeGen |
| #15 | DiffRender core structure | P1 | 3 | CodeGen |
| #19 | Anthropic API client | P0 | 5 | CodeGen |
| #24 | Tool trait and registry | P0 | 5 | CodeGen |

**Total Points**: 16

## 📅 Daily Breakdown

### Day 1 (Mon) - Kickoff

**Morning**
```bash
# 全タスク並列開始
miyabi agent run coordinator --issue 10 &
miyabi agent run coordinator --issue 15 &
miyabi agent run coordinator --issue 19 &
miyabi agent run coordinator --issue 24 &
```

**Tasks**:
- [ ] #10 開始 - MarkdownStream struct定義
- [ ] #15 開始 - DiffRender struct定義
- [ ] #19 開始 - AnthropicClient struct定義
- [ ] #24 開始 - Tool trait定義

**End of Day**: 基本struct完成

### Day 2 (Tue) - Core Implementation

**Tasks**:
- [ ] #10 完了 - State enum、buffer管理
- [ ] #15 完了 - Diff parsing logic
- [ ] #19 継続 - Streaming SSE処理
- [ ] #24 継続 - Registry実装

**Reviews**:
```bash
gh pr list --state open
gh pr review <番号> --approve
```

### Day 3 (Wed) - Integration

**Tasks**:
- [ ] #19 完了 - Error handling、retry
- [ ] #24 完了 - Schema generation

**Code Review**:
- [ ] #10 PR レビュー・マージ
- [ ] #15 PR レビュー・マージ

### Day 4 (Thu) - Testing

**Tasks**:
- [ ] 統合テスト実行
- [ ] #19 PR レビュー・マージ
- [ ] #24 PR レビュー・マージ

```bash
cargo test --all
cargo clippy --all-targets
```

### Day 5 (Fri) - Sprint Review

**Tasks**:
- [ ] 全PRマージ確認
- [ ] デモ準備
- [ ] Sprint 2準備

## ✅ Acceptance Criteria

- [ ] `MarkdownStream` が文字を受け取り状態遷移できる
- [ ] `DiffRender` がunified diffをパースできる
- [ ] `AnthropicClient` がAPIに接続しストリーム受信できる
- [ ] `Tool` traitが定義され、registryに登録できる
- [ ] 全テストがパス
- [ ] Clippyエラーなし

## 🚧 Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| API接続失敗 | モック作成、retry実装 |
| 設計変更 | 早期レビュー |

---

# Sprint 2: Parser & State

**Period**: December 2-6, 2025
**Goal**: Incremental parsing and state management

## 🎯 Sprint Goal

> パーサーと状態管理を実装し、基盤構造を完成させる

## 📋 Sprint Backlog

| Issue | Task | Priority | Points | Depends |
|-------|------|----------|--------|---------|
| #11 | Incremental markdown parser | P0 | 5 | #10 |
| #16 | Diff visualization | P1 | 5 | #15 |
| #20 | Conversation state | P0 | 5 | #19 |
| #25 | File operation tools | P0 | 5 | #24 |

**Total Points**: 20

## 📅 Daily Breakdown

### Day 1 (Mon) - Kickoff

```bash
miyabi agent run coordinator --issue 11 &
miyabi agent run coordinator --issue 16 &
miyabi agent run coordinator --issue 20 &
miyabi agent run coordinator --issue 25 &
```

**Tasks**:
- [ ] #11 開始 - pulldown-cmark統合
- [ ] #16 開始 - Color scheme実装
- [ ] #20 開始 - Message history構造
- [ ] #25 開始 - ReadTool実装

### Day 2 (Tue) - Core Features

**Tasks**:
- [ ] #11 継続 - Partial parsing
- [ ] #16 継続 - Line numbers、indicators
- [ ] #20 継続 - Serialization
- [ ] #25 継続 - WriteTool、EditTool

### Day 3 (Wed) - Completion

**Tasks**:
- [ ] #11 完了 - Event→Span変換
- [ ] #16 完了 - 全視覚要素
- [ ] #20 完了 - Persistence
- [ ] #25 完了 - 全File tools

### Day 4 (Thu) - Review

**Tasks**:
- [ ] PRレビュー
- [ ] 統合テスト
- [ ] Bug fix

### Day 5 (Fri) - Sprint Review

**Tasks**:
- [ ] 全PRマージ
- [ ] v0.2.0リリース準備
- [ ] Sprint 3準備

## ✅ Acceptance Criteria

- [ ] Markdown streaming が部分コンテンツを正しくレンダリング
- [ ] Diff表示が色分け、行番号付き
- [ ] Conversation履歴が保存・復元可能
- [ ] File tools (Read/Write/Edit) が全て動作
- [ ] 全テストパス

---

# Sprint 3: Integration

**Period**: December 9-13, 2025
**Goal**: LLM tools and advanced features

## 🎯 Sprint Goal

> LLMツール連携と高度な機能を実装する

## 📋 Sprint Backlog

| Issue | Task | Priority | Points | Depends |
|-------|------|----------|--------|---------|
| #12 | Code block handling | P1 | 3 | #11 |
| #17 | Syntax highlighting in diff | P2 | 3 | #16 |
| #21 | Token management | P1 | 5 | #20 |
| #22 | Tool definitions | P0 | 5 | #20 |
| #26 | Bash tool | P0 | 5 | #25 |

**Total Points**: 21

## 📅 Daily Breakdown

### Day 1 (Mon) - Kickoff

```bash
miyabi agent run coordinator --issue 12 &
miyabi agent run coordinator --issue 17 &
miyabi agent run coordinator --issue 21 &
miyabi agent run coordinator --issue 22 &
miyabi agent run coordinator --issue 26 &
```

### Day 2 (Tue) - Implementation

**Focus**: #22 (Tool definitions) - CRITICAL PATH

### Day 3 (Wed) - Integration

**Tasks**:
- [ ] Tool definitions と API client統合
- [ ] Bash tool と File tools統合

### Day 4 (Thu) - Testing

**Tasks**:
- [ ] End-to-end テスト
- [ ] Tool実行テスト

### Day 5 (Fri) - Sprint Review

**Milestone**: v0.3.0 機能フリーズ

## ✅ Acceptance Criteria

- [ ] Code blockがシンタックスハイライト付きでレンダリング
- [ ] DiffにもSyntax highlighting適用
- [ ] Token count表示、自動pruning動作
- [ ] Tool definitionsがAPI requestに含まれる
- [ ] Bash toolがコマンド実行、結果返却

---

# Sprint 4: Production Ready

**Period**: December 16-20, 2025
**Goal**: Polish and production readiness

## 🎯 Sprint Goal

> プロダクション品質に仕上げ、v1.0.0をリリースする

## 📋 Sprint Backlog

| Issue | Task | Priority | Points | Depends |
|-------|------|----------|--------|---------|
| #13 | Cursor tracking | P2 | 2 | #12 |
| #14 | Markdown tests | P2 | 3 | #13 |
| #18 | Diff navigation | P2 | 2 | #17 |
| #23 | Rate limiting | P2 | 3 | #22 |
| #27 | Search tools | P1 | 5 | #26 |
| #28 | Result formatting | P2 | 3 | #27 |

**Total Points**: 18

## 📅 Daily Breakdown

### Day 1 (Mon) - Kickoff

```bash
miyabi agent run coordinator --issue 13 &
miyabi agent run coordinator --issue 18 &
miyabi agent run coordinator --issue 23 &
miyabi agent run coordinator --issue 27 &
```

### Day 2 (Tue) - Completion

**Tasks**:
- [ ] #13 完了
- [ ] #14 開始・完了
- [ ] #18 完了
- [ ] #23 完了
- [ ] #27 完了
- [ ] #28 開始

### Day 3 (Wed) - Final Integration

**Tasks**:
- [ ] #28 完了
- [ ] Full integration test
- [ ] Performance testing

### Day 4 (Thu) - QA

**Tasks**:
- [ ] Bug fixes
- [ ] Documentation
- [ ] Release notes

### Day 5 (Fri) - Release

**Tasks**:
- [ ] v1.0.0 タグ作成
- [ ] Release作成
- [ ] 🎉 Celebration!

## ✅ Acceptance Criteria

- [ ] 全機能が動作
- [ ] ドキュメント完備
- [ ] パフォーマンス基準達成
- [ ] セキュリティチェック完了
- [ ] v1.0.0 リリース

---

## 📊 Sprint Metrics

### Velocity Tracking

| Sprint | Planned | Completed | Velocity |
|--------|---------|-----------|----------|
| 1 | 16 | - | - |
| 2 | 20 | - | - |
| 3 | 21 | - | - |
| 4 | 18 | - | - |

**Total**: 75 points

### Burndown

Sprint終了時に更新

---

## 🔄 Ceremonies

### Daily Standup (10min)

```bash
# 毎朝実行
miyabi status
gh issue list --label "🏗️ state:implementing"
```

### Sprint Planning (始め)

1. Sprint Goal確認
2. Backlog確定
3. タスク割り当て
4. リスク確認

### Sprint Review (終わり)

1. 完了タスクデモ
2. フィードバック収集
3. 次Sprint調整

### Sprint Retrospective

- What went well?
- What to improve?
- Action items

---

## 🚀 Execution Commands

### Sprint 1 Quick Start

```bash
cd /Users/shunsuke/Dev/mergegate

# Critical Path開始
miyabi agent run coordinator --issue 19  # API Client
miyabi agent run coordinator --issue 24  # Tool Trait

# Parallel tasks
miyabi agent run coordinator --issue 10  # Markdown Core
miyabi agent run coordinator --issue 15  # Diff Core
```

### Progress Check

```bash
# 完了タスク
gh issue list --state closed --label "✅ state:done"

# 進行中
gh issue list --label "🏗️ state:implementing"

# PRs
gh pr list --state open
```

---

## 📝 Notes

- 各SprintでCritical Pathを優先
- ブロッカーは即座にエスカレーション
- PRは24時間以内にレビュー
- テストは継続的に実行

---

**Let's build something amazing! 🚀**
