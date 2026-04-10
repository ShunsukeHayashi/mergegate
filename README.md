# MergeGate

MergeGate is a deterministic execution protocol for AI-assisted development.
It is the execution layer for GitNexus-style code intelligence: GitNexus helps agents understand what will break, and MergeGate makes them execute that work in a safe, verifiable order.

In short:

- `GitNexus`: understand the codebase
- `MergeGate`: execute changes safely

The project currently supports both `miyabi` and `mergegate` as CLI entrypoints. `miyabi` remains the legacy/default command, and `mergegate` is the product-aligned alias.

This repository has two common entry points:

- `miyabi tui` / `mergegate tui`: interactive terminal assistant
- `miyabi gate` / `mergegate gate`: deterministic task execution and file-lock workflow for repo work

If you arrived here because of deterministic execution, task locks, PR gates, or agent-safe repo work, start with `mergegate gate` or `miyabi gate`, not the TUI.

## 60-Second Setup

```bash
# 1. Clone and build
git clone https://github.com/ShunsukeHayashi/mergegate.git
cd mergegate
cargo build --release

# 2. Set API key
export ANTHROPIC_API_KEY="sk-ant-..."

# 3. Run
./target/release/mergegate tui
```

That's it. Start with the TUI, or jump straight into `mergegate gate` for repo workflow control.

---

## Why MergeGate

- **Execution layer for code intelligence** - Pair blast-radius understanding with deterministic execution, not chat-only automation
- **Deterministic task execution** - Register, analyze, lock, review, and merge in a verifiable order
- **Repo-safe agent workflow** - File locks and gate checks reduce accidental overlap and unsafe edits
- **Interactive TUI** - Terminal assistant for chat, prompts, and agent execution
- **Agent mode** - Autonomous execution with tool approval
- **Session management** - Persist and resume conversations
- **Configurable** - TOML-based configuration

## Positioning

MergeGate was designed for teams that already believe code understanding is not enough.
Knowing the blast radius of a change is only half of the problem. Agents also need a protocol for when work starts, which files are locked, what must be recorded before editing, and what counts as done.

That is the role split:

- `GitNexus`: query, context, impact, detect-changes, rename, wiki, graph exploration
- `MergeGate`: register, impact record, assign, file lock, branch, PR, merge, completion discipline

If GitNexus answers "what is this change connected to?", MergeGate answers "how do we carry it through safely?"

## Installation

### Prerequisites

- Rust 1.70+
- Anthropic API key

### Build from Source

```bash
git clone https://github.com/ShunsukeHayashi/mergegate.git
cd mergegate
cargo build --release
```

Binary will be at `target/release/miyabi` and `target/release/mergegate`.

## Quick Start

Choose the path that matches what you want to do.

### A. Use MergeGate as a terminal assistant

Run the TUI after configuring your API key.

#### 1. Generate Config

```bash
./target/release/miyabi init
```

This creates `~/.miyabi/config.toml`.

#### 2. Set API Key

Edit `~/.miyabi/config.toml`:

```toml
[api]
api_key = "sk-ant-..."
```

Or use environment variable:

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

#### 3. Launch TUI

```bash
./target/release/miyabi tui
# or simply
./target/release/miyabi
```

### B. Use MergeGate as a task execution gate

If you are working inside a repository and want task registration, impact tracking, file locks, and PR/merge recording, use `mergegate gate` or `miyabi gate`.

#### 1. Check whether the repo is already initialized

```bash
./target/release/miyabi gate status
```

If you see `tasks: 0`, that means the ledger exists but is currently empty.

#### 2. Initialize MergeGate for this repo if needed

```bash
./target/release/miyabi gate init
```

This creates `project_memory/tasks.json` and prepares the repo for gate-managed work.

#### 3. Read the built-in workflow guide

```bash
./target/release/miyabi gate guide
```

#### 4. Start a task

```bash
./target/release/miyabi gate register --issue 123 --title "Fix login redirect"
./target/release/miyabi gate impact issue-123 --risk medium --symbols 3
./target/release/miyabi gate assign issue-123 --agent codex --node macbook --files "src/auth.rs"
```

Typical flow:

1. `register`
2. `impact`
3. `assign`
4. implement
5. `branch`
6. `pr`
7. `merge` or `manual-complete`

## Usage

`MergeGate` is the product and documentation name.
Both `miyabi` and `mergegate` work today.

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
miyabi gate status        # Show task ledger status
miyabi gate init          # Initialize MergeGate in the current repo
miyabi gate guide         # Show the full gate workflow guide
mergegate gate status     # Same gate workflow with product-aligned command
mergegate tui             # Same TUI with product-aligned command
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

The `mergegate-core` crate provides powerful utilities for building AI applications.

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
mergegate/
├── crates/
│   ├── mergegate-cli/      # CLI entry point
│   ├── mergegate-core/     # Core library
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
│   └── mergegate-tui/      # TUI implementation
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

## Troubleshooting

### Common Issues

#### "API key not found"
```bash
# Check if environment variable is set
echo $ANTHROPIC_API_KEY

# Or add to config file
vim ~/.miyabi/config.toml
# [api]
# api_key = "sk-ant-..."
```

#### "Device not configured" error on TUI start
This usually means the terminal doesn't support the required features. Try:
```bash
# Use a different terminal emulator (iTerm2, Alacritty, etc.)
# Or run with explicit terminal type
TERM=xterm-256color ./target/release/miyabi tui
```

#### Build errors
```bash
# Make sure Rust is up to date
rustup update

# Clean and rebuild
cargo clean
cargo build --release
```

#### Session not saving
```bash
# Check permissions on sessions directory
ls -la ~/.miyabi/sessions/

# Create if missing
mkdir -p ~/.miyabi/sessions
```

### Debug Mode

For verbose logging:
```bash
RUST_LOG=debug ./target/release/miyabi tui
```

### Getting Help

- Check `miyabi --help` for CLI options
- Press `F1` in TUI for keybindings
- Open an issue: https://github.com/ShunsukeHayashi/mergegate/issues

## License

MIT

---

Built with Rust, Ratatui, and Claude API.
