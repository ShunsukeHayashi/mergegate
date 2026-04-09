# Next Steps — DTP 実装後のロードマップ

_Phase A/B 完了時点での計画。clippy 修正 + Phase C 完了後に更新。_

---

## 直近（今日〜明日）

### 1. clippy 修正 → Phase C 完了

```
clippy 7件修正 (10分) → Phase C: GitHub Evidence + E2E (10分)
合計: 20分
```

Phase C でやること:
- github.rs の既存 `get_pull_request()` を使って merge 証跡取得
- protocol.rs に `verify_merge()` 追加
- escape hatch: `force_unlock`, `manual_complete`
- E2E テスト: register → ... → done

### 2. rust-ai-pipeline 品質ゲート統合

`ai-pipeline phase1` を DTP の GATE 4.5 として組み込む:
- `miyabi gate assign` 後に `ai-pipeline phase1 --project . --format json` を実行
- `all_passed == false` なら implementing に留まる
- `failure_kind` を tasks.json に記録

### 3. GNI 再インデックス + 最終インパクト確認

Phase C 完了後に `npx gitnexus analyze --force` して全体の影響を最終確認。

---

## 短期（今週）

### 4. OpenClaw ドッキング実装

`docs/dtp/OPENCLAW-DOCKING-PLAN.md` に基づく:

| # | 項目 | 工数 |
|---|------|------|
| 4a | `miyabi gate` の JSON 出力を OpenClaw hooks でキャッチ | 30分 |
| 4b | OpenClaw main が `miyabi gate dispatchable` でタスク取得 | 30分 |
| 4c | `sessions spawn` でサブエージェントに配布 | 30分 |
| 4d | `project_memory/tasks.json` を memory sync 対象に追加 | 15分 |
| 4e | E2E: OpenClaw main → miyabi gate → カエデ実装 → サクラレビュー → merge | 1時間 |

### 5. Maestro Playbook 登録

Phase A/B/C の autorun を Maestro Auto Run 形式に変換:
- チェックボックス形式のマークダウン
- Maestro GUI から実行可能に
- Session Isolation + Worktree Support 有効化

### 6. miyabi CLI (npm) との統合

`npx miyabi` (TypeScript版) と `miyabi` (Rust版) の共存:
- `npx miyabi bus` → 既存のキュー管理
- `miyabi gate` → DTP のGATE管理
- 将来的に `npx miyabi` の `task` と `bus` を Rust 版に移行

---

## 中期（今月）

### 7. Heartbeat デーモン化

- `miyabi gate heartbeat <task-id>` を cron で定期実行
- launchd plist で Mac 起動時に自動スタート
- lease 300秒 / heartbeat 60秒 / stale 2回ミスで解放

### 8. tasks.json の git push 自動同期

- Phase 完了時に自動 `git add project_memory/tasks.json && git commit && git push`
- 他ノードで `git pull` → tasks.json 更新 → ロック状態が共有される
- 衝突時は CAS version で検出

### 9. Telegram 通知統合

- GATE 通過/拒否を Telegram に通知
- HIGH/CRITICAL の人間承認を Telegram ボタンで
- Phase 完了報告を Telegram + VOICEBOX 両方に

### 10. GitHub Actions CI 追加（任意）

現在はローカル完結だが、PR レビュー時の自動チェックとして:
- `cargo test + cargo clippy` を GitHub Actions で実行
- 品質ゲートは Rust コンパイラが担保するので CI は補助

---

## 長期（来月以降）

### 11. rust-ai-pipeline Phase 2〜5 統合

品質ゲートの多層化:
- Phase 2: マルチエージェント分離（テスト作成者 ≠ 実装者）
- Phase 3: Miri / Verus / Kani（静的検証）
- Phase 4: proptest + cargo-fuzz（動的検証）
- Phase 5: cargo-mutants（メタ検証）

### 12. miyabi-private (TypeScript) からの完全移行

miyabi-private の task-manager (TypeScript) → miyabi-core (Rust) への段階的移行:
- BidirectionalSync → Rust 再実装
- GitHub Label Sync → github.rs に統合
- Projects V2 Sync → github_tools.rs に統合

### 13. OpenClaw Framework 公開

DTP を OpenClaw のプラグインとして公開:
- `openclaw plugin install miyabi-dtp`
- 任意の OpenClaw 環境で使える確定的タスク実行基盤
- ドキュメント: how-to-dock-dtp-with-openclaw.md

---

## 優先度マトリックス

```
          緊急
           │
     ┌─────┼─────┐
     │  1  │  4  │
重要 ─  C+  │  OC │ ← 今週
     │clippy│dock │
     ├─────┼─────┤
     │  7  │  10 │
     │heart│  CI │ ← 来週以降
     │beat │     │
     └─────┼─────┘
           │
         非緊急
```

---

_Phase C 完了後に再評価して優先度を更新する。_
