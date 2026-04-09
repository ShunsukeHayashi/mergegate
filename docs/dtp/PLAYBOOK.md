# Deterministic Task Execution Protocol — Maestro Implementation Playbook

_Version: 0.1.0 | Created: 2026-04-10 | Status: PLAN (未実装)_

---

## 0. North Star

> LLM の揺らぎを許さない。tasks.json の GATE が許可し、GitHub が確定し、git merge が不可逆にする。この三段階だけが「完了」を定義する。

---

## 1. 前提条件チェックリスト（実装開始前に全て GREEN であること）

### 1.1 リポジトリ・インデックス

| # | 条件 | 検証コマンド | 状態 |
|---|------|------------|------|
| 1 | openclaw-workspace GNI インデックス fresh | `npx gitnexus status --repo openclaw-workspace` | GREEN (50,130 nodes) |
| 2 | miyabi-private GNI インデックス fresh | `npx gitnexus status --repo miyabi-private` | GREEN (12,448 nodes) |
| 3 | rust-ai-pipeline GNI インデックス fresh | `npx gitnexus status --repo rust-ai-pipeline` | GREEN (813 nodes) |
| 4 | agent-skill-bus GNI インデックス fresh | `npx gitnexus status --repo agent-skill-bus` | GREEN (154 nodes) |

### 1.2 既存テスト GREEN

| # | パッケージ | コマンド | 状態 |
|---|-----------|---------|------|
| 5 | @miyabi/task-manager | `cd Miyabi/packages/task-manager && npm test` | 要確認 |
| 6 | rust-ai-pipeline | `cd ~/dev/products/rust-ai-pipeline && cargo test` | 要確認 |
| 7 | agent-skill-bus | `cd ~/dev/tools/agent-skill-bus && npm test` | 要確認 |

### 1.3 Codex 3体レビュー反映

| # | 指摘 | 対応方針 | 反映先 Phase |
|---|------|---------|-------------|
| R1-1 | canTransition() が conditions を評価していない | Protocol を唯一の遷移窓口にする | Phase 2 |
| R1-2 | applyTransition() の結果が store に永続化されない | store-backed immutable update に統一 | Phase 3 |
| R1-3 | GATE が「存在チェック」で「出自検証」がない | GitHub API で PR/merge を実検証 | Phase 5 |
| R1-4 | reviewing → done が既存 27 ルールと衝突 | `merged` 状態を追加 | Phase 2 |
| R1-5 | TTL 7200s 固定は不適切 | lease 300s + heartbeat 60s に変更 | Phase 4 |
| R2-1 | tasks.json の read-modify-write がクロスマシン race | atomic write + CAS version check | Phase 3 |
| R2-2 | assignAndLock() の TOCTOU 脆弱性 | OS flock + re-read + re-check + write | Phase 4 |
| R2-3 | softDependencies を DAG level に入れると過度に直列化 | soft は dispatch priority にのみ使用 | Phase 2 |
| R2-4 | dagLevels キャッシュは永続化しない方が安全 | 読み込み時再計算に変更 | Phase 2 |
| R2-5 | JSONL event log + snapshot JSON が安全 | task-events.jsonl + tasks.snapshot.json 構成 | Phase 3 |
| R3-1 | tasks.json を事実の正に昇格させすぎ | execution ledger に下げ、SSOT は GitHub | Phase 5 |
| R3-2 | GitHub API 障害時の degraded mode 未定義 | awaiting_github_sync 状態 + retry queue | Phase 5 |
| R3-3 | Issue closed → done は危険 | PR merged + issue linked で確定 | Phase 5 |
| R3-4 | mergeCommit の取得経路が未仕様 | PR API の merge_commit_sha から取得 | Phase 5 |
| R3-5 | PRなし正当完了の escape hatch がない | completionMode = manual/github-pr/external-op | Phase 6 |

---

## 2. アーキテクチャ概要

### 2.1 二層 SSOT + Execution Ledger

```
+------------------------------------------+
|          GitHub (Fact SSOT)               |
|  Issue 状態 / PR merged / Review status  |
+-----+------------------------------------+
      |  pull (authoritative facts)
      |  push (proposed state)
      v
+------------------------------------------+
|     tasks.snapshot.json                   |
|     (Execution Ledger / Gate Cache)       |
|     ← materialized view of event log     |
+-----+------------------------------------+
      ^
      | replay / rebuild
      |
+------------------------------------------+
|     task-events.jsonl                     |
|     (Append-only Event Log)              |
|     state_transition / lock_acquired /   |
|     lock_released / dag_changed /        |
|     github_synced / gate_passed /        |
|     gate_rejected / human_approved       |
+------------------------------------------+
```

### 2.2 状態遷移図（Codex R1-4 反映: merged 追加）

```
[*] ──GATE 0──→ draft
                  │
              GATE 1
                  │
                  v
               pending ←──── blocked (dep未解決)
                  │              ^
              GATE 2             │
              (deps done)        │
                  │              │
                  v              │
              analyzing ─────→ blocked (lock競合)
                  │
              GATE 3
              (impact + human approval)
                  │
                  v
            implementing ──── GATE 4 (lock + heartbeat)
                  │
              GATE 5+6
              (branch + PR)
                  │
                  v
              reviewing
                  │
              GATE 7
              (PR merged verified via GitHub API)
                  │
                  v
               merged  ←── NEW STATE
                  │
              GATE 8
              (Issue closed + worklog)
                  │
                  v
                done
```

### 2.3 コンポーネントマップ（全既存資産の統合先）

```
+-----------------------------------------------------------------------+
|  DeterministicExecutionProtocol (NEW: ~200 lines)                     |
|  唯一の状態遷移窓口。全 GATE をここに集約。                            |
+--+--+--+--+--+--+--+-------------------------------------------------+
   |  |  |  |  |  |  |
   v  v  v  v  v  v  v
+------+ +------+ +------+ +------+ +------+ +------+ +------+
|State | |DAG   | |Lock  | |Store | |Sync  | |Impact| |Audit |
|Machin| |Engine| |Mgr   | |      | |      | |      | |      |
+------+ +------+ +------+ +------+ +------+ +------+ +------+
  既存     移植     統合     NEW      改造     NEW      既存
  Miyabi   KOTOW    miyabi   events   bidirec  GNI→    skill-
  task-    ARI      -priv    .jsonl   tional   Task    bus
  mgr     crowd    lock.ts  +snap    -sync    Impact  record
          +a-s-b   +a-s-b   shot              adapter -run
          queue    queue
```

---

## 3. Phase 定義（全 8 Phase）

### Phase 0: 前提条件の確定（実装前）

**目的**: 全テスト GREEN、全インデックス fresh、Codex レビュー反映方針確定

**入力**: なし
**出力**: 上記チェックリスト全項目 GREEN

**手順**:
1. `cd ~/dev/ops/openclaw-workspace/Miyabi/packages/task-manager && npm test`
2. `cd ~/dev/products/rust-ai-pipeline && cargo test`
3. `cd ~/dev/tools/agent-skill-bus && npm test`
4. 全テスト GREEN でなければ修正してから Phase 1 に進む

**完了条件**: チェックリスト 1.1 + 1.2 の全項目 GREEN

---

### Phase 1: 型定義の拡張

**目的**: ManagedTask に DAG/Lock/Impact/確定的参照フィールドを追加

**対象ファイル**:
- `Miyabi/packages/task-manager/src/types/task.ts`

**追加する型**:

```typescript
// --- NEW: ファイルロック ---
export interface TaskLock {
  lockedBy: string;           // "{agentName}@{nodeName}"
  lockedAt: string;           // ISO 8601
  leaseDurationSec: number;   // R1-5: 固定 TTL ではなく lease
  lastHeartbeat: string;      // R1-5: heartbeat timestamp
  affectedFiles: string[];
}

// --- NEW: Impact 記録 ---
export interface TaskImpact {
  riskLevel: 'LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL';
  affectedSymbols: number;
  depth1: string[];
  analyzedAt: string;
  analyzedCommit: string;     // R1-3: 分析時の HEAD commit
  inputHash: string;          // R1-3: 対象ファイルセットのハッシュ
}

// --- NEW: GitHub 証跡 ---
export interface GitHubEvidence {
  prNumber: number;
  prHeadRef: string;          // R1-3: PR の head branch
  prState: 'OPEN' | 'MERGED' | 'CLOSED';
  mergeCommitSha: string | null;
  mergedAt: string | null;
  reviewDecision: 'APPROVED' | 'CHANGES_REQUESTED' | 'REVIEW_REQUIRED' | null;
  issueState: 'OPEN' | 'CLOSED';
  issueClosedByPR: boolean;
}

// --- NEW: 完了モード (R3-5) ---
export type CompletionMode = 'github-pr' | 'manual' | 'external-op';

// --- TaskState 拡張 (R1-4: merged 追加) ---
export type TaskState =
  | 'draft' | 'pending' | 'analyzing' | 'implementing'
  | 'reviewing' | 'merged'  // ← NEW
  | 'deploying' | 'done'
  | 'blocked' | 'failed' | 'cancelled'
  | 'awaiting_github_sync';  // ← NEW (R3-2)
```

**ManagedTask への追加フィールド**:

```typescript
export interface ManagedTask extends BaseTask {
  // ...既存フィールド...

  // DAG (R2-3: soft は別扱い)
  dependents: string[];
  softDependencies: string[];

  // Lock
  lock: TaskLock | null;

  // Impact
  impact: TaskImpact | null;

  // 確定的参照
  branchName: string | null;
  githubEvidence: GitHubEvidence | null;  // R1-3: 単なる prNumber/mergeCommit ではなく検証済み証跡

  // 完了モード (R3-5)
  completionMode: CompletionMode;

  // 人間承認 (R1-3)
  humanApproval: {
    required: boolean;
    approvedBy: string | null;
    approvedAt: string | null;
    reason: string | null;
  } | null;
}
```

**createManagedTask() の更新**: 新フィールドのデフォルト値追加

**テスト**: 既存テストが壊れないことを確認 + 新フィールドの存在テスト

**Rust 連携**: `rust-ai-pipeline/src/pipeline.rs` の `PipelineReport` 構造体を参考に、`StepResult` パターン（name/success/duration/stdout/stderr）を GATE の通過記録に流用

**完了条件**: `npm test` GREEN、新型が compile 通る

---

### Phase 2: ステートマシン拡張 + DAG 統合

**目的**: 
- `merged` 状態と `awaiting_github_sync` 状態を追加
- conditions を実評価するように変更
- DAG エンジンを移植・統合

**対象ファイル**:
- `Miyabi/packages/task-manager/src/state/task-state-machine.ts`（改造）
- `Miyabi/packages/task-manager/src/dag/task-dag.ts`（NEW: 移植元 KOTOWARI）

#### 2.1 ステートマシン改造

**R1-1 対応**: `conditions` を文字列配列から **predicate 関数** に変更

```typescript
export interface StateTransitionRule {
  from: TaskState;
  to: TaskState;
  predicates?: ((task: ManagedTask, context: TransitionContext) => boolean)[];
  sideEffects?: string[];
}

export interface TransitionContext {
  store: TaskStore;       // 他タスクの状態を参照するため
  triggeredBy: 'user' | 'agent' | 'system' | 'github-webhook';
  reason?: string;
  humanApproval?: boolean;
}
```

**追加する遷移ルール**:

```typescript
// merged (NEW)
{ from: 'reviewing', to: 'merged', predicates: [
  (task) => task.githubEvidence !== null,
  (task) => task.githubEvidence?.prState === 'MERGED',
  (task) => task.githubEvidence?.mergeCommitSha !== null,
  (task) => /^[0-9a-f]{40}$/.test(task.githubEvidence?.mergeCommitSha ?? ''),
]},
{ from: 'merged', to: 'done', predicates: [
  (task) => task.githubEvidence?.issueState === 'CLOSED',
]},

// awaiting_github_sync (NEW, R3-2)
{ from: 'reviewing', to: 'awaiting_github_sync', predicates: [] },
{ from: 'merged', to: 'awaiting_github_sync', predicates: [] },
{ from: 'awaiting_github_sync', to: 'merged', predicates: [
  (task) => task.githubEvidence !== null,
]},
{ from: 'awaiting_github_sync', to: 'reviewing', predicates: [] },

// skip_analysis を削除 (R1-7 対応)
// { from: 'pending', to: 'implementing' } ← 削除
```

**R1-2 対応**: `applyTransition()` を store-backed に変更

```typescript
applyTransition(
  task: ManagedTask,
  to: TaskState,
  context: TransitionContext
): TransitionResult & { task?: ManagedTask } {
  // 1. predicates を全て評価
  const rule = this.getTransitionRule(task.currentState, to);
  if (!rule) return { valid: false, ... };
  
  for (const pred of rule.predicates ?? []) {
    if (!pred(task, context)) {
      return { valid: false, error: `Predicate failed for ${task.currentState} → ${to}` };
    }
  }
  
  // 2. 新 task を生成
  const updatedTask = { ...task, currentState: to, stateHistory: [...] };
  
  // 3. store に即座に永続化（R1-2）
  context.store.updateTask(task.id, updatedTask);
  
  return { valid: true, task: updatedTask };
}
```

#### 2.2 DAG エンジン移植

**移植元**: `KOTOWARI/skills/openclaw-crowd/src/scheduling/task-dependency-graph.ts`

**新ファイル**: `Miyabi/packages/task-manager/src/dag/task-dag.ts`

**変更点**:
- `ScheduledTask` → `ManagedTask` 型変換
- `hard`/`soft` を `dependencies`/`softDependencies` にマップ
- R2-3: soft deps は level 計算に含めない（dispatch priority のみ）
- R2-4: `dagLevels` は永続化せず、`computeLevels()` で毎回再計算
- `getReadyTasks(store)`: `dependencies` が全て `done` or `merged` のタスクを返す
- `getDispatchScore(task)`: priority × age × softSatisfied × lockAvailability の合成スコア

**テスト**: KOTOWARI の既存テストを移植 + ManagedTask 型でのテスト追加

**完了条件**: DAG テスト GREEN、循環検出テスト GREEN、soft deps が level に影響しないことのテスト GREEN

---

### Phase 3: Event Store（JSONL + Snapshot）

**目的**: R2-1/R2-5 対応。tasks.json を monolithic truth から event log + snapshot に分離

**新ファイル**:
- `Miyabi/packages/task-manager/src/store/event-store.ts`
- `Miyabi/packages/task-manager/src/store/snapshot-store.ts`

#### 3.1 Event Store

```typescript
export interface TaskEvent {
  id: string;             // ascending ID
  ts: string;             // ISO 8601
  type: 'state_transition' | 'lock_acquired' | 'lock_released' | 'lock_heartbeat'
      | 'dag_changed' | 'github_synced' | 'gate_passed' | 'gate_rejected'
      | 'human_approved' | 'impact_recorded' | 'branch_created' | 'pr_created'
      | 'merge_verified' | 'audit_recorded';
  taskId: string;
  agent: string;
  node: string;
  payload: Record<string, unknown>;
  version: number;        // R2-1: CAS 用バージョン
}

export class EventStore {
  private filePath: string;  // project_memory/task-events.jsonl

  append(event: TaskEvent): void;           // append-only
  replay(since?: string): TaskEvent[];      // 全イベント再生
  replayForTask(taskId: string): TaskEvent[];
}
```

#### 3.2 Snapshot Store

```typescript
export interface TasksSnapshot {
  version: number;        // R2-1: CAS バージョン
  generatedAt: string;
  generatedFromEventId: string;  // どこまで replay したか
  tasks: ManagedTask[];
  fileLocks: Record<string, { taskId: string; agent: string; node: string; expiresAt: string }>;
}

export class SnapshotStore {
  private filePath: string;  // project_memory/tasks.snapshot.json
  private lockFilePath: string;  // project_memory/.tasks.lock

  // R2-1: atomic write with OS flock + CAS
  load(): TasksSnapshot;
  save(snapshot: TasksSnapshot, expectedVersion: number): void;  // version mismatch → throw
  rebuild(eventStore: EventStore): TasksSnapshot;  // event log から再構築
}
```

**R2-1 対応: atomic write の実装**:

```typescript
save(snapshot: TasksSnapshot, expectedVersion: number): void {
  // 1. OS flock を取得（fd-level）
  const lockFd = fs.openSync(this.lockFilePath, 'w');
  flock(lockFd, LOCK_EX);  // blocking
  
  try {
    // 2. 現在の version を再読込（CAS）
    const current = this.load();
    if (current.version !== expectedVersion) {
      throw new Error(`CAS conflict: expected v${expectedVersion}, got v${current.version}`);
    }
    
    // 3. atomic rename で書き込み
    const tmpPath = this.filePath + '.tmp';
    fs.writeFileSync(tmpPath, JSON.stringify({ ...snapshot, version: expectedVersion + 1 }, null, 2));
    fs.renameSync(tmpPath, this.filePath);
  } finally {
    flock(lockFd, LOCK_UN);
    fs.closeSync(lockFd);
  }
}
```

**Rust 連携**: `rust-ai-pipeline` の `PipelineReport` の JSON シリアライズパターン（`serde::Serialize`）を参考に、event の JSON 形式を統一。将来的に Rust 側で event log を消費する際に互換性を持たせる。

**完了条件**: event append テスト、snapshot rebuild テスト、CAS conflict テスト、concurrent write テスト（fork して同時書き込み）

---

### Phase 4: File Lock Manager（lease + heartbeat）

**目的**: R1-5/R2-2 対応。固定 TTL → renewable lease に変更

**新ファイル**: `Miyabi/packages/task-manager/src/lock/file-lock-manager.ts`

**移植元**:
- `miyabi-private/packages/miyabi/src/util/lock.ts`（RWLock パターン）
- `agent-skill-bus/src/queue.js`（TTL + 競合チェック）

```typescript
export interface LeaseConfig {
  leaseDurationSec: number;   // default: 300
  heartbeatIntervalSec: number; // default: 60
  staleAfterMissedHeartbeats: number; // default: 2
}

export class FileLockManager {
  constructor(
    private eventStore: EventStore,
    private snapshotStore: SnapshotStore,
    private config: LeaseConfig
  ) {}

  // R2-2: 原子的な lock 獲得
  acquireLock(taskId: string, agent: string, node: string, files: string[]): void {
    // 1. OS flock で snapshot をロック
    // 2. snapshot を再読込
    // 3. 競合チェック（files が他タスクにロック中か）
    // 4. stale lock の自動解放（heartbeat 2回 miss）
    // 5. lock 書き込み + event 記録
    // 6. OS flock 解放
  }

  // heartbeat（lease 更新）
  renewLease(taskId: string, agent: string, node: string): void {
    // event: lock_heartbeat を append
    // snapshot の lastHeartbeat を更新
  }

  // 解放
  releaseLock(taskId: string): void {
    // event: lock_released を append
    // snapshot から fileLock エントリを削除
  }

  // stale 検出
  releaseExpiredLeases(): { released: string[]; active: string[] } {
    // lastHeartbeat + leaseDuration + staleAfterMissed × heartbeatInterval < now → stale
  }

  // 競合チェック
  hasConflict(files: string[]): { conflicting: boolean; heldBy?: string; taskId?: string } {
    // snapshot の fileLocks をチェック
  }
}
```

**Rust 連携**: `rust-ai-pipeline/src/signal.rs` の `Signal.is_fresh(max_age_ms)` パターンを heartbeat の鮮度判定に流用。同じ「timestamp + threshold → fresh/stale」の設計思想。

**テスト**:
- lease 獲得 → heartbeat → 解放 のライフサイクル
- 競合検出（同一ファイルに2タスク）
- stale 検出（heartbeat なしで lease 期限切れ）
- concurrent acquire（2プロセスが同時に acquireLock）→ 1つだけ成功

**完了条件**: 全テスト GREEN

---

### Phase 5: GitHub 同期の再設計

**目的**: R3-1/R3-2/R3-3/R3-4 対応。二層 SSOT を正しく実装

**対象ファイル**:
- `Miyabi/packages/task-manager/src/sync/bidirectional-sync.ts`（大幅改造）
- `Miyabi/packages/task-manager/src/sync/github-evidence-fetcher.ts`（NEW）

#### 5.1 GitHub Evidence Fetcher

```typescript
export class GitHubEvidenceFetcher {
  constructor(private octokit: Octokit, private owner: string, private repo: string) {}

  // R3-4: PR API から merge commit を取得
  async fetchPREvidence(prNumber: number): Promise<GitHubEvidence> {
    const { data: pr } = await this.octokit.pulls.get({
      owner: this.owner, repo: this.repo, pull_number: prNumber
    });
    return {
      prNumber,
      prHeadRef: pr.head.ref,
      prState: pr.merged ? 'MERGED' : pr.state === 'closed' ? 'CLOSED' : 'OPEN',
      mergeCommitSha: pr.merge_commit_sha,
      mergedAt: pr.merged_at,
      reviewDecision: await this.fetchReviewDecision(prNumber),
      issueState: await this.fetchLinkedIssueState(prNumber),
      issueClosedByPR: await this.checkIssueClosedByPR(prNumber),
    };
  }

  // R3-3: Issue が PR merge で close されたか検証
  private async checkIssueClosedByPR(prNumber: number): Promise<boolean> {
    // GitHub GraphQL: pullRequest.closingIssuesReferences
  }
}
```

#### 5.2 同期の再設計（R3-1: authority-aware bidirectional）

```typescript
export type SyncDirection = 
  | 'push_proposal'           // ローカル → GitHub（提案）
  | 'pull_authoritative';     // GitHub → ローカル（事実の取得）

export class DeterministicSync {
  // R3-1: 何の事実はどちらが authoritative かを状態ごとに定義
  private authority: Record<string, 'github' | 'local'> = {
    'taskCompletion': 'github',   // done/merged は GitHub が authority
    'executionLock': 'local',     // lock/DAG はローカルが authority
    'reviewStatus': 'github',     // review 結果は GitHub が authority
    'assignedAgent': 'local',     // アサインはローカルが authority
  };

  // R3-2: GitHub API 障害時
  async syncWithDegradedMode(tasks: ManagedTask[]): Promise<SyncResult> {
    try {
      return await this.fullSync(tasks);
    } catch (error) {
      if (this.isGitHubAPIError(error)) {
        // awaiting_github_sync に遷移
        // retry queue に追加
        // event: github_sync_failed を記録
        return { degraded: true, retryAt: new Date(Date.now() + 60000) };
      }
      throw error;
    }
  }
}
```

**Rust 連携**: `rust-ai-pipeline/src/remote.rs` の `ssh_exec()` + `RemoteResult` パターンを GitHub API 呼び出しの結果型に流用。success/stdout/stderr の三つ組は `GitHubAPIResult` にそのまま適用可能。

**テスト**:
- PR merged → evidence 取得 → merged 遷移
- Issue close by non-PR → done に遷移しないことを検証
- GitHub API 障害 → awaiting_github_sync 遷移
- retry 後の復旧

**完了条件**: 全テスト GREEN

---

### Phase 6: DeterministicExecutionProtocol（核心）

**目的**: 全 GATE を1つのクラスに集約。唯一の状態遷移窓口。

**新ファイル**: `Miyabi/packages/task-manager/src/protocol/deterministic-execution.ts`

**依存**: Phase 1-5 の全コンポーネント

```typescript
export class DeterministicExecutionProtocol {
  private stateMachine: TaskStateMachine;  // Phase 2
  private dag: TaskDAG;                    // Phase 2
  private lockManager: FileLockManager;    // Phase 4
  private eventStore: EventStore;          // Phase 3
  private snapshotStore: SnapshotStore;    // Phase 3
  private sync: DeterministicSync;         // Phase 5
  private evidenceFetcher: GitHubEvidenceFetcher;  // Phase 5

  // ==========================================
  // GATE 0: タスク登録（Issue 必須）
  // ==========================================
  registerTask(input: CreateTaskInput & {
    githubIssueNumber: number;
    completionMode: CompletionMode;
  }): ManagedTask {
    // GATE: githubIssueNumber > 0
    // GATE: completionMode は明示的に指定
    // EVENT: state_transition (→ draft)
    // STORE: snapshot に追加
  }

  // ==========================================
  // GATE 1: DAG 登録（循環依存拒否）
  // ==========================================
  addToDAG(taskId: string, dependencies: string[], softDependencies: string[]): void {
    // GATE: dag.detectCycle() === null
    // GATE: dependencies は全て既存タスク ID
    // EVENT: dag_changed
  }

  // ==========================================
  // GATE 2: 依存解決チェック
  // ==========================================
  checkDependencies(taskId: string): 'ready' | 'blocked' {
    // GATE: dependencies.every(d => ['done', 'merged'].includes(store.get(d).currentState))
    // blocked → pending は dep 解決後に自動遷移
    // EVENT: state_transition (pending → analyzing) or (pending → blocked)
  }

  // ==========================================
  // GATE 3: Impact 分析記録
  // ==========================================
  recordImpact(taskId: string, impact: TaskImpact): void {
    // GATE: impact.analyzedCommit === current HEAD
    // GATE: impact.inputHash matches current file set
    // GATE: HIGH/CRITICAL → humanApproval.required = true
    // EVENT: impact_recorded
  }

  // ==========================================
  // GATE 3.5: 人間承認（HIGH/CRITICAL のみ）
  // ==========================================
  recordHumanApproval(taskId: string, approvedBy: string, reason: string): void {
    // GATE: task.impact.riskLevel in ['HIGH', 'CRITICAL']
    // GATE: task.humanApproval.required === true
    // EVENT: human_approved
  }

  // ==========================================
  // GATE 4: アサイン + ロック獲得（原子的）
  // ==========================================
  assignAndLock(taskId: string, agent: string, node: string, files: string[]): void {
    // GATE: impact !== null
    // GATE: HIGH/CRITICAL → humanApproval.approvedAt !== null
    // GATE: lockManager.hasConflict(files) === false
    // ATOMIC: OS flock → re-read → re-check → write → release
    // EVENT: lock_acquired
    // EVENT: state_transition (analyzing → implementing)
  }

  // ==========================================
  // Heartbeat（実装中に定期実行）
  // ==========================================
  heartbeat(taskId: string, agent: string, node: string): void {
    // GATE: task.lock.lockedBy === `${agent}@${node}`
    // EVENT: lock_heartbeat
  }

  // ==========================================
  // GATE 5: ブランチ記録
  // ==========================================
  recordBranch(taskId: string, branchName: string): void {
    // GATE: /^(feature|fix|hotfix)\/issue-\d+-/.test(branchName)
    // GATE: git branch --list で実在確認
    // EVENT: branch_created
  }

  // ==========================================
  // GATE 6: PR 記録
  // ==========================================
  async recordPR(taskId: string, prNumber: number): Promise<void> {
    // GATE: prNumber > 0
    // GATE: branchName !== null
    // VERIFY: evidenceFetcher.fetchPREvidence(prNumber)
    // GATE: evidence.prHeadRef === task.branchName
    // GATE: evidence.prState === 'OPEN'
    // EVENT: pr_created
    // EVENT: state_transition (implementing → reviewing)
  }

  // ==========================================
  // GATE 7: Merge 検証
  // ==========================================
  async verifyMerge(taskId: string): Promise<void> {
    // FETCH: evidenceFetcher.fetchPREvidence(task.githubEvidence.prNumber)
    // GATE: evidence.prState === 'MERGED'
    // GATE: evidence.mergeCommitSha matches /^[0-9a-f]{40}$/
    // GATE: evidence.reviewDecision === 'APPROVED'
    // EVENT: merge_verified
    // EVENT: state_transition (reviewing → merged)
    // EFFECT: lockManager.releaseLock(taskId)
    // EFFECT: dependents の blocked → pending 遷移
  }

  // ==========================================
  // GATE 8: 完了確定
  // ==========================================
  async confirmDone(taskId: string): Promise<void> {
    // FETCH: evidence.issueState
    // GATE: evidence.issueClosedByPR === true (github-pr mode)
    //   OR: completionMode === 'manual' + humanApproval
    //   OR: completionMode === 'external-op' + humanApproval
    // EVENT: state_transition (merged → done)
    // EVENT: audit_recorded
    // EFFECT: worklog に記録
    // EFFECT: agent-skill-bus に record-run
  }

  // ==========================================
  // Escape Hatches（R3-5: 監査可能な例外経路）
  // ==========================================
  forceUnlock(taskId: string, reason: string, operator: string): void {
    // EVENT: lock_released (forced=true, reason, operator)
  }

  reconcileFromGitHub(taskId: string): Promise<void> {
    // GitHub の状態を pull して snapshot を上書き
    // EVENT: github_synced (reconcile)
  }

  manualComplete(taskId: string, reason: string, operator: string): void {
    // GATE: completionMode === 'manual' or 'external-op'
    // EVENT: state_transition (→ done) + human_approved
  }
}
```

**Rust 連携**: `rust-ai-pipeline` の Phase 1 パイプライン（build → clippy → test、最初の失敗で打ち切り）と同じパターン。GATE 0 → 1 → ... → 8 を順に実行し、最初の GATE 失敗で打ち切る。`PipelineReport` の `steps[]` と `failure_kind` をそのまま GATE の通過/拒否記録に流用。

将来的には、Rust で Phase 1 パイプライン（build/clippy/test）を GATE 4.5 として組み込み、「コンパイルが通らないコードは implementing → reviewing に遷移できない」という品質ゲートにする。

**テスト**:
- happy path: draft → ... → done の全シーケンス
- GATE 拒否: 各 GATE で条件不足 → 遷移不可
- 競合: 2タスクが同じファイルをロック → 1つだけ成功
- escape hatch: forceUnlock → 理由記録
- GitHub 障害: verifyMerge → awaiting_github_sync → retry → merged

**完了条件**: 全テスト GREEN

---

### Phase 7: CLI コマンド

**目的**: Protocol を CLI から操作可能にする

**新ファイル**: `Miyabi/packages/task-manager/src/cli/deterministic-cli.ts`

```bash
miyabi task register --issue 45 --title "JWT移行" --deps task-000 --mode github-pr
miyabi task dag                          # DAG 可視化（ターミナル）
miyabi task dispatchable                 # 実行可能タスク一覧
miyabi task assign task-001 --agent kotowari-dev --node macbook-pro --files "src/auth/*.ts"
miyabi task heartbeat task-001           # lease 更新
miyabi task status [task-id]             # 状態確認（JSON or human-readable）
miyabi task locks                        # ロック一覧
miyabi task unlock task-001 --reason "agent crashed" --operator hayashi
miyabi task verify-merge task-001        # GitHub から merge 証跡を取得
miyabi task confirm-done task-001        # 完了確定
miyabi task sync                         # GitHub 同期
miyabi task events [task-id]             # event log 表示
miyabi task rebuild-snapshot             # snapshot を event log から再構築
miyabi task reconcile task-001           # GitHub から強制同期
```

**Rust 連携**: `rust-ai-pipeline` の CLI パターン（`parse_cli_args()` → `CliMode` enum → `run_*()` 関数）を参考に、サブコマンドを enum で定義。`ensure_cargo_project()` のような前提条件チェックを各コマンドの冒頭に配置。

**完了条件**: 全サブコマンドのヘルプ表示 + register → assign → verify-merge → confirm-done の E2E テスト

---

### Phase 8: 統合テスト + Agent Skill Bus 記録

**目的**: 全 Phase を結合して1タスクの draft → done を通す

**テストシナリオ**:

```
1. gh issue create → Issue #N
2. miyabi task register --issue N --title "test" --mode github-pr
3. miyabi task dag → Level 0 に task 表示
4. miyabi task dispatchable → task が ready
5. GNI impact 実行 → miyabi task record-impact
6. miyabi task assign --agent test --node local --files "src/test.ts"
7. git checkout -b feature/issue-N-test
8. miyabi task record-branch feature/issue-N-test
9. echo "// test" >> src/test.ts && git commit
10. gh pr create → PR #M
11. miyabi task record-pr M
12. gh pr merge M
13. miyabi task verify-merge
14. miyabi task confirm-done
15. miyabi task status → done ✅
16. gh issue view N → Closed ✅
17. npx agent-skill-bus record-run → recorded ✅
```

**rust-ai-pipeline 統合テスト（将来）**:

```
Step 6.5: ai-pipeline phase1 --project . --format json
  → all_passed === true なら GATE 4.5 通過
  → failure_kind !== null なら implementing に留まる
```

**完了条件**: E2E テスト GREEN、event log に全ステップの記録あり、snapshot が正しく再構築可能

---

## 4. 実装順序と依存関係 DAG

```
Phase 0 ──→ Phase 1 ──→ Phase 2 ──→ Phase 3 ──→ Phase 4
                                         │           │
                                         └─────┬─────┘
                                               │
                                               v
                                           Phase 5 ──→ Phase 6 ──→ Phase 7 ──→ Phase 8
```

| Phase | 依存 | 新規コード量 | リスク |
|-------|------|-------------|--------|
| 0 | なし | 0 | LOW |
| 1 | 0 | ~80 行 | LOW |
| 2 | 1 | ~250 行 | MEDIUM（ステートマシン改造） |
| 3 | 1 | ~200 行 | MEDIUM（atomic write） |
| 4 | 3 | ~150 行 | MEDIUM（lease + heartbeat） |
| 5 | 3 | ~200 行 | HIGH（GitHub API 依存） |
| 6 | 2,3,4,5 | ~300 行 | HIGH（核心の統合） |
| 7 | 6 | ~150 行 | LOW（CLI ボイラープレート） |
| 8 | 7 | ~100 行 | LOW（E2E テスト） |
| **合計** | | **~1,430 行** | |

---

## 5. エージェント割り当て（Maestro Playbook 実行時）

| Phase | 推奨エージェント | 理由 |
|-------|---------------|------|
| 0 | Claude Code（ローカル） | テスト実行 + インデックス確認 |
| 1 | Codex（worktree） | 型定義のみ、副作用なし |
| 2 | Claude Code（ローカル） | ステートマシン改造は対話が必要 |
| 3 | Codex（worktree） | event store は独立実装可能 |
| 4 | Codex（worktree） | lock manager も独立実装可能 |
| 5 | Claude Code（ローカル） | GitHub API の挙動確認が必要 |
| 6 | Claude Code（ローカル） | 全コンポーネント統合は対話が必要 |
| 7 | Codex（worktree） | CLI ボイラープレート |
| 8 | Claude Code（ローカル） | E2E テストは実環境が必要 |

**並列実行可能**:
- Phase 3 と Phase 4 は同時実行可能（Codex 2体）
- Phase 2 は Phase 3/4 と並行可能（依存は Phase 1 のみ）

---

## 6. リスク軽減策

| リスク | 軽減策 |
|--------|--------|
| ステートマシン改造で既存テストが壊れる | Phase 2 開始前に既存テストを全部 GREEN にする |
| atomic write の OS flock がクロスプラットフォームで動かない | macOS + Linux でテスト。Windows は SMB 経由のため advisory lock は非保証 → 単一 writer に fallback |
| GitHub API レート制限 | secondary rate limit 対応の exponential backoff + 5000 req/hr の予算管理 |
| event log が肥大化 | 30日以上前の event は archive + snapshot rebuild で圧縮 |
| Rust pipeline 統合が遅れる | Phase 6 の GATE 4.5 は optional。なくても draft → done は通る |

---

## 7. 検証基準（全 Phase 完了後）

| # | 検証項目 | 方法 |
|---|---------|------|
| 1 | 全 GATE が条件不足で拒否する | 各 GATE の境界値ユニットテスト |
| 2 | LLM が GATE をバイパスできない | Protocol 以外の経路で applyTransition を呼ぶテスト → 失敗 |
| 3 | 2エージェントが同一ファイルをロックできない | concurrent acquire テスト |
| 4 | 依存未解決タスクが analyzing に入れない | DAG + ステートマシン結合テスト |
| 5 | merge commit なしに done に遷移できない | GATE 7 のユニットテスト |
| 6 | GitHub 障害時に安全に停止する | mock API + awaiting_github_sync テスト |
| 7 | event log から snapshot を正確に再構築できる | rebuild + diff テスト |
| 8 | E2E: draft → done の全シーケンス | Phase 8 の統合テスト |

---

_This playbook is the single source of truth for implementation. Do not start coding until Phase 0 is GREEN._
