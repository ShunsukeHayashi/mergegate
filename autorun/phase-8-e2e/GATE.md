# Approval Gate: Phase 8

## Pass Criteria
- [ ] E2E: gh issue create → dtp register → ... → dtp confirm-done → Issue Closed
- [ ] OpenClaw integration design doc exists at docs/openclaw-integration.md
- [ ] agent-skill-bus record-run recorded
- [ ] All events in task-events.jsonl for the E2E run

## On Failure
- GitHub API failure → run with mock mode
- OpenClaw unreachable → document as known limitation
- Retry (max 3), then human sign-off
