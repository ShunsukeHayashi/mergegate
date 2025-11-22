//! Streaming Markdown Renderer
//!
//! This module provides a streaming markdown renderer that can display
//! partial content as it arrives from an LLM response.
//!
//! Features:
//! - Incremental rendering as content streams in
//! - Full CommonMark support via pulldown-cmark
//! - Syntax highlighting for code blocks
//! - Table rendering with borders
//! - Link and image handling
//! - Blockquotes with visual indicators

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use pulldown_cmark::{Parser, Event, Tag, TagEnd, CodeBlockKind, Options};
use unicode_width::UnicodeWidthStr;

/// Color palette - Tokyo Night theme
mod colors {
    use ratatui::style::Color;

    pub const HEADING_1: Color = Color::Rgb(122, 162, 247);    // Blue
    pub const HEADING_2: Color = Color::Rgb(125, 207, 255);    // Cyan
    pub const HEADING_3: Color = Color::Rgb(187, 154, 247);    // Purple
    pub const CODE_BG: Color = Color::Rgb(36, 40, 59);         // Dark background
    pub const CODE_FG: Color = Color::Rgb(169, 177, 214);      // Light gray
    pub const LINK: Color = Color::Rgb(125, 207, 255);         // Cyan
    pub const EMPHASIS: Color = Color::Rgb(224, 175, 104);     // Yellow
    pub const STRONG: Color = Color::Rgb(247, 118, 142);       // Red/Pink
    pub const BLOCKQUOTE: Color = Color::Rgb(115, 218, 202);   // Teal
    pub const LIST_MARKER: Color = Color::Rgb(158, 206, 106);  // Green
    pub const TABLE_BORDER: Color = Color::Rgb(86, 95, 137);   // Dim
    pub const STRIKETHROUGH: Color = Color::Rgb(169, 177, 214); // Gray
    pub const HORIZONTAL_RULE: Color = Color::Rgb(86, 95, 137); // Dim
}

/// Inline style state for nested formatting
#[derive(Debug, Clone, Default)]
pub struct InlineStyle {
    /// Bold text
    pub bold: bool,
    /// Italic text
    pub italic: bool,
    /// Strikethrough text
    pub strikethrough: bool,
    /// Inline code
    pub code: bool,
    /// Link URL (if in a link)
    pub link_url: Option<String>,
}

impl InlineStyle {
    /// Build a style from current state
    pub fn to_style(&self) -> Style {
        let mut style = Style::default();

        if self.bold {
            style = style.add_modifier(Modifier::BOLD).fg(colors::STRONG);
        }
        if self.italic {
            style = style.add_modifier(Modifier::ITALIC).fg(colors::EMPHASIS);
        }
        if self.strikethrough {
            style = style.add_modifier(Modifier::CROSSED_OUT).fg(colors::STRIKETHROUGH);
        }
        if self.code {
            style = style.bg(colors::CODE_BG).fg(colors::CODE_FG);
        }
        if self.link_url.is_some() {
            style = style.fg(colors::LINK).add_modifier(Modifier::UNDERLINED);
        }

        style
    }
}

/// Table cell data
#[derive(Debug, Clone)]
pub struct TableCell {
    /// Cell content
    pub content: String,
    /// Cell alignment
    pub alignment: Alignment,
}

/// Text alignment
#[derive(Debug, Clone, Copy, Default)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
}

/// Table state for building tables
#[derive(Debug, Clone, Default)]
pub struct TableState {
    /// Header row
    pub headers: Vec<TableCell>,
    /// Data rows
    pub rows: Vec<Vec<TableCell>>,
    /// Current row being built
    pub current_row: Vec<TableCell>,
    /// Current cell content
    pub current_cell: String,
    /// Whether in header
    pub in_header: bool,
    /// Column alignments
    pub alignments: Vec<Alignment>,
}

impl TableState {
    /// Finish current cell
    pub fn finish_cell(&mut self) {
        let alignment = self.alignments
            .get(self.current_row.len())
            .copied()
            .unwrap_or_default();

        self.current_row.push(TableCell {
            content: std::mem::take(&mut self.current_cell),
            alignment,
        });
    }

    /// Finish current row
    pub fn finish_row(&mut self) {
        if !self.current_row.is_empty() {
            if self.in_header {
                self.headers = std::mem::take(&mut self.current_row);
                self.in_header = false;
            } else {
                self.rows.push(std::mem::take(&mut self.current_row));
            }
        }
    }

    /// Render table to lines
    pub fn render(&self) -> Vec<Line<'static>> {
        if self.headers.is_empty() {
            return vec![];
        }

        let mut lines = Vec::new();

        // Calculate column widths
        let mut widths: Vec<usize> = self.headers
            .iter()
            .map(|c| c.content.width().max(3))
            .collect();

        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    widths[i] = widths[i].max(cell.content.width());
                }
            }
        }

        // Render header
        let border_style = Style::default().fg(colors::TABLE_BORDER);
        let header_style = Style::default().fg(colors::HEADING_2).add_modifier(Modifier::BOLD);

        // Top border
        let top_border = format!("┌{}┐",
            widths.iter()
                .map(|w| "─".repeat(*w + 2))
                .collect::<Vec<_>>()
                .join("┬")
        );
        lines.push(Line::from(Span::styled(top_border, border_style)));

        // Header row
        let header_cells: Vec<String> = self.headers
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                let width = widths.get(i).copied().unwrap_or(0);
                format!(" {:^width$} ", cell.content, width = width)
            })
            .collect();

        let mut header_spans = vec![Span::styled("│", border_style)];
        for (i, cell) in header_cells.iter().enumerate() {
            header_spans.push(Span::styled(cell.clone(), header_style));
            if i < header_cells.len() - 1 {
                header_spans.push(Span::styled("│", border_style));
            }
        }
        header_spans.push(Span::styled("│", border_style));
        lines.push(Line::from(header_spans));

        // Header/body separator
        let separator = format!("├{}┤",
            widths.iter()
                .map(|w| "─".repeat(*w + 2))
                .collect::<Vec<_>>()
                .join("┼")
        );
        lines.push(Line::from(Span::styled(separator, border_style)));

        // Data rows
        for row in &self.rows {
            let row_cells: Vec<String> = row
                .iter()
                .enumerate()
                .map(|(i, cell)| {
                    let width = widths.get(i).copied().unwrap_or(0);
                    match cell.alignment {
                        Alignment::Left => format!(" {:<width$} ", cell.content, width = width),
                        Alignment::Center => format!(" {:^width$} ", cell.content, width = width),
                        Alignment::Right => format!(" {:>width$} ", cell.content, width = width),
                    }
                })
                .collect();

            let mut row_spans = vec![Span::styled("│", border_style)];
            for (i, cell) in row_cells.iter().enumerate() {
                row_spans.push(Span::raw(cell.clone()));
                if i < row_cells.len() - 1 {
                    row_spans.push(Span::styled("│", border_style));
                }
            }
            row_spans.push(Span::styled("│", border_style));
            lines.push(Line::from(row_spans));
        }

        // Bottom border
        let bottom_border = format!("└{}┘",
            widths.iter()
                .map(|w| "─".repeat(*w + 2))
                .collect::<Vec<_>>()
                .join("┴")
        );
        lines.push(Line::from(Span::styled(bottom_border, border_style)));

        lines
    }
}

/// Parsing context for incremental markdown rendering
#[derive(Debug, Clone, Default)]
pub struct ParseContext {
    /// Current inline style state
    pub inline_style: InlineStyle,
    /// Current list depth
    pub list_depth: usize,
    /// Current list item number (for ordered lists)
    pub list_number: Option<u64>,
    /// In code block
    pub in_code_block: bool,
    /// Code block language
    pub code_language: String,
    /// Code block content
    pub code_content: String,
    /// In blockquote
    pub in_blockquote: bool,
    /// Blockquote depth
    pub blockquote_depth: usize,
    /// Table state
    pub table: Option<TableState>,
    /// Current line spans being built
    pub current_spans: Vec<Span<'static>>,
    /// Completed lines
    pub lines: Vec<Line<'static>>,
}

impl ParseContext {
    /// Push text with current inline style
    pub fn push_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }

        let style = self.inline_style.to_style();
        self.current_spans.push(Span::styled(text.to_string(), style));
    }

    /// Finish current line
    pub fn finish_line(&mut self) {
        if !self.current_spans.is_empty() {
            // Add blockquote prefix if needed
            if self.in_blockquote {
                let prefix = "│ ".repeat(self.blockquote_depth);
                let mut spans = vec![
                    Span::styled(prefix, Style::default().fg(colors::BLOCKQUOTE))
                ];
                spans.extend(std::mem::take(&mut self.current_spans));
                self.lines.push(Line::from(spans));
            } else {
                self.lines.push(Line::from(std::mem::take(&mut self.current_spans)));
            }
        } else if self.in_blockquote {
            // Empty blockquote line
            let prefix = "│ ".repeat(self.blockquote_depth);
            self.lines.push(Line::from(Span::styled(
                prefix,
                Style::default().fg(colors::BLOCKQUOTE)
            )));
        }
    }

    /// Push a complete line
    pub fn push_line(&mut self, line: Line<'static>) {
        self.finish_line();
        self.lines.push(line);
    }

    /// Get list marker for current depth
    pub fn get_list_marker(&self) -> String {
        if let Some(num) = self.list_number {
            format!("{}{}. ", "  ".repeat(self.list_depth.saturating_sub(1)), num)
        } else {
            let markers = ['•', '◦', '▪', '▸'];
            let marker = markers[(self.list_depth.saturating_sub(1)) % markers.len()];
            format!("{}{} ", "  ".repeat(self.list_depth.saturating_sub(1)), marker)
        }
    }
}

/// State of the markdown stream
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum StreamState {
    /// No content yet
    #[default]
    Idle,
    /// Actively receiving content
    Streaming,
    /// All content received
    Complete,
}

/// Buffer for accumulating streamed content
#[derive(Debug, Clone, Default)]
pub struct StreamBuffer {
    /// Raw text content
    content: String,
    /// Whether the content has changed since last render
    dirty: bool,
}

impl StreamBuffer {
    /// Create a new empty buffer
    pub fn new() -> Self {
        Self::default()
    }

    /// Push new content to the buffer
    pub fn push_str(&mut self, s: &str) {
        self.content.push_str(s);
        self.dirty = true;
    }

    /// Get the current content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Check if buffer has changed
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark buffer as clean (rendered)
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.content.clear();
        self.dirty = true;
    }

    /// Get content length
    pub fn len(&self) -> usize {
        self.content.len()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}

/// Cursor position in the content
#[derive(Debug, Clone, Copy, Default)]
pub struct CursorPosition {
    /// Line number (0-indexed)
    pub line: usize,
    /// Column position (0-indexed)
    pub column: usize,
}

impl CursorPosition {
    /// Create new position
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// Scroll state for viewport management
#[derive(Debug, Clone)]
pub struct ScrollState {
    /// Current scroll offset (line number at top of viewport)
    pub offset: usize,
    /// Total number of content lines
    pub total_lines: usize,
    /// Viewport height in lines
    pub viewport_height: usize,
    /// Whether auto-scroll is enabled
    pub auto_scroll: bool,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self {
            offset: 0,
            total_lines: 0,
            viewport_height: 20,
            auto_scroll: true,
        }
    }
}

impl ScrollState {
    /// Create new scroll state
    pub fn new() -> Self {
        Self::default()
    }

    /// Set viewport height
    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport_height = height.max(1);
    }

    /// Get maximum scroll offset
    pub fn max_offset(&self) -> usize {
        self.total_lines.saturating_sub(self.viewport_height)
    }

    /// Check if we can scroll up
    pub fn can_scroll_up(&self) -> bool {
        self.offset > 0
    }

    /// Check if we can scroll down
    pub fn can_scroll_down(&self) -> bool {
        self.offset < self.max_offset()
    }

    /// Scroll up by n lines
    pub fn scroll_up(&mut self, n: usize) {
        self.offset = self.offset.saturating_sub(n);
        self.auto_scroll = false;
    }

    /// Scroll down by n lines
    pub fn scroll_down(&mut self, n: usize) {
        self.offset = (self.offset + n).min(self.max_offset());
        self.auto_scroll = false;
    }

    /// Scroll to top
    pub fn scroll_to_top(&mut self) {
        self.offset = 0;
        self.auto_scroll = false;
    }

    /// Scroll to bottom
    pub fn scroll_to_bottom(&mut self) {
        self.offset = self.max_offset();
        self.auto_scroll = true;
    }

    /// Page up
    pub fn page_up(&mut self) {
        self.scroll_up(self.viewport_height.saturating_sub(2));
    }

    /// Page down
    pub fn page_down(&mut self) {
        self.scroll_down(self.viewport_height.saturating_sub(2));
    }

    /// Update total lines and auto-scroll if enabled
    pub fn update_total_lines(&mut self, total: usize) {
        self.total_lines = total;
        if self.auto_scroll {
            self.offset = self.max_offset();
        }
    }

    /// Get scroll percentage (0.0 - 1.0)
    pub fn scroll_percentage(&self) -> f32 {
        if self.max_offset() == 0 {
            1.0
        } else {
            self.offset as f32 / self.max_offset() as f32
        }
    }
}

/// Streaming markdown renderer
#[derive(Debug, Clone)]
pub struct MarkdownStream {
    /// Current state
    state: StreamState,
    /// Content buffer
    buffer: StreamBuffer,
    /// Scroll state
    scroll: ScrollState,
    /// Cursor position
    cursor: CursorPosition,
}

impl Default for MarkdownStream {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownStream {
    /// Create a new markdown stream
    pub fn new() -> Self {
        Self {
            state: StreamState::Idle,
            buffer: StreamBuffer::new(),
            scroll: ScrollState::new(),
            cursor: CursorPosition::default(),
        }
    }

    /// Get current state
    pub fn state(&self) -> &StreamState {
        &self.state
    }

    /// Check if streaming is active
    pub fn is_streaming(&self) -> bool {
        self.state == StreamState::Streaming
    }

    /// Check if stream is complete
    pub fn is_complete(&self) -> bool {
        self.state == StreamState::Complete
    }

    /// Push new content chunk
    pub fn push_str(&mut self, s: &str) {
        if self.state == StreamState::Idle {
            self.state = StreamState::Streaming;
        }
        self.buffer.push_str(s);

        // Update cursor position to end of content
        let content = self.buffer.content();
        let line_count = content.lines().count();
        let last_line_len = content.lines().last().map(|l| l.len()).unwrap_or(0);
        self.cursor = CursorPosition::new(
            line_count.saturating_sub(1),
            last_line_len,
        );
    }

    /// Mark stream as complete
    pub fn complete(&mut self) {
        self.state = StreamState::Complete;
    }

    /// Reset the stream
    pub fn reset(&mut self) {
        self.state = StreamState::Idle;
        self.buffer.clear();
        self.scroll = ScrollState::new();
        self.cursor = CursorPosition::default();
    }

    /// Get current content
    pub fn content(&self) -> &str {
        self.buffer.content()
    }

    /// Get content length
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Get scroll state reference
    pub fn scroll(&self) -> &ScrollState {
        &self.scroll
    }

    /// Get mutable scroll state
    pub fn scroll_mut(&mut self) -> &mut ScrollState {
        &mut self.scroll
    }

    /// Get cursor position
    pub fn cursor(&self) -> CursorPosition {
        self.cursor
    }

    /// Set viewport height
    pub fn set_viewport_height(&mut self, height: usize) {
        self.scroll.set_viewport_height(height);
    }

    /// Set scroll offset (legacy API compatibility)
    pub fn set_scroll(&mut self, offset: usize) {
        self.scroll.offset = offset;
        self.scroll.auto_scroll = false;
    }

    /// Enable auto-scroll
    pub fn enable_auto_scroll(&mut self) {
        self.scroll.auto_scroll = true;
    }

    /// Get scroll offset (legacy API compatibility)
    pub fn scroll_offset(&self) -> usize {
        self.scroll.offset
    }

    /// Scroll up by n lines
    pub fn scroll_up(&mut self, n: usize) {
        self.scroll.scroll_up(n);
    }

    /// Scroll down by n lines
    pub fn scroll_down(&mut self, n: usize) {
        self.scroll.scroll_down(n);
    }

    /// Scroll to top
    pub fn scroll_to_top(&mut self) {
        self.scroll.scroll_to_top();
    }

    /// Scroll to bottom
    pub fn scroll_to_bottom(&mut self) {
        self.scroll.scroll_to_bottom();
    }

    /// Page up
    pub fn page_up(&mut self) {
        self.scroll.page_up();
    }

    /// Page down
    pub fn page_down(&mut self) {
        self.scroll.page_down();
    }

    /// Get total line count
    pub fn line_count(&self) -> usize {
        self.scroll.total_lines
    }

    /// Render content to Ratatui lines using pulldown-cmark
    ///
    /// This implementation provides full CommonMark support with:
    /// - Syntax highlighting for code blocks
    /// - Table rendering with borders
    /// - Nested lists with proper markers
    /// - Blockquotes with visual indicators
    /// - Inline formatting (bold, italic, code, links)
    pub fn render(&mut self) -> Vec<Line<'static>> {
        let content = self.buffer.content().to_string();
        self.buffer.mark_clean();

        if content.is_empty() {
            return vec![Line::from(Span::styled(
                "Waiting for response...",
                Style::default().fg(Color::DarkGray),
            ))];
        }

        // Configure pulldown-cmark options
        let options = Options::ENABLE_TABLES
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_TASKLISTS;

        let parser = Parser::new_ext(&content, options);
        let mut ctx = ParseContext::default();

        for event in parser {
            match event {
                // Block-level events
                Event::Start(tag) => {
                    match tag {
                        Tag::Heading { level, .. } => {
                            ctx.finish_line();
                            let style = match level {
                                pulldown_cmark::HeadingLevel::H1 => {
                                    Style::default().fg(colors::HEADING_1).add_modifier(Modifier::BOLD)
                                }
                                pulldown_cmark::HeadingLevel::H2 => {
                                    Style::default().fg(colors::HEADING_2).add_modifier(Modifier::BOLD)
                                }
                                pulldown_cmark::HeadingLevel::H3 => {
                                    Style::default().fg(colors::HEADING_3).add_modifier(Modifier::BOLD)
                                }
                                _ => Style::default().add_modifier(Modifier::BOLD),
                            };
                            let prefix = match level {
                                pulldown_cmark::HeadingLevel::H1 => "# ",
                                pulldown_cmark::HeadingLevel::H2 => "## ",
                                pulldown_cmark::HeadingLevel::H3 => "### ",
                                pulldown_cmark::HeadingLevel::H4 => "#### ",
                                pulldown_cmark::HeadingLevel::H5 => "##### ",
                                pulldown_cmark::HeadingLevel::H6 => "###### ",
                            };
                            ctx.current_spans.push(Span::styled(prefix.to_string(), style));
                        }
                        Tag::Paragraph => {
                            ctx.finish_line();
                        }
                        Tag::BlockQuote(_) => {
                            ctx.finish_line();
                            ctx.in_blockquote = true;
                            ctx.blockquote_depth += 1;
                        }
                        Tag::CodeBlock(kind) => {
                            ctx.finish_line();
                            ctx.in_code_block = true;
                            ctx.code_language = match kind {
                                CodeBlockKind::Fenced(lang) => lang.to_string(),
                                CodeBlockKind::Indented => String::new(),
                            };
                            ctx.code_content.clear();

                            // Code block header
                            let header = if ctx.code_language.is_empty() {
                                "```".to_string()
                            } else {
                                format!("```{}", ctx.code_language)
                            };
                            ctx.lines.push(Line::from(Span::styled(
                                header,
                                Style::default().fg(colors::TABLE_BORDER),
                            )));
                        }
                        Tag::List(start) => {
                            ctx.finish_line();
                            ctx.list_depth += 1;
                            ctx.list_number = start;
                        }
                        Tag::Item => {
                            ctx.finish_line();
                            let marker = ctx.get_list_marker();
                            ctx.current_spans.push(Span::styled(
                                marker,
                                Style::default().fg(colors::LIST_MARKER),
                            ));
                            if let Some(ref mut num) = ctx.list_number {
                                *num += 1;
                            }
                        }
                        Tag::Table(alignments) => {
                            ctx.finish_line();
                            let mut table = TableState::default();
                            table.alignments = alignments
                                .into_iter()
                                .map(|a| match a {
                                    pulldown_cmark::Alignment::Left => Alignment::Left,
                                    pulldown_cmark::Alignment::Center => Alignment::Center,
                                    pulldown_cmark::Alignment::Right => Alignment::Right,
                                    pulldown_cmark::Alignment::None => Alignment::Left,
                                })
                                .collect();
                            ctx.table = Some(table);
                        }
                        Tag::TableHead => {
                            if let Some(ref mut table) = ctx.table {
                                table.in_header = true;
                            }
                        }
                        Tag::TableRow => {}
                        Tag::TableCell => {}
                        Tag::Emphasis => {
                            ctx.inline_style.italic = true;
                        }
                        Tag::Strong => {
                            ctx.inline_style.bold = true;
                        }
                        Tag::Strikethrough => {
                            ctx.inline_style.strikethrough = true;
                        }
                        Tag::Link { dest_url, .. } => {
                            ctx.inline_style.link_url = Some(dest_url.to_string());
                        }
                        Tag::Image { dest_url, .. } => {
                            ctx.current_spans.push(Span::styled(
                                format!("[image: {}]", dest_url),
                                Style::default().fg(colors::LINK).add_modifier(Modifier::DIM),
                            ));
                        }
                        _ => {}
                    }
                }
                Event::End(tag_end) => {
                    match tag_end {
                        TagEnd::Heading(_) => {
                            ctx.finish_line();
                            ctx.lines.push(Line::from("")); // Add spacing
                        }
                        TagEnd::Paragraph => {
                            ctx.finish_line();
                            ctx.lines.push(Line::from("")); // Add spacing
                        }
                        TagEnd::BlockQuote(_) => {
                            ctx.finish_line();
                            ctx.blockquote_depth = ctx.blockquote_depth.saturating_sub(1);
                            ctx.in_blockquote = ctx.blockquote_depth > 0;
                        }
                        TagEnd::CodeBlock => {
                            // Render code block content with syntax highlighting
                            for line in ctx.code_content.lines() {
                                let highlighted = highlight_code_line(line, &ctx.code_language);
                                ctx.lines.push(highlighted);
                            }
                            ctx.lines.push(Line::from(Span::styled(
                                "```",
                                Style::default().fg(colors::TABLE_BORDER),
                            )));
                            ctx.in_code_block = false;
                            ctx.code_content.clear();
                        }
                        TagEnd::List(_) => {
                            ctx.finish_line();
                            ctx.list_depth = ctx.list_depth.saturating_sub(1);
                            if ctx.list_depth == 0 {
                                ctx.list_number = None;
                            }
                        }
                        TagEnd::Item => {
                            ctx.finish_line();
                        }
                        TagEnd::Table => {
                            if let Some(table) = ctx.table.take() {
                                for line in table.render() {
                                    ctx.lines.push(line);
                                }
                            }
                            ctx.lines.push(Line::from("")); // Add spacing
                        }
                        TagEnd::TableHead => {
                            if let Some(ref mut table) = ctx.table {
                                table.finish_row();
                            }
                        }
                        TagEnd::TableRow => {
                            if let Some(ref mut table) = ctx.table {
                                table.finish_row();
                            }
                        }
                        TagEnd::TableCell => {
                            if let Some(ref mut table) = ctx.table {
                                table.finish_cell();
                            }
                        }
                        TagEnd::Emphasis => {
                            ctx.inline_style.italic = false;
                        }
                        TagEnd::Strong => {
                            ctx.inline_style.bold = false;
                        }
                        TagEnd::Strikethrough => {
                            ctx.inline_style.strikethrough = false;
                        }
                        TagEnd::Link => {
                            if let Some(url) = ctx.inline_style.link_url.take() {
                                ctx.current_spans.push(Span::styled(
                                    format!(" ({})", url),
                                    Style::default().fg(colors::TABLE_BORDER).add_modifier(Modifier::DIM),
                                ));
                            }
                        }
                        _ => {}
                    }
                }
                Event::Text(text) => {
                    if ctx.in_code_block {
                        ctx.code_content.push_str(&text);
                    } else if let Some(ref mut table) = ctx.table {
                        table.current_cell.push_str(&text);
                    } else {
                        ctx.push_text(&text);
                    }
                }
                Event::Code(code) => {
                    ctx.current_spans.push(Span::styled(
                        format!(" {} ", code),
                        Style::default().bg(colors::CODE_BG).fg(colors::CODE_FG),
                    ));
                }
                Event::SoftBreak => {
                    ctx.push_text(" ");
                }
                Event::HardBreak => {
                    ctx.finish_line();
                }
                Event::Rule => {
                    ctx.finish_line();
                    ctx.lines.push(Line::from(Span::styled(
                        "─".repeat(40),
                        Style::default().fg(colors::HORIZONTAL_RULE),
                    )));
                    ctx.lines.push(Line::from("")); // Add spacing
                }
                Event::TaskListMarker(checked) => {
                    let marker = if checked { "☑ " } else { "☐ " };
                    ctx.current_spans.push(Span::styled(
                        marker.to_string(),
                        Style::default().fg(colors::LIST_MARKER),
                    ));
                }
                _ => {}
            }
        }

        // Finish any remaining content
        ctx.finish_line();

        let mut lines = ctx.lines;

        // Add cursor if streaming
        if self.state == StreamState::Streaming {
            if let Some(last) = lines.last_mut() {
                last.spans.push(Span::styled(
                    "▌",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::SLOW_BLINK),
                ));
            } else {
                lines.push(Line::from(Span::styled(
                    "▌",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::SLOW_BLINK),
                )));
            }
        }

        // Update scroll state with total line count
        self.scroll.update_total_lines(lines.len());

        lines
    }
}

/// Highlight a single line of code based on language
fn highlight_code_line(line: &str, _language: &str) -> Line<'static> {
    // Basic syntax highlighting with Dracula-inspired colors
    let mut spans = Vec::new();
    let mut current_word = String::new();
    let mut in_string = false;
    let mut string_char = '"';

    let keywords = [
        "fn", "let", "mut", "const", "pub", "mod", "use", "struct", "enum", "impl",
        "trait", "where", "if", "else", "match", "for", "while", "loop", "return",
        "break", "continue", "async", "await", "self", "Self", "true", "false",
        "Some", "None", "Ok", "Err", "type", "static", "extern", "crate", "super",
        // TypeScript/JavaScript
        "function", "class", "interface", "extends", "import", "export", "default",
        "new", "this", "try", "catch", "throw", "finally", "typeof", "instanceof",
        // Python
        "def", "class", "import", "from", "as", "pass", "raise", "with", "yield",
        "lambda", "global", "nonlocal", "assert", "del", "in", "is", "not", "and", "or",
    ];

    let types = [
        "String", "Vec", "Option", "Result", "Box", "Rc", "Arc", "HashMap", "HashSet",
        "bool", "char", "str", "i8", "i16", "i32", "i64", "i128", "isize",
        "u8", "u16", "u32", "u64", "u128", "usize", "f32", "f64",
        // TypeScript
        "number", "string", "boolean", "void", "null", "undefined", "any", "never",
        "Array", "Promise", "Map", "Set", "Object",
    ];

    for ch in line.chars() {
        if in_string {
            current_word.push(ch);
            if ch == string_char {
                spans.push(Span::styled(
                    std::mem::take(&mut current_word),
                    Style::default().fg(Color::Rgb(158, 206, 106)), // Green
                ));
                in_string = false;
            }
        } else if ch == '"' || ch == '\'' {
            // Flush current word
            if !current_word.is_empty() {
                let style = get_word_style(&current_word, &keywords, &types);
                spans.push(Span::styled(std::mem::take(&mut current_word), style));
            }
            in_string = true;
            string_char = ch;
            current_word.push(ch);
        } else if ch.is_alphanumeric() || ch == '_' {
            current_word.push(ch);
        } else {
            // Flush current word
            if !current_word.is_empty() {
                let style = get_word_style(&current_word, &keywords, &types);
                spans.push(Span::styled(std::mem::take(&mut current_word), style));
            }
            // Handle operators and punctuation
            let style = match ch {
                '(' | ')' | '[' | ']' | '{' | '}' => {
                    Style::default().fg(Color::Rgb(189, 147, 249)) // Purple
                }
                '+' | '-' | '*' | '/' | '=' | '<' | '>' | '!' | '&' | '|' | '^' | '%' => {
                    Style::default().fg(Color::Rgb(255, 121, 198)) // Pink
                }
                ':' | ';' | ',' | '.' => {
                    Style::default().fg(colors::TABLE_BORDER)
                }
                '#' => {
                    Style::default().fg(Color::Rgb(255, 184, 108)) // Orange (attributes)
                }
                _ => Style::default().fg(colors::CODE_FG),
            };
            spans.push(Span::styled(ch.to_string(), style));
        }
    }

    // Flush remaining word
    if !current_word.is_empty() {
        let style = if in_string {
            Style::default().fg(Color::Rgb(158, 206, 106)) // Green
        } else {
            get_word_style(&current_word, &keywords, &types)
        };
        spans.push(Span::styled(current_word, style));
    }

    Line::from(spans)
}

/// Get style for a word based on its type
fn get_word_style(word: &str, keywords: &[&str], types: &[&str]) -> Style {
    if keywords.contains(&word) {
        Style::default().fg(Color::Rgb(255, 121, 198)) // Pink - keywords
    } else if types.contains(&word) {
        Style::default().fg(Color::Rgb(139, 233, 253)) // Cyan - types
    } else if word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
        Style::default().fg(Color::Rgb(139, 233, 253)) // Cyan - types (PascalCase)
    } else if word.chars().all(|c| c.is_ascii_digit() || c == '.') {
        Style::default().fg(Color::Rgb(189, 147, 249)) // Purple - numbers
    } else {
        Style::default().fg(colors::CODE_FG)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_state_transitions() {
        let mut stream = MarkdownStream::new();
        assert_eq!(stream.state(), &StreamState::Idle);

        stream.push_str("Hello");
        assert_eq!(stream.state(), &StreamState::Streaming);

        stream.complete();
        assert_eq!(stream.state(), &StreamState::Complete);
    }

    #[test]
    fn test_buffer_operations() {
        let mut buffer = StreamBuffer::new();
        assert!(buffer.is_empty());

        buffer.push_str("Hello");
        assert!(!buffer.is_empty());
        assert_eq!(buffer.len(), 5);
        assert!(buffer.is_dirty());

        buffer.mark_clean();
        assert!(!buffer.is_dirty());

        buffer.push_str(" World");
        assert!(buffer.is_dirty());
        assert_eq!(buffer.content(), "Hello World");
    }

    #[test]
    fn test_render_basic() {
        let mut stream = MarkdownStream::new();
        stream.push_str("# Hello\n\nThis is text.");

        let lines = stream.render();
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_render_code_block() {
        let mut stream = MarkdownStream::new();
        stream.push_str("```rust\nfn main() {}\n```");

        let lines = stream.render();
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_reset() {
        let mut stream = MarkdownStream::new();
        stream.push_str("Content");
        stream.complete();

        stream.reset();
        assert_eq!(stream.state(), &StreamState::Idle);
        assert!(stream.is_empty());
    }

    #[test]
    fn test_scroll_state() {
        let mut scroll = ScrollState::new();
        scroll.total_lines = 100;
        scroll.viewport_height = 20;

        assert_eq!(scroll.max_offset(), 80);
        assert!(scroll.can_scroll_down());
        assert!(!scroll.can_scroll_up());

        scroll.scroll_down(10);
        assert_eq!(scroll.offset, 10);
        assert!(scroll.can_scroll_up());
        assert!(!scroll.auto_scroll);

        scroll.scroll_up(5);
        assert_eq!(scroll.offset, 5);

        scroll.scroll_to_bottom();
        assert_eq!(scroll.offset, 80);
        assert!(scroll.auto_scroll);

        scroll.scroll_to_top();
        assert_eq!(scroll.offset, 0);
        assert!(!scroll.auto_scroll);
    }

    #[test]
    fn test_page_navigation() {
        let mut scroll = ScrollState::new();
        scroll.total_lines = 100;
        scroll.viewport_height = 20;

        scroll.page_down();
        assert_eq!(scroll.offset, 18); // viewport_height - 2

        scroll.page_up();
        assert_eq!(scroll.offset, 0);
    }

    #[test]
    fn test_scroll_percentage() {
        let mut scroll = ScrollState::new();
        scroll.total_lines = 100;
        scroll.viewport_height = 20;

        assert_eq!(scroll.scroll_percentage(), 0.0);

        scroll.scroll_to_bottom();
        assert_eq!(scroll.scroll_percentage(), 1.0);

        scroll.offset = 40;
        assert_eq!(scroll.scroll_percentage(), 0.5);
    }

    #[test]
    fn test_cursor_position_tracking() {
        let mut stream = MarkdownStream::new();

        stream.push_str("Line 1");
        assert_eq!(stream.cursor().line, 0);
        assert_eq!(stream.cursor().column, 6);

        stream.push_str("\nLine 2\nLine 3");
        assert_eq!(stream.cursor().line, 2);
        assert_eq!(stream.cursor().column, 6);
    }

    #[test]
    fn test_stream_scroll_methods() {
        let mut stream = MarkdownStream::new();
        // Use blank lines to create separate paragraphs in markdown
        stream.push_str("Line 1\n\nLine 2\n\nLine 3\n\nLine 4\n\nLine 5");
        stream.render(); // Updates total_lines
        stream.set_viewport_height(3);

        // Markdown renders paragraphs with blank lines between them
        let line_count = stream.line_count();
        assert!(line_count >= 5, "Expected at least 5 lines, got {}", line_count);
        assert!(stream.scroll().can_scroll_down());

        stream.scroll_down(2);
        assert_eq!(stream.scroll_offset(), 2);

        stream.scroll_up(1);
        assert_eq!(stream.scroll_offset(), 1);

        stream.scroll_to_top();
        assert_eq!(stream.scroll_offset(), 0);

        stream.scroll_to_bottom();
        // Scroll to bottom = total_lines - viewport_height
        let expected_offset = line_count.saturating_sub(3);
        assert_eq!(stream.scroll_offset(), expected_offset);
    }

    #[test]
    fn test_auto_scroll_on_new_content() {
        let mut stream = MarkdownStream::new();
        stream.set_viewport_height(3);

        // Add content with proper markdown paragraph separation
        stream.push_str("Line 1\n\nLine 2\n\nLine 3\n\nLine 4\n\nLine 5");
        let lines = stream.render();

        // Markdown renders paragraphs (at least 5 lines with blank lines)
        let initial_count = lines.len();
        assert!(initial_count >= 5, "Expected at least 5 lines, got {}", initial_count);

        // Add more content
        stream.push_str("\n\nLine 6");
        let lines = stream.render();

        // Should have more lines now
        assert!(lines.len() > initial_count, "Expected more lines after adding content");
    }
}
