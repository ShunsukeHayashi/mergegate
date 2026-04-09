# DAG Integrity / Distributed Coordination Review

対象:
- Plan: `/Users/shunsukehayashi/.claude/plans/snuggly-bouncing-turtle.md`
- DAG 実装: `KOTOWARI/skills/openclaw-crowd/src/scheduling/task-dependency-graph.ts`
- Queue/Lock 実装: `/Users/shunsukehayashi/dev/tools/agent-skill-bus/src/queue.js`

## Findings

### 1. Critical: `tasks.json` を単一の truth + lock table に集約すると、分散書き込みで整合性が壊れます

- Plan は `fileLocks` と `dagLevels` を `tasks.json` に統合する前提です (`snuggly-bouncing-turtle.md:115-117`, `:217-223`, `:345-353`)。
- しかし現行 queue は JSONL を append 中心で扱い、少なくとも lock 取得だけは append-only に寄せています (`queue.js:25-27`, `:149-157`)。一方で status 更新は全体書き換えで、ここでも既に lost update リスクがあります (`queue.js:241-246`)。
- Plan の TaskStore は `load() -> mutate -> save()` 型に見え、分散ノードが同時に `tasks.json` を更新すると、lock 取得・状態遷移・dagLevels 再計算が互いを上書きしやすいです (`snuggly-bouncing-turtle.md:204-223`)。

影響:
- 2 エージェントが同時に lock 取得できたように見える
- `currentState` と `fileLocks` が食い違う
- `dagLevels` が古いまま残る
- GitHub sync の巻き戻しが他のローカル更新を消す

推奨:
- `tasks.json` を monolithic source of truth にするなら、必須で「単一 writer」か「OS ファイルロック + compare-and-swap + revision 番号」を入れるべきです。
- そうしないなら、`tasks.json` は snapshot/read model に留め、競合しやすいイベントは JSONL event log に分離する方が安全です。
- 最低でも `syncVersion` を gate に使い、`save()` は `expectedVersion` 不一致なら失敗させるべきです。

### 2. Critical: lock 競合判定が TOCTOU で、分散協調の核心がまだ閉じていません

- queue 実装でも `getDispatchable()` の判定と `startExecution()` の lock 獲得は別ステップです (`queue.js:79-131`, `:134-161`)。これは単機能 queue としては妥当ですが、分散 coordinator の最終形には不十分です。
- Plan の `assignAndLock()` も `hasConflict(files) === false` を gate にしていますが (`snuggly-bouncing-turtle.md:159-164`)、チェックと書き込みが原子的である保証が計画上ありません。

影響:
- 2 ノードが同時に `hasConflict=false` を観測して両方進む
- 同一ファイルの逐次タスクが並行実行される

推奨:
- `assignAndLock()` は「read-check-write」を 1 つの原子操作にするべきです。
- 具体案:
  - `flock` 相当で store 全体に短時間のメタロックを取る
  - 再読込
  - conflict 再判定
  - lock 書き込み + state 遷移 + version increment
  - fsync/atomic rename
- sequential task は「同じファイルを使うから level を落とす」のではなく、dispatch 時に lock が直列化する設計で十分です。ただしその lock は原子的である必要があります。

### 3. High: `softDependencies` を DAG edge として扱うと、parallel levels が過度に直列化されます

- 現行 DAG は soft edge を保持しても、実行可否では hard のみブロックし、soft は待ちません (`task-dependency-graph.ts:92-129`, `:272-279`)。
- Plan は `hard/soft` を `dependencies`/`softDependencies` に分けると言いながら (`snuggly-bouncing-turtle.md:100-103`)、`dagLevels` をトポロジカルソート結果のキャッシュとして持ちます (`:217-223`)。

問題:
- soft dep を level 計算に混ぜると、「本来は並列に走れるが、できれば後にしたい」タスクまで別レベルへ押し出されます。
- 逆に soft dep を level 計算から完全に外すと、UI の見た目と dispatch 順がずれる可能性があります。

推奨:
- hard DAG と soft preference を分離してください。
- `dagLevels` は hard dependencies のみで計算する。
- soft deps は level を変えず、同一 ready set 内の優先順位付けにだけ使う。
- ルール例:
  - hard dep 未完了: `blocked`
  - hard dep 完了かつ soft dep 未完了: `ready_with_soft_wait`
  - dispatcher は `ready` より後ろに置くが、空いていれば実行可

### 4. High: `dagLevels` キャッシュは無効化条件が広く、現行計画だと stale になりやすいです

- Plan は `dagLevels` を永続キャッシュとして持ちます (`snuggly-bouncing-turtle.md:217-223`, `:349-353`)。
- しかし DAG level は「依存辺の変更」だけでなく、「タスク追加/削除」「hard/soft の切替」「依存先の存在消失」「reopen/retry に伴う実行可能性の再評価」の影響を受けます。
- さらに level 自体は構造キャッシュであり、`done`/`blocked` は構造ではなく状態です。両者を同じ配列に期待すると意味が混ざります。

推奨:
- `dagLevels` を durable state にしない方がよいです。読み込み時再計算、または memoized cache に留めるのが安全です。
- 永続化するなら `dagHash` を併記し、次の変更で必ず invalidate:
  - task add/remove
  - dependency add/remove
  - hard/soft type change
  - import/sync による DAG 修正
- task status 変更では `dagLevels` 自体は invalidate せず、別に `readySetComputedAt` などを持つ方が責務分離できます。

### 5. Medium: GitHub を常に優先すると DAG truth と execution truth がねじれます

- Plan は `conflictStrategy: deterministic` で GitHub を常に優先し、ローカル done を blocked に巻き戻す方針です (`snuggly-bouncing-turtle.md:230-235`)。
- ただし GitHub Issue の open/closed は DAG 実行状態の完全な proxy ではありません。merge 後に close が遅れる、手動 reopen、Issue と PR が 1:1 でない、などが普通に起きます。

影響:
- `recordMerge()` で done にした後、sync が blocked に戻す
- downstream 解放済みなのに upstream を blocked に戻し、DAG の一貫性が崩れる

推奨:
- 完了確定は Issue 状態単独ではなく、`mergeCommit` と PR merge state を優先してください。
- GitHub Issue open/closed は advisory signal に留めるべきです。

## Direct Answers

### Is Kahn algorithm the right choice?

はい、`hard dependency` の DAG 構造検証と level 計算には妥当です。  
ただし用途は限定すべきです。

- 向いている:
  - cycle のない順序の算出
  - hard deps ベースの parallel levels
  - ready set の基礎計算
- 向いていない:
  - soft deps を含む dispatch policy
  - file lock を含む最終 dispatch 判定
  - retry/reopen/lease expiry を含む実行状態管理

結論:
- `Kahn for structure`
- `state machine for lifecycle`
- `lock manager for actual dispatch`

この三層分離がよいです。

### How should soft deps work with DAG levels?

soft dep は level を作る edge ではなく、同一 level 内の優先順ヒントとして扱うのが安全です。

推奨モデル:
- `hardDependencies`: 実行可否を決める
- `softDependencies`: 実行順 preference を決める
- `dagLevels`: hard のみで計算
- `dispatch score`: priority, age, softSatisfied, lockAvailability を合成

状態名を増やせるなら `ready` と `ready_soft_blocked` を分けると観測しやすいです。

### dagLevels cache invalidation?

永続化しないのが第一候補です。  
永続化するなら DAG 構造専用キャッシュにしてください。

invalidate 条件:
- task add/remove
- dependency add/remove
- dependency type hard/soft change
- sync/import による DAG repair

invalidate 不要:
- `pending -> analyzing -> implementing -> done` などの状態遷移だけ
- lock acquire/release

### File lock for sequential tasks sharing files?

必要です。しかも task-level ではなく file-set lease として扱うべきです。

推奨:
- lock key は正規化済みファイルパス集合
- `assignAndLock()` で原子的に lease 取得
- heartbeat/renew を入れる
- TTL expiry は即 failed 固定ではなく、`orphaned_lock` として再取得可能にする

同じファイルを触る sequential tasks は DAG level が同じでも問題ありません。  
実行順は lock で直列化し、必要なら soft dep で順序 preference を足します。

### JSONL vs monolithic JSON for concurrent writes?

結論:
- 高頻度イベント: JSONL が向いています
- snapshot/read model: monolithic JSON が向いています
- 単一 writer なしで monolithic JSON だけに寄せるのは危険です

おすすめ構成:
- `task-events.jsonl`
  - state transition
  - lock acquired/released/expired
  - DAG changed
  - sync reconciled
- `tasks.snapshot.json`
  - 現在状態の materialized view
  - 起動時または定期で再生成

もし `tasks.json` 1ファイルにこだわるなら、少なくとも:
- OS lock
- atomic rename
- version check
- crash recovery

この 4 点が必要です。

## Suggested Design Adjustment

最小変更で守りを固めるなら、以下の形がよいです。

1. DAG は hard deps のみで Kahn。
2. soft deps は dispatch priority にだけ使う。
3. lock/state/sync は event log に記録。
4. `tasks.json` は snapshot とし、壊れても再構築可能にする。
5. `assignAndLock()` を唯一の dispatch commit point にする。
6. GitHub は external confirmation であって、唯一の truth にはしない。

## Residual Risk

この計画は「LLM の揺らぎを封じる」方向性自体は非常に良いです。  
ただし現状のまま `tasks.json` に lock table と cached dagLevels を押し込むと、DAG の正しさより先に分散更新の競合で壊れる可能性が高いです。最初に固めるべきはアルゴリズム選定より commit/lock の原子性です。
