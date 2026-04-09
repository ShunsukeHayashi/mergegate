# Auto Run Playbook Review Round 2

対象レビューの前提:
- 指定された `docs/autorun/00`〜`08`, `docs/PLAYBOOK.md`, `docs/PLAN-v1.md` はこのワークスペースでは見つかりませんでした。
- そのため、Phase 0〜8 と Codex 3-body 反映がまとまっている実質的な対象として `[deterministic-protocol-playbook.md](/Users/shunsukehayashi/dev/ops/openclaw-workspace/docs/design/deterministic-protocol-playbook.md)` を主レビュー対象にし、元草案として `/Users/shunsukehayashi/.claude/plans/snuggly-bouncing-turtle.md`、既存レビューとして `KOTOWARI/reviews/102-104` を補助参照しました。
- 以下はその前提での detailed review です。

## Findings

### 1. Critical: Phase dependency DAG が本文と矛盾しており、Phase 4 の依存が不足しています

- 依存関係表では Phase 4 は Phase 3 のみ依存です (`deterministic-protocol-playbook.md:864-867`)。
- しかし本文では Phase 4 の `FileLockManager` は `EventStore` と `SnapshotStore` に依存し (`:473-478`)、さらに Phase 1 で追加した `TaskLock` 型にも依存します (`:177-184`)。
- これは表で `4 | 3` と書かれているのが不完全で、実質は `4 | 1,3` です。
- 同様に Phase 5 も表では `3` のみ依存ですが (`:866`)、`GitHubEvidence` や `CompletionMode` は Phase 1 で入る型に依存しています (`:196-209`, `:223-249`)。

影響:
- 自動オーケストレータが依存表だけを見て実行すると、未定義型のまま Phase 4/5 を先行開始し得ます。
- 並列実行時の「着手可能判定」が誤るため、Playbook の DAG Integrity を損ねます。

推奨:
- 依存表を以下に修正してください。
  - Phase 4 → `1,3`
  - Phase 5 → `1,3`
  - Phase 6 → `1,2,3,4,5`
- 依存 DAG は prose ではなく machine-readable にも持つべきです。少なくとも `phase_id`, `depends_on`, `artifacts_produced`, `artifacts_required` を JSON/YAML で定義した方がよいです。

### 2. Critical: approval gate が「Phase 遷移」を止める設計になっておらず、個別 API gate と Playbook gate が分離しています

- 文書全体では「Do not start coding until Phase 0 is GREEN」とありますが (`deterministic-protocol-playbook.md:921`)、Phase 1 以降を止める機械的 gate は Phase 完了条件の文章だけです (`:163`, `:259`, `:364`, `:452`, `:522`, `:605`, `:777`, `:806`, `:844`)。
- Phase 6 の GATE 群は task lifecycle 用であり (`:627-762`)、Playbook lifecycle 用ではありません。つまり `registerTask` や `assignAndLock` の gate はある一方、「Phase 3 が未完了なら Phase 4 の実装を開始できない」を enforce する仕組みが書かれていません。

影響:
- 自律エージェントが「コードを書き始める/次 Phase へ進む」判定を自然言語で行ってしまう
- Playbook の自律実行で premature transition が起きる

推奨:
- Phase ごとに明示的な `approvalGate` を追加してください。
- 最低限必要な gate:
  - `phase0_gate`: 全テスト GREEN と index fresh の実コマンド結果を保存済み
  - `phaseN_exit_gate`: 生成ファイル、テスト、レビュー項目、ログ記録が揃っている
  - `phaseN_to_phaseN+1_gate`: 前 Phase の exit gate artifact を検証済み
- これを CLI でも `miyabi playbook phase-approve <phase>` のように機械可読化するのが安全です。

### 3. High: retry 条件が failure mode 網羅になっていません

- 文書に retry が明示されるのは主に GitHub API degraded mode だけです (`deterministic-protocol-playbook.md:580-590`, `:602-603`, `:775`)。
- しかし failure mode はもっと多いです。

未定義または不十分な failure mode:
- CAS conflict 発生時の retry/backoff 方針がない (`:433-437`)
- `flock` 取得失敗・lock file 破損時の retry がない (`:428-445`, `:480-488`)
- event append 失敗時の retry / fail-fast 条件がない (`:393-399`)
- snapshot rebuild が壊れたときの fallback がない (`:418-420`, `:844`)
- stale lease 解放後に元エージェントが遅延 heartbeat を送った場合の fence token がない (`:503-505`, `:692-695`)
- GitHub evidence fetch は retry があるが、GraphQL の closingIssuesReferences 部分の partial failure 方針がない (`:557-560`)
- `recordBranch`, `recordPR`, `verifyMerge`, `confirmDone` で idempotency が定義されていない (`:698-744`)

推奨:
- 各 Phase に「retry matrix」を追加してください。
- 各 API で少なくとも次を定義:
  - retryable / non-retryable
  - max attempts
  - backoff
  - idempotency key
  - terminal state
- lock 系は heartbeat だけでなく fencing token を入れるべきです。lease を再取得した新オーナーより古い heartbeat は無効化できないと race が残ります。

### 4. High: Codex 3-body review 反映は前進しているが、未反映の運用タスクがまだあります

反映済み:
- soft deps を level から外す (`deterministic-protocol-playbook.md:43-45`, `:357-360`)
- event log + snapshot (`:45`, `:368-452`)
- GitHub evidence (`:46-50`, `:536-560`)
- completionMode (`:50`, `:208-209`, `:738-740`, `:759-761`)

未反映または不足:
- review source の traceability がない
  - R1/R2/R3 のどのレビュー文書・誰の判断に基づくかを artifact として残していない。表 (`:34-50`) だけでは実装後の監査に弱いです。
- single-writer fallback の実運用手順がない
  - Windows/SMB で advisory lock 非保証と書いているだけで (`:899`)、誰が writer になるか、フェイルオーバー手順、二重 writer 検出がありません。
- queue/lock 連携の migration task がない
  - 既存 `agent-skill-bus` と `task-manager` のどちらが lock authority になるか、切替手順がないです。これは earlier review の coordination 論点に対して未着手です。
- approval/audit hook の統合タスクがない
  - Phase 8 で `record-run` はありますが (`:833`)、Phase 単位のレビュー承認記録、human approval の永続化フロー、announce/notification は Playbook に未定義です。
- protocol bypass hardening task がない
  - 検証項目には「Protocol 以外の経路で applyTransition を呼ぶテスト」がありますが (`:911`)、実際に `applyTransition` を private 化・export しない・store 直書き禁止 lint を入れる等の implementation task が相当 Phase にありません。

### 5. High: autonomous execution にはまだ曖昧さが残っています

曖昧な点:
- 成果物の exact artifact が Phase ごとに固定されていない
  - 例えば Phase 2 は何ファイルが追加されれば done か曖昧です (`:270-364`)。
- 誰が human approval を出せるか不明
  - `approvedBy` はあるが authority が未定義です (`:243-249`, `:671-675`)。
- branch existence check の実行コンテキストが不明
  - `git branch --list` はどの repo/worktree で走るか未指定です (`:700-703`)。
- `current HEAD` / `current file set` の基準が曖昧
  - `recordImpact()` の gate が repo root / branch / worktree のどれを指すか不明です (`:661-665`)。
- GitHub linked issue 判定の repo 境界が不明
  - mono-repo / cross-repo PR の扱いが未定義です (`:557-560`, `:736-740`)。

結論:
- 現状のままでは「理解している人間」なら進められますが、「Playbook だけ読んだ agent が曖昧さなく自律実行」はまだ厳しいです。

## Direct Answers

### 1. Are the Phase dependencies correct?

部分的には正しいですが、不完全です。

正しい点:
- Phase 6 が 2/3/4/5 の統合に依存するのは妥当です (`deterministic-protocol-playbook.md:867`)。
- Phase 7 → Phase 8 の流れも妥当です (`:868-869`)。

誤りまたは不足:
- Phase 4 は実質 `1,3` 依存です。
- Phase 5 は実質 `1,3` 依存です。
- 並列可能の記述「Phase 2 は Phase 3/4 と並行可能」は、Phase 6 で統合する前提では正しいものの、型変更が Phase 2/3/4 に跨るので merge coordination task が必要です (`:888-890`)。

### 2. Are the approval gates sufficient to prevent premature Phase transitions?

いいえ。task gate はあるが phase gate は足りません。

不足している gate:
- Phase start gate
- Phase exit artifact verification
- reviewer/human approval authority
- protocol bypass prevention before later phases start
- migration completion gate before switching authority from old queue/lock to new protocol

### 3. Are the retry conditions complete for each failure mode?

いいえ。不完全です。

特に不足:
- CAS conflict retry
- lock acquisition retry and fencing
- late heartbeat handling
- event append failure
- snapshot corruption/rebuild fallback
- GitHub partial evidence failure
- idempotent re-run of `recordPR`, `verifyMerge`, `confirmDone`

### 4. Is any Phase missing tasks that should be there based on the Codex 3-body reviews in docs/reviews/?

はい。少なくとも次が抜けています。

- Phase 2 or 3:
  - protocol bypass hardening task
  - transition API visibility restriction
- Phase 3 or 4:
  - single-writer / fallback writer election task
  - fence token implementation
- Phase 5:
  - legacy authority migration plan from existing queue/lock
  - GitHub evidence partial-failure handling
- Phase 8:
  - phase-level approval/audit recording
  - crash recovery / replay test
  - idempotent rerun test

補足:
- `KOTOWARI/reviews/102-104` は直接この Playbook のレビューではありませんが、そこに共通している教訓は「SSOT 不明」「計画を止める gate が弱い」「CI/計測がないまま完了条件だけ測定可能として書かれている」です (`KOTOWARI/reviews/102-storage-layer-review.md`, `KOTOWARI/reviews/103-testing-strategy-review.md`, `KOTOWARI/reviews/104-implementation-timeline-review.md`)。この弱点は現 Playbook にも残っています。

### 5. Can an agent execute these Playbooks autonomously without ambiguity?

まだ完全には無理です。

理由:
- 依存が表と実装本文でずれる
- Phase gate が機械可読でない
- retry matrix が不足
- authority/operator の定義が不足
- repo/worktree/repo-owner 境界が曖昧

人間の伴走ありなら実行可能ですが、完全自律化には次が必要です。

## Recommended Changes Before Auto Run

1. `playbook.yaml` を追加し、各 Phase に `depends_on`, `inputs`, `outputs`, `approval_gate`, `retry_policy`, `owner`, `commands` を持たせる。
2. 依存表を修正する。
   - Phase 4 → `1,3`
   - Phase 5 → `1,3`
   - Phase 6 → `1,2,3,4,5`
3. Phase gate を CLI 化する。
   - 例: `miyabi playbook check-phase 3`, `miyabi playbook approve-phase 3`
4. retry matrix を各 API/Phase に追加する。
5. fence token と single-writer fallback を明文化する。
6. legacy queue/lock から新 protocol への authority cutover task を追加する。
7. crash recovery / replay / idempotent rerun を Phase 8 の必須テストにする。

## Bottom Line

この Playbook は初回草案よりかなり改善されており、Codex 3-body review の主要論点は多く反映されています。  
ただし「自律実行できる Auto Run Playbook」として見ると、まだ task gate の設計が中心で、phase orchestration gate・retry completeness・authority migration が不足しています。現状評価は「設計としては strong、Auto Run 用ハーネスとしては未完成」です。
