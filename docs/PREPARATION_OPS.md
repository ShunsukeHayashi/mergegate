# Preparation Operations - miyabi-cli-standalone

**Purpose**: Sprint 1開始前の準備作業チェックリスト
**Target Date**: 2025-11-25 (Sprint 1 Start)

---

## 📋 Pre-Sprint Checklist

### 1. Environment Setup

#### 1.1 Development Environment

- [ ] **Rust toolchain**
  ```bash
  rustup update stable
  rustup component add clippy rustfmt
  cargo --version  # 1.75+
  ```

- [ ] **GitHub CLI**
  ```bash
  gh --version
  gh auth status
  ```

- [ ] **Miyabi CLI**
  ```bash
  /Users/shunsuke/Dev/01-miyabi/_core/miyabi-private/target/release/miyabi --version
  ```

#### 1.2 API Keys

- [ ] **GitHub Token**
  ```bash
  # .env に設定
  echo "GITHUB_TOKEN=ghp_xxx" >> .env

  # 必要なスコープ: repo, workflow
  gh auth refresh -s repo,workflow
  ```

- [ ] **Anthropic API Key**
  ```bash
  echo "ANTHROPIC_API_KEY=sk-ant-xxx" >> .env

  # テスト
  curl https://api.anthropic.com/v1/messages \
    -H "x-api-key: $ANTHROPIC_API_KEY" \
    -H "anthropic-version: 2023-06-01" \
    -d '{"model":"claude-sonnet-4-20250514","max_tokens":10,"messages":[{"role":"user","content":"Hi"}]}'
  ```

#### 1.3 Project Dependencies

- [ ] **Node.js dependencies**
  ```bash
  npm install
  ```

- [ ] **Cargo dependencies** (初回ビルドで取得)
  ```bash
  cargo fetch
  ```

---

### 2. Repository Setup

#### 2.1 Labels Verification

- [ ] **必要なラベル確認**
  ```bash
  gh label list --repo ShunsukeHayashi/miyabi-cli-standalone | wc -l
  # Expected: 45+ labels
  ```

- [ ] **不足ラベル追加**
  ```bash
  # 必要に応じて
  gh label create "🔥 urgent" --color "d73a4a" --description "Urgent issue"
  ```

#### 2.2 Milestones Verification

- [ ] **マイルストーン確認**
  ```bash
  gh api repos/ShunsukeHayashi/miyabi-cli-standalone/milestones | jq '.[].title'
  ```

  Expected:
  - v0.2.0 - Advanced Text Rendering
  - v0.3.0 - Chat Composer
  - v0.4.0 - Approval & Navigation
  - v1.0.0 - Production Ready

#### 2.3 Issues Verification

- [ ] **Issue一覧確認**
  ```bash
  gh issue list --repo ShunsukeHayashi/miyabi-cli-standalone --state all | wc -l
  # Expected: 28 issues
  ```

- [ ] **Sprint 1タスク確認**
  ```bash
  gh issue list --milestone "v0.2.0 - Advanced Text Rendering" --label "📊 priority:P0-Critical"
  ```

---

### 3. Codebase Preparation

#### 3.1 Directory Structure

- [ ] **必要ディレクトリ作成**
  ```bash
  mkdir -p crates/miyabi-cli/src
  mkdir -p crates/miyabi-core/src
  mkdir -p crates/miyabi-tui/src
  mkdir -p tests
  ```

#### 3.2 Cargo.toml Setup

- [ ] **ワークスペース設定確認**
  ```bash
  cat Cargo.toml | grep "members"
  ```

- [ ] **各crateのCargo.toml確認**
  ```bash
  ls crates/*/Cargo.toml
  ```

#### 3.3 Initial Build Test

- [ ] **ビルドテスト**
  ```bash
  cargo build
  cargo test
  cargo clippy
  ```

---

### 4. CI/CD Setup

#### 4.1 GitHub Actions

- [ ] **ワークフロー確認**
  ```bash
  ls .github/workflows/
  ```

- [ ] **CI設定** (create if not exists)
  ```yaml
  # .github/workflows/ci.yml
  name: CI
  on: [push, pull_request]
  jobs:
    build:
      runs-on: ubuntu-latest
      steps:
        - uses: actions/checkout@v4
        - uses: dtolnay/rust-toolchain@stable
        - run: cargo build --all
        - run: cargo test --all
        - run: cargo clippy --all -- -D warnings
  ```

#### 4.2 Branch Protection

- [ ] **main ブランチ保護**
  - Require PR reviews
  - Require status checks
  - Require linear history

---

### 5. Documentation

#### 5.1 Required Documents

- [x] README.md
- [x] CLAUDE.md
- [x] GOALS.md
- [x] docs/WBS.md
- [x] docs/SPRINT_PLANNING.md
- [x] docs/OPERATION_PLAN.md
- [x] docs/PRODUCT_SPEC.md
- [x] docs/PREPARATION_OPS.md (this file)

#### 5.2 PlantUML Diagrams

- [x] docs/wbs-diagram.puml
- [x] docs/gantt-chart.puml

---

### 6. Reference Materials

#### 6.1 Codex CLI Reference

- [ ] **Clone Codex CLI**
  ```bash
  git clone https://github.com/openai/codex.git /tmp/codex-reference
  ```

- [ ] **主要ファイル確認**
  ```bash
  ls /tmp/codex-reference/codex-rs/tui/src/
  ```

- [ ] **参照用ブックマーク**
  - markdown_stream.rs
  - diff_render.rs
  - chat_composer.rs
  - textarea.rs

#### 6.2 Miyabi Private Reference

- [ ] **Agent実装参照**
  ```bash
  ls /Users/shunsuke/Dev/01-miyabi/_core/miyabi-private/crates/
  ```

---

### 7. Team Communication

#### 7.1 Notification Setup

- [ ] **Slack/Discord通知** (if applicable)

- [ ] **GitHub通知設定**
  - Watch repository
  - Enable PR notifications

#### 7.2 Daily Standup Schedule

- [ ] **Standup時間設定**
  - Time: 毎朝 9:00
  - Duration: 10分
  - Format: miyabi status確認

---

### 8. Monitoring Setup

#### 8.1 Miyabi Monitoring

- [ ] **ログディレクトリ確認**
  ```bash
  mkdir -p logs
  mkdir -p reports
  ```

- [ ] **ログローテーション設定**
  ```bash
  # 30日以上のログを削除
  find logs -mtime +30 -delete
  ```

#### 8.2 GitHub Insights

- [ ] **Pulse確認**
  ```
  https://github.com/ShunsukeHayashi/miyabi-cli-standalone/pulse
  ```

---

### 9. Sprint 1 Kickoff Preparation

#### 9.1 First Tasks Ready

- [ ] **Issue #10 (MarkdownStream core)**
  ```bash
  gh issue view 10 --repo ShunsukeHayashi/miyabi-cli-standalone
  ```

- [ ] **Issue #15 (DiffRender core)**
  ```bash
  gh issue view 15
  ```

- [ ] **Issue #19 (API client)**
  ```bash
  gh issue view 19
  ```

- [ ] **Issue #24 (Tool trait)**
  ```bash
  gh issue view 24
  ```

#### 9.2 Agent Configuration

- [ ] **Miyabi設定確認**
  ```bash
  cat .miyabi.yml
  ```

- [ ] **.claude設定確認**
  ```bash
  ls .claude/
  ls .claude/agents/specs/
  ```

---

## 🚀 Kickoff Commands

### Sprint 1 Day 1 (2025-11-25)

```bash
# 1. 環境確認
cd /Users/shunsuke/Dev/miyabi-cli-standalone
miyabi status

# 2. Critical Path 開始
miyabi agent run coordinator --issue 19  # API Client (CRITICAL)

# 3. Parallel tasks 開始
miyabi agent run coordinator --issue 10  # Markdown Core
miyabi agent run coordinator --issue 15  # Diff Core
miyabi agent run coordinator --issue 24  # Tool Trait

# 4. 進捗監視
watch -n 60 'gh issue list --label "🏗️ state:implementing"'
```

---

## ✅ Final Checklist

### Must Have (Sprint開始前)

- [ ] Rust toolchain ready
- [ ] API keys configured
- [ ] GitHub labels/milestones set
- [ ] Initial crate structure
- [ ] CI workflow active

### Should Have

- [ ] Codex CLI reference cloned
- [ ] PlantUML diagrams reviewed
- [ ] All documentation in place

### Nice to Have

- [ ] Notification integrations
- [ ] Performance baselines

---

## 📞 Emergency Contacts

| Issue | Action |
|-------|--------|
| API Key invalid | Regenerate at console.anthropic.com |
| GitHub rate limit | Wait 1 hour or use different token |
| Build failure | Check Cargo.toml, dependencies |
| Agent stuck | Check logs, reset issue state |

---

## 📝 Sign-off

| Item | Status | Date | Notes |
|------|--------|------|-------|
| Environment | ⬜ | | |
| Repository | ⬜ | | |
| Codebase | ⬜ | | |
| CI/CD | ⬜ | | |
| Documentation | ✅ | 2025-11-22 | Complete |
| References | ⬜ | | |
| Monitoring | ⬜ | | |

---

**Ready for Sprint 1? Let's go! 🚀**
