# Phase 3: Event Store（JSONL append-only + Snapshot rebuild）

> 依存: Phase 1 GREEN
> 承認ゲート: event append + snapshot rebuild のテスト GREEN
> 並列実行可能: Phase 2 と同時実行 OK（依存は Phase 1 のみ）

## 現状

`src/store.rs` は in-memory の TaskStore。ファイル永続化なし。
Codex R2-1/R2-5 指摘: monolithic JSON は concurrent write に弱い → JSONL event log + snapshot。

## タスク

- [ ] `EventStore` struct を追加: `append(event: TaskEvent)`, `replay() -> Vec<TaskEvent>`, `replay_for_task(task_id) -> Vec<TaskEvent>`
  - ファイルパス: `project_memory/task-events.jsonl`
  - append は `OpenOptions::new().append(true).create(true)` で排他不要
- [ ] `SnapshotStore` struct を追加: `load() -> TasksDocument`, `save(doc, expected_version) -> Result<()>`, `rebuild(events) -> TasksDocument`
  - ファイルパス: `project_memory/tasks.snapshot.json`
  - R2-1: `save()` は OS flock (`fs2::FileExt::lock_exclusive()`) + version CAS + atomic rename
  - CAS: `current.version != expected_version` → Error
  - atomic rename: write to `.tmp` → `fs::rename()` → done
- [ ] TaskStore に `persist()` メソッド追加: 現在の in-memory state を snapshot に書き出し
- [ ] TaskStore に `load_from_disk()` メソッド追加: snapshot を読み込んで in-memory に復元
- [ ] `rebuild_from_events()`: event log を replay して snapshot を再構築するテスト
- [ ] CAS conflict テスト: 2回目の save が version mismatch で失敗する
- [ ] tempfile crate を使った一時ディレクトリでのテスト

## 承認ゲート

- `cargo test` 全 GREEN
- event append → replay roundtrip テスト GREEN
- snapshot save → load roundtrip テスト GREEN
- CAS conflict テスト GREEN
- rebuild テスト GREEN

## リトライ条件

- flock が macOS で動かない → `fs2` crate は macOS 対応済み、パーミッション確認
- atomic rename が cross-filesystem → 同一ディレクトリ内で tmp → rename
