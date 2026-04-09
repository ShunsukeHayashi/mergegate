# Approval Gate: Phase 0

## Pass Criteria (ALL must be true)
- [ ] `cargo build` → exit 0
- [ ] `cargo test` → all GREEN
- [ ] `cargo clippy --all-targets -- -D warnings` → 0 warnings
- [ ] `ls -la refs/` → no broken symlinks

## On Failure
- Fix the failing check
- Retry (max 3 attempts)
- Escalate to human after 3 failures
