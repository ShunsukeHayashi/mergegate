# Next Steps — Gate 可視化・検証機能の完成

_Updated: 2026-04-12_

---

## 直近の完了条件

Gate CLI を「ledger を読める・監査できる・運用に載せられる」状態まで引き上げる。

完了条件:

- `mergegate gate validate` が snapshot 整合性を検査できる
- `mergegate gate export-json` / `export-md` が同じ filter 仕様を持つ
- `mergegate gate stats --format json` のスキーマが固定される
- dashboard が `/api/tasks` / `/api/stats` / `/api/validate` を使って CLI と同じ定義で表示する
- README のコマンド例が現行実装と一致する

---

## 今週

### 1. Validation を正式公開

- `gate validate` を追加
- text 出力で `clean` / `warning` / `error` を先頭表示
- JSON 出力を OpenClaw 連携向けに固定
- 終了コードを定義
  - `0`: clean
  - `1`: warnings only
  - `2`: consistency error

### 2. Export / Stats の統一

- `export-json` に `--state` / `--risk` / `--since`
- `export-md` に同一 filter を追加
- `stats` に `failed` を追加し、completed / active / waiting の分類を固定
- CLI と dashboard が同一集計ロジックを使うようにする

### 3. Dashboard API の整備

- `/api/tasks`
- `/api/stats`
- `/api/validate`
- 既存 `/api/status` は後方互換のため維持してもよいが、UI は新APIを優先利用する

### 4. テストとドキュメント同期

- `cargo test --all`
- README の使用例を更新
- `docs/PRODUCT_SPEC.md` の先頭を CLI-first に修正

---

## 次フェーズ

### OpenClaw ドッキング

- `dispatchable` と `validate` を使って外部オーケストレータ接続
- heartbeat / lock 状態を OpenClaw 側で監視
- `project_memory/tasks.json` を execution mirror として配布

### 品質ゲートの多層化

- `ai-pipeline phase1` 統合
- 追加の静的検証・動的検証
- GitHub Actions で補助チェック

### 配布・公開面の強化

- guide と docs を一本化
- dashboard の利用例を追加
- export 出力を外部ツールから読みやすい形に磨く
