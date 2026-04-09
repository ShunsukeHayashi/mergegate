# Deterministic Task Execution Protocol — Implementation Plan

## Context

### 問題
LLM エージェントは確率的にトークンを生成するため、「完了しました」と言いながら Issue を Close しない、impact 分析を飛ばす、別エージェントと同じファイルを同時編集する、といった **揺らぎ** が発生する。現在、この揺らぎを防ぐ仕組みが **3箇所に分散して不完全に存在** している。

### ゴール
`project_memory/tasks.json` を **確定的 DAG + ステートマシン + ファイルロック** の三位一体として実装し、LLM の発言ではなく **JSON の状態遷移だけが事実** となる実行プロトコルを構築する。ゴールに対して確定的な手順を踏めば、確実に成果物1点にたどり着く。

### North Star（一文）
> LLM の揺らぎを許さない。tasks.json の状態遷移が許可し、GitHub が確定し、git merge が不可逆にする。この三段階だけが「完了」を定義する。

---

## 既存ピースの統合マップ

```
統合先: @miyabi/task-manager (Miyabi/packages/task-manager/)

① ステートマシン（既存・そのまま使う）
   src/state/task-state-machine.ts
   - 10状態、27遷移ルール
   - applyTransition() で状態遷移を検証・適用
   - conditions/sideEffects 付き

② DAG（移植元: KOTOWARI/skills/openclaw-crowd/src/scheduling/）
   task-dependency-graph.ts
   - Kahn のトポロジカルソート
   - 循環検出（DFS）
   - hard/soft 依存の区別
   - getReadyTasks() で実行可能タスクを返す

③ ファイルロック（移植元: agent-skill-bus/src/queue.js）
   PromptRequestQueue
   - JSONL ベースのロック管理
   - TTL 付きロック（デフォルト 7200秒）
   - startExecution() でロック獲得 + 競合チェック
   - releaseExpiredLocks() で TTL 切れを自動解放
   - getDispatchable() で DAG 依存 + ロック競合を同時チェック
```

---

## 実装計画

### Phase 1: ManagedTask 型の拡張

**ファイル**: `Miyabi/packages/task-manager/src/types/task.ts`

追加するフィールド:

```typescript
export interface TaskLock {
  lockedBy: string;        // "{agentName}@{nodeName}"
  lockedAt: string;        // ISO 8601
  ttl: number;             // 秒
  affectedFiles: string[]; // ロック対象ファイルパス
}

export interface TaskImpact {
  riskLevel: 'LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL';
  affectedSymbols: number;
  depth1: string[];        // 直接影響シンボル名
  analyzedAt: string;      // GNI 分析タイムスタンプ
}

// ManagedTask に追加
export interface ManagedTask extends BaseTask {
  // ...既存フィールド...

  // NEW: DAG
  dependents: string[];           // このタスクが done になったら解放されるタスク
  softDependencies: string[];     // あれば待つが、なくても進められる

  // NEW: ロック
  lock: TaskLock | null;

  // NEW: Impact（C&I Phase A の結果）
  impact: TaskImpact | null;

  // NEW: 確定的参照
  branchName: string | null;      // feature/issue-{N}-{slug}
  prNumber: number | null;
  mergeCommit: string | null;     // merge 後のハッシュ
}
```

### Phase 2: TaskDependencyGraph の移植

**新ファイル**: `Miyabi/packages/task-manager/src/dag/task-dag.ts`

KOTOWARI の `task-dependency-graph.ts` から以下を移植:
- `addTask()`, `removeTask()`, `addDependency()`
- `topologicalSort()` (Kahn のアルゴリズム)
- `detectCycle()` (DFS)
- `getReadyTasks()` — dependencies が全て done のタスクを返す
- `getParallelGroups()` — 同一レベルのタスク群を返す（DAG levels）

**変更点**:
- `ScheduledTask` → `ManagedTask` に型を合わせる
- `hard`/`soft` 依存を `dependencies`/`softDependencies` にマップ
- シリアライズ/デシリアライズを JSON 対応に

### Phase 3: FileLockManager の移植

**新ファイル**: `Miyabi/packages/task-manager/src/lock/file-lock-manager.ts`

agent-skill-bus の `queue.js` から以下を移植:
- ロック獲得: `acquireLock(taskId, agent, node, files, ttl)`
- ロック解放: `releaseLock(taskId)`
- TTL 切れ自動解放: `releaseExpiredLocks()`
- 競合チェック: `hasConflict(files): { conflicting: boolean, heldBy: string, taskId: string }`

**変更点**:
- JSONL → tasks.json 内の `fileLocks` フィールドに統合
- `prId` → `taskId` にリネーム
- TypeScript 化

### Phase 4: DeterministicExecutionProtocol — 確定的シーケンスの実装

**新ファイル**: `Miyabi/packages/task-manager/src/protocol/deterministic-execution.ts`

これが核心。全ステップを **ステートマシンのゲート** として実装:

```typescript
export class DeterministicExecutionProtocol {
  private stateMachine: TaskStateMachine;
  private dag: TaskDAG;
  private lockManager: FileLockManager;
  private store: TaskStore;  // tasks.json の読み書き

  /**
   * Step 0: タスク登録 — Issue が存在しなければ拒否
   * GATE: githubIssueNumber が null → 登録拒否
   */
  registerTask(input: CreateTaskInput & { githubIssueNumber: number }): ManagedTask

  /**
   * Step 1: DAG に登録 — 循環依存があれば拒否
   * GATE: detectCycle() が non-null → 登録拒否
   */
  addToDAG(taskId: string, dependencies: string[]): void

  /**
   * Step 2: 依存解決チェック — 全依存が done でなければ blocked
   * GATE: dependencies.every(d => store.get(d).currentState === 'done')
   */
  checkDependencies(taskId: string): 'ready' | 'blocked'

  /**
   * Step 3: Impact 分析記録 — impact が null なら implementing に遷移不可
   * GATE: task.impact !== null && task.impact.riskLevel !== undefined
   * GATE: HIGH/CRITICAL → humanApprovalRequired = true
   */
  recordImpact(taskId: string, impact: TaskImpact): void

  /**
   * Step 4: アサイン + ロック獲得 — 競合があれば拒否
   * GATE: lockManager.hasConflict(files) === false
   * EFFECT: lock フィールドに書き込み、fileLocks テーブルに登録
   * EFFECT: 状態遷移 analyzing → implementing
   */
  assignAndLock(taskId: string, agent: string, node: string, files: string[]): void

  /**
   * Step 5: ブランチ作成 — branchName が null なら PR 作成不可
   * GATE: branchName matches /^(feature|fix|hotfix)\/issue-\d+-/
   */
  recordBranch(taskId: string, branchName: string): void

  /**
   * Step 6: PR 作成 — prNumber が null なら reviewing に遷移不可
   * GATE: prNumber > 0 && branchName !== null
   * EFFECT: 状態遷移 implementing → reviewing
   */
  recordPR(taskId: string, prNumber: number): void

  /**
   * Step 7: Merge — mergeCommit が null なら done に遷移不可
   * GATE: mergeCommit matches /^[0-9a-f]{40}$/
   * EFFECT: 状態遷移 reviewing → done
   * EFFECT: ロック解放
   * EFFECT: dependents の blocked → pending 遷移
   * EFFECT: GitHub Issue Close 確認
   */
  recordMerge(taskId: string, mergeCommit: string): void

  /**
   * Step 8: 監査記録
   * EFFECT: worklog.md に追記
   * EFFECT: agent-skill-bus に record-run
   */
  recordAudit(taskId: string): void
}
```

**重要**: 各メソッドは **GATE 条件を満たさなければ例外を投げる**。LLM が「大丈夫です」と言っても、GATE が閉じていれば実行不可能。

### Phase 5: TaskStore — tasks.json の永続化

**新ファイル**: `Miyabi/packages/task-manager/src/store/task-store.ts`

```typescript
export class TaskStore {
  private filePath: string;  // project_memory/tasks.json

  load(): TasksDocument
  save(doc: TasksDocument): void
  getTask(id: string): ManagedTask | null
  updateTask(id: string, patch: Partial<ManagedTask>): void
  getFileLocks(): Record<string, string>  // file → taskId
  setFileLock(file: string, taskId: string): void
  removeFileLock(file: string): void
}

export interface TasksDocument {
  version: number;
  lastUpdated: string;
  tasks: ManagedTask[];
  fileLocks: Record<string, string>;
  dagLevels: string[][];  // トポロジカルソート結果のキャッシュ
}
```

### Phase 6: GitHub 同期の強化

**既存ファイル**: `src/sync/bidirectional-sync.ts`

変更:
- `syncTasks()` に **merge commit 検証** を追加
  - ローカルで done なのに GitHub で Issue が Open → **矛盾検出、done → blocked に巻き戻し**
  - GitHub で Closed なのにローカルで implementing → **pull で done に更新**
- `conflictStrategy` に `'deterministic'` を追加
  - GitHub の状態を常に優先（LLM のローカル判断を信用しない）

### Phase 7: CLI コマンド

**新ファイル**: `Miyabi/packages/task-manager/src/cli/deterministic-cli.ts`

```bash
# タスク登録（Issue 必須）
miyabi task register --issue 45 --title "JWT→セッション移行" --deps task-000

# DAG 可視化
miyabi task dag

# 依存解決 + ディスパッチ可能タスク一覧
miyabi task dispatchable

# アサイン + ロック
miyabi task assign task-001 --agent kotowari-dev --node macbook-pro --files "src/auth/*.ts"

# 状態確認（JSON出力、人間可読）
miyabi task status [task-id]

# ロック一覧
miyabi task locks

# 強制ロック解放（TTL切れ or 人間判断）
miyabi task unlock task-001

# 同期（GitHub → ローカル → GitHub）
miyabi task sync
```

---

## ファイル構成（最終）

```
Miyabi/packages/task-manager/src/
├── types/
│   ├── task.ts          # ← Phase 1: TaskLock, TaskImpact, 拡張フィールド追加
│   ├── config.ts        # ← Phase 6: 'deterministic' conflict strategy 追加
│   └── index.ts
├── state/
│   └── task-state-machine.ts  # 既存（変更なし）
├── dag/                        # ← Phase 2: NEW
│   └── task-dag.ts
├── lock/                       # ← Phase 3: NEW
│   └── file-lock-manager.ts
├── protocol/                   # ← Phase 4: NEW（核心）
│   └── deterministic-execution.ts
├── store/                      # ← Phase 5: NEW
│   └── task-store.ts
├── sync/
│   ├── bidirectional-sync.ts  # ← Phase 6: merge commit 検証追加
│   └── ...
├── execution/
│   └── ...                    # 既存（変更なし）
├── decomposition/
│   └── ...                    # 既存（変更なし）
├── cli/                       # ← Phase 7: NEW
│   └── deterministic-cli.ts
├── task-manager.ts            # ← DeterministicExecutionProtocol を統合
└── index.ts                   # ← 新モジュールの export 追加
```

---

## 成果物: tasks.json の確定的スキーマ

```jsonc
{
  "version": 1,
  "lastUpdated": "2026-04-10T00:00:00Z",
  "tasks": [
    {
      "id": "task-001",
      "title": "JWT→セッション認証移行",
      "description": "...",
      "type": "refactor",
      "priority": 80,
      "githubIssueNumber": 45,
      "currentState": "implementing",
      "stateHistory": [
        { "from": "draft", "to": "pending", "timestamp": "...", "triggeredBy": "user" },
        { "from": "pending", "to": "analyzing", "timestamp": "...", "triggeredBy": "system" },
        { "from": "analyzing", "to": "implementing", "timestamp": "...", "triggeredBy": "system" }
      ],
      "assignedAgent": "kotowari-dev",
      "dependencies": ["task-000"],
      "softDependencies": [],
      "dependents": ["task-002", "task-003"],
      "lock": {
        "lockedBy": "kotowari-dev@macbook-pro",
        "lockedAt": "2026-04-10T00:00:00Z",
        "ttl": 7200,
        "affectedFiles": ["src/auth/auth.controller.ts", "src/middleware/auth.middleware.ts"]
      },
      "impact": {
        "riskLevel": "HIGH",
        "affectedSymbols": 12,
        "depth1": ["LoginHandler", "RefreshHandler", "LogoutHandler"],
        "analyzedAt": "2026-04-10T00:00:00Z"
      },
      "branchName": "feature/issue-45-jwt-to-session",
      "prNumber": null,
      "mergeCommit": null,
      "syncVersion": 3,
      "retryCount": 0
    }
  ],
  "fileLocks": {
    "src/auth/auth.controller.ts": "task-001",
    "src/middleware/auth.middleware.ts": "task-001"
  },
  "dagLevels": [
    ["task-000"],
    ["task-001"],
    ["task-002", "task-003"]
  ]
}
```

---

## 確定的ゲート一覧（LLM の揺らぎを封じる全チェックポイント）

| Step | 遷移 | GATE（これが false なら遷移不可） |
|------|------|------|
| 0 | → draft | `githubIssueNumber > 0` |
| 1 | draft → pending | `title.length > 0 && description.length > 0 && dependencies は明示的配列` |
| 2 | pending → analyzing | `dependencies.every(d => tasks[d].currentState === 'done')` |
| 3 | analyzing → implementing | `impact !== null && (impact.riskLevel !== 'HIGH' && !== 'CRITICAL' \|\| humanApproved)` |
| 4 | implementing 開始 | `lock !== null && fileLocks に競合なし` |
| 5 | implementing 中 | `branchName matches /^(feature\|fix\|hotfix)\/issue-\d+-/` |
| 6 | implementing → reviewing | `prNumber > 0` |
| 7 | reviewing → done | `mergeCommit matches /^[0-9a-f]{40}$/` |
| 8 | done 後 | `worklog に記録済み && Issue が Closed` |

**全 GATE は JSON フィールドの値チェック。LLM の自然言語判断は一切介在しない。**

---

## 検証方法

1. **ユニットテスト**: 各 GATE の境界値テスト（vitest、既存テスト基盤を使用）
2. **統合テスト**: draft → done の全シーケンスを1タスクで通す
3. **競合テスト**: 2つのタスクが同じファイルをロックしようとして拒否されることを検証
4. **DAG テスト**: 循環依存を検出して拒否、依存解決後に正しい順序でディスパッチ
5. **GitHub 同期テスト**: ローカル done ↔ GitHub Closed の整合性検証
6. **TTL テスト**: ロック TTL 切れ時の自動解放と failed 遷移

```bash
cd Miyabi/packages/task-manager
npm test                    # 全テスト
npm run test -- --grep "deterministic"  # プロトコルテストのみ
```

---

## 先行研究・設計根拠

- **Temporal.io / Durable Execution**: ワークフローの各ステップを永続化し、障害後も再開可能にする。tasks.json はこれのファイルベース簡易版。
- **Kubernetes Controller Pattern**: Desired State（tasks.json）と Actual State（GitHub）の差分を検出して reconcile する。bidirectional-sync がこれに相当。
- **Database MVCC / Pessimistic Locking**: ファイルロックは悲観的ロック。TTL はデッドロック防止。
- **Apache Airflow DAG**: タスク間の依存関係をDAGで表現し、トポロジカルソートで実行順序を決定。task-dag.ts がこれに相当。
- **Harness Engineering（docs/design/harness-engineering.md）**: 「制約をファイルに書き込み、会話ではなくハーネスで強制する」— 本プロトコルの思想的根拠。

---

## 実装順序とリスク

| Phase | 工数目安 | リスク | 依存 |
|-------|---------|--------|------|
| 1. 型拡張 | 小 | LOW | なし |
| 2. DAG 移植 | 中 | LOW | Phase 1 |
| 3. ロック移植 | 中 | MEDIUM（TTL設計） | Phase 1 |
| 4. プロトコル | 大 | HIGH（核心） | Phase 1,2,3 |
| 5. TaskStore | 中 | LOW | Phase 1 |
| 6. GitHub同期強化 | 中 | MEDIUM（API制限） | Phase 4,5 |
| 7. CLI | 小 | LOW | Phase 4,5 |
