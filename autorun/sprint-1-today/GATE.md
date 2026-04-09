# Sprint 1 承認ゲート

## 通過条件（全て GREEN で Sprint 2 へ進む）

- [ ] `cargo test --all` → 全 GREEN（900+ テスト）
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` → ゼロエラー
- [ ] `cargo build --release` → 成功
- [ ] verify_merge が mock で動作する
- [ ] force_unlock が event log に記録する
- [ ] manual_complete が done に遷移する
- [ ] E2E テスト: register → done の全シーケンスが通る
- [ ] GATE 拒否テスト: 6件以上が正しく拒否する
- [ ] GNI 再インデックス完了
- [ ] `v1.0-dtp-complete` タグが打たれている

## 失敗時

- テスト失敗 → 原因特定 → 修正 → 再テスト（最大3回）
- clippy エラー → 該当行修正 → 再チェック
- 3回連続失敗 → エスカレーション（人間判断）
