//! Miyabi CLI - Main entry point

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

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
            println!("Miyabi Status: Ready");
            println!(
                "Config path: {:?}",
                miyabi_core::config::Config::default_path()
            );
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
    }

    Ok(())
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    } else {
        s.to_string()
    }
}
