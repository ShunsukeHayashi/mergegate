# MiyabiGate — 確定的タスク実行スキル

## バイナリ

**`~/bin/miyabi-gate`** （全ノードにインストール済み）

## 概要

Polaris (DTP) の公式 GATE CLI。タスクの登録・ロック・検証・完了を確定的に実行する。
ファイル編集前に必ず `miyabi-gate gate assign` でロックを取得すること。

Claude Code では、このスキルを `MiyabiGate` として扱う。
`miyabi` は別の hub CLI なので、Polaris の GATE 操作には使わず、標準コマンドは `miyabi-gate gate ...` とする。
`miyabi gate` と打つと別 CLI の help を見に行くので、誤って使わないこと。

## クイックスタート

```bash
miyabi-gate gate init                                    # 初回のみ
miyabi-gate gate register --issue 1 --title "タスク名"  # タスク登録
miyabi-gate gate impact issue-1 --risk low --symbols 0  # 影響分析
miyabi-gate gate assign issue-1 --agent claude --node macbook --files "src/main.rs"  # ロック獲得
# → 作業実施
miyabi-gate gate manual-complete issue-1 --reason "完了" --operator claude  # 完了
```

## トリガー

MiyabiGate, miyabi gate, miyabi-gate, polaris, dtp, タスク登録, ロック, assign, GATE, 確定的, deterministic, ファイルロック, 依存関係, DAG

## 前提

- バイナリ: `cargo build --release` で `target/release/miyabi` を生成
- ワークスペース: `/Users/shunsukehayashi/dev/platform/miyabi-cli-standalone`
- tasks.json: `project_memory/tasks.json`（デフォルト）

## 公式の使い分け

- `MiyabiGate`: task の register、impact、assign、branch、pr、merge を進める
- `Polaris`: Phase 実行、品質チェック、handoff を進める
- `polaris-gate` rule: `miyabi-cli-standalone` での必須順序を強制する

## コマンド一覧

### タスク登録

```bash
miyabi-gate gate register --issue 45 --title "認証移行"
miyabi-gate gate register --issue 45 --title "認証移行" --format json
miyabi-gate gate register --issue 45 --title "認証移行" --store-path /path/to/tasks.json
```

### タスク状態確認

```bash
miyabi-gate gate status                    # 全タスク一覧
miyabi-gate gate status task-001           # 特定タスク
miyabi-gate gate status --format json      # JSON出力
```

### 実行可能タスク

```bash
miyabi-gate gate dispatchable              # DAG依存解決済みのタスク
miyabi-gate gate dispatchable --format json
```

### DAG 可視化

```bash
miyabi-gate gate dag                       # DAGレベル表示
```

### impact 記録

```bash
miyabi-gate gate impact task-001 --risk LOW --symbols 3
miyabi-gate gate impact task-001 --risk HIGH --symbols 12  # → 人間承認が必要
miyabi-gate gate impact task-001 --risk HIGH --symbols 12 --approve  # 承認付き
```

### ロック獲得 + 実装開始

```bash
miyabi-gate gate assign task-001 --agent claude --node macbook --files "src/auth.rs,src/middleware.rs"
```

### ブランチ記録

```bash
miyabi-gate gate branch task-001 feature/issue-45-auth
```

### PR 記録

```bash
miyabi-gate gate pr task-001 78
```

### merge 検証 + 完了

```bash
miyabi-gate gate merge task-001 a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2
```

### ロック一覧

```bash
miyabi-gate gate locks                     # 現在のロック一覧
miyabi-gate gate locks --format json
```

### escape hatch

```bash
miyabi-gate gate force-unlock task-001 --reason "agent crashed" --operator hayashi
miyabi-gate gate manual-complete task-001 --reason "doc task, no PR" --operator hayashi
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
miyabi-gate gate register --issue 45 --title "認証移行" --format json

# 状態確認
miyabi-gate gate status task-001 --format json

# ロック獲得
miyabi-gate gate assign task-001 --agent claude --node macbook --files "src/auth.rs"
```

### Codex から

```bash
# Codex は --format json で結果をパースする
result=$(miyabi-gate gate register --issue 45 --title "test" --format json)
exit_code=$?
if [ $exit_code -eq 0 ]; then
  task_id=$(echo "$result" | jq -r '.task_id')
  miyabi-gate gate assign "$task_id" --agent codex --node macbook --files "src/test.rs"
fi
```

### OpenClaw から

```bash
# OpenClaw main がディスパッチ
miyabi-gate gate dispatchable --format json | jq -r '.[0].id'
# → サブエージェントに渡す
```

## Claude Code での正式運用

1. `miyabi-gate gate register` で task を作る
2. `miyabi-gate gate impact` で risk を記録する
3. `miyabi-gate gate assign` でロックを取ってから編集する
4. 実装後に `miyabi-gate gate branch` と `miyabi-gate gate pr` を記録する
5. merge 済みなら `miyabi-gate gate merge`、PR を伴わない文書作業なら `manual-complete` を使う

`miyabi-cli-standalone` では `polaris-gate` rule が前提なので、assign 前の編集は不可。

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
- `polaris-gate` rule — `miyabi-cli-standalone` での必須運用ルール
