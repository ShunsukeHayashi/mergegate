# Codex Instructions — Polaris (miyabi-gate)

**バイナリ: `~/bin/miyabi-gate`**

このリポで作業する前に **必ず miyabi-gate gate を使うこと**。
使わずにファイルを編集すると pre-commit hook が拒否する。

## 初回（このリポで初めて作業する場合）

```bash
miyabi-gate gate init
```

## タスク実行フロー（必ずこの順序で実行）

```bash
# 1. タスク登録（Issue 番号必須）
miyabi-gate gate register --issue 45 --title "認証機能修正"

# 業務タスク（コード変更なし）の場合:
miyabi-gate gate register --issue 45 --title "ドキュメントレビュー" --completion-mode manual

# 2. 影響分析を記録
miyabi-gate gate impact <task-id> --risk low --symbols 3
# HIGH/CRITICAL は承認必須:
miyabi-gate gate impact <task-id> --risk high --symbols 12 --approve

# 3. ファイルロック獲得（ここから作業開始）
miyabi-gate gate assign <task-id> --agent codex --node macbook --files "src/auth.rs"
# → 実行プランが表示される。それに従う。

# 4. ロックしたファイルだけを編集する

# 5a. コードタスクの完了:
miyabi-gate gate branch <task-id> feature/issue-45-auth
miyabi-gate gate pr <task-id> 78
miyabi-gate gate merge <task-id> <40文字SHA>

# 5b. 業務タスクの完了:
miyabi-gate gate manual-complete <task-id> --reason "完了理由" --operator codex
```

## 確認コマンド

```bash
miyabi-gate gate status              # 全タスク一覧
miyabi-gate gate locks               # ロック中ファイル一覧
miyabi-gate gate dag                 # DAG 依存関係
miyabi-gate gate dispatchable        # 実行可能タスク
miyabi-gate gate attach <task-id>    # コンテキスト表示
miyabi-gate gate --format json status # JSON 出力
```

## 緊急時

```bash
miyabi-gate gate force-unlock <task-id> --reason "理由" --operator codex
```

## 禁止

- `miyabi-gate gate assign` なしでファイル編集 → pre-commit hook が拒否
- 他エージェントのロック中ファイルに触る → CLI が拒否 (exit 1)
- Issue なしでタスク開始 → GATE 0 が拒否

## exit code

- 0: 成功
- 1: GATE 拒否（条件を満たしてリトライ）
- 2: 入力エラー（コマンド修正）

## コミット規約

日本語タグ形式: `[追加]`, `[修正]`, `[改善]`, `[整備]`, `[文書]`, `[検証]`, `[完了]`
Issue 番号を含める: `[修正] 認証機能 (#45)`

## 品質チェック（コミット前）

```bash
cargo test --all
cargo clippy --all-targets --all-features -- -D warnings
```
