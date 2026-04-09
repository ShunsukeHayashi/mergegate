# Approval Gate: Phase 5

## Pass Criteria
- [ ] `cargo test` → all GREEN
- [ ] MockFetcher: PR merged → evidence → Merged transition GREEN
- [ ] MockFetcher: API down → AwaitingGithubSync GREEN
- [ ] MockFetcher: Issue manual close → NOT done GREEN
- [ ] GitHubEvidenceFetcher trait defined with mock + real impl

## On Failure
- gh CLI not found → fall back to HTTP
- Rate limit → exponential backoff
- Retry (max 3)
