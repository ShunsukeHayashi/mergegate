# Review: OpenClaw Agent Integration for DTP

## Executive Summary

現状の `deterministic-task-protocol` は、OpenClaw `main` エージェントが自律的に `draft -> done` を回すにはまだ入口が足りません。最大の理由は 3 つです。

1. `dtp` バイナリに実用 CLI がまだなく、`src/main.rs` は単なる bootstrap 出力しか持っていません。[`src/main.rs:1`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/src/main.rs#L1)
2. protocol 実装は in-memory happy path に留まっており、GitHub 証跡検証、lease heartbeat、reconcile、event log、JSON 出力、終了コード、identity 付きの運用コマンドが未実装です。[`src/protocol.rs:53`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/src/protocol.rs#L53) [`src/store.rs:47`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/src/store.rs#L47)
3. repo 直下の `AGENTS.md` は Rust 開発ガイドとしては十分ですが、OpenClaw main がこの repo でどう振る舞うかは書かれていません。逆に `refs/AGENTS.md` と `refs/SOUL.md` は広すぎて、この repo 専用の実行契約になっていません。[`AGENTS.md:1`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/AGENTS.md#L1) [`refs/AGENTS.md:1`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/refs/AGENTS.md#L1) [`refs/SOUL.md:56`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/refs/SOUL.md#L56)

Playbook 側はかなり正しく、OpenClaw integration の要件もおおむね言語化できています。特に authority-aware sync、`awaiting_github_sync`、lease + heartbeat、escape hatch、CLI subcommands は妥当です。[`docs/PLAYBOOK.md:46`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/docs/PLAYBOOK.md#L46) [`docs/PLAYBOOK.md:456`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/docs/PLAYBOOK.md#L456) [`docs/PLAYBOOK.md:526`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/docs/PLAYBOOK.md#L526) [`docs/PLAYBOOK.md:747`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/docs/PLAYBOOK.md#L747) [`autorun/phase-7-cli/TASKS.md:8`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/autorun/phase-7-cli/TASKS.md#L8)

以下、質問ごとに整理します。

## 1. Can an OpenClaw agent (main) call the DTP CLI to execute the full protocol? What is missing?

### Short answer

まだできません。

### Why not

- `dtp` CLI が未実装です。`src/main.rs` は `"dtp: deterministic-task-protocol bootstrap"` を出すだけです。[`src/main.rs:1`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/src/main.rs#L1)
- protocol の public API は Rust から直接呼ぶ前提のメソッド群だけで、OpenClaw main が shell command として安全に使える interface になっていません。[`src/protocol.rs:62`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/src/protocol.rs#L62)
- `TaskStore::with_file()` はあるものの、protocol の `new()` は default in-memory store を使うだけで、CLI/daemon から永続ストアを注入する導線がありません。[`src/protocol.rs:54`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/src/protocol.rs#L54) [`src/store.rs:53`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/src/store.rs#L53)
- merge 検証は `record_merge(task_id, merge_commit)` に文字列を渡せば通る設計で、GitHub API からの出自検証がありません。OpenClaw agent が勝手に SHA を渡せてしまいます。[`src/protocol.rs:183`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/src/protocol.rs#L183) [`src/gate.rs:72`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/src/gate.rs#L72)
- lock は固定 TTL のみで heartbeat/lease renew がありません。cron から lease 維持もできません。[`src/types.rs:26`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/src/types.rs#L26) [`src/lock.rs:52`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/src/lock.rs#L52)
- state model がまだ `merged` / `awaiting_github_sync` / `manual` 系を持っていません。OpenClaw 運用ではここが重要です。[`src/types.rs:7`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/src/types.rs#L7) [`docs/PLAYBOOK.md:211`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/docs/PLAYBOOK.md#L211)
- output contract が未定義です。OpenClaw main が自律運転するには JSON 出力と機械判定可能な exit code が必須ですが、現状ありません。[`autorun/phase-7-cli/TASKS.md:28`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/autorun/phase-7-cli/TASKS.md#L28)

### Minimum missing pieces before OpenClaw can drive it

1. 実 CLI 実装
2. 永続 store path の指定
3. GitHub credentials / repo context の指定
4. JSON 出力
5. 明確な exit code
6. identity-aware lock ownership
7. heartbeat renew / stale sweep
8. GitHub evidence fetch + reconcile
9. manual/external completion path
10. audit hooks

## 2. How should `CLAUDE.md` and `AGENTS.md` be structured for this repo so agents know the rules?

### Current problem

- repo 直下 `AGENTS.md` は開発者向けで、OpenClaw/Codex/Claude がこの repo で何を守るべきかの運用契約が不足しています。[`AGENTS.md:16`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/AGENTS.md#L16)
- `refs/AGENTS.md` は openclaw-workspace 全体の map、`refs/SOUL.md` は OpenClaw main の人格/優先順位です。どちらも大事ですが、この repo 専用の「DTPをどう運転するか」までは書いていません。[`refs/AGENTS.md:6`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/refs/AGENTS.md#L6) [`refs/SOUL.md:58`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/refs/SOUL.md#L58)
- 現状 `.claude/` と `.codex/` にガイドファイルがありません。これは repo-specific behavior を agents に教える機会を逃しています。

### Recommended structure

#### `AGENTS.md` at repo root

役割: repo-local execution contract。全エージェント共通の「この repo ではどう動くか」を簡潔に書く。

入れるべき内容:

1. Repo mission
2. SSOT split
3. Mandatory startup reads
4. Allowed / forbidden transitions
5. Required commands before and after edits
6. DTP-specific invariants
7. CLI-first operation rules
8. Testing and verification contract
9. Links to `refs/AGENTS.md` / `refs/SOUL.md` / `docs/PLAYBOOK.md`

サンプル見出し:

- `Purpose`
- `Read First`
- `Fact vs Execution Ledger`
- `If You Modify Protocol`
- `If You Run as OpenClaw Main`
- `Required Verification`
- `Escape Hatches`

特に重要な文言:

- `GitHub is fact SSOT; local snapshot is execution ledger.`
- `No agent may mark a task done without GitHub evidence or an audited manual/external completion path.`
- `All automation must prefer dtp CLI over direct JSON edits.`
- `Heartbeat ownership must match agent@node.`

#### `CLAUDE.md`

役割: Claude Code / OpenClaw-style operator brief。対話型エージェントや main agent 向けに「どの順で読むか」「どう dispatch するか」を書く。

入れるべき内容:

1. Session startup for this repo
2. `docs/PLAYBOOK.md` is normative for implementation order
3. `autorun/*/TASKS.md` and `GATE.md` drive phase execution
4. When acting as OpenClaw main, prefer orchestration and delegation, not direct code hoarding
5. When acting on user message, direct answer still wins per `refs/SOUL.md`
6. Exact `dtp` commands for autonomous operation
7. Escalation points

推奨順序:

1. `refs/SOUL.md`
2. repo `AGENTS.md`
3. `docs/PLAYBOOK.md`
4. relevant `autorun/phase-*/TASKS.md`
5. relevant handoff note

### Concrete recommendation

- `AGENTS.md`: durable, cross-agent, repo-local invariant
- `CLAUDE.md`: runtime/operator workflow for Claude/OpenClaw style agents
- `refs/AGENTS.md` / `refs/SOUL.md`: inherited upstream context only

つまり、`refs/*` を読むだけでは不十分で、この repo 自身の `AGENTS.md` と `CLAUDE.md` に DTP運転ルールを持つべきです。

## 3. What is the minimal CLI interface needed for an agent to autonomously run draft-to-done?

### Truly minimal autonomous set

Playbook の一覧は広めですが、OpenClaw main が自律的に `draft -> done` を回すための最小集合はこれです。

1. `dtp register`
2. `dtp dispatchable`
3. `dtp impact`
4. `dtp approve`
5. `dtp assign`
6. `dtp heartbeat`
7. `dtp branch`
8. `dtp pr`
9. `dtp verify-merge`
10. `dtp confirm-done`
11. `dtp status --json`

### Why each is needed

- `register`: Issue と completion mode を固定する
- `dispatchable`: main が次に実行すべき task を判定する
- `impact`: GATE 3 を通す
- `approve`: HIGH/CRITICAL の人間承認を監査付きで残す
- `assign`: lock と owner identity を確定する
- `heartbeat`: 実装中 lease を維持する
- `branch`: 実在 branch を state と結びつける
- `pr`: PR identity を task に紐づける
- `verify-merge`: GitHub authority から merged 事実を pull する
- `confirm-done`: completion mode に応じて最終確定する
- `status --json`: orchestration loop が機械判定する

### Additional commands that are near-minimal in production

OpenClaw 運用まで考えると、次の 4 つも事実上必要です。

1. `dtp unlock --reason --operator`
2. `dtp reconcile`
3. `dtp sweep-leases`
4. `dtp events --json`

### Output contract

各コマンドに最低限必要な contract:

- `--json` support
- stable schema
- deterministic exit code
- no interactive prompt by default

推奨 exit code:

- `0`: success
- `1`: gate rejected
- `2`: invalid input
- `3`: conflict / lease lost
- `4`: GitHub unavailable / degraded
- `5`: invariant violation / bug

### Missing from current implementation

- `src/main.rs` に clap CLI なし
- `approve` コマンド相当なし
- `status` / `dispatchable` / `reconcile` / `sweep-leases` なし
- JSON output なし
- store path / repo context / operator identity flags なし

## 4. How should heartbeat/lease renewal work when called from a cron job?

### Current gap

現状は TTL だけで、lease renew 機構がありません。[`src/types.rs:33`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/src/types.rs#L33) [`src/lock.rs:52`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/src/lock.rs#L52)

Playbook は lease 300s + heartbeat 60s + missed heartbeats 2 を提案していて、これは妥当です。[`docs/PLAYBOOK.md:40`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/docs/PLAYBOOK.md#L40) [`docs/PLAYBOOK.md:467`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/docs/PLAYBOOK.md#L467)

### Recommended model

#### Lease data

`TaskLock` should carry:

- `locked_by`
- `locked_at`
- `lease_duration_sec`
- `heartbeat_interval_sec`
- `last_heartbeat`
- `fencing_token`
- `affected_files`

### Cron interaction model

cron からは 2 系統に分けるべきです。

1. owner cron: lease renew
2. janitor cron: stale lease sweep

#### Owner renew

コマンド例:

```bash
dtp heartbeat <task-id> --agent main --node mainmini --json
```

必要な gate:

- task exists
- task is in `implementing` or other active state
- lock exists
- lock owner matches `agent@node`
- lease is not already fenced off by newer owner

成功時:

- `last_heartbeat` 更新
- `lock_heartbeat` event 記録
- new `expires_at` を JSON 出力

失敗時:

- exit `3` if lock ownership lost
- exit `1` if task not heartbeat-eligible
- exit `4` if store or GitHub degraded path relevant

#### Janitor sweep

コマンド例:

```bash
dtp sweep-leases --json
```

責務:

- stale lease を検出
- forced release を event として記録
- task state を `blocked` or `pending_recovery` 相当に移す
- owner mismatch を報告

### Important rule

cron は「とりあえず更新」してはいけません。owner identity を必ず指定し、`main@mainmini` が保持する lock を `main@mainmini` の cron だけが renew できるようにすべきです。

### Recommended timings

- lease duration: 300s
- heartbeat interval: 60s
- stale after: 2 missed intervals
- janitor sweep: every 2-5 min

### Why cron needs both renew and sweep

OpenClaw main は long-running conversation とは限らず、cron や heartbeat runner に lease 維持を委譲する場面があります。そのため:

- active owner renews
- janitor reclaims abandoned leases

の二面構成が必要です。renew だけだと orphan lock が残り、sweep だけだと稼働中タスクを誤解放します。

## 5. What escape hatches are needed for production operation?

### Required escape hatches

1. `force-unlock`
2. `reconcile-from-github`
3. `manual-complete`
4. `external-op-complete`
5. `reopen`
6. `takeover-lock`
7. `mark-awaiting-github-sync`
8. `retry-github-sync`
9. `abandon-task`
10. `supersede-task`

### Why they are needed

- agent crash
- lock orphaning
- GitHub outage
- merge happened externally
- no-PR legitimate work
- human intervention in production

### Minimum audited fields for every escape hatch

- `operator`
- `reason`
- `approved_by`
- `timestamp`
- `previous_state`
- `new_state`
- `correlation_id`

### Specific recommendations

#### `force-unlock`

必要条件:

- operator required
- reason required
- current lock holder and age recorded

#### `manual-complete`

使いどころ:

- docs-only
- external ops
- emergency human completion

必要条件:

- `completion_mode in {manual, external-op}`
- human approval present
- audit event recorded

#### `reconcile-from-github`

使いどころ:

- PR merged externally
- issue closed externally
- webhook missed

これは production では特に重要です。OpenClaw main は drift 修正のために自律的に reconcile できる必要があります。

#### `takeover-lock`

クラッシュ復旧で重要です。`force-unlock` と `assign` を別々にやるより、

- old owner stale confirmed
- operator approves takeover
- new fencing token issued

を一発でやる方が安全です。

## Additional Findings

### A. Current `AGENTS.md` is too generic for autonomous agents

repo root の `AGENTS.md` は Rust 開発ガイドとしては問題ありませんが、agent execution protocol が不足しています。OpenClaw main はこの文書を読んでも、

- どの docs を先に読むべきか
- `refs/*` をどう扱うか
- `dtp` をどう叩くか
- いつ人間に escalate すべきか

が分かりません。[`AGENTS.md:63`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/AGENTS.md#L63)

### B. `refs/AGENTS.md` and `refs/SOUL.md` should remain references, not runtime-only truth

symlink 自体は良いです。upstream workspace のルールと main の人格を持ち込めます。[`refs/AGENTS.md:6`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/refs/AGENTS.md#L6) [`refs/SOUL.md:1`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/refs/SOUL.md#L1)

ただし、この repo の運用契約まで symlink 先に依存すべきではありません。repo local の `AGENTS.md` / `CLAUDE.md` で上書きすべきです。

### C. Playbook already implies the right CLI, but implementation lags far behind

Playbook と autorun task は、OpenClaw integration の設計書としてかなり使えます。特に:

- `heartbeat`
- `verify-merge`
- `confirm-done`
- `unlock`
- `reconcile`
- `run-e2e`

はそのまま CLI contract にすべきです。[`docs/PLAYBOOK.md:781`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/docs/PLAYBOOK.md#L781) [`autorun/phase-7-cli/TASKS.md:10`](/Users/shunsukehayashi/dev/platform/_core/miyabi-private/deterministic-task-protocol/autorun/phase-7-cli/TASKS.md#L10)

## Recommended Next Moves

1. Implement real `dtp` CLI in `src/main.rs` with clap and JSON output first.
2. Add repo-local `CLAUDE.md` focused on OpenClaw/Codex operation order.
3. Expand repo-local `AGENTS.md` from generic Rust guidance to execution contract.
4. Add persistent store path + identity flags to every mutating command.
5. Implement `heartbeat` and `sweep-leases` before trying autonomous OpenClaw runs.
6. Implement `verify-merge` and `reconcile` before allowing `confirm-done`.
7. Add audited escape hatches before production use.

## Direct Answers

### (1) Can OpenClaw main call the dtp CLI for the full protocol?

まだ無理です。`dtp` CLI 自体が未実装で、GitHub verification、heartbeat、JSON output、persistent store、escape hatch が足りません。

### (2) How should `CLAUDE.md` and `AGENTS.md` be structured?

- `AGENTS.md`: repo-local invariant and execution contract
- `CLAUDE.md`: runtime/operator workflow for Claude/OpenClaw style agents
- `refs/*`: inherited upstream context only

### (3) Minimal CLI for autonomous draft-to-done?

`register`, `dispatchable`, `impact`, `approve`, `assign`, `heartbeat`, `branch`, `pr`, `verify-merge`, `confirm-done`, `status --json` が最小です。運用上は `unlock`, `reconcile`, `sweep-leases`, `events --json` もほぼ必須です。

### (4) How should heartbeat/lease renewal work from cron?

owner cron による `heartbeat` と janitor cron による `sweep-leases` の二系統。identity match と fencing token が必要です。lease 300s / heartbeat 60s / stale after 2 misses を推奨します。

### (5) What escape hatches are needed?

`force-unlock`, `reconcile-from-github`, `manual-complete`, `external-op-complete`, `reopen`, `takeover-lock`, `mark-awaiting-github-sync`, `retry-github-sync`, `abandon-task`, `supersede-task` が必要です。全て audit fields 必須です。
