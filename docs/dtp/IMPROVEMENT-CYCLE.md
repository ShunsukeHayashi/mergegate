# 改善サイクル — シータサイクル分析に基づく詳細プラン

_MergeGate タスク: issue-86 | 2026-04-10_

---

## θ分析結果

| 指標 | 値 | 判定 |
|------|-----|------|
| dream イベント | 39件 | ⚠ 少ない（実運用データ不足） |
| gate 拒否 | 0件 | ← 実タスクがまだ少ない |
| lock 競合 | 0件 | ← 並列実行の実績がない |
| agent-skill-bus | declining (44 runs) | ⚠ 劣化傾向 |
| codex-cli-usage-rules | 0.95→0.70 | 🔴 大幅ドリフト |
| Bus queue | 51件中49件未処理 | 🔴 滞留 |

---

## 改善サイクル（7日間）

### Day 1-2: テスト補強 + hooks 検証

**目的**: lock.rs と store.rs のテスト不足を解消し、hooks が実際に機能するか検証

| タスク | Issue | 担当 | 完了条件 |
|--------|-------|------|---------|
| lock.rs テスト追加（acquire/release/conflict 組み合わせ） | 新規 | Codex | 6 pub fn に対して 6+ テスト |
| store.rs テスト追加（EventStore/SnapshotStore 境界値） | 新規 | Codex | 16 pub fn に対して 10+ テスト |
| Claude Code hooks ブロック検証（実際に Edit をブロック） | 新規 | Claude Code | ロック外ファイル編集が拒否されることを動画で記録 |
| pre-commit hook 検証（ロック外 commit が拒否） | — | Claude Code | POLARIS_AGENT_ID 設定下で拒否確認 |

### Day 3-4: attach_context 精度向上

**目的**: アタッチされる情報の質を上げる

| タスク | Issue | 担当 | 完了条件 |
|--------|-------|------|---------|
| Obsidian Vault 検索の実効性テスト（OBSIDIAN_VAULT_PATH 設定） | #77 | Claude Code | 3件以上の関連ノートがアタッチされる |
| attach のファイル抜粋を GNI impact 結果と連動 | 新規 | Codex | depth=1 ファイルも自動アタッチ |
| attach --format json の出力をエージェントのプロンプトに注入するスクリプト | 新規 | Claude Code | Codex ディスパッチ時に自動注入 |

### Day 5: Bus 滞留解消 + Codex ルール遵守率改善

**目的**: 49件の未処理キューを消化し、codex-cli-usage-rules のドリフトを修正

| タスク | Issue | 担当 | 完了条件 |
|--------|-------|------|---------|
| Bus の古い queued タスクをトリアージ（不要なら archive） | 新規 | Claude Code | queued < 10 |
| codex-cli-usage-rules のスキルを更新（miyabi-gate 必須を強化） | — | Claude Code | drift スコア 0.70→0.90 |
| npx miyabi bus dispatch → miyabi gate register の自動連携テスト | #61 | Claude Code | dispatch したタスクが tasks.json に登録される |

### Day 6: 実運用（全タスクを Polaris 経由）

**目的**: 1日の全作業を Polaris で回す

| タスク | 担当 | 完了条件 |
|--------|------|---------|
| 朝: miyabi gate status で前日の状態確認 | Claude Code | ダッシュボード表示 |
| 全タスク: register → impact → assign → 作業 → complete | 全エージェント | 5タスク以上を完走 |
| 夕: miyabi gate dream --auto で学び抽出 | 自動 (launchd) | DreamReport にパターンが出る |
| 夕: skill-bus record-run で結果記録 | 自動 | skill-runs.jsonl に追記 |

### Day 7: レビュー + 次サイクル計画

**目的**: 1週間の実運用結果をレビューし、次の改善点を特定

| タスク | 担当 | 完了条件 |
|--------|------|---------|
| dream レポートの確認（gate 拒否パターン、lock 競合、完了時間） | Claude Code | レポート作成 |
| ビジョン達成度の再計測 | Claude Code | 80% → 目標 90% |
| 次サイクルの Issue 作成 | Claude Code | 改善 Issue 3件以上 |
| skill-bus health 確認 | Claude Code | declining スキルの改善 |

---

## サイクル DAG

```
Day 1-2: テスト補強 + hooks 検証
    │
    ▼
Day 3-4: attach 精度向上
    │
    ▼
Day 5: Bus 滞留解消 + Codex ルール改善
    │
    ▼
Day 6: 実運用（全タスク Polaris 経由）
    │
    ▼
Day 7: レビュー + 次サイクル計画
    │
    ▼
Day 8〜: 次の改善サイクル
```

---

## 承認ゲート

| Day | ゲート条件 |
|-----|-----------|
| Day 2 完了 | lock.rs 6+ テスト、store.rs 10+ テスト、hooks ブロック確認済み |
| Day 4 完了 | Obsidian 3件アタッチ、GNI 連動アタッチ動作 |
| Day 5 完了 | Bus queued < 10、codex drift 0.90+ |
| Day 6 完了 | 5タスク以上 Polaris 完走 |
| Day 7 完了 | ビジョン達成度 85%+、次サイクル Issue 3件 |

---

## シータサイクル統合

このプランは毎週自動で回す:

```
θ1: miyabi gate dream → パターン抽出
θ2: npx miyabi bus health → スキル健全性
θ3: npx miyabi bus drift → ドリフト検出
θ4: 改善 Issue 自動作成（dream の学びから）
θ5: skill-bus record-run → 実行記録
θ6: SKILL.md 自動更新（drift 修正）
```

launchd:
- 毎日 03:00: `miyabi gate dream --auto`
- 毎日 06:00: `funnel-metrics-daily.sh`
- 毎週月曜 09:00: `npx miyabi bus health + drift`（手動 or cron 追加）
