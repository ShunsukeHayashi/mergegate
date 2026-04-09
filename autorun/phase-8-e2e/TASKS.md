# Phase 8: E2E 統合テスト + OpenClaw 連携設計

> 依存: Phase 7 GREEN
> 承認ゲート: E2E テスト GREEN + OpenClaw 連携仕様書完成

## タスク

- [ ] E2E テスト（`dtp run-e2e`）:
  1. `gh issue create` → Issue #N
  2. `dtp register --issue N --title "e2e test"`
  3. `dtp dispatchable` → task が ready
  4. `dtp impact task-001 --risk LOW --symbols 2`
  5. `dtp assign task-001 --agent test --node local --files "src/test.rs"`
  6. `dtp branch task-001 feature/issue-N-e2e`
  7. `dtp pr task-001 M` (実際に PR 作成)
  8. `dtp verify-merge task-001` (実際に merge)
  9. `dtp confirm-done task-001`
  10. `dtp status task-001` → Done ✅
  11. `gh issue view N` → Closed ✅
  12. `dtp events task-001` → 全ステップの event 記録あり
- [ ] OpenClaw 連携設計書作成:
  - OpenClaw main エージェントが `dtp` CLI をどう呼ぶか
  - エージェントの SOUL.md に DTP GATE を記述
  - heartbeat を cron で自動実行する仕組み
  - 複数ノードでの tasks.snapshot.json 共有方法（git push or sshfs）
- [ ] rust-ai-pipeline 統合テスト:
  - `dtp assign` 後に `ai-pipeline phase1 --format json` を実行
  - `all_passed == false` なら implementing に留まる（GATE 4.5）
- [ ] `npx agent-skill-bus record-run` で E2E 結果を記録

## 承認ゲート

- E2E テスト GREEN
- OpenClaw 連携設計書が `docs/openclaw-integration.md` に存在
- agent-skill-bus に記録済み

## リトライ条件

- GitHub API で E2E が失敗 → mock mode で再テスト
- OpenClaw Gateway 到達不能 → Tailscale/SSH 確認
