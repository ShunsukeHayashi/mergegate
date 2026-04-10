# Miyabi CLI - 開発者ガイド

Rust製のClaude API統合ターミナルUIアシスタント

## 概要

Miyabi CLIはAnthropic Claude APIと連携した高機能ターミナルUIアシスタントです。

## 主要機能

### TUI (Terminal User Interface)
- リアルタイムMarkdownレンダリング
- シンタックスハイライト
- Vimキーバインド対応
- コマンドパレット (Ctrl+P)
- セッション管理

### セッション管理
- 自動保存 (設定可能)
- JSON/Markdownエクスポート
- セッション再開機能

### 設定システム
- TOML設定ファイル (~/.miyabi/config.toml)
- 環境変数オーバーライド
- CLIオプション

### 組み込みツール
- Bash実行
- ファイル読み書き
- Glob/Grep検索
- コード編集

## クイックスタート

```bash
# ビルド
cargo build --release

# 起動
./target/release/miyabi
```

## 使い方

### 基本コマンド

```bash
# TUI起動
miyabi

# バージョン確認
miyabi version

# ヘルプ
miyabi --help

# ステータス確認
miyabi status
```

### CLIオプション

```bash
# モデル指定
miyabi --model claude-sonnet-4-5-20250929

# トークン数指定
miyabi --max-tokens 16384

# 設定ファイル指定
miyabi --config path/to/config.toml

# セッション再開
miyabi --session <session-id>

# 拡張思考モード
miyabi --thinking
```

### セッション操作

```bash
# 一覧表示
miyabi sessions

# Markdownエクスポート
miyabi sessions -m <id>

# JSONエクスポート
miyabi sessions -e <id>

# 削除
miyabi sessions -d <id>
```

### Agentモード

```bash
# 基本実行
miyabi agent "Pythonでhello worldを作成"

# オプション付き
miyabi agent "リファクタリング" \
  --max-iterations 20 \
  --auto-approve \
  --system "あなたはRustエキスパートです"
```

## 設定

### 設定ファイル

`~/.miyabi/config.toml`:

```toml
[api]
api_key = "sk-ant-..."
model = "claude-sonnet-4-5-20250929"
max_tokens = 8192
timeout_secs = 120
max_retries = 3
thinking = false
system_prompt = "You are a helpful assistant"

[ui]
theme = "tokyo-night"
vim_mode = false
show_sidebar = false
show_status_bar = true
show_breadcrumb = true
show_line_numbers = true

[session]
auto_save = true
auto_save_interval = 30
max_sessions = 100

[tools]
enable_bash = true
enable_file_tools = true
enable_search_tools = true
auto_approve_low_risk = false
bash_timeout = 120
```

### 環境変数

```bash
export ANTHROPIC_API_KEY="your-key"
export MIYABI_MODEL="claude-sonnet-4-5-20250929"
export MIYABI_MAX_TOKENS="8192"
export MIYABI_THINKING="true"
```

## TUIキーバインド

| キー | 動作 |
|------|------|
| Enter | メッセージ送信 |
| Ctrl+P | コマンドパレット |
| F1 | ヘルプ |
| Esc | キャンセル/閉じる |
| Ctrl+C | 終了 |
| j/k | スクロール |
| Page Up/Down | ページスクロール |
| Tab | フォーカス切り替え |

## Core Library機能

### プロジェクトルール (.miyabirules)

プロジェクトルートに`.miyabirules`ファイルを配置:

```yaml
version: 1
rules:
  - name: "no-unwrap"
    pattern: ".unwrap()"
    suggestion: "?演算子を使用してください"
    file_extensions: ["rs"]
    severity: "warning"

agent_preferences:
  codegen:
    style: "functional"
    error_handling: "result"
```

### フィーチャーフラグ

```rust
use miyabi_core::FeatureFlagManager;

let flags = FeatureFlagManager::new();
flags.set_flag("new_feature", true);

if flags.is_enabled("new_feature") {
    // 新機能を使用
}
```

### プラグインシステム

カスタムプラグインで機能拡張:

```rust
use miyabi_core::{Plugin, PluginManager, PluginMetadata};

struct MyPlugin;

impl Plugin for MyPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "my-plugin".to_string(),
            version: "1.0.0".to_string(),
            description: Some("カスタムプラグイン".to_string()),
            author: None,
        }
    }
    // ...
}

let manager = PluginManager::new();
manager.register(Box::new(MyPlugin))?;
```

### キャッシュシステム

LLMレスポンスのキャッシュ:

```rust
use miyabi_core::{create_llm_cache, LLMCacheKey};

let cache = create_llm_cache(); // 1時間TTL
let key = LLMCacheKey::new("prompt", "model", Some(0.7));
cache.insert(key.clone(), "response".to_string()).await;
```

### Circuit Breaker

障害時の自動遮断:

```rust
use miyabi_core::CircuitBreaker;

let breaker = CircuitBreaker::default();
let result = breaker.call(|| {
    Box::pin(async { api_call().await })
}).await;
```

## プロジェクト構造

```
miyabi-cli-standalone/
├── crates/
│   ├── miyabi-cli/         # CLIエントリーポイント
│   ├── miyabi-core/        # コアライブラリ
│   └── miyabi-tui/         # TUI実装
├── .miyabi/                # ローカル設定
│   ├── config.toml         # 設定ファイル
│   ├── sessions/           # 保存セッション
│   ├── commands/           # カスタムコマンド
│   └── templates/          # テンプレート
├── Cargo.toml
├── README.md
└── MIYABI.md               # このファイル
```

## 開発

```bash
# ビルド
cargo build --release

# テスト
cargo test --all

# Lint
cargo clippy --all-targets -- -D warnings
cargo fmt --all --check

# 実行
cargo run -p mergegate-cli --bin mergegate
```

## ライセンス

MIT License
