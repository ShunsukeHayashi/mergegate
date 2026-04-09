# Approval Gate: Phase 6

## Pass Criteria
- [ ] `cargo test` → all GREEN (20+ tests)
- [ ] E2E happy path: draft → done test GREEN
- [ ] GATE rejection tests: 8+ tests, all correctly reject
- [ ] Escape hatch tests: force_unlock, manual_complete GREEN
- [ ] All operations emit events to EventStore
- [ ] Snapshot persists after every state change

## On Failure
- Component integration mismatch → check trait bounds
- Event ordering → verify append is sequential
- Retry (max 3), then escalate
