# Next Steps — 次フェーズ

_Updated: 2026-04-12_

---

監査面の v1 は完了した前提で、ここからは外部連携と品質ゲートを広げる。
現在の固定済み基盤は以下:

- `mergegate gate validate` の text / JSON / exit code 契約
- `export-json` / `export-md` の共通 filter 契約
- `stats --format json` の固定分類
- dashboard の `/api/tasks` `/api/stats` `/api/validate`
- README / product spec の CLI-first 同期

本流方針:

- MergeGate は Rust-first protocol product として進める
- cross-source PM dashboard 化は本体スコープに含めない
- UI は MergeGate-native surface に限定して強化する

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
