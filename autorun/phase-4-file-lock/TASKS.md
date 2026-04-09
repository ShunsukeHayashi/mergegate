# Phase 4: File Lock Manager（lease + heartbeat）

> 依存: Phase 3 GREEN（EventStore が必要）
> 承認ゲート: lease lifecycle + stale detection + concurrent acquire テスト GREEN

## 現状

`src/lock.rs` に FileLockManager が実装済み（acquire/release/has_conflict/release_expired）。
TTL ベースの固定期限。Codex R1-5/R2-2 指摘の heartbeat/lease が未実装。

## タスク

- [ ] TaskLock に `last_heartbeat: DateTime<Utc>` フィールド追加（Phase 1 で追加済みのはず）
- [ ] `is_expired()` を heartbeat ベースに変更: `now - last_heartbeat > lease_duration + 2 * heartbeat_interval`
- [ ] `renew_lease(task_id, agent, node)` メソッド追加: `last_heartbeat` を更新 + event 記録
- [ ] `acquire_lock()` を atomic 化（R2-2）:
  1. `snapshot_store.lock_exclusive()` で OS flock 取得
  2. snapshot 再読込
  3. `has_conflict()` 再チェック
  4. stale lock の自動解放
  5. lock 書き込み + event 記録
  6. OS flock 解放
- [ ] `LeaseConfig` struct: `lease_duration_sec: 300`, `heartbeat_interval_sec: 60`, `stale_after_missed: 2`
- [ ] テスト: lease 獲得 → heartbeat → 解放の lifecycle
- [ ] テスト: heartbeat なしで stale 検出 → 自動解放
- [ ] テスト: 同一ファイルに 2 タスクが acquire → 2つ目が LockError::Conflict
- [ ] テスト: stale lock 解放後に新タスクが acquire 成功
- [ ] proptest: ランダムなファイルセットで acquire/release を繰り返して不変条件検証

## 承認ゲート

- `cargo test` 全 GREEN
- lease lifecycle テスト GREEN
- stale detection テスト GREEN
- conflict テスト GREEN
- proptest GREEN

## リトライ条件

- heartbeat interval の計算が off-by-one → Duration の比較を `>=` で統一
- OS flock のデッドロック → `try_lock_exclusive()` + timeout fallback
