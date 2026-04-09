# miyabi gate CLI — 確定的タスク実行スキル

## 概要

Polaris (DTP) の CLI インターフェース。エージェントがタスクの登録・ロック・検証・完了を確定的に実行する。

## トリガー

miyabi gate, polaris, dtp, タスク登録, ロック, GATE, 確定的, deterministic

## 前提

- バイナリ: `cargo build --release` で `target/release/miyabi` を生成
- ワークスペース: `/Users/shunsukehayashi/dev/platform/miyabi-cli-standalone`
- tasks.json: `project_memory/tasks.json`（デフォルト）

## コマンド一覧

### タスク登録

```bash
miyabi gate register --issue 45 --title "認証移行"
miyabi gate register --issue 45 --title "認証移行" --format json
miyabi gate register --issue 45 --title "認証移行" --store-path /path/to/tasks.json
```

### タスク状態確認

```bash
miyabi gate status                    # 全タスク一覧
miyabi gate status task-001           # 特定タスク
miyabi gate status --format json      # JSON出力
```

### 実行可能タスク

```bash
miyabi gate dispatchable              # DAG依存解決済みのタスク
miyabi gate dispatchable --format json
```

### DAG 可視化

```bash
miyabi gate dag                       # DAGレベル表示
```

### impact 記録

```bash
miyabi gate impact task-001 --risk LOW --symbols 3
miyabi gate impact task-001 --risk HIGH --symbols 12  # → 人間承認が必要
miyabi gate impact task-001 --risk HIGH --symbols 12 --approve  # 承認付き
```

### ロック獲得 + 実装開始

```bash
miyabi gate assign task-001 --agent codex --node macbook --files "src/auth.rs,src/middleware.rs"
```

### ブランチ記録

```bash
miyabi gate branch task-001 feature/issue-45-auth
```

### PR 記録

```bash
miyabi gate pr task-001 78
```

### merge 検証 + 完了

```bash
miyabi gate merge task-001 a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2
```

### ロック一覧

```bash
miyabi gate locks                     # 現在のロック一覧
miyabi gate locks --format json
```

### escape hatch

```bash
miyabi gate force-unlock task-001 --reason "agent crashed" --operator hayashi
miyabi gate manual-complete task-001 --reason "doc task, no PR" --operator hayashi
```

## exit code

| コード | 意味 | エージェントの対処 |
|--------|------|------------------|
| 0 | 成功 | 次のステップへ進む |
| 1 | GATE 拒否 | 理由を確認して条件を満たしてからリトライ |
| 2 | 入力エラー | コマンドを修正して再実行 |

## エージェントの使い方

### Claude Code から

```bash
# タスク登録
miyabi gate register --issue 45 --title "認証移行" --format json

# 状態確認
miyabi gate status task-001 --format json

# ロック獲得
miyabi gate assign task-001 --agent claude --node macbook --files "src/auth.rs"
```

### Codex から

```bash
# Codex は --format json で結果をパースする
result=$(miyabi gate register --issue 45 --title "test" --format json)
exit_code=$?
if [ $exit_code -eq 0 ]; then
  task_id=$(echo "$result" | jq -r '.task_id')
  miyabi gate assign "$task_id" --agent codex --node macbook --files "src/test.rs"
fi
```

### OpenClaw から

```bash
# OpenClaw main がディスパッチ
miyabi gate dispatchable --format json | jq -r '.[0].id'
# → サブエージェントに渡す
```

## GATE フロー

```
register (GATE 0: Issue必須)
  → status: pending
  → check_dependencies (GATE 2: 依存解決)
  → status: ready
  → impact (GATE 3: 分析記録 + HIGH承認)
  → status: analyzing
  → assign (GATE 4: ロック獲得)
  → status: implementing
  → branch (GATE 5: ブランチ名検証)
  → pr (GATE 6: PR番号記録)
  → status: reviewing
  → merge (GATE 7: SHA検証)
  → status: done ✅
```

## 関連スキル

- `polaris-ops` — 開発オペレーション全般
- `rust-llm-pitfalls` — Rust 開発時の注意点
- `context-and-impact` — コンテキスト収集パイプライン
- `gitnexus-impact-analysis` — GNI 影響分析
