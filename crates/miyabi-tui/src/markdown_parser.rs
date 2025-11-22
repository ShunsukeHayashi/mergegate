//! Incremental Markdown Parser
//!
//! This module provides incremental parsing of markdown content using pulldown-cmark,
//! with caching for efficient re-rendering during streaming.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd, CodeBlockKind};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Incremental markdown parser with caching
#[derive(Debug, Clone)]
pub struct MarkdownParser {
    /// Cached parsed lines
    cache: Vec<Line<'static>>,
    /// Last parsed content length
    last_len: usize,
    /// Parser options
    options: Options,
}

impl Default for MarkdownParser {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownParser {
    /// Create a new parser
    pub fn new() -> Self {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_FOOTNOTES);
        options.insert(Options::ENABLE_TASKLISTS);

        Self {
            cache: Vec::new(),
            last_len: 0,
            options,
        }
    }

    /// Parse markdown content
    ///
    /// Uses caching to only reparse when content has changed
    pub fn parse(&mut self, content: &str) -> Vec<Line<'static>> {
        // If content length is same, use cache
        if content.len() == self.last_len && !self.cache.is_empty() {
            return self.cache.clone();
        }

        let lines = self.parse_content(content);
        self.cache = lines.clone();
        self.last_len = content.len();

        lines
    }

    /// Force a full reparse
    pub fn invalidate(&mut self) {
        self.cache.clear();
        self.last_len = 0;
    }

    /// Parse content into styled lines
    fn parse_content(&self, content: &str) -> Vec<Line<'static>> {
        let parser = Parser::new_ext(content, self.options);
        let mut renderer = EventRenderer::new();

        for event in parser {
            renderer.handle_event(event);
        }

        renderer.finish()
    }

    /// Get cached line count
    pub fn line_count(&self) -> usize {
        self.cache.len()
    }
}

/// Renderer that converts pulldown-cmark events to Ratatui lines
struct EventRenderer {
    lines: Vec<Line<'static>>,
    current_spans: Vec<Span<'static>>,
    style_stack: Vec<Style>,
    in_code_block: bool,
    code_block_lang: String,
    list_depth: usize,
    list_index: Option<u64>,
}

impl EventRenderer {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            current_spans: Vec::new(),
            style_stack: vec![Style::default()],
            in_code_block: false,
            code_block_lang: String::new(),
            list_depth: 0,
            list_index: None,
        }
    }

    fn current_style(&self) -> Style {
        self.style_stack.last().copied().unwrap_or_default()
    }

    fn push_style(&mut self, style: Style) {
        let combined = self.current_style().patch(style);
        self.style_stack.push(combined);
    }

    fn pop_style(&mut self) {
        if self.style_stack.len() > 1 {
            self.style_stack.pop();
        }
    }

    fn flush_line(&mut self) {
        if !self.current_spans.is_empty() {
            let spans = std::mem::take(&mut self.current_spans);
            self.lines.push(Line::from(spans));
        } else {
            self.lines.push(Line::default());
        }
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.handle_start_tag(tag),
            Event::End(tag) => self.handle_end_tag(tag),
            Event::Text(text) => self.handle_text(&text),
            Event::Code(code) => self.handle_inline_code(&code),
            Event::SoftBreak => self.current_spans.push(Span::raw(" ")),
            Event::HardBreak => self.flush_line(),
            Event::Rule => {
                self.flush_line();
                self.lines.push(Line::from(Span::styled(
                    "─".repeat(40),
                    Style::default().fg(Color::DarkGray),
                )));
            }
            Event::TaskListMarker(checked) => {
                let marker = if checked { "[x] " } else { "[ ] " };
                self.current_spans.push(Span::styled(
                    marker.to_string(),
                    Style::default().fg(Color::Yellow),
                ));
            }
            _ => {}
        }
    }

    fn handle_start_tag(&mut self, tag: Tag) {
        match tag {
            Tag::Heading { level, .. } => {
                let style = match level {
                    pulldown_cmark::HeadingLevel::H1 => Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                    pulldown_cmark::HeadingLevel::H2 => Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                    pulldown_cmark::HeadingLevel::H3 => Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                    _ => Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                };
                self.push_style(style);
            }
            Tag::Paragraph => {}
            Tag::CodeBlock(kind) => {
                self.in_code_block = true;
                self.code_block_lang = match kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                // Add code block header
                let header = if self.code_block_lang.is_empty() {
                    "```".to_string()
                } else {
                    format!("```{}", self.code_block_lang)
                };
                self.lines.push(Line::from(Span::styled(
                    header,
                    Style::default().fg(Color::DarkGray),
                )));
            }
            Tag::List(start) => {
                self.list_depth += 1;
                self.list_index = start;
            }
            Tag::Item => {
                let indent = "  ".repeat(self.list_depth.saturating_sub(1));
                let marker = if let Some(idx) = self.list_index {
                    self.list_index = Some(idx + 1);
                    format!("{}{}. ", indent, idx)
                } else {
                    format!("{}- ", indent)
                };
                self.current_spans.push(Span::styled(
                    marker,
                    Style::default().fg(Color::Yellow),
                ));
            }
            Tag::Emphasis => {
                self.push_style(Style::default().add_modifier(Modifier::ITALIC));
            }
            Tag::Strong => {
                self.push_style(Style::default().add_modifier(Modifier::BOLD));
            }
            Tag::Strikethrough => {
                self.push_style(Style::default().add_modifier(Modifier::CROSSED_OUT));
            }
            Tag::BlockQuote(_) => {
                self.current_spans.push(Span::styled(
                    "│ ",
                    Style::default().fg(Color::DarkGray),
                ));
                self.push_style(Style::default().fg(Color::Gray));
            }
            Tag::Link { .. } => {
                self.push_style(Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED));
            }
            _ => {}
        }
    }

    fn handle_end_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Heading(_) => {
                self.pop_style();
                self.flush_line();
            }
            TagEnd::Paragraph => {
                self.flush_line();
            }
            TagEnd::CodeBlock => {
                self.in_code_block = false;
                self.lines.push(Line::from(Span::styled(
                    "```",
                    Style::default().fg(Color::DarkGray),
                )));
            }
            TagEnd::List(_) => {
                self.list_depth = self.list_depth.saturating_sub(1);
                if self.list_depth == 0 {
                    self.list_index = None;
                }
            }
            TagEnd::Item => {
                self.flush_line();
            }
            TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link => {
                self.pop_style();
            }
            TagEnd::BlockQuote(_) => {
                self.pop_style();
                self.flush_line();
            }
            _ => {}
        }
    }

    fn handle_text(&mut self, text: &str) {
        if self.in_code_block {
            // Code blocks get special styling
            for line in text.lines() {
                self.lines.push(Line::from(Span::styled(
                    line.to_string(),
                    Style::default().fg(Color::Green),
                )));
            }
        } else {
            let style = self.current_style();
            self.current_spans.push(Span::styled(text.to_string(), style));
        }
    }

    fn handle_inline_code(&mut self, code: &str) {
        self.current_spans.push(Span::styled(
            format!("`{}`", code),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::DIM),
        ));
    }

    fn finish(mut self) -> Vec<Line<'static>> {
        // Flush any remaining spans
        if !self.current_spans.is_empty() {
            self.flush_line();
        }

        // Ensure we have at least one line
        if self.lines.is_empty() {
            self.lines.push(Line::default());
        }

        self.lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_creation() {
        let parser = MarkdownParser::new();
        assert_eq!(parser.line_count(), 0);
    }

    #[test]
    fn test_parse_heading() {
        let mut parser = MarkdownParser::new();
        let lines = parser.parse("# Hello World");

        assert!(!lines.is_empty());
    }

    #[test]
    fn test_parse_code_block() {
        let mut parser = MarkdownParser::new();
        let content = "```rust\nfn main() {}\n```";
        let lines = parser.parse(content);

        // Should have: ```rust, code line, ```
        assert!(lines.len() >= 3);
    }

    #[test]
    fn test_parse_list() {
        let mut parser = MarkdownParser::new();
        let content = "- Item 1\n- Item 2\n- Item 3";
        let lines = parser.parse(content);

        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_parse_nested_list() {
        let mut parser = MarkdownParser::new();
        let content = "- Item 1\n  - Nested\n- Item 2";
        let lines = parser.parse(content);

        assert!(!lines.is_empty());
    }

    #[test]
    fn test_parse_emphasis() {
        let mut parser = MarkdownParser::new();
        let content = "This is *italic* and **bold** text";
        let lines = parser.parse(content);

        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_parse_inline_code() {
        let mut parser = MarkdownParser::new();
        let content = "Use `println!` to print";
        let lines = parser.parse(content);

        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_caching() {
        let mut parser = MarkdownParser::new();
        let content = "# Test\n\nParagraph";

        // First parse
        let lines1 = parser.parse(content);

        // Second parse should use cache
        let lines2 = parser.parse(content);

        assert_eq!(lines1.len(), lines2.len());
    }

    #[test]
    fn test_invalidate_cache() {
        let mut parser = MarkdownParser::new();
        let _ = parser.parse("# Test");

        parser.invalidate();
        assert_eq!(parser.line_count(), 0);
    }

    #[test]
    fn test_parse_blockquote() {
        let mut parser = MarkdownParser::new();
        let content = "> This is a quote";
        let lines = parser.parse(content);

        assert!(!lines.is_empty());
    }

    #[test]
    fn test_parse_horizontal_rule() {
        let mut parser = MarkdownParser::new();
        let content = "Before\n\n---\n\nAfter";
        let lines = parser.parse(content);

        assert!(lines.len() >= 3);
    }

    #[test]
    fn test_empty_content() {
        let mut parser = MarkdownParser::new();
        let lines = parser.parse("");

        assert!(!lines.is_empty()); // Should have at least one empty line
    }

    #[test]
    fn test_incomplete_code_block() {
        let mut parser = MarkdownParser::new();
        // Incomplete code block (no closing ```)
        let content = "```rust\nfn main()";
        let lines = parser.parse(content);

        // Should still parse without panicking
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_task_list() {
        let mut parser = MarkdownParser::new();
        let content = "- [ ] Unchecked\n- [x] Checked";
        let lines = parser.parse(content);

        assert_eq!(lines.len(), 2);
    }
}
