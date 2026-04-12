# MergeGate

[![CI](https://github.com/ShunsukeHayashi/mergegate/actions/workflows/ci.yml/badge.svg)](https://github.com/ShunsukeHayashi/mergegate/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

MergeGate is an engine-agnostic gate CLI for AI-assisted development.

It does not need to be your coding agent, your chat runtime, or your terminal UI. Its job is simpler and more durable:

- register work
- record impact
- lock files
- assign execution
- track branch and PR state
- verify completion

In short:

- `GitNexus`: understand the codebase
- `MergeGate`: execute changes safely

MergeGate is designed to sit in front of coding agents such as Claude Code, Codex, and Gemini CLI. The execution engine can change. The gate protocol should not.

## What This Project Is

MergeGate is a repo workflow tool for deterministic execution.

The core product is:

- `miyabi gate ...`
- `mergegate gate ...`

The binary still supports both `miyabi` and `mergegate` entrypoints today. `mergegate` is the product-aligned command. `miyabi` remains as a compatibility alias.

## 60-Second Setup

```bash
git clone https://github.com/ShunsukeHayashi/mergegate.git
cd mergegate
cargo build --release

./target/release/mergegate gate status
./target/release/mergegate gate init
./target/release/mergegate gate guide
```

No API key is required for gate workflow.

## Why MergeGate

- **Engine agnostic**: Use Claude Code, Codex, Gemini CLI, or another agent runtime
- **Deterministic execution**: Register, analyze, lock, implement, review, and merge in a verifiable order
- **Repo-safe coordination**: Reduce accidental overlap with file locks and explicit task ownership
- **Execution ledger**: Keep branch, PR, merge, and completion state tied to actual work
- **Protocol over chat**: The product is the workflow gate, not a built-in assistant

## Positioning

MergeGate exists because understanding code is not enough.

Agents also need a protocol for:

- when a task starts
- which files are locked
- what impact was recorded
- who is implementing
- how branch and PR state are attached
- what counts as done

That is the role split:

- `GitNexus`: query, context, impact, detect-changes, rename, wiki, graph exploration
- `MergeGate`: register, impact record, assign, file lock, branch, PR, merge, completion discipline

If GitNexus answers "what is this change connected to?", MergeGate answers "how do we carry it through safely?"

## Installation

### Prerequisites

- Rust 1.70+

### Build from Source

```bash
git clone https://github.com/ShunsukeHayashi/mergegate.git
cd mergegate
cargo build --release
```

Binary will be at `target/release/miyabi` and `target/release/mergegate`.

## Quick Start

### 1. Check repo status

```bash
./target/release/mergegate gate status
```

If you see `tasks: 0`, the ledger exists and is simply empty.

### 2. Initialize gate ledger if needed

```bash
./target/release/mergegate gate init
```

This creates `project_memory/tasks.json`.

### 3. Read the built-in workflow guide

```bash
./target/release/mergegate gate guide
```

### 4. Audit the ledger

```bash
./target/release/mergegate gate validate
./target/release/mergegate gate --format json validate
./target/release/mergegate gate export-json --state implementing --risk HIGH --since 2026-04-12
./target/release/mergegate gate export-md --state implementing --risk HIGH --since 2026-04-12
./target/release/mergegate gate --format json stats
```

### 5. Start a task

```bash
./target/release/mergegate gate register --issue 123 --title "Fix login redirect"
./target/release/mergegate gate impact issue-123 --risk medium --symbols 3
./target/release/mergegate gate assign issue-123 --agent codex --node macbook --files "src/auth.rs"
```

Typical flow:

1. `register`
2. `impact`
3. `assign`
4. implement with your coding agent of choice
5. `branch`
6. `pr`
7. `merge` or `manual-complete`

## Usage

```bash
mergegate gate status
mergegate gate init
mergegate gate guide
mergegate gate register --issue 123 --title "Fix login redirect"
mergegate gate impact issue-123 --risk medium --symbols 3
mergegate gate assign issue-123 --agent codex --node macbook --files "src/auth.rs"
mergegate gate branch issue-123 codex/fix-login-redirect
mergegate gate pr issue-123 456
mergegate gate merge issue-123 <sha>
mergegate gate validate
mergegate gate --format json validate
mergegate gate export-json --state implementing --risk HIGH --since 2026-04-12
mergegate gate export-md --state implementing --risk HIGH --since 2026-04-12
mergegate gate --format json stats
mergegate gate serve
```

`mergegate gate serve` opens the MergeGate-native web surface for:

- Gate Overview: validation severity, operator next actions, ready and attention queues
- Task Ledger: filterable state/risk/since views plus task detail drill-in
- Dependency Map: DAG levels, blocked chain visibility, and dispatchable frontier

Static asset loading follows this order:

1. `MERGEGATE_DASHBOARD_STATIC_DIR`
2. `web/dashboard/dist`
3. embedded Rust-owned dashboard fallback

For the first foundation release boundary and verification gate, see [docs/FOUNDATION_RELEASE_CHECKLIST.md](/Users/shunsukehayashi/dev/personal/HAYASHI_SHUNSUKE/mergegate/docs/FOUNDATION_RELEASE_CHECKLIST.md).

Compatibility alias:

```bash
miyabi gate status
```

## Core Commands

```bash
mergegate gate status
mergegate gate init
mergegate gate guide
mergegate gate register --issue 123 --title "Fix login redirect"
mergegate gate assign issue-123 --agent codex --node macbook --files "src/auth.rs"
mergegate gate impact issue-123 --risk medium --symbols 3
mergegate gate branch issue-123 codex/fix-login-redirect
mergegate gate pr issue-123 456
mergegate gate merge issue-123 <sha>
mergegate gate manual-complete issue-123 --reason "docs-only change" --operator shunsuke
mergegate gate validate
mergegate gate --format json validate
mergegate gate export-json --state implementing --risk HIGH --since 2026-04-12
mergegate gate export-md --state implementing --risk HIGH --since 2026-04-12
mergegate gate --format json stats
mergegate gate locks
mergegate gate dispatchable
mergegate gate dag
mergegate gate serve
```

## Product Direction

MergeGate is intentionally moving away from:

- built-in TUI as the product center
- built-in agent runtime as the product center
- vendor-specific backend identity

The durable surface is the gate CLI.

The mainline product direction is:

- Rust-first deterministic execution protocol
- CLI/API as the source of truth
- MergeGate-native UI as a supporting surface
- no multi-source PM dashboard positioning in the core product

PM dashboard assets may inform MergeGate UI/UX, but cross-source orchestration remains a separate higher-level layer.
See [docs/PRODUCT_DIRECTION.md](/Users/shunsukehayashi/dev/personal/HAYASHI_SHUNSUKE/mergegate/docs/PRODUCT_DIRECTION.md).
The first public cut should ship the foundation scope first, then land the next UI train separately. See [docs/FOUNDATION_RELEASE_CHECKLIST.md](/Users/shunsukehayashi/dev/personal/HAYASHI_SHUNSUKE/mergegate/docs/FOUNDATION_RELEASE_CHECKLIST.md).

## Workspace Layout

```text
crates/
  mergegate-cli/   # CLI entrypoint
  mergegate-core/  # Gate protocol, store, locks, workflow logic
project_memory/
  tasks.json       # Execution ledger
skills/
  mergegate-cli/
  mergegate-ops/
```

## Troubleshooting

### `gate status` says `tasks: 0`

That is not an error. It means the ledger exists but no tasks have been registered yet.

### `gate init` says the project is already initialized

That is usually safe. It means `project_memory/tasks.json` already exists.

### `gate validate` returns exit code `1` or `2`

`1` means warnings only. `2` means a real consistency problem such as orphaned locks,
invalid transitions, or circular dependencies.

### I want to use another coding agent

That is the intended model. MergeGate is the gate, not the engine.

## Free Guide: AI Development Team Playbook

Planning to introduce AI coding agents to your team? We wrote a practical guide covering:

- How to choose between Claude Code, Codex, and Gemini CLI
- 3 failure patterns that kill AI dev adoption
- Review workflow and permission design for AI agents
- How to run a low-risk 4-week PoC
- Why you need a gate protocol like MergeGate

**[Download the guide (PDF)](https://miyabi-assets.pages.dev/assets/ai-dev-team-guide.pdf)** — free, no signup required.

Want a walkthrough customized to your team? **[Chat with us on LINE](https://miyabi-line-crm.supernovasyun.workers.dev/auth/line?ref=b2b-github)** for a free 30-min consultation.
