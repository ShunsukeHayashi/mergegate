# DTP Playbook v3 — 完全版ロードマップ

_v3: Phase A/B 完了後の全タスクを網羅。直近〜長期まで。_

---

## North Star

> LLM の揺らぎを許さない。GATE が許可し、GitHub が確定し、merge が不可逆にする。

---

## 完了済み

| Phase | コミット | 追加行 | 内容 |
|-------|---------|--------|------|
| A | 986d907 | +1,485 | gate.rs + lock.rs + store.rs + protocol.rs |
| B | 273c416 | +1,171 | CLI サブコマンド (miyabi gate *) |

---

## Sprint 1: 品質確定（今日）

### 1.1 clippy 修正

**状態**: 🔄 Codex 実行中

| # | エラー | ファイル | 修正内容 |
|---|--------|---------|---------|
| 1 | unused import: error | anthropic.rs:12 | `use` 行から `error` を削除 |
| 2 | impl can be derived | gate.rs:46 | `#[derive(Default)]` に変更 |
| 3 | map_or can be simplified | gate.rs:137 | `.is_some_and()` に変更 |
| 4 | map_or can be simplified | gate.rs:155 | `.is_some_and()` に変更 |
| 5 | map_or can be simplified | gate.rs:170 | `.is_some_and()` に変更 |
| 6 | large enum variant | protocol.rs:471 | `Box<ExecutionTask>` に変更 |
| 7 | MSRV incompatible .unlock() | store.rs:361 | `drop(lock_file)` に変更 |

**GATE**: `cargo clippy --all-targets --all-features -- -D warnings` → ゼロエラー
**GATE**: `cargo test` → 900+ テスト GREEN

### 1.2 Phase C: GitHub Evidence + E2E

**依存**: 1.1 完了後

**タスク**:

- [ ] `crates/miyabi-core/src/github.rs` の既存 API を確認
  - `get_pull_request()` → PullRequest struct の中身
  - `merge_commit_sha` フィールドがあるか
  - なければ `PullRequest` struct に追加

- [ ] `crates/miyabi-core/src/protocol.rs` に `verify_merge()` 追加
  ```rust
  pub fn verify_merge(&self, task_id: &str, github: &GitHubClient) -> Result<GateReport> {
      let task = self.snapshot_store.load()?.get_task(task_id)?;
      let pr_number = task.pr_number.ok_or(GateError::MissingPr)?;
      let pr = github.get_pull_request(pr_number).await?;
      
      // 検証: PR が merged で、SHA が 40hex
      if pr.state != "merged" { return Err(GateError::NotMerged); }
      let sha = pr.merge_commit_sha.ok_or(GateError::MissingMergeCommit)?;
      if sha.len() != 40 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
          return Err(GateError::InvalidMergeCommit);
      }
      
      // tasks.json 更新
      // ロック解放
      // 後続タスク解放
  }
  ```

- [ ] escape hatch 追加
  ```rust
  pub fn force_unlock(&self, task_id: &str, reason: &str, operator: &str) -> Result<()>
  pub fn manual_complete(&self, task_id: &str, reason: &str, operator: &str) -> Result<()>
  ```

- [ ] CLI に `verify-merge` と `force-unlock` サブコマンド追加

- [ ] E2E テスト（mock GitHub）
  - register → check_deps → impact → assign → branch → pr → verify_merge → done
  - force_unlock テスト
  - manual_complete テスト

- [ ] `cargo test` GREEN
- [ ] `cargo clippy` GREEN

**担当**: Claude Code (local) — GitHub API アクセスが必要
**GATE**: E2E テスト GREEN + clippy GREEN
**完了時**: `git tag v1.0-dtp-phase-c`

### 1.3 GNI 再インデックス + 最終確認

- [ ] `npx gitnexus analyze --force`
- [ ] Impact 確認: 新規関数の依存グラフが正しいか
- [ ] 既存への影響がゼロのままか確認

---

## Sprint 2: OpenClaw ドッキング（今週）

### 2.1 JSON 出力の標準化

**目的**: OpenClaw が `miyabi gate` の出力をパースできるようにする

- [ ] 全サブコマンドで `--format json` が動作することを確認
- [ ] JSON スキーマを固定
  ```json
  {
    "status": "ok" | "gate_rejected" | "error",
    "exit_code": 0 | 1 | 2,
    "task_id": "phase-a",
    "gate": "gate_3_impact" | null,
    "message": "人間可読メッセージ",
    "data": { ... }
  }
  ```
- [ ] exit code の一貫性テスト

**GATE**: 全コマンドの JSON 出力が上記スキーマに準拠

### 2.2 OpenClaw hooks 連携

**目的**: DTP イベントを OpenClaw hooks でキャッチ

- [ ] `miyabi gate register` 完了時に stdout に JSON イベント出力
- [ ] `miyabi gate merge` 完了時に stdout に JSON イベント出力
- [ ] OpenClaw hooks 設定ファイルに DTP イベントテンプレートを追加
  ```yaml
  hooks:
    - event: "tool:after"
      filter: "miyabi gate"
      action: "notify"
      channel: "telegram"
  ```

**参照**: `crates/miyabi-core/src/hooks.rs` の HookEvent パターン

### 2.3 OpenClaw main からの呼び出しテスト

**目的**: OpenClaw main エージェントが DTP を駆動できることを確認

- [ ] OpenClaw main のセッションで以下を実行
  ```
  miyabi gate register --issue 1 --title "test" --store-path ./tasks.json --format json
  miyabi gate status --store-path ./tasks.json --format json
  miyabi gate assign test-task --agent main --node gateway --files "src/test.rs" --store-path ./tasks.json
  ```
- [ ] exit code で分岐する SOUL.md の追記
  ```
  miyabi gate の exit code が 1 なら、GATE 拒否。理由を確認して対処すること。
  exit code が 2 なら、入力エラー。コマンドを修正して再実行。
  ```

**GATE**: OpenClaw main が register → status → assign を成功実行

### 2.4 tasks.json の memory sync

**目的**: 複数ノードで tasks.json を共有

- [ ] OpenClaw memory sync の対象パスに `project_memory/` を追加
- [ ] sync 頻度: タスク状態変更時（即時）+ 定期（5分）
- [ ] 衝突検出: tasks.json の `version` フィールドで CAS

**参照**: `crates/miyabi-core/src/openclaw.rs` の OpenClawClient

### 2.5 サブエージェント配布テスト

**目的**: DAG レベルに基づいてサブエージェントにタスクを振り分け

- [ ] `miyabi gate dispatchable --format json` で実行可能タスクを取得
- [ ] OpenClaw `sessions spawn` でカエデ（CodeGen）を起動
- [ ] カエデが `miyabi gate assign` → 実装 → `miyabi gate pr`
- [ ] サクラ（Review）に引き継ぎ
- [ ] `miyabi gate merge` で完了

**GATE**: カエデ → サクラ → merge の一連が動作

---

## Sprint 3: 運用基盤（来週）

### 3.1 Heartbeat デーモン

**目的**: ファイルロックの lease を自動更新

- [ ] `miyabi gate heartbeat <task-id>` コマンドが動作確認済み
- [ ] launchd plist を作成
  ```xml
  <plist>
    <dict>
      <key>Label</key>
      <string>com.miyabi.dtp.heartbeat</string>
      <key>ProgramArguments</key>
      <array>
        <string>/path/to/miyabi</string>
        <string>gate</string>
        <string>heartbeat</string>
        <string>--all</string>
      </array>
      <key>StartInterval</key>
      <integer>60</integer>
    </dict>
  </plist>
  ```
- [ ] `launchctl load` で自動起動
- [ ] stale 検出テスト: heartbeat 停止 → 2回ミス → ロック自動解放

### 3.2 tasks.json の git 自動同期

**目的**: Phase 完了時に tasks.json を自動で push

- [ ] `miyabi gate merge` 完了時のフック
  ```bash
  git add project_memory/tasks.json
  git commit -m "[自動] tasks.json 更新: $TASK_ID → done"
  git push
  ```
- [ ] `crates/miyabi-core/src/hooks.rs` の HookEvent に `DtpTaskCompleted` を追加
- [ ] hooks.yaml に自動コミットフックを登録

### 3.3 Telegram 通知

**目的**: GATE 通過/拒否をリアルタイム通知

- [ ] GATE 通過時: `「✅ task-001: Gate 3 (impact) 通過。リスク: LOW」`
- [ ] GATE 拒否時: `「❌ task-001: Gate 4 (lock) 拒否。src/auth.rs はロック中」`
- [ ] HIGH/CRITICAL 承認要求: `「⚠️ task-001: HIGH リスク。承認してください」` + ボタン
- [ ] Phase 完了: `「🎉 Phase A 完了。次は Phase B です」`

**参照**: OpenClaw Telegram チャンネル設定

### 3.4 VOICEBOX アナウンス自動化

**目的**: 進捗を音声で自動通知

- [ ] `miyabi gate` の各コマンド完了時に `~/bin/announce` を自動実行
- [ ] hooks.yaml で設定
  ```yaml
  hooks:
    - event: "dtp:gate_passed"
      action: "announce"
      template: "Polaris: {task_id} のゲート {gate_name} を通過しました"
    - event: "dtp:gate_rejected"
      action: "announce"
      template: "Polaris: {task_id} のゲート {gate_name} が拒否されました。理由: {reason}"
  ```

### 3.5 Maestro Playbook 登録

**目的**: Maestro GUI から DTP を実行可能に

- [ ] Phase A/B/C を Maestro Auto Run 形式に変換
- [ ] Maestro Playbook Exchange に登録
- [ ] Session Isolation 有効化
- [ ] Worktree Support 有効化

---

## Sprint 4: 品質ゲート多層化（今月）

### 4.1 rust-ai-pipeline Phase 1 統合

**目的**: `cargo build + clippy + test` を DTP の GATE 4.5 として組み込む

- [ ] `miyabi gate assign` 後に自動実行
  ```bash
  ai-pipeline phase1 --project . --format json
  ```
- [ ] `all_passed == false` → implementing に留まる
- [ ] `failure_kind` を tasks.json に記録
- [ ] CLI に `miyabi gate quality-check <task-id>` サブコマンド追加

### 4.2 proptest 拡張

**目的**: GATE ロジックの property-based testing

- [ ] gate.rs: ランダムなタスク状態で GATE 通過/拒否が一貫することを検証
- [ ] lock.rs: ランダムなファイルセットで acquire/release の不変条件検証
- [ ] store.rs: ランダムなイベント列で snapshot rebuild が冪等であることを検証

### 4.3 cargo-mutants

**目的**: テストの品質を mutation testing で検証

- [ ] `cargo mutants` 実行
- [ ] ミューテーションスコア 80% 以上を目標
- [ ] 殺せないミュータントがあれば、テストを追加

---

## Sprint 5: 移行 + 公開（来月以降）

### 5.1 miyabi-private (TypeScript) からの段階的移行

| 対象 | TypeScript (現在) | Rust (移行先) |
|------|------------------|-------------|
| TaskStateMachine | miyabi-task-manager | protocol.rs (済) |
| BidirectionalSync | miyabi-task-manager | 未実装 → github.rs 拡張 |
| GitHub Label Sync | miyabi-task-manager | github.rs (既存) |
| TaskExecutor | miyabi-task-manager | orchestration.rs (既存) |
| WorktreeCoordinator | miyabi-task-manager | 未実装 → tool.rs 拡張 |
| LLMDecomposer | miyabi-task-manager | 未実装 → 別 crate |

### 5.2 OpenClaw プラグインとして公開

- [ ] `openclaw plugin install miyabi-dtp` で導入可能に
- [ ] プラグインマニフェスト作成
- [ ] ドキュメント: how-to-dock-dtp-with-openclaw.md
- [ ] 公開先: Miyabi-G-K org の public リポ

### 5.3 npm パッケージとしての配布

- [ ] `npx miyabi gate` で Rust バイナリを自動ダウンロード+実行
- [ ] platform 別バイナリ: macOS (arm64/x86), Linux, Windows
- [ ] GitHub Releases でバイナリ配布
- [ ] npm の `postinstall` で適切なバイナリを取得

---

## Sprint DAG

```
Sprint 1 (今日)
  ├── 1.1 clippy修正 ← Codex実行中
  ├── 1.2 Phase C ← 1.1 完了後
  └── 1.3 GNI確認 ← 1.2 完了後
         │
         ▼
Sprint 2 (今週)
  ├── 2.1 JSON標準化
  ├── 2.2 hooks連携 ← 2.1 完了後
  ├── 2.3 OpenClaw呼出 ← 2.1 完了後
  ├── 2.4 memory sync ← 2.3 完了後
  └── 2.5 サブエージェント ← 2.3 + 2.4 完了後
         │
         ▼
Sprint 3 (来週)
  ├── 3.1 Heartbeat
  ├── 3.2 git自動同期 ← 3.1 と並列可
  ├── 3.3 Telegram ← 3.2 と並列可
  ├── 3.4 VOICEBOX ← 3.3 と並列可
  └── 3.5 Maestro登録
         │
         ▼
Sprint 4 (今月)
  ├── 4.1 品質ゲート統合
  ├── 4.2 proptest ← 4.1 と並列可
  └── 4.3 cargo-mutants ← 4.2 完了後
         │
         ▼
Sprint 5 (来月以降)
  ├── 5.1 TS→Rust移行
  ├── 5.2 OpenClaw公開 ← 5.1 と並列可
  └── 5.3 npm配布 ← 5.2 完了後
```

---

## 各 Sprint の承認ゲート

| Sprint | GATE | 承認者 |
|--------|------|--------|
| 1 | `cargo test GREEN` + `cargo clippy ZERO` + E2E テスト | 自動 |
| 2 | OpenClaw main が register→merge を完走 | 人間確認 |
| 3 | Heartbeat + Telegram + VOICEBOX が全て動作 | 人間確認 |
| 4 | mutation score 80%+ | 自動 |
| 5 | npm install で動く + OpenClaw plugin install で動く | 人間確認 |

---

## ロールバックポイント

| 地点 | タグ | 内容 |
|------|------|------|
| Phase A 完了 | `v0.1-dtp-phase-a` | gate + lock + store + protocol |
| Phase B 完了 | `v0.2-dtp-phase-b` | CLI サブコマンド |
| Phase C 完了 | `v1.0-dtp-complete` | GitHub Evidence + E2E |
| Sprint 2 完了 | `v1.1-openclaw-dock` | OpenClaw ドッキング |
| Sprint 3 完了 | `v1.2-ops-ready` | 運用基盤完成 |
| Sprint 4 完了 | `v1.3-quality-gates` | 多層品質ゲート |
| Sprint 5 完了 | `v2.0-public` | 公開版 |

---

_This is the single source of truth for all DTP development._
