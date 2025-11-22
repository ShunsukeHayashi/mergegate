//! Git Diff Renderer
//!
//! This module provides parsing and rendering of unified diff format.

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Type of diff line change
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffLineType {
    /// Context line (unchanged)
    Context,
    /// Added line
    Addition,
    /// Removed line
    Deletion,
    /// Hunk header
    HunkHeader,
    /// File header
    FileHeader,
}

/// A single line in a diff
#[derive(Debug, Clone)]
pub struct DiffLine {
    /// Type of change
    pub line_type: DiffLineType,
    /// Line content (without +/- prefix)
    pub content: String,
    /// Old line number (for context and deletions)
    pub old_line_num: Option<usize>,
    /// New line number (for context and additions)
    pub new_line_num: Option<usize>,
}

/// A hunk in a diff (a contiguous block of changes)
#[derive(Debug, Clone)]
pub struct DiffHunk {
    /// Starting line in old file
    pub old_start: usize,
    /// Number of lines in old file
    pub old_count: usize,
    /// Starting line in new file
    pub new_start: usize,
    /// Number of lines in new file
    pub new_count: usize,
    /// Header text (e.g., function name)
    pub header: String,
    /// Lines in this hunk
    pub lines: Vec<DiffLine>,
}

/// A single file's diff
#[derive(Debug, Clone)]
pub struct FileDiff {
    /// Old file path
    pub old_path: String,
    /// New file path
    pub new_path: String,
    /// Hunks in this file
    pub hunks: Vec<DiffHunk>,
}

/// Git diff renderer
#[derive(Debug, Clone, Default)]
pub struct DiffRender {
    /// Parsed file diffs
    pub files: Vec<FileDiff>,
    /// Current scroll offset
    pub scroll_offset: usize,
}

impl DiffRender {
    /// Create a new diff renderer
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse unified diff format
    pub fn parse(&mut self, diff_text: &str) -> &mut Self {
        self.files.clear();

        let mut current_file: Option<FileDiff> = None;
        let mut current_hunk: Option<DiffHunk> = None;
        let mut old_line_num = 0;
        let mut new_line_num = 0;

        for line in diff_text.lines() {
            // File header: diff --git a/file b/file
            if line.starts_with("diff --git") {
                // Save previous file if exists
                if let Some(mut file) = current_file.take() {
                    if let Some(hunk) = current_hunk.take() {
                        file.hunks.push(hunk);
                    }
                    self.files.push(file);
                }

                // Parse file paths
                let parts: Vec<&str> = line.split_whitespace().collect();
                let old_path = parts.get(2).unwrap_or(&"").trim_start_matches("a/").to_string();
                let new_path = parts.get(3).unwrap_or(&"").trim_start_matches("b/").to_string();

                current_file = Some(FileDiff {
                    old_path,
                    new_path,
                    hunks: Vec::new(),
                });
                continue;
            }

            // Skip --- and +++ lines (we already have paths)
            if line.starts_with("---") || line.starts_with("+++") {
                continue;
            }

            // Hunk header: @@ -start,count +start,count @@ context
            if line.starts_with("@@") {
                // Save previous hunk
                if let Some(ref mut file) = current_file {
                    if let Some(hunk) = current_hunk.take() {
                        file.hunks.push(hunk);
                    }
                }

                // Parse hunk header
                if let Some((old_start, old_count, new_start, new_count, header)) = parse_hunk_header(line) {
                    old_line_num = old_start;
                    new_line_num = new_start;

                    current_hunk = Some(DiffHunk {
                        old_start,
                        old_count,
                        new_start,
                        new_count,
                        header,
                        lines: vec![DiffLine {
                            line_type: DiffLineType::HunkHeader,
                            content: line.to_string(),
                            old_line_num: None,
                            new_line_num: None,
                        }],
                    });
                }
                continue;
            }

            // Diff content lines
            if let Some(ref mut hunk) = current_hunk {
                let (line_type, content) = if line.starts_with('+') {
                    (DiffLineType::Addition, line[1..].to_string())
                } else if line.starts_with('-') {
                    (DiffLineType::Deletion, line[1..].to_string())
                } else if line.starts_with(' ') {
                    (DiffLineType::Context, line[1..].to_string())
                } else {
                    (DiffLineType::Context, line.to_string())
                };

                let (old_num, new_num) = match line_type {
                    DiffLineType::Addition => {
                        let num = new_line_num;
                        new_line_num += 1;
                        (None, Some(num))
                    }
                    DiffLineType::Deletion => {
                        let num = old_line_num;
                        old_line_num += 1;
                        (Some(num), None)
                    }
                    DiffLineType::Context => {
                        let old = old_line_num;
                        let new = new_line_num;
                        old_line_num += 1;
                        new_line_num += 1;
                        (Some(old), Some(new))
                    }
                    _ => (None, None),
                };

                hunk.lines.push(DiffLine {
                    line_type,
                    content,
                    old_line_num: old_num,
                    new_line_num: new_num,
                });
            }
        }

        // Save last file and hunk
        if let Some(mut file) = current_file {
            if let Some(hunk) = current_hunk {
                file.hunks.push(hunk);
            }
            self.files.push(file);
        }

        self
    }

    /// Get total number of lines
    pub fn line_count(&self) -> usize {
        self.files.iter()
            .flat_map(|f| &f.hunks)
            .map(|h| h.lines.len())
            .sum()
    }

    /// Check if diff is empty
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Render diff to Ratatui lines
    pub fn render(&self) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        for file in &self.files {
            // File header
            lines.push(Line::from(Span::styled(
                format!("diff --git a/{} b/{}", file.old_path, file.new_path),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )));

            for hunk in &file.hunks {
                for diff_line in &hunk.lines {
                    let line = self.render_line(diff_line);
                    lines.push(line);
                }
            }
        }

        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "No changes",
                Style::default().fg(Color::DarkGray),
            )));
        }

        lines
    }

    /// Render a single diff line
    fn render_line(&self, diff_line: &DiffLine) -> Line<'static> {
        let (prefix, style) = match diff_line.line_type {
            DiffLineType::Addition => (
                "+",
                Style::default().fg(Color::Green),
            ),
            DiffLineType::Deletion => (
                "-",
                Style::default().fg(Color::Red),
            ),
            DiffLineType::Context => (
                " ",
                Style::default(),
            ),
            DiffLineType::HunkHeader => (
                "",
                Style::default().fg(Color::Cyan),
            ),
            DiffLineType::FileHeader => (
                "",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        };

        // Build line number prefix
        let line_nums = match (diff_line.old_line_num, diff_line.new_line_num) {
            (Some(old), Some(new)) => format!("{:4} {:4} ", old, new),
            (Some(old), None) => format!("{:4}      ", old),
            (None, Some(new)) => format!("     {:4} ", new),
            (None, None) => "          ".to_string(),
        };

        let content = if diff_line.line_type == DiffLineType::HunkHeader {
            diff_line.content.clone()
        } else {
            format!("{}{}{}", line_nums, prefix, diff_line.content)
        };

        Line::from(Span::styled(content, style))
    }
}

/// Parse hunk header: @@ -old_start,old_count +new_start,new_count @@ context
fn parse_hunk_header(line: &str) -> Option<(usize, usize, usize, usize, String)> {
    let line = line.trim_start_matches("@@ ");
    let parts: Vec<&str> = line.splitn(2, " @@").collect();

    if parts.is_empty() {
        return None;
    }

    let ranges = parts[0];
    let header = parts.get(1).unwrap_or(&"").trim().to_string();

    let range_parts: Vec<&str> = ranges.split_whitespace().collect();
    if range_parts.len() < 2 {
        return None;
    }

    let old_range = range_parts[0].trim_start_matches('-');
    let new_range = range_parts[1].trim_start_matches('+');

    let (old_start, old_count) = parse_range(old_range)?;
    let (new_start, new_count) = parse_range(new_range)?;

    Some((old_start, old_count, new_start, new_count, header))
}

/// Parse a range like "10,5" or "10"
fn parse_range(range: &str) -> Option<(usize, usize)> {
    let parts: Vec<&str> = range.split(',').collect();
    let start = parts[0].parse().ok()?;
    let count = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    Some((start, count))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_DIFF: &str = r#"diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,4 +1,5 @@
 fn main() {
-    println!("Hello");
+    println!("Hello, World!");
+    println!("Goodbye");
 }
"#;

    #[test]
    fn test_parse_diff() {
        let mut renderer = DiffRender::new();
        renderer.parse(SAMPLE_DIFF);

        assert_eq!(renderer.files.len(), 1);
        assert_eq!(renderer.files[0].old_path, "src/main.rs");
        assert_eq!(renderer.files[0].new_path, "src/main.rs");
    }

    #[test]
    fn test_hunk_parsing() {
        let mut renderer = DiffRender::new();
        renderer.parse(SAMPLE_DIFF);

        let hunk = &renderer.files[0].hunks[0];
        assert_eq!(hunk.old_start, 1);
        assert_eq!(hunk.old_count, 4);
        assert_eq!(hunk.new_start, 1);
        assert_eq!(hunk.new_count, 5);
    }

    #[test]
    fn test_line_types() {
        let mut renderer = DiffRender::new();
        renderer.parse(SAMPLE_DIFF);

        let lines = &renderer.files[0].hunks[0].lines;

        // First line is hunk header
        assert_eq!(lines[0].line_type, DiffLineType::HunkHeader);

        // Find addition and deletion
        let has_addition = lines.iter().any(|l| l.line_type == DiffLineType::Addition);
        let has_deletion = lines.iter().any(|l| l.line_type == DiffLineType::Deletion);

        assert!(has_addition);
        assert!(has_deletion);
    }

    #[test]
    fn test_render() {
        let mut renderer = DiffRender::new();
        renderer.parse(SAMPLE_DIFF);

        let lines = renderer.render();
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_empty_diff() {
        let renderer = DiffRender::new();
        assert!(renderer.is_empty());

        let lines = renderer.render();
        assert_eq!(lines.len(), 1); // "No changes" message
    }
}
