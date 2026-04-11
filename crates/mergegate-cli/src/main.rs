/// MergeGate CLI - Main entry point
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use clap::{Args, CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum};
use miyabi_core::{FeatureFlagManager, RulesLoader};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::Command;
use tracing_subscriber::EnvFilter;

/// Global feature flags manager
static FEATURE_FLAGS: std::sync::OnceLock<FeatureFlagManager> = std::sync::OnceLock::new();
static INVOKED_BINARY_NAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();

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

fn detect_invoked_binary_name() -> String {
    std::env::args()
        .next()
        .and_then(|path| {
            std::path::Path::new(&path)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "miyabi".to_string())
}

fn current_binary_name() -> &'static str {
    INVOKED_BINARY_NAME
        .get_or_init(detect_invoked_binary_name)
        .as_str()
}

fn gate_command(command: &str) -> String {
    if command.is_empty() {
        format!("{} gate", current_binary_name())
    } else {
        format!("{} gate {}", current_binary_name(), command)
    }
}

fn agent_guide() -> String {
    AGENT_GUIDE_TEMPLATE
        .replace("{{GATE}}", &gate_command(""))
        .replace("{{BINARY}}", current_binary_name())
}

#[derive(Parser)]
#[command(author, version, about = "MergeGate - Engine-agnostic gate CLI for AI-assisted development", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
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
    #[command(
        about = "Deterministic Task Protocol gate controls",
        long_about = "Run the gate init command to set up a new project.\n\nDeterministic Task Protocol gate controls"
    )]
    Gate {
        /// Output format
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
        /// Emit a hook-friendly JSON event envelope to stdout
        #[arg(long)]
        emit_event: bool,
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

#[derive(Clone, Debug, Args)]
struct ExportFilterArgs {
    /// Filter by task state (e.g. implementing, done)
    #[arg(long)]
    state: Option<String>,
    /// Filter by impact risk (LOW, MEDIUM, HIGH, CRITICAL)
    #[arg(long)]
    risk: Option<String>,
    /// Filter by created-at timestamp (RFC3339)
    #[arg(long)]
    since: Option<String>,
}

#[derive(Subcommand)]
enum GateCommand {
    /// Initialize project memory for the current repository
    Init,
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
        /// Skip skill-bus auto-enqueue
        #[arg(long)]
        no_bus: bool,
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
    /// Force-refresh task context attachments
    Refresh { task_id: String },
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
    /// Validate snapshot consistency
    Validate,
    /// Export tasks as JSON
    ExportJson {
        #[command(flatten)]
        filter: ExportFilterArgs,
    },
    /// Export tasks as Markdown
    ExportMd {
        #[command(flatten)]
        filter: ExportFilterArgs,
    },
    /// Show task statistics
    Stats,
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
    /// Print the full workflow guide for agents
    Guide,
    /// Renew active lock heartbeats
    Heartbeat {
        /// Renew all implementing task leases
        #[arg(long)]
        all: bool,
    },
}

#[derive(Debug, Serialize)]
struct AssignPlanAttachment {
    attachment_type: String,
    source: String,
    token_estimate: usize,
    content: String,
}

#[derive(Debug, Serialize)]
struct AssignExecutionPlan {
    task_title: String,
    risk_level: Option<String>,
    locked_files: Vec<String>,
    completion_mode: String,
    context_attachments: Vec<AssignPlanAttachment>,
    next_steps: Vec<String>,
}

#[derive(Debug, Serialize)]
struct InitStatus {
    initialized: bool,
    current_dir: String,
    created_path: String,
    git_repo: bool,
    github_remote: Option<String>,
    gitignore_updated: bool,
    github_project_detected: bool,
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

    let invoked_binary_name = detect_invoked_binary_name();
    let _ = INVOKED_BINARY_NAME.set(invoked_binary_name.clone());
    let clap_binary_name: &'static str = Box::leak(invoked_binary_name.into_boxed_str());
    let matches = Cli::command().name(clap_binary_name).get_matches();
    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|error| error.exit());

    match cli.command {
        None => {
            let mut command = Cli::command();
            command = command.name(current_binary_name());
            command.print_help()?;
            println!();
            println!();
            println!("Start here:");
            println!("  {}", gate_command("status"));
            println!("  {}", gate_command("init"));
            println!("  {}", gate_command("guide"));
        }
        Some(Commands::Status) => {
            use miyabi_core::config::Config;

            let config = Config::load().unwrap_or_default();

            println!("MergeGate Status: Ready");
            println!();
            println!("Binary:   {}", current_binary_name());
            println!("Config:   {}", Config::default_path().display());
            println!();
            println!("Core workflow:");
            println!("  {}", gate_command("status"));
            println!("  {}", gate_command("init"));
            println!("  {}", gate_command("guide"));
            println!();
            println!("Direct runtime config remains on disk for compatibility.");
            println!("Model:    {}", config.api.model);
            println!("Sessions: {}", config.sessions_dir().display());

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
        Some(Commands::Gate {
            format,
            emit_event,
            store_path,
            command,
        }) => {
            let code = handle_gate_command(&format, emit_event, &store_path, command)?;
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
    emit_event: bool,
    store_path: &std::path::Path,
    command: GateCommand,
) -> anyhow::Result<i32> {
    use miyabi_core::protocol::{
        DeterministicExecutionProtocol, ImpactInput, ProtocolError, RegisterTaskRequest,
        StatusReport,
    };
    use miyabi_core::store::{CompletionMode, ImpactRiskLevel};

    let protocol = DeterministicExecutionProtocol::from_store_path(store_path.to_path_buf());
    let mut success_code = 0;
    let actor = "miyabi-cli";
    let node = std::env::var("HOSTNAME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "local".to_string());

    let result = match command {
        GateCommand::Init => {
            let status = initialize_gate_project(format, emit_event, store_path)?;
            if emit_event {
                emit_gate_event("gate_initialized", None, &status);
            }
            Ok(())
        }
        GateCommand::Register {
            issue,
            title,
            task_id,
            dependencies,
            soft_dependencies,
            priority,
            completion_mode,
            no_bus,
        } => {
            let task_id = task_id.unwrap_or_else(|| derive_task_id(issue, &title));
            let task_title = title.clone();
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
                    if emit_event {
                        emit_gate_event("task_registered", Some(&task.id), &task);
                    } else if matches!(format, OutputFormat::Json) {
                        println!("{}", serde_json::to_string_pretty(&task).unwrap());
                    } else {
                        println!("registered: {} ({})", task.id, task.title);
                    }
                    if !no_bus {
                        bus_enqueue(&task.id, &task_title, store_path);
                    }
                })
        }
        GateCommand::Status { task_id } => {
            protocol
                .status(task_id.as_deref())
                .map(|status| match status {
                    StatusReport::Task(task) => {
                        if emit_event {
                            emit_gate_event("task_status", Some(&task.id), &task);
                        } else if matches!(format, OutputFormat::Json) {
                            println!("{}", serde_json::to_string_pretty(&task).unwrap());
                        } else {
                            print_gate_task_status(&task);
                        }
                    }
                    StatusReport::Snapshot(snapshot) => {
                        if emit_event {
                            emit_gate_event("status_snapshot", None, &snapshot);
                        } else if matches!(format, OutputFormat::Json) {
                            println!("{}", serde_json::to_string_pretty(&snapshot).unwrap());
                        } else {
                            let dispatchable = protocol.dispatchable().ok();
                            print_gate_snapshot_status(&snapshot, dispatchable.as_ref());
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
                let output = serde_json::json!({
                    "assignment": result,
                    "plan": assign_plan_to_json(&plan),
                });
                if emit_event {
                    emit_gate_event("lock_acquired", Some(&task_id), &output);
                } else if matches!(format, OutputFormat::Json) {
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
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
                if emit_event {
                    emit_gate_event("impact_recorded", Some(&task.id), &task);
                } else if matches!(format, OutputFormat::Json) {
                    println!("{}", serde_json::to_string_pretty(&task).unwrap());
                } else {
                    println!("impact recorded: {}", task.id);
                }
            }),
        GateCommand::Branch { task_id, name } => protocol
            .record_branch(&task_id, &name, actor, &node)
            .map(|task| {
                if emit_event {
                    emit_gate_event("branch_created", Some(&task.id), &task);
                } else if matches!(format, OutputFormat::Json) {
                    println!("{}", serde_json::to_string_pretty(&task).unwrap());
                } else {
                    println!("branch recorded: {} -> {}", task.id, name);
                }
            }),
        GateCommand::Attach { task_id } => {
            protocol
                .attach_context(&task_id, actor, &node)
                .map(|attachments| {
                    if emit_event {
                        emit_gate_event("context_attached", Some(&task_id), &attachments);
                    } else if matches!(format, OutputFormat::Json) {
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
        GateCommand::Refresh { task_id } => {
            protocol
                .refresh_context(&task_id, actor, &node)
                .map(|attachments| {
                    if emit_event {
                        emit_gate_event("context_refreshed", Some(&task_id), &attachments);
                    } else if matches!(format, OutputFormat::Json) {
                        println!("{}", serde_json::to_string_pretty(&attachments).unwrap());
                    } else if attachments.is_empty() {
                        println!("no context attachments: {}", task_id);
                    } else {
                        println!("context refreshed: {}", task_id);
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
                if emit_event {
                    emit_gate_event("pr_created", Some(&task.id), &task);
                } else if matches!(format, OutputFormat::Json) {
                    println!("{}", serde_json::to_string_pretty(&task).unwrap());
                } else {
                    println!("pr recorded: {} -> #{}", task.id, number);
                }
            }),
        GateCommand::Merge { task_id, sha } => protocol
            .record_merge(&task_id, &sha, actor, &node)
            .map(|task| {
                if emit_event {
                    emit_gate_event("task_completed", Some(&task.id), &task);
                } else if matches!(format, OutputFormat::Json) {
                    println!("{}", serde_json::to_string_pretty(&task).unwrap());
                } else {
                    println!("merge recorded: {} -> {}", task.id, sha);
                }
            }),
        GateCommand::VerifyMerge { task_id, repo } => protocol
            .verify_merge(&task_id, &repo, actor, &node)
            .map(|task| {
                if emit_event {
                    emit_gate_event("task_completed", Some(&task.id), &task);
                } else if matches!(format, OutputFormat::Json) {
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
                if emit_event {
                    emit_gate_event("lock_released", Some(&task.id), &task);
                } else if matches!(format, OutputFormat::Json) {
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
                if emit_event {
                    emit_gate_event("task_completed", Some(&task.id), &task);
                } else if matches!(format, OutputFormat::Json) {
                    println!("{}", serde_json::to_string_pretty(&task).unwrap());
                } else {
                    println!("task completed manually: {} by {}", task.id, operator);
                }
            }),
        GateCommand::Locks => protocol.locks().map(|locks| {
            if emit_event {
                emit_gate_event("locks_reported", None, &locks);
            } else if matches!(format, OutputFormat::Json) {
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
            if emit_event {
                emit_gate_event("dag_reported", None, &report);
            } else if matches!(format, OutputFormat::Json) {
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
            } else {
                for (index, level) in report.levels.iter().enumerate() {
                    println!("level {}: {}", index, level.join(", "));
                }
            }
        }),
        GateCommand::Validate => load_snapshot(store_path)
            .map_err(ProtocolError::Internal)
            .map(|snapshot| {
                let report = miyabi_core::validate::validate_snapshot(&snapshot);
                success_code = report.exit_code();
                if emit_event {
                    emit_gate_event("validation_reported", None, &report);
                } else if matches!(format, OutputFormat::Json) {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&validation_report_json(&report)).unwrap()
                    );
                } else {
                    println!("{report}");
                }
            }),
        GateCommand::ExportJson { filter } => load_snapshot(store_path)
            .map_err(ProtocolError::Internal)
            .and_then(|snapshot| {
                let filter = parse_export_filter(filter)
                    .map_err(|error| ProtocolError::input(error.to_string()))?;
                let export = miyabi_core::export::export_json(&snapshot, filter);
                println!("{export}");
                Ok(())
            }),
        GateCommand::ExportMd { filter } => load_snapshot(store_path)
            .map_err(ProtocolError::Internal)
            .and_then(|snapshot| {
                let filter = parse_export_filter(filter)
                    .map_err(|error| ProtocolError::input(error.to_string()))?;
                let export = miyabi_core::export_md::export_markdown(&snapshot, filter.as_ref());
                println!("{export}");
                Ok(())
            }),
        GateCommand::Stats => load_snapshot(store_path)
            .map_err(ProtocolError::Internal)
            .map(|snapshot| {
                let stats = miyabi_core::stats::compute_stats(&snapshot);
                if matches!(format, OutputFormat::Json) {
                    println!("{}", serde_json::to_string_pretty(&stats).unwrap());
                } else {
                    println!("{stats}");
                }
            }),
        GateCommand::Dispatchable => protocol.dispatchable().map(|report| {
            if emit_event {
                emit_gate_event("dispatchable_reported", None, &report);
            } else if matches!(format, OutputFormat::Json) {
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
            } else if report.tasks.is_empty() {
                println!("no dispatchable tasks");
                println!("next:");
                println!("  {}", gate_command("status"));
                println!("  {}", gate_command("dag"));
                println!("  {}", gate_command("guide"));
            } else {
                for task in report.tasks {
                    println!("{} [{}] {}", task.id, task.priority, task.title);
                }
            }
        }),
        GateCommand::Serve { port } => {
            serve_dashboard(store_path, port)?;
            if emit_event {
                emit_gate_event(
                    "dashboard_started",
                    None,
                    serde_json::json!({ "port": port }),
                );
            }
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
                    if emit_event {
                        emit_gate_event("dream_recorded", None, &report);
                    } else if matches!(format, OutputFormat::Json) {
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
        GateCommand::Heartbeat { all } => {
            if !all {
                Err(ProtocolError::input("heartbeat currently requires --all"))
            } else {
                protocol.heartbeat_all().map(|renewed| {
                    let output = serde_json::json!({
                        "renewed": renewed,
                        "count": renewed.len(),
                    });
                    if emit_event {
                        emit_gate_event("heartbeat_renewed", None, &output);
                    } else if matches!(format, OutputFormat::Json) {
                        println!("{}", serde_json::to_string_pretty(&output).unwrap());
                    } else {
                        println!("renewed leases: {}", renewed.len());
                        for task_id in renewed {
                            println!("  {}", task_id);
                        }
                    }
                })
            }
        }
        GateCommand::Guide => {
            print!("{}", agent_guide());
            Ok(())
        }
    };

    Ok(match result {
        Ok(()) => success_code,
        Err(ProtocolError::GateRejected(message)) => {
            emit_gate_error(format, emit_event, "gate_rejected", &message);
            1
        }
        Err(ProtocolError::DependencyBlocked(message)) => {
            emit_gate_error(format, emit_event, "gate_rejected", &message);
            1
        }
        Err(ProtocolError::Input(message)) => {
            emit_gate_error(format, emit_event, "input_error", &message);
            2
        }
        Err(ProtocolError::Internal(error)) => {
            emit_gate_error(format, emit_event, "internal_error", &error.to_string());
            1
        }
    })
}

fn load_snapshot(
    store_path: &std::path::Path,
) -> Result<miyabi_core::store::TasksSnapshot, miyabi_core::Error> {
    let snapshot_store = miyabi_core::store::SnapshotStore::new(
        store_path.to_path_buf(),
        store_path
            .parent()
            .map(|parent| parent.join(".tasks.lock"))
            .unwrap_or_else(|| PathBuf::from(".tasks.lock")),
    );
    snapshot_store.load()
}

fn parse_export_filter(
    args: ExportFilterArgs,
) -> anyhow::Result<Option<miyabi_core::export::ExportFilter>> {
    if args.state.is_none() && args.risk.is_none() && args.since.is_none() {
        return Ok(None);
    }

    let since = args.since.as_deref().map(parse_export_since).transpose()?;

    Ok(Some(miyabi_core::export::ExportFilter {
        // TaskState serialises as snake_case; normalise to lowercase so users can pass
        // either "Implementing" or "implementing" interchangeably.
        state: args.state.map(|s| s.to_ascii_lowercase()),
        // ImpactRiskLevel serialises as SCREAMING_SNAKE_CASE; normalise to uppercase.
        risk_level: args.risk.map(|r| r.to_ascii_uppercase()),
        since,
    }))
}

fn parse_export_since(value: &str) -> anyhow::Result<DateTime<Utc>> {
    let parsed = DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .or_else(|_| {
            chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map(|date| date.and_hms_opt(0, 0, 0).expect("valid midnight").and_utc())
        })?;
    Ok(parsed)
}

fn validation_report_json(report: &miyabi_core::validate::ValidationReport) -> serde_json::Value {
    serde_json::json!({
        "severity": report.severity(),
        "exit_code": report.exit_code(),
        "orphaned_locks": report.orphaned_locks,
        "invalid_transitions": report.invalid_transitions,
        "circular_dependencies": report.circular_dependencies,
        "warnings": report.warnings,
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
            format!("3. {} {task_id} ...", gate_command("branch")),
            format!("4. {} {task_id} ...", gate_command("pr")),
            format!("5. {} {task_id} ...", gate_command("merge")),
        ],
        miyabi_core::store::CompletionMode::Manual => vec![
            "1. Complete the work".to_string(),
            format!(
                "2. {} {task_id} --reason ... --operator ...",
                gate_command("manual-complete")
            ),
        ],
        miyabi_core::store::CompletionMode::ExternalOp => vec![
            "1. Complete external operation".to_string(),
            format!(
                "2. {} {task_id} --reason ... --operator ...",
                gate_command("manual-complete")
            ),
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

fn print_gate_task_status(task: &miyabi_core::store::ExecutionTask) {
    println!("task: {}", task.id);
    println!("title: {}", task.title);
    println!("state: {:?}", task.current_state);

    if task.issue_number > 0 {
        println!("issue: #{}", task.issue_number);
    }
    println!("completion mode: {:?}", task.completion_mode);

    if let Some(impact) = &task.impact {
        println!(
            "impact: {:?} (symbols: {})",
            impact.risk_level, impact.affected_symbols
        );
    } else {
        println!("impact: not recorded");
    }

    match task.current_state {
        miyabi_core::store::TaskState::Pending | miyabi_core::store::TaskState::Draft => {
            if task.impact.is_none() {
                println!("next:");
                println!(
                    "  {} {} --risk low --symbols 0",
                    gate_command("impact"),
                    task.id
                );
                println!(
                    "  {} {} --agent <name> --node <machine> --files \"path/to/file\"",
                    gate_command("assign"),
                    task.id
                );
            } else {
                println!("next:");
                println!(
                    "  {} {} --agent <name> --node <machine> --files \"path/to/file\"",
                    gate_command("assign"),
                    task.id
                );
            }
        }
        miyabi_core::store::TaskState::Implementing => {
            println!("next:");
            println!("  {} {} <branch-name>", gate_command("branch"), task.id);
            println!("  {} {}", gate_command("attach"), task.id);
        }
        miyabi_core::store::TaskState::Reviewing => {
            println!("next:");
            println!("  {} {} <number>", gate_command("pr"), task.id);
            println!("  {} {} <40-char-sha>", gate_command("merge"), task.id);
        }
        miyabi_core::store::TaskState::Merged | miyabi_core::store::TaskState::Done => {
            println!("next:");
            println!("  {}", gate_command("dispatchable"));
        }
        _ => {}
    }
}

fn print_gate_snapshot_status(
    snapshot: &miyabi_core::store::TasksSnapshot,
    dispatchable: Option<&miyabi_core::protocol::DispatchableReport>,
) {
    if snapshot.tasks.is_empty() {
        println!("tasks: 0");
        println!("status: ready to initialize or register your first task");
        println!("this is not an error. it means the ledger is empty.");
        println!("next:");
        println!("  {}", gate_command("init"));
        println!(
            "  {} --issue <N> --title \"Your task\"",
            gate_command("register")
        );
        println!("  {}", gate_command("guide"));
        return;
    }

    let total = snapshot.tasks.len();
    let completed = snapshot
        .tasks
        .iter()
        .filter(|task| {
            matches!(
                task.current_state,
                miyabi_core::store::TaskState::Done | miyabi_core::store::TaskState::Merged
            )
        })
        .count();
    let active = snapshot
        .tasks
        .iter()
        .filter(|task| {
            matches!(
                task.current_state,
                miyabi_core::store::TaskState::Implementing
                    | miyabi_core::store::TaskState::Reviewing
                    | miyabi_core::store::TaskState::Analyzing
            )
        })
        .count();
    let waiting = total.saturating_sub(completed + active);
    let dispatchable_count = dispatchable.map(|report| report.count).unwrap_or(0);

    println!(
        "tasks: {} (dispatchable: {}, locks: {})",
        total,
        dispatchable_count,
        snapshot.file_locks.len()
    );
    println!(
        "summary: {} completed, {} active, {} waiting",
        completed, active, waiting
    );

    if let Some(report) = dispatchable {
        if !report.tasks.is_empty() {
            println!("next tasks:");
            for task in report.tasks.iter().take(3) {
                println!("  {} [{}] {}", task.id, task.priority, task.title);
            }
            println!("next:");
            println!("  {} <task-id>", gate_command("status"));
            println!(
                "  {} <task-id> --agent <name> --node <machine> --files \"path/to/file\"",
                gate_command("assign")
            );
        } else {
            println!("next:");
            println!("  {}", gate_command("dag"));
            println!("  {}", gate_command("locks"));
            println!("  {}", gate_command("guide"));
        }
    }

    println!("all tasks:");
    for task in &snapshot.tasks {
        println!("  {} [{:?}] {}", task.id, task.current_state, task.title);
    }
}

fn initialize_gate_project(
    format: &OutputFormat,
    emit_event: bool,
    store_path: &std::path::Path,
) -> anyhow::Result<InitStatus> {
    let current_dir = std::env::current_dir()?;
    let created_path = store_path.display().to_string();
    let initialized = if store_path.exists() {
        false
    } else {
        if let Some(parent) = store_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let snapshot = miyabi_core::store::TasksSnapshot::default();
        fs::write(store_path, serde_json::to_vec_pretty(&snapshot)?)?;
        true
    };

    let git_repo = is_git_repository();
    let github_remote = git_origin_github_remote();
    let gitignore_updated =
        ensure_project_memory_gitignore_entries(&current_dir.join(".gitignore"))?;
    let github_project_detected = github_remote
        .as_deref()
        .and_then(github_owner_from_remote)
        .is_some_and(is_github_project_detected);
    let status = InitStatus {
        initialized,
        current_dir: current_dir.display().to_string(),
        created_path,
        git_repo,
        github_remote,
        gitignore_updated,
        github_project_detected,
    };

    if emit_event {
        return Ok(status);
    }

    if matches!(format, OutputFormat::Json) {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "status": if status.initialized { "initialized" } else { "already_initialized" },
                "current_dir": status.current_dir,
                "created": status.created_path,
                "git_repository": status.git_repo,
                "github_remote": status.github_remote,
                "gitignore_updated": status.gitignore_updated,
                "github_project_detected": status.github_project_detected,
                "next_steps": [
                    gate_command("status"),
                    gate_command("guide"),
                    format!("{} --issue <N> --title ...", gate_command("register"))
                ],
            }))?
        );
    } else {
        if status.initialized {
            println!("MergeGate initialized in {}", status.current_dir);
            println!("Created: {}", status.created_path);
        } else {
            println!("Already initialized");
        }
        if !status.git_repo {
            println!("⚠️ Not a git repository. Run: git init");
        }
        if status.github_remote.is_none() {
            println!("⚠️ No GitHub remote. Run: gh repo create <name> --private");
        }
        println!("Next steps:");
        println!("  {}", gate_command("status"));
        println!("  {}", gate_command("guide"));
        println!("  {} --issue <N> --title ...", gate_command("register"));
        print_init_checklist(&status);
    }

    Ok(status)
}

fn print_init_checklist(status: &InitStatus) {
    if status.git_repo {
        println!("✅ Git repository");
    } else {
        println!("⚠️ Git repository");
    }

    if let Some(remote) = &status.github_remote {
        println!("✅ GitHub remote: {}", remote);
    } else {
        println!("⚠️ GitHub remote");
    }

    println!("✅ project_memory/tasks.json initialized");

    println!("✅ .gitignore updated");

    if status.github_project_detected {
        println!("✅ GitHub Project detected");
    } else {
        println!("⚠️ GitHub Project not detected (optional: gh project create)");
    }
}

fn is_git_repository() -> bool {
    command_stdout("git", &["rev-parse", "--git-dir"]).is_some()
}

fn git_origin_github_remote() -> Option<String> {
    let remote = command_stdout("git", &["remote", "get-url", "origin"])?;
    parse_github_remote(&remote)
}

fn parse_github_remote(remote: &str) -> Option<String> {
    let trimmed = remote.trim();
    let slug = if let Some(rest) = trimmed.strip_prefix("git@github.com:") {
        rest
    } else if let Some(index) = trimmed.find("github.com/") {
        &trimmed[(index + "github.com/".len())..]
    } else {
        return None;
    };

    Some(slug.trim_end_matches(".git").trim_matches('/').to_string())
}

fn github_owner_from_remote(remote: &str) -> Option<&str> {
    remote.split('/').next().filter(|owner| !owner.is_empty())
}

fn is_github_project_detected(owner: &str) -> bool {
    command_stdout("gh", &["project", "list", "--owner", owner, "--limit", "1"])
        .is_some_and(|stdout| !stdout.trim().is_empty())
}

fn ensure_project_memory_gitignore_entries(path: &std::path::Path) -> anyhow::Result<bool> {
    let required_entries = [
        "project_memory/task-events.jsonl",
        "project_memory/tasks.snapshot.json",
        "project_memory/.tasks.lock",
    ];

    let mut content = if path.exists() {
        fs::read_to_string(path)?
    } else {
        String::new()
    };
    let original = content.clone();

    for entry in required_entries {
        if !content.lines().any(|line| line.trim() == entry) {
            if !content.is_empty() && !content.ends_with('\n') {
                content.push('\n');
            }
            content.push_str(entry);
            content.push('\n');
        }
    }

    if content != original {
        fs::write(path, content)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn command_stdout(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        None
    } else {
        Some(stdout)
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

fn emit_gate_event(event: &str, task_id: Option<&str>, payload: impl serde::Serialize) {
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "event": event,
            "task_id": task_id,
            "source": "miyabi-gate",
            "ts": chrono::Utc::now(),
            "payload": payload,
        }))
        .unwrap()
    );
}

fn emit_gate_error(format: &OutputFormat, emit_event: bool, kind: &str, message: &str) {
    if emit_event {
        emit_gate_event(kind, None, serde_json::json!({ "message": message }));
    } else if matches!(format, OutputFormat::Json) {
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

const AGENT_GUIDE_TEMPLATE: &str = r#"
# MergeGate ({{GATE}}) — Agent Guide

## What is this?

MergeGate is a deterministic task execution protocol. It enforces a strict
workflow so that any agent on any machine produces the same verifiable result.
Tasks are tracked in project_memory/tasks.json, not in conversation memory.

## Rules

1. You MUST NOT edit files without acquiring a lock via `assign`.
2. You MUST register a task before starting work.
3. You MUST record impact analysis before assigning.
4. HIGH/CRITICAL risk requires --approve flag.
5. Every code task ends with branch → pr → merge. No exceptions.

## Workflow (execute in this exact order)

```
Step 1: Register
  {{GATE}} register --issue <N> --title "Task description"

Step 2: Impact analysis
  {{GATE}} impact <task-id> --risk <low|medium|high|critical> --symbols <N>
  # Add --approve if risk is high or critical

Step 3: Assign (acquires file locks)
  {{GATE}} assign <task-id> --agent <your-name> --node <machine> --files "file1.rs,file2.rs"
  # This prints an execution plan and context attachments. Read them.

Step 4: Work
  # Edit ONLY the locked files. Pre-commit hook blocks unlocked files.

Step 5: Branch
  {{GATE}} branch <task-id> feature/issue-<N>-<slug>

Step 6: PR
  {{GATE}} pr <task-id> <PR-number>

Step 7: Merge
  {{GATE}} merge <task-id> <merge-commit-sha>
```

## For document-only tasks (no PR needed)

```
{{GATE}} register --issue <N> --title "Doc task" --completion-mode manual
{{GATE}} impact <task-id> --risk low --symbols 0
{{GATE}} assign <task-id> --agent <name> --node <machine> --files "docs/file.md"
# ... do the work ...
{{GATE}} manual-complete <task-id> --reason "reason" --operator <name>
```

## Checking state

```
{{GATE}} status              # All tasks
{{GATE}} status <task-id>    # One task
{{GATE}} locks               # Active file locks
{{GATE}} dag                 # Dependency graph
{{GATE}} dispatchable        # Tasks ready to work on
{{GATE}} attach <task-id>    # View context attachments
```

## Context attachments (auto-injected on assign)

When you run `assign`, MergeGate automatically attaches:
- GitHub Issue body
- Impact analysis result
- Obsidian vault notes matching the task title (if OBSIDIAN_VAULT_PATH is set)
- Wikilinks from those notes (expanded recursively)
- First 30 lines of each locked file
- First 30 lines of depth-1 impact files (direct callers)

Use `{{GATE}} attach <task-id>` to inspect the attachments.
Use `{{GATE}} --format json status` or `{{GATE}} --format json locks`
when you need machine-readable output from supported commands.
for programmatic injection into prompts.

## Emergency commands

```
{{GATE}} force-unlock <task-id> --reason "why" --operator <name>
{{GATE}} manual-complete <task-id> --reason "why" --operator <name>
{{GATE}} heartbeat --all    # Renew all lease heartbeats
```

## Exit codes

  0 = success
  1 = GATE rejected (fix the condition and retry)
  2 = input error (fix the command)

## Quality checks (run before every commit)

```
cargo test --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Self-improvement

```
{{GATE}} dream               # Extract learnings from event log
{{GATE}} dream --auto        # Also write High learnings to docs/ and update SKILL.md
{{GATE}} serve               # Web dashboard at localhost:4848
```

## Command Reference

### init
  Initialize project memory in the current repo.
  {{GATE}} init

### register
  Register a new task. Creates an entry in tasks.json.
  {{GATE}} register --issue <N> --title "Title"
  {{GATE}} register --issue <N> --title "Title" --completion-mode manual
  {{GATE}} register --issue <N> --title "Title" --dependencies dep-1,dep-2
  {{GATE}} register --issue <N> --title "Title" --no-bus
  Options:
    --issue <N>              GitHub issue number (required, 0 = auto-create)
    --title <TEXT>           Task title (required)
    --task-id <ID>           Explicit ID (default: issue-N)
    --dependencies <IDs>     Comma-separated hard dependency task IDs
    --soft-dependencies <IDs> Comma-separated soft dependency task IDs
    --priority <N>           Priority score (default: 0)
    --completion-mode <MODE> github-pr (default) | manual | external-op
    --no-bus                 Skip skill-bus auto-enqueue

### impact
  Record impact analysis for a task.
  {{GATE}} impact <task-id> --risk low --symbols 3
  {{GATE}} impact <task-id> --risk high --symbols 12 --approve
  {{GATE}} impact <task-id> --risk medium --symbols 5 --depth1 "src/a.rs,src/b.rs"
  Options:
    --risk <LEVEL>           low | medium | high | critical
    --symbols <N>            Number of affected symbols
    --approve                Required for high/critical risk
    --depth1 <FILES>         Comma-separated depth-1 impact files
    --analyzed-commit <SHA>  Git commit used for analysis
    --input-hash <HASH>      Hash of analysis input

### assign
  Acquire file locks and start implementation.
  {{GATE}} assign <task-id> --agent codex --node macbook --files "src/main.rs,src/lib.rs"
  Options:
    --agent <NAME>           Agent name (required)
    --node <NAME>            Machine name (required)
    --files <PATHS>          Comma-separated file paths to lock (required)

### branch
  Record branch creation.
  {{GATE}} branch <task-id> feature/issue-45-auth

### pr
  Record PR creation.
  {{GATE}} pr <task-id> 88

### merge
  Record merge verification. Releases locks and unblocks dependents.
  {{GATE}} merge <task-id> <40-char-SHA>

### status
  Show task status.
  {{GATE}} status              # All tasks
  {{GATE}} status <task-id>    # One task
  {{GATE}} --format json status

### locks
  List active file locks.
  {{GATE}} locks
  {{GATE}} --format json locks

### dag
  Show DAG dependency levels.
  {{GATE}} dag

### validate
  Validate snapshot consistency.
  {{GATE}} validate
  {{GATE}} --format json validate

### dispatchable
  Show tasks ready to be worked on (dependencies resolved, no lock).
  {{GATE}} dispatchable

### export-json
  Export tasks as filtered JSON.
  {{GATE}} export-json
  {{GATE}} export-json --state implementing --risk HIGH
  {{GATE}} export-json --since 2026-04-12T00:00:00Z

### export-md
  Export tasks as filtered Markdown.
  {{GATE}} export-md
  {{GATE}} export-md --state pending --since 2026-04-12

### stats
  Show aggregate task statistics.
  {{GATE}} stats
  {{GATE}} --format json stats

### attach
  View context attachments for a task.
  {{GATE}} attach <task-id>

### refresh
  Force-refresh context attachments (clears cache).
  {{GATE}} refresh <task-id>

### verify-merge
  Verify merge state via GitHub API.
  {{GATE}} verify-merge <task-id> --repo owner/repo

### force-unlock
  Emergency: release a lock without completing the task.
  {{GATE}} force-unlock <task-id> --reason "why" --operator "name"

### manual-complete
  Complete a task without merge verification (for doc/ops tasks).
  {{GATE}} manual-complete <task-id> --reason "why" --operator "name"

### dream
  Analyze event logs and extract learnings.
  {{GATE}} dream
  {{GATE}} dream --since 24h
  {{GATE}} dream --auto
  {{GATE}} dream --auto --vault-path /path/to/obsidian
  Options:
    --since <DURATION>       Filter events (e.g. 24h, 7d, 30m)
    --auto                   Write High learnings to docs/ + update SKILL.md
    --vault-path <PATH>      Obsidian vault for exported learnings

### heartbeat
  Renew lock lease heartbeats.
  {{GATE}} heartbeat --all

### serve
  Start web dashboard.
  {{GATE}} serve
  {{GATE}} serve --port 8080
  Options:
    --port <N>               Port number (default: 4848)

### guide
  Print this guide.
  {{GATE}} guide

### Global options (apply to all gate commands)
  --format <text|json>       Output format (default: text)
  --store-path <PATH>        Path to tasks.json (default: project_memory/tasks.json)
"#;

const POLARIS_DASHBOARD_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>MergeGate Dashboard</title>
  <style>
    :root {
      color-scheme: light;
      --bg: #f5f7fb;
      --panel: rgba(255,255,255,0.92);
      --panel-strong: #ffffff;
      --text: #172033;
      --muted: #667085;
      --border: #d9e1ec;
      --shadow: 0 16px 40px rgba(15, 23, 42, 0.08);
      --success: #15803d;
      --warning: #b45309;
      --danger: #b42318;
      --info: #2563eb;
      --neutral: #475467;
      --hero-a: #eff6ff;
      --hero-b: #fdf2f8;
      --hero-c: #f8fafc;
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      font-family: ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      color: var(--text);
      background:
        radial-gradient(circle at top left, rgba(37, 99, 235, 0.08), transparent 32%),
        radial-gradient(circle at top right, rgba(190, 24, 93, 0.08), transparent 28%),
        linear-gradient(180deg, var(--hero-a) 0%, var(--hero-b) 35%, var(--hero-c) 100%);
    }
    .shell {
      max-width: 1240px;
      margin: 0 auto;
      padding: 18px;
    }
    .panel, .hero {
      background: var(--panel);
      border: 1px solid var(--border);
      border-radius: 24px;
      box-shadow: var(--shadow);
      backdrop-filter: blur(8px);
    }
    .hero {
      padding: 22px;
      margin-bottom: 18px;
      display: grid;
      gap: 18px;
    }
    .hero-top {
      display: flex;
      justify-content: space-between;
      align-items: flex-start;
      gap: 16px;
    }
    h1 {
      margin: 0 0 8px;
      font-size: clamp(1.9rem, 4vw, 3rem);
      line-height: 1.05;
    }
    h2 {
      margin: 0 0 14px;
      font-size: 1rem;
      letter-spacing: 0.01em;
    }
    h3 {
      margin: 0 0 8px;
      font-size: 0.98rem;
    }
    p { margin: 0; }
    .subtitle, .meta, .muted, .list-meta {
      color: var(--muted);
    }
    .hero-status {
      display: inline-flex;
      align-items: center;
      gap: 10px;
      padding: 12px 16px;
      border-radius: 999px;
      font-weight: 700;
      font-size: 0.95rem;
      border: 1px solid transparent;
      white-space: nowrap;
    }
    .hero-status.clean {
      color: var(--success);
      background: rgba(21, 128, 61, 0.12);
      border-color: rgba(21, 128, 61, 0.2);
    }
    .hero-status.warning {
      color: var(--warning);
      background: rgba(180, 83, 9, 0.12);
      border-color: rgba(180, 83, 9, 0.2);
    }
    .hero-status.error {
      color: var(--danger);
      background: rgba(180, 35, 24, 0.12);
      border-color: rgba(180, 35, 24, 0.2);
    }
    .hero-summary {
      display: grid;
      gap: 14px;
      grid-template-columns: repeat(4, minmax(0, 1fr));
    }
    .metric {
      background: var(--panel-strong);
      border: 1px solid var(--border);
      border-radius: 18px;
      padding: 16px;
    }
    .metric-label {
      color: var(--muted);
      font-size: 0.85rem;
      margin-bottom: 8px;
    }
    .metric-value {
      font-size: clamp(1.5rem, 3vw, 2.2rem);
      font-weight: 800;
      line-height: 1;
    }
    .metric-note {
      margin-top: 8px;
      color: var(--muted);
      font-size: 0.88rem;
    }
    .layout {
      display: grid;
      gap: 18px;
      grid-template-columns: 1.15fr 0.85fr;
    }
    .stack {
      display: grid;
      gap: 18px;
    }
    .panel {
      padding: 18px;
    }
    .section-head {
      display: flex;
      justify-content: space-between;
      align-items: baseline;
      gap: 12px;
      margin-bottom: 14px;
    }
    .section-kicker {
      color: var(--muted);
      font-size: 0.86rem;
    }
    .action-list, .task-list, .detail-list {
      list-style: none;
      margin: 0;
      padding: 0;
      display: grid;
      gap: 12px;
    }
    .action-item, .task-item, .detail-item {
      border: 1px solid var(--border);
      border-radius: 18px;
      background: var(--panel-strong);
      padding: 14px;
    }
    .action-item {
      display: grid;
      gap: 6px;
    }
    .action-label {
      display: inline-flex;
      align-items: center;
      gap: 8px;
      font-size: 0.82rem;
      font-weight: 800;
      letter-spacing: 0.02em;
      text-transform: uppercase;
    }
    .action-label.error { color: var(--danger); }
    .action-label.warning { color: var(--warning); }
    .action-label.info { color: var(--info); }
    .task-top {
      display: flex;
      justify-content: space-between;
      align-items: flex-start;
      gap: 10px;
      margin-bottom: 8px;
    }
    .task-title {
      font-weight: 700;
      line-height: 1.35;
      word-break: break-word;
    }
    .task-id {
      color: var(--muted);
      font-size: 0.85rem;
      margin-top: 3px;
    }
    .task-meta {
      font-size: 0.9rem;
      color: var(--muted);
    }
    .badge {
      display: inline-flex;
      justify-content: center;
      align-items: center;
      min-width: 96px;
      padding: 5px 11px;
      border-radius: 999px;
      color: white;
      font-size: 0.8rem;
      font-weight: 800;
      text-transform: lowercase;
      letter-spacing: 0.01em;
    }
    .badge.pending { background: var(--neutral); }
    .badge.implementing { background: var(--info); }
    .badge.reviewing { background: #7c3aed; }
    .badge.done, .badge.merged { background: var(--success); }
    .badge.blocked, .badge.failed, .badge.cancelled { background: var(--danger); }
    .badge.draft, .badge.analyzing, .badge.deploying, .badge.awaiting_github_sync {
      background: #64748b;
    }
    .summary-strip {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      margin-top: 12px;
    }
    .pill {
      display: inline-flex;
      align-items: center;
      gap: 8px;
      border-radius: 999px;
      padding: 7px 12px;
      font-size: 0.84rem;
      font-weight: 700;
      background: #f3f6fb;
      color: var(--text);
      border: 1px solid var(--border);
    }
    .empty {
      color: var(--muted);
      font-style: italic;
      padding: 2px 0;
    }
    details {
      border: 1px solid var(--border);
      border-radius: 18px;
      background: var(--panel-strong);
      padding: 14px 16px;
    }
    summary {
      cursor: pointer;
      list-style: none;
      font-weight: 700;
    }
    summary::-webkit-details-marker { display: none; }
    .details-grid {
      margin-top: 14px;
      display: grid;
      gap: 12px;
    }
    .detail-item strong {
      display: block;
      margin-bottom: 6px;
    }
    @media (max-width: 980px) {
      .layout {
        grid-template-columns: 1fr;
      }
      .hero-summary {
        grid-template-columns: repeat(2, minmax(0, 1fr));
      }
    }
    @media (max-width: 640px) {
      .shell { padding: 12px; }
      .hero, .panel { border-radius: 18px; }
      .hero-top, .task-top {
        flex-direction: column;
        align-items: flex-start;
      }
      .hero-summary {
        grid-template-columns: 1fr;
      }
    }
  </style>
</head>
<body>
  <div class="shell">
    <section class="hero">
      <div class="hero-top">
        <div>
          <h1>MergeGate Control Tower</h1>
          <p class="subtitle">まず結論、その次にやること、そのあと詳細を見るための画面です。</p>
        </div>
        <div id="hero-status" class="hero-status clean">Loading status...</div>
      </div>
      <p class="meta" id="meta">Loading...</p>
      <div class="hero-summary">
        <div class="metric">
          <div class="metric-label">Dispatchable Tasks</div>
          <div class="metric-value" id="metric-dispatchable">-</div>
          <div class="metric-note">いま着手できるタスク数</div>
        </div>
        <div class="metric">
          <div class="metric-label">Validation Issues</div>
          <div class="metric-value" id="metric-issues">-</div>
          <div class="metric-note">lock / transition / cycle / warning</div>
        </div>
        <div class="metric">
          <div class="metric-label">Active Locks</div>
          <div class="metric-value" id="metric-locks">-</div>
          <div class="metric-note">いま競合しうるファイルロック数</div>
        </div>
        <div class="metric">
          <div class="metric-label">Completion</div>
          <div class="metric-value" id="metric-completion">-</div>
          <div class="metric-note">全体の完了率</div>
        </div>
      </div>
    </section>

    <section class="layout">
      <div class="stack">
        <article class="panel">
          <div class="section-head">
            <div>
              <h2>Next Actions</h2>
              <p class="section-kicker">ここだけ見れば、次に何をすべきかが分かります。</p>
            </div>
          </div>
          <ul id="next-actions" class="action-list"><li class="empty">Loading actions...</li></ul>
        </article>

        <article class="panel">
          <div class="section-head">
            <div>
              <h2>Work Queues</h2>
              <p class="section-kicker">着手可能、進行中、詰まり、完了を分けて表示します。</p>
            </div>
          </div>
          <div class="stack">
            <div>
              <h3>Ready To Start</h3>
              <ul id="queue-ready" class="task-list"><li class="empty">Loading...</li></ul>
            </div>
            <div>
              <h3>Needs Attention</h3>
              <ul id="queue-attention" class="task-list"><li class="empty">Loading...</li></ul>
            </div>
            <div>
              <h3>In Progress</h3>
              <ul id="queue-progress" class="task-list"><li class="empty">Loading...</li></ul>
            </div>
          </div>
        </article>
      </div>

      <div class="stack">
        <article class="panel">
          <div class="section-head">
            <div>
              <h2>Current Health</h2>
              <p class="section-kicker">今のシステム状態を短くまとめます。</p>
            </div>
          </div>
          <div id="health-summary" class="empty">Loading health...</div>
          <div id="health-pills" class="summary-strip"></div>
        </article>

        <article class="panel">
          <div class="section-head">
            <div>
              <h2>Active Locks</h2>
              <p class="section-kicker">競合の原因になりやすい箇所です。</p>
            </div>
          </div>
          <ul id="locks" class="task-list"><li class="empty">Loading locks...</li></ul>
        </article>

        <details open>
          <summary>Deeper Detail</summary>
          <div class="details-grid">
            <div class="detail-item">
              <strong>Validation Detail</strong>
              <ul id="validation-detail" class="detail-list"><li class="empty">Loading...</li></ul>
            </div>
            <div class="detail-item">
              <strong>DAG Levels</strong>
              <ul id="dag" class="detail-list"><li class="empty">Loading...</li></ul>
            </div>
          </div>
        </details>
      </div>
    </section>
  </div>
  <script>
    const ACTIVE_STATES = new Set(["analyzing", "implementing", "reviewing", "deploying"]);
    const WAITING_STATES = new Set(["draft", "pending", "blocked", "awaiting_github_sync"]);
    const DONE_STATES = new Set(["done", "merged"]);

    function escapeHtml(value) {
      return String(value)
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;")
        .replaceAll('"', "&quot;")
        .replaceAll("'", "&#39;");
    }

    function setListEmpty(id, message) {
      document.getElementById(id).innerHTML = '<li class="empty">' + escapeHtml(message) + '</li>';
    }

    function badgeClass(state) {
      return "badge " + (state || "unknown");
    }

    function taskMeta(task) {
      const deps = Array.isArray(task.dependencies) && task.dependencies.length > 0
        ? task.dependencies.join(", ")
        : "none";
      return "priority " + task.priority + " | deps: " + deps;
    }

    function renderTaskList(id, tasks, emptyMessage) {
      const el = document.getElementById(id);
      if (!Array.isArray(tasks) || tasks.length === 0) {
        setListEmpty(id, emptyMessage);
        return;
      }

      el.innerHTML = tasks.map(task => {
        const state = task.current_state || "unknown";
        return (
          '<li class="task-item">' +
            '<div class="task-top">' +
              '<div>' +
                '<div class="task-title">' + escapeHtml(task.title) + '</div>' +
                '<div class="task-id">' + escapeHtml(task.id) + '</div>' +
              '</div>' +
              '<span class="' + badgeClass(state) + '">' + escapeHtml(state) + '</span>' +
            '</div>' +
            '<div class="task-meta">' + escapeHtml(taskMeta(task)) + '</div>' +
          '</li>'
        );
      }).join("");
    }

    function renderLockList(locks) {
      const entries = Object.entries(locks || {});
      if (entries.length === 0) {
        setListEmpty("locks", "No active locks");
        return;
      }

      document.getElementById("locks").innerHTML = entries.map(([file, lock]) => (
        '<li class="task-item">' +
          '<div class="task-title">' + escapeHtml(file) + '</div>' +
          '<div class="task-meta">' +
            escapeHtml(lock.agent + "@" + lock.node + " | task: " + lock.task_id) +
          '</div>' +
        '</li>'
      )).join("");
    }

    function renderDetailList(id, values, emptyMessage) {
      if (!Array.isArray(values) || values.length === 0) {
        setListEmpty(id, emptyMessage);
        return;
      }

      document.getElementById(id).innerHTML = values
        .map(value => '<li class="detail-item"><div class="list-meta">' + escapeHtml(value) + '</div></li>')
        .join("");
    }

    function renderDag(report) {
      const levels = Array.isArray(report.levels) ? report.levels : [];
      if (levels.length === 0) {
        setListEmpty("dag", "No DAG levels available");
        return;
      }

      document.getElementById("dag").innerHTML = levels.map((level, index) => (
        '<li class="detail-item">' +
          '<strong>Level ' + index + '</strong>' +
          '<div class="list-meta">' + escapeHtml(level.length > 0 ? level.join(", ") : "empty") + '</div>' +
        '</li>'
      )).join("");
    }

    function validationIssueCount(report) {
      return (
        (report.orphaned_locks || []).length +
        (report.invalid_transitions || []).length +
        (report.circular_dependencies || []).length +
        (report.warnings || []).length
      );
    }

    function setHeroStatus(severity, issueCount, dispatchableCount) {
      const el = document.getElementById("hero-status");
      const statusText = severity === "error"
        ? "Action Needed"
        : severity === "warning"
          ? "Review Needed"
          : dispatchableCount > 0
            ? "Ready To Move"
            : "Healthy";
      el.className = "hero-status " + severity;
      el.textContent = statusText + " · " + issueCount + " issue(s)";
    }

    function renderMetrics(stats, locks, dispatchableCount, issueCount) {
      document.getElementById("metric-dispatchable").textContent = String(dispatchableCount);
      document.getElementById("metric-issues").textContent = String(issueCount);
      document.getElementById("metric-locks").textContent = String(Object.keys(locks || {}).length);
      document.getElementById("metric-completion").textContent =
        (Math.round(stats.completion_rate_pct || 0)) + "%";
    }

    function renderHealth(stats, validation, tasks, locks) {
      const parts = [];
      if (validation.severity === "error") {
        parts.push("整合性エラーがあるため、まず validation を解消する必要があります。");
      } else if (validation.severity === "warning") {
        parts.push("致命的ではない warning があります。作業前に確認すると安全です。");
      } else {
        parts.push("ledger の整合性は保たれています。");
      }

      const active = tasks.filter(task => ACTIVE_STATES.has(task.current_state)).length;
      const blocked = tasks.filter(task => task.current_state === "blocked").length;
      parts.push(active > 0 ? active + " 件のタスクが進行中です。" : "進行中タスクはありません。");
      parts.push(blocked > 0 ? blocked + " 件の blocked タスクがあります。" : "blocked タスクはありません。");

      document.getElementById("health-summary").textContent = parts.join(" ");
      document.getElementById("health-pills").innerHTML = [
        ["completed", stats.completed + " completed"],
        ["active", stats.active + " active"],
        ["waiting", stats.waiting + " waiting"],
        ["failed", stats.failed + " failed"],
        ["locks", Object.keys(locks || {}).length + " locks"]
      ].map(([_, label]) => '<span class="pill">' + escapeHtml(label) + '</span>').join("");
    }

    function renderNextActions(tasks, validation, locks) {
      const actions = [];

      if ((validation.invalid_transitions || []).length > 0 || (validation.orphaned_locks || []).length > 0) {
        actions.push({
          kind: "error",
          title: "Fix ledger consistency first",
          body: "Run `mergegate gate validate` and resolve lock or transition mismatches before assigning more work."
        });
      }

      const blocked = tasks.filter(task => task.current_state === "blocked");
      if (blocked.length > 0) {
        actions.push({
          kind: "warning",
          title: "Unblock blocked tasks",
          body: blocked.slice(0, 3).map(task => task.id).join(", ") + " need dependency or review follow-up."
        });
      }

      const ready = tasks.filter(task => task.current_state === "pending" && (!task.dependencies || task.dependencies.length === 0));
      if (ready.length > 0) {
        actions.push({
          kind: "info",
          title: "Start a ready task",
          body: ready.slice(0, 3).map(task => task.id + " — " + task.title).join(" | ")
        });
      }

      if (actions.length === 0) {
        actions.push({
          kind: "info",
          title: "No urgent action",
          body: "The ledger looks stable. Review in-progress work or wait for the next task to become dispatchable."
        });
      }

      document.getElementById("next-actions").innerHTML = actions.map(action => (
        '<li class="action-item">' +
          '<div class="action-label ' + action.kind + '">' + escapeHtml(action.kind) + '</div>' +
          '<div class="task-title">' + escapeHtml(action.title) + '</div>' +
          '<div class="task-meta">' + escapeHtml(action.body) + '</div>' +
        '</li>'
      )).join("");
    }

    function renderQueues(tasks, validation) {
      const ready = tasks.filter(task => task.current_state === "pending");
      const attention = tasks.filter(task =>
        task.current_state === "blocked" ||
        task.current_state === "failed" ||
        task.current_state === "awaiting_github_sync"
      );
      const progress = tasks.filter(task => ACTIVE_STATES.has(task.current_state));

      renderTaskList("queue-ready", ready.slice(0, 6), "No ready tasks");
      renderTaskList("queue-attention", attention.slice(0, 6), validation.severity === "error" ? "Validation has the main issue right now" : "No tasks need attention");
      renderTaskList("queue-progress", progress.slice(0, 6), "No work in progress");
    }

    function renderValidationDetail(report) {
      const details = [];
      for (const item of report.orphaned_locks || []) details.push("orphaned lock: " + item);
      for (const item of report.invalid_transitions || []) details.push("invalid transition: " + item);
      for (const item of report.circular_dependencies || []) details.push("cycle: " + item);
      for (const item of report.warnings || []) details.push("warning: " + item);
      renderDetailList("validation-detail", details, "No validation issues");
    }

    async function refresh() {
      try {
        const [tasksRes, statsRes, validateRes, locksRes, dagRes] = await Promise.all([
          fetch("/api/tasks"),
          fetch("/api/stats"),
          fetch("/api/validate"),
          fetch("/api/locks"),
          fetch("/api/dag")
        ]);

        if (!tasksRes.ok || !statsRes.ok || !validateRes.ok || !locksRes.ok || !dagRes.ok) {
          throw new Error("API request failed");
        }

        const [snapshot, stats, validation, locks, dag] = await Promise.all([
          tasksRes.json(),
          statsRes.json(),
          validateRes.json(),
          locksRes.json(),
          dagRes.json()
        ]);

        const tasks = Array.isArray(snapshot.tasks) ? snapshot.tasks : [];
        const dispatchableCount = tasks.filter(task => task.current_state === "pending").length;
        const issueCount = validationIssueCount(validation);

        setHeroStatus(validation.severity || "clean", issueCount, dispatchableCount);
        renderMetrics(stats, locks, dispatchableCount, issueCount);
        renderHealth(stats, validation, tasks, locks);
        renderNextActions(tasks, validation, locks);
        renderQueues(tasks, validation);
        renderLockList(locks);
        renderValidationDetail(validation);
        renderDag(dag);

        document.getElementById("meta").textContent =
          "Snapshot version " + snapshot.version +
          " · updated " + snapshot.generated_at +
          " · auto-refresh every 3s";
      } catch (error) {
        document.getElementById("hero-status").className = "hero-status error";
        document.getElementById("hero-status").textContent = "Dashboard Unavailable";
        document.getElementById("meta").textContent = "Refresh failed: " + error.message;
        document.getElementById("health-summary").textContent = "Dashboard data could not be loaded.";
        document.getElementById("health-pills").innerHTML = "";
        setListEmpty("next-actions", "Failed to load actions");
        setListEmpty("queue-ready", "Failed to load tasks");
        setListEmpty("queue-attention", "Failed to load tasks");
        setListEmpty("queue-progress", "Failed to load tasks");
        setListEmpty("locks", "Failed to load locks");
        setListEmpty("validation-detail", "Failed to load validation");
        setListEmpty("dag", "Failed to load DAG");
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
    println!("MergeGate Dashboard listening on http://127.0.0.1:{port}");

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                if let Err(error) = handle_dashboard_connection(&protocol, store_path, &mut stream)
                {
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
    store_path: &std::path::Path,
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
        "/api/tasks" | "/api/status" => {
            let body =
                serde_json::to_vec_pretty(&load_snapshot(store_path)?).map_err(io::Error::other)?;
            write_http_response(stream, "200 OK", "application/json; charset=utf-8", &body)?;
        }
        "/api/stats" => {
            let stats = miyabi_core::stats::compute_stats(&load_snapshot(store_path)?);
            let body = serde_json::to_vec_pretty(&stats).map_err(io::Error::other)?;
            write_http_response(stream, "200 OK", "application/json; charset=utf-8", &body)?;
        }
        "/api/validate" => {
            let report = miyabi_core::validate::validate_snapshot(&load_snapshot(store_path)?);
            let body = serde_json::to_vec_pretty(&validation_report_json(&report))
                .map_err(io::Error::other)?;
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

fn bus_enqueue(task_id: &str, title: &str, store_path: &std::path::Path) {
    // Derive repo root from store_path (e.g. project_memory/tasks.json -> repo root)
    let repo_root = store_path
        .parent()
        .and_then(|pm| pm.parent())
        .unwrap_or(std::path::Path::new("."));
    let skill_runs_path = repo_root.join("skills/self-improving-skills/skill-runs.jsonl");

    if let Some(path) = Some(skill_runs_path).filter(|p| p.parent().is_some_and(|d| d.exists())) {
        let entry = serde_json::json!({
            "ts": chrono::Utc::now().to_rfc3339(),
            "agent": std::env::var("POLARIS_AGENT_ID").unwrap_or_else(|_| "system".into()),
            "skill": "mergegate-ops",
            "task": format!("register: {title} ({task_id})"),
            "result": "queued",
            "score": 0.0,
            "notes": "auto-enqueued on register"
        });
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            use std::io::Write;
            let _ = writeln!(
                file,
                "{}",
                serde_json::to_string(&entry).unwrap_or_default()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use miyabi_core::store::{ExecutionTask, TaskState, TasksSnapshot};
    use miyabi_core::validate::validate_snapshot;

    #[test]
    fn parse_export_since_accepts_rfc3339_and_date_only() {
        assert_eq!(
            parse_export_since("2026-04-12T09:30:00Z").unwrap(),
            Utc.with_ymd_and_hms(2026, 4, 12, 9, 30, 0).unwrap()
        );
        assert_eq!(
            parse_export_since("2026-04-12").unwrap(),
            Utc.with_ymd_and_hms(2026, 4, 12, 0, 0, 0).unwrap()
        );
    }

    #[test]
    fn parse_export_filter_returns_none_when_empty() {
        let filter = parse_export_filter(ExportFilterArgs {
            state: None,
            risk: None,
            since: None,
        })
        .unwrap();

        assert!(filter.is_none());
    }

    #[test]
    fn parse_export_filter_normalises_state_and_risk_case() {
        let filter = parse_export_filter(ExportFilterArgs {
            state: Some("Implementing".to_string()),
            risk: Some("high".to_string()),
            since: None,
        })
        .unwrap()
        .unwrap();

        assert_eq!(filter.state.as_deref(), Some("implementing"));
        assert_eq!(filter.risk_level.as_deref(), Some("HIGH"));
    }


    #[test]
    fn validate_returns_warning_exit_code_for_missing_dependency_reference() {
        let path = write_snapshot(TasksSnapshot {
            tasks: vec![task_with_missing_dependency("task-a")],
            ..TasksSnapshot::default()
        });

        let code =
            handle_gate_command(&OutputFormat::Json, false, &path, GateCommand::Validate).unwrap();

        assert_eq!(code, 1);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn validate_returns_error_exit_code_for_invalid_transition() {
        let path = write_snapshot(TasksSnapshot {
            tasks: vec![{
                let mut task = ExecutionTask::new("task-a", "Task A");
                task.current_state = TaskState::Implementing;
                task
            }],
            ..TasksSnapshot::default()
        });

        let code =
            handle_gate_command(&OutputFormat::Json, false, &path, GateCommand::Validate).unwrap();

        assert_eq!(code, 2);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn validation_report_json_includes_severity_and_exit_code() {
        let report = validate_snapshot(&TasksSnapshot {
            tasks: vec![{
                let mut task = ExecutionTask::new("task-a", "Task A");
                task.current_state = TaskState::Implementing;
                task
            }],
            ..TasksSnapshot::default()
        });

        let value = validation_report_json(&report);

        assert_eq!(value["severity"], "error");
        assert_eq!(value["exit_code"], 2);
        assert!(value["invalid_transitions"].is_array());
    }

    fn write_snapshot(snapshot: TasksSnapshot) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("mergegate-test-{unique}.json"));
        std::fs::write(&path, serde_json::to_vec_pretty(&snapshot).unwrap()).unwrap();
        path
    }

    fn task_with_missing_dependency(id: &str) -> ExecutionTask {
        let mut task = ExecutionTask::new(id, "Task A");
        task.dependencies = vec!["missing-task".to_string()];
        task.current_state = TaskState::Pending;
        task
    }
}
