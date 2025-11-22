//! Markdown to Terminal Renderer
//!
//! Converts markdown text to styled ratatui Lines/Spans

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Markdown styling configuration
pub struct MarkdownStyles {
    pub h1: Style,
    pub h2: Style,
    pub h3: Style,
    pub bold: Style,
    pub italic: Style,
    pub code: Style,
    pub code_block: Style,
    pub link: Style,
    pub list_marker: Style,
    pub blockquote: Style,
}

impl Default for MarkdownStyles {
    fn default() -> Self {
        Self {
            h1: Style::default()
                .fg(Color::Rgb(224, 175, 104))
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            h2: Style::default()
                .fg(Color::Rgb(224, 175, 104))
                .add_modifier(Modifier::BOLD),
            h3: Style::default()
                .fg(Color::Rgb(192, 202, 245))
                .add_modifier(Modifier::ITALIC),
            bold: Style::default().add_modifier(Modifier::BOLD),
            italic: Style::default().add_modifier(Modifier::ITALIC),
            code: Style::default().fg(Color::Rgb(125, 207, 255)),
            code_block: Style::default().fg(Color::Rgb(125, 207, 255)),
            link: Style::default()
                .fg(Color::Rgb(125, 207, 255))
                .add_modifier(Modifier::UNDERLINED),
            list_marker: Style::default().fg(Color::Rgb(158, 206, 106)),
            blockquote: Style::default().fg(Color::Rgb(158, 206, 106)),
        }
    }
}

/// Markdown renderer
pub struct MarkdownRenderer {
    styles: MarkdownStyles,
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        Self {
            styles: MarkdownStyles::default(),
        }
    }

    pub fn with_styles(styles: MarkdownStyles) -> Self {
        Self { styles }
    }

    /// Render markdown text to styled Lines
    pub fn render(&self, text: &str) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let mut in_code_block = false;
        let mut code_block_content: Vec<String> = Vec::new();

        for line in text.lines() {
            if line.trim().starts_with("```") {
                if in_code_block {
                    // End code block
                    lines.push(Line::from(Span::styled(
                        "┌─────────────────────────────────────────────────────────────┐",
                        Style::default().fg(Color::Rgb(86, 95, 137)),
                    )));
                    for code_line in &code_block_content {
                        lines.push(Line::from(vec![
                            Span::styled("│ ", Style::default().fg(Color::Rgb(86, 95, 137))),
                            Span::styled(code_line.clone(), self.styles.code_block),
                        ]));
                    }
                    lines.push(Line::from(Span::styled(
                        "└─────────────────────────────────────────────────────────────┘",
                        Style::default().fg(Color::Rgb(86, 95, 137)),
                    )));
                    code_block_content.clear();
                    in_code_block = false;
                } else {
                    in_code_block = true;
                }
                continue;
            }

            if in_code_block {
                code_block_content.push(line.to_string());
                continue;
            }

            let rendered = self.render_line(line);
            lines.push(rendered);
        }

        lines
    }

    fn render_line(&self, line: &str) -> Line<'static> {
        let trimmed = line.trim();

        // Headers
        if let Some(stripped) = trimmed.strip_prefix("### ") {
            return Line::from(vec![
                Span::styled("   ", Style::default()),
                Span::styled(stripped.to_string(), self.styles.h3),
            ]);
        }
        if let Some(stripped) = trimmed.strip_prefix("## ") {
            return Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(stripped.to_string(), self.styles.h2),
            ]);
        }
        if let Some(stripped) = trimmed.strip_prefix("# ") {
            return Line::from(vec![
                Span::styled(" ", Style::default()),
                Span::styled(stripped.to_string(), self.styles.h1),
            ]);
        }

        // Blockquotes
        if let Some(stripped) = trimmed.strip_prefix("> ") {
            return Line::from(vec![
                Span::styled("  > ", self.styles.blockquote),
                Span::styled(
                    stripped.to_string(),
                    Style::default().fg(Color::Rgb(192, 202, 245)),
                ),
            ]);
        }

        // Unordered lists
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let indent = line.len() - line.trim_start().len();
            let prefix = "  ".repeat(indent / 2);
            return Line::from(vec![
                Span::raw(prefix),
                Span::styled("• ", self.styles.list_marker),
                Span::styled(trimmed[2..].to_string(), Style::default()),
            ]);
        }

        // Horizontal rule
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            return Line::from(Span::styled(
                "────────────────────────────────────────────────────────────",
                Style::default().fg(Color::Rgb(86, 95, 137)),
            ));
        }

        // Regular paragraph with inline formatting
        self.render_inline_line(line)
    }

    fn render_inline_line(&self, line: &str) -> Line<'static> {
        let mut spans = Vec::new();
        let mut current = String::new();
        let mut chars = line.chars().peekable();
        let mut in_bold = false;
        let mut in_italic = false;
        let mut in_code = false;

        while let Some(c) = chars.next() {
            match c {
                '`' if !in_bold && !in_italic => {
                    if !current.is_empty() {
                        spans.push(Span::styled(
                            current.clone(),
                            if in_code {
                                self.styles.code
                            } else {
                                Style::default().fg(Color::Rgb(192, 202, 245))
                            },
                        ));
                        current.clear();
                    }
                    in_code = !in_code;
                }
                '*' if chars.peek() == Some(&'*') && !in_code => {
                    chars.next();
                    if !current.is_empty() {
                        let style = if in_bold {
                            self.styles.bold
                        } else if in_italic {
                            self.styles.italic
                        } else {
                            Style::default().fg(Color::Rgb(192, 202, 245))
                        };
                        spans.push(Span::styled(current.clone(), style));
                        current.clear();
                    }
                    in_bold = !in_bold;
                }
                '*' | '_' if !in_code && !in_bold => {
                    if !current.is_empty() {
                        let style = if in_italic {
                            self.styles.italic
                        } else {
                            Style::default().fg(Color::Rgb(192, 202, 245))
                        };
                        spans.push(Span::styled(current.clone(), style));
                        current.clear();
                    }
                    in_italic = !in_italic;
                }
                _ => {
                    current.push(c);
                }
            }
        }

        if !current.is_empty() {
            let style = if in_code {
                self.styles.code
            } else if in_bold {
                self.styles.bold
            } else if in_italic {
                self.styles.italic
            } else {
                Style::default().fg(Color::Rgb(192, 202, 245))
            };
            spans.push(Span::styled(current, style));
        }

        if spans.is_empty() {
            Line::from("")
        } else {
            Line::from(spans)
        }
    }
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}
