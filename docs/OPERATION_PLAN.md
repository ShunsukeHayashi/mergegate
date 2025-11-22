# Operation Plan - miyabi-cli-standalone

**Project**: miyabi-cli-standalone
**Created**: 2025-11-22
**Execution Method**: Miyabi Autonomous Agents

---

## 🚀 Quick Start

### Prerequisites

```bash
# 1. 環境確認
miyabi status

# 2. 環境変数設定
export GITHUB_TOKEN=ghp_xxx
export ANTHROPIC_API_KEY=sk-ant-xxx

# 3. Miyabiバイナリへのパス（オプション）
alias miyabi='/Users/shunsuke/Dev/01-miyabi/_core/miyabi-private/target/release/miyabi'
```

---

## 📋 Execution Workflow

### Phase 1: Sprint 1 開始

#### Day 1: Parallel Foundation Tasks

```bash
# Stream A: TUI
miyabi agent run coordinator --issue 10  # Core Structure
miyabi agent run coordinator --issue 15  # Diff Core

# Stream B: LLM (CRITICAL)
miyabi agent run coordinator --issue 19  # API Client

# Stream C: Tools
miyabi agent run coordinator --issue 24  # Tool Trait
```

#### 進捗監視

```bash
# ステータス確認
miyabi status

# Issue一覧
gh issue list --repo ShunsukeHayashi/miyabi-cli-standalone \
  --label "🏗️ state:implementing"

# PR確認
gh pr list --repo ShunsukeHayashi/miyabi-cli-standalone
```

---

## 🔄 Daily Operations

### Morning Routine

```bash
# 1. 前日のPRをレビュー
gh pr list --repo ShunsukeHayashi/miyabi-cli-standalone --state open

# 2. ブロッカー確認
gh issue list --label "🚫 state:blocked"

# 3. 今日のタスク開始
miyabi agent run coordinator --issue <次のissue番号>
```

### Task Execution

```bash
# 単一Issue実行
miyabi agent run coordinator --issue <番号>

# 並列実行（別ターミナルで）
miyabi agent run coordinator --issue 10 &
miyabi agent run coordinator --issue 15 &
miyabi agent run coordinator --issue 19 &
```

### Evening Review

```bash
# 完了タスク確認
gh issue list --label "✅ state:done" --state closed

# 明日の計画
gh issue list --label "📥 state:pending" --milestone "v0.2.0 - Advanced Text Rendering"
```

---

## 🎯 Sprint Execution Plan

### Sprint 1 (Week 1)

| Day | Tasks | Command |
|-----|-------|---------|
| Mon | #10, #15, #19, #24 | `miyabi agent run coordinator --issue 10` (x4 parallel) |
| Tue | Continue + Review | Review PRs, merge if ready |
| Wed | #11, #16, #20, #25 | Start dependent tasks |
| Thu | Continue + Review | Review PRs |
| Fri | Integration testing | Manual testing, bug fixes |

### Sprint 2-4

同様のパターンで継続

---

## 📊 Monitoring Commands

### Real-time Status

```bash
# Miyabiステータス
miyabi status

# GitHub Issues
gh issue list --repo ShunsukeHayashi/miyabi-cli-standalone

# ラベル別
gh issue list --label "📊 priority:P0-Critical"
gh issue list --label "🏗️ state:implementing"
gh issue list --label "👀 state:reviewing"
```

### PR Management

```bash
# PR一覧
gh pr list

# 特定PRの詳細
gh pr view <番号>

# PRをマージ
gh pr merge <番号> --squash
```

### Logs

```bash
# Miyabiログ
ls -la logs/

# 最新ログ
tail -f logs/*.log
```

---

## 🔧 Agent Commands Reference

### Coordinator Agent

```bash
# Issue分析と実装
miyabi agent run coordinator --issue <番号>

# ドライラン（実行せず計画のみ）
miyabi agent run coordinator --issue <番号> --dry-run

# 詳細ログ
RUST_LOG=debug miyabi agent run coordinator --issue <番号>
```

### CodeGen Agent (自動呼び出し)

Coordinatorが自動的にCodeGenを呼び出します。

### Review Agent (自動呼び出し)

PR作成後、自動的にレビューが実行されます。

---

## 🚨 Troubleshooting

### Agent が停止した場合

```bash
# 状態確認
miyabi status

# ログ確認
cat logs/latest.log

# Issue状態をリセット
gh issue edit <番号> --remove-label "🏗️ state:implementing"
gh issue edit <番号> --add-label "📥 state:pending"

# 再実行
miyabi agent run coordinator --issue <番号>
```

### Rate Limit エラー

```bash
# GitHub API制限確認
gh api rate_limit

# 待機後再実行
sleep 3600  # 1時間待機
miyabi agent run coordinator --issue <番号>
```

### ビルドエラー

```bash
# 手動ビルド確認
cargo build --release

# テスト実行
cargo test --all

# Lint
cargo clippy --all-targets
```

---

## 📈 Progress Tracking

### Milestone Progress

```bash
# マイルストーン一覧
gh api repos/ShunsukeHayashi/miyabi-cli-standalone/milestones

# 特定マイルストーンの進捗
gh issue list --milestone "v0.2.0 - Advanced Text Rendering"
```

### Metrics

```bash
# コード行数
find crates -name "*.rs" | xargs wc -l | tail -1

# Issue統計
gh issue list --state all --json state | jq 'group_by(.state) | map({state: .[0].state, count: length})'
```

---

## 🔄 CI/CD Integration

### GitHub Actions

PRがマージされると自動的に:
1. `cargo test` 実行
2. `cargo clippy` チェック
3. `cargo fmt` 確認

### 手動トリガー

```bash
# ワークフロー実行
gh workflow run build.yml
```

---

## 📝 Best Practices

### 1. Critical Path 優先

常にP0タスクを優先:
- #19 (API Client) → #20 → #22
- #24 (Tool Trait) → #25 → #26

### 2. 並列実行

独立したタスクは並列実行:
- Stream A (#10-14, #15-18)
- Stream B (#19-23)
- Stream C (#24-28)

### 3. 早期レビュー

PRは作成後すぐにレビュー:
```bash
gh pr review <番号> --approve
gh pr merge <番号> --squash
```

### 4. ブロッカー対応

ブロッカーは即座に対応:
```bash
gh issue list --label "🚫 state:blocked"
```

---

## 🎯 Execution Checklist

### Sprint Start

- [ ] `miyabi status` で環境確認
- [ ] 当該Sprintのタスク確認
- [ ] Critical Pathタスクを優先開始

### Daily

- [ ] PRレビュー・マージ
- [ ] ブロッカー確認
- [ ] 新タスク開始

### Sprint End

- [ ] 全PRマージ
- [ ] 統合テスト
- [ ] マイルストーン確認
- [ ] 次Sprint計画

---

## 🚀 First Execution

```bash
# 今すぐ開始
cd /Users/shunsuke/Dev/miyabi-cli-standalone

# Critical Path開始
miyabi agent run coordinator --issue 19  # API Client (最重要)
miyabi agent run coordinator --issue 10  # Core Structure
miyabi agent run coordinator --issue 24  # Tool Trait
```

---

**Happy Coding with Miyabi! 🤖**
