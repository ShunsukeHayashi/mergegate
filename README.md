# Miyabi CLI

A powerful terminal-based AI assistant with TUI (Terminal User Interface) built in Rust.

## Features

- **Interactive TUI** - Beautiful terminal interface with markdown rendering
- **Chat Mode** - Conversational AI assistant
- **Agent Mode** - Autonomous task execution with tool approval
- **Session Management** - Persist and resume conversations
- **Configurable** - TOML-based configuration
- **Extended Thinking** - Support for Claude 4.5+ extended thinking

## Installation

### Prerequisites

- Rust 1.70+
- Anthropic API key

### Build from Source

```bash
git clone https://github.com/ShunsukeHayashi/miyabi-cli-standalone.git
cd miyabi-cli-standalone
cargo build --release
```

Binary will be at `target/release/miyabi`.

## Quick Start

### 1. Generate Config

```bash
./target/release/miyabi init
```

This creates `~/.miyabi/config.toml`.

### 2. Set API Key

Edit `~/.miyabi/config.toml`:

```toml
[api]
api_key = "sk-ant-..."
```

Or use environment variable:

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

### 3. Launch TUI

```bash
./target/release/miyabi tui
# or simply
./target/release/miyabi
```

## Usage

### CLI Commands

```bash
miyabi                    # Start TUI (default)
miyabi tui                # Start TUI explicitly
miyabi status             # Show status
miyabi init               # Generate default config
miyabi sessions           # List sessions
miyabi sessions -d <id>   # Delete session
miyabi sessions -e <id>   # Export session to JSON
miyabi version            # Show version info
miyabi agent <prompt>     # Run autonomous agent
```

### CLI Options

```bash
miyabi --model claude-sonnet-4-5-20250929    # Override model
miyabi --max-tokens 16384                     # Override max tokens
miyabi --thinking                             # Enable extended thinking
miyabi --config /path/to/config.toml          # Custom config
miyabi --session <id>                         # Resume session
```

### TUI Keybindings

| Key | Action |
|-----|--------|
| `Enter` | Send message |
| `Ctrl+P` | Command palette |
| `F1` | Help |
| `Esc` | Close overlay / Cancel |
| `Ctrl+C` | Quit |
| `j/k` | Scroll down/up |
| `Page Up/Down` | Page scroll |

## Configuration

Config file: `~/.miyabi/config.toml`

```toml
[api]
api_key = "sk-ant-..."                        # Anthropic API key
model = "claude-sonnet-4-5-20250929"          # Model to use
max_tokens = 8192                             # Max response tokens
thinking = false                              # Extended thinking
system_prompt = "You are a helpful assistant" # System prompt
timeout_secs = 120                            # Request timeout
max_retries = 3                               # Retry attempts

[ui]
show_sidebar = false      # Show sidebar
show_status_bar = true    # Show status bar
show_breadcrumb = true    # Show breadcrumb
theme = "tokyo-night"     # Color theme
vim_mode = false          # Vim keybindings
show_line_numbers = true  # Line numbers in code

[session]
auto_save = true          # Auto-save sessions
auto_save_interval = 30   # Save interval (seconds)
max_sessions = 100        # Max stored sessions

[tools]
enable_bash = true        # Enable bash tool
enable_file_tools = true  # Enable file read/write
enable_search_tools = true # Enable glob/grep
auto_approve_low_risk = false # Auto-approve safe tools
bash_timeout = 120        # Bash command timeout
```

### Environment Variables

Override config with environment variables:

```bash
ANTHROPIC_API_KEY=...     # API key
MIYABI_MODEL=...          # Model name
MIYABI_MAX_TOKENS=...     # Max tokens
MIYABI_THINKING=true      # Enable thinking
```

## Agent Mode

Run autonomous tasks with tool execution:

```bash
# Basic usage
miyabi agent "Create a hello world Python script"

# With options
miyabi agent "Refactor the utils module" \
  --max-iterations 20 \
  --auto-approve \
  --system "You are an expert Rust developer"
```

Agent mode features:
- Autonomous tool execution (bash, file operations, search)
- Approval workflow for risky operations
- Iteration limits for safety
- JSON output format option

## Core Library Features

The `miyabi-core` crate provides powerful utilities for building AI applications.

### Git Utilities

```rust
use miyabi_core::{find_git_root, get_current_branch, is_in_git_repo};

// Find repository root
let root = find_git_root()?;

// Get current branch
let branch = get_current_branch(&root)?;

// Check if in git repo
if is_in_git_repo(&path) {
    println!("In a git repository");
}
```

### Logger System

```rust
use miyabi_core::{init_logger, LogLevel, LogFormat};

// Initialize with defaults
init_logger();

// Or with custom config
use miyabi_core::LoggerConfig;
let config = LoggerConfig {
    level: LogLevel::Debug,
    format: LogFormat::Json,
    ..Default::default()
};
init_logger_with_config(config);
```

### Retry with Backoff

```rust
use miyabi_core::{retry_with_backoff, BackoffRetryConfig};

let config = BackoffRetryConfig::default();
let result = retry_with_backoff(config, || async {
    // Your operation that might fail
    make_api_call().await
}).await?;
```

### Circuit Breaker

```rust
use miyabi_core::{CircuitBreaker, CircuitState};

let breaker = CircuitBreaker::default();

let result = breaker.call(|| {
    Box::pin(async {
        // Your operation
        Ok::<_, std::io::Error>(())
    })
}).await;

// Check state
match breaker.state().await {
    CircuitState::Closed => println!("Normal operation"),
    CircuitState::Open => println!("Circuit open - blocking calls"),
    CircuitState::HalfOpen => println!("Testing recovery"),
}
```

### Rules System (.miyabirules)

Support for project-specific rules via `.miyabirules` files:

```yaml
# .miyabirules
version: 1
rules:
  - name: "no-unwrap"
    pattern: ".unwrap()"
    suggestion: "Use ? operator or proper error handling"
    file_extensions: ["rs"]
    severity: "warning"

agent_preferences:
  codegen:
    style: "functional"
    error_handling: "result"
```

```rust
use miyabi_core::{RulesLoader, MiyabiRules};

let loader = RulesLoader::new(project_root);
let rules = loader.load_or_default()?;

// Get rules for a file
let applicable = rules.rules_for_file(Path::new("src/main.rs"));
```

### Cache System

```rust
use miyabi_core::{create_llm_cache, create_api_cache, LLMCacheKey};

// LLM response cache (1 hour TTL)
let cache = create_llm_cache();
let key = LLMCacheKey::new("prompt", "model", Some(0.7));

cache.insert(key.clone(), "response".to_string()).await;
let cached = cache.get(&key).await;

// API cache (30 min TTL)
let api_cache = create_api_cache();
```

### Plugin System

```rust
use miyabi_core::{Plugin, PluginManager, PluginMetadata, PluginContext, PluginResult};
use anyhow::Result;

struct MyPlugin;

impl Plugin for MyPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "my-plugin".to_string(),
            version: "1.0.0".to_string(),
            description: Some("My custom plugin".to_string()),
            author: None,
        }
    }

    fn init(&mut self) -> Result<()> {
        Ok(())
    }

    fn execute(&self, ctx: &PluginContext) -> Result<PluginResult> {
        Ok(PluginResult {
            success: true,
            message: Some("Done".to_string()),
            data: None,
        })
    }
}

// Register and execute
let manager = PluginManager::new();
manager.register(Box::new(MyPlugin))?;
let result = manager.execute("my-plugin", &PluginContext::default())?;
```

### Feature Flags

```rust
use miyabi_core::{FeatureFlagManager, FeatureFlag};

let flags = FeatureFlagManager::new();

// Simple flag
flags.set_flag("new_feature", true);

if flags.is_enabled("new_feature") {
    // Use new feature
}

// With rollout percentage
flags.set_flag_with_options(
    "beta_feature",
    true,
    Some("Beta testing".to_string()),
    Some(0.5), // 50% rollout
);

// Load from config
use std::collections::HashMap;
let mut config = HashMap::new();
config.insert("flag1".to_string(), true);
flags.load_from_map(config);
```

## Project Structure

```
miyabi-cli-standalone/
├── crates/
│   ├── miyabi-cli/         # CLI entry point
│   ├── miyabi-core/        # Core library
│   │   ├── agent.rs        # Agent system
│   │   ├── anthropic.rs    # Anthropic API client
│   │   ├── cache.rs        # TTL cache system
│   │   ├── config.rs       # Configuration
│   │   ├── error_policy.rs # Circuit breaker
│   │   ├── feature_flags.rs # Feature flag management
│   │   ├── git.rs          # Git utilities
│   │   ├── logger.rs       # Logging system
│   │   ├── plugin.rs       # Plugin system
│   │   ├── retry.rs        # Retry with backoff
│   │   ├── rules.rs        # .miyabirules support
│   │   ├── session.rs      # Session management
│   │   ├── tools.rs        # Built-in tools
│   │   └── ...
│   └── miyabi-tui/         # TUI implementation
│       ├── app.rs          # Main application
│       ├── views.rs        # UI views
│       └── ...
├── .miyabi/                # Local config
├── Cargo.toml              # Workspace config
└── README.md
```

## Development

### Build

```bash
cargo build              # Debug build
cargo build --release    # Release build
```

### Test

```bash
cargo test --all         # Run all tests
```

### Lint

```bash
cargo clippy --all-targets -- -D warnings
cargo fmt --all --check
```

## Models

Supported models:
- `claude-sonnet-4-5-20250929` (default)
- `claude-haiku-4-5-20251001`
- `claude-sonnet-4-20250514`

## License

MIT

---

Built with Rust, Ratatui, and Claude API.
