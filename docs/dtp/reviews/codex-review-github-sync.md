# Review: Deterministic Task Execution Protocol

## Findings

### 1. High: 計画は二層SSOTをまだ完全には実装しておらず、`tasks.json` を「事実の正」に昇格させすぎています

- 計画は「JSON の状態遷移だけが事実」と置いていますが、ビジョン文書と仕様文書はタスク状態のファクトSSOTを GitHub Issue / Projects に置いています。ここが正面衝突しています。
  - Plan: [`snuggly-bouncing-turtle.md`](/Users/shunsukehayashi/.claude/plans/snuggly-bouncing-turtle.md#L8) [`snuggly-bouncing-turtle.md`](/Users/shunsukehayashi/.claude/plans/snuggly-bouncing-turtle.md#L12)
  - Vision: [`agent-execution-and-memory-vision.md`](/Users/shunsukehayashi/dev/ops/openclaw-workspace/docs/design/agent-execution-and-memory-vision.md#L87) [`agent-execution-and-memory-vision.md`](/Users/shunsukehayashi/dev/ops/openclaw-workspace/docs/design/agent-execution-and-memory-vision.md#L110)
  - Spec: [`context-and-impact-spec.md`](/Users/shunsukehayashi/dev/ops/openclaw-workspace/docs/design/context-and-impact-spec.md#L54) [`context-and-impact-spec.md`](/Users/shunsukehayashi/dev/ops/openclaw-workspace/docs/design/context-and-impact-spec.md#L67)
- 現状の設計だと `tasks.json` は execution ledger と gate engine であるべきなのに、完了判定の authority まで持っています。二層SSOTを守るなら、
  - GitHub: 受入・完了・人間承認の正
  - `tasks.json`: ローカル実行状態、ロック、依存、再試行、監査
  に役割を分ける必要があります。
- `done` の必要条件は「`tasks.json` に `mergeCommit` があること」ではなく、「GitHub 上で受理された完了証跡を同期済みであること」にすべきです。`tasks.json` はそのミラーであるべきです。

### 2. High: GitHub 障害時の劣化モードが未定義で、決定性より停止性を失う可能性があります

- 計画は GitHub を強いゲートにしていますが、「GitHub API が落ちた」「レート制限」「ネットワーク断」のときに何が許可され、何が禁止されるかが定義されていません。
  - Plan merge/close gating: [`snuggly-bouncing-turtle.md`](/Users/shunsukehayashi/.claude/plans/snuggly-bouncing-turtle.md#L179)
  - Current sync has no degraded mode, retry journal, or outage classification: [`bidirectional-sync.ts`](/Users/shunsukehayashi/dev/ops/openclaw-workspace/Miyabi/packages/task-manager/src/sync/bidirectional-sync.ts#L105) [`bidirectional-sync.ts`](/Users/shunsukehayashi/dev/ops/openclaw-workspace/Miyabi/packages/task-manager/src/sync/bidirectional-sync.ts#L163)
- このままだと障害時に次の2択になります。
  - `done` を絶対に付けられず運用停止
  - 人が裏口で進めて決定性崩壊
- 必要なのは「安全に止まる」中間状態です。少なくとも以下が必要です。
  - `awaiting_github_sync` または `externally_completed_unverified`
  - GitHub API 障害と論理矛盾を分ける error class
  - pull/push の retry queue と backoff
  - 最後に確認できた GitHub 証跡の timestamp
  - 明示的な human override と理由記録

### 3. High: 双方向同期の計画が危険で、Issue close をそのまま `done` に写像すると誤完了になります

- 計画は「GitHub で Closed なのにローカルで implementing → pull で done に更新」としていますが、Issue は PR merge 以外の理由でも閉じられます。これをそのまま `done` とみなすのは強すぎます。
  - Plan: [`snuggly-bouncing-turtle.md`](/Users/shunsukehayashi/.claude/plans/snuggly-bouncing-turtle.md#L230)
- 逆方向の「ローカル done なのに Issue Open → blocked に巻き戻し」も不自然です。現行状態機械では `done -> blocked` は無効で、許されるのは `done -> pending` だけです。
  - State machine: [`task-state-machine.ts`](/Users/shunsukehayashi/dev/ops/openclaw-workspace/Miyabi/packages/task-manager/src/state/task-state-machine.ts#L76)
  - Plan rollback idea: [`snuggly-bouncing-turtle.md`](/Users/shunsukehayashi/.claude/plans/snuggly-bouncing-turtle.md#L231)
- しかも既存 `BidirectionalSync.sync()` は pull 結果をローカル task/store に適用しておらず、衝突を集めるだけです。計画の「pull で done に更新」は、既存の延長では実現されません。
  - [`bidirectional-sync.ts`](/Users/shunsukehayashi/dev/ops/openclaw-workspace/Miyabi/packages/task-manager/src/sync/bidirectional-sync.ts#L254)
- 推奨は one-way authority を明確化することです。
  - GitHub -> ローカル: 受理・クローズ・PR merge の事実を pull
  - ローカル -> GitHub: 提案された状態を push
  - ただし `done` は PR merge 証跡つき close のときだけ確定

### 4. Medium: merge commit hash の取得方法と検証経路が仕様化されていません

- 計画は `recordMerge(taskId, mergeCommit)` を置いていますが、その hash をどこから、どうやって、何と突き合わせて取得するかが未定義です。
  - Plan: [`snuggly-bouncing-turtle.md`](/Users/shunsukehayashi/.claude/plans/snuggly-bouncing-turtle.md#L179)
- 現状の sync 実装には PR 取得や merge SHA 取得の処理がありません。Issue/Label/Project しか見ていません。
  - [`github-label-sync.ts`](/Users/shunsukehayashi/dev/ops/openclaw-workspace/Miyabi/packages/task-manager/src/sync/github-label-sync.ts#L216)
  - [`projects-v2-sync.ts`](/Users/shunsukehayashi/dev/ops/openclaw-workspace/Miyabi/packages/task-manager/src/sync/projects-v2-sync.ts#L344)
- 取得方法は次のように固定するのがよいです。
  - `recordPR()` 時に `prNumber` を必須保存する
  - merge 検証時は Issue API ではなく PR API を見る
  - 第一候補: REST `GET /repos/{owner}/{repo}/pulls/{pull_number}` の `merge_commit_sha`
  - 代替: GraphQL `pullRequest.mergeCommit.oid`
  - 追加で `mergedAt`, `merged`, `state`, `baseRefOid`, `headRefOid` を保存する
- 重要なのは「Issue close から merge hash を推測しない」ことです。`prNumber` がないタスクは merge-based completion を確定できません。

### 5. Medium: 正当な例外経路が不足しており、現実運用で手動回避が増えそうです

- 仕様は HIGH/CRITICAL に human approval を要求していますが、実際の例外経路が定義されていません。
  - Plan: [`snuggly-bouncing-turtle.md`](/Users/shunsukehayashi/.claude/plans/snuggly-bouncing-turtle.md#L152)
- 必要なのは「抜け道」ではなく「監査可能な escape hatch」です。例えば:
  - GitHub 障害時に一時的に `reviewing` のまま凍結する
  - 外部で既に merge/close されたタスクを `reconcile` で取り込む
  - ドキュメント作業や運用作業のように PR を伴わない legitimate completion を `completionMode=manual|github-pr|external-op` で区別する
  - 強制 unlock / reopen / superseded / abandoned の理由欄を必須化する
- 今の計画はコード変更中心の happy path には強いですが、実運用で必ず出る「PRなしで正当に終わる仕事」「外部作業」「GitHub障害中の継続作業」を吸収できません。

### 6. Medium: 現行同期実装の前提を踏まえると、Phase 6 は「強化」ではなく再設計に近いです

- 現在の `BidirectionalSync` は conflict strategy も `local-wins` がデフォルトで、`newest-wins` も未実装同然です。
  - [`bidirectional-sync.ts`](/Users/shunsukehayashi/dev/ops/openclaw-workspace/Miyabi/packages/task-manager/src/sync/bidirectional-sync.ts#L23)
  - [`bidirectional-sync.ts`](/Users/shunsukehayashi/dev/ops/openclaw-workspace/Miyabi/packages/task-manager/src/sync/bidirectional-sync.ts#L40)
  - [`bidirectional-sync.ts`](/Users/shunsukehayashi/dev/ops/openclaw-workspace/Miyabi/packages/task-manager/src/sync/bidirectional-sync.ts#L384)
- さらに current sync は `ManagedTask` を mutate せず、store 連携もありません。
  - [`task-manager.ts`](/Users/shunsukehayashi/dev/ops/openclaw-workspace/Miyabi/packages/task-manager/src/task-manager.ts#L240)
  - [`bidirectional-sync.ts`](/Users/shunsukehayashi/dev/ops/openclaw-workspace/Miyabi/packages/task-manager/src/sync/bidirectional-sync.ts#L258)
- そのため deterministic sync を本当にやるなら、必要なのは conflict strategy 追加だけではなく、
  - state reconciliation engine
  - persistent sync cursor / version
  - per-task sync status
  - webhook event idempotency
  - API failure taxonomy
  の実装です。

## Direct Answers

### What if GitHub API is down?

- 今の計画だけでは未対応です。
- 推奨は fail-closed ですが、完全停止ではなく `awaiting_github_sync` へ遷移させることです。
- 実行中の作業は続けてもよいですが、`done` と lock release の一部は「GitHub未検証」のまま分離保存すべきです。
- 監査上は `completionRequestedAt`, `githubVerifiedAt`, `githubSyncError` を残すべきです。

### One-way push vs bidirectional?

- 完全双方向より「authority-aware bidirectional」が適切です。
- Push はローカルの提案状態を GitHub に反映するために使う。
- Pull は GitHub の受理状態をローカルへ反映するために使う。
- ただし authority は対称ではありません。`done`/`accepted` は GitHub 優先、実行中ロックや DAG はローカル優先です。
- 要するに transport は bidirectional、SSOT は asymmetric にすべきです。

### How to get mergeCommit hash?

- Issue ではなく PR から取得します。
- `recordPR()` 時に `prNumber` を保存し、reconcile 時にその PR を取得します。
- 実装候補:
  - REST: `pulls.get(...).data.merge_commit_sha`
  - GraphQL: `pullRequest.mergeCommit.oid`
- `mergeCommit` だけでなく `merged`, `mergedAt`, `headSha`, `baseSha`, `mergeMethod` も一緒に保持すると検証が安定します。

### Does the plan fully implement two-layer SSOT?

- いいえ、未完成です。
- 現状は `tasks.json` を事実の正に寄せすぎており、Vision/Spec が要求する「GitHub=ファクトSSOT、repo/tasks.json=文脈・実行ミラー」という分離が崩れています。

### Escape hatches for legitimate work?

- まだ不十分です。
- 少なくとも `manual-completion`, `external-completion`, `awaiting-github-sync`, `force-unlock-with-reason`, `reconcile-from-github` は必要です。
- どれも「自由に bypass」ではなく、理由・操作者・時刻・承認者を残すべきです。

## Recommended Changes

1. `tasks.json` の位置づけを「execution ledger / gate cache」に下げ、ファクトSSOTは GitHub のまま明記する。
2. `done` を単純 state ではなく `accepted=true` を含む受理イベントで確定させる。
3. sync を再設計し、`push proposal` と `pull authoritative facts` を分ける。
4. `Issue closed => done` を廃止し、`PR merged + issue closed/linked accepted` を確定条件にする。
5. GitHub 障害時の `awaiting_github_sync` 系ステートと retry journal を追加する。
6. manual/external completion の監査可能な escape hatch を先に仕様化する。

## Overall

計画の方向性自体はかなり良いです。特に DAG、ロック、ゲートを `tasks.json` に集約して LLM の自然言語を無力化する発想は強いです。ただし GitHub Sync と end-to-end deterministic guarantee の観点では、いまの文面は「ローカル deterministic engine」は強い一方で、「GitHub を authority とした分散整合」はまだ甘いです。

決定的にしたいなら、`tasks.json` と GitHub のどちらも同じように信じるのではなく、「何の事実はどちらが authoritative か」を状態ごとに切り分ける必要があります。
