# MergeGate
Engine-agnostic gate CLI for AI-assisted development.

Execution layer for GitNexus-style code intelligence:
- `GitNexus`: understand the codebase
- `MergeGate`: execute changes safely

Core product:
- `mergegate gate ...`
- `miyabi gate ...` (compatibility alias)

MergeGate is designed to sit in front of Claude Code, Codex, Gemini CLI, and other coding agents. The engine can change. The gate protocol should not.

Start with:

```bash
cargo build --release
./target/release/mergegate gate status
./target/release/mergegate gate init
./target/release/mergegate gate guide
```

