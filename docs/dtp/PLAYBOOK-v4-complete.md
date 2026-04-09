# DTP Playbook v4 — ビジョン完全準拠版

_v4: 原則 1（記憶）+ Bus ドッキング + ブランチ戦略を統合。全 38 要素をカバー。_

---

## North Star

> 記憶はアタッチメント。ジグで手順を強制。GitHub で事実を確定。

---

## 完了済み

| 内容 | コミット | 行数 |
|------|---------|------|
| Phase A: gate + lock + store + protocol | 986d907 | +1,485 |
| Phase B: CLI サブコマンド | 273c416 | +1,171 |
| clippy 修正 | 437b959 | 0 (既に反映済み) |

---

## Sprint 1: GATE 修正 + Phase C（今日）

> ビジョン対応: 原則 2 (ジグ) を 58% → 90% に

| # | Issue | タスク | 行数 |
|---|-------|--------|------|
| 1 | #52 | GATE 0: issue=0 拒否 | ~5 |
| 2 | #53 | GATE 5: ブランチ名バリデーション | ~10 |
| 3 | #54 | GATE 3: HIGH risk 承認チェック | ~20 |
| 4 | #55 | GATE 7: merge 後ロック自動解放 + 後続タスク解放 | ~15 |
| 5 | #56 | exit code: GATE 拒否=1, 入力エラー=2 | ~10 |
| 6 | #57 | 依存ブロック assign → exit 1 | ~10 |
| 7 | — | verify_merge: GitHub API で PR merged を検証 | ~50 |
| 8 | — | escape hatch: force_unlock + manual_complete | ~50 |
| 9 | — | E2E テスト: register → done 全シーケンス | ~100 |

**GATE**: cargo test GREEN + clippy ZERO + E2E テスト GREEN
**タグ**: `v1.0-dtp-complete`

---

## Sprint 2: Bus ドッキング + ブランチ戦略（今週）

> ビジョン対応: 原則 3 (SSOT) を 44% → 80% に

| # | Issue | タスク | 行数 |
|---|-------|--------|------|
| 1 | #61 | Bus ドッキング: register → auto enqueue | ~30 |
| 2 | #61 | Bus ドッキング: merge → auto complete | ~20 |
| 3 | #65 | Bus データパス統合 (HAYASHI_SHUNSUKE → 参照) | ~10 |
| 4 | #64 | 並列 Codex ブランチ戦略: assign 時に自動 branch + worktree | ~50 |
| 5 | — | JSON 出力スキーマ統一 | ~30 |
| 6 | — | OpenClaw hooks 連携 | ~50 |
| 7 | — | OpenClaw main 呼び出しテスト | テスト |
| 8 | — | tasks.json memory sync | ~30 |

**GATE**: Bus enqueue/complete 動作 + OpenClaw main が register→merge 完走
**タグ**: `v1.1-bus-docked`

---

## Sprint 3: 記憶アタッチメント + ドリーミング（来週）

> ビジョン対応: 原則 1 (記憶) を 0% → 70% に ← **核心**

| # | Issue | タスク | 行数 |
|---|-------|--------|------|
| 1 | #58 | attach_context: Issue 本文取得 | ~20 |
| 2 | #58 | attach_context: GNI impact 結果をアタッチ | ~20 |
| 3 | #58 | attach_context: ロック対象ファイルの先頭 50 行抜粋 | ~20 |
| 4 | #58 | attach_context: トークン閾値で切り詰め | ~20 |
| 5 | #58 | tasks.json に context_attachments フィールド追加 | ~15 |
| 6 | #58 | CLI: assign 時に attach_context 自動実行 | ~10 |
| 7 | #58 | CLI: --format json で attachments を出力 | ~10 |
| 8 | #59 | dream: event log replay | ~30 |
| 9 | #59 | dream: パターン抽出 (GATE 拒否頻度、ロック競合) | ~40 |
| 10 | #59 | dream: 学び重要度判定 (High/Medium/Low) | ~20 |
| 11 | #59 | dream: High → docs/ にコミット | ~20 |
| 12 | #59 | dream: Medium → Issue コメントに追記 | ~15 |
| 13 | #59 | dream: Low → アーカイブ | ~10 |
| 14 | #59 | CLI: miyabi gate dream [--since 24h] [--auto] | ~20 |
| 15 | #59 | launchd: 毎晩 03:00 に dream --auto 実行 | plist |
| 16 | #63 | Web ダッシュボード: miyabi gate serve (localhost:4848) | ~100 |
| 17 | — | Heartbeat デーモン (launchd 60秒) | ~50 |
| 18 | — | Telegram 通知 (GATE 通過/拒否) | ~50 |
| 19 | — | VOICEBOX 自動化 (hooks.yaml) | ~20 |
| 20 | — | git 自動同期 (merge 後に auto push) | ~30 |
| 21 | — | Maestro Playbook 登録 | 設定 |

**GATE**: attach_context が動作 + dream が docs/ に昇格 + Web ダッシュボード表示
**タグ**: `v1.2-memory-attached`

---

## Sprint 4: シータサイクル + Obsidian + 品質ゲート（今月）

> ビジョン対応: 原則 1 を 70% → 100% + 自己改善 0% → 100%

| # | Issue | タスク | 行数 |
|---|-------|--------|------|
| 1 | #60 | シータ θ1: dream に Knowledge Watcher 差分検知を統合 | ~30 |
| 2 | #60 | シータ θ5: skill-bus record-run を dream 内で自動呼出 | ~10 |
| 3 | #60 | シータ θ6: Self-Improving → SKILL.md 自動更新 | ~30 |
| 4 | #60 | シータ θ6: attach_context の選別ルール自動改善 | ~20 |
| 5 | #62 | Obsidian: dream → Vault ノート生成 | ~50 |
| 6 | #62 | Obsidian: wikilink 自動生成 | ~30 |
| 7 | #62 | Obsidian: MOC 自動更新 | ~20 |
| 8 | #62 | Obsidian: Smart Connections 埋め込み再生成トリガー | ~10 |
| 9 | #62 | attach_context: Smart Connections セマンティック検索追加 | ~30 |
| 10 | #62 | attach_context: wikilink 追跡で関連ノート展開 | ~20 |
| 11 | — | rust-ai-pipeline Phase 1 統合 (GATE 4.5) | ~50 |
| 12 | — | proptest 拡張 (gate/lock/store) | ~100 |
| 13 | — | cargo-mutants (mutation score 80%+) | 設定 |

**GATE**: シータが自律巡回 + Obsidian にノート生成 + mutation 80%+
**タグ**: `v1.3-theta-obsidian`

---

## Sprint 5: 移行 + 公開（来月以降）

| # | タスク |
|---|--------|
| 1 | miyabi-private (TS) → Rust 段階的移行 |
| 2 | OpenClaw プラグインとして公開 |
| 3 | npm パッケージ配布 |

**タグ**: `v2.0-public`

---

## Sprint DAG

```
Sprint 1 (#52-57 + Phase C)  ← 今日。GATE を固める
    │
    ▼
Sprint 2 (#61,64,65)         ← 今週。Bus接続 + ブランチ戦略
    │
    ▼
Sprint 3 (#58,59,63)         ← 来週。★記憶アタッチメント + ドリーミング★
    │
    ▼
Sprint 4 (#60,62)            ← 今月。シータ + Obsidian + 品質ゲート
    │
    ▼
Sprint 5                     ← 来月。公開
```

---

## ビジョン達成度の推移

```
          原則1  原則2  原則3  可視化  自己改善  全体
現在:       0%   58%   44%    13%     0%      32%
Sprint1後: 0%   90%   50%    13%     0%      38%
Sprint2後: 0%   95%   80%    13%     0%      47%
Sprint3後: 70%  95%   85%    60%    10%      72%
Sprint4後: 100% 100% 90%    70%    100%     93%
Sprint5後: 100% 100% 100%   80%    100%     97%
```

---

## 全 Issue → Sprint マッピング

| Issue | Sprint | 原則 |
|-------|--------|------|
| #52 GATE 0 Issue=0 | 1 | 原則2 |
| #53 GATE 5 ブランチ名 | 1 | 原則2 |
| #54 GATE 3 HIGH承認 | 1 | 原則2 |
| #55 GATE 7 ロック解放 | 1 | 原則2 |
| #56 exit code | 1 | 原則2 |
| #57 依存 exit code | 1 | 原則2 |
| #58 記憶アタッチメント | 3 | **原則1** |
| #59 ドリーミング | 3 | **原則1** |
| #60 シータサイクル | 4 | 自己改善 |
| #61 Bus ドッキング | 2 | 原則3 |
| #62 Obsidian 連携 | 4 | **原則1** |
| #63 Web ダッシュボード | 3 | 可視化 |
| #64 ブランチ戦略 | 2 | 原則2 |
| #65 Bus データパス | 2 | 原則3 |

---

_This is Playbook v4. It covers all 38 elements of the VISION._
_Previous versions (v1-v3) are archived in docs/dtp/._
