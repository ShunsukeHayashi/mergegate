# Deterministic Task Execution Protocol — UML Diagrams

---

## 1. As-Is vs To-Be: タスク完了までのシーケンス比較

### As-Is（現状: LLM の自己申告ベース）

```mermaid
sequenceDiagram
    participant H as Human
    participant LLM as Agent (LLM)
    participant GH as GitHub

    H->>LLM: 「認証機能を直して」
    LLM->>LLM: (考える)
    LLM->>LLM: ファイルを編集
    Note over LLM: impact分析なし
    Note over LLM: ロックなし
    Note over LLM: 別エージェントも同時編集可能
    LLM-->>H: 「完了しました！」
    
    H->>GH: Issue確認
    Note over GH: Issue: Open ❌
    Note over GH: PR: なし ❌
    Note over GH: merge: なし ❌
    
    H-->>LLM: 「完了してないじゃん」
    LLM->>LLM: (やり直し...)
```

### To-Be（実装後: GATE による確定的制御）

```mermaid
sequenceDiagram
    participant H as Human
    participant P as Protocol (tasks.json)
    participant LLM as Agent (LLM)
    participant GNI as GitNexus
    participant GH as GitHub

    H->>GH: Issue #45 作成
    H->>P: registerTask(issue=45)
    P->>P: GATE 0: issue存在? ✅

    P->>P: GATE 2: 依存解決? ✅
    P->>P: draft → pending → analyzing

    LLM->>GNI: gitnexus_impact("AuthController")
    GNI-->>P: {riskLevel: HIGH, symbols: 12}
    P->>P: GATE 3: impact記録済 ✅
    P->>H: ⚠️ HIGH risk — 承認必要
    H-->>P: 承認

    P->>P: GATE 4: ファイルロック競合なし? ✅
    P->>P: lock獲得 + analyzing → implementing

    LLM->>LLM: ブランチ作成 + コード修正
    LLM->>P: recordBranch("feature/issue-45-auth")
    P->>P: GATE 5: ブランチ名検証 ✅

    LLM->>GH: gh pr create → PR #78
    LLM->>P: recordPR(78)
    P->>P: GATE 6: prNumber > 0 ✅
    P->>P: implementing → reviewing

    H->>GH: Review → Approve → Merge
    GH-->>P: mergeCommit = "a1b2c3d4..."
    P->>P: GATE 7: 40文字hex ✅
    P->>P: reviewing → done
    P->>P: ロック解放 + 後続タスク解放
    P->>GH: Issue #45 → Closed ✅
```

---

## 2. ステートマシン: 既存 vs 拡張

### As-Is（既存: 条件チェックなし）

```mermaid
stateDiagram-v2
    [*] --> draft
    draft --> pending
    pending --> analyzing
    pending --> implementing: skip_analysis
    analyzing --> implementing
    implementing --> reviewing
    reviewing --> done
    reviewing --> implementing: needs_revision
    
    note right of pending: 依存チェックなし\n誰でも遷移可能
    note right of analyzing: impact なしでも\nimplementingに行ける
    note right of reviewing: PRなくても\ndoneに行ける
```

### To-Be（拡張: 全遷移に GATE）

```mermaid
stateDiagram-v2
    [*] --> draft: GATE 0\ngithubIssueNumber > 0

    draft --> pending: GATE 1\ntitle + description + deps[]

    pending --> analyzing: GATE 2\ndeps.every(done)
    pending --> blocked: deps未解決

    analyzing --> implementing: GATE 3 + 4\nimpact != null\nlock獲得済\nHIGH→人間承認
    analyzing --> blocked: ロック競合

    implementing --> reviewing: GATE 5 + 6\nbranchName valid\nprNumber > 0

    reviewing --> done: GATE 7\nmergeCommit = /[0-9a-f]{40}/

    done --> [*]: GATE 8\nworklog記録 + Issue Closed

    blocked --> pending: 依存解決 or ロック解放

    note right of draft: Issue がなければ\n登録すら不可能
    note right of analyzing: GNI分析が空なら\n実装に入れない
    note right of implementing: ロックがなければ\nコード編集不可
    note right of reviewing: merge commitなしに\n完了は不可能
```

---

## 3. ファイルロック: 競合防止の仕組み

### As-Is（ロックなし）

```mermaid
sequenceDiagram
    participant A as Agent A (MacBook)
    participant F as auth.controller.ts
    participant B as Agent B (Windows)

    A->>F: 編集開始
    B->>F: 編集開始（同時）
    Note over F: 💥 競合！
    A->>F: コミット
    B->>F: コミット（上書き）
    Note over F: Agent A の変更が消失
```

### To-Be（ファイルロック付き）

```mermaid
sequenceDiagram
    participant A as Agent A (MacBook)
    participant T as tasks.json
    participant B as Agent B (Windows)
    participant F as auth.controller.ts

    A->>T: assignAndLock(task-001, files=[auth.controller.ts])
    T->>T: fileLocks["auth.controller.ts"] = "task-001" ✅
    T-->>A: ロック獲得OK

    B->>T: assignAndLock(task-002, files=[auth.controller.ts])
    T->>T: fileLocks["auth.controller.ts"] 既にロック中 ❌
    T-->>B: Error: "task-001がロック中"

    A->>F: 編集 → commit → PR → merge
    A->>T: recordMerge(task-001, "a1b2c3...")
    T->>T: fileLocks["auth.controller.ts"] 解放

    B->>T: assignAndLock(task-002, files=[auth.controller.ts])
    T->>T: fileLocks 空 ✅
    T-->>B: ロック獲得OK
    B->>F: 編集開始（安全）
```

---

## 4. DAG 依存関係: 実行順序の強制

```mermaid
graph TD
    subgraph "DAG Level 0（最初に実行）"
        T0[task-000<br/>DBスキーマ変更<br/>state: done ✅]
    end

    subgraph "DAG Level 1（Level 0 完了後）"
        T1[task-001<br/>API実装<br/>state: implementing 🔒]
    end

    subgraph "DAG Level 2（Level 1 完了後）"
        T2[task-002<br/>フロントエンド<br/>state: blocked ⏳]
        T3[task-003<br/>テスト作成<br/>state: blocked ⏳]
    end

    T0 -->|"hard dep"| T1
    T1 -->|"hard dep"| T2
    T1 -->|"hard dep"| T3

    style T0 fill:#22c55e,color:#fff
    style T1 fill:#3b82f6,color:#fff
    style T2 fill:#f59e0b,color:#fff
    style T3 fill:#f59e0b,color:#fff
```

```mermaid
sequenceDiagram
    participant DAG as DAG Engine
    participant SM as State Machine
    participant T0 as task-000 (DB)
    participant T1 as task-001 (API)
    participant T2 as task-002 (Frontend)

    Note over DAG: Level 0 実行
    T0->>SM: pending → implementing → done ✅
    
    DAG->>DAG: task-000 done → Level 1 解放
    
    Note over DAG: Level 1 実行
    T1->>SM: blocked → pending（依存解決）
    T1->>SM: pending → analyzing → implementing

    Note over T2: task-001 がまだ done じゃない
    T2->>SM: pending → analyzing を試みる
    SM-->>T2: ❌ GATE 2 拒否: task-001 が done ではない

    T1->>SM: implementing → reviewing → done ✅
    DAG->>DAG: task-001 done → Level 2 解放

    Note over DAG: Level 2 実行（並列可能）
    T2->>SM: blocked → pending ✅
```

---

## 5. 全体アーキテクチャ: 3ピース統合

### As-Is（分散・未接続）

```mermaid
graph LR
    subgraph "Miyabi task-manager"
        SM[State Machine<br/>27 rules]
        SYNC[GitHub Sync<br/>bidirectional]
        EXEC[Task Executor<br/>worktree]
    end

    subgraph "KOTOWARI openclaw-crowd"
        DAG[DAG Engine<br/>topological sort]
        SCHED[Scheduler<br/>priority queue]
    end

    subgraph "agent-skill-bus"
        QUEUE[JSONL Queue<br/>prompt requests]
        LOCK[File Locks<br/>TTL-based]
    end

    SM -.->|"未接続"| DAG
    SM -.->|"未接続"| LOCK
    DAG -.->|"未接続"| LOCK
    
    style SM fill:#ef4444,color:#fff
    style DAG fill:#ef4444,color:#fff
    style LOCK fill:#ef4444,color:#fff
```

### To-Be（統合: DeterministicExecutionProtocol）

```mermaid
graph TB
    subgraph "DeterministicExecutionProtocol"
        direction TB
        
        subgraph "GATE Layer（門番）"
            G0[GATE 0: Issue存在]
            G2[GATE 2: 依存解決]
            G3[GATE 3: Impact記録]
            G4[GATE 4: ロック獲得]
            G6[GATE 6: PR作成]
            G7[GATE 7: Merge確定]
        end
        
        subgraph "Engine Layer（実行）"
            SM[State Machine<br/>27 rules + GATE条件]
            DAG[DAG Engine<br/>Kahn sort + 依存解決]
            LOCK[File Lock Manager<br/>TTL + 競合検出]
        end
        
        subgraph "Store Layer（永続化）"
            TJ[tasks.json<br/>確定的スキーマ]
            GH[GitHub Issues/PR<br/>Fact SSOT]
        end
    end

    G0 --> SM
    G2 --> DAG
    G3 --> SM
    G4 --> LOCK
    G6 --> SM
    G7 --> SM

    SM --> TJ
    DAG --> TJ
    LOCK --> TJ
    TJ <-->|"bidirectional sync"| GH

    style G0 fill:#f59e0b,color:#000
    style G2 fill:#f59e0b,color:#000
    style G3 fill:#f59e0b,color:#000
    style G4 fill:#f59e0b,color:#000
    style G6 fill:#f59e0b,color:#000
    style G7 fill:#f59e0b,color:#000
    style SM fill:#22c55e,color:#fff
    style DAG fill:#22c55e,color:#fff
    style LOCK fill:#22c55e,color:#fff
    style TJ fill:#3b82f6,color:#fff
    style GH fill:#3b82f6,color:#fff
```

---

## 6. マルチマシン協調: Before / After

### As-Is

```mermaid
graph LR
    subgraph MacBook
        A1[Agent A<br/>auth.ts 編集中]
        M1[MEMORY.md<br/>v3]
    end
    
    subgraph Windows
        A2[Agent B<br/>auth.ts 編集中]
        M2[MEMORY.md<br/>v2 ← 古い!]
    end
    
    subgraph mainmini
        A3[Agent C<br/>何をしてるか不明]
        M3[MEMORY.md<br/>v1 ← もっと古い!]
    end

    A1 -.->|"💥 競合"| A2
    M1 -.->|"❌ ズレ"| M2
    M2 -.->|"❌ ズレ"| M3
```

### To-Be

```mermaid
graph TB
    subgraph "tasks.json（共有 SSOT）"
        TJ["task-001: implementing<br/>lock: Agent A @ MacBook<br/>files: [auth.ts] 🔒<br/><br/>task-002: blocked ⏳<br/>depends: task-001<br/><br/>task-003: pending<br/>files: [user.ts] — 別ファイルなので並行OK"]
    end
    
    subgraph MacBook
        A1[Agent A<br/>auth.ts 編集 ✅<br/>ロック保持]
    end
    
    subgraph Windows
        A2[Agent B<br/>auth.ts 触れない 🚫<br/>task-002 blocked]
    end
    
    subgraph mainmini
        A3[Agent C<br/>user.ts 編集 ✅<br/>task-003 別ロック]
    end

    A1 -->|"read/write"| TJ
    A2 -->|"read only"| TJ
    A3 -->|"read/write"| TJ
    TJ <-->|"sync"| GH[GitHub Issues<br/>Fact SSOT]
```

---

## 7. GATE チェーン: 1タスクの完全ライフサイクル

```mermaid
graph LR
    START((Start)) --> G0{GATE 0<br/>Issue?}
    G0 -->|No| REJECT0[❌ 登録拒否]
    G0 -->|Yes| DRAFT[draft]
    
    DRAFT --> G1{GATE 1<br/>title+desc?}
    G1 -->|No| REJECT1[❌]
    G1 -->|Yes| PENDING[pending]
    
    PENDING --> G2{GATE 2<br/>deps done?}
    G2 -->|No| BLOCKED[blocked ⏳]
    G2 -->|Yes| ANALYZING[analyzing]
    
    ANALYZING --> G3{GATE 3<br/>impact?}
    G3 -->|No| REJECT3[❌ GNI回せ]
    G3 -->|HIGH| HUMAN{人間承認?}
    G3 -->|LOW/MED| G4
    HUMAN -->|No| REJECT_H[❌]
    HUMAN -->|Yes| G4
    
    G4{GATE 4<br/>lock?} -->|競合| REJECT4[❌ 待て]
    G4 -->|OK| IMPL[implementing 🔒]
    
    IMPL --> G5{GATE 5<br/>branch?}
    G5 -->|No| REJECT5[❌]
    G5 -->|Yes| G6{GATE 6<br/>PR?}
    G6 -->|No| REJECT6[❌]
    G6 -->|Yes| REVIEW[reviewing]
    
    REVIEW --> G7{GATE 7<br/>merge?}
    G7 -->|No| REJECT7[❌]
    G7 -->|Yes| DONE[done ✅]
    
    DONE --> G8{GATE 8<br/>logged?}
    G8 -->|Yes| FIN((End))
    
    BLOCKED -->|"dep解決"| PENDING

    style REJECT0 fill:#ef4444,color:#fff
    style REJECT1 fill:#ef4444,color:#fff
    style REJECT3 fill:#ef4444,color:#fff
    style REJECT_H fill:#ef4444,color:#fff
    style REJECT4 fill:#ef4444,color:#fff
    style REJECT5 fill:#ef4444,color:#fff
    style REJECT6 fill:#ef4444,color:#fff
    style REJECT7 fill:#ef4444,color:#fff
    style DONE fill:#22c55e,color:#fff
    style BLOCKED fill:#f59e0b,color:#000
    style IMPL fill:#3b82f6,color:#fff
```

---

_Generated: 2026-04-10 | Deterministic Task Execution Protocol UML_
