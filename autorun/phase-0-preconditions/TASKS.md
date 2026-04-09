# Phase 0: 前提条件の確定

> 承認ゲート: 全チェック GREEN でなければ Phase 1 に進まない

## 検証タスク

- [ ] `cargo build` が成功する（現在のコードベースがコンパイル通る）
- [ ] `cargo test` が全て GREEN（現在 2/2 通過済み）
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` が警告ゼロ
- [ ] GNI インデックスが fresh: `npx gitnexus status --repo deterministic-task-protocol` ← 未インデックス、Phase 1 開始前に実行
- [ ] refs/ シンボリックリンクが全て有効: `ls -la refs/` で broken link なし
- [ ] Codex 3体レビュー結果が `docs/reviews/` に存在する

## 承認ゲート

全チェックボックスが埋まったら Phase 1 へ進む。1つでも未完了なら修正してリトライ。

## リトライ条件

- clippy 警告 → 該当コードを修正して再度 clippy 実行
- テスト失敗 → 失敗テストの原因を特定して修正、再度 `cargo test`
- GNI stale → `npx gitnexus analyze --force` を実行
- シンボリックリンク切れ → 参照先パスを確認して再作成
