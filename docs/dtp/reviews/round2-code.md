# Round 2 Code Review: Deterministic Task Protocol Rust Codebase

> **アーカイブ・スコープ注意（2026-04-10）**  
> 本文は **当時の作業ディレクトリ**（`openclaw-workspace`）向けの調査記録であり、**本リポジトリ `miyabi-cli-standalone` の `crates/miyabi-core` / `crates/miyabi-cli` に対するコードレビュー結果ではありません**。ここに書かれた「Rust が無い」等の記述は **そのワークスペース限定**です。現行実装・レビューは `docs/dtp/PLAYBOOK-v2.md` および `crates/**/*.rs` を正としてください。

対象依頼:
- Read all Rust source files in `src/`: `lib.rs`, `types.rs`, `state.rs`, `dag.rs`, `lock.rs`, `store.rs`, `protocol.rs`, `gate.rs`, `main.rs`
- Check:
  1. GATE conditions completeness/correctness
  2. State machine coverage vs Playbook
  3. DAG correctness
  4. Soundness / error handling / logic bugs
  5. `cargo test` / `cargo clippy` and test sufficiency

作業場所:
- Workspace root: `/Users/shunsukehayashi/dev/ops/openclaw-workspace`
- Relevant existing implementation found: `Miyabi/packages/task-manager/`
- Relevant playbook found: `docs/design/deterministic-protocol-playbook.md`

## Executive Summary

現ワークスペースでは、依頼で指定された Rust codebase は見つかりませんでした。少なくともこの repo 配下には:

- `Cargo.toml`
- `src/lib.rs`
- `src/types.rs`
- `src/state.rs`
- `src/dag.rs`
- `src/lock.rs`
- `src/store.rs`
- `src/protocol.rs`
- `src/gate.rs`
- `src/main.rs`

が存在しません。

代わりに存在するのは TypeScript 実装:

- `Miyabi/packages/task-manager/src/task-manager.ts`
- `Miyabi/packages/task-manager/src/types/task.ts`
- `Miyabi/packages/task-manager/src/state/task-state-machine.ts`

および、未実装 Playbook:

- `docs/design/deterministic-protocol-playbook.md`

です。

したがって、依頼された Rust コードそのものに対する (1)〜(5) の完全レビューは **実施不能** です。以下では、その理由、現存実装との乖離、実行した `cargo` コマンドの結果を記録します。

## What Was Actually Found

### Found

- Playbook: `docs/design/deterministic-protocol-playbook.md`
- TypeScript task manager package: `Miyabi/packages/task-manager/src/...`

### Not Found

- Rust crate root (`Cargo.toml`)
- Requested Rust source files under any `src/`
- `gate.rs`, `protocol.rs`, `dag.rs`, `lock.rs`, `store.rs` as Rust modules

## Command Evidence

### Source tree inspection

`Miyabi/packages/task-manager/src` contains only TypeScript files/directories:

- `index.ts`
- `task-manager.ts`
- `types/*.ts`
- `state/task-state-machine.ts`
- `execution/*.ts`
- `sync/*.ts`
- `decomposition/*.ts`

No Rust files were present there.

### cargo test

Executed in:

- `/Users/shunsukehayashi/dev/ops/openclaw-workspace/Miyabi/packages/task-manager`

Result:

```text
error: could not find `Cargo.toml` in `/Users/shunsukehayashi/dev/ops/openclaw-workspace/Miyabi/packages/task-manager` or any parent directory
```

### cargo clippy

Executed in:

- `/Users/shunsukehayashi/dev/ops/openclaw-workspace/Miyabi/packages/task-manager`

Result:

```text
error: could not find `Cargo.toml` in `/Users/shunsukehayashi/dev/ops/openclaw-workspace/Miyabi/packages/task-manager` or any parent directory
```

## Review Outcome by Requested Question

## 1. Are the GATE conditions in `gate.rs` complete and correct?

**Unable to review directly** because `gate.rs` does not exist in this workspace.

What can be said from the Playbook:

- The playbook defines GATE semantics conceptually in `docs/design/deterministic-protocol-playbook.md`
- But there is no Rust `gate.rs` implementation here to verify completeness, ordering, or enforcement

What can be said from the existing TypeScript implementation:

- There is no equivalent `gate.ts` module
- Existing state conditions are only strings on transition rules in `task-state-machine.ts`
- Those conditions are not evaluated by `canTransition()` or `applyTransition()`

Implication:

- In the currently present code, GATE semantics are not implemented as executable guards
- So if the intended Rust crate exists elsewhere, it has not been checked in here

## 2. Does the state machine in `state.rs` cover all transitions from the Playbook?

**Unable to review directly** because `state.rs` does not exist.

However, comparing the Playbook to the existing TypeScript state machine shows a clear mismatch:

Playbook expects:

- `merged` state
- `awaiting_github_sync` state
- stronger gate-driven transitions around review/merge/finalization

Existing TypeScript state machine has only:

- `draft`
- `pending`
- `analyzing`
- `implementing`
- `reviewing`
- `deploying`
- `done`
- `blocked`
- `failed`
- `cancelled`

Missing vs Playbook:

- `merged`
- `awaiting_github_sync`

Also, the TypeScript state machine still includes transitions incompatible with the newer deterministic plan:

- `pending -> implementing` with `skip_analysis`
- `reviewing -> done` without an explicit `merged` state

So if `state.rs` was meant to implement the Playbook, that implementation is not present here, and the nearest existing implementation is not yet aligned with the Playbook.

## 3. Is the DAG implementation in `dag.rs` correct?

**Unable to review directly** because `dag.rs` does not exist.

No Rust DAG implementation matching the requested file layout was found in this workspace.

There is also no TypeScript `dag` module currently present under `Miyabi/packages/task-manager/src/` matching the deterministic protocol plan. The playbook describes one, but it is still plan-only in this repo.

Therefore:

- topological sort correctness: not reviewable
- cycle detection correctness: not reviewable
- dispatch semantics: not reviewable

## 4. Are there any soundness issues, missing error handling, or logic bugs?

For the requested Rust codebase: **not reviewable because the codebase is absent**.

For the nearest existing TypeScript implementation, there are already serious soundness gaps:

### A. Transition conditions are declarative only, not enforced

In `Miyabi/packages/task-manager/src/state/task-state-machine.ts`:

- transition rules contain `conditions?: string[]`
- `canTransition(from, to)` checks only `from -> to`
- `applyTransition()` does not evaluate those conditions

Consequence:

- `review_approved`, `dependency_blocked`, `skip_analysis`, etc. are currently metadata, not executable gates

### B. Updated task objects are returned but not persisted by callers

`applyTransition()` returns `{ valid, task }`, but the calling code does not consistently write the updated task back into storage.

Example:

- `TaskManager.updateTaskState()` returns `result.valid` only
- it does not replace the stored task with `result.task`

Consequence:

- state transition may be "accepted" but not materially stored

### C. No deterministic gate/protocol module exists yet

The playbook assumes:

- protocol as the only transition entry point
- lock manager
- store
- gate module
- event log + snapshot

None of those implementations are present in the current package.

### D. No Rust test surface exists for the requested deterministic protocol

Since the Rust crate is absent, there is no way to validate:

- gate edge cases
- stale lease handling
- lock race recovery
- cycle detection
- GitHub evidence verification

## 5. Does cargo test pass and are the tests sufficient?

### `cargo test`

No. It does not run because there is no Rust crate in the inspected location.

### `cargo clippy`

No. Same reason.

### Test sufficiency

For the requested Rust crate: **not assessable**.

For the existing TypeScript package:

- there are tests under `Miyabi/packages/task-manager/tests/`
- but they are not Rust tests
- and they cannot establish correctness of a nonexistent Rust deterministic protocol implementation

So the direct answer to the user’s question is:

- `cargo test` does not pass because there is no Cargo project here
- test sufficiency for the requested Rust codebase cannot be evaluated because that codebase is absent

## Concrete Conclusion

This round cannot complete the requested Rust code review because the Rust codebase described in the prompt is not present in the workspace.

The strongest defensible review result is:

1. The requested Rust files do not exist in this repo/workspace.
2. `cargo test` and `cargo clippy` fail immediately due to missing `Cargo.toml`.
3. The only nearby implementation is TypeScript, and it is still pre-deterministic:
   - no gate module
   - no DAG module for this protocol
   - no lock/store/protocol modules
   - state conditions not enforced
   - state updates not reliably persisted
4. The Playbook is a plan, not an implemented Rust system in this workspace.

## Recommended Next Step

If the intended Rust codebase lives elsewhere, provide one of:

- the exact crate path
- the repository path containing `Cargo.toml`
- the branch/worktree where `src/gate.rs`, `src/protocol.rs`, etc. exist

Once that path is available, the requested full code review can be performed precisely.
