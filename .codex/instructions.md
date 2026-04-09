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
