# 知的財産保護方針

## 権利者

合同会社みやび (Miyabi G.K.) / 林 駿甫 (Shunsuke Hayashi)

## ライセンス

**BSL 1.1 (Business Source License)**
- 非商用・教育目的: 無料
- 商用利用: 合同会社みやびとの別途ライセンス契約が必要
- 4年後に Apache 2.0 に自動移行

## リポジトリ

- **Miyabi-G-K/miyabi-cli-standalone**: Private
- **Miyabi-G-K/deterministic-task-protocol**: Private
- 公開しない。npm にも公開しない。

## 保護対象

1. **Polaris (DTP) アーキテクチャ**
   - GATE チェーンによる確定的状態遷移
   - ファイルロック + DAG + ステートマシンの三位一体
   - ワークツリー不要の論理的並列分離

2. **記憶アタッチメント方式**
   - ピンポイント・コンテキスト差し込み
   - ドリーミング（event log → 学び昇格）
   - シータサイクル統合

3. **miyabi gate CLI**
   - 11+ サブコマンドの実装
   - pre-commit / post-commit hook 統合
   - Bus ドッキングブリッジ

## 禁止事項

- npm / crates.io への公開禁止
- GitHub を public にしない
- ソースコードの外部共有禁止
- フォーク・再配布は BSL 1.1 に従う

## エージェントへの指示

- コードを外部に送信しない
- API キー・トークンをコミットしない
- Issue/PR の内容を外部に公開しない
- `git push --force` で履歴を公開リポに流さない
