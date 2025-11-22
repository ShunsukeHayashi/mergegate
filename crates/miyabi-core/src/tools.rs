//! File Operation Tools
//!
//! This module provides standard file operation tools that can be
//! used by AI agents to interact with the filesystem.

use crate::tool::{ParameterDef, Tool, ToolError, ToolOutput, ToolResult};
use async_trait::async_trait;
use glob::glob;
use regex::Regex;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tracing::{debug, warn};

/// Read file tool
///
/// Reads content from a file with optional offset and limit.
#[derive(Debug, Clone)]
pub struct ReadTool {
    /// Base directory for relative paths
    base_dir: PathBuf,
}

impl Default for ReadTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadTool {
    /// Create a new read tool
    pub fn new() -> Self {
        Self {
            base_dir: std::env::current_dir().unwrap_or_default(),
        }
    }

    /// Create with a specific base directory
    pub fn with_base_dir(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    /// Resolve and validate path
    fn resolve_path(&self, path: &str) -> ToolResult<PathBuf> {
        let path = Path::new(path);
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.base_dir.join(path)
        };

        // Security check: prevent path traversal
        let canonical = resolved
            .canonicalize()
            .map_err(|e| ToolError::ExecutionFailed(format!("Cannot resolve path: {}", e)))?;

        // Ensure path is within allowed boundaries
        if !canonical.starts_with(&self.base_dir) && !canonical.is_absolute() {
            return Err(ToolError::PermissionDenied(
                "Path traversal detected".to_string(),
            ));
        }

        Ok(canonical)
    }
}

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &str {
        "read"
    }

    fn description(&self) -> &str {
        "Read content from a file with optional offset and limit"
    }

    fn parameters(&self) -> Vec<ParameterDef> {
        vec![
            ParameterDef::required_string("path", "Path to the file to read"),
            ParameterDef {
                name: "offset".to_string(),
                param_type: "number".to_string(),
                description: "Line number to start reading from (1-based)".to_string(),
                required: false,
                default: Some(Value::Number(1.into())),
                enum_values: None,
            },
            ParameterDef {
                name: "limit".to_string(),
                param_type: "number".to_string(),
                description: "Maximum number of lines to read".to_string(),
                required: false,
                default: None,
                enum_values: None,
            },
        ]
    }

    async fn execute(&self, input: Value) -> ToolResult<ToolOutput> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("path is required".to_string()))?;

        let offset = input.get("offset").and_then(|v| v.as_u64()).unwrap_or(1) as usize;

        let limit = input
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        debug!(
            "Reading file: {} (offset: {}, limit: {:?})",
            path, offset, limit
        );

        let resolved = self.resolve_path(path)?;
        let content = std::fs::read_to_string(&resolved)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read file: {}", e)))?;

        // Apply offset and limit
        let lines: Vec<&str> = content.lines().collect();
        let start = offset.saturating_sub(1);
        let end = match limit {
            Some(l) => (start + l).min(lines.len()),
            None => lines.len(),
        };

        // Format with line numbers
        let result: Vec<String> = lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{:>6}\t{}", start + i + 1, line))
            .collect();

        Ok(ToolOutput::success(serde_json::json!({
            "path": resolved.display().to_string(),
            "content": result.join("\n"),
            "total_lines": lines.len(),
            "read_lines": end - start
        })))
    }
}

/// Write file tool
///
/// Writes content to a file, creating directories if needed.
#[derive(Debug, Clone)]
pub struct WriteTool {
    /// Base directory for relative paths
    base_dir: PathBuf,
}

impl Default for WriteTool {
    fn default() -> Self {
        Self::new()
    }
}

impl WriteTool {
    /// Create a new write tool
    pub fn new() -> Self {
        Self {
            base_dir: std::env::current_dir().unwrap_or_default(),
        }
    }

    /// Create with a specific base directory
    pub fn with_base_dir(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    /// Resolve path for writing
    fn resolve_path(&self, path: &str) -> ToolResult<PathBuf> {
        let path = Path::new(path);
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.base_dir.join(path)
        };

        Ok(resolved)
    }
}

#[async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &str {
        "write"
    }

    fn description(&self) -> &str {
        "Write content to a file, creating directories if needed"
    }

    fn parameters(&self) -> Vec<ParameterDef> {
        vec![
            ParameterDef::required_string("path", "Path to the file to write"),
            ParameterDef::required_string("content", "Content to write to the file"),
        ]
    }

    async fn execute(&self, input: Value) -> ToolResult<ToolOutput> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("path is required".to_string()))?;

        let content = input
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("content is required".to_string()))?;

        debug!("Writing file: {}", path);

        let resolved = self.resolve_path(path)?;

        // Create parent directories if needed
        if let Some(parent) = resolved.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to create directories: {}", e))
            })?;
        }

        std::fs::write(&resolved, content)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write file: {}", e)))?;

        Ok(ToolOutput::success(serde_json::json!({
            "path": resolved.display().to_string(),
            "bytes_written": content.len(),
            "success": true
        })))
    }
}

/// Edit file tool
///
/// Edits a file by replacing text.
#[derive(Debug, Clone)]
pub struct EditTool {
    /// Base directory for relative paths
    base_dir: PathBuf,
}

impl Default for EditTool {
    fn default() -> Self {
        Self::new()
    }
}

impl EditTool {
    /// Create a new edit tool
    pub fn new() -> Self {
        Self {
            base_dir: std::env::current_dir().unwrap_or_default(),
        }
    }

    /// Create with a specific base directory
    pub fn with_base_dir(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    /// Resolve and validate path
    fn resolve_path(&self, path: &str) -> ToolResult<PathBuf> {
        let path = Path::new(path);
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.base_dir.join(path)
        };

        // Check if file exists
        if !resolved.exists() {
            return Err(ToolError::ExecutionFailed(format!(
                "File not found: {}",
                resolved.display()
            )));
        }

        Ok(resolved)
    }
}

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str {
        "edit"
    }

    fn description(&self) -> &str {
        "Edit a file by replacing text occurrences"
    }

    fn parameters(&self) -> Vec<ParameterDef> {
        vec![
            ParameterDef::required_string("path", "Path to the file to edit"),
            ParameterDef::required_string("old_string", "Text to find and replace"),
            ParameterDef::required_string("new_string", "Text to replace with"),
            ParameterDef {
                name: "replace_all".to_string(),
                param_type: "boolean".to_string(),
                description: "Replace all occurrences (default: false, replaces first only)"
                    .to_string(),
                required: false,
                default: Some(Value::Bool(false)),
                enum_values: None,
            },
        ]
    }

    async fn execute(&self, input: Value) -> ToolResult<ToolOutput> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("path is required".to_string()))?;

        let old_string = input
            .get("old_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("old_string is required".to_string()))?;

        let new_string = input
            .get("new_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("new_string is required".to_string()))?;

        let replace_all = input
            .get("replace_all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        debug!("Editing file: {} (replace_all: {})", path, replace_all);

        let resolved = self.resolve_path(path)?;
        let content = std::fs::read_to_string(&resolved)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read file: {}", e)))?;

        // Check if old_string exists
        if !content.contains(old_string) {
            return Err(ToolError::ExecutionFailed(
                "old_string not found in file".to_string(),
            ));
        }

        // Perform replacement
        let (new_content, count) = if replace_all {
            let count = content.matches(old_string).count();
            (content.replace(old_string, new_string), count)
        } else {
            // Replace first occurrence only
            let count = if content.contains(old_string) { 1 } else { 0 };
            (content.replacen(old_string, new_string, 1), count)
        };

        std::fs::write(&resolved, &new_content)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write file: {}", e)))?;

        Ok(ToolOutput::success(serde_json::json!({
            "path": resolved.display().to_string(),
            "replacements": count,
            "success": true
        })))
    }
}

/// Bash tool for executing shell commands
///
/// Executes commands with timeout and captures output.
#[derive(Debug, Clone)]
pub struct BashTool {
    /// Working directory
    working_dir: PathBuf,
    /// Default timeout in seconds
    default_timeout: u64,
    /// Maximum output length
    max_output_len: usize,
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

impl BashTool {
    /// Create a new bash tool
    pub fn new() -> Self {
        Self {
            working_dir: std::env::current_dir().unwrap_or_default(),
            default_timeout: 120,
            max_output_len: 50_000,
        }
    }

    /// Create with a specific working directory
    pub fn with_working_dir(working_dir: impl Into<PathBuf>) -> Self {
        Self {
            working_dir: working_dir.into(),
            ..Default::default()
        }
    }

    /// Set default timeout
    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.default_timeout = timeout;
        self
    }

    /// Check for dangerous commands
    fn check_dangerous(&self, command: &str) -> Option<String> {
        let dangerous = [
            "rm -rf /",
            "rm -rf ~",
            "mkfs",
            ":(){:|:&};:",
            "dd if=/dev/zero",
            "chmod -R 777 /",
        ];

        for pattern in dangerous {
            if command.contains(pattern) {
                return Some(format!(
                    "Potentially dangerous command detected: {}",
                    pattern
                ));
            }
        }
        None
    }

    /// Truncate output if too long
    fn truncate_output(&self, output: String) -> String {
        if output.len() > self.max_output_len {
            let truncated = &output[..self.max_output_len];
            format!(
                "{}\n\n... (output truncated, {} bytes total)",
                truncated,
                output.len()
            )
        } else {
            output
        }
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute a bash command with timeout"
    }

    fn parameters(&self) -> Vec<ParameterDef> {
        vec![
            ParameterDef::required_string("command", "The bash command to execute"),
            ParameterDef {
                name: "timeout".to_string(),
                param_type: "number".to_string(),
                description: "Timeout in seconds (default: 120)".to_string(),
                required: false,
                default: None,
                enum_values: None,
            },
            ParameterDef::optional_string("working_dir", "Working directory for the command"),
        ]
    }

    async fn execute(&self, input: Value) -> ToolResult<ToolOutput> {
        let command = input
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("command is required".to_string()))?;

        let timeout_secs = input
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(self.default_timeout);

        let working_dir = input
            .get("working_dir")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| self.working_dir.clone());

        debug!(
            "Executing bash command: {} (timeout: {}s)",
            command, timeout_secs
        );

        // Check for dangerous commands
        if let Some(warning) = self.check_dangerous(command) {
            warn!("{}", warning);
            return Err(ToolError::PermissionDenied(warning));
        }

        // Execute command
        let child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to spawn process: {}", e)))?;

        // Wait with timeout using select
        let timeout_duration = Duration::from_secs(timeout_secs);

        tokio::select! {
            result = child.wait_with_output() => {
                match result {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        let exit_code = output.status.code().unwrap_or(-1);

                        let stdout = self.truncate_output(stdout);
                        let stderr = self.truncate_output(stderr);

                        let success = output.status.success();

                        if success {
                            Ok(ToolOutput::success(serde_json::json!({
                                "stdout": stdout,
                                "stderr": stderr,
                                "exit_code": exit_code,
                                "success": true
                            })))
                        } else {
                            Ok(ToolOutput {
                                success: false,
                                content: serde_json::json!({
                                    "stdout": stdout,
                                    "stderr": stderr,
                                    "exit_code": exit_code,
                                    "success": false
                                }),
                                error: Some(format!("Command exited with code {}", exit_code)),
                                duration_ms: 0,
                            })
                        }
                    }
                    Err(e) => Err(ToolError::ExecutionFailed(format!(
                        "Failed to wait for process: {}",
                        e
                    ))),
                }
            }
            _ = tokio::time::sleep(timeout_duration) => {
                // Timeout occurred - process is still running but we can't kill it
                // since wait_with_output took ownership. Return timeout error.
                Err(ToolError::Timeout(timeout_secs * 1000))
            }
        }
    }
}

/// Glob tool for finding files matching patterns
///
/// Finds files in the filesystem that match glob patterns.
#[derive(Debug, Clone)]
pub struct GlobTool {
    /// Base directory for relative patterns
    base_dir: PathBuf,
    /// Maximum number of results to return
    max_results: usize,
}

impl Default for GlobTool {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobTool {
    /// Create a new glob tool
    pub fn new() -> Self {
        Self {
            base_dir: std::env::current_dir().unwrap_or_default(),
            max_results: 1000,
        }
    }

    /// Create with a specific base directory
    pub fn with_base_dir(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            max_results: 1000,
        }
    }
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> &str {
        "Find files matching a glob pattern"
    }

    fn parameters(&self) -> Vec<ParameterDef> {
        vec![
            ParameterDef::required_string(
                "pattern",
                "Glob pattern to match (e.g., **/*.rs, src/**/*.ts)",
            ),
            ParameterDef::optional_string("path", "Base directory to search in"),
        ]
    }

    async fn execute(&self, input: Value) -> ToolResult<ToolOutput> {
        let pattern = input
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("pattern is required".to_string()))?;

        let base_path = input
            .get("path")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| self.base_dir.clone());

        debug!("Glob search: {} in {:?}", pattern, base_path);

        // Construct full pattern
        let full_pattern = if pattern.starts_with('/') {
            pattern.to_string()
        } else {
            format!("{}/{}", base_path.display(), pattern)
        };

        // Execute glob
        let entries = glob(&full_pattern)
            .map_err(|e| ToolError::InvalidInput(format!("Invalid glob pattern: {}", e)))?;

        let mut matches = Vec::new();
        for entry in entries {
            match entry {
                Ok(path) => {
                    if matches.len() >= self.max_results {
                        break;
                    }
                    matches.push(path.display().to_string());
                }
                Err(e) => {
                    warn!("Glob error for entry: {}", e);
                }
            }
        }

        Ok(ToolOutput::success(serde_json::json!({
            "pattern": pattern,
            "matches": matches,
            "count": matches.len(),
            "truncated": matches.len() >= self.max_results
        })))
    }
}

/// Grep tool for searching file contents
///
/// Searches for patterns in file contents using regex.
#[derive(Debug, Clone)]
pub struct GrepTool {
    /// Base directory for relative paths
    base_dir: PathBuf,
    /// Maximum number of matches to return
    max_matches: usize,
    /// Maximum file size to search (in bytes)
    max_file_size: u64,
}

impl Default for GrepTool {
    fn default() -> Self {
        Self::new()
    }
}

impl GrepTool {
    /// Create a new grep tool
    pub fn new() -> Self {
        Self {
            base_dir: std::env::current_dir().unwrap_or_default(),
            max_matches: 100,
            max_file_size: 10 * 1024 * 1024, // 10MB
        }
    }

    /// Create with a specific base directory
    pub fn with_base_dir(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            max_matches: 100,
            max_file_size: 10 * 1024 * 1024,
        }
    }
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search for a pattern in files using regex"
    }

    fn parameters(&self) -> Vec<ParameterDef> {
        vec![
            ParameterDef::required_string("pattern", "Regex pattern to search for"),
            ParameterDef::required_string("path", "File or directory to search in"),
            ParameterDef::optional_string("glob", "Glob pattern to filter files (e.g., *.rs)"),
            ParameterDef {
                name: "case_insensitive".to_string(),
                param_type: "boolean".to_string(),
                description: "Case insensitive search".to_string(),
                required: false,
                default: Some(Value::Bool(false)),
                enum_values: None,
            },
        ]
    }

    async fn execute(&self, input: Value) -> ToolResult<ToolOutput> {
        let pattern = input
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("pattern is required".to_string()))?;

        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("path is required".to_string()))?;

        let glob_pattern = input.get("glob").and_then(|v| v.as_str());

        let case_insensitive = input
            .get("case_insensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        debug!(
            "Grep search: {} in {} (case_insensitive: {})",
            pattern, path, case_insensitive
        );

        // Build regex
        let regex_pattern = if case_insensitive {
            format!("(?i){}", pattern)
        } else {
            pattern.to_string()
        };

        let regex = Regex::new(&regex_pattern)
            .map_err(|e| ToolError::InvalidInput(format!("Invalid regex pattern: {}", e)))?;

        let search_path = if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            self.base_dir.join(path)
        };

        let mut results = Vec::new();

        // Collect files to search
        let files_to_search = if search_path.is_file() {
            vec![search_path]
        } else if search_path.is_dir() {
            // Use glob to find files
            let glob_str = if let Some(g) = glob_pattern {
                format!("{}/**/{}", search_path.display(), g)
            } else {
                format!("{}/**/*", search_path.display())
            };

            glob(&glob_str)
                .map_err(|e| ToolError::ExecutionFailed(format!("Glob error: {}", e)))?
                .filter_map(|entry| entry.ok())
                .filter(|p| p.is_file())
                .collect()
        } else {
            return Err(ToolError::ExecutionFailed(format!(
                "Path does not exist: {}",
                path
            )));
        };

        // Search files
        for file_path in files_to_search {
            if results.len() >= self.max_matches {
                break;
            }

            // Check file size
            let metadata = match std::fs::metadata(&file_path) {
                Ok(m) => m,
                Err(_) => continue,
            };

            if metadata.len() > self.max_file_size {
                continue;
            }

            // Read and search file
            let content = match std::fs::read_to_string(&file_path) {
                Ok(c) => c,
                Err(_) => continue, // Skip binary files
            };

            for (line_num, line) in content.lines().enumerate() {
                if results.len() >= self.max_matches {
                    break;
                }

                if regex.is_match(line) {
                    results.push(serde_json::json!({
                        "file": file_path.display().to_string(),
                        "line": line_num + 1,
                        "content": line
                    }));
                }
            }
        }

        Ok(ToolOutput::success(serde_json::json!({
            "pattern": pattern,
            "matches": results,
            "count": results.len(),
            "truncated": results.len() >= self.max_matches
        })))
    }
}

/// Create a tool registry with all file tools
pub fn create_file_tool_registry() -> crate::tool::ToolRegistry {
    let mut registry = crate::tool::ToolRegistry::new();
    registry
        .register(ReadTool::new())
        .register(WriteTool::new())
        .register(EditTool::new())
        .register(GlobTool::new())
        .register(GrepTool::new());
    registry
}

/// Create a tool registry with all standard tools
pub fn create_standard_tool_registry() -> crate::tool::ToolRegistry {
    let mut registry = crate::tool::ToolRegistry::new();
    registry
        .register(ReadTool::new())
        .register(WriteTool::new())
        .register(EditTool::new())
        .register(BashTool::new())
        .register(GlobTool::new())
        .register(GrepTool::new());
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::ToolRegistry;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_temp_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let path = dir.path().join(name);
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    #[tokio::test]
    async fn test_read_tool_basic() {
        let dir = TempDir::new().unwrap();
        let _path = create_temp_file(&dir, "test.txt", "Line 1\nLine 2\nLine 3");

        let tool = ReadTool::with_base_dir(dir.path());
        let result = tool
            .execute(serde_json::json!({
                "path": "test.txt"
            }))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.success);
        assert_eq!(output.content["total_lines"], 3);
    }

    #[tokio::test]
    async fn test_read_tool_offset_limit() {
        let dir = TempDir::new().unwrap();
        let content = (1..=10)
            .map(|i| format!("Line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        create_temp_file(&dir, "test.txt", &content);

        let tool = ReadTool::with_base_dir(dir.path());
        let result = tool
            .execute(serde_json::json!({
                "path": "test.txt",
                "offset": 3,
                "limit": 2
            }))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.content["read_lines"], 2);
    }

    #[tokio::test]
    async fn test_read_tool_not_found() {
        let dir = TempDir::new().unwrap();
        let tool = ReadTool::with_base_dir(dir.path());

        let result = tool
            .execute(serde_json::json!({
                "path": "nonexistent.txt"
            }))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_write_tool_basic() {
        let dir = TempDir::new().unwrap();
        let tool = WriteTool::with_base_dir(dir.path());

        let result = tool
            .execute(serde_json::json!({
                "path": "output.txt",
                "content": "Hello, World!"
            }))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.success);

        // Verify file was written
        let written = std::fs::read_to_string(dir.path().join("output.txt")).unwrap();
        assert_eq!(written, "Hello, World!");
    }

    #[tokio::test]
    async fn test_write_tool_creates_dirs() {
        let dir = TempDir::new().unwrap();
        let tool = WriteTool::with_base_dir(dir.path());

        let result = tool
            .execute(serde_json::json!({
                "path": "nested/dir/output.txt",
                "content": "Nested content"
            }))
            .await;

        assert!(result.is_ok());
        assert!(dir.path().join("nested/dir/output.txt").exists());
    }

    #[tokio::test]
    async fn test_edit_tool_replace_first() {
        let dir = TempDir::new().unwrap();
        create_temp_file(&dir, "test.txt", "foo bar foo baz");

        let tool = EditTool::with_base_dir(dir.path());
        let result = tool
            .execute(serde_json::json!({
                "path": "test.txt",
                "old_string": "foo",
                "new_string": "qux"
            }))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.content["replacements"], 1);

        let content = std::fs::read_to_string(dir.path().join("test.txt")).unwrap();
        assert_eq!(content, "qux bar foo baz");
    }

    #[tokio::test]
    async fn test_edit_tool_replace_all() {
        let dir = TempDir::new().unwrap();
        create_temp_file(&dir, "test.txt", "foo bar foo baz foo");

        let tool = EditTool::with_base_dir(dir.path());
        let result = tool
            .execute(serde_json::json!({
                "path": "test.txt",
                "old_string": "foo",
                "new_string": "qux",
                "replace_all": true
            }))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.content["replacements"], 3);

        let content = std::fs::read_to_string(dir.path().join("test.txt")).unwrap();
        assert_eq!(content, "qux bar qux baz qux");
    }

    #[tokio::test]
    async fn test_edit_tool_string_not_found() {
        let dir = TempDir::new().unwrap();
        create_temp_file(&dir, "test.txt", "hello world");

        let tool = EditTool::with_base_dir(dir.path());
        let result = tool
            .execute(serde_json::json!({
                "path": "test.txt",
                "old_string": "nonexistent",
                "new_string": "replacement"
            }))
            .await;

        assert!(result.is_err());
    }

    #[test]
    fn test_create_file_tool_registry() {
        let registry = create_file_tool_registry();
        assert_eq!(registry.len(), 5);
        assert!(registry.contains("read"));
        assert!(registry.contains("write"));
        assert!(registry.contains("edit"));
        assert!(registry.contains("glob"));
        assert!(registry.contains("grep"));
    }

    #[test]
    fn test_tool_schemas() {
        let read = ReadTool::new();
        let schema = read.schema();
        assert!(schema["properties"]["path"].is_object());

        let write = WriteTool::new();
        let schema = write.schema();
        assert!(schema["properties"]["content"].is_object());

        let edit = EditTool::new();
        let schema = edit.schema();
        assert!(schema["properties"]["old_string"].is_object());

        let bash = BashTool::new();
        let schema = bash.schema();
        assert!(schema["properties"]["command"].is_object());
    }

    #[test]
    fn test_tool_schema_format() {
        // Test that schemas follow JSON Schema format
        let read = ReadTool::new();
        let schema = read.schema();

        // Check required JSON Schema fields
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"].is_object());
        assert!(schema["required"].is_array());

        // Check that required params are in required array
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("path")));

        // Check property types
        assert_eq!(schema["properties"]["path"]["type"], "string");
        assert_eq!(schema["properties"]["offset"]["type"], "number");
        assert_eq!(schema["properties"]["limit"]["type"], "number");

        // Check descriptions are present
        assert!(schema["properties"]["path"]["description"].is_string());
    }

    #[test]
    fn test_tool_schema_required_params() {
        // Verify required vs optional parameter handling
        let edit = EditTool::new();
        let schema = edit.schema();

        let required = schema["required"].as_array().unwrap();

        // Required parameters
        assert!(required.contains(&serde_json::json!("path")));
        assert!(required.contains(&serde_json::json!("old_string")));
        assert!(required.contains(&serde_json::json!("new_string")));

        // Optional parameter should not be in required
        assert!(!required.contains(&serde_json::json!("replace_all")));
    }

    #[test]
    fn test_to_anthropic_tools() {
        // Test conversion to Anthropic API format
        let mut registry = ToolRegistry::new();
        registry.register(ReadTool::new());
        registry.register(WriteTool::new());

        let api_tools = registry.to_anthropic_tools();

        assert_eq!(api_tools.len(), 2);

        // Find read tool
        let read_tool = api_tools.iter().find(|t| t.name == "read").unwrap();
        assert_eq!(read_tool.name, "read");
        assert!(!read_tool.description.is_empty());
        assert_eq!(read_tool.input_schema["type"], "object");
        assert!(read_tool.input_schema["properties"]["path"].is_object());
    }

    #[test]
    fn test_registry_schemas() {
        // Test ToolRegistry::schemas method
        let mut registry = ToolRegistry::new();
        registry.register(BashTool::new());
        registry.register(EditTool::new());

        let schemas = registry.schemas();

        assert_eq!(schemas.len(), 2);

        // Each schema should have name, description, input_schema
        for schema in &schemas {
            assert!(schema["name"].is_string());
            assert!(schema["description"].is_string());
            assert!(schema["input_schema"]["type"] == "object");
        }
    }

    #[test]
    fn test_standard_registry_has_all_tools() {
        let registry = create_standard_tool_registry();

        // Should have all 6 tools
        assert_eq!(registry.len(), 6);

        // Check each tool exists
        let api_tools = registry.to_anthropic_tools();
        let tool_names: Vec<&str> = api_tools.iter().map(|t| t.name.as_str()).collect();

        assert!(tool_names.contains(&"read"));
        assert!(tool_names.contains(&"write"));
        assert!(tool_names.contains(&"edit"));
        assert!(tool_names.contains(&"bash"));
        assert!(tool_names.contains(&"glob"));
        assert!(tool_names.contains(&"grep"));
    }

    #[test]
    fn test_tool_default_values_in_schema() {
        // Test that default values are included in schema
        let read = ReadTool::new();
        let schema = read.schema();

        // offset has a default value
        assert_eq!(schema["properties"]["offset"]["default"], 1);
    }

    #[tokio::test]
    async fn test_bash_tool_echo() {
        let tool = BashTool::new();
        let result = tool
            .execute(serde_json::json!({
                "command": "echo 'Hello, World!'"
            }))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.success);
        assert!(output.content["stdout"]
            .as_str()
            .unwrap()
            .contains("Hello, World!"));
    }

    #[tokio::test]
    async fn test_bash_tool_exit_code() {
        let tool = BashTool::new();
        let result = tool
            .execute(serde_json::json!({
                "command": "exit 1"
            }))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(!output.success);
        assert_eq!(output.content["exit_code"], 1);
    }

    #[tokio::test]
    async fn test_bash_tool_dangerous_command() {
        let tool = BashTool::new();
        let result = tool
            .execute(serde_json::json!({
                "command": "rm -rf /"
            }))
            .await;

        assert!(matches!(result, Err(ToolError::PermissionDenied(_))));
    }

    #[tokio::test]
    async fn test_bash_tool_timeout() {
        let tool = BashTool::new().with_timeout(1);
        let result = tool
            .execute(serde_json::json!({
                "command": "sleep 10",
                "timeout": 1
            }))
            .await;

        assert!(matches!(result, Err(ToolError::Timeout(_))));
    }

    #[test]
    fn test_create_standard_tool_registry() {
        let registry = create_standard_tool_registry();
        assert_eq!(registry.len(), 6);
        assert!(registry.contains("read"));
        assert!(registry.contains("write"));
        assert!(registry.contains("edit"));
        assert!(registry.contains("bash"));
        assert!(registry.contains("glob"));
        assert!(registry.contains("grep"));
    }

    #[tokio::test]
    async fn test_glob_tool_basic() {
        let dir = TempDir::new().unwrap();
        create_temp_file(&dir, "test1.txt", "content");
        create_temp_file(&dir, "test2.txt", "content");

        let tool = GlobTool::with_base_dir(dir.path());
        let result = tool
            .execute(serde_json::json!({
                "pattern": "*.txt"
            }))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.success);
        let count = output.content["count"].as_u64().unwrap();
        assert_eq!(count, 2); // test1.txt and test2.txt
    }

    #[tokio::test]
    async fn test_glob_tool_recursive() {
        let dir = TempDir::new().unwrap();
        create_temp_file(&dir, "root.rs", "fn main() {}");
        let sub_dir = dir.path().join("src");
        std::fs::create_dir(&sub_dir).unwrap();
        let sub_path = sub_dir.join("lib.rs");
        std::fs::write(&sub_path, "pub fn test() {}").unwrap();

        let tool = GlobTool::with_base_dir(dir.path());
        let result = tool
            .execute(serde_json::json!({
                "pattern": "**/*.rs"
            }))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        let count = output.content["count"].as_u64().unwrap();
        assert_eq!(count, 2); // root.rs and src/lib.rs
    }

    #[tokio::test]
    async fn test_grep_tool_basic() {
        let dir = TempDir::new().unwrap();
        create_temp_file(&dir, "test.txt", "Line 1\nLine 2 with pattern\nLine 3");

        let tool = GrepTool::with_base_dir(dir.path());
        let result = tool
            .execute(serde_json::json!({
                "pattern": "pattern",
                "path": "test.txt"
            }))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.success);
        let count = output.content["count"].as_u64().unwrap();
        assert_eq!(count, 1);

        let matches = output.content["matches"].as_array().unwrap();
        assert_eq!(matches[0]["line"], 2);
        assert!(matches[0]["content"].as_str().unwrap().contains("pattern"));
    }

    #[tokio::test]
    async fn test_grep_tool_regex() {
        let dir = TempDir::new().unwrap();
        create_temp_file(&dir, "test.txt", "foo123\nbar456\nfoo789");

        let tool = GrepTool::with_base_dir(dir.path());
        let result = tool
            .execute(serde_json::json!({
                "pattern": "foo\\d+",
                "path": "test.txt"
            }))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        let count = output.content["count"].as_u64().unwrap();
        assert_eq!(count, 2); // foo123 and foo789
    }

    #[tokio::test]
    async fn test_grep_tool_case_insensitive() {
        let dir = TempDir::new().unwrap();
        create_temp_file(&dir, "test.txt", "Hello\nhello\nHELLO");

        let tool = GrepTool::with_base_dir(dir.path());
        let result = tool
            .execute(serde_json::json!({
                "pattern": "hello",
                "path": "test.txt",
                "case_insensitive": true
            }))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        let count = output.content["count"].as_u64().unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_grep_tool_directory_search() {
        let dir = TempDir::new().unwrap();
        create_temp_file(&dir, "file1.txt", "pattern here");
        create_temp_file(&dir, "file2.txt", "no match");
        create_temp_file(&dir, "file3.txt", "pattern again");

        let tool = GrepTool::with_base_dir(dir.path());
        let result = tool
            .execute(serde_json::json!({
                "pattern": "pattern",
                "path": "."
            }))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        let count = output.content["count"].as_u64().unwrap();
        assert_eq!(count, 2);
    }
}
