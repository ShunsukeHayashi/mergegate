//! Miyabi OpenClaw Integration
//!
//! This module provides OpenClaw CLI wrapper functionality for Miyabi.

use reqwest::Client;
use serde::Serialize;
use std::time::Duration;

/// OpenClaw client for communicating with the Gateway
#[derive(Clone)]
pub struct OpenClawClient {
    gateway_url: String,
    token: String,
    client: Client,
}

impl OpenClawClient {
    /// Create a new OpenClaw client
    pub fn new(gateway_url: String, token: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            gateway_url,
            token,
            client,
        }
    }

    /// Send a message to an agent via the Gateway
    pub async fn send(&self, agent: &str, message: &str) -> Result<String, OpenClawError> {
        let payload = SendMessageRequest {
            agent_id: agent.to_string(),
            message: message.to_string(),
        };

        let response = self.client
            .post(format!("{}/api/message", self.gateway_url))
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&payload)
            .send()
            .await
            .map_err(|e| OpenClawError::Network(e.to_string()))?;

        if response.status().is_success() {
            Ok("Message sent successfully".to_string())
        } else {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            Err(OpenClawError::Api(status, body))
        }
    }

    /// Broadcast a message to all core agents
    pub async fn broadcast(&self, message: &str) -> Result<Vec<String>, OpenClawError> {
        let agents = ["maestro", "kade", "sakura", "tsubaki", "botan", "nagare"];
        let mut results = Vec::new();

        for agent in agents {
            match self.send(agent, message).await {
                Ok(msg) => results.push(format!("{}: {}", agent, msg)),
                Err(e) => results.push(format!("{}: {}", agent, e)),
            }
        }

        Ok(results)
    }

    /// Get agent list
    pub fn get_agents() -> Vec<AgentInfo> {
        vec![
            // Core Society
            AgentInfo {
                id: "maestro".to_string(),
                name: "しきるん".to_string(),
                emoji: "🎭".to_string(),
                role: "Conductor".to_string(),
                society: "Core".to_string(),
            },
            AgentInfo {
                id: "kade".to_string(),
                name: "カエデ".to_string(),
                emoji: "🍁".to_string(),
                role: "CodeGen".to_string(),
                society: "Core".to_string(),
            },
            AgentInfo {
                id: "sakura".to_string(),
                name: "サクラ".to_string(),
                emoji: "🌸".to_string(),
                role: "Review".to_string(),
                society: "Core".to_string(),
            },
            AgentInfo {
                id: "tsubaki".to_string(),
                name: "ツバキ".to_string(),
                emoji: "🌺".to_string(),
                role: "PR".to_string(),
                society: "Core".to_string(),
            },
            AgentInfo {
                id: "botan".to_string(),
                name: "ボタン".to_string(),
                emoji: "🌼".to_string(),
                role: "Deploy".to_string(),
                society: "Core".to_string(),
            },
            AgentInfo {
                id: "nagare".to_string(),
                name: "ながれるん".to_string(),
                emoji: "🌊".to_string(),
                role: "Workflow".to_string(),
                society: "Core".to_string(),
            },
            // Investment Society
            AgentInfo {
                id: "scout".to_string(),
                name: "スカウト".to_string(),
                emoji: "🔍".to_string(),
                role: "Explorer".to_string(),
                society: "Investment".to_string(),
            },
            AgentInfo {
                id: "crystal".to_string(),
                name: "クリスタル".to_string(),
                emoji: "💎".to_string(),
                role: "Valuer".to_string(),
                society: "Investment".to_string(),
            },
            AgentInfo {
                id: "dealer".to_string(),
                name: "ディーラー".to_string(),
                emoji: "🎰".to_string(),
                role: "Trader".to_string(),
                society: "Investment".to_string(),
            },
            AgentInfo {
                id: "sentinel".to_string(),
                name: "センチネル".to_string(),
                emoji: "🛡️".to_string(),
                role: "Risk Manager".to_string(),
                society: "Investment".to_string(),
            },
            AgentInfo {
                id: "architect".to_string(),
                name: "アーキテクト".to_string(),
                emoji: "🏗️".to_string(),
                role: "Portfolio Manager".to_string(),
                society: "Investment".to_string(),
            },
            AgentInfo {
                id: "watchman".to_string(),
                name: "ウォッチマン".to_string(),
                emoji: "👁️".to_string(),
                role: "News Monitor".to_string(),
                society: "Investment".to_string(),
            },
            AgentInfo {
                id: "chart".to_string(),
                name: "チャート".to_string(),
                emoji: "📈".to_string(),
                role: "Technical Analyst".to_string(),
                society: "Investment".to_string(),
            },
            AgentInfo {
                id: "fundy".to_string(),
                name: "ファンディ".to_string(),
                emoji: "📊".to_string(),
                role: "Fundamental Analyst".to_string(),
                society: "Investment".to_string(),
            },
            AgentInfo {
                id: "scribe".to_string(),
                name: "スクライブ".to_string(),
                emoji: "📝".to_string(),
                role: "Reporter".to_string(),
                society: "Investment".to_string(),
            },
            // Content Society
            AgentInfo {
                id: "tweeter".to_string(),
                name: "ツイーター".to_string(),
                emoji: "🐦".to_string(),
                role: "X Specialist".to_string(),
                society: "Content".to_string(),
            },
            AgentInfo {
                id: "pen".to_string(),
                name: "ペン".to_string(),
                emoji: "✒️".to_string(),
                role: "Writer".to_string(),
                society: "Content".to_string(),
            },
            AgentInfo {
                id: "vidpro".to_string(),
                name: "ビッドプロ".to_string(),
                emoji: "🎬".to_string(),
                role: "Video Producer".to_string(),
                society: "Content".to_string(),
            },
            AgentInfo {
                id: "artist".to_string(),
                name: "アーティスト".to_string(),
                emoji: "🎨".to_string(),
                role: "Designer".to_string(),
                society: "Content".to_string(),
            },
            AgentInfo {
                id: "optimizer".to_string(),
                name: "オプティマイザー".to_string(),
                emoji: "🔧".to_string(),
                role: "SEO Specialist".to_string(),
                society: "Content".to_string(),
            },
            AgentInfo {
                id: "scheduler".to_string(),
                name: "スケジューラー".to_string(),
                emoji: "📅".to_string(),
                role: "Calendar Manager".to_string(),
                society: "Content".to_string(),
            },
            // Marketing Society
            AgentInfo {
                id: "hiro".to_string(),
                name: "ヒロ".to_string(),
                emoji: "🚀".to_string(),
                role: "Promoter".to_string(),
                society: "Marketing".to_string(),
            },
            AgentInfo {
                id: "kazoeru".to_string(),
                name: "カゾエル".to_string(),
                emoji: "🔢".to_string(),
                role: "Metrics Tracker".to_string(),
                society: "Marketing".to_string(),
            },
            AgentInfo {
                id: "funnel".to_string(),
                name: "ファネル".to_string(),
                emoji: "🌪️".to_string(),
                role: "CRO Specialist".to_string(),
                society: "Marketing".to_string(),
            },
            AgentInfo {
                id: "adops".to_string(),
                name: "アドオプス".to_string(),
                emoji: "📢".to_string(),
                role: "Ad Manager".to_string(),
                society: "Marketing".to_string(),
            },
        ]
    }

    /// Resolve agent alias to canonical ID
    pub fn resolve_agent_alias(alias: &str) -> String {
        match alias {
            // Core Society - maestro
            "shikirun" | "conductor" | "orchestrator" => "maestro".to_string(),
            // Core Society - kade
            "kaede" | "creator" | "codegen" | "developer" => "kade".to_string(),
            // Core Society - sakura
            "reviewer" | "qa" | "critic" => "sakura".to_string(),
            // Core Society - tsubaki
            "integrator" | "pr-manager" | "merge-bot" => "tsubaki".to_string(),
            // Core Society - botan
            "deployer" | "release-manager" | "deployment" => "botan".to_string(),
            // Core Society - nagare
            "nagarerun" | "workflow" | "automation" | "n8n-specialist" => "nagare".to_string(),

            // Investment Society - scout
            "researcher" | "explorer" => "scout".to_string(),
            // Investment Society - crystal
            "valuer" | "analyst" => "crystal".to_string(),
            // Investment Society - dealer
            "trader" | "executor" => "dealer".to_string(),
            // Investment Society - sentinel
            "risk-manager" | "guardian-rm" => "sentinel".to_string(),
            // Investment Society - architect
            "portfolio-manager" | "allocator" | "architect-inv" => "architect".to_string(),
            // Investment Society - watchman
            "news-monitor" | "sentinel-news" => "watchman".to_string(),
            // Investment Society - chart
            "technical-analyst" | "chart-reader" => "chart".to_string(),
            // Investment Society - fundy
            "fundamental-analyst" | "value-investor" => "fundy".to_string(),
            // Investment Society - scribe
            "reporter" | "documenter" | "scribe-inv" => "scribe".to_string(),

            // Content Society - tweeter
            "twitter-specialist" | "x-poster" => "tweeter".to_string(),
            // Content Society - pen
            "writer" | "author" => "pen".to_string(),
            // Content Society - vidpro
            "video-producer" | "youtuber" => "vidpro".to_string(),
            // Content Society - artist
            "designer" | "visual-creator" => "artist".to_string(),
            // Content Society - optimizer
            "seo-specialist" | "seo-analyst" => "optimizer".to_string(),
            // Content Society - scheduler
            "calendar-manager" | "planner" => "scheduler".to_string(),

            // Marketing Society - hiro
            "promoter" | "growth-hacker" => "hiro".to_string(),
            // Marketing Society - kazoeru
            "metrics-tracker" | "data-analyst" => "kazoeru".to_string(),
            // Marketing Society - funnel
            "conversion-optimizer" | "cro-specialist" => "funnel".to_string(),
            // Marketing Society - adops
            "ad-manager" | "media-buyer" => "adops".to_string(),

            // default - return as-is
            _ => alias.to_string(),
        }
    }
}

/// Agent information
#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub emoji: String,
    pub role: String,
    pub society: String,
}

/// Request for sending a message
#[derive(Serialize)]
struct SendMessageRequest {
    #[serde(rename = "agentId")]
    agent_id: String,
    message: String,
}

/// OpenClaw error types
#[derive(Debug, thiserror::Error)]
pub enum OpenClawError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("API error (status {0}): {1}")]
    Api(u16, String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Agent '{0}' not found")]
    AgentNotFound(String),
}

/// Result type for OpenClaw operations
pub type OpenClawResult<T> = Result<T, OpenClawError>;
