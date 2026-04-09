# Approval Gate: Phase 4

## Pass Criteria
- [ ] `cargo test` → all GREEN
- [ ] Lease acquire → heartbeat → release lifecycle test GREEN
- [ ] Stale detection test GREEN (no heartbeat → expired)
- [ ] Conflict test GREEN (2 tasks same file → reject)
- [ ] proptest for lock invariants GREEN

## On Failure
- Timing issues → use fixed timestamps in tests
- Retry (max 3)
