# Miyabi CLI ユーザーマニュアル

**Version**: 0.1.0
**Last Updated**: 2025-11-23

---

## 目次

1. [はじめに](#はじめに)
2. [インストール](#インストール)
3. [初期設定](#初期設定)
4. [基本的な使い方](#基本的な使い方)
5. [TUIモード](#tuiモード)
6. [Agentモード](#agentモード)
7. [セッション管理](#セッション管理)
8. [設定ファイル](#設定ファイル)
9. [プロジェクトルール](#プロジェクトルール)
10. [トラブルシューティング](#トラブルシューティング)

---

## はじめに

Miyabi CLIは、ターミナルで動作するAIアシスタントです。Claude APIを使用して、対話型のチャットや自律的なタスク実行が可能です。

### 主な機能

- **TUIモード**: 美しいターミナルUIでの対話
- **Agentモード**: ファイル操作やコマンド実行を含む自律実行
- **セッション管理**: 会話履歴の保存・再開
- **プロジェクトルール**: .miyabirulesによるカスタムルール

---

## インストール

### 必要要件

- Rust 1.70以上
- Anthropic APIキー

### ビルド手順

```bash
# 1. リポジトリをクローン
git clone https://github.com/ShunsukeHayashi/miyabi-cli-standalone.git
cd miyabi-cli-standalone

# 2. リリースビルド
cargo build --release

# 3. バイナリの確認
ls -la target/release/miyabi
```

### パスを通す（オプション）

```bash
# ~/.bashrc または ~/.zshrc に追加
export PATH="$PATH:/path/to/miyabi-cli-standalone/target/release"

# 設定を反映
source ~/.bashrc  # または source ~/.zshrc
```

---

## 初期設定

### 1. 設定ファイルの生成

```bash
./target/release/miyabi init
```

これにより `~/.miyabi/config.toml` が作成されます。

### 2. APIキーの設定

**方法1: 環境変数（推奨）**
```bash
export ANTHROPIC_API_KEY="sk-ant-api03-..."
```

永続化する場合は `~/.bashrc` や `~/.zshrc` に追加：
```bash
echo 'export ANTHROPIC_API_KEY="sk-ant-api03-..."' >> ~/.zshrc
```

**方法2: 設定ファイル**
```bash
vim ~/.miyabi/config.toml
```

```toml
[api]
api_key = "sk-ant-api03-..."
```

### 3. 設定の確認

```bash
./target/release/miyabi status
```

出力例：
```
Miyabi Status: Ready

Config:   /Users/you/.miyabi/config.toml
Sessions: /Users/you/.miyabi/sessions
Model:    claude-sonnet-4-5-20250929

Rules:    0 rules loaded
```

---

## 基本的な使い方

### コマンド一覧

| コマンド | 説明 |
|---------|------|
| `miyabi tui` | TUIモードを起動 |
| `miyabi agent <prompt>` | Agentモードで実行 |
| `miyabi status` | ステータス表示 |
| `miyabi version` | バージョン情報 |
| `miyabi init` | 設定ファイル生成 |
| `miyabi sessions` | セッション一覧 |
| `miyabi rules` | プロジェクトルール表示 |

### グローバルオプション

| オプション | 説明 |
|-----------|------|
| `-m, --model <MODEL>` | 使用するモデルを指定 |
| `--max-tokens <N>` | 最大トークン数 |
| `--thinking` | Extended Thinking有効化 |
| `-c, --config <PATH>` | 設定ファイルパス |
| `-s, --session <ID>` | セッションID指定 |

### 使用例

```bash
# バージョン確認
./target/release/miyabi version

# ステータス確認
./target/release/miyabi status

# モデル指定でTUI起動
./target/release/miyabi tui --model claude-sonnet-4-5-20250929

# Extended Thinking有効でTUI起動
./target/release/miyabi tui --thinking
```

---

## TUIモード

### 起動

```bash
./target/release/miyabi tui
```

### キーバインド

#### 基本操作

| キー | 動作 |
|------|------|
| `Enter` | メッセージ送信 |
| `Ctrl+C` | 終了 |
| `Esc` | オーバーレイを閉じる / キャンセル |
| `F1` | ヘルプ表示 |

#### ナビゲーション

| キー | 動作 |
|------|------|
| `j` / `↓` | 下にスクロール |
| `k` / `↑` | 上にスクロール |
| `Page Down` | ページ下へ |
| `Page Up` | ページ上へ |
| `g` | 先頭へ |
| `G` | 末尾へ |

#### コマンド

| キー | 動作 |
|------|------|
| `Ctrl+P` | コマンドパレット |
| `Ctrl+N` | 新規セッション |
| `Ctrl+O` | セッションを開く |
| `Ctrl+S` | セッション保存 |

### Vimモード

設定で有効化できます：

```toml
[ui]
vim_mode = true
```

有効にすると、テキスト入力でVimキーバインドが使用可能になります。

---

## Agentモード

Agentモードは、プロンプトに基づいて自律的にタスクを実行します。

### 基本使用

```bash
./target/release/miyabi agent "タスクの説明"
```

### オプション

| オプション | 説明 | デフォルト |
|-----------|------|-----------|
| `--max-iterations <N>` | 最大イテレーション数 | 10 |
| `--auto-approve` | ツール実行を自動承認 | false |
| `--format <FORMAT>` | 出力形式 (text/json) | text |
| `--system <PROMPT>` | システムプロンプト | なし |

### 使用例

```bash
# 基本的な実行
./target/release/miyabi agent "Create a Python script that prints hello world"

# 自動承認モード（注意して使用）
./target/release/miyabi agent --auto-approve "List all .rs files in src/"

# イテレーション数を増やす
./target/release/miyabi agent --max-iterations 20 "Refactor the utils module"

# JSON出力
./target/release/miyabi agent --format json "Show current directory"

# カスタムシステムプロンプト
./target/release/miyabi agent --system "You are an expert Rust developer" "Review main.rs"
```

### 利用可能なツール

Agentモードでは以下のツールが使用可能です：

- **Bash**: シェルコマンドの実行
- **Read**: ファイルの読み取り
- **Write**: ファイルの書き込み
- **Edit**: ファイルの編集
- **Glob**: ファイルパターン検索
- **Grep**: テキスト検索

### セキュリティ

- デフォルトでは危険な操作は承認を求められます
- `--auto-approve` は信頼できるタスクのみに使用してください
- `--max-iterations` で無限ループを防止

---

## セッション管理

### セッション一覧の表示

```bash
./target/release/miyabi sessions
```

出力例：
```
ID                                   Title                Messages Tokens     Updated
------------------------------------------------------------------------------------------
133b91bf-99c9-4097-974c-c60c9abd2495 test                 6        246        2025-11-22 14:36
19af5958-183f-4b39-be2e-b14b0809181c test                 2        21         2025-11-22 14:03
```

### セッションの操作

```bash
# セッションを削除
./target/release/miyabi sessions -d <session-id>

# JSONにエクスポート
./target/release/miyabi sessions -e <session-id>

# Markdownにエクスポート
./target/release/miyabi sessions -m <session-id>
```

### セッションの再開

```bash
# 特定のセッションからTUIを起動
./target/release/miyabi tui -s <session-id>
```

### 保存場所

セッションは `~/.miyabi/sessions/` に保存されます。

---

## 設定ファイル

### 設定ファイルの場所

```
~/.miyabi/config.toml
```

### 完全な設定例

```toml
[api]
# Anthropic APIキー（環境変数ANTHROPIC_API_KEYでも設定可能）
api_key = "sk-ant-api03-..."

# 使用するモデル
model = "claude-sonnet-4-5-20250929"

# 最大トークン数
max_tokens = 8192

# Extended Thinking（Claude 4.5+）
thinking = false

# システムプロンプト
system_prompt = "You are a helpful AI assistant."

# リクエストタイムアウト（秒）
timeout_secs = 120

# 最大リトライ回数
max_retries = 3

[ui]
# サイドバー表示
show_sidebar = false

# ステータスバー表示
show_status_bar = true

# パンくずリスト表示
show_breadcrumb = true

# カラーテーマ
theme = "tokyo-night"

# Vimモード
vim_mode = false

# コードブロックの行番号
show_line_numbers = true

[session]
# セッションの自動保存
auto_save = true

# 自動保存間隔（秒）
auto_save_interval = 30

# 最大セッション数
max_sessions = 100

[tools]
# Bashツールを有効化
enable_bash = true

# ファイルツール（read/write/edit）を有効化
enable_file_tools = true

# 検索ツール（glob/grep）を有効化
enable_search_tools = true

# 低リスクツールの自動承認
auto_approve_low_risk = false

# Bashコマンドのタイムアウト（秒）
bash_timeout = 120
```

### 環境変数による上書き

| 環境変数 | 説明 |
|---------|------|
| `ANTHROPIC_API_KEY` | APIキー |
| `MIYABI_MODEL` | モデル名 |
| `MIYABI_MAX_TOKENS` | 最大トークン数 |
| `MIYABI_THINKING` | Extended Thinking (true/false) |

---

## プロジェクトルール

### .miyabirulesファイル

プロジェクトルートに `.miyabirules` ファイルを作成することで、プロジェクト固有のルールを定義できます。

### ファイル形式

```yaml
version: 1

rules:
  - name: "no-unwrap"
    pattern: ".unwrap()"
    suggestion: "Use ? operator or proper error handling"
    file_extensions: ["rs"]
    severity: "warning"

  - name: "no-println-debug"
    pattern: "println!"
    suggestion: "Use tracing macros for logging"
    file_extensions: ["rs"]
    severity: "info"

agent_preferences:
  codegen:
    style: "functional"
    error_handling: "result"
    min_score: 80
  review:
    min_score: 70
```

### ルールの確認

```bash
./target/release/miyabi rules
```

### 重要度レベル

| レベル | 説明 |
|--------|------|
| `error` | 重大な問題（赤） |
| `warning` | 警告（黄） |
| `info` | 情報（青） |

---

## トラブルシューティング

### "API key not found" エラー

```bash
# 環境変数を確認
echo $ANTHROPIC_API_KEY

# 設定されていない場合
export ANTHROPIC_API_KEY="sk-ant-..."
```

### "Device not configured" エラー

ターミナルがTUIをサポートしていない可能性があります：

```bash
# 別のターミナルを試す（iTerm2, Alacritty等）

# または明示的にTERM設定
TERM=xterm-256color ./target/release/miyabi tui
```

### ビルドエラー

```bash
# Rustを最新版に更新
rustup update

# クリーンビルド
cargo clean
cargo build --release
```

### セッションが保存されない

```bash
# ディレクトリの確認
ls -la ~/.miyabi/sessions/

# なければ作成
mkdir -p ~/.miyabi/sessions
```

### デバッグモード

詳細なログを表示：

```bash
RUST_LOG=debug ./target/release/miyabi tui
```

### ネットワークエラー

```bash
# タイムアウトを延長
# ~/.miyabi/config.toml
[api]
timeout_secs = 300
max_retries = 5
```

---

## サポート

### ヘルプの確認

```bash
# 全体のヘルプ
./target/release/miyabi --help

# サブコマンドのヘルプ
./target/release/miyabi agent --help
./target/release/miyabi sessions --help
```

### 問題報告

GitHub Issues: https://github.com/ShunsukeHayashi/miyabi-cli-standalone/issues

### TUIでのヘルプ

TUI起動中に `F1` キーでヘルプを表示できます。

---

## 付録

### 対応モデル

- `claude-sonnet-4-5-20250929` (デフォルト)
- `claude-haiku-4-5-20251001`
- `claude-sonnet-4-20250514`

### ファイル構成

```
~/.miyabi/
├── config.toml      # 設定ファイル
└── sessions/        # セッションデータ
    ├── xxx.json
    └── yyy.json
```

### ショートカット早見表

| 操作 | キー |
|------|------|
| 送信 | Enter |
| 終了 | Ctrl+C |
| ヘルプ | F1 |
| コマンドパレット | Ctrl+P |
| 新規セッション | Ctrl+N |
| セッションを開く | Ctrl+O |
| 保存 | Ctrl+S |

---

**Built with Rust, Ratatui, and Claude API**
