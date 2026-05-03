# Foundation Release Checklist

This checklist defines the release boundary for the first MergeGate foundation release.

## Scope

This release intentionally ships the Rust-owned foundation only:

- `gate validate`
- `gate export-json`
- `gate export-md`
- `gate stats`
- `gate serve`
- Rust-owned dashboard APIs and embedded fallback UI

This release does not include the next UI train:

- `issue-118` Gate Overview refinement
- `issue-119` Task Ledger refinement
- `issue-120-ui-dependency-map` Dependency Map refinement

## Release Files

Release commits may include:

- `README.md`
- `crates/mergegate-cli/src/dashboard_embedded.html`
- `crates/mergegate-cli/src/main.rs`
- `crates/mergegate-core/src/dashboard.rs`
- `crates/mergegate-core/src/export.rs`
- `crates/mergegate-core/src/export_md.rs`
- `crates/mergegate-core/src/lib.rs`
- `crates/mergegate-core/src/stats.rs`
- `crates/mergegate-core/src/validate.rs`
- `docs/PRODUCT_DIRECTION.md`
- `docs/PRODUCT_SPEC.md`
- `docs/USER_MANUAL.md`
- `docs/dtp/NEXT-STEPS.md`
- this checklist

Release commits must not include operational drift such as:

- `AGENTS.md`
- `CLAUDE.md`
- `project_memory/*`
- `skills/self-improving-skills/skill-runs.jsonl`
- unrelated local test-only edits such as `crates/mergegate-core/src/github_tools.rs`

## Verification Gate

Run all checks from a clean worktree:

```bash
cargo build --workspace --release
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
./target/release/mergegate gate status
./target/release/mergegate gate validate
./target/release/mergegate gate export-json --state implementing --risk HIGH --since 2026-04-12
./target/release/mergegate gate export-md --state implementing --risk HIGH --since 2026-04-12
./target/release/mergegate gate --format json stats
```

Smoke-check `gate serve` before tagging:

1. Start `./target/release/mergegate gate serve --port 4850`
2. Confirm `/`, `/api/tasks`, `/api/stats`, and `/api/validate` return successfully
3. Stop the server cleanly

## Release Decision

Cut the foundation release once all of the following are true:

- `issue-117` is complete as foundation scope
- the worktree is clean except for intentional release files
- the verification gate passes
- the release commit is reproducible from the checked-in source

After that point, tag the foundation release and move `issue-118`, `issue-119`, and `issue-120-ui-dependency-map` into the next UI release train.
