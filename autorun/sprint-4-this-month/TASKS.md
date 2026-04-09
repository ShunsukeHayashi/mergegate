# Sprint 4: 品質ゲート多層化（今月）

> Sprint 3 の v1.2 タグ完了後に開始

## 4.1 rust-ai-pipeline Phase 1 統合 (~50行)

- [ ] `miyabi gate assign` 後に `ai-pipeline phase1 --format json` 自動実行
- [ ] `all_passed == false` → implementing に留まる
- [ ] `failure_kind` を tasks.json に記録
- [ ] CLI に `miyabi gate quality-check <task-id>` 追加

## 4.2 proptest 拡張 (~100行)

- [ ] gate.rs: ランダムタスク状態で GATE 一貫性検証
- [ ] lock.rs: ランダムファイルセットで acquire/release 不変条件
- [ ] store.rs: ランダムイベント列で snapshot rebuild 冪等性

## 4.3 cargo-mutants (~50行設定)

- [ ] `cargo mutants` 実行
- [ ] ミューテーションスコア 80% 以上
- [ ] 殺せないミュータントにテスト追加

## 承認ゲート

- [ ] mutation score 80%+
- [ ] `git tag v1.3-quality-gates`
