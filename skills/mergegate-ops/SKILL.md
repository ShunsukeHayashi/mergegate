# MergeGate Ops — DTP Development Operations

## 概要

Deterministic Task Protocol (MergeGate) の開発・テスト・デプロイを自動化するスキル。
Phase 実行、品質チェック、引き継ぎ、進捗アナウンスを一括管理。

Claude Code では、このスキルは GATE 操作そのものを持たない。GATE 操作は `mergegate-cli` を使い、
このスキルはその上で Phase と品質を管理する。

## トリガー

Polaris, polaris, dtp, 確定的, deterministic, phase, gate, autorun

## 公式の役割分担

- `mergegate-cli`: task register、impact、assign、branch、pr、merge
- `mergegate-ops`: phase 実行、品質チェック、handoff、announce
- `polaris-gate` rule: `miyabi-cli-standalone` での順序強制

## Quick Start

```bash
cd /Users/shunsukehayashi/dev/platform/mergegate

# 先に GATE を進める
mergegate gate --format json status
mergegate gate dispatchable

# その後に MergeGate の品質チェックを回す
cargo build
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## コマンド

### 品質チェック（毎回実行）

```bash
# 3点セット: これが全 GREEN = 品質 OK
cargo build && cargo test && cargo clippy --all-targets --all-features -- -D warnings
```

### Phase 実行

```bash
# Phase N の TASKS.md を読んでチェックボックスを埋める
cat autorun/phase-{N}-*/TASKS.md

# GATE 確認
cat autorun/phase-{N}-*/GATE.md

# task 台帳の状態も確認
mergegate gate --format json status
mergegate gate --format json locks

# Phase 完了時にタグを打つ
git tag v0.{N+1}-phase{N}-done
git push origin v0.{N+1}-phase{N}-done
```

### 引き継ぎ

```bash
# HANDOFF_NOTE.md を作成（autorun/HANDOFF.md のテンプレートに従う）
# git commit + push
# 次のエージェントに通知
~/bin/announce "MergeGate: Phase {N} 完了。Phase {N+1} に引き継ぎます。"
```

### 進捗アナウンス

```bash
# Phase 開始
~/bin/announce "MergeGate: Phase {N} 開始。{Phase名}を実行します。"

# GATE 通過
~/bin/announce "MergeGate: Phase {N} GATE 通過。テスト全て成功。"

# エラー
~/bin/announce "MergeGate: Phase {N} エラー。{理由}。リトライ {M} 回目。"

# 完了
~/bin/announce "MergeGate: Phase {N} 完了。次は Phase {N+1} です。"
```

### ロールバック

```bash
# 安全地点に戻る
git tag  # タグ一覧を確認
git checkout v0.{N}-phase{N-1}-done
git checkout -b rollback/phase-{N}-retry
# 修正後
git checkout main && git merge rollback/phase-{N}-retry
```

### コンテキストアタッチメント

```bash
# assign 後に自動アタッチされる情報:
# - Issue 本文
# - Impact 分析結果
# - Obsidian Vault の関連ノート (OBSIDIAN_VAULT_PATH 設定時)
# - ロック対象ファイルの先頭30行
# - depth-1 impact ファイル（直接呼び出し元）の先頭30行

# 手動でアタッチ内容を確認:
mergegate gate attach <task-id>

# JSON 出力でプロンプト注入用:
mergegate gate attach <task-id>
```

### Dream (学び抽出)

```bash
# タスクイベントからパターンを抽出:
mergegate gate dream
mergegate gate dream --auto  # 自動実行（launchd 毎日 03:00）
```

### Codex ディスパッチ

```bash
# Phase N を Codex にアサイン（attach でコンテキスト自動注入）
mergegate gate attach <task-id> > /tmp/ctx.json
tmux send-keys -t %{pane} "codex 'Execute task. Context: $(cat /tmp/ctx.json | head -100)'" Enter
```

### Agent Skill Bus 記録

```bash
npx agent-skill-bus record-run \
  --agent {agent} \
  --skill mergegate-ops \
  --task "Phase {N}: {summary}" \
  --result {success|fail} \
  --score {0.0-1.0}
```

## 判断基準

| 状況 | アクション |
|------|-----------|
| cargo test GREEN + clippy GREEN | Phase 続行 |
| GATE 未登録 / assign 未実施 | `mergegate-cli` に戻る |
| cargo test FAIL | 修正してリトライ（最大3回） |
| 3回連続 FAIL | エスカレーション（人間に報告） |
| 人間が「止めろ」 | 即座に停止、commit + push |

## 関連スキル

- `context-and-impact`: コンテキスト収集（Phase A）
- `gitnexus-impact-analysis`: GNI 影響分析
- `mergegate-cli`: MergeGate の公式 GATE 操作
- `miyabi-ops`: エージェント振り分け
- `agent-skill-bus`: タスクキュー + 自己改善
