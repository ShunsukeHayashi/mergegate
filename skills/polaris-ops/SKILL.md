# Polaris Ops — DTP 開発オペレーションスキル

## 概要

Deterministic Task Protocol (Polaris) の開発・テスト・デプロイを自動化するスキル。
Phase 実行、品質チェック、引き継ぎ、進捗アナウンスを一括管理。

## トリガー

polaris, dtp, 確定的, deterministic, phase, gate, autorun

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

# Phase 完了時にタグを打つ
git tag v0.{N+1}-phase{N}-done
git push origin v0.{N+1}-phase{N}-done
```

### 引き継ぎ

```bash
# HANDOFF_NOTE.md を作成（autorun/HANDOFF.md のテンプレートに従う）
# git commit + push
# 次のエージェントに通知
~/bin/announce "Polaris: Phase {N} 完了。Phase {N+1} に引き継ぎます。"
```

### 進捗アナウンス

```bash
# Phase 開始
~/bin/announce "Polaris: Phase {N} 開始。{Phase名}を実行します。"

# GATE 通過
~/bin/announce "Polaris: Phase {N} GATE 通過。テスト全て成功。"

# エラー
~/bin/announce "Polaris: Phase {N} エラー。{理由}。リトライ {M} 回目。"

# 完了
~/bin/announce "Polaris: Phase {N} 完了。次は Phase {N+1} です。"
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

### Codex ディスパッチ

```bash
# Phase N を Codex にアサイン
tmux send-keys -t %{pane} "codex 'Execute Phase {N}. Read autorun/phase-{N}-*/TASKS.md. Run cargo test when done. Write HANDOFF_NOTE.md.'" Enter
```

### Agent Skill Bus 記録

```bash
npx agent-skill-bus record-run \
  --agent {agent} \
  --skill polaris-ops \
  --task "Phase {N}: {summary}" \
  --result {success|fail} \
  --score {0.0-1.0}
```

## 判断基準

| 状況 | アクション |
|------|-----------|
| cargo test GREEN + clippy GREEN | Phase 続行 |
| cargo test FAIL | 修正してリトライ（最大3回） |
| 3回連続 FAIL | エスカレーション（人間に報告） |
| 人間が「止めろ」 | 即座に停止、commit + push |

## 関連スキル

- `context-and-impact`: コンテキスト収集（Phase A）
- `gitnexus-impact-analysis`: GNI 影響分析
- `miyabi-ops`: エージェント振り分け
- `agent-skill-bus`: タスクキュー + 自己改善
