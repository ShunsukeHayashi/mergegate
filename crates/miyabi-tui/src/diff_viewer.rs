//! Diff Viewer Widget
//!
//! This module provides an enhanced diff visualization with proper colors,
//! line numbers, and indicators for a professional git diff display.

use crate::diff_render::{DiffRender, DiffLine, DiffLineType};
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
}

impl Default for DiffViewerOptions {
    fn default() -> Self {
        Self {
            colors: DiffColors::default(),
            show_line_numbers: true,
            line_number_width: 4,
            show_gutter: true,
            show_file_headers: true,
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
        }
    }

    /// Create with custom options
    pub fn with_options(options: DiffViewerOptions) -> Self {
        Self {
            diff: DiffRender::new(),
            options,
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

            for hunk in &file.hunks {
                for diff_line in &hunk.lines {
                    lines.push(self.render_line(diff_line));
                }
            }
        }

        lines
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

    /// Render a single diff line
    fn render_line(&self, line: &DiffLine) -> Line<'static> {
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

        // Content
        let (content, style) = match line.line_type {
            DiffLineType::Addition => {
                let mut style = Style::default().fg(self.options.colors.addition_fg);
                if let Some(bg) = self.options.colors.addition_bg {
                    style = style.bg(bg);
                }
                (line.content.clone(), style)
            }
            DiffLineType::Deletion => {
                let mut style = Style::default().fg(self.options.colors.deletion_fg);
                if let Some(bg) = self.options.colors.deletion_bg {
                    style = style.bg(bg);
                }
                (line.content.clone(), style)
            }
            DiffLineType::Context => (
                line.content.clone(),
                Style::default().fg(self.options.colors.context_fg),
            ),
            DiffLineType::HunkHeader => (
                line.content.clone(),
                Style::default()
                    .fg(self.options.colors.hunk_header_fg)
                    .add_modifier(Modifier::DIM),
            ),
            DiffLineType::FileHeader => (
                line.content.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        };

        spans.push(Span::styled(content, style));

        Line::from(spans)
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
}
