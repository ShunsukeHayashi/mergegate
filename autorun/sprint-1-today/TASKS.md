# Sprint 1: 品質確定（今日）

> Phase C + clippy 完了 → v1.0 タグ

## 1.1 verify_merge ラッパー (~50行)

- [x] `crates/miyabi-core/src/protocol.rs` に `verify_merge()` を追加
  - 既存の `get_pull_request()` (github.rs) を呼ぶ
  - `PullRequest.merge_commit_sha` を取得
  - `pr.state == "merged"` を検証
  - SHA が 40hex であることを検証
  - tasks.json 更新: merge_commit + state → done
  - ロック解放
  - 後続タスク blocked → pending
- [x] CLI に `miyabi gate verify-merge <task-id>` サブコマンド追加
- [x] テスト: mock PullRequest で merged → done 遷移

## 1.2 escape hatch (~50行)

- [x] `protocol.rs` に `force_unlock()` 追加
  ```
  fn force_unlock(&self, task_id: &str, reason: &str, operator: &str) -> Result<()>
  ```
  - ロック即解放
  - event log に reason + operator を記録
  - state は変更しない（implementing のまま）
- [x] `protocol.rs` に `manual_complete()` 追加
  ```
  fn manual_complete(&self, task_id: &str, reason: &str, operator: &str) -> Result<()>
  ```
  - PR/merge なしで done に遷移
  - event log に reason + operator + "manual" を記録
  - `CompletionMode::Manual` として区別（2026-04-10 実装反映）
- [x] CLI に `miyabi gate force-unlock <task-id> --reason R --operator O` 追加
- [x] CLI に `miyabi gate manual-complete <task-id> --reason R --operator O` 追加
- [x] テスト: force_unlock → ロック解放確認
- [x] テスト: manual_complete → done 遷移 + event 記録

## 1.3 E2E テスト (~100行)

- [x] `crates/miyabi-core/src/protocol.rs` の tests モジュールに E2E 追加（`full_lifecycle_register_to_merged_releases_lock` — マージ完了時は `TaskState::Merged`）
  ```
  #[test]
  fn full_lifecycle_register_to_merged_releases_lock() {
      // register → impact → assign → branch → pr → merge(40hex)
      // assert: Merged, lock == None
  }
  ```
- [x] GATE 拒否テスト: issue=0 → GateError（既存テスト群でカバー）
- [x] GATE 拒否テスト: impact なしで assign → GateError（既存）
- [x] GATE 拒否テスト: HIGH risk + 承認なし → GateError（既存）
- [x] GATE 拒否テスト: ロック競合 → LockError（既存）
- [x] GATE 拒否テスト: 不正 SHA → GateError（既存）
- [x] escape hatch テスト: force_unlock + manual_complete

## 1.4 最終確認

- [x] `cargo test --all` → 全 GREEN
- [x] `cargo clippy --all-targets --all-features -- -D warnings` → ゼロエラー
- [x] `cargo build --release` → リリースビルド成功
- [ ] `npx gitnexus analyze --force` → 再インデックス（必要時に手動）
- [ ] `git tag v1.0-dtp-complete`（リリース判断後）
- [ ] `git push origin v1.0-dtp-complete`
- [ ] `~/bin/announce "Polaris v1.0 完成。全 GATE 実装済み。テスト GREEN。"`
