# Repository Guidelines

## Project Structure & Module Organization
- Rust workspace lives under `crates/`: `miyabi-cli` (CLI entry), `miyabi-tui` (UI), and `miyabi-core` (shared logic, config, sessions). Shared workspace config in `Cargo.toml`.
- TypeScript side sits in `src/` with `index.ts` as the entry point; Vitest specs live in `tests/`. Build artifacts output to `dist/`.
- Supporting assets: `.claude/` agent configs, `docs/` for design notes, `.miyabi/` for local runtime data, and `.github/` for CI workflows.

## Build, Test, and Development Commands
- Rust: `cargo build --workspace --release` (release build), `cargo test --all` (unit/integration), `cargo clippy --all-targets -- -D warnings` (lint), `cargo fmt --all` (format).
- TypeScript: `npm run dev` (tsx watch), `npm run build` (tsc emit to `dist/`), `npm run typecheck` (tsc no emit), `npm run lint` (eslint per `.eslintrc.json`), `npm test` (vitest).
- Install toolchains: `npm install` for JS deps; Rust uses stable 1.75+ per workspace metadata.

## Coding Style & Naming Conventions
- Rust: run `cargo fmt` before commits; clippy must be clean with `-D warnings`. Modules/files use `snake_case`, types/traits `PascalCase`, constants `SCREAMING_SNAKE_CASE`. Prefer `anyhow::Result` and `thiserror` for errors; avoid `unwrap` in non-test code.
- TypeScript: strict mode on; avoid `any` (warned), silence unused via `_` prefix. Follow ESLint rules and keep imports sorted logically. Prefer async/await over callbacks.

## Testing Guidelines
- Rust tests colocated in each crate (`mod tests` blocks or `tests/` directories); write integration tests when touching cross-crate behavior. Run `cargo test --all` before PRs.
- TypeScript tests follow `*.test.ts` in `tests/`; structure with `describe/it` and keep fast. Aim to cover error paths and CLI flags.

## Commit & Pull Request Guidelines
- Use Conventional Commits (`feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`). Keep subject under ~72 chars; include scope when helpful (`feat(tui): add diff renderer`).
- PRs should describe the change, linked issue, and test evidence (`cargo test --all`, `npm test`, etc.). Add screenshots or terminal recordings for TUI changes when relevant. Keep diffs focused; separate refactors from feature work.

## Environment & Configuration
- Environment defaults from `.env.example`; typical variables: `GITHUB_TOKEN`, `ANTHROPIC_API_KEY`, `REPOSITORY`, optional `RUST_LOG`/`RUST_BACKTRACE` for debugging.
- Local config/state lives in `~/.miyabi/` and `.miyabi/` (do not commit secrets). Review `.miyabi.yml` for runtime behavior before modifying agent workflows.
