//! Miyabi CLI - Main entry point

use clap::{Parser, Subcommand, ValueEnum};
use miyabi_core::{FeatureFlagManager, RulesLoader};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

/// Global feature flags manager
static FEATURE_FLAGS: std::sync::OnceLock<FeatureFlagManager> = std::sync::OnceLock::new();

/// Get the global feature flags manager
pub fn feature_flags() -> &'static FeatureFlagManager {
    FEATURE_FLAGS.get_or_init(|| {
        let manager = FeatureFlagManager::new();
        // Default feature flags
        manager.set_flag("extended_thinking", true);
        manager.set_flag("auto_save_sessions", true);
        manager.set_flag("syntax_highlighting", true);
        manager.set_flag("vim_mode", false);
        manager
    })
}

#[derive(Parser)]
#[command(name = "miyabi")]
#[command(author, version, about = "Miyabi - Autonomous AI Development Framework", long_about = None)]
struct Cli {
    /// Model to use (overrides config)
    #[arg(short, long)]
    model: Option<String>,

    /// Maximum tokens for responses (overrides config)
    #[arg(long)]
    max_tokens: Option<u32>,

    /// Enable Extended Thinking (Claude 4.5+)
    #[arg(long)]
    thinking: bool,

    /// Path to config file
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Session ID to load on startup
    #[arg(short, long)]
    session: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the TUI interface
    Tui,
    /// Show status
    Status,
    /// Generate default config file
    Init,
    /// Manage sessions
    Sessions {
        /// Delete a session by ID
        #[arg(short, long)]
        delete: Option<String>,
        /// Export a session to JSON file
        #[arg(short, long)]
        export: Option<String>,
        /// Export a session to Markdown file
        #[arg(short, long)]
        markdown: Option<String>,
    },
    /// Show version and system information
    Version,
    /// Show project rules (.miyabirules)
    Rules {
        /// Show detailed rule information
        #[arg(short, long)]
        verbose: bool,
    },
    /// Run agent with a prompt (autonomous execution)
    Agent {
        /// The prompt to execute
        prompt: String,
        /// Maximum iterations (default: 10)
        #[arg(long, default_value = "10")]
        max_iterations: usize,
        /// Auto-approve all tool executions
        #[arg(long)]
        auto_approve: bool,
        /// Output format: text or json
        #[arg(long, default_value = "text")]
        format: String,
        /// System prompt for the agent
        #[arg(long)]
        system: Option<String>,
    },
    /// Deterministic Task Protocol gate controls
    Gate {
        /// Output format
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
        /// Path to the task ledger JSON file
        #[arg(long, default_value = "project_memory/tasks.json")]
        store_path: PathBuf,
        /// Gate subcommand
        #[command(subcommand)]
        command: GateCommand,
    },
    /// OpenClaw integration - control OpenClaw agents
    Openclaw {
        /// OpenClaw subcommand
        #[command(subcommand)]
        command: OpenclawCommand,
    },
    /// Collaborator canvas control via collab CLI
    Collab {
        /// Canvas subcommand
        #[command(subcommand)]
        command: CollabCommand,
    },
}

#[derive(Clone, Debug, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Clone, Debug, ValueEnum)]
enum CompletionModeArg {
    GithubPr,
    Manual,
    ExternalOp,
}

#[derive(Clone, Debug, ValueEnum)]
enum ImpactRiskArg {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Subcommand)]
enum GateCommand {
    /// Register a task in the execution ledger
    Register {
        /// GitHub issue number
        #[arg(long, default_value_t = 0)]
        issue: u64,
        /// Task title
        #[arg(long)]
        title: String,
        /// Explicit task ID (defaults to issue-N or slugified title)
        #[arg(long)]
        task_id: Option<String>,
        /// Hard dependencies (comma separated)
        #[arg(long, value_delimiter = ',')]
        dependencies: Vec<String>,
        /// Soft dependencies (comma separated)
        #[arg(long, value_delimiter = ',')]
        soft_dependencies: Vec<String>,
        /// Priority score
        #[arg(long, default_value_t = 0)]
        priority: u32,
        /// Completion mode
        #[arg(long, value_enum, default_value_t = CompletionModeArg::GithubPr)]
        completion_mode: CompletionModeArg,
    },
    /// Show status for one task or the whole ledger
    Status {
        /// Optional task ID
        task_id: Option<String>,
    },
    /// Assign a task and acquire file locks
    Assign {
        task_id: String,
        #[arg(long)]
        agent: String,
        #[arg(long)]
        node: String,
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        files: Vec<String>,
    },
    /// Record impact analysis
    Impact {
        task_id: String,
        #[arg(long, value_enum)]
        risk: ImpactRiskArg,
        #[arg(long)]
        approve: bool,
        #[arg(long)]
        symbols: usize,
        #[arg(long, value_delimiter = ',')]
        depth1: Vec<String>,
        #[arg(long)]
        analyzed_commit: Option<String>,
        #[arg(long)]
        input_hash: Option<String>,
    },
    /// Record branch creation
    Branch { task_id: String, name: String },
    /// Record PR creation
    Pr { task_id: String, number: u64 },
    /// Record merge verification
    Merge { task_id: String, sha: String },
    /// List active locks
    Locks,
    /// Show DAG levels
    Dag,
    /// Show dispatchable tasks
    Dispatchable,
}

/// Collab canvas subcommands — wraps the collab CLI at ~/.local/bin/collab
#[derive(Subcommand)]
enum CollabCommand {
    /// List tiles on the canvas
    List {
        /// Output as JSON array
        #[arg(long)]
        json: bool,
        /// Filter by tile type (note, code, term, image, graph)
        #[arg(long)]
        r#type: Option<String>,
        /// Count only
        #[arg(long)]
        count: bool,
    },
    /// Add a tile to the canvas
    Add {
        /// Tile type (note, code, term, image, graph)
        tile_type: String,
        /// File to attach (required for note/code)
        #[arg(long)]
        file: Option<String>,
        /// Position in grid units "x,y"
        #[arg(long)]
        pos: Option<String>,
        /// Size in grid units "w,h"
        #[arg(long)]
        size: Option<String>,
        /// Skip if tile with same file already exists
        #[arg(long)]
        idempotent: bool,
    },
    /// Remove a tile from the canvas
    Rm {
        /// Tile ID to remove
        tile_id: String,
    },
    /// Move a tile to a new position
    Move {
        /// Tile ID to move
        tile_id: String,
        /// New position in grid units "x,y"
        #[arg(long)]
        pos: String,
    },
    /// Resize a tile
    Resize {
        /// Tile ID to resize
        tile_id: String,
        /// New size in grid units "w,h"
        #[arg(long)]
        size: String,
    },
    /// Get or set the canvas viewport
    Viewport {
        /// Set pan position "x,y"
        #[arg(long)]
        pan: Option<String>,
        /// Set zoom level (e.g. 1.0)
        #[arg(long)]
        zoom: Option<f64>,
    },
    /// Show Collaborator connection status
    Status,
}

#[derive(Subcommand)]
enum OpenclawCommand {
    /// List all available agents
    Agents,
    /// Show OpenClaw status
    Status,
    /// Show detailed help for OpenClaw commands
    Help,
    /// Send a message to an agent
    Send {
        /// Agent name (e.g., maestro, kade, sakura)
        agent: String,
        /// Message to send
        message: String,
    },
    /// Broadcast a message to all agents
    Broadcast {
        /// Message to broadcast
        message: String,
    },
    /// Broadcast to a specific society
    BroadcastSociety {
        /// Society name (core, investment, content, marketing)
        society: String,
        /// Message to broadcast
        message: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Tui) | None => {
            // Run TUI
            use crossterm::{
                event::{DisableMouseCapture, EnableMouseCapture},
                execute,
                terminal::{
                    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
                },
            };
            use miyabi_core::config::Config;
            use miyabi_tui::App;
            use ratatui::prelude::*;
            use std::io;

            // Load config (from custom path or default)
            let mut config = if let Some(config_path) = &cli.config {
                Config::load_from(config_path)?
            } else {
                Config::load().unwrap_or_default()
            };

            // Apply CLI overrides
            if let Some(model) = &cli.model {
                config.api.model = model.clone();
            }
            if let Some(max_tokens) = cli.max_tokens {
                config.api.max_tokens = max_tokens;
            }
            if cli.thinking {
                config.api.thinking = true;
            }

            enable_raw_mode()?;
            let mut stdout = io::stdout();
            execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
            let backend = CrosstermBackend::new(stdout);
            let mut terminal = Terminal::new(backend)?;

            let mut app = App::with_config(config);

            // Load session if specified
            if let Some(session_id) = &cli.session {
                if let Err(e) = app.load_session(session_id) {
                    eprintln!("Warning: Failed to load session {}: {}", session_id, e);
                }
            }

            let res = app.run(&mut terminal).await;

            disable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )?;
            terminal.show_cursor()?;

            if let Err(err) = res {
                eprintln!("Error: {}", err);
            }
        }
        Some(Commands::Status) => {
            use miyabi_core::config::Config;

            let config = Config::load().unwrap_or_default();

            println!("Miyabi Status: Ready");
            println!();
            println!("Config:   {}", Config::default_path().display());
            println!("Sessions: {}", config.sessions_dir().display());
            println!("Model:    {}", config.api.model);
            println!();

            // Load and show rules info
            let cwd = std::env::current_dir().unwrap_or_default();
            let loader = RulesLoader::new(cwd);
            match loader.load() {
                Ok(Some(rules)) => {
                    println!("Rules:    {} rules loaded", rules.rules.len());
                    if !rules.agent_preferences.is_empty() {
                        println!(
                            "Agents:   {} agent preferences",
                            rules.agent_preferences.len()
                        );
                    }
                }
                Ok(None) => {
                    println!("Rules:    No .miyabirules found");
                }
                Err(e) => {
                    println!("Rules:    Error loading - {}", e);
                }
            }

            // Show feature flags
            let flags = feature_flags();
            let all_flags = flags.get_all_flags();
            let enabled_count = all_flags.iter().filter(|f| f.enabled).count();
            println!("Flags:    {}/{} enabled", enabled_count, all_flags.len());
        }
        Some(Commands::Init) => {
            use miyabi_core::config::Config;
            let path = Config::generate_default()?;
            println!("Generated default config at: {:?}", path);
        }
        Some(Commands::Sessions {
            delete,
            export,
            markdown,
        }) => {
            use miyabi_core::anthropic::{ContentBlock, Role};
            use miyabi_core::config::Config;
            use miyabi_core::session::SessionStorage;

            let config = Config::load().unwrap_or_default();
            let storage = SessionStorage::new(config.sessions_dir());

            if let Some(id) = delete {
                // Delete session
                match storage.delete(&id) {
                    Ok(_) => println!("Deleted session: {}", id),
                    Err(e) => eprintln!("Failed to delete session {}: {}", id, e),
                }
            } else if let Some(id) = export {
                // Export session to JSON
                match storage.load(&id) {
                    Ok(session) => {
                        let filename = format!("{}.json", id);
                        let json = serde_json::to_string_pretty(&session)?;
                        std::fs::write(&filename, json)?;
                        println!("Exported session to: {}", filename);
                    }
                    Err(e) => eprintln!("Failed to load session {}: {}", id, e),
                }
            } else if let Some(id) = markdown {
                // Export session to Markdown
                match storage.load(&id) {
                    Ok(session) => {
                        let filename = format!("{}.md", id);
                        let mut md = String::new();

                        // Header
                        md.push_str(&format!("# Session: {}\n\n", session.title));
                        md.push_str(&format!("**Model**: {}\n", session.model));
                        md.push_str(&format!(
                            "**Date**: {}\n",
                            session.created_at.format("%Y-%m-%d %H:%M")
                        ));
                        md.push_str(&format!("**Tokens**: {}\n\n", session.tokens_used));
                        md.push_str("---\n\n");

                        // Messages
                        for message in &session.messages {
                            let role = match message.role {
                                Role::User => "You",
                                Role::Assistant => "Assistant",
                            };

                            md.push_str(&format!("## {}\n\n", role));

                            for content in &message.content {
                                match content {
                                    ContentBlock::Text { text } => {
                                        md.push_str(text);
                                        md.push_str("\n\n");
                                    }
                                    ContentBlock::ToolUse { name, input, .. } => {
                                        md.push_str(&format!(
                                            "**Tool**: {}\n```json\n{}\n```\n\n",
                                            name, input
                                        ));
                                    }
                                    ContentBlock::ToolResult { content, .. } => {
                                        md.push_str(&format!(
                                            "**Result**:\n```\n{}\n```\n\n",
                                            content
                                        ));
                                    }
                                }
                            }
                        }

                        std::fs::write(&filename, md)?;
                        println!("Exported session to: {}", filename);
                    }
                    Err(e) => eprintln!("Failed to load session {}: {}", id, e),
                }
            } else {
                // List all sessions
                match storage.list() {
                    Ok(sessions) => {
                        if sessions.is_empty() {
                            println!("No sessions found.");
                        } else {
                            println!(
                                "{:<36} {:<20} {:<8} {:<10} Updated",
                                "ID", "Title", "Messages", "Tokens"
                            );
                            println!("{}", "-".repeat(90));
                            for session in sessions {
                                let updated = session.updated_at.format("%Y-%m-%d %H:%M");
                                println!(
                                    "{:<36} {:<20} {:<8} {:<10} {}",
                                    session.id,
                                    truncate_str(&session.title, 18),
                                    session.messages.len(),
                                    session.tokens_used,
                                    updated
                                );
                            }
                        }
                    }
                    Err(e) => eprintln!("Failed to list sessions: {}", e),
                }
            }
        }
        Some(Commands::Version) => {
            use miyabi_core::config::Config;

            let config = Config::load().unwrap_or_default();

            println!("Miyabi v{}", env!("CARGO_PKG_VERSION"));
            println!();
            println!("Model:    {}", config.api.model);
            println!("Config:   {}", Config::default_path().display());
            println!("Sessions: {}", config.sessions_dir().display());
            println!();
            println!(
                "Platform: {} ({})",
                std::env::consts::OS,
                std::env::consts::ARCH
            );
        }
        Some(Commands::Rules { verbose }) => {
            let cwd = std::env::current_dir().unwrap_or_default();
            let loader = RulesLoader::new(cwd.clone());

            match loader.load() {
                Ok(Some(rules)) => {
                    println!("Project Rules (.miyabirules)");
                    println!("============================");
                    println!();

                    if rules.rules.is_empty() {
                        println!("No rules defined.");
                    } else {
                        println!("Rules ({}):", rules.rules.len());
                        for rule in &rules.rules {
                            let status = if rule.enabled { "✓" } else { "✗" };
                            let severity = match rule.severity.as_str() {
                                "error" => "🔴",
                                "warning" => "🟡",
                                _ => "🔵",
                            };
                            println!(
                                "  {} {} {} - {}",
                                status, severity, rule.name, rule.suggestion
                            );

                            if verbose {
                                if let Some(pattern) = &rule.pattern {
                                    println!("      Pattern: {}", pattern);
                                }
                                if !rule.file_extensions.is_empty() {
                                    println!(
                                        "      Extensions: {}",
                                        rule.file_extensions.join(", ")
                                    );
                                }
                                println!();
                            }
                        }
                    }

                    if !rules.agent_preferences.is_empty() {
                        println!();
                        println!("Agent Preferences ({}):", rules.agent_preferences.len());
                        for (agent, prefs) in &rules.agent_preferences {
                            println!("  {}:", agent);
                            if let Some(style) = &prefs.style {
                                println!("    Style: {}", style);
                            }
                            if let Some(handling) = &prefs.error_handling {
                                println!("    Error Handling: {}", handling);
                            }
                            if let Some(score) = prefs.min_score {
                                println!("    Min Score: {}", score);
                            }
                        }
                    }

                    if verbose && !rules.settings.is_empty() {
                        println!();
                        println!("Settings:");
                        for (key, value) in &rules.settings {
                            println!("  {}: {}", key, value);
                        }
                    }
                }
                Ok(None) => {
                    println!(
                        "No .miyabirules file found in {} or parent directories.",
                        cwd.display()
                    );
                    println!();
                    println!("Create a .miyabirules file to define project-specific rules.");
                    println!("See: miyabi --help for more information.");
                }
                Err(e) => {
                    eprintln!("Error loading rules: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Agent {
            prompt,
            max_iterations,
            auto_approve,
            format,
            system,
        }) => {
            use miyabi_core::{
                config::Config, Agent, AgentConfig, AgentEvent, AnthropicClient, ExecutorRegistry,
            };
            use tokio::sync::mpsc;

            // Load config
            let mut config = if let Some(config_path) = &cli.config {
                Config::load_from(config_path)?
            } else {
                Config::load().unwrap_or_default()
            };

            // Apply CLI overrides
            if let Some(model) = &cli.model {
                config.api.model = model.clone();
            }
            if let Some(max_tokens) = cli.max_tokens {
                config.api.max_tokens = max_tokens;
            }
            if cli.thinking {
                config.api.thinking = true;
            }

            // Get API key
            let api_key = config
                .api
                .api_key
                .clone()
                .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
                .ok_or_else(|| {
                    anyhow::anyhow!("No API key found. Set ANTHROPIC_API_KEY or add to config.")
                })?;

            // Create client
            let client = AnthropicClient::new(api_key)?
                .with_model(&config.api.model)
                .with_max_tokens(config.api.max_tokens)
                .with_thinking(config.api.thinking);

            // Create executor registry with standard tools
            let registry = ExecutorRegistry::with_standard_tools();

            // Configure agent
            let agent_config = AgentConfig {
                max_iterations,
                max_tokens_per_turn: config.api.max_tokens,
                require_approval: !auto_approve,
                auto_approve_patterns: if auto_approve {
                    vec![
                        "read".to_string(),
                        "glob".to_string(),
                        "grep".to_string(),
                        "write".to_string(),
                        "edit".to_string(),
                        "bash".to_string(),
                    ]
                } else {
                    vec!["read".to_string(), "glob".to_string(), "grep".to_string()]
                },
                ..Default::default()
            };

            // Create agent
            let mut agent = Agent::new(client, registry).with_config(agent_config);

            // Set system prompt
            if let Some(sys) = system {
                agent = agent.with_system_prompt(sys);
            } else if let Some(sys) = config.api.system_prompt {
                agent = agent.with_system_prompt(sys);
            }

            // Create event channel for progress
            let (tx, mut rx) = mpsc::channel(100);
            let agent = agent.with_event_channel(tx);

            // Spawn agent execution
            let agent_handle = tokio::spawn(async move { agent.run(&prompt).await });

            // Process events
            while let Some(event) = rx.recv().await {
                match &event {
                    AgentEvent::Started { prompt } => {
                        if format != "json" {
                            eprintln!("🚀 Agent started with prompt: {}", truncate_str(prompt, 50));
                        }
                    }
                    AgentEvent::Thinking { iteration } => {
                        if format != "json" {
                            eprintln!("💭 Iteration {}", iteration + 1);
                        }
                    }
                    AgentEvent::ToolDetected { name, .. } => {
                        if format != "json" {
                            eprintln!("🔧 Tool detected: {}", name);
                        }
                    }
                    AgentEvent::ToolExecuting { name, .. } => {
                        if format != "json" {
                            eprintln!("⚡ Executing: {}", name);
                        }
                    }
                    AgentEvent::ToolCompleted { name, .. } => {
                        if format != "json" {
                            eprintln!("✅ Completed: {}", name);
                        }
                    }
                    AgentEvent::ToolFailed { name, error } => {
                        if format != "json" {
                            eprintln!("❌ Failed {}: {}", name, error);
                        }
                    }
                    AgentEvent::Completed { result } => {
                        if format == "json" {
                            println!("{}", serde_json::to_string_pretty(&result)?);
                        } else {
                            println!("\n{}", result.output);
                            eprintln!(
                                "\n📊 Stats: {} iterations, {} tool calls, {} tokens",
                                result.iterations, result.tool_calls, result.total_tokens
                            );
                        }
                    }
                    AgentEvent::Failed { error } => {
                        eprintln!("❌ Agent failed: {}", error);
                    }
                    _ => {}
                }
            }

            // Wait for agent to complete
            match agent_handle.await? {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Agent error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Gate {
            format,
            store_path,
            command,
        }) => {
            let code = handle_gate_command(&format, &store_path, command)?;
            std::process::exit(code);
        }
        Some(Commands::Openclaw { command }) => {
            use miyabi_core::openclaw::OpenClawClient;
            use std::env;

            // Get OpenClaw configuration
            let gateway_url = env::var("OPENCLAW_GATEWAY_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:18789".to_string());
            let token = env::var("OPENCLAW_TOKEN").unwrap_or_else(|_| {
                // Try to read from openclaw.json
                #[allow(unused_imports)]
                use std::fs;
                #[allow(unused_imports)]
                use std::path::PathBuf;

                let config_path = PathBuf::from(env::var("HOME").unwrap_or_default())
                    .join(".openclaw")
                    .join("openclaw.json");

                // Fallback: try to read token from config
                if let Ok(content) = fs::read_to_string(&config_path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(gateway) = json.get("gateway") {
                            if let Some(auth) = gateway.get("auth") {
                                if let Some(t) = auth.get("token") {
                                    return t.as_str().unwrap_or("").to_string();
                                }
                            }
                        }
                    }
                }

                String::new()
            });

            if token.is_empty() {
                eprintln!("❌ Error: OPENCLAW_TOKEN not set");
                eprintln!("  Set environment variable: export OPENCLAW_TOKEN=your_token");
                eprintln!("  Or add to ~/.miyabi/config.toml");
                return Ok(());
            }

            // Handle Status command separately to avoid borrowing issues
            if let OpenclawCommand::Status = command {
                let token_display = if token.len() > 4 {
                    format!("{}***", &token[..4])
                } else {
                    "***".to_string()
                };
                println!("📊 OpenClaw Status:");
                println!("  Gateway: {}", gateway_url);
                println!("  Token: {}", token_display);
                return Ok(());
            }

            let client = OpenClawClient::new(gateway_url, token);

            match command {
                OpenclawCommand::Agents => {
                    // List agents grouped by society
                    let agents = OpenClawClient::get_agents();
                    println!("🎭 Miyabi エージェント一覧 ({} agents):", agents.len());
                    println!();

                    // Group by society
                    let mut grouped: HashMap<&str, Vec<_>> = HashMap::new();
                    for agent in &agents {
                        grouped.entry(&agent.society).or_default().push(agent);
                    }

                    // Define society order
                    let society_order = vec!["Core", "Investment", "Content", "Marketing"];

                    for society in &society_order {
                        if let Some(society_agents) = grouped.get(*society) {
                            println!("【{} Society】", society);
                            for agent in society_agents {
                                println!("  {} {} ({})", agent.emoji, agent.name, agent.id);
                                println!("      Role: {}", agent.role);
                            }
                            println!();
                        }
                    }
                }
                OpenclawCommand::Send { agent, message } => {
                    // Send message
                    let resolved_agent = OpenClawClient::resolve_agent_alias(&agent);
                    match client.send(&resolved_agent, &message).await {
                        Ok(_msg) => {
                            println!("✓ メッセージを送信しました:");
                            println!("  Agent: {}", resolved_agent);
                            println!("  Message: {}", message);
                        }
                        Err(e) => {
                            eprintln!("❌ 送信エラー: {}", e);
                        }
                    }
                }
                OpenclawCommand::Broadcast { message } => {
                    // Broadcast message to all core agents
                    match client.broadcast(&message).await {
                        Ok(results) => {
                            println!("✓ ブロードキャストを送信しました:");
                            println!("  Message: {}", message);
                            println!();
                            for result in results {
                                println!("  {}", result);
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ ブロードキャストエラー: {}", e);
                        }
                    }
                }
                OpenclawCommand::BroadcastSociety { society, message } => {
                    // Broadcast to specific society
                    let society_agents = match society.to_lowercase().as_str() {
                        "core" => vec!["maestro", "kade", "sakura", "tsubaki", "botan", "nagare"],
                        "investment" => vec![
                            "scout",
                            "crystal",
                            "dealer",
                            "sentinel",
                            "architect",
                            "watchman",
                            "chart",
                            "fundy",
                            "scribe",
                        ],
                        "content" => vec![
                            "tweeter",
                            "pen",
                            "vidpro",
                            "artist",
                            "optimizer",
                            "scheduler",
                        ],
                        "marketing" => vec!["hiro", "kazoeru", "funnel", "adops"],
                        _ => {
                            eprintln!("❌ 不明なSociety: {}", society);
                            eprintln!("   利用可能: core, investment, content, marketing");
                            return Ok(());
                        }
                    };

                    println!("✓ {} Society にブロードキャスト:", society);
                    println!("  Message: {}", message);
                    println!();

                    for agent in society_agents {
                        match client.send(agent, &message).await {
                            Ok(_) => println!("  ✓ {}", agent),
                            Err(e) => eprintln!("  ❌ {}: {}", agent, e),
                        }
                    }
                }
                OpenclawCommand::Help => {
                    // Show detailed help
                    println!("📖 Miyabi OpenClaw CLI - 詳細ヘルプ");
                    println!();
                    println!("【基本コマンド】");
                    println!();
                    println!("  miyabi openclaw agents");
                    println!("      → 全エージェント一覧を表示 (Society別)");
                    println!();
                    println!("  miyabi openclaw status");
                    println!("      → OpenClaw Gatewayの状態を確認");
                    println!();
                    println!("  miyabi openclaw send <agent> <message>");
                    println!("      → 特定のエージェントにメッセージを送信");
                    println!();
                    println!("  miyabi openclaw broadcast <message>");
                    println!("      → 全コアエージェントにブロードキャスト");
                    println!();
                    println!("  miyabi openclaw broadcast-society <society> <message>");
                    println!("      → 特定のSocietyにブロードキャスト");
                    println!();
                    println!("【エージェントIDとエイリアス】");
                    println!();
                    println!("  Core Society (6):");
                    println!("    maestro    しきるん🎭 - shikirun, conductor, orchestrator");
                    println!("    kade       カエデ🍁 - kaede, creator, codegen, developer");
                    println!("    sakura     サクラ🌸 - reviewer, qa, critic");
                    println!("    tsubaki    ツバキ🌺 - integrator, pr-manager, merge-bot");
                    println!("    botan      ボタン🌼 - deployer, release-manager, deployment");
                    println!("    nagare     ながれるん🌊 - nagarerun, workflow, automation");
                    println!();
                    println!("  Investment Society (9):");
                    println!("    scout      スカウト🔍 - researcher, explorer");
                    println!("    crystal    クリスタル💎 - valuer, analyst");
                    println!("    dealer     ディーラー🎰 - trader, executor");
                    println!("    sentinel   センチネル🛡️ - risk-manager, guardian-rm");
                    println!("    architect  アーキテクト🏗️ - portfolio-manager, allocator");
                    println!("    watchman   ウォッチマン👁️ - news-monitor, sentinel-news");
                    println!("    chart      チャート📈 - technical-analyst, chart-reader");
                    println!("    fundy      ファンディ📊 - fundamental-analyst, value-investor");
                    println!("    scribe     スクライブ📝 - reporter, documenter");
                    println!();
                    println!("  Content Society (6):");
                    println!("    tweeter    ツイーター🐦 - twitter-specialist, x-poster");
                    println!("    pen        ペン✒️ - writer, author");
                    println!("    vidpro     ビッドプロ🎬 - video-producer, youtuber");
                    println!("    artist     アーティスト🎨 - designer, visual-creator");
                    println!("    optimizer  オプティマイザー🔧 - seo-specialist, seo-analyst");
                    println!("    scheduler  スケジューラー📅 - calendar-manager, planner");
                    println!();
                    println!("  Marketing Society (4):");
                    println!("    hiro       ヒロ🚀 - promoter, growth-hacker");
                    println!("    kazoeru    カゾエル🔢 - metrics-tracker, data-analyst");
                    println!("    funnel     ファネル🌪️ - conversion-optimizer, cro-specialist");
                    println!("    adops      アドオプス📢 - ad-manager, media-buyer");
                    println!();
                    println!("【使用例】");
                    println!();
                    println!("  # 個別送信");
                    println!("  miyabi openclaw send maestro \"実装タスクを割り当てて\"");
                    println!("  miyabi openclaw send kade \"コードレビューお願いします\"");
                    println!("  miyabi openclaw send shikirun \"エイリアスでもOK\"");
                    println!();
                    println!("  # ブロードキャスト");
                    println!("  miyabi openclaw broadcast \"システムメンテナンス開始\"");
                    println!("  miyabi openclaw broadcast-society content \"新記事投稿\"");
                    println!();
                    println!("【環境変数】");
                    println!();
                    println!(
                        "  OPENCLAW_GATEWAY_URL  - Gateway URL (default: http://127.0.0.1:18789)"
                    );
                    println!("  OPENCLAW_TOKEN         - Gateway認証トークン");
                    println!();
                    println!("【設定ファイル】");
                    println!();
                    println!("  ~/.openclaw/openclaw.json  - 設定ファイルから自動読み込み");
                    println!();
                    println!(
                        "---
                    🌸 Miyabi Framework - OpenClaw Integration"
                    );
                }
                OpenclawCommand::Status => {
                    // Already handled above
                    unreachable!();
                }
            }
        }

        Some(Commands::Collab { command }) => {
            use std::env;
            use std::process::Command;

            let collab_bin = {
                let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                format!("{}/.local/bin/collab", home)
            };

            let mut args: Vec<String> = Vec::new();

            match command {
                CollabCommand::List {
                    json,
                    r#type,
                    count,
                } => {
                    args.push("tile".to_string());
                    args.push("list".to_string());
                    if json {
                        args.push("--json".to_string());
                    }
                    if count {
                        args.push("--count".to_string());
                    }
                    if let Some(t) = r#type {
                        args.push("--type".to_string());
                        args.push(t);
                    }
                }
                CollabCommand::Add {
                    tile_type,
                    file,
                    pos,
                    size,
                    idempotent,
                } => {
                    args.push("tile".to_string());
                    args.push("add".to_string());
                    args.push(tile_type);
                    if let Some(f) = file {
                        args.push("--file".to_string());
                        args.push(f);
                    }
                    if let Some(p) = pos {
                        args.push("--pos".to_string());
                        args.push(p);
                    }
                    if let Some(s) = size {
                        args.push("--size".to_string());
                        args.push(s);
                    }
                    if idempotent {
                        args.push("--idempotent".to_string());
                    }
                }
                CollabCommand::Rm { tile_id } => {
                    args.push("tile".to_string());
                    args.push("rm".to_string());
                    args.push(tile_id);
                }
                CollabCommand::Move { tile_id, pos } => {
                    args.push("tile".to_string());
                    args.push("move".to_string());
                    args.push(tile_id);
                    args.push("--pos".to_string());
                    args.push(pos);
                }
                CollabCommand::Resize { tile_id, size } => {
                    args.push("tile".to_string());
                    args.push("resize".to_string());
                    args.push(tile_id);
                    args.push("--size".to_string());
                    args.push(size);
                }
                CollabCommand::Viewport { pan, zoom } => {
                    if pan.is_some() || zoom.is_some() {
                        args.push("viewport".to_string());
                        args.push("set".to_string());
                        if let Some(p) = pan {
                            args.push("--pan".to_string());
                            args.push(p);
                        }
                        if let Some(z) = zoom {
                            args.push("--zoom".to_string());
                            args.push(z.to_string());
                        }
                    } else {
                        args.push("viewport".to_string());
                    }
                }
                CollabCommand::Status => {
                    args.push("status".to_string());
                }
            }

            let status = Command::new(&collab_bin).args(&args).status();

            match status {
                Ok(s) => {
                    if !s.success() {
                        std::process::exit(s.code().unwrap_or(1));
                    }
                }
                Err(e) => {
                    eprintln!("error: failed to run collab CLI ({}): {}", collab_bin, e);
                    eprintln!(
                        "  → Install collab CLI: https://github.com/ShunsukeHayashi/collab-cli"
                    );
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}

fn handle_gate_command(
    format: &OutputFormat,
    store_path: &std::path::Path,
    command: GateCommand,
) -> anyhow::Result<i32> {
    use miyabi_core::protocol::{
        DeterministicExecutionProtocol, ImpactInput, ProtocolError, RegisterTaskRequest,
        StatusReport,
    };
    use miyabi_core::store::{CompletionMode, ImpactRiskLevel};

    let protocol = DeterministicExecutionProtocol::from_store_path(store_path.to_path_buf());
    let actor = "miyabi-cli";
    let node = std::env::var("HOSTNAME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "local".to_string());

    let result = match command {
        GateCommand::Register {
            issue,
            title,
            task_id,
            dependencies,
            soft_dependencies,
            priority,
            completion_mode,
        } => {
            let task_id = task_id.unwrap_or_else(|| derive_task_id(issue, &title));
            protocol
                .register(
                    RegisterTaskRequest {
                        issue,
                        task_id,
                        title,
                        dependencies,
                        soft_dependencies,
                        priority,
                        completion_mode: match completion_mode {
                            CompletionModeArg::GithubPr => CompletionMode::GithubPr,
                            CompletionModeArg::Manual => CompletionMode::Manual,
                            CompletionModeArg::ExternalOp => CompletionMode::ExternalOp,
                        },
                    },
                    actor,
                    &node,
                )
                .map(|task| {
                    if matches!(format, OutputFormat::Json) {
                        println!("{}", serde_json::to_string_pretty(&task).unwrap());
                    } else {
                        println!("registered: {} ({})", task.id, task.title);
                    }
                })
        }
        GateCommand::Status { task_id } => {
            protocol
                .status(task_id.as_deref())
                .map(|status| match status {
                    StatusReport::Task(task) => {
                        if matches!(format, OutputFormat::Json) {
                            println!("{}", serde_json::to_string_pretty(&task).unwrap());
                        } else {
                            println!("{}: {:?} - {}", task.id, task.current_state, task.title);
                        }
                    }
                    StatusReport::Snapshot(snapshot) => {
                        if matches!(format, OutputFormat::Json) {
                            println!("{}", serde_json::to_string_pretty(&snapshot).unwrap());
                        } else {
                            println!("tasks: {}", snapshot.tasks.len());
                            for task in snapshot.tasks {
                                println!("  {} [{:?}] {}", task.id, task.current_state, task.title);
                            }
                        }
                    }
                })
        }
        GateCommand::Assign {
            task_id,
            agent,
            node: agent_node,
            files,
        } => protocol
            .assign(&task_id, &agent, &agent_node, &files)
            .map(|result| {
                if matches!(format, OutputFormat::Json) {
                    println!("{}", serde_json::to_string_pretty(&result).unwrap());
                } else {
                    println!("assigned: {} -> {}@{}", result.task.id, agent, agent_node);
                }
            }),
        GateCommand::Impact {
            task_id,
            risk,
            approve,
            symbols,
            depth1,
            analyzed_commit,
            input_hash,
        } => protocol
            .record_impact(
                &task_id,
                ImpactInput {
                    risk_level: match risk {
                        ImpactRiskArg::Low => ImpactRiskLevel::Low,
                        ImpactRiskArg::Medium => ImpactRiskLevel::Medium,
                        ImpactRiskArg::High => ImpactRiskLevel::High,
                        ImpactRiskArg::Critical => ImpactRiskLevel::Critical,
                    },
                    affected_symbols: symbols,
                    depth1,
                    analyzed_commit,
                    input_hash,
                    approve,
                },
                actor,
                &node,
            )
            .map(|task| {
                if matches!(format, OutputFormat::Json) {
                    println!("{}", serde_json::to_string_pretty(&task).unwrap());
                } else {
                    println!("impact recorded: {}", task.id);
                }
            }),
        GateCommand::Branch { task_id, name } => protocol
            .record_branch(&task_id, &name, actor, &node)
            .map(|task| {
                if matches!(format, OutputFormat::Json) {
                    println!("{}", serde_json::to_string_pretty(&task).unwrap());
                } else {
                    println!("branch recorded: {} -> {}", task.id, name);
                }
            }),
        GateCommand::Pr { task_id, number } => protocol
            .record_pr(&task_id, number, actor, &node)
            .map(|task| {
                if matches!(format, OutputFormat::Json) {
                    println!("{}", serde_json::to_string_pretty(&task).unwrap());
                } else {
                    println!("pr recorded: {} -> #{}", task.id, number);
                }
            }),
        GateCommand::Merge { task_id, sha } => protocol
            .record_merge(&task_id, &sha, actor, &node)
            .map(|task| {
                if matches!(format, OutputFormat::Json) {
                    println!("{}", serde_json::to_string_pretty(&task).unwrap());
                } else {
                    println!("merge recorded: {} -> {}", task.id, sha);
                }
            }),
        GateCommand::Locks => protocol.locks().map(|locks| {
            if matches!(format, OutputFormat::Json) {
                println!("{}", serde_json::to_string_pretty(&locks).unwrap());
            } else if locks.is_empty() {
                println!("no active locks");
            } else {
                for (file, lock) in locks {
                    println!("{} -> {}@{}", file, lock.agent, lock.node);
                }
            }
        }),
        GateCommand::Dag => protocol.dag().map(|report| {
            if matches!(format, OutputFormat::Json) {
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
            } else {
                for (index, level) in report.levels.iter().enumerate() {
                    println!("level {}: {}", index, level.join(", "));
                }
            }
        }),
        GateCommand::Dispatchable => protocol.dispatchable().map(|report| {
            if matches!(format, OutputFormat::Json) {
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
            } else if report.tasks.is_empty() {
                println!("no dispatchable tasks");
            } else {
                for task in report.tasks {
                    println!("{} [{}] {}", task.id, task.priority, task.title);
                }
            }
        }),
    };

    Ok(match result {
        Ok(()) => 0,
        Err(ProtocolError::GateRejected(message)) => {
            emit_gate_error(format, "gate_rejected", &message);
            1
        }
        Err(ProtocolError::DependencyBlocked(message)) => {
            emit_gate_error(format, "gate_rejected", &message);
            1
        }
        Err(ProtocolError::Input(message)) => {
            emit_gate_error(format, "input_error", &message);
            2
        }
        Err(ProtocolError::Internal(error)) => {
            emit_gate_error(format, "internal_error", &error.to_string());
            1
        }
    })
}

fn emit_gate_error(format: &OutputFormat, kind: &str, message: &str) {
    if matches!(format, OutputFormat::Json) {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "error": kind,
                "message": message,
            }))
            .unwrap()
        );
    } else {
        eprintln!("{}: {}", kind, message);
    }
}

fn derive_task_id(issue: u64, title: &str) -> String {
    if issue > 0 {
        return format!("issue-{issue}");
    }

    let slug: String = title
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    if slug.is_empty() {
        "task".to_string()
    } else {
        slug
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    } else {
        s.to_string()
    }
}
