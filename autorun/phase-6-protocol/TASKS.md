# Phase 6: Protocol 統合（全 GATE を1クラスに結合）

> 依存: Phase 2 + 3 + 4 + 5 全て GREEN
> 承認ゲート: E2E happy path + 全 GATE 拒否テスト + escape hatch テスト GREEN
> これが核心。全コンポーネントをここでつなぐ。

## 現状

`src/protocol.rs` に DeterministicExecutionProtocol が実装済み。
register → check_dependencies → record_impact → assign_and_lock → record_branch → record_pr → record_merge の一連が動く。
ただし Phase 1-5 の拡張がまだ統合されていない。

## タスク

- [ ] Protocol に EventStore を統合: 全 GATE 通過/拒否を event として記録
- [ ] Protocol に SnapshotStore を統合: 状態遷移を即座に永続化
- [ ] Protocol に FileLockManager の atomic acquire を統合
- [ ] Protocol に GitHubEvidenceFetcher を統合: verify_merge() で evidence を取得
- [ ] `verify_merge()`: record_merge() を置き換え。evidence fetcher 経由で:
  1. PR API から GitHubEvidence を取得
  2. pr_state == MERGED を検証
  3. merge_commit_sha が 40hex を検証
  4. review_decision == APPROVED を検証
  5. evidence を task に保存
  6. Reviewing → Merged に遷移
- [ ] `confirm_done()`: Merged → Done の遷移。evidence fetcher 経由で:
  1. issue_state == CLOSED を検証
  2. issue_closed_by_pr == true を検証（CompletionMode::GithubPr の場合）
  3. worklog に記録
  4. Done に遷移
- [ ] `heartbeat()`: lease 更新メソッド
- [ ] Escape hatches（R3-5）:
  - `force_unlock(task_id, reason, operator)`: 監査付き強制ロック解放
  - `manual_complete(task_id, reason, operator)`: PRなし完了（CompletionMode::Manual）
  - `reconcile_from_github(task_id)`: GitHub の状態を強制 pull
  - 全 escape hatch は event log に `reason` + `operator` + `timestamp` を記録
- [ ] テスト: E2E happy path（draft → ... → done）全シーケンス
- [ ] テスト: GATE 0 拒否（Issue なし → 登録拒否）
- [ ] テスト: GATE 2 拒否（依存未解決 → blocked）
- [ ] テスト: GATE 3 拒否（impact なし → implementing 拒否）
- [ ] テスト: GATE 3 拒否（HIGH risk + 承認なし → 拒否）
- [ ] テスト: GATE 4 拒否（ロック競合 → 拒否）
- [ ] テスト: GATE 5 拒否（ブランチ名不正 → 拒否）
- [ ] テスト: GATE 7 拒否（不正 SHA → merged 拒否）
- [ ] テスト: escape hatch: force_unlock → event log に記録
- [ ] テスト: escape hatch: manual_complete → Done with reason
- [ ] テスト: 2タスクの DAG 依存: task-0 done → task-1 ready → task-1 実行可能

## 承認ゲート

- `cargo test` 全 GREEN（20 件以上）
- E2E happy path テスト GREEN
- 全 GATE 拒否テスト GREEN（8 件以上）
- escape hatch テスト GREEN（2 件以上）
- event log に全操作が記録されている

## リトライ条件

- Protocol と StateMachine の責務境界で混乱 → Protocol が唯一の外部窓口、StateMachine は internal
- EventStore と SnapshotStore の初期化順序 → Protocol::new() で両方を初期化
