# .miyabi Directory

Miyabi CLI設定・拡張ディレクトリ

## ディレクトリ構造

```
.miyabi/
├── config.toml         # メイン設定ファイル
├── README.md           # このファイル
├── agents/
│   └── specs/          # Agent仕様定義
├── commands/           # カスタムスラッシュコマンド
├── prompts/            # 再利用可能プロンプト
├── templates/          # コード・ドキュメントテンプレート
├── sessions/           # 保存されたセッションデータ
└── tasks/              # タスク管理データ
```

## 設定ファイル (config.toml)

### API設定

```toml
[api]
# API Key (環境変数 ANTHROPIC_API_KEY でも設定可能)
api_key = "sk-ant-..."

# 使用モデル
model = "claude-sonnet-4-5-20250929"

# 最大トークン数
max_tokens = 8192

# タイムアウト (秒)
timeout_secs = 120

# リトライ回数
max_retries = 3

# 拡張思考モード
thinking = false

# システムプロンプト
system_prompt = "You are a helpful assistant"
```

### UI設定

```toml
[ui]
# テーマ: tokyo-night, dracula, monokai, etc.
theme = "tokyo-night"

# Vimキーバインド
vim_mode = false

# サイドバー表示
show_sidebar = false

# ステータスバー表示
show_status_bar = true

# パンくずリスト表示
show_breadcrumb = true

# コードの行番号表示
show_line_numbers = true
```

### セッション設定

```toml
[session]
# 自動保存
auto_save = true

# 保存間隔 (秒)
auto_save_interval = 30

# 最大セッション数
max_sessions = 100
```

### ツール設定

```toml
[tools]
# Bashツール有効化
enable_bash = true

# ファイル操作ツール有効化
enable_file_tools = true

# 検索ツール有効化
enable_search_tools = true

# 低リスク操作の自動承認
auto_approve_low_risk = false

# Bashタイムアウト (秒)
bash_timeout = 120
```

## カスタムコマンド

`commands/` 配下に `.md` ファイルを作成してスラッシュコマンドを定義。

### 例: /review コマンド

`commands/review.md`:
```markdown
# コードレビュー

以下のコードをレビューしてください:

1. バグの可能性
2. パフォーマンス問題
3. セキュリティ脆弱性
4. コーディング規約違反

改善提案も含めてください。
```

使用: TUIで `/review` と入力

## プロンプトテンプレート

`prompts/` 配下に再利用可能なプロンプトを保存。

### 例: Rustコードレビュー

`prompts/rust-review.md`:
```markdown
# Rustコードレビューチェックリスト

- [ ] unwrap/expect の使用箇所
- [ ] エラーハンドリング
- [ ] 所有権とライフタイム
- [ ] unsafe コードの妥当性
- [ ] Clippy警告の確認
```

## Agent仕様

`agents/specs/` にカスタムAgent仕様を定義。

### 例: コードリファクタリングAgent

`agents/specs/refactor-agent.yaml`:
```yaml
name: refactor-agent
version: "1.0"
description: コードリファクタリング専門Agent

capabilities:
  - code_analysis
  - refactoring
  - testing

preferences:
  style: functional
  error_handling: result
  max_iterations: 10
```

## セッション管理

### コマンドライン操作

```bash
# セッション一覧
miyabi sessions

# 特定セッションで再開
miyabi --session <session-id>

# Markdownエクスポート
miyabi sessions -m <session-id>

# JSONエクスポート
miyabi sessions -e <session-id>

# セッション削除
miyabi sessions -d <session-id>
```

### セッションファイル形式

セッションは `sessions/` に JSON 形式で保存:

```json
{
  "id": "uuid",
  "created_at": "2025-01-01T00:00:00Z",
  "updated_at": "2025-01-01T01:00:00Z",
  "model": "claude-sonnet-4-5-20250929",
  "messages": [...]
}
```

## プロジェクトルール (.miyabirules)

プロジェクトルートに `.miyabirules` ファイルを配置して、プロジェクト固有のルールを定義:

```yaml
version: 1

rules:
  - name: "no-unwrap"
    pattern: ".unwrap()"
    suggestion: "Use ? operator or proper error handling"
    file_extensions: ["rs"]
    severity: "warning"
    enabled: true

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
    clippy_strict: true

settings:
  auto_format: true
  max_file_size: 1048576
```

## 環境変数

設定ファイルの値は環境変数でオーバーライド可能:

```bash
# 必須
export ANTHROPIC_API_KEY="sk-ant-..."

# オプション
export MIYABI_MODEL="claude-sonnet-4-5-20250929"
export MIYABI_MAX_TOKENS="16384"
export MIYABI_THINKING="true"
export MIYABI_THEME="dracula"
```

## Core Library機能の利用

### キャッシュ

LLMレスポンスのキャッシュで高速化:

```rust
use miyabi_core::{create_llm_cache, LLMCacheKey};

let cache = create_llm_cache(); // 1時間TTL
```

### フィーチャーフラグ

機能の段階的ロールアウト:

```rust
use miyabi_core::FeatureFlagManager;

let flags = FeatureFlagManager::new();
flags.set_flag("new_ui", true);
```

### Circuit Breaker

API障害時の保護:

```rust
use miyabi_core::CircuitBreaker;

let breaker = CircuitBreaker::default();
// 5回失敗で開く、60秒後に半開
```

### プラグイン

カスタム機能の追加:

```rust
use miyabi_core::{Plugin, PluginManager};

let manager = PluginManager::new();
manager.register(Box::new(MyPlugin))?;
```

## トラブルシューティング

### API接続エラー

1. API Keyの確認: `echo $ANTHROPIC_API_KEY`
2. ネットワーク接続確認
3. タイムアウト値の増加

### セッション破損

1. `sessions/` から該当ファイルを削除
2. `miyabi sessions` で確認

### 設定が反映されない

1. 設定ファイルの構文確認: `cat config.toml`
2. 環境変数の確認
3. CLIオプションの優先順位確認

## 使い方

```bash
# TUI起動
miyabi

# バージョン情報
miyabi version

# ステータス確認
miyabi status

# ヘルプ
miyabi --help
```
