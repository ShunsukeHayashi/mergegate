# Deterministic Task Protocol — Auto Run Index

> 全 Phase の依存 DAG + 承認ゲート + リトライループ + アサインメント

## Phase DAG

```
Phase 0 (前提条件)
    │
    ▼
Phase 1 (型ハードニング)
    │
    ├────────────┬────────────┐
    ▼            ▼            │
Phase 2       Phase 3        │
(State+GATE)  (Event Store)  │
    │            │            │
    │            ▼            │
    │         Phase 4         │
    │         (File Lock)     │
    │            │            │
    ├────────────┘            │
    ▼                         │
Phase 5 (GitHub Sync) ◄──────┘
    │
    ▼
Phase 6 (Protocol 統合) ← 核心
    │
    ▼
Phase 7 (CLI)
    │
    ▼
Phase 8 (E2E + OpenClaw)
```

## Phase 別アサインメント

| Phase | ファイル | 推奨エージェント | 並列可否 | 承認ゲート |
|-------|---------|---------------|---------|-----------|
| 0 | `00-phase0-preconditions.md` | Claude Code (local) | — | cargo test + clippy GREEN |
| 1 | `01-phase1-harden-types.md` | Codex A (worktree) | — | cargo test GREEN + 5 新規テスト |
| 2 | `02-phase2-state-machine-gates.md` | Codex B (worktree) | 3 と並列 OK | 不正遷移テスト 5件 GREEN |
| 3 | `03-phase3-event-store.md` | Codex C (worktree) | 2 と並列 OK | CAS + rebuild テスト GREEN |
| 4 | `04-phase4-file-lock-lease.md` | Codex A (worktree) | 3 完了後 | lease lifecycle テスト GREEN |
| 5 | `05-phase5-github-sync.md` | Claude Code (local) | 3 完了後 | mock API テスト GREEN |
| 6 | `06-phase6-protocol-integration.md` | Claude Code (local) | 2+3+4+5 完了後 | E2E + GATE 拒否 20件 GREEN |
| 7 | `07-phase7-cli.md` | Codex B (worktree) | 6 完了後 | CLI 動作確認 |
| 8 | `08-phase8-e2e-openclaw.md` | Claude Code (local) | 7 完了後 | E2E GREEN + OpenClaw 設計書 |

## 承認フロー

```
Phase N 実行
    │
    ▼
cargo test + cargo clippy
    │
    ├─ GREEN → 承認ゲート通過
    │           │
    │           ▼
    │         Phase N の全チェックボックス確認
    │           │
    │           ├─ 全て ✅ → Phase N+1 へ
    │           └─ 未完了あり → 追加作業してリトライ
    │
    └─ FAIL → リトライ
               │
               ├─ コンパイルエラー → 該当コード修正 → 再ビルド
               ├─ テスト失敗 → 原因特定 → 修正 → 再テスト
               ├─ clippy 警告 → 該当コード修正 → 再 clippy
               └─ 3回連続失敗 → エスカレーション（人間判断）
```

## Codex ディスパッチコマンド

```bash
# Phase 1: Codex A に型ハードニングをアサイン
tmux send-keys -t %9 "codex 'Execute Phase 1 from docs/autorun/01-phase1-harden-types.md. Read the file, implement ALL checkboxes, run cargo test, run cargo clippy. Do NOT proceed to Phase 2.'" Enter

# Phase 2: Codex B にステートマシンをアサイン（Phase 1 完了後）
tmux send-keys -t %10 "codex 'Execute Phase 2 from docs/autorun/02-phase2-state-machine-gates.md. Read the file, implement ALL checkboxes, run cargo test. Do NOT proceed to Phase 3.'" Enter

# Phase 3: Codex C に Event Store をアサイン（Phase 1 完了後、Phase 2 と並列）
tmux send-keys -t %11 "codex 'Execute Phase 3 from docs/autorun/03-phase3-event-store.md. Read the file, implement ALL checkboxes, run cargo test. Do NOT proceed to Phase 4.'" Enter
```

## リトライループ

各 Phase は以下のループで実行:

```
ATTEMPT = 1
MAX_ATTEMPTS = 3

loop:
    execute(Phase N)
    result = cargo test + cargo clippy
    
    if result == GREEN and all_checkboxes_done:
        APPROVE → Phase N+1
        break
    
    if ATTEMPT >= MAX_ATTEMPTS:
        ESCALATE → 人間に報告、Phase N を手動レビュー
        break
    
    ATTEMPT += 1
    diagnose(failure)
    fix(failure)
    goto loop
```
