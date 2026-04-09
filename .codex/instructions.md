# Codex Instructions for Polaris

## 最重要ルール: miyabi gate を使え

タスクを始める前に必ず:
1. `miyabi gate register --issue <N> --title "タスク名"`
2. `miyabi gate impact <task-id> --risk <level> --symbols <N>`
3. `miyabi gate assign <task-id> --agent codex --node macbook --files "変更ファイル"`

ファイルを編集した後:
4. `miyabi gate branch <task-id> feature/issue-<N>-<slug>`
5. `miyabi gate pr <task-id> <PR番号>`
6. `miyabi gate merge <task-id> <SHA>`

## 禁止

- assign せずにファイルを編集しない（pre-commit hook が拒否する）
- 他のエージェントがロック中のファイルに触らない
- Issue なしで作業を始めない

## 確認

```bash
miyabi gate status           # 全タスク
miyabi gate locks            # ロック中ファイル
miyabi gate dispatchable     # 実行可能タスク
miyabi gate dag              # 依存関係
```

## 品質

```bash
cargo test --all
cargo clippy --all-targets --all-features -- -D warnings
```

## コミット

日本語タグ形式: `[追加]`, `[修正]`, `[改善]`, `[整備]`, `[文書]`, `[検証]`, `[完了]`
Issue 番号を含める: `[修正] GATE 0 拒否 (#52)`
