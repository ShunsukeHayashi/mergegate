# Phase 7: CLI（dtp コマンド）

> 依存: Phase 6 GREEN
> 承認ゲート: 全サブコマンドが動作 + `dtp run-e2e` が GREEN

## タスク

- [ ] `src/main.rs` に clap ベースの CLI を実装（Cargo.toml に `clap = { version = "4", features = ["derive"] }` 追加）
- [ ] サブコマンド一覧:
  ```
  dtp register --issue <N> --title <T> [--deps <id,...>] [--mode github-pr|manual|external-op]
  dtp dag                          # DAG 可視化（テキスト）
  dtp dispatchable                 # 実行可能タスク一覧
  dtp impact <task-id> --risk <L|M|H|C> --symbols <N> [--approve]
  dtp assign <task-id> --agent <A> --node <N> --files <F,...>
  dtp heartbeat <task-id>
  dtp branch <task-id> <branch-name>
  dtp pr <task-id> <pr-number>
  dtp verify-merge <task-id>       # GitHub API 経由
  dtp confirm-done <task-id>
  dtp status [task-id]             # JSON or human-readable
  dtp locks                        # ロック一覧
  dtp unlock <task-id> --reason <R> --operator <O>
  dtp events [task-id]             # event log 表示
  dtp rebuild-snapshot             # event log から snapshot 再構築
  dtp run-e2e                      # 内蔵 E2E テスト（Phase 8 統合テスト）
  ```
- [ ] `--format json` オプション: 全コマンドで JSON 出力対応（rust-ai-pipeline の Phase1OutputFormat パターン）
- [ ] 終了コード: 0=成功, 1=GATE拒否, 2=入力不正（rust-ai-pipeline の EXIT_* パターン）
- [ ] テスト: register → status で task が存在する
- [ ] テスト: 不正入力（issue=0）→ 終了コード 2

## 承認ゲート

- `cargo build` 成功
- `dtp register --issue 1 --title test` が動作する
- `dtp status` が JSON を返す

## リトライ条件

- clap の derive マクロでコンパイルエラー → features を確認
