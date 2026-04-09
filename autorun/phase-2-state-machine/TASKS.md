# Phase 2: ステートマシンに GATE predicate を統合

> 依存: Phase 1 GREEN
> 承認ゲート: 全遷移ルールにテスト + 不正遷移が全て拒否される

## 現状

`src/state.rs` は `can_transition(from, to) -> bool` の単純な matches! 式。
conditions/predicates の実評価がない（Codex R1-1 の致命的指摘）。

## タスク

- [ ] `TaskStateMachine` に `Merged` と `AwaitingGithubSync` の遷移ルールを追加
- [ ] `can_transition()` を `can_transition(task: &ManagedTask, to: TaskState) -> Result<(), GateError>` に変更
  - draft → pending: title 非空 + github_issue_number > 0
  - pending → ready: dependencies.all(done) （DAG 経由で検証）
  - ready → analyzing: 無条件
  - analyzing → implementing: impact.is_some() + lock.is_some() + HIGH/CRITICAL は human_approval
  - implementing → reviewing: branch_name.is_some() + pr_number > 0
  - reviewing → merged: github_evidence.pr_state == MERGED + merge_commit_sha が 40hex
  - merged → done: github_evidence.issue_state == CLOSED
- [ ] `skip_analysis` ルール（pending → implementing）を削除（R1-7）
- [ ] 不正遷移テスト: analyzing → implementing を impact なしで試みる → GateError
- [ ] 不正遷移テスト: reviewing → merged を不正 SHA で試みる → GateError
- [ ] 不正遷移テスト: merged → done を Issue Open で試みる → GateError
- [ ] 全合法遷移の happy path テスト

## 承認ゲート

- `cargo test` 全 GREEN
- 不正遷移テストが最低 5 件存在し、全て正しく拒否される
- `can_transition()` が ManagedTask を受け取る形に変更されている

## リトライ条件

- 既存テスト破壊 → protocol.rs のテストが新しい can_transition() に対応しているか確認
- gate.rs との責務重複 → gate.rs は個別 GATE 関数、state.rs は遷移の orchestration と明確に分離
