ターゲットとする機能のベースライン： codex cli Ref_URL=https://github.com/openai/codex.git

ターゲットとする機能をベースに拡張すべき機能を有するPJ: /Users/shunsuke/dev/01-miyabi/_core/miyabi-private

===


# miyabi-cli-standalone

**Miyabi CLI Standalone** - 軽量スタンドアロン版 Miyabi CLI/TUI

## プロジェクト概要

Miyabi本体から独立した軽量CLIツール。TUI (Terminal User Interface) とCLI機能を提供。

## 技術スタック

- **言語**: Rust 2021 Edition, TypeScript
- **TUIフレームワーク**: Ratatui + Crossterm
- **CLIフレームワーク**: Clap
- **非同期ランタイム**: Tokio

## ディレクトリ構造

```
miyabi-cli-standalone/
├── crates/
│   ├── miyabi-cli/          # CLI実装
│   ├── miyabi-core/         # コアライブラリ
│   └── miyabi-tui/          # TUI実装
├── src/                     # TypeScriptソース
│   └── index.ts
├── tests/                   # テスト
├── .claude/                 # Claude Code設定
│   ├── agents/              # Agent仕様
│   ├── commands/            # カスタムコマンド
│   └── prompts/             # プロンプト
├── docs/                    # ドキュメント
├── logs/                    # ログ
├── reports/                 # レポート
├── Cargo.toml               # Rustワークスペース設定
├── package.json             # Node.js設定
└── .miyabi.yml              # Miyabi設定
```

## 開発コマンド

### Rust

```bash
# ビルド
cargo build --release

# テスト
cargo test --all

# Lint
cargo clippy --all-targets -- -D warnings

# フォーマット
cargo fmt --all
```

### TypeScript

```bash
# インストール
npm install

# ビルド
npm run build

# Lint
npm run lint
```

## コミット規約

Conventional Commits準拠:
- `feat:` - 新機能
- `fix:` - バグ修正
- `refactor:` - リファクタリング
- `docs:` - ドキュメント
- `test:` - テスト
- `chore:` - その他

## 環境変数

```bash
GITHUB_TOKEN=ghp_xxx        # GitHub PAT
RUST_LOG=info               # ログレベル
RUST_BACKTRACE=1            # バックトレース
```

## Miyabi統合

```bash
# ステータス確認
miyabi status

# Agent実行
miyabi agent run coordinator --issue <番号>
```

## コーディング規約

### Rust
- `cargo fmt` でフォーマット
- `cargo clippy` 警告ゼロ
- Result型でエラーハンドリング
- async/awaitでの非同期処理

### TypeScript
- ESLint設定に従う
- 厳格な型付け

---

**このファイルはClaude Codeが自動参照します。**
