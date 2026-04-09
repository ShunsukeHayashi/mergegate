# Sprint 2: OpenClaw ドッキング（今週）

> Sprint 1 の v1.0 タグ完了後に開始

## 2.1 JSON 出力の標準化 (~30行)

- [ ] 全コマンドの JSON 出力スキーマを統一
  ```json
  { "status": "ok"|"gate_rejected"|"error", "exit_code": 0|1|2, "task_id": "...", "data": {} }
  ```
- [ ] テスト: 各コマンドの JSON が上記スキーマに準拠

## 2.2 OpenClaw hooks 連携 (~100行)

- [ ] `miyabi gate register` 完了時に stdout に JSON イベント出力
- [ ] `miyabi gate merge` 完了時に stdout に JSON イベント出力
- [ ] OpenClaw hooks 設定に DTP イベントテンプレート追加
- [ ] テスト: イベント出力が正しい形式

## 2.3 OpenClaw main 呼び出しテスト

- [ ] OpenClaw main セッションで register → status → assign を実行
- [ ] exit code で分岐する SOUL.md 追記
- [ ] サブエージェント（カエデ）に作業を配布するテスト

## 2.4 tasks.json の memory sync (~30行)

- [ ] OpenClaw memory sync 対象に `project_memory/` を追加
- [ ] sync 頻度設定（タスク変更時 + 定期5分）

## 2.5 E2E: OpenClaw → miyabi gate → エージェント → merge

- [ ] register → dispatchable → spawn kade → assign → pr → merge → done
- [ ] 全ステップのログ確認

## 承認ゲート

- [ ] OpenClaw main が register → merge を完走
- [ ] `git tag v1.1-openclaw-dock`
