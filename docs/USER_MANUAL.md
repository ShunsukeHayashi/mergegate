# MergeGate ユーザーマニュアル

**Version**: 0.1.0
**Last Updated**: 2026-04-10

## はじめに

MergeGate は、AI-assisted development 向けの engine-agnostic gate CLI です。

このツールはエージェント本体ではありません。TUI や direct backend を前提にせず、repo 作業を安全に進めるための protocol を提供します。

役割は次の通りです。

- task を登録する
- impact を記録する
- file lock を取る
- branch / PR / merge 状態を結び付ける
- completion を記録する

設計思想:

- `GitNexus`: コードベースを理解する
- `MergeGate`: 変更を安全に実行する

## 何をするツールか

MergeGate の中心機能は `gate` です。

```bash
mergegate gate ...
```

または互換 alias として:

```bash
miyabi gate ...
```

実行エンジンは Claude Code、Codex、Gemini CLI など、どれでも構いません。MergeGate はその前後で repo workflow を制御します。

## インストール

### 必要要件

- Rust 1.70以上

### ビルド

```bash
git clone https://github.com/ShunsukeHayashi/mergegate.git
cd mergegate
cargo build --release
```

## 最初にやること

### 1. repo の状態確認

```bash
./target/release/mergegate gate status
```

`tasks: 0` は正常です。ledger はあるが task がまだ無い状態です。

### 2. 初期化

```bash
./target/release/mergegate gate init
```

これで `project_memory/tasks.json` が作成されます。

### 3. ガイド確認

```bash
./target/release/mergegate gate guide
```

## 基本フロー

```bash
./target/release/mergegate gate register --issue 123 --title "Fix login redirect"
./target/release/mergegate gate impact issue-123 --risk medium --symbols 3
./target/release/mergegate gate assign issue-123 --agent codex --node macbook --files "src/auth.rs"
./target/release/mergegate gate branch issue-123 codex/fix-login-redirect
./target/release/mergegate gate pr issue-123 456
./target/release/mergegate gate merge issue-123 <sha>
```

典型的な順番:

1. `register`
2. `impact`
3. `assign`
4. 実装
5. `branch`
6. `pr`
7. `merge` または `manual-complete`

## コマンド一覧

| コマンド | 説明 |
|---------|------|
| `mergegate gate status` | ledger 全体または task 状態を表示 |
| `mergegate gate init` | ledger を初期化 |
| `mergegate gate guide` | workflow ガイドを表示 |
| `mergegate gate register` | task を登録 |
| `mergegate gate impact` | impact を記録 |
| `mergegate gate assign` | task を割り当てて lock を取得 |
| `mergegate gate branch` | branch を記録 |
| `mergegate gate pr` | PR 番号を記録 |
| `mergegate gate merge` | merge を記録 |
| `mergegate gate manual-complete` | 手動完了を記録 |
| `mergegate gate locks` | active lock を表示 |
| `mergegate gate dispatchable` | 今着手できる task を表示 |
| `mergegate gate dag` | 依存順序を表示 |
| `mergegate gate serve` | MergeGate-native web surface を表示 |

## Web Surface

`mergegate gate serve` でローカル dashboard を起動できます。

```bash
./target/release/mergegate gate serve
./target/release/mergegate gate serve --port 8080
```

この UI は次の 3 面で構成されます。

- Gate Overview: validation severity、next actions、ready / attention / progress queue
- Task Ledger: `state` `risk` `since` filter と task detail drill-in
- Dependency Map: DAG level、blocked chain、dispatchable frontier の可視化

静的アセットの探索順は次の通りです。

1. `MERGEGATE_DASHBOARD_STATIC_DIR`
2. `web/dashboard/dist`
3. 内蔵 dashboard fallback

内蔵 UI でも API は Rust-owned contract をそのまま表示します。frontend 独自の task state は持ちません。

foundation release の境界と最終確認手順は [docs/FOUNDATION_RELEASE_CHECKLIST.md](FOUNDATION_RELEASE_CHECKLIST.md) を参照してください。

JSON 形式で統計を出す場合は、`mergegate gate stats --format json` ではなく次を使います。

```bash
./target/release/mergegate gate --format json stats
```

## 実行エンジンとの関係

MergeGate は coding agent を置き換えるものではありません。

想定している使い方:

- Claude Code が実装する
- Codex が実装する
- Gemini CLI が実装する
- MergeGate が task / lock / merge discipline を管理する

つまり:

- エージェントは交換可能
- gate protocol は固定

## よくある質問

### API キーは必要ですか

`gate` workflow だけなら不要です。

### TUI は必要ですか

不要です。MergeGate の本体ではありません。

### built-in backend は必要ですか

不要です。MergeGate の本体ではありません。

### `miyabi` と `mergegate` のどちらを使えばいいですか

新規利用では `mergegate` を推奨します。`miyabi` は互換 alias です。

## トラブルシューティング

### `tasks: 0`

異常ではありません。task が未登録なだけです。

### どの task から始めればよいか分からない

```bash
./target/release/mergegate gate dispatchable
./target/release/mergegate gate dag
./target/release/mergegate gate guide
```

### 既存 repo に導入済みか分からない

```bash
./target/release/mergegate gate status
```

ledger が無ければ `gate init` に進めます。
