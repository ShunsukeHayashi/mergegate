# Phase 1: 型定義のハードニング

> 依存: Phase 0 GREEN
> 承認ゲート: `cargo test` GREEN + 新規テスト 5 件以上追加

## 現状

`src/types.rs` に ManagedTask, TaskState, TaskLock, TaskImpact, TasksDocument が定義済み。
基本構造は動いているが、Codex レビュー指摘の以下が未反映:

## タスク

- [ ] TaskState に `Merged` バリアントを追加（R1-4: reviewing と done の間）
- [ ] TaskState に `AwaitingGithubSync` バリアントを追加（R3-2: GitHub 障害時）
- [ ] `CompletionMode` enum を追加: `GithubPr | Manual | ExternalOp`（R3-5）
- [ ] `GitHubEvidence` struct を追加: pr_number, pr_head_ref, pr_state, merge_commit_sha, merged_at, review_decision, issue_state, issue_closed_by_pr（R3-4）
- [ ] `HumanApproval` struct を追加: required, approved_by, approved_at, reason
- [ ] ManagedTask に `completion_mode`, `github_evidence`, `human_approval` フィールド追加
- [ ] TaskImpact に `analyzed_commit`, `input_hash` フィールド追加（R1-3: 鮮度検証用）
- [ ] TaskLock に `last_heartbeat` フィールド追加（R1-5: lease + heartbeat）
- [ ] `TaskEvent` struct を追加（event log 用: id, ts, event_type, task_id, agent, node, payload）
- [ ] `GateResult` struct を追加（gate_id, passed, reason, checked_at）
- [ ] 各新型の serde roundtrip テストを追加（最低 5 件）

## 承認ゲート

- `cargo test` 全 GREEN
- `cargo clippy -- -D warnings` 警告ゼロ
- 新規テスト 5 件以上が追加されている

## リトライ条件

- コンパイルエラー → 型定義の修正、既存コードとの整合性確認
- 既存テスト破壊 → ManagedTask::new() のデフォルト値を適切に設定
