# Codex Instructions for Polaris

## 必読ファイル
1. CLAUDE.md（ルール）
2. autorun/INDEX.md（Phase DAG）
3. 該当 Phase の TASKS.md

## 品質チェック（タスク完了前に必ず実行）
```bash
cargo test && cargo clippy --all-targets --all-features -- -D warnings
```

## コミットルール
日本語タグ形式: `[追加]`, `[修正]`, `[改善]`, `[整備]`, `[文書]`, `[検証]`, `[完了]`

## 禁止事項
- unsafe コード
- GATE バイパス
- テスト RED のまま完了宣言
- git push --force

## miyabi gate CLI

バイナリ: `target/release/miyabi` または `~/bin/miyabi-gate`

```bash
# タスク登録
miyabi gate register --issue 45 --title "タスク名"

# 状態確認
miyabi gate --format json status

# ロック獲得
miyabi gate assign task-001 --agent codex --node macbook --files "src/auth.rs"

# ブランチ記録
miyabi gate branch task-001 feature/issue-45-auth

# PR 記録
miyabi gate pr task-001 78

# merge 完了
miyabi gate merge task-001 a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2

# ロック一覧
miyabi gate locks

# exit code: 0=成功, 1=GATE拒否, 2=入力エラー
```
