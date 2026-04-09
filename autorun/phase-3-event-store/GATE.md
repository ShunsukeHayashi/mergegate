# Approval Gate: Phase 3

## Pass Criteria
- [ ] `cargo test` → all GREEN
- [ ] Event append → replay roundtrip test GREEN
- [ ] Snapshot save → load roundtrip test GREEN
- [ ] CAS version conflict test GREEN
- [ ] Rebuild from events test GREEN

## On Failure
- flock issues → check fs2 crate compatibility
- atomic rename fails → ensure same filesystem
- Retry (max 3)
