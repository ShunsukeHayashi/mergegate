# Sprint 1: 品質確定（今日）

> Phase C + clippy 完了 → v1.0 タグ

## 1.1 verify_merge ラッパー (~50行)

- [ ] `crates/miyabi-core/src/protocol.rs` に `verify_merge()` を追加
  - 既存の `get_pull_request()` (github.rs) を呼ぶ
  - `PullRequest.merge_commit_sha` を取得
  - `pr.state == "merged"` を検証
  - SHA が 40hex であることを検証
  - tasks.json 更新: merge_commit + state → done
  - ロック解放
  - 後続タスク blocked → pending
- [ ] CLI に `miyabi gate verify-merge <task-id>` サブコマンド追加
- [ ] テスト: mock PullRequest で merged → done 遷移

## 1.2 escape hatch (~50行)

- [ ] `protocol.rs` に `force_unlock()` 追加
  ```
  fn force_unlock(&self, task_id: &str, reason: &str, operator: &str) -> Result<()>
  ```
  - ロック即解放
  - event log に reason + operator を記録
  - state は変更しない（implementing のまま）
- [ ] `protocol.rs` に `manual_complete()` 追加
  ```
  fn manual_complete(&self, task_id: &str, reason: &str, operator: &str) -> Result<()>
  ```
  - PR/merge なしで done に遷移
  - event log に reason + operator + "manual" を記録
  - CompletionMode::Manual として区別
- [ ] CLI に `miyabi gate force-unlock <task-id> --reason R --operator O` 追加
- [ ] CLI に `miyabi gate manual-complete <task-id> --reason R --operator O` 追加
- [ ] テスト: force_unlock → ロック解放確認
- [ ] テスト: manual_complete → done 遷移 + event 記録

## 1.3 E2E テスト (~100行)

- [ ] `crates/miyabi-core/src/protocol.rs` の tests モジュールに E2E 追加
  ```
  #[test]
  fn full_lifecycle_register_to_done() {
      // 1. register(issue=1, title="test")
      // 2. check_dependencies → ready
      // 3. record_impact(risk=LOW)
      // 4. assign_and_lock(agent="test", files=["src/test.rs"])
      // 5. record_branch("feature/issue-1-test")
      // 6. record_pr(42)
      // 7. record_merge("a1b2c3d4...40hex")
      // 8. assert: state == Done, lock == None
  }
  ```
- [ ] GATE 拒否テスト: issue=0 → GateError
- [ ] GATE 拒否テスト: impact なしで assign → GateError
- [ ] GATE 拒否テスト: HIGH risk + 承認なし → GateError
- [ ] GATE 拒否テスト: ロック競合 → LockError
- [ ] GATE 拒否テスト: 不正 SHA → GateError
- [ ] escape hatch テスト: force_unlock + manual_complete

## 1.4 最終確認

- [ ] `cargo test --all` → 全 GREEN
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` → ゼロエラー
- [ ] `cargo build --release` → リリースビルド成功
- [ ] `npx gitnexus analyze --force` → 再インデックス
- [ ] `git tag v1.0-dtp-complete`
- [ ] `git push origin v1.0-dtp-complete`
- [ ] `~/bin/announce "Polaris v1.0 完成。全 GATE 実装済み。テスト GREEN。"`
