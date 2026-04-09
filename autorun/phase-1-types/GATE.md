# Approval Gate: Phase 1

## Pass Criteria
- [ ] `cargo test` → all GREEN
- [ ] `cargo clippy -- -D warnings` → 0 warnings
- [ ] At least 5 new tests added for new types
- [ ] All Codex R1/R2/R3 type changes reflected

## On Failure
- Compile error → fix types, check ManagedTask::new() defaults
- Test failure → fix assertions
- Retry (max 3)
