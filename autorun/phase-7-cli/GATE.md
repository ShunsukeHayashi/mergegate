# Approval Gate: Phase 7

## Pass Criteria
- [ ] `cargo build` → dtp binary compiles
- [ ] `dtp register --issue 1 --title test` → creates task
- [ ] `dtp status` → JSON output with task list
- [ ] `dtp --help` → all subcommands listed
- [ ] Exit codes: 0=success, 1=gate-rejected, 2=input-error

## On Failure
- clap derive issues → check feature flags
- Retry (max 3)
