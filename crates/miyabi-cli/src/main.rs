//! Miyabi CLI - Main entry point

use chrono::Duration as ChronoDuration;
use clap::{Parser, Subcommand, ValueEnum};
use miyabi_core::{FeatureFlagManager, RulesLoader};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
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
    /// Attach task context for execution
    Attach { task_id: String },
    /// Record PR creation
    Pr { task_id: String, number: u64 },
    /// Record merge verification
    Merge { task_id: String, sha: String },
    /// Verify merge state using GitHub metadata
    VerifyMerge {
        task_id: String,
        #[arg(long)]
        repo: String,
    },
    /// Force-release an active lock
    ForceUnlock {
        task_id: String,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        operator: String,
    },
    /// Mark a task complete without merge verification
    ManualComplete {
        task_id: String,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        operator: String,
    },
    /// List active locks
    Locks,
    /// Show DAG levels
    Dag,
    /// Show dispatchable tasks
    Dispatchable,
    /// Serve a minimal web dashboard
    Serve {
        /// Port to bind the dashboard to
        #[arg(long, default_value_t = 4848)]
        port: u16,
    },
    /// Analyze recent event logs and extract learnings
    Dream {
        /// Analyze only recent events, e.g. 24h, 30m, 7d
        #[arg(long)]
        since: Option<String>,
        /// Obsidian vault path for exported learnings
        #[arg(long)]
        vault_path: Option<PathBuf>,
        /// Persist High learnings into docs/learnings/
        #[arg(long)]
        auto: bool,
    },
}

#[derive(Debug)]
struct AssignPlanAttachment {
    attachment_type: String,
    source: String,
    token_estimate: usize,
    content: String,
}

#[derive(Debug)]
struct AssignExecutionPlan {
    task_title: String,
    risk_level: Option<String>,
    locked_files: Vec<String>,
    completion_mode: String,
    context_attachments: Vec<AssignPlanAttachment>,
    next_steps: Vec<String>,
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
            .and_then(|result| {
                let attachments = protocol.attach_context(&task_id, actor, &node)?;
                let plan = build_assign_execution_plan(&result.task, attachments);
                if matches!(format, OutputFormat::Json) {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "assignment": result,
                            "plan": assign_plan_to_json(&plan),
                        }))
                        .unwrap()
                    );
                } else {
                    println!("assigned: {} -> {}@{}", result.task.id, agent, agent_node);
                    print_assign_execution_plan(&result, &plan);
                }
                Ok(())
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
        GateCommand::Attach { task_id } => {
            protocol
                .attach_context(&task_id, actor, &node)
                .map(|attachments| {
                    if matches!(format, OutputFormat::Json) {
                        println!("{}", serde_json::to_string_pretty(&attachments).unwrap());
                    } else if attachments.is_empty() {
                        println!("no context attachments: {}", task_id);
                    } else {
                        println!("context attachments: {}", task_id);
                        if std::env::var_os("OBSIDIAN_VAULT_PATH").is_some() {
                            println!("obsidian search: enabled via OBSIDIAN_VAULT_PATH");
                        }
                        for attachment in attachments {
                            println!(
                                "--- [{}] {} ({} tokens)",
                                attachment.attachment_type,
                                attachment.source,
                                attachment.token_estimate
                            );
                            println!("{}", attachment.content);
                        }
                    }
                })
        }
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
        GateCommand::VerifyMerge { task_id, repo } => protocol
            .verify_merge(&task_id, &repo, actor, &node)
            .map(|task| {
                if matches!(format, OutputFormat::Json) {
                    println!("{}", serde_json::to_string_pretty(&task).unwrap());
                } else {
                    let sha = task
                        .github_evidence
                        .as_ref()
                        .and_then(|evidence| evidence.merge_commit_sha.as_deref())
                        .unwrap_or("unknown");
                    println!("merge verified: {} -> {}", task.id, sha);
                }
            }),
        GateCommand::ForceUnlock {
            task_id,
            reason,
            operator,
        } => protocol
            .force_unlock(&task_id, &reason, &operator)
            .map(|task| {
                if matches!(format, OutputFormat::Json) {
                    println!("{}", serde_json::to_string_pretty(&task).unwrap());
                } else {
                    println!("lock released: {} by {}", task.id, operator);
                }
            }),
        GateCommand::ManualComplete {
            task_id,
            reason,
            operator,
        } => protocol
            .manual_complete(&task_id, &reason, &operator)
            .map(|task| {
                if matches!(format, OutputFormat::Json) {
                    println!("{}", serde_json::to_string_pretty(&task).unwrap());
                } else {
                    println!("task completed manually: {} by {}", task.id, operator);
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
        GateCommand::Serve { port } => {
            serve_dashboard(store_path, port)?;
            Ok(())
        }
        GateCommand::Dream {
            since,
            vault_path,
            auto,
        } => {
            let since = since
                .as_deref()
                .map(parse_gate_since)
                .transpose()
                .map_err(|error: anyhow::Error| ProtocolError::input(error.to_string()))?;
            let repo_root =
                std::env::current_dir().map_err(|error| ProtocolError::input(error.to_string()))?;
            protocol
                .dream(since, auto, &repo_root, actor, &node)
                .and_then(|report| {
                    if matches!(format, OutputFormat::Json) {
                        println!("{}", serde_json::to_string_pretty(&report).unwrap());
                    } else {
                        print_dream_report(&report);
                    }

                    if auto {
                        let obsidian_written: Vec<PathBuf> = report
                            .learnings
                            .iter()
                            .filter(|learning| learning.importance == miyabi_core::Importance::High)
                            .map(|learning| {
                                miyabi_core::dream::obsidian_export(learning, vault_path.as_deref())
                            })
                            .collect::<Result<Vec<_>, _>>()
                            .map_err(ProtocolError::Internal)?;
                        if !matches!(format, OutputFormat::Json) {
                            if obsidian_written.is_empty() {
                                println!("obsidian notes written: none");
                            } else {
                                println!("obsidian notes written: {}", obsidian_written.len());
                            }
                        }
                    }

                    Ok(())
                })
        }
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

fn build_assign_execution_plan(
    task: &miyabi_core::store::ExecutionTask,
    attachments: Vec<miyabi_core::store::ContextAttachment>,
) -> AssignExecutionPlan {
    AssignExecutionPlan {
        task_title: task.title.clone(),
        risk_level: task.impact.as_ref().map(|impact| match impact.risk_level {
            miyabi_core::store::ImpactRiskLevel::Low => "low".to_string(),
            miyabi_core::store::ImpactRiskLevel::Medium => "medium".to_string(),
            miyabi_core::store::ImpactRiskLevel::High => "high".to_string(),
            miyabi_core::store::ImpactRiskLevel::Critical => "critical".to_string(),
        }),
        locked_files: task
            .lock
            .as_ref()
            .map(|lock| lock.affected_files.clone())
            .unwrap_or_default(),
        completion_mode: completion_mode_label(task.completion_mode).to_string(),
        context_attachments: attachments
            .into_iter()
            .map(|attachment| AssignPlanAttachment {
                attachment_type: attachment.attachment_type,
                source: attachment.source,
                token_estimate: attachment.token_estimate,
                content: attachment.content,
            })
            .collect(),
        next_steps: assign_next_steps(&task.id, task.completion_mode),
    }
}

fn completion_mode_label(mode: miyabi_core::store::CompletionMode) -> &'static str {
    match mode {
        miyabi_core::store::CompletionMode::GithubPr => "github-pr",
        miyabi_core::store::CompletionMode::Manual => "manual",
        miyabi_core::store::CompletionMode::ExternalOp => "external-op",
    }
}

fn assign_next_steps(
    task_id: &str,
    completion_mode: miyabi_core::store::CompletionMode,
) -> Vec<String> {
    match completion_mode {
        miyabi_core::store::CompletionMode::GithubPr => vec![
            "1. Create branch".to_string(),
            "2. Make changes".to_string(),
            format!("3. miyabi gate branch {task_id} ..."),
            format!("4. miyabi gate pr {task_id} ..."),
            format!("5. miyabi gate merge {task_id} ..."),
        ],
        miyabi_core::store::CompletionMode::Manual => vec![
            "1. Complete the work".to_string(),
            format!("2. miyabi gate manual-complete {task_id} --reason ... --operator ..."),
        ],
        miyabi_core::store::CompletionMode::ExternalOp => vec![
            "1. Complete external operation".to_string(),
            format!("2. miyabi gate manual-complete {task_id} --reason ... --operator ..."),
        ],
    }
}

fn print_assign_execution_plan(
    result: &miyabi_core::protocol::AssignmentResult,
    plan: &AssignExecutionPlan,
) {
    println!("task title: {}", plan.task_title);
    println!(
        "risk level: {}",
        plan.risk_level.as_deref().unwrap_or("not recorded")
    );

    if plan.locked_files.is_empty() {
        println!("locked files: none");
    } else {
        println!("locked files:");
        for file in &plan.locked_files {
            println!("  - {}", file);
        }
    }

    println!("completion mode: {}", plan.completion_mode);

    if plan.context_attachments.is_empty() {
        println!("context attachments: none");
    } else {
        println!("context attachments:");
        for attachment in &plan.context_attachments {
            println!(
                "  - [{}] {} ({} tokens)",
                attachment.attachment_type, attachment.source, attachment.token_estimate
            );
            println!("{}", attachment.content);
        }
    }

    println!("next steps:");
    for step in &plan.next_steps {
        println!("  {}", step);
    }

    if result.lock_conflict.conflicting {
        println!("lock conflict: true");
    }
}

fn assign_plan_to_json(plan: &AssignExecutionPlan) -> serde_json::Value {
    serde_json::json!({
        "task_title": plan.task_title,
        "risk_level": plan.risk_level,
        "locked_files": plan.locked_files,
        "completion_mode": plan.completion_mode,
        "context_attachments": plan
            .context_attachments
            .iter()
            .map(|attachment| serde_json::json!({
                "attachment_type": attachment.attachment_type,
                "source": attachment.source,
                "token_estimate": attachment.token_estimate,
                "content": attachment.content,
            }))
            .collect::<Vec<_>>(),
        "next_steps": plan.next_steps,
    })
}

fn parse_gate_since(input: &str) -> anyhow::Result<ChronoDuration> {
    let trimmed = input.trim();
    if trimmed.len() < 2 {
        return Err(anyhow::anyhow!("invalid --since value: {trimmed}"));
    }

    let (number, unit) = trimmed.split_at(trimmed.len() - 1);
    let value: i64 = number
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid --since value: {trimmed}"))?;

    match unit {
        "s" => Ok(ChronoDuration::seconds(value)),
        "m" => Ok(ChronoDuration::minutes(value)),
        "h" => Ok(ChronoDuration::hours(value)),
        "d" => Ok(ChronoDuration::days(value)),
        _ => Err(anyhow::anyhow!(
            "unsupported --since unit: {unit} (use s, m, h, d)"
        )),
    }
}

fn print_dream_report(report: &miyabi_core::DreamReport) {
    println!("events processed: {}", report.events_processed);

    if report.patterns.gate_rejections.is_empty() {
        println!("gate rejections: none");
    } else {
        println!("gate rejections:");
        let mut gates: Vec<_> = report.patterns.gate_rejections.iter().collect();
        gates.sort_by(|left, right| left.0.cmp(right.0));
        for (gate, count) in gates {
            println!("  {} -> {}", gate, count);
        }
    }

    if report.patterns.lock_conflicts.is_empty() {
        println!("lock conflicts: none");
    } else {
        println!("lock conflicts:");
        let mut files: Vec<_> = report.patterns.lock_conflicts.iter().collect();
        files.sort_by(|left, right| left.0.cmp(right.0));
        for (file, count) in files {
            println!("  {} -> {}", file, count);
        }
    }

    if report.patterns.completion_times.is_empty() {
        println!("completion times: none");
    } else {
        println!("completion times:");
        for (task_id, duration) in &report.patterns.completion_times {
            println!("  {} -> {}s", task_id, duration.as_secs());
        }
    }

    if report.learnings.is_empty() {
        println!("learnings: none");
    } else {
        println!("learnings:");
        for learning in &report.learnings {
            println!(
                "  [{:?}] {}{}",
                learning.importance,
                learning.title,
                learning
                    .related_task
                    .as_deref()
                    .map(|task| format!(" ({task})"))
                    .unwrap_or_default()
            );
            println!("    {}", learning.content);
        }
    }
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

const POLARIS_DASHBOARD_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Polaris Dashboard</title>
  <style>
    :root {
      color-scheme: light;
      --bg: #f4f7fb;
      --panel: #ffffff;
      --text: #162033;
      --muted: #667085;
      --border: #d6dfeb;
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      color: var(--text);
      background: linear-gradient(180deg, #eef4ff 0%, #f8fafc 55%, #f4f7fb 100%);
    }
    .shell {
      max-width: 1120px;
      margin: 0 auto;
      padding: 16px;
    }
    header, .panel {
      background: rgba(255, 255, 255, 0.9);
      border: 1px solid var(--border);
      border-radius: 18px;
      box-shadow: 0 10px 28px rgba(15, 23, 42, 0.06);
    }
    header {
      padding: 20px;
      margin-bottom: 16px;
    }
    h1 {
      margin: 0 0 8px;
      font-size: clamp(1.6rem, 3vw, 2.4rem);
    }
    h2 {
      margin: 0 0 12px;
      font-size: 1.05rem;
    }
    .subtitle, .meta {
      margin: 0;
      color: var(--muted);
    }
    .grid {
      display: grid;
      gap: 16px;
      grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    }
    .panel {
      padding: 16px;
    }
    .task-list, .dag-list, .lock-list {
      list-style: none;
      margin: 0;
      padding: 0;
      display: grid;
      gap: 10px;
    }
    .task-item, .dag-item, .lock-item {
      padding: 12px;
      border: 1px solid var(--border);
      border-radius: 14px;
      background: #fbfdff;
    }
    .task-top {
      display: flex;
      justify-content: space-between;
      align-items: center;
      gap: 8px;
      margin-bottom: 6px;
    }
    .task-title {
      font-weight: 600;
      word-break: break-word;
    }
    .task-meta, .dag-meta, .lock-meta {
      font-size: 0.9rem;
      color: var(--muted);
    }
    .badge {
      display: inline-flex;
      justify-content: center;
      align-items: center;
      min-width: 92px;
      padding: 4px 10px;
      border-radius: 999px;
      color: #ffffff;
      font-size: 0.82rem;
      font-weight: 700;
      text-transform: lowercase;
    }
    .empty {
      color: var(--muted);
      font-style: italic;
    }
    @media (max-width: 640px) {
      .shell { padding: 12px; }
      header, .panel { border-radius: 14px; }
      .task-top { align-items: flex-start; flex-direction: column; }
    }
  </style>
</head>
<body>
  <div class="shell">
    <header>
      <h1>Polaris Dashboard</h1>
      <p class="subtitle">Deterministic Task Protocol live view</p>
      <p class="meta" id="meta">Loading...</p>
    </header>
    <section class="grid">
      <article class="panel">
        <h2>Tasks</h2>
        <ul id="tasks" class="task-list"><li class="empty">Loading tasks...</li></ul>
      </article>
      <article class="panel">
        <h2>DAG Levels</h2>
        <ul id="dag" class="dag-list"><li class="empty">Loading DAG...</li></ul>
      </article>
      <article class="panel">
        <h2>File Locks</h2>
        <ul id="locks" class="lock-list"><li class="empty">Loading locks...</li></ul>
      </article>
    </section>
  </div>
  <script>
    const stateColors = {
      pending: "#6b7280",
      implementing: "#2563eb",
      merged: "#15803d",
      done: "#15803d",
      blocked: "#dc2626"
    };

    function setEmpty(id, message) {
      document.getElementById(id).innerHTML = '<li class="empty">' + message + '</li>';
    }

    function colorForState(state) {
      return stateColors[state] || "#7c3aed";
    }

    function renderTasks(snapshot) {
      const tasks = Array.isArray(snapshot.tasks) ? snapshot.tasks : [];
      const el = document.getElementById("tasks");
      if (tasks.length === 0) {
        setEmpty("tasks", "No tasks in snapshot");
        return;
      }

      el.innerHTML = "";
      for (const task of tasks) {
        const item = document.createElement("li");
        item.className = "task-item";

        const top = document.createElement("div");
        top.className = "task-top";

        const title = document.createElement("div");
        title.className = "task-title";
        title.textContent = task.title + " (" + task.id + ")";

        const badge = document.createElement("span");
        badge.className = "badge";
        badge.style.background = colorForState(task.current_state);
        badge.textContent = task.current_state;

        top.appendChild(title);
        top.appendChild(badge);

        const meta = document.createElement("div");
        meta.className = "task-meta";
        const deps = Array.isArray(task.dependencies) && task.dependencies.length > 0
          ? task.dependencies.join(", ")
          : "none";
        meta.textContent = "priority " + task.priority + " | deps: " + deps;

        item.appendChild(top);
        item.appendChild(meta);
        el.appendChild(item);
      }
    }

    function renderDag(report) {
      const levels = Array.isArray(report.levels) ? report.levels : [];
      const el = document.getElementById("dag");
      if (levels.length === 0) {
        setEmpty("dag", "No DAG levels available");
        return;
      }

      el.innerHTML = "";
      levels.forEach((level, index) => {
        const item = document.createElement("li");
        item.className = "dag-item";

        const title = document.createElement("div");
        title.className = "task-title";
        title.textContent = "Level " + index;

        const meta = document.createElement("div");
        meta.className = "dag-meta";
        meta.textContent = Array.isArray(level) && level.length > 0 ? level.join(", ") : "empty";

        item.appendChild(title);
        item.appendChild(meta);
        el.appendChild(item);
      });
    }

    function renderLocks(locks) {
      const entries = Object.entries(locks || {});
      const el = document.getElementById("locks");
      if (entries.length === 0) {
        setEmpty("locks", "No active file locks");
        return;
      }

      el.innerHTML = "";
      for (const [file, lock] of entries) {
        const item = document.createElement("li");
        item.className = "lock-item";

        const title = document.createElement("div");
        title.className = "task-title";
        title.textContent = file;

        const meta = document.createElement("div");
        meta.className = "lock-meta";
        meta.textContent = lock.agent + "@" + lock.node + " | task: " + lock.task_id;

        item.appendChild(title);
        item.appendChild(meta);
        el.appendChild(item);
      }
    }

    async function refresh() {
      try {
        const [statusRes, locksRes, dagRes] = await Promise.all([
          fetch("/api/status"),
          fetch("/api/locks"),
          fetch("/api/dag")
        ]);

        if (!statusRes.ok || !locksRes.ok || !dagRes.ok) {
          throw new Error("API request failed");
        }

        const [status, locks, dag] = await Promise.all([
          statusRes.json(),
          locksRes.json(),
          dagRes.json()
        ]);

        renderTasks(status);
        renderLocks(locks);
        renderDag(dag);

        document.getElementById("meta").textContent =
          "Snapshot version " + status.version + " | updated " + status.generated_at +
          " | auto-refresh every 3s";
      } catch (error) {
        document.getElementById("meta").textContent = "Refresh failed: " + error.message;
        setEmpty("tasks", "Failed to load tasks");
        setEmpty("dag", "Failed to load DAG");
        setEmpty("locks", "Failed to load locks");
      }
    }

    refresh();
    setInterval(refresh, 3000);
  </script>
</body>
</html>
"##;

fn serve_dashboard(store_path: &std::path::Path, port: u16) -> anyhow::Result<()> {
    let protocol = miyabi_core::protocol::DeterministicExecutionProtocol::from_store_path(
        store_path.to_path_buf(),
    );
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    println!("Polaris Dashboard listening on http://127.0.0.1:{port}");

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                if let Err(error) = handle_dashboard_connection(&protocol, &mut stream) {
                    eprintln!("dashboard request error: {error}");
                }
            }
            Err(error) => eprintln!("dashboard accept error: {error}"),
        }
    }

    Ok(())
}

fn handle_dashboard_connection(
    protocol: &miyabi_core::protocol::DeterministicExecutionProtocol,
    stream: &mut TcpStream,
) -> anyhow::Result<()> {
    let mut request_line = String::new();
    let mut reader = BufReader::new(stream.try_clone()?);
    reader.read_line(&mut request_line)?;

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or("/");

    if method != "GET" {
        write_http_response(
            stream,
            "405 Method Not Allowed",
            "text/plain; charset=utf-8",
            b"method not allowed",
        )?;
        return Ok(());
    }

    match path {
        "/" => write_http_response(
            stream,
            "200 OK",
            "text/html; charset=utf-8",
            POLARIS_DASHBOARD_HTML.as_bytes(),
        )?,
        "/api/status" => {
            let body =
                serde_json::to_vec_pretty(&protocol.status(None)?).map_err(io::Error::other)?;
            write_http_response(stream, "200 OK", "application/json; charset=utf-8", &body)?;
        }
        "/api/locks" => {
            let body = serde_json::to_vec_pretty(&protocol.locks()?).map_err(io::Error::other)?;
            write_http_response(stream, "200 OK", "application/json; charset=utf-8", &body)?;
        }
        "/api/dag" => {
            let body = serde_json::to_vec_pretty(&protocol.dag()?).map_err(io::Error::other)?;
            write_http_response(stream, "200 OK", "application/json; charset=utf-8", &body)?;
        }
        _ => write_http_response(
            stream,
            "404 Not Found",
            "text/plain; charset=utf-8",
            b"not found",
        )?,
    }

    Ok(())
}

fn write_http_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    stream.flush()
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
