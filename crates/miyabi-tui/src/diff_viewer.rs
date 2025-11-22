//! Diff Viewer Widget
//!
//! This module provides an enhanced diff visualization with proper colors,
//! line numbers, and indicators for a professional git diff display.

use crate::diff_render::{DiffRender, DiffLine, DiffLineType};
use crate::syntax::{normalize_language, SyntaxHighlighter};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Color scheme for diff visualization
#[derive(Debug, Clone)]
pub struct DiffColors {
    /// Addition text color
    pub addition_fg: Color,
    /// Addition background color
    pub addition_bg: Option<Color>,
    /// Deletion text color
    pub deletion_fg: Color,
    /// Deletion background color
    pub deletion_bg: Option<Color>,
    /// Context text color
    pub context_fg: Color,
    /// Hunk header color
    pub hunk_header_fg: Color,
    /// Line number color
    pub line_number_fg: Color,
    /// Gutter indicator color for additions
    pub gutter_add_fg: Color,
    /// Gutter indicator color for deletions
    pub gutter_del_fg: Color,
}

impl Default for DiffColors {
    fn default() -> Self {
        Self {
            // GitHub-inspired colors
            addition_fg: Color::Rgb(0x2e, 0xa0, 0x43), // #2ea043
            addition_bg: Some(Color::Rgb(0x0d, 0x11, 0x17)),
            deletion_fg: Color::Rgb(0xf8, 0x51, 0x49), // #f85149
            deletion_bg: Some(Color::Rgb(0x11, 0x0d, 0x0d)),
            context_fg: Color::Gray,
            hunk_header_fg: Color::Cyan,
            line_number_fg: Color::DarkGray,
            gutter_add_fg: Color::Green,
            gutter_del_fg: Color::Red,
        }
    }
}

impl DiffColors {
    /// Create a minimal color scheme (no backgrounds)
    pub fn minimal() -> Self {
        Self {
            addition_bg: None,
            deletion_bg: None,
            ..Default::default()
        }
    }

    /// Create a high contrast color scheme
    pub fn high_contrast() -> Self {
        Self {
            addition_fg: Color::LightGreen,
            deletion_fg: Color::LightRed,
            context_fg: Color::White,
            ..Default::default()
        }
    }
}

/// Options for diff viewer
#[derive(Debug, Clone)]
pub struct DiffViewerOptions {
    /// Color scheme to use
    pub colors: DiffColors,
    /// Show line numbers
    pub show_line_numbers: bool,
    /// Line number width (characters)
    pub line_number_width: usize,
    /// Show gutter indicators (+/-)
    pub show_gutter: bool,
    /// Show file headers
    pub show_file_headers: bool,
    /// Enable syntax highlighting for code content
    pub enable_syntax_highlighting: bool,
}

impl Default for DiffViewerOptions {
    fn default() -> Self {
        Self {
            colors: DiffColors::default(),
            show_line_numbers: true,
            line_number_width: 4,
            show_gutter: true,
            show_file_headers: true,
            enable_syntax_highlighting: true,
        }
    }
}

/// Diff viewer that renders parsed diffs with enhanced visualization
#[derive(Debug, Clone)]
pub struct DiffViewer {
    /// Parsed diff data
    diff: DiffRender,
    /// Viewer options
    options: DiffViewerOptions,
    /// Syntax highlighter for code content
    highlighter: SyntaxHighlighter,
}

impl Default for DiffViewer {
    fn default() -> Self {
        Self::new()
    }
}

impl DiffViewer {
    /// Create a new diff viewer
    pub fn new() -> Self {
        Self {
            diff: DiffRender::new(),
            options: DiffViewerOptions::default(),
            highlighter: SyntaxHighlighter::new(),
        }
    }

    /// Create with custom options
    pub fn with_options(options: DiffViewerOptions) -> Self {
        Self {
            diff: DiffRender::new(),
            options,
            highlighter: SyntaxHighlighter::new(),
        }
    }

    /// Set the diff data
    pub fn set_diff(&mut self, diff: DiffRender) {
        self.diff = diff;
    }

    /// Parse diff text
    pub fn parse(&mut self, diff_text: &str) -> &mut Self {
        self.diff.parse(diff_text);
        self
    }

    /// Get line count
    pub fn line_count(&self) -> usize {
        self.diff.line_count()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.diff.is_empty()
    }

    /// Render to styled lines
    pub fn render(&self) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        if self.diff.is_empty() {
            lines.push(Line::from(Span::styled(
                "No changes",
                Style::default().fg(Color::DarkGray),
            )));
            return lines;
        }

        for file in &self.diff.files {
            // File header
            if self.options.show_file_headers {
                lines.push(self.render_file_header(&file.old_path, &file.new_path));
            }

            // Extract file extension for syntax highlighting
            let extension = Self::extract_extension(&file.new_path);

            for hunk in &file.hunks {
                for diff_line in &hunk.lines {
                    lines.push(self.render_line(diff_line, extension.as_deref()));
                }
            }
        }

        lines
    }

    /// Extract file extension from path
    fn extract_extension(path: &str) -> Option<String> {
        std::path::Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_string())
    }

    /// Render file header
    fn render_file_header(&self, old_path: &str, new_path: &str) -> Line<'static> {
        let content = if old_path == new_path {
            format!(" {} ", new_path)
        } else {
            format!(" {} → {} ", old_path, new_path)
        };

        Line::from(Span::styled(
            content,
            Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ))
    }

    /// Render a single diff line with optional syntax highlighting
    fn render_line(&self, line: &DiffLine, extension: Option<&str>) -> Line<'static> {
        let mut spans = Vec::new();

        // Line numbers
        if self.options.show_line_numbers {
            let (old_num, new_num) = self.format_line_numbers(line);
            spans.push(Span::styled(
                old_num,
                Style::default().fg(self.options.colors.line_number_fg),
            ));
            spans.push(Span::styled(
                new_num,
                Style::default().fg(self.options.colors.line_number_fg),
            ));
        }

        // Gutter indicator
        if self.options.show_gutter {
            let (indicator, color) = match line.line_type {
                DiffLineType::Addition => ("+", self.options.colors.gutter_add_fg),
                DiffLineType::Deletion => ("-", self.options.colors.gutter_del_fg),
                DiffLineType::Context => (" ", self.options.colors.context_fg),
                DiffLineType::HunkHeader => ("@", self.options.colors.hunk_header_fg),
                DiffLineType::FileHeader => (" ", Color::White),
            };
            spans.push(Span::styled(
                format!("{} ", indicator),
                Style::default().fg(color),
            ));
        }

        // Content with optional syntax highlighting
        match line.line_type {
            DiffLineType::Addition | DiffLineType::Deletion | DiffLineType::Context => {
                // Determine background color for additions/deletions
                let bg_color = match line.line_type {
                    DiffLineType::Addition => self.options.colors.addition_bg,
                    DiffLineType::Deletion => self.options.colors.deletion_bg,
                    _ => None,
                };

                // Apply syntax highlighting if enabled and extension is available
                if self.options.enable_syntax_highlighting {
                    if let Some(ext) = extension {
                        let lang = normalize_language(ext);
                        let highlighted = self.highlighter.highlight_line(&line.content, lang);

                        // Apply background color to each highlighted span
                        for span in highlighted.spans {
                            let mut style = span.style;
                            if let Some(bg) = bg_color {
                                style = style.bg(bg);
                            }
                            spans.push(Span::styled(span.content.into_owned(), style));
                        }
                    } else {
                        // No extension, use default colors
                        let style = self.get_content_style(&line.line_type, bg_color);
                        spans.push(Span::styled(line.content.clone(), style));
                    }
                } else {
                    // Syntax highlighting disabled
                    let style = self.get_content_style(&line.line_type, bg_color);
                    spans.push(Span::styled(line.content.clone(), style));
                }
            }
            DiffLineType::HunkHeader => {
                spans.push(Span::styled(
                    line.content.clone(),
                    Style::default()
                        .fg(self.options.colors.hunk_header_fg)
                        .add_modifier(Modifier::DIM),
                ));
            }
            DiffLineType::FileHeader => {
                spans.push(Span::styled(
                    line.content.clone(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ));
            }
        }

        Line::from(spans)
    }

    /// Get content style for a line type (when syntax highlighting is disabled)
    fn get_content_style(&self, line_type: &DiffLineType, bg_color: Option<Color>) -> Style {
        let mut style = match line_type {
            DiffLineType::Addition => Style::default().fg(self.options.colors.addition_fg),
            DiffLineType::Deletion => Style::default().fg(self.options.colors.deletion_fg),
            DiffLineType::Context => Style::default().fg(self.options.colors.context_fg),
            _ => Style::default(),
        };

        if let Some(bg) = bg_color {
            style = style.bg(bg);
        }

        style
    }

    /// Format line numbers
    fn format_line_numbers(&self, line: &DiffLine) -> (String, String) {
        let width = self.options.line_number_width;

        let old = match line.old_line_num {
            Some(n) => format!("{:>width$} ", n, width = width),
            None => format!("{:>width$} ", "", width = width),
        };

        let new = match line.new_line_num {
            Some(n) => format!("{:>width$} ", n, width = width),
            None => format!("{:>width$} ", "", width = width),
        };

        (old, new)
    }
}

/// Render a diff string to styled lines with default options
pub fn render_diff(diff_text: &str) -> Vec<Line<'static>> {
    let mut viewer = DiffViewer::new();
    viewer.parse(diff_text);
    viewer.render()
}

/// Render a diff string with minimal options (no backgrounds)
pub fn render_diff_minimal(diff_text: &str) -> Vec<Line<'static>> {
    let options = DiffViewerOptions {
        colors: DiffColors::minimal(),
        ..Default::default()
    };
    let mut viewer = DiffViewer::with_options(options);
    viewer.parse(diff_text);
    viewer.render()
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
    fn test_viewer_creation() {
        let viewer = DiffViewer::new();
        assert!(viewer.is_empty());
    }

    #[test]
    fn test_parse_and_render() {
        let mut viewer = DiffViewer::new();
        viewer.parse(SAMPLE_DIFF);

        assert!(!viewer.is_empty());
        let lines = viewer.render();
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_custom_colors() {
        let colors = DiffColors {
            addition_fg: Color::LightGreen,
            deletion_fg: Color::LightRed,
            ..Default::default()
        };
        let options = DiffViewerOptions {
            colors,
            ..Default::default()
        };
        let mut viewer = DiffViewer::with_options(options);
        viewer.parse(SAMPLE_DIFF);

        let lines = viewer.render();
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_minimal_colors() {
        let colors = DiffColors::minimal();
        assert!(colors.addition_bg.is_none());
        assert!(colors.deletion_bg.is_none());
    }

    #[test]
    fn test_high_contrast() {
        let colors = DiffColors::high_contrast();
        assert_eq!(colors.addition_fg, Color::LightGreen);
    }

    #[test]
    fn test_render_diff_helper() {
        let lines = render_diff(SAMPLE_DIFF);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_render_diff_minimal_helper() {
        let lines = render_diff_minimal(SAMPLE_DIFF);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_empty_diff() {
        let viewer = DiffViewer::new();
        let lines = viewer.render();

        assert_eq!(lines.len(), 1);
        // Should show "No changes"
    }

    #[test]
    fn test_options_default() {
        let options = DiffViewerOptions::default();
        assert!(options.show_line_numbers);
        assert!(options.show_gutter);
        assert!(options.show_file_headers);
        assert_eq!(options.line_number_width, 4);
    }

    #[test]
    fn test_hide_line_numbers() {
        let options = DiffViewerOptions {
            show_line_numbers: false,
            ..Default::default()
        };
        let mut viewer = DiffViewer::with_options(options);
        viewer.parse(SAMPLE_DIFF);

        let lines = viewer.render();
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_hide_gutter() {
        let options = DiffViewerOptions {
            show_gutter: false,
            ..Default::default()
        };
        let mut viewer = DiffViewer::with_options(options);
        viewer.parse(SAMPLE_DIFF);

        let lines = viewer.render();
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_syntax_highlighting_enabled_by_default() {
        let options = DiffViewerOptions::default();
        assert!(options.enable_syntax_highlighting);
    }

    #[test]
    fn test_syntax_highlighting_disabled() {
        let options = DiffViewerOptions {
            enable_syntax_highlighting: false,
            ..Default::default()
        };
        let mut viewer = DiffViewer::with_options(options);
        viewer.parse(SAMPLE_DIFF);

        let lines = viewer.render();
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_extract_extension() {
        assert_eq!(
            DiffViewer::extract_extension("src/main.rs"),
            Some("rs".to_string())
        );
        assert_eq!(
            DiffViewer::extract_extension("app.js"),
            Some("js".to_string())
        );
        assert_eq!(
            DiffViewer::extract_extension("no_extension"),
            None
        );
        assert_eq!(
            DiffViewer::extract_extension("/path/to/file.py"),
            Some("py".to_string())
        );
    }

    #[test]
    fn test_syntax_highlighting_with_rust() {
        let mut viewer = DiffViewer::new();
        viewer.parse(SAMPLE_DIFF);

        let lines = viewer.render();
        // Should have file header + hunk header + content lines
        assert!(lines.len() > 2);
    }

    #[test]
    fn test_syntax_highlighting_with_python() {
        let python_diff = r#"diff --git a/app.py b/app.py
--- a/app.py
+++ b/app.py
@@ -1,3 +1,4 @@
 def main():
-    print("Hello")
+    print("Hello, World!")
+    return 0
"#;
        let mut viewer = DiffViewer::new();
        viewer.parse(python_diff);

        let lines = viewer.render();
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_syntax_highlighting_with_javascript() {
        let js_diff = r#"diff --git a/index.js b/index.js
--- a/index.js
+++ b/index.js
@@ -1,2 +1,3 @@
 const x = 1;
+const y = 2;
 console.log(x);
"#;
        let mut viewer = DiffViewer::new();
        viewer.parse(js_diff);

        let lines = viewer.render();
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_syntax_highlighting_preserves_backgrounds() {
        let mut viewer = DiffViewer::new();
        viewer.parse(SAMPLE_DIFF);

        // Render and check that we have content
        let lines = viewer.render();
        assert!(!lines.is_empty());

        // The rendered lines should have spans for syntax-highlighted content
        for line in &lines {
            assert!(!line.spans.is_empty());
        }
    }
}
