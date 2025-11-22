# .miyabi Directory

Miyabi CLI設定ディレクトリ

## 構造

```
.miyabi/
├── agents/
│   └── specs/          # Agent仕様定義
├── commands/           # カスタムコマンド
├── prompts/            # 再利用可能プロンプト
├── templates/          # テンプレート
├── sessions/           # 保存されたセッション
└── config.toml         # 設定ファイル
```

## 設定ファイル

`config.toml` で設定をカスタマイズ:

```toml
[api]
model = "claude-sonnet-4-20250514"
max_tokens = 8192

[ui]
theme = "tokyo-night"
vim_mode = false

[session]
auto_save = true

[tools]
enable_bash = true
```

## カスタムコマンド

`commands/` 配下に `*.md` ファイルを作成してカスタムコマンドを定義。

## セッション管理

```bash
# セッション一覧
miyabi sessions

# Markdownエクスポート
miyabi sessions -m <session-id>

# 削除
miyabi sessions -d <session-id>
```

## 使い方

```bash
# TUI起動
miyabi

# バージョン情報
miyabi version

# ヘルプ
miyabi --help
```
