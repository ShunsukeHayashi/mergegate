# DTP Playbook v2 — miyabi-cli-standalone 統合版

_v2: miyabi-core の既存資産を最大活用。DTP リポのゼロ実装を廃止。_

---

## North Star

> LLM の揺らぎを許さない。GATE が許可し、GitHub が確定し、merge が不可逆にする。

---

## 方針転換の理由

miyabi-core に DAG (823行), GitHub (947行), 承認 (231行), 並列実行 (451行), ワークフロー (889行), セッション永続化 (466行), エラーポリシー (536行) が **既に Rust で実装済み**。

DTP リポでゼロから書いていた Phase 2 (DAG), Phase 5 (GitHub), Phase 7 (CLI) の大部分は **不要**。

---

## 既存資産マップ（miyabi-core）

| ファイル | 行数 | DTP で使える API |
|---------|------|-----------------|
| `dag.rs` | 823 | TaskGraph, topological_sort, build_levels, TaskGraphBuilder |
| `github.rs` | 467 | GitHubClient, get_issue, get_pull_request, close_issue, create_issue |
| `github_tools.rs` | 480 | Tool registry, execute |
| `orchestration.rs` | 451 | Orchestrator, ParallelConfig, execute_sequential |
| `agent/approval.rs` | 231 | ApprovalCallback trait, AutoApproveAll, RejectHighRisk |
| `workflow.rs` | 889 | Workflow, StepResult, StepStatus, WorkflowManager |
| `error_policy.rs` | 536 | CircuitBreaker, FallbackStrategy |
| `session.rs` | 466 | SessionStorage (save/load/list) — tasks.json 永続化の参考 |
| `git.rs` | 211 | find_git_root, get_current_branch, has_uncommitted_changes |
| `openclaw.rs` | 354 | OpenClawClient, get_agents |
| `hooks.rs` | 862 | HookEvent, HookManager, execute |
| `tool.rs` | 1359 | execute_dag, execute_parallel |
| `config.rs` | 525 | 設定管理 |
| **合計** | **7654** | |

---

## 新規追加するもの（4 ファイル / 約 400行）

```
crates/miyabi-core/src/
├── gate.rs      (~100行) NEW: GATE 検証関数
├── lock.rs      (~100行) NEW: ファイルロック (lease + heartbeat)
├── protocol.rs  (~150行) NEW: DeterministicExecutionProtocol
└── store.rs     (~50行)  NEW: tasks.json 永続化
```

---

## Phase 構成（v1 の 8 Phase → v2 の 3 Phase に圧縮）

### Phase A: GATE + Lock + Store（新規 4 ファイル）

**やること**:

- [ ] `src/gate.rs` を作成
  - `ensure_issue(task) -> Result<()>`: github_issue_number > 0
  - `ensure_dependencies_done(task, done_ids) -> Result<()>`: 全依存が done
  - `ensure_impact(task, human_approved) -> Result<()>`: impact 記録済 + HIGH→承認
  - `ensure_branch_name(name) -> Result<()>`: feature/issue-N-* 形式
  - `ensure_pr_ready(task) -> Result<()>`: branch + pr_number 存在
  - `ensure_merge_commit(sha) -> Result<()>`: 40文字 hex
- [ ] `src/lock.rs` を作成
  - `FileLockManager`: acquire_lock, release_lock, has_conflict, release_expired
  - lease_duration + last_heartbeat ベース
  - `from_tasks()` で既存タスクからロック状態を復元
- [ ] `src/store.rs` を作成
  - `TaskStore`: with_file(path), load, save (atomic rename), add_task, update_task, get_task
  - tasks.json スキーマ: version, tasks[], file_locks, dag_levels
  - session.rs の save/load パターンを参考に実装
- [ ] `src/protocol.rs` を作成
  - `DeterministicExecutionProtocol`: 全 GATE を統合
  - register_task → check_dependencies → record_impact → assign_and_lock → record_branch → record_pr → record_merge
  - 各メソッドで GATE 検証 → 状態遷移 → store 永続化
- [ ] `src/lib.rs` に `pub mod gate; pub mod lock; pub mod protocol; pub mod store;` を追加
- [ ] テスト: happy path (register → ... → done)
- [ ] テスト: GATE 拒否 (issue なし, impact なし, 不正 SHA, ロック競合)
- [ ] テスト: HIGH risk → 承認なし → 拒否
- [ ] `cargo test` GREEN
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` GREEN

**担当**: Codex A (worktree)
**依存**: なし
**GATE**: test GREEN + clippy GREEN + 8 テスト以上

**参照ファイル**:
- `src/dag.rs` — TaskGraph の API（DAG 操作のインターフェース確認）
- `src/session.rs` — SessionStorage の save/load パターン
- `src/agent/approval.rs` — ApprovalCallback の承認パターン
- `src/error.rs` — エラー型の定義パターン

---

### Phase B: CLI サブコマンド追加

**やること**:

- [ ] `src/main.rs`（既存 CLI エントリ）に DTP サブコマンドを追加
  - `miyabi-cli gate register --issue N --title T`
  - `miyabi-cli gate status [task-id]`
  - `miyabi-cli gate assign <task-id> --agent A --node N --files F`
  - `miyabi-cli gate impact <task-id> --risk R --symbols N`
  - `miyabi-cli gate branch <task-id> <name>`
  - `miyabi-cli gate pr <task-id> <number>`
  - `miyabi-cli gate merge <task-id> <sha>`
  - `miyabi-cli gate locks`
  - `miyabi-cli gate dag`
  - `miyabi-cli gate dispatchable`
- [ ] `--format json` 対応（全コマンド）
- [ ] 終了コード: 0=成功, 1=GATE拒否, 2=入力不正
- [ ] `--store-path` オプション（tasks.json のパス指定）
- [ ] テスト: register → status で task 表示
- [ ] `cargo test` GREEN
- [ ] `cargo clippy` GREEN

**担当**: Codex B (worktree)
**依存**: Phase A GREEN
**GATE**: register + status が動作 + JSON 出力

**参照ファイル**:
- `src/main.rs`（既存 CLI 構造）
- 既存のサブコマンド実装パターン

---

### Phase C: GitHub Evidence + E2E

**やること**:

- [ ] `src/github.rs` の既存 `get_pull_request()` を使って merge 証跡を取得するラッパー追加
  - `fetch_merge_evidence(pr_number) -> Result<MergeEvidence>`
  - `MergeEvidence { merge_commit_sha, merged_at, state, head_ref }`
- [ ] protocol.rs に `verify_merge()` を追加（record_merge を置換）
  - GitHub API 経由で PR merged を検証
  - 障害時は `AwaitingGithubSync` 的なエラーを返す
- [ ] protocol.rs に escape hatch 追加
  - `force_unlock(task_id, reason)`
  - `manual_complete(task_id, reason)`
- [ ] E2E テスト:
  1. register → check_dependencies → record_impact → assign_and_lock
  2. record_branch → record_pr → verify_merge（mock）
  3. status → Done
- [ ] `cargo test` GREEN
- [ ] `cargo clippy` GREEN
- [ ] `npx miyabi bus record-run --skill dtp --result success`

**担当**: Claude Code (local) — GitHub API アクセスが必要
**依存**: Phase A + B GREEN
**GATE**: E2E テスト GREEN + clippy GREEN

**参照ファイル**:
- `src/github.rs` — GitHubClient (既存)
- `src/error_policy.rs` — CircuitBreaker (障害時フォールバック)
- `src/hooks.rs` — HookEvent (audit hooks)

---

## Phase DAG

```
Phase A (gate + lock + store + protocol)
    │
    ├──→ Phase B (CLI サブコマンド)
    │        │
    └────────┤
             ▼
         Phase C (GitHub Evidence + E2E)
```

Phase A と B は **A 完了後に B 開始**（直列）。
Phase C は **A + B 完了後に開始**。

---

## 時間見積もり

| Phase | 新規行 | テスト行 | 時間 |
|-------|--------|---------|------|
| A | 400 | 200 | 15分 |
| B | 150 | 50 | 10分 |
| C | 100 | 100 | 10分 |
| **合計** | **650** | **350** | **35分** |

Codex 並列なら **A(15分) → B+C 並列(10分) = 約 25分**。

---

## v1 → v2 の差分

| 項目 | v1 (8 Phase) | v2 (3 Phase) |
|------|-------------|-------------|
| 新規コード量 | 1,780行 | 1,000行 |
| Phase 数 | 8 | 3 |
| DAG 実装 | 新規 | **不要**（dag.rs 823行あり） |
| GitHub 実装 | 新規 | **不要**（github.rs 947行あり） |
| 承認実装 | 新規 | **不要**（approval.rs 231行あり） |
| 並列実行 | 新規 | **不要**（orchestration.rs 451行あり） |
| CLI 構造 | 新規 | **不要**（main.rs の既存構造に追加） |
| 推定時間 | 1〜1.5時間 | **25〜35分** |

---

## Codex レビュー反映状況

| 指摘 | v2 での対応 |
|------|-----------|
| R1-1: conditions 未評価 | Phase A: gate.rs で検証関数を独立実装、protocol.rs が唯一の窓口 |
| R1-3: 出自検証なし | Phase C: github.rs 経由で merge 証跡を検証 |
| R1-5: TTL 固定 | Phase A: lock.rs に lease_duration + last_heartbeat |
| R2-1: atomic write | Phase A: store.rs で atomic rename |
| R2-2: TOCTOU | Phase A: lock.rs の acquire で再チェック |
| R3-1: SSOT 昇格しすぎ | tasks.json は execution ledger、GitHub が authority |
| R3-4: merge commit 取得経路 | Phase C: github.rs の get_pull_request() で merge_commit_sha 取得 |
| R3-5: escape hatch | Phase C: force_unlock, manual_complete |
| Playbook R2: Phase 依存不完全 | v2 で 3 Phase に圧縮、依存が明確 |
| Playbook R2: Phase gate 不足 | 各 Phase に GATE (test + clippy) を明記 |

---

## ロールバックポイント

| Phase | タグ | 戻り方 |
|-------|------|--------|
| A 完了 | `v0.1-dtp-phase-a` | `git checkout v0.1-dtp-phase-a` |
| B 完了 | `v0.2-dtp-phase-b` | `git checkout v0.2-dtp-phase-b` |
| C 完了 | `v1.0-dtp-complete` | `git checkout v1.0-dtp-complete` |

---

## 引き継ぎ

Phase 間の引き継ぎは `autorun/HANDOFF.md` のプロシージャに従う。
v2 では Phase が 3 つだけなので、引き継ぎポイントは 2 箇所:

1. Phase A → Phase B: gate/lock/store/protocol が動く状態を確認
2. Phase B → Phase C: CLI が動く状態を確認

---

_This is the single source of truth for DTP implementation in miyabi-cli-standalone._
_Do not start coding until this Playbook is approved._
