# Product Direction — MergeGate

_Last Updated: 2026-04-12_

## Core Definition

MergeGate is a **Rust-first deterministic execution protocol product**.

Its core value is not chat, a built-in agent runtime, or a general-purpose PM dashboard.
Its core value is a durable execution gate that can:

- register work
- record impact
- lock files
- decide what is dispatchable
- attach branch / PR / merge evidence to execution
- validate and audit the ledger

The source of truth for this product is:

- Rust core types and protocol behavior
- `mergegate gate ...` CLI
- MergeGate API surfaces exposed by `mergegate gate serve`

## What MergeGate Is

MergeGate is the product that answers:

- what can safely start now
- what is blocked
- which files are owned
- what evidence is attached to the task
- whether the ledger is internally consistent

The durable surface is:

- `gate CLI`
- `ledger`
- `validate`
- `dispatchable`
- `lock`
- `dag`
- `stats`
- `audit/export`

## What MergeGate Is Not

MergeGate is **not** the product center for:

- a built-in TUI runtime
- a built-in chat runtime
- a vendor-specific coding agent
- a cross-source PM dashboard
- portfolio or organization-wide operations analytics

Those surfaces may exist around MergeGate, but they are not the main product.

## UI Policy

UI is important, but it is a **supporting surface** for the protocol.

The official MergeGate UI should:

- visualize MergeGate-native concepts first
- use Rust/API results as the single source of truth
- minimize frontend-only business logic
- help operators decide what to dispatch, unblock, validate, and review

The official first-class UI surfaces are:

- Gate Overview
- Task Ledger
- Dependency Map

`Project Flow` and wider PM rollups are secondary and should not redefine the product.

## PM Dashboard Boundary

PM dashboard assets are valuable, but only some belong inside MergeGate.

Safe to absorb:

- information architecture such as `dispatchable first`, `alerts first`, and `dependency pressure`
- visual patterns and reusable UI components
- task triage and DAG presentation ideas

Do not absorb into MergeGate core product definition:

- multi-source federation across `personal / maestro / openclaw / github / skillbus`
- `UnifiedTask` as the product-wide task model
- Next.js server runtime as a runtime requirement
- portfolio-first or PM-first positioning

Cross-source orchestration belongs in a higher-level PM dashboard or ops cockpit, where MergeGate is one input among others.

## Architecture Boundary

MergeGate:

- gate engine
- ledger
- CLI
- API
- dedicated MergeGate UI

Higher-level PM dashboard:

- cross-source orchestration
- portfolio views
- organization or personal ops analytics
- source federation and comparison

The two layers should cooperate, but they should not be collapsed into one product.

## Decision Rule

When evaluating roadmap items, use this filter:

1. Does this make the Rust protocol stronger, safer, or easier to operate?
2. Does this improve a MergeGate-native UI surface without changing the product boundary?
3. If not, should it live in the higher-level PM dashboard instead?

If a proposal does not strengthen MergeGate as a protocol product, it should not become part of the mainline by default.
