# L-Step / Funnel / Analytics 調査報告

_Polaris タスク: issue-85 | 調査者: claude@macbook-pro | 2026-04-10_

---

## 調査結果

### L-Step の現状

- **タグ数**: 251 個
- **友だち**: API 接続正常（最新登録: 2026-04-10 ダックス）
- **CLI**: `~/bin/lstep` 動作確認済み
- **API**: `api.lineml.jp` (L-Step Open API v2)

### ファネル構造（4段階）

```
L1: TRAFFIC (流入)
  src:xmcp-giveaway       → X プレゼント企画からの流入
  src:teachable-email      → Teachable メールからの流入

L2: CAPTURE (獲得)
  course:xmcp-started      → コース開始
  course:xmcp-completed    → コース完了

L3: NURTURE (育成)
  funnel:replied            → 返信あり
  funnel:hot-lead           → ホットリード
  funnel:day1-reached 〜 day7-reached → 日次到達
  interest:trends           → トレンド興味
  interest:competitor       → 競合興味
  interest:ai-posting       → AI投稿興味

L4: CONVERT (転換)
  offer:ppal-sent           → オファー送信済み
  offer:ppal-clicked        → オファークリック
```

### 自動化の状態

| 自動化 | 状態 | 頻度 |
|--------|------|------|
| funnel-metrics-daily.sh | launchd 設定済み | 毎朝 6:00 |
| lstep_bridge.ts | OpenClaw Gateway 転送 | Webhook 即時 |
| lstep-automation.md | ローンチシーケンス定義 | 24時間カウントダウン |

### 運用資産 (lstep-ops/)

9 カテゴリに整理済み:
01-funnel-specs, 02-tag-maps, 03-runbooks, 04-segments,
05-campaigns, 06-automations, 07-analytics, 08-templates, 09-scripts

### 改善点

1. **funnel-metrics-daily.sh のログがない**
   → tmp/funnel-metrics-daily.log が空。cron が正常に動いているか要確認。

2. **ファネル KPI の Google Sheets が手動確認**
   → Polaris のダッシュボード (miyabi gate serve) にファネル KPI を統合できると便利。

3. **L-Step と Polaris の接続がない**
   → L-Step のタグ変更を Polaris のイベントとして記録すれば、
     dream でマーケティングの学びも自動抽出できる。

4. **セグメント別のコンバージョン率が未計算**
   → L2→L3、L3→L4 の遷移率を自動計算して Sheets に追記すべき。

5. **lstep_bridge.ts → OpenClaw の転送が片方向**
   → OpenClaw から L-Step への逆方向（タグ自動付与等）が未実装。
