# MergeGate
Deterministic execution protocol for AI-assisted development.

Execution layer for GitNexus-style code intelligence:
- `GitNexus`: understand the codebase
- `MergeGate`: execute changes safely

Current CLI entrypoints: `miyabi` and `mergegate`

- `miyabi tui` / `mergegate tui`: terminal assistant
- `miyabi gate` / `mergegate gate`: deterministic task execution, file locks, and PR/merge workflow

Start with `cargo build --release`, then run `./target/release/mergegate --help` or `./target/release/mergegate gate guide`.
