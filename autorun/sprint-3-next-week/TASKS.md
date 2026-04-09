# Sprint 3: 運用基盤（来週）

> Sprint 2 の v1.1 タグ完了後に開始

## 3.1 Heartbeat デーモン (~50行)

- [ ] `miyabi gate heartbeat --all` で全アクティブタスクの lease 更新
- [ ] launchd plist 作成（60秒間隔）
- [ ] `launchctl load` で自動起動
- [ ] stale 検出テスト

## 3.2 tasks.json git 自動同期 (~30行)

- [ ] merge 完了時に自動 git add + commit + push
- [ ] hooks.rs に DtpTaskCompleted イベント追加
- [ ] hooks.yaml に自動コミットフック登録

## 3.3 Telegram 通知 (~50行)

- [ ] GATE 通過/拒否を Telegram に通知
- [ ] HIGH/CRITICAL 承認要求を Telegram ボタンで
- [ ] Phase 完了報告

## 3.4 VOICEBOX アナウンス自動化 (~20行)

- [ ] hooks.yaml で DTP イベント → announce 連携
- [ ] テンプレート: "Polaris: {task_id} のゲート {gate_name} を通過しました"

## 3.5 Maestro Playbook 登録

- [ ] Auto Run 形式に変換
- [ ] Maestro GUI で実行確認
- [ ] Session Isolation + Worktree Support

## 承認ゲート

- [ ] Heartbeat + Telegram + VOICEBOX が全て動作
- [ ] `git tag v1.2-ops-ready`
