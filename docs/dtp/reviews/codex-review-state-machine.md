# State Machine Gates Correctness Review

対象:
- Plan: `/Users/shunsukehayashi/.claude/plans/snuggly-bouncing-turtle.md`
- State machine: `/Users/shunsukehayashi/dev/ops/openclaw-workspace/Miyabi/packages/task-manager/src/state/task-state-machine.ts`
- Types: `/Users/shunsukehayashi/dev/ops/openclaw-workspace/Miyabi/packages/task-manager/src/types/task.ts`

レビュー観点:
- 9つの GATE が十分か
- LLM が手順を飛ばせる抜け道があるか
- 既存 27 遷移ルールと整合するか
- 追加 state が必要か
- TTL 7200s の妥当性
- 複数マシンでの file lock race

## Findings

### 1. Critical: 現行 state machine は conditions を実評価していないため、plan の GATE を state machine へ載せても素通りする

`TaskStateMachine.canTransition()` は `from -> to` の組み合わせしか見ておらず、`conditions` は一切評価していません。`transition()` と `applyTransition()` も同様です。したがって plan の「GATE を state machine の gate として実装」という意図に対して、既存実装を「変更なし」で使う案は成立しません。

根拠:
- `conditions` が定義されている: `task-state-machine.ts:34-80`
- `canTransition()` は `r.to === to` しか見ない: `task-state-machine.ts:103-106`
- `applyTransition()` も `transition()` の結果だけで更新 task を返す: `task-state-machine.ts:149-189`

影響:
- `skip_analysis`, `dependency_blocked`, `review_approved`, `requires_deployment`, `no_deployment` など既存 27 ルール側の条件文字列も現在は強制力ゼロです。
- plan 側で 9 GATE を追加しても、別経路から `applyTransition(task, 'done', ...)` を呼べば通ります。

必要修正:
- state machine 自体に `canTransition(task, to, context)` を追加して条件評価を内包する
- もしくは protocol を唯一の遷移窓口にして、`TaskStateMachine` を外から直接呼べないようにする

### 2. Critical: 既存呼び出し元は applyTransition() の戻り値 task を保存しておらず、状態遷移そのものが永続化されない

現行コードでは `applyTransition()` は新しい `task` を返しますが、主要呼び出し元がそれを map に戻していません。つまり transition が valid でも、実 task は更新されない経路があります。

根拠:
- `TaskManager.updateTaskState()` は `result.valid` を返すだけ: `task-manager.ts:148-158`
- `TaskExecutor.execute()` でも `transitionResult.task` を採用していない: `task-executor.ts:107-126`, `157-177`, `186-187`

影響:
- plan の gate を増やす前に、現行 state progression が fact になっていない
- 「JSON の状態遷移だけが事実」という North Star と真逆

必要修正:
- 全遷移を store-backed immutable update に統一
- `applyTransition` で返した task を即 store に commit できないなら、その API を使わせない

### 3. High: 9 GATE は「必要証跡」は見ているが、「順序の完全性」をまだ保証していない

現行 9 GATE は以下の点で不足があります。

- Step 3 は `impact !== null` だけで、impact が対象 branch / commit / file set に対する最新分析かを検証していない
- Step 4 は `lock !== null` を見るが、「lock を取った主体」と「今実行している agent」が一致するかを見ていない
- Step 5 は `branchName` の書式だけで、実際に remote / worktree 上に存在するかを見ていない
- Step 6 は `prNumber > 0` だけで、PR がその branch に紐づくか、Draft/Ready/Closed/Merged のどれかを見ていない
- Step 7 は `mergeCommit` の SHA 形式だけで、対象 PR が merged 済みか、merge commit がその PR/head を含むかを見ていない
- Step 8 は `Issue Closed` を見るが、close reason や linked PR による close か、人手クローズかを区別しない

LLM の飛ばし方の例:
- 適当な 40 hex を `mergeCommit` に書けば reviewing -> done 条件を満たせる
- 別 task の PR 番号を流用して implementing -> reviewing へ進める
- 古い impact 結果を再利用して analyzing -> implementing に進める

追加すべき gate:
- `impact.analyzedAt >= latestTaskMaterializationAt`
- `impact.fileSet` or `impact.inputHash` が現在の対象と一致
- `lock.lockedBy === assignee@node` かつ lock 未期限切れ
- `branchExists && branchHeadCommit !== null`
- `pr.headRefName === branchName && pr.state === OPEN`
- `pr.reviewDecision === APPROVED` または required checks success
- `mergeCommit` が GitHub 上で当該 PR の merge commit と一致
- `issue.closedByPullRequest === prNumber` 相当の検証

### 4. High: plan の reviewing -> done は既存 27 ルールと衝突する

plan では Step 7 を `reviewing -> done` にしていますが、既存 state machine では:
- `reviewing -> done` は `review_approved && no_deployment`
- `deploying -> done` は sideEffects `close_issue`, `merge_pr`

根拠:
- `task-state-machine.ts:57-65`

衝突内容:
- plan では merge が done の必須条件になっている
- 既存ルールでは `reviewing -> done` は「デプロイ不要なら done」、`deploying -> done` が merge/close を担う

このままだと:
- デプロイ不要タスクの完了 semantic が変わる
- 既存の `deploying` state がほぼ空洞化する

結論:
- 既存 27 ルールと互換にするなら、`reviewing -> done` を廃止して `reviewing -> merging|deploying` を経由させるべきです
- 少なくとも `merge` と `deploy` は別の irreversible event なので同じ gate にしない方が安全です

### 5. High: 追加 state が必要。少なくとも `ready`, `locked`, `merged` のどれかを state として持たないと gate が metadata 化して見落とされやすい

現行 state は:
- draft, pending, analyzing, implementing, reviewing, deploying, done, blocked, failed, cancelled

不足している意味上の段階:
- `ready`: dependencies 解消済みだが未着手
- `locked` または `assigned`: 実装開始前に資源確保済み
- `merged`: review 承認済みで merge 完了、ただし post-merge audit/close 未完
- `auditing` または `finalizing`: worklog/skill-bus/issue close 整合を取る終端前処理

理由:
- 現 plan は Step 4, 5 を「implementing 中の metadata」で表しており、state 上は見えません
- そのためオペレーション側が `currentState === implementing` だけ見たときに、lock 未取得・branch 未作成・PR 未作成の task を同列に扱ってしまいます

最小提案:
- `pending -> ready -> analyzing -> locked -> implementing -> reviewing -> merged -> done`
- deploy が必要なら `reviewing -> deploying -> merged -> done` でもよい

もし state を増やしたくないなら:
- state ではなく `phaseChecklist` の必須項目を machine-evaluable に持つ必要があります

### 6. Medium: pending -> analyzing gate が既存 semantics と少しズレている

plan Step 2 は `dependencies.every(done)` を `pending -> analyzing` の gate にしていますが、既存 state machine には:
- `pending -> blocked` with `dependency_blocked`
- `blocked -> pending` with `dependency_resolved`

根拠:
- `task-state-machine.ts:38-40`, `68-69`

気になる点:
- dependency 未解決の task は本来 `blocked` に入る設計
- plan の `checkDependencies(): 'ready' | 'blocked'` はあるが、gate table 上は `pending -> analyzing` しか記述がなく、`pending` に留め続ける実装にも見える

提案:
- dependency 未解決時は必ず `blocked`
- 依存解決後に `blocked -> pending` を踏ませる
- `ready` を導入するなら `pending` は単なる backlog、`ready` が dispatchable を表す方が明確

### 7. Medium: `pending -> implementing (skip_analysis)` ルールとの整合が未定義

既存 27 ルールには `pending -> implementing` が存在します。

根拠:
- `task-state-machine.ts:39`

plan では Step 3 で impact 記録が implementing の必須条件なので、skip_analysis を事実上禁止する方向に見えます。これは設計としてはよいですが、既存ルールを「変更なし」とは両立しません。

提案:
- `skip_analysis` を削除する
- もしくは `analysis_mode: full | waived` を明示し、waived でも human approval 必須にする

### 8. Medium: TTL 7200s は固定値としては長すぎ、しかも heartbeat 前提がない

7200 秒は 2 時間です。長時間の実装には短すぎることもありますが、「エージェントが死んだのに他ノードが2時間待つ」という意味では長すぎます。

問題:
- lock refresh/heartbeat が plan にない
- 実装中に TTL 失効すると、別ノードが同じファイルを取りに行ける
- 逆に crash 後の回復は最長 2 時間止まる

提案:
- 固定 TTL ではなく short lease + heartbeat 方式
- 例: lease 300s, renew every 60s, stale after 2 missed renewals
- 人手 override 用の `force unlock` は残す

7200s が妥当なのは:
- 単一マシン
- 長いジョブ
- 他ノードが少ない

今回は「across machines」が前提なので、固定 7200s は非推奨です。

### 9. Medium: tasks.json 内 `fileLocks` 更新はクロスマシン race に弱い

plan は JSONL から `tasks.json` の `fileLocks` へ統合しますが、単一 JSON ドキュメントの read-modify-write は append-only より競合に弱いです。

想定 race:
- Machine A, B が同時に load
- 両者とも conflict なしと判断
- 両者が別々に save
- 後勝ち write で片方の lock が消える、または両方が lock 獲得したと誤認する

JSONL append-only より悪化する点:
- append-only は競合検査を後から再生しやすい
- 単一 JSON overwrite は lost update しやすい

最低限必要:
- OS レベルの lock file か atomic rename
- compare-and-swap 用の version check
- `syncVersion` ではなく document version で CAS
- lock acquisition を「check + write」でなく「single atomic claim」に寄せる

クロスマシンで本当に安全にするなら:
- 共有 filesystem の advisory lock 依存は弱い
- ローカル files しかないなら coordinator 1台に集約すべき
- もしくは GitHub/SQLite/Postgres 等の単一調停者が必要

## Are all 9 GATEs sufficient?

結論: 不十分です。

足りていないのは主に 3 系統です。

1. Provenance gate
- その証跡が「今の task/branch/PR のものか」を確認していない

2. Freshness gate
- impact / lock / PR / merge 情報が最新かを見ていない

3. Authority gate
- 誰がその state を進めてよいか、誰の lock か、human approval がどこに記録されているかを見ていない

## Can an LLM still skip steps?

はい。現案のままだと skip できます。

代表例:
- metadata を直接埋めて `recordPR()` や `recordMerge()` を呼ぶ
- 既存の `updateTaskState()` / `applyTransition()` 経路から protocol をバイパスする
- stale な lock を握ったまま別ノードが再取得する
- 実 branch/PR/GitHub state を見ずに JSON だけ整えて done にする

## Compatibility with existing 27 rules

結論: そのままでは非互換です。

非互換ポイント:
- `conditions` が未評価なので既存ルールの意味が実装されていない
- `pending -> implementing (skip_analysis)` と plan の mandatory impact gate が衝突
- `reviewing -> done` / `deploying -> done` の意味が plan の merge 必須完了定義と衝突

互換にしたいなら:
- 27 ルールを単に残すのでなく、条件を実評価する新 API へ移行
- 完了定義を `done = merged + issue closed + audit recorded` に寄せるなら、ルール表も更新が必要

## Should additional states exist?

結論: 追加した方が安全です。

推奨候補:
- `ready`
- `locked` or `assigned`
- `merged`
- `auditing` or `finalizing`

最小でも `merged` は有用です。`review approved` と `merge complete` と `audit complete` は別イベントだからです。

## Is TTL 7200s appropriate?

結論: 固定値としては不適切です。

- 単一マシンなら許容
- 複数マシン協調では長すぎる
- heartbeat/renewal がないため safety より liveness も safety も中途半端

推奨:
- lease 300-600s
- renew interval 60-120s
- owner heartbeat 必須
- expired lock reacquire 時に fencing token を使う

## Race conditions in file locks across machines

結論: 現案のままでは race があります。

主な懸念:
- lost update
- double-acquire
- stale lock resurrection
- clock skew による TTL 判定ズレ
- network partition 中の二重実行

対策優先順:
1. Single writer/coordinator に lock 判定を集約
2. atomic write + CAS version check
3. lease renewal と fencing token
4. monotonic timestamp source を一元化

## Recommended design changes

1. `TaskStateMachine` を gate-aware にする
- `canTransition(task, to, context)` を導入
- `conditions` を文字列配列でなく predicate 群へ寄せる

2. `DeterministicExecutionProtocol` を唯一の state mutation 経路にする
- `TaskManager.updateTaskState()` などの素通し経路は封鎖または内部専用化

3. 完了 state を分解する
- `reviewing -> merged -> done`
- deploy が必要なら `reviewing -> deploying -> merged -> done`

4. lock は fixed TTL でなく renewable lease にする

5. `tasks.json` overwrite で lock を持たない
- 少なくとも CAS + atomic rename
- 可能なら coordinator/SQLite 等へ分離

6. gate を「presence check」から「verified linkage check」に上げる
- PR 番号がある、ではなく「その task の branch を head に持つ open/merged PR がある」

## Bottom line

plan の方向性自体は正しいです。ただし現在の state machine を「そのまま使う」前提だと、GATE は仕様書上の強い言葉に留まり、実装上は抜けられます。

最優先で直すべき点は次の 3 つです。

1. state machine に条件評価を実装する
2. state transition を store に永続反映する
3. merge / audit / close を `done` 前に明示的に分離する

この 3 点を入れない限り、「LLM の揺らぎを JSON state だけで封じる」保証には届きません。
