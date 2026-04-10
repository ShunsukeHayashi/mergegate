# MergeGate CLI — Deterministic Execution Skill

## この repo での正しい入口

- 開発中: `cargo run -p mergegate-cli --bin mergegate -- gate ...`
- ビルド済み: `./target/release/mergegate gate ...`

## 概要

MergeGate (DTP) の公式 GATE CLI。タスクの登録・ロック・検証・完了を確定的に実行する。
ファイル編集前に必ず `mergegate gate assign` または `miyabi gate assign` でロックを取得すること。

この `mergegate` リポジトリでは `mergegate gate` と `miyabi gate` の両方が使える。
他環境に `miyabi-gate` という別ラッパーがあっても、この repo では `mergegate gate` を先に案内する。

## 初回ユーザー向けの最短導線

まずは次の 3 つだけ覚えればよい。

```bash
./target/release/mergegate gate status
./target/release/mergegate gate init
./target/release/mergegate gate guide
```

意味:

- `status`: すでに初期化済みか確認する
- `init`: 未導入なら `project_memory/tasks.json` を作る
- `guide`: 正しい手順をその場で読む

`tasks: 0` は異常ではなく、「ledger はあるがタスクがまだない」状態。

## クイックスタート

```bash
./target/release/mergegate gate init                                    # 初回のみ
./target/release/mergegate gate register --issue 1 --title "タスク名"  # タスク登録
./target/release/mergegate gate impact issue-1 --risk low --symbols 0  # 影響分析
./target/release/mergegate gate assign issue-1 --agent claude --node macbook --files "src/main.rs"  # ロック獲得
# → 作業実施
./target/release/mergegate gate manual-complete issue-1 --reason "完了" --operator claude  # 完了
```

## トリガー

MergeGate, mergegate gate, miyabi gate, miyabi-gate, polaris, dtp, タスク登録, ロック, assign, GATE, 確定的, deterministic, ファイルロック, 依存関係, DAG

## 前提

- バイナリ: `cargo build --release` で `target/release/mergegate` と `target/release/miyabi` を生成
- tasks.json: `project_memory/tasks.json`（デフォルト）
- repo 内の説明は `mergegate gate` を主表記として読む

## 公式の使い分け

- `mergegate-cli`: task の register、impact、assign、branch、pr、merge を進める
- `mergegate-ops`: Phase 実行、品質チェック、handoff を進める
- `polaris-gate` rule: `mergegate` での必須順序を強制する

## コマンド一覧

### タスク登録

```bash
./target/release/mergegate gate register --issue 45 --title "認証移行"
./target/release/mergegate gate --format json register --issue 45 --title "認証移行"
./target/release/mergegate gate --store-path /path/to/tasks.json register --issue 45 --title "認証移行"
```

### タスク状態確認

```bash
./target/release/mergegate gate status              # 全タスク一覧
./target/release/mergegate gate status task-001     # 特定タスク
./target/release/mergegate gate --format json status
```

### 実行可能タスク

```bash
./target/release/mergegate gate dispatchable              # DAG依存解決済みのタスク
```

### DAG 可視化

```bash
./target/release/mergegate gate dag                       # DAGレベル表示
```

### impact 記録

```bash
./target/release/mergegate gate impact task-001 --risk low --symbols 3
./target/release/mergegate gate impact task-001 --risk high --symbols 12  # → 人間承認が必要
./target/release/mergegate gate impact task-001 --risk high --symbols 12 --approve  # 承認付き
```

### ロック獲得 + 実装開始

```bash
./target/release/mergegate gate assign task-001 --agent claude --node macbook --files "src/auth.rs,src/middleware.rs"
```

### ブランチ記録

```bash
./target/release/mergegate gate branch task-001 feature/issue-45-auth
```

### PR 記録

```bash
./target/release/mergegate gate pr task-001 78
```

### merge 検証 + 完了

```bash
./target/release/mergegate gate merge task-001 a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2
```

### ロック一覧

```bash
./target/release/mergegate gate locks                     # 現在のロック一覧
./target/release/mergegate gate --format json locks
```

### コンテ��ストアタッチメント

```bash
./target/release/mergegate gate attach task-001              # アタッチ内容を表示
# 自動アタッチ対象: Issue, Impact, Obsidian notes, locked files, depth-1 impact files
```

### 学び抽出 (Dream)

```bash
./target/release/mergegate gate dream                        # パターン抽出
./target/release/mergegate gate dream --auto                 # 自動実行
```

### Web ダッシュボード

```bash
./target/release/mergegate gate serve                        # localhost:4848 でダッシュボード起動
```

### escape hatch

```bash
./target/release/mergegate gate force-unlock task-001 --reason "agent crashed" --operator hayashi
./target/release/mergegate gate manual-complete task-001 --reason "doc task, no PR" --operator hayashi
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
./target/release/mergegate gate --format json register --issue 45 --title "認証移行"

# 状態確認
./target/release/mergegate gate --format json status task-001

# ロック獲得
./target/release/mergegate gate assign task-001 --agent claude --node macbook --files "src/auth.rs"
```

### Codex から

```bash
# Codex は --format json で結果をパースする
result=$(./target/release/mergegate gate --format json register --issue 45 --title "test")
exit_code=$?
if [ $exit_code -eq 0 ]; then
  task_id=$(echo "$result" | jq -r '.task_id')
  ./target/release/mergegate gate assign "$task_id" --agent codex --node macbook --files "src/test.rs"
fi
```

### OpenClaw から

```bash
# OpenClaw main がディスパッチ
./target/release/mergegate gate dispatchable
# → サブエージェントに渡す
```

## Claude Code での正式運用

1. `mergegate gate register` で task を作る
2. `mergegate gate impact` で risk を記録する
3. `mergegate gate assign` でロックを取ってから編集する
4. 実装後に `mergegate gate branch` と `mergegate gate pr` を記録する
5. merge 済みなら `mergegate gate merge`、PR を伴わない文書作業なら `manual-complete` を使う

`mergegate` では `polaris-gate` rule が前提なので、assign 前の編集は不可。

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

- `mergegate-ops` — MergeGate の開発オペレーション全般
- `rust-llm-pitfalls` — Rust 開発時の注意点
- `context-and-impact` — コンテキスト収集パイプライン
- `gitnexus-impact-analysis` — GNI 影響分析
- `polaris-gate` rule — `mergegate` での必須運用ルール
