//! Approval callback system for tool execution

use async_trait::async_trait;
use serde_json::Value;

use super::executor::RiskLevel;

/// Decision from approval callback
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalDecision {
    /// Tool execution approved
    Approved,
    /// Tool execution rejected with optional reason
    Rejected(Option<String>),
    /// Request to modify the input before execution
    ModifyInput(Value),
}

/// Request sent to approval callback
#[derive(Debug, Clone)]
pub struct ApprovalRequest {
    /// Tool use ID from Claude
    pub id: String,
    /// Tool name
    pub name: String,
    /// Tool input parameters
    pub input: Value,
    /// Risk level of the tool
    pub risk_level: RiskLevel,
    /// Description of what the tool does
    pub description: String,
}

/// Trait for implementing custom approval handlers
///
/// Implement this trait to create custom approval UIs or automated
/// approval logic based on tool risk levels.
///
/// # Example
///
/// ```rust
/// use miyabi_core::agent::{ApprovalCallback, ApprovalRequest, ApprovalDecision};
/// use async_trait::async_trait;
///
/// struct AutoApprover;
///
/// #[async_trait]
/// impl ApprovalCallback for AutoApprover {
///     async fn request_approval(&self, request: &ApprovalRequest) -> ApprovalDecision {
///         // Auto-approve low risk tools
///         match request.risk_level {
///             miyabi_core::agent::RiskLevel::Low => ApprovalDecision::Approved,
///             _ => ApprovalDecision::Rejected(Some("Manual approval required".to_string())),
///         }
///     }
/// }
/// ```
#[async_trait]
pub trait ApprovalCallback: Send + Sync {
    /// Request approval for tool execution
    ///
    /// This method is called before executing a tool that requires approval.
    /// The implementation should decide whether to approve, reject, or modify
    /// the tool execution.
    async fn request_approval(&self, request: &ApprovalRequest) -> ApprovalDecision;

    /// Called when tool execution is completed (optional)
    async fn on_tool_completed(&self, _request: &ApprovalRequest, _output: &str) {}

    /// Called when tool execution fails (optional)
    async fn on_tool_failed(&self, _request: &ApprovalRequest, _error: &str) {}
}

/// Default approval callback that auto-approves everything
///
/// This is used when no custom approval callback is provided.
/// Use with caution in production environments.
pub struct AutoApproveAll;

#[async_trait]
impl ApprovalCallback for AutoApproveAll {
    async fn request_approval(&self, _request: &ApprovalRequest) -> ApprovalDecision {
        ApprovalDecision::Approved
    }
}

/// Approval callback that requires all high-risk tools to be rejected
pub struct RejectHighRisk;

#[async_trait]
impl ApprovalCallback for RejectHighRisk {
    async fn request_approval(&self, request: &ApprovalRequest) -> ApprovalDecision {
        match request.risk_level {
            RiskLevel::Low | RiskLevel::Medium => ApprovalDecision::Approved,
            RiskLevel::High | RiskLevel::Critical => ApprovalDecision::Rejected(Some(format!(
                "Tool {} requires manual approval (risk level: {})",
                request.name,
                request.risk_level.as_str()
            ))),
        }
    }
}

/// Approval callback that uses an async channel for interactive approval
///
/// This is useful for TUI/GUI applications that need to present
/// approval requests to the user.
pub struct ChannelApprover {
    /// Channel to send approval requests
    request_tx: tokio::sync::mpsc::Sender<ApprovalRequest>,
    /// Channel to receive approval decisions
    decision_rx: std::sync::Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<ApprovalDecision>>>,
}

impl ChannelApprover {
    /// Create a new channel-based approver
    pub fn new() -> (
        Self,
        tokio::sync::mpsc::Receiver<ApprovalRequest>,
        tokio::sync::mpsc::Sender<ApprovalDecision>,
    ) {
        let (request_tx, request_rx) = tokio::sync::mpsc::channel(10);
        let (decision_tx, decision_rx) = tokio::sync::mpsc::channel(10);

        let approver = Self {
            request_tx,
            decision_rx: std::sync::Arc::new(tokio::sync::Mutex::new(decision_rx)),
        };

        (approver, request_rx, decision_tx)
    }
}

#[async_trait]
impl ApprovalCallback for ChannelApprover {
    async fn request_approval(&self, request: &ApprovalRequest) -> ApprovalDecision {
        // Send request through channel
        if self.request_tx.send(request.clone()).await.is_err() {
            return ApprovalDecision::Rejected(Some("Approval channel closed".to_string()));
        }

        // Wait for decision
        let mut rx = self.decision_rx.lock().await;
        match rx.recv().await {
            Some(decision) => decision,
            None => ApprovalDecision::Rejected(Some("No approval decision received".to_string())),
        }
    }
}

impl Default for ChannelApprover {
    fn default() -> Self {
        Self::new().0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_auto_approve_all() {
        let approver = AutoApproveAll;
        let request = ApprovalRequest {
            id: "test-1".to_string(),
            name: "bash".to_string(),
            input: serde_json::json!({"command": "ls"}),
            risk_level: RiskLevel::High,
            description: "Execute bash command".to_string(),
        };

        let decision = approver.request_approval(&request).await;
        assert_eq!(decision, ApprovalDecision::Approved);
    }

    #[tokio::test]
    async fn test_reject_high_risk() {
        let approver = RejectHighRisk;

        // Low risk should be approved
        let low_risk = ApprovalRequest {
            id: "test-1".to_string(),
            name: "read".to_string(),
            input: serde_json::json!({"path": "/test"}),
            risk_level: RiskLevel::Low,
            description: "Read file".to_string(),
        };
        assert_eq!(
            approver.request_approval(&low_risk).await,
            ApprovalDecision::Approved
        );

        // High risk should be rejected
        let high_risk = ApprovalRequest {
            id: "test-2".to_string(),
            name: "bash".to_string(),
            input: serde_json::json!({"command": "rm -rf /"}),
            risk_level: RiskLevel::High,
            description: "Execute bash command".to_string(),
        };
        match approver.request_approval(&high_risk).await {
            ApprovalDecision::Rejected(_) => {}
            _ => panic!("Expected rejection"),
        }
    }

    #[tokio::test]
    async fn test_channel_approver() {
        let (approver, mut request_rx, decision_tx) = ChannelApprover::new();

        let request = ApprovalRequest {
            id: "test-1".to_string(),
            name: "write".to_string(),
            input: serde_json::json!({"path": "/test", "content": "data"}),
            risk_level: RiskLevel::Medium,
            description: "Write file".to_string(),
        };

        // Spawn task to handle approval
        let handle = tokio::spawn(async move {
            // Receive request
            let _req = request_rx.recv().await.unwrap();
            // Send approval
            decision_tx.send(ApprovalDecision::Approved).await.unwrap();
        });

        let decision = approver.request_approval(&request).await;
        assert_eq!(decision, ApprovalDecision::Approved);

        handle.await.unwrap();
    }
}
