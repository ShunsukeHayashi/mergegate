# Approval Gate: Phase 2

## Pass Criteria
- [ ] `cargo test` → all GREEN
- [ ] At least 5 invalid transition tests that correctly reject
- [ ] `can_transition()` takes `&ManagedTask` (not just from/to)
- [ ] `Merged` and `AwaitingGithubSync` states have transition rules

## On Failure
- Protocol tests break → update protocol.rs to use new API
- Retry (max 3)
