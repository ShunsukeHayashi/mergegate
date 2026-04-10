//! GitHub API Client
//!
//! Provides integration with GitHub for issues, pull requests, and repository operations.

use anyhow::{anyhow, Result};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};

/// GitHub API client
#[derive(Clone)]
pub struct GitHubClient {
    client: reqwest::Client,
    base_url: String,
    owner: String,
    repo: String,
}

/// GitHub Issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub labels: Vec<Label>,
    pub assignees: Vec<User>,
    pub html_url: String,
    pub created_at: String,
    pub updated_at: String,
}

/// GitHub Pull Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub head: PullRequestRef,
    pub base: PullRequestRef,
    pub html_url: String,
    pub draft: bool,
    pub mergeable: Option<bool>,
    pub created_at: String,
    pub updated_at: String,
}

/// Pull request branch reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestRef {
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub sha: String,
}

/// GitHub Label
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub name: String,
    pub color: String,
    pub description: Option<String>,
}

/// GitHub User
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub login: String,
    pub id: u64,
}

/// Create issue request
#[derive(Debug, Serialize)]
pub struct CreateIssueRequest {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignees: Option<Vec<String>>,
}

/// Create pull request request
#[derive(Debug, Serialize)]
pub struct CreatePullRequestRequest {
    pub title: String,
    pub head: String,
    pub base: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft: Option<bool>,
}

/// Comment on issue or PR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: u64,
    pub body: String,
    pub user: User,
    pub created_at: String,
}

/// Create comment request
#[derive(Debug, Serialize)]
pub struct CreateCommentRequest {
    pub body: String,
}

impl GitHubClient {
    /// Create a new GitHub client
    pub fn new(token: &str, owner: &str, repo: &str) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", token))?,
        );
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(USER_AGENT, HeaderValue::from_static("miyabi-cli"));
        headers.insert(
            "X-GitHub-Api-Version",
            HeaderValue::from_static("2022-11-28"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self {
            client,
            base_url: "https://api.github.com".to_string(),
            owner: owner.to_string(),
            repo: repo.to_string(),
        })
    }

    /// Create from environment variables
    pub fn from_env() -> Result<Self> {
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| anyhow!("GITHUB_TOKEN not set"))?;
        let repo = std::env::var("GITHUB_REPOSITORY")
            .or_else(|_| std::env::var("REPOSITORY"))
            .map_err(|_| anyhow!("GITHUB_REPOSITORY or REPOSITORY not set"))?;

        let parts: Vec<&str> = repo.split('/').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid repository format. Expected: owner/repo"));
        }

        Self::new(&token, parts[0], parts[1])
    }

    // ========== Issues ==========

    /// List issues
    pub async fn list_issues(
        &self,
        state: Option<&str>,
        labels: Option<&str>,
    ) -> Result<Vec<Issue>> {
        let mut url = format!(
            "{}/repos/{}/{}/issues",
            self.base_url, self.owner, self.repo
        );

        let mut params = vec![];
        if let Some(s) = state {
            params.push(format!("state={}", s));
        }
        if let Some(l) = labels {
            params.push(format!("labels={}", l));
        }
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(anyhow!("GitHub API error {}: {}", status, text));
        }

        let issues: Vec<Issue> = response.json().await?;
        Ok(issues)
    }

    /// Get a single issue
    pub async fn get_issue(&self, number: u64) -> Result<Issue> {
        let url = format!(
            "{}/repos/{}/{}/issues/{}",
            self.base_url, self.owner, self.repo, number
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(anyhow!("GitHub API error {}: {}", status, text));
        }

        let issue: Issue = response.json().await?;
        Ok(issue)
    }

    /// Create an issue
    pub async fn create_issue(&self, request: CreateIssueRequest) -> Result<Issue> {
        let url = format!(
            "{}/repos/{}/{}/issues",
            self.base_url, self.owner, self.repo
        );

        let response = self.client.post(&url).json(&request).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(anyhow!("GitHub API error {}: {}", status, text));
        }

        let issue: Issue = response.json().await?;
        Ok(issue)
    }

    /// Add labels to an issue
    pub async fn add_labels(&self, issue_number: u64, labels: Vec<String>) -> Result<Vec<Label>> {
        let url = format!(
            "{}/repos/{}/{}/issues/{}/labels",
            self.base_url, self.owner, self.repo, issue_number
        );

        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({ "labels": labels }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(anyhow!("GitHub API error {}: {}", status, text));
        }

        let result: Vec<Label> = response.json().await?;
        Ok(result)
    }

    /// Close an issue
    pub async fn close_issue(&self, number: u64) -> Result<Issue> {
        let url = format!(
            "{}/repos/{}/{}/issues/{}",
            self.base_url, self.owner, self.repo, number
        );

        let response = self
            .client
            .patch(&url)
            .json(&serde_json::json!({ "state": "closed" }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(anyhow!("GitHub API error {}: {}", status, text));
        }

        let issue: Issue = response.json().await?;
        Ok(issue)
    }

    // ========== Pull Requests ==========

    /// List pull requests
    pub async fn list_pull_requests(&self, state: Option<&str>) -> Result<Vec<PullRequest>> {
        let mut url = format!("{}/repos/{}/{}/pulls", self.base_url, self.owner, self.repo);

        if let Some(s) = state {
            url = format!("{}?state={}", url, s);
        }

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(anyhow!("GitHub API error {}: {}", status, text));
        }

        let prs: Vec<PullRequest> = response.json().await?;
        Ok(prs)
    }

    /// Get a single pull request
    pub async fn get_pull_request(&self, number: u64) -> Result<PullRequest> {
        let url = format!(
            "{}/repos/{}/{}/pulls/{}",
            self.base_url, self.owner, self.repo, number
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(anyhow!("GitHub API error {}: {}", status, text));
        }

        let pr: PullRequest = response.json().await?;
        Ok(pr)
    }

    /// Create a pull request
    pub async fn create_pull_request(
        &self,
        request: CreatePullRequestRequest,
    ) -> Result<PullRequest> {
        let url = format!("{}/repos/{}/{}/pulls", self.base_url, self.owner, self.repo);

        let response = self.client.post(&url).json(&request).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(anyhow!("GitHub API error {}: {}", status, text));
        }

        let pr: PullRequest = response.json().await?;
        Ok(pr)
    }

    /// Merge a pull request
    pub async fn merge_pull_request(
        &self,
        number: u64,
        commit_message: Option<&str>,
    ) -> Result<()> {
        let url = format!(
            "{}/repos/{}/{}/pulls/{}/merge",
            self.base_url, self.owner, self.repo, number
        );

        let mut body = serde_json::json!({});
        if let Some(msg) = commit_message {
            body["commit_message"] = serde_json::Value::String(msg.to_string());
        }

        let response = self.client.put(&url).json(&body).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(anyhow!("GitHub API error {}: {}", status, text));
        }

        Ok(())
    }

    // ========== Comments ==========

    /// List comments on an issue
    pub async fn list_comments(&self, issue_number: u64) -> Result<Vec<Comment>> {
        let url = format!(
            "{}/repos/{}/{}/issues/{}/comments",
            self.base_url, self.owner, self.repo, issue_number
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(anyhow!("GitHub API error {}: {}", status, text));
        }

        let comments: Vec<Comment> = response.json().await?;
        Ok(comments)
    }

    /// Create a comment on an issue
    pub async fn create_comment(&self, issue_number: u64, body: &str) -> Result<Comment> {
        let url = format!(
            "{}/repos/{}/{}/issues/{}/comments",
            self.base_url, self.owner, self.repo, issue_number
        );

        let request = CreateCommentRequest {
            body: body.to_string(),
        };

        let response = self.client.post(&url).json(&request).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(anyhow!("GitHub API error {}: {}", status, text));
        }

        let comment: Comment = response.json().await?;
        Ok(comment)
    }

    // ========== Repository Info ==========

    /// Get repository information
    pub async fn get_repo(&self) -> Result<serde_json::Value> {
        let url = format!("{}/repos/{}/{}", self.base_url, self.owner, self.repo);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(anyhow!("GitHub API error {}: {}", status, text));
        }

        let repo: serde_json::Value = response.json().await?;
        Ok(repo)
    }

    /// Get owner and repo
    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn repo(&self) -> &str {
        &self.repo
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_issue_request_serialization() {
        let request = CreateIssueRequest {
            title: "Test issue".to_string(),
            body: Some("Test body".to_string()),
            labels: Some(vec!["bug".to_string()]),
            assignees: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("Test issue"));
        assert!(json.contains("bug"));
        assert!(!json.contains("assignees"));
    }

    #[test]
    fn test_create_pr_request_serialization() {
        let request = CreatePullRequestRequest {
            title: "Test PR".to_string(),
            head: "feature-branch".to_string(),
            base: "main".to_string(),
            body: Some("PR description".to_string()),
            draft: Some(true),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("feature-branch"));
        assert!(json.contains("main"));
        assert!(json.contains("true"));
    }
}
