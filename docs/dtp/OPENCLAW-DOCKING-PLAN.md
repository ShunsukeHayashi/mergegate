# OpenClaw ドッキングプラン — DTP × OpenClaw 統合設計

---

## 現状のドッキングポイント（5箇所）

```
┌──────────────────────────────────────────────────────────────────┐
│  OpenClaw (TypeScript/Node.js)                                    │
│                                                                    │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────────┐  │
│  │ exec-approvals │  │ hooks          │  │ sessions-spawn     │  │
│  │ スナップショット│  │ イベント発火   │  │ サブエージェント   │  │
│  │ 承認/拒否判定  │  │ 外部通知       │  │ 起動/停止          │  │
│  └───────┬────────┘  └───────┬────────┘  └─────────┬──────────┘  │
│          │                    │                      │             │
│  ┌───────┴────────┐  ┌───────┴────────┐  ┌─────────┴──────────┐  │
│  │ routing        │  │ memory         │  │ gateway            │  │
│  │ セッションキー │  │ sync/search    │  │ WebSocket/HTTP     │  │
│  │ エージェント   │  │ ベクトル検索   │  │ メッセージ配信     │  │
│  │ 振り分け       │  │                │  │                    │  │
│  └────────────────┘  └────────────────┘  └────────────────────┘  │
│                                                                    │
└────────────────────┬─────────────────────────────────────────────┘
                     │
                     │ Shell / HTTP
                     │
┌────────────────────┴─────────────────────────────────────────────┐
│  miyabi-cli-standalone (Rust)                                     │
│                                                                    │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐            │
│  │ gate.rs  │ │ lock.rs  │ │store.rs  │ │protocol  │ ← Phase A  │
│  │ GATE検証 │ │ ファイル │ │tasks.json│ │.rs       │            │
│  │          │ │ ロック   │ │永続化    │ │統合窓口  │            │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘            │
│                                                                    │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ 既存      │
│  │ dag.rs   │ │github.rs │ │approval  │ │openclaw  │            │
│  │ DAG 823行│ │ GH 947行 │ │.rs 231行 │ │.rs 354行 │            │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘            │
└──────────────────────────────────────────────────────────────────┘
```

---

## ドッキングポイント詳細

### 1. exec-approvals ↔ gate.rs

**OpenClaw 側**: `exec-approvals-cli.ts`
- `loadSnapshot()` / `saveSnapshot()` で承認状態を永続化
- `normalizeAllowlistEntry()` で許可リストを管理
- `resolveAgentKey()` でエージェント特定

**DTP 側**: `gate.rs`
- `ensure_impact(task, human_approved)` で HIGH/CRITICAL を承認ゲート
- `ensure_issue()` で Issue 存在を検証

**ドッキング方法**:
- OpenClaw の `exec-approvals` が DTP の GATE 結果を参照
- DTP の `protocol.record_impact()` が承認を要求 → OpenClaw が approval snapshot に記録
- 連携: `miyabi gate impact <task-id> --risk HIGH` → exit code 1（承認待ち）→ OpenClaw が承認 → `miyabi gate impact <task-id> --risk HIGH --approve`

**改善が必要**:
- [ ] gate.rs に `--approve` フラグ対応（承認者 ID + 理由を記録）
- [ ] exit code で承認状態を返す（0=承認済み, 1=承認待ち, 2=拒否）

### 2. hooks ↔ protocol.rs イベント

**OpenClaw 側**: `hooks/`
- `loadHookEntries()` でフックを登録
- `registerInternalHook()` でイベント発火
- イベント: ツール実行前後、セッション開始/終了、メッセージ送受信

**DTP 側**: `protocol.rs` の各メソッド
- register_task → `task_registered` イベント
- assign_and_lock → `lock_acquired` イベント
- record_merge → `task_completed` イベント

**ドッキング方法**:
- DTP の各 protocol メソッドが完了後に hook を発火
- OpenClaw の hooks システムが DTP イベントを購読
- 連携: `miyabi gate register` 実行 → stdout に JSON イベント → OpenClaw hook がキャッチ

**改善が必要**:
- [ ] protocol.rs の各メソッドに `--emit-event` オプション（JSON イベントを stdout に出力）
- [ ] OpenClaw hooks に DTP イベント用のフックテンプレートを追加

### 3. sessions-spawn ↔ orchestration.rs

**OpenClaw 側**: `subagent-spawn.ts`
- `spawnSubagentDirect()` でサブエージェントを起動
- セッションキーで管理

**DTP 側**: `orchestration.rs`
- `Orchestrator` で並列実行
- `ParallelConfig` で同時実行数制御

**ドッキング方法**:
- OpenClaw main が DTP のタスクを受け取る
- `miyabi gate dispatchable` で実行可能タスクを取得
- 各タスクを `spawnSubagentDirect()` でサブエージェントに配布
- サブエージェントが `miyabi gate assign` → 作業 → `miyabi gate pr` → `miyabi gate merge`

**改善が必要**:
- [ ] `miyabi gate dispatchable --format json` でマシン可読出力
- [ ] 各タスクに `assigned_agent` フィールドを OpenClaw agent ID で記録

### 4. memory ↔ store.rs

**OpenClaw 側**: `memory/`
- `syncMemoryFiles()` でファイル同期
- `searchVector()` / `searchKeyword()` でメモリ検索
- `indexFileEntryIfChanged()` でインデックス更新

**DTP 側**: `store.rs`
- `tasks.json` の読み書き
- atomic rename で永続化

**ドッキング方法**:
- `tasks.json` を OpenClaw のメモリ同期対象に含める
- OpenClaw の `syncMemoryFiles()` が `project_memory/tasks.json` を各ノードに配布
- 検索: `searchKeyword("task-001")` で特定タスクの状態を取得

**改善が必要**:
- [ ] `project_memory/` を OpenClaw memory sync の対象パスに追加
- [ ] tasks.json の変更を OpenClaw memory index に自動反映

### 5. routing ↔ dag.rs タスク振り分け

**OpenClaw 側**: `routing/`
- `resolveAgentIdFromSessionKey()` でエージェントを特定
- `normalizeAgentId()` / `sanitizeAgentId()`

**DTP 側**: `dag.rs`
- `TaskGraph` で依存関係管理
- `build_levels()` で並列実行グループ計算

**ドッキング方法**:
- DTP の DAG レベルに基づいて、OpenClaw routing がエージェントを振り分け
- Level 0 のタスク → 即座にディスパッチ
- Level 1 のタスク → Level 0 完了後にディスパッチ
- 連携: `miyabi gate dag --format json` → OpenClaw が読んで routing 判定

**改善が必要**:
- [ ] dag.rs の出力に `assigned_agent` を含める
- [ ] OpenClaw routing が DTP DAG レベルを考慮するアダプタ

---

## ドッキング実行シーケンス

```
OpenClaw main (Gateway)
    │
    ▼
1. miyabi gate register --issue 45 --title "認証移行"
    │ → tasks.json に登録、GATE 0 検証
    │ → exit 0 (成功) or exit 2 (Issue なし)
    │
    ▼
2. miyabi gate dispatchable --format json
    │ → 実行可能タスク一覧を返す
    │
    ▼
3. openclaw sessions spawn --agent kade
    │ → サブエージェント（カエデ）を起動
    │
    ▼
4. miyabi gate assign task-001 --agent kade --node macbook --files "src/auth.rs"
    │ → ファイルロック獲得、implementing に遷移
    │ → exit 0 (成功) or exit 1 (ロック競合)
    │
    ▼
5. (カエデが実装)
    │
    ▼
6. miyabi gate branch task-001 feature/issue-45-auth
    │
    ▼
7. miyabi gate pr task-001 78
    │ → reviewing に遷移
    │
    ▼
8. openclaw sessions spawn --agent sakura
    │ → サクラにレビュー依頼
    │
    ▼
9. miyabi gate merge task-001
    │ → GitHub API で merge 検証
    │ → ロック解放、Done に遷移
    │ → 後続タスク解放
    │
    ▼
10. openclaw hooks emit "task_completed" --task task-001
    │ → Telegram 通知、worklog 記録
```

---

## 改善が必要な項目まとめ

| # | 項目 | 場所 | 難易度 | Phase |
|---|------|------|--------|-------|
| 1 | `--approve` フラグ + 承認者記録 | gate.rs | 低 | A |
| 2 | exit code 標準化 (0/1/2) | CLI | 低 | B |
| 3 | `--emit-event` JSON イベント出力 | protocol.rs | 中 | B |
| 4 | `--format json` 全コマンド対応 | CLI | 低 | B |
| 5 | `assigned_agent` の OpenClaw ID 記録 | store.rs | 低 | A |
| 6 | OpenClaw hooks に DTP テンプレート | OpenClaw 側 | 中 | C |
| 7 | memory sync に tasks.json を追加 | OpenClaw 側 | 低 | C |
| 8 | routing が DAG レベルを考慮 | OpenClaw 側 | 中 | C 以降 |

**#1-5 は miyabi-cli-standalone 側（Phase A/B で対応可能）**
**#6-8 は OpenClaw 側（Phase C 以降で対応）**

---

## リスク評価

| リスク | 影響 | 対策 |
|--------|------|------|
| OpenClaw Gateway が落ちている | DTP CLI は動くが通知が飛ばない | DTP は GitHub SSOT に依存、Gateway は補助 |
| tasks.json の同期タイミングズレ | 別ノードが古い状態を見る | memory sync の頻度を上げる or git push で共有 |
| サブエージェントがクラッシュ | ロックが残る | TTL + stale 検出で自動解放 |
| exec-approvals と gate.rs の二重承認 | 混乱 | DTP gate を authority にし、exec-approvals は参照のみ |

---

_This plan defines how OpenClaw docks with DTP. Implementation starts after Phase A/B are complete._
