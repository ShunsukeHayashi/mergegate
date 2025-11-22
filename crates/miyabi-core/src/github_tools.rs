//! GitHub Tools for Agent Execution
//!
//! Tools that allow agents to interact with GitHub issues, PRs, and repositories.

use async_trait::async_trait;
use serde_json::Value;

use crate::github::{CreateIssueRequest, CreatePullRequestRequest, GitHubClient};
use crate::tool::{ParameterDef, Tool, ToolError, ToolOutput, ToolRegistry, ToolResult};

/// Tool to list GitHub issues
pub struct ListIssuesTool {
    client: GitHubClient,
}

impl ListIssuesTool {
    pub fn new(client: GitHubClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for ListIssuesTool {
    fn name(&self) -> &str {
        "github_list_issues"
    }

    fn description(&self) -> &str {
        "List issues from the GitHub repository. Can filter by state (open/closed) and labels."
    }

    fn parameters(&self) -> Vec<ParameterDef> {
        vec![
            ParameterDef::optional_string("state", "Filter by state: open, closed, or all"),
            ParameterDef::optional_string("labels", "Comma-separated list of labels to filter by"),
        ]
    }

    async fn execute(&self, input: Value) -> ToolResult<ToolOutput> {
        let state = input.get("state").and_then(|v| v.as_str());
        let labels = input.get("labels").and_then(|v| v.as_str());

        match self.client.list_issues(state, labels).await {
            Ok(issues) => {
                let mut output = String::new();
                output.push_str(&format!("Found {} issues:\n\n", issues.len()));

                for issue in issues {
                    let labels: Vec<_> = issue.labels.iter().map(|l| l.name.as_str()).collect();
                    output.push_str(&format!(
                        "#{} - {} [{}]\n  Labels: {}\n  URL: {}\n\n",
                        issue.number,
                        issue.title,
                        issue.state,
                        if labels.is_empty() {
                            "none".to_string()
                        } else {
                            labels.join(", ")
                        },
                        issue.html_url
                    ));
                }

                Ok(ToolOutput::success(output))
            }
            Err(e) => Err(ToolError::ExecutionFailed(e.to_string())),
        }
    }
}

/// Tool to get a specific GitHub issue
pub struct GetIssueTool {
    client: GitHubClient,
}

impl GetIssueTool {
    pub fn new(client: GitHubClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for GetIssueTool {
    fn name(&self) -> &str {
        "github_get_issue"
    }

    fn description(&self) -> &str {
        "Get details of a specific GitHub issue by number."
    }

    fn parameters(&self) -> Vec<ParameterDef> {
        vec![ParameterDef {
            name: "number".to_string(),
            param_type: "integer".to_string(),
            description: "The issue number".to_string(),
            required: true,
            default: None,
            enum_values: None,
        }]
    }

    async fn execute(&self, input: Value) -> ToolResult<ToolOutput> {
        let number = input
            .get("number")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| ToolError::InvalidInput("number is required".to_string()))?;

        match self.client.get_issue(number).await {
            Ok(issue) => {
                let labels: Vec<_> = issue.labels.iter().map(|l| l.name.as_str()).collect();
                let assignees: Vec<_> = issue.assignees.iter().map(|u| u.login.as_str()).collect();

                let output = format!(
                    "Issue #{}: {}\n\nState: {}\nLabels: {}\nAssignees: {}\nCreated: {}\nUpdated: {}\nURL: {}\n\nBody:\n{}",
                    issue.number,
                    issue.title,
                    issue.state,
                    if labels.is_empty() { "none".to_string() } else { labels.join(", ") },
                    if assignees.is_empty() { "none".to_string() } else { assignees.join(", ") },
                    issue.created_at,
                    issue.updated_at,
                    issue.html_url,
                    issue.body.unwrap_or_else(|| "No description".to_string())
                );

                Ok(ToolOutput::success(output))
            }
            Err(e) => Err(ToolError::ExecutionFailed(e.to_string())),
        }
    }
}

/// Tool to create a GitHub issue
pub struct CreateIssueTool {
    client: GitHubClient,
}

impl CreateIssueTool {
    pub fn new(client: GitHubClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for CreateIssueTool {
    fn name(&self) -> &str {
        "github_create_issue"
    }

    fn description(&self) -> &str {
        "Create a new GitHub issue in the repository."
    }

    fn parameters(&self) -> Vec<ParameterDef> {
        vec![
            ParameterDef::required_string("title", "The title of the issue"),
            ParameterDef::optional_string("body", "The body/description of the issue"),
            ParameterDef {
                name: "labels".to_string(),
                param_type: "array".to_string(),
                description: "Array of label names to add".to_string(),
                required: false,
                default: None,
                enum_values: None,
            },
        ]
    }

    async fn execute(&self, input: Value) -> ToolResult<ToolOutput> {
        let title = input
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("title is required".to_string()))?;

        let body = input.get("body").and_then(|v| v.as_str()).map(String::from);
        let labels = input.get("labels").and_then(|v| {
            v.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
        });

        let request = CreateIssueRequest {
            title: title.to_string(),
            body,
            labels,
            assignees: None,
        };

        match self.client.create_issue(request).await {
            Ok(issue) => {
                let output = format!(
                    "Created issue #{}: {}\nURL: {}",
                    issue.number, issue.title, issue.html_url
                );
                Ok(ToolOutput::success(output))
            }
            Err(e) => Err(ToolError::ExecutionFailed(e.to_string())),
        }
    }
}

/// Tool to add a comment to an issue
pub struct AddCommentTool {
    client: GitHubClient,
}

impl AddCommentTool {
    pub fn new(client: GitHubClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for AddCommentTool {
    fn name(&self) -> &str {
        "github_add_comment"
    }

    fn description(&self) -> &str {
        "Add a comment to a GitHub issue or pull request."
    }

    fn parameters(&self) -> Vec<ParameterDef> {
        vec![
            ParameterDef {
                name: "number".to_string(),
                param_type: "integer".to_string(),
                description: "The issue or PR number".to_string(),
                required: true,
                default: None,
                enum_values: None,
            },
            ParameterDef::required_string("body", "The comment text"),
        ]
    }

    async fn execute(&self, input: Value) -> ToolResult<ToolOutput> {
        let number = input
            .get("number")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| ToolError::InvalidInput("number is required".to_string()))?;

        let body = input
            .get("body")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("body is required".to_string()))?;

        match self.client.create_comment(number, body).await {
            Ok(comment) => {
                let output = format!(
                    "Added comment to #{}\nComment ID: {}\nBy: {}",
                    number, comment.id, comment.user.login
                );
                Ok(ToolOutput::success(output))
            }
            Err(e) => Err(ToolError::ExecutionFailed(e.to_string())),
        }
    }
}

/// Tool to list pull requests
pub struct ListPullRequestsTool {
    client: GitHubClient,
}

impl ListPullRequestsTool {
    pub fn new(client: GitHubClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for ListPullRequestsTool {
    fn name(&self) -> &str {
        "github_list_prs"
    }

    fn description(&self) -> &str {
        "List pull requests from the GitHub repository."
    }

    fn parameters(&self) -> Vec<ParameterDef> {
        vec![ParameterDef::optional_string(
            "state",
            "Filter by state: open, closed, or all",
        )]
    }

    async fn execute(&self, input: Value) -> ToolResult<ToolOutput> {
        let state = input.get("state").and_then(|v| v.as_str());

        match self.client.list_pull_requests(state).await {
            Ok(prs) => {
                let mut output = String::new();
                output.push_str(&format!("Found {} pull requests:\n\n", prs.len()));

                for pr in prs {
                    output.push_str(&format!(
                        "#{} - {} [{}]\n  {} -> {}\n  Draft: {}\n  URL: {}\n\n",
                        pr.number,
                        pr.title,
                        pr.state,
                        pr.head.ref_name,
                        pr.base.ref_name,
                        pr.draft,
                        pr.html_url
                    ));
                }

                Ok(ToolOutput::success(output))
            }
            Err(e) => Err(ToolError::ExecutionFailed(e.to_string())),
        }
    }
}

/// Tool to create a pull request
pub struct CreatePullRequestTool {
    client: GitHubClient,
}

impl CreatePullRequestTool {
    pub fn new(client: GitHubClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for CreatePullRequestTool {
    fn name(&self) -> &str {
        "github_create_pr"
    }

    fn description(&self) -> &str {
        "Create a new pull request."
    }

    fn parameters(&self) -> Vec<ParameterDef> {
        vec![
            ParameterDef::required_string("title", "The title of the pull request"),
            ParameterDef::required_string("head", "The branch containing the changes"),
            ParameterDef::required_string("base", "The branch to merge into (e.g., main)"),
            ParameterDef::optional_string("body", "The body/description of the pull request"),
            ParameterDef {
                name: "draft".to_string(),
                param_type: "boolean".to_string(),
                description: "Whether to create as a draft PR".to_string(),
                required: false,
                default: None,
                enum_values: None,
            },
        ]
    }

    async fn execute(&self, input: Value) -> ToolResult<ToolOutput> {
        let title = input
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("title is required".to_string()))?;

        let head = input
            .get("head")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("head is required".to_string()))?;

        let base = input
            .get("base")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("base is required".to_string()))?;

        let body = input.get("body").and_then(|v| v.as_str()).map(String::from);
        let draft = input.get("draft").and_then(|v| v.as_bool());

        let request = CreatePullRequestRequest {
            title: title.to_string(),
            head: head.to_string(),
            base: base.to_string(),
            body,
            draft,
        };

        match self.client.create_pull_request(request).await {
            Ok(pr) => {
                let output = format!(
                    "Created PR #{}: {}\n{} -> {}\nURL: {}",
                    pr.number, pr.title, pr.head.ref_name, pr.base.ref_name, pr.html_url
                );
                Ok(ToolOutput::success(output))
            }
            Err(e) => Err(ToolError::ExecutionFailed(e.to_string())),
        }
    }
}

/// Tool to add labels to an issue
pub struct AddLabelsTool {
    client: GitHubClient,
}

impl AddLabelsTool {
    pub fn new(client: GitHubClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for AddLabelsTool {
    fn name(&self) -> &str {
        "github_add_labels"
    }

    fn description(&self) -> &str {
        "Add labels to a GitHub issue."
    }

    fn parameters(&self) -> Vec<ParameterDef> {
        vec![
            ParameterDef {
                name: "number".to_string(),
                param_type: "integer".to_string(),
                description: "The issue number".to_string(),
                required: true,
                default: None,
                enum_values: None,
            },
            ParameterDef {
                name: "labels".to_string(),
                param_type: "array".to_string(),
                description: "Array of label names to add".to_string(),
                required: true,
                default: None,
                enum_values: None,
            },
        ]
    }

    async fn execute(&self, input: Value) -> ToolResult<ToolOutput> {
        let number = input
            .get("number")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| ToolError::InvalidInput("number is required".to_string()))?;

        let labels = input
            .get("labels")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .ok_or_else(|| ToolError::InvalidInput("labels is required".to_string()))?;

        match self.client.add_labels(number, labels).await {
            Ok(result) => {
                let label_names: Vec<_> = result.iter().map(|l| l.name.as_str()).collect();
                let output = format!("Added labels to #{}: {}", number, label_names.join(", "));
                Ok(ToolOutput::success(output))
            }
            Err(e) => Err(ToolError::ExecutionFailed(e.to_string())),
        }
    }
}

/// Create a tool registry with GitHub tools
pub fn create_github_tool_registry(client: GitHubClient) -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    registry.register(ListIssuesTool::new(client.clone()));
    registry.register(GetIssueTool::new(client.clone()));
    registry.register(CreateIssueTool::new(client.clone()));
    registry.register(AddCommentTool::new(client.clone()));
    registry.register(ListPullRequestsTool::new(client.clone()));
    registry.register(CreatePullRequestTool::new(client.clone()));
    registry.register(AddLabelsTool::new(client));

    registry
}
