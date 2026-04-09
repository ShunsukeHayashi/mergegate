# Phase 5: GitHub 同期（Evidence Fetcher + Deterministic Sync）

> 依存: Phase 3 GREEN
> 承認ゲート: mock GitHub API でのテスト GREEN + degraded mode テスト GREEN

## 現状

GitHub 連携は未実装。protocol.rs の `record_merge()` は merge_commit 文字列を直接受け取るのみ。
Codex R3-1/R3-3/R3-4 指摘: GitHub API から直接 evidence を取得して検証すべき。

## タスク

- [ ] `src/github.rs` モジュールを新規作成
- [ ] `GitHubEvidenceFetcher` trait を定義:
  ```rust
  pub trait GitHubEvidenceFetcher {
      fn fetch_pr_evidence(&self, pr_number: u64) -> Result<GitHubEvidence, SyncError>;
      fn fetch_issue_state(&self, issue_number: u64) -> Result<IssueState, SyncError>;
  }
  ```
- [ ] `RealGitHubFetcher`: `gh` CLI 経由で実装（`gh pr view --json mergeCommit,state,...`）
- [ ] `MockGitHubFetcher`: テスト用
- [ ] `DeterministicSync` struct:
  - `pull_evidence(task_id)`: GitHub から evidence を取得して task に保存
  - `push_state(task_id)`: ローカル状態をラベル/Projects に反映
  - `sync_with_degraded_mode()`: API 障害時は AwaitingGithubSync に遷移
- [ ] `SyncError` enum: `ApiUnavailable`, `RateLimited`, `NotFound`, `StateMismatch`
- [ ] R3-2: `AwaitingGithubSync` → リトライキュー → API 復旧後に `Merged` に遷移
- [ ] R3-3: `Issue Closed` だけでは done にしない。`issue_closed_by_pr == true` を検証
- [ ] protocol.rs の `record_merge()` を `verify_merge()` に改名: evidence fetcher 経由で merge を検証
- [ ] テスト: MockFetcher で PR merged → evidence 取得 → Merged 遷移
- [ ] テスト: MockFetcher で API 障害 → AwaitingGithubSync 遷移 → retry → Merged
- [ ] テスト: Issue が手動 close → done に遷移しない

## 承認ゲート

- `cargo test` 全 GREEN
- mock ベースの sync テスト GREEN
- degraded mode テスト GREEN

## リトライ条件

- `gh` CLI が見つからない → PATH 確認、または HTTP client に fallback
- GitHub API rate limit → exponential backoff (1s, 2s, 4s, 8s, max 60s)
