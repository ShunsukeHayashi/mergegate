//! Integration tests for Agent module

use super::*;
use crate::anthropic::AnthropicClient;

#[cfg(test)]
mod approval_integration_tests {
    use super::*;

    #[test]
    fn test_agent_with_auto_approve_all() {
        let client = AnthropicClient::new("test-key".to_string()).unwrap();
        let registry = ExecutorRegistry::with_standard_tools();

        // Agent should be created successfully with AutoApproveAll callback
        let _agent = Agent::new(client, registry)
            .with_approval_callback(AutoApproveAll);
    }

    #[test]
    fn test_agent_with_reject_high_risk() {
        let client = AnthropicClient::new("test-key".to_string()).unwrap();
        let registry = ExecutorRegistry::with_standard_tools();

        // Agent should be created successfully with RejectHighRisk callback
        let _agent = Agent::new(client, registry)
            .with_approval_callback(RejectHighRisk);
    }

    #[test]
    fn test_agent_with_custom_config() {
        let client = AnthropicClient::new("test-key".to_string()).unwrap();
        let registry = ExecutorRegistry::with_standard_tools();

        let config = AgentConfig {
            max_iterations: 20,
            ..AgentConfig::default()
        };

        let _agent = Agent::new(client, registry)
            .with_config(config)
            .with_approval_callback(AutoApproveAll);
    }

    #[tokio::test]
    async fn test_approval_request_creation() {
        let request = ApprovalRequest {
            id: "test-123".to_string(),
            name: "bash".to_string(),
            input: serde_json::json!({"command": "ls -la"}),
            risk_level: RiskLevel::High,
            description: "Execute bash command".to_string(),
        };

        assert_eq!(request.name, "bash");
        assert_eq!(request.risk_level, RiskLevel::High);
    }

    #[tokio::test]
    async fn test_approval_decision_types() {
        // Test Approved
        let approved = ApprovalDecision::Approved;
        assert_eq!(approved, ApprovalDecision::Approved);

        // Test Rejected
        let rejected = ApprovalDecision::Rejected(Some("Not allowed".to_string()));
        match rejected {
            ApprovalDecision::Rejected(reason) => {
                assert_eq!(reason, Some("Not allowed".to_string()));
            }
            _ => panic!("Expected Rejected"),
        }

        // Test ModifyInput
        let new_input = serde_json::json!({"command": "ls"});
        let modified = ApprovalDecision::ModifyInput(new_input.clone());
        match modified {
            ApprovalDecision::ModifyInput(input) => {
                assert_eq!(input, new_input);
            }
            _ => panic!("Expected ModifyInput"),
        }
    }

    #[tokio::test]
    async fn test_channel_approver_creation() {
        let (approver, mut request_rx, decision_tx) = ChannelApprover::new();

        // Test that channels are functional
        let request = ApprovalRequest {
            id: "test-1".to_string(),
            name: "write".to_string(),
            input: serde_json::json!({"path": "/test"}),
            risk_level: RiskLevel::Medium,
            description: "Write file".to_string(),
        };

        // Spawn approval handler
        let handle = tokio::spawn(async move {
            let _req = request_rx.recv().await.unwrap();
            decision_tx.send(ApprovalDecision::Approved).await.unwrap();
        });

        let decision = approver.request_approval(&request).await;
        assert_eq!(decision, ApprovalDecision::Approved);

        handle.await.unwrap();
    }

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Low < RiskLevel::Medium);
        assert!(RiskLevel::Medium < RiskLevel::High);
        assert!(RiskLevel::High < RiskLevel::Critical);
    }

    #[test]
    fn test_risk_level_requires_approval() {
        assert!(!RiskLevel::Low.requires_approval());
        assert!(!RiskLevel::Medium.requires_approval());
        assert!(RiskLevel::High.requires_approval());
        assert!(RiskLevel::Critical.requires_approval());
    }

    #[test]
    fn test_agent_config_with_custom_approval() {
        let client = AnthropicClient::new("test-key".to_string()).unwrap();
        let registry = ExecutorRegistry::with_standard_tools();

        let config = AgentConfig {
            max_iterations: 5,
            max_tokens_per_turn: 4096,
            require_approval: true,
            auto_approve_patterns: vec!["read".to_string()],
            ..AgentConfig::default()
        };

        // Agent should be created successfully with custom config
        let _agent = Agent::new(client, registry)
            .with_config(config)
            .with_approval_callback(RejectHighRisk);
    }

    #[test]
    fn test_agent_config_default_values() {
        let config = AgentConfig::default();
        assert_eq!(config.max_iterations, 10);
        assert_eq!(config.max_tokens_per_turn, 8192);
        assert!(config.require_approval);
        assert!(config.auto_approve_patterns.contains(&"read".to_string()));
        assert!(config.auto_approve_patterns.contains(&"glob".to_string()));
        assert!(config.auto_approve_patterns.contains(&"grep".to_string()));
    }
}

#[cfg(test)]
mod executor_registry_tests {
    use super::*;

    #[test]
    fn test_registry_with_standard_tools() {
        let registry = ExecutorRegistry::with_standard_tools();

        // Check all standard tools are registered (lowercase names)
        assert!(registry.get("read").is_some());
        assert!(registry.get("write").is_some());
        assert!(registry.get("edit").is_some());
        assert!(registry.get("glob").is_some());
        assert!(registry.get("grep").is_some());
        assert!(registry.get("bash").is_some());
    }

    #[test]
    fn test_registry_risk_levels() {
        let registry = ExecutorRegistry::with_standard_tools();

        // Low risk
        assert_eq!(registry.risk_level("read"), Some(RiskLevel::Low));
        assert_eq!(registry.risk_level("glob"), Some(RiskLevel::Low));
        assert_eq!(registry.risk_level("grep"), Some(RiskLevel::Low));

        // Medium risk
        assert_eq!(registry.risk_level("write"), Some(RiskLevel::Medium));
        assert_eq!(registry.risk_level("edit"), Some(RiskLevel::Medium));

        // High risk
        assert_eq!(registry.risk_level("bash"), Some(RiskLevel::High));
    }

    #[test]
    fn test_registry_requires_approval() {
        let registry = ExecutorRegistry::with_standard_tools();

        // Should not require approval (low risk)
        assert!(!registry.requires_approval("read"));
        assert!(!registry.requires_approval("glob"));

        // Should require approval (high risk)
        assert!(registry.requires_approval("bash"));

        // Unknown tool defaults to requiring approval
        assert!(registry.requires_approval("UnknownTool"));
    }

    #[test]
    fn test_registry_to_api_tools() {
        let registry = ExecutorRegistry::with_standard_tools();
        let tools = registry.to_api_tools();

        assert!(!tools.is_empty());

        // Check that tools have names and descriptions
        for tool in &tools {
            assert!(!tool.name.is_empty());
            assert!(!tool.description.is_empty());
        }
    }

    #[test]
    fn test_registry_tool_names() {
        let registry = ExecutorRegistry::with_standard_tools();
        let names = registry.tool_names();

        assert!(names.contains(&"read".to_string()));
        assert!(names.contains(&"write".to_string()));
        assert!(names.contains(&"bash".to_string()));
    }
}

#[cfg(test)]
mod agent_event_tests {
    use super::*;

    #[test]
    fn test_agent_result_clone() {
        let result = AgentResult {
            output: "Test output".to_string(),
            iterations: 3,
            tool_calls: 5,
            total_tokens: 1000,
            messages: vec![],
        };

        let cloned = result.clone();
        assert_eq!(cloned.output, result.output);
        assert_eq!(cloned.iterations, result.iterations);
        assert_eq!(cloned.tool_calls, result.tool_calls);
    }

    #[test]
    fn test_agent_error_display() {
        let error = AgentError::MaxIterationsReached(10);
        let display = format!("{}", error);
        assert!(display.contains("10"));

        let error = AgentError::MaxTokensReached;
        let display = format!("{}", error);
        assert!(!display.is_empty());
    }
}
