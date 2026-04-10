//! Text Wrapping Module
//!
//! Provides intelligent text wrapping for terminal display,
//! following the OpenAI Codex TUI pattern with textwrap integration.

use ratatui::text::{Line, Span};
use std::borrow::Cow;
use textwrap::{wrap_algorithms::Penalties, Options, WordSeparator, WordSplitter, WrapAlgorithm};
use unicode_width::UnicodeWidthStr;

/// Wrapping options configuration
pub struct WrapOptions<'a> {
    /// Target width for wrapping
    pub width: usize,
    /// Prefix for the first line
    pub initial_indent: Cow<'a, str>,
    /// Prefix for subsequent lines
    pub subsequent_indent: Cow<'a, str>,
    /// Whether to break words that exceed width
    pub break_words: bool,
}

impl<'a> Default for WrapOptions<'a> {
    fn default() -> Self {
        Self {
            width: 80,
            initial_indent: Cow::Borrowed(""),
            subsequent_indent: Cow::Borrowed(""),
            break_words: false,
        }
    }
}

impl<'a> WrapOptions<'a> {
    /// Create options with specified width
    pub fn new(width: usize) -> Self {
        Self {
            width,
            ..Default::default()
        }
    }

    /// Set initial indent
    pub fn initial_indent(mut self, indent: impl Into<Cow<'a, str>>) -> Self {
        self.initial_indent = indent.into();
        self
    }

    /// Set subsequent indent
    pub fn subsequent_indent(mut self, indent: impl Into<Cow<'a, str>>) -> Self {
        self.subsequent_indent = indent.into();
        self
    }

    /// Set word breaking behavior
    pub fn break_words(mut self, break_words: bool) -> Self {
        self.break_words = break_words;
        self
    }

    /// Convert to textwrap Options
    fn to_textwrap_options(&self) -> Options<'_> {
        Options::new(self.width)
            .initial_indent(&self.initial_indent)
            .subsequent_indent(&self.subsequent_indent)
            .break_words(self.break_words)
            .word_separator(WordSeparator::UnicodeBreakProperties)
            .word_splitter(WordSplitter::NoHyphenation)
            .wrap_algorithm(WrapAlgorithm::OptimalFit(Penalties::default()))
    }
}

/// Wrap a single line of text into multiple lines
pub fn word_wrap_line(line: &Line<'_>, width: usize) -> Vec<Line<'static>> {
    let options = WrapOptions::new(width);
    word_wrap_line_with_options(line, &options)
}

/// Wrap a single line with custom options
pub fn word_wrap_line_with_options(
    line: &Line<'_>,
    options: &WrapOptions<'_>,
) -> Vec<Line<'static>> {
    if line.spans.is_empty() {
        return vec![Line::from("")];
    }

    // Track spans with their byte ranges in the flattened string
    let mut flat_text = String::new();
    let mut span_ranges: Vec<(usize, usize, ratatui::style::Style)> = Vec::new();

    for span in &line.spans {
        let start = flat_text.len();
        flat_text.push_str(&span.content);
        let end = flat_text.len();
        span_ranges.push((start, end, span.style));
    }

    if flat_text.is_empty() {
        return vec![Line::from("")];
    }

    // Use textwrap to compute line breaks
    let textwrap_opts = options.to_textwrap_options();
    let wrapped = textwrap::wrap(&flat_text, textwrap_opts);

    let mut result = Vec::new();
    let mut current_pos = 0;

    for wrapped_line in wrapped {
        let line_str = wrapped_line.trim_start();
        if line_str.is_empty() {
            result.push(Line::from(""));
            continue;
        }

        // Find position in original text (skip leading whitespace)
        let skip_ws = wrapped_line.len() - line_str.len();
        let line_start = current_pos + skip_ws;
        let line_end = line_start + line_str.len();

        // Build spans for this wrapped line
        let mut line_spans = Vec::new();

        for &(span_start, span_end, style) in &span_ranges {
            // Check if this span overlaps with current line
            if span_end <= line_start || span_start >= line_end {
                continue;
            }

            // Calculate overlap
            let overlap_start = span_start.max(line_start);
            let overlap_end = span_end.min(line_end);

            if overlap_start < overlap_end {
                let text = &flat_text[overlap_start..overlap_end];
                line_spans.push(Span::styled(text.to_string(), style));
            }
        }

        if line_spans.is_empty() {
            result.push(Line::from(line_str.to_string()));
        } else {
            result.push(Line::from(line_spans));
        }

        current_pos = line_end;
    }

    if result.is_empty() {
        vec![Line::from("")]
    } else {
        result
    }
}

/// Wrap multiple lines of text
pub fn word_wrap_lines<'a, I>(lines: I, width: usize) -> Vec<Line<'static>>
where
    I: IntoIterator<Item = Line<'a>>,
{
    let mut result = Vec::new();
    for line in lines {
        result.extend(word_wrap_line(&line, width));
    }
    result
}

/// Wrap plain text string into styled lines
pub fn wrap_text(text: &str, width: usize) -> Vec<Line<'static>> {
    let options = WrapOptions::new(width);
    let textwrap_opts = options.to_textwrap_options();
    let wrapped = textwrap::wrap(text, textwrap_opts);

    wrapped
        .into_iter()
        .map(|s| Line::from(s.into_owned()))
        .collect()
}

/// Calculate display width of text (Unicode-aware)
pub fn display_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

/// Truncate text to fit within width, adding ellipsis if needed
pub fn truncate_with_ellipsis(text: &str, width: usize) -> String {
    let text_width = display_width(text);
    if text_width <= width {
        return text.to_string();
    }

    if width < 3 {
        return "...".chars().take(width).collect();
    }

    let target_width = width - 3;
    let mut result = String::new();
    let mut current_width = 0;

    for ch in text.chars() {
        let ch_width = UnicodeWidthStr::width(ch.to_string().as_str());
        if current_width + ch_width > target_width {
            break;
        }
        result.push(ch);
        current_width += ch_width;
    }

    result.push_str("...");
    result
}

/// Pad text to specified width
pub fn pad_to_width(text: &str, width: usize) -> String {
    let text_width = display_width(text);
    if text_width >= width {
        return text.to_string();
    }

    let padding = width - text_width;
    format!("{}{}", text, " ".repeat(padding))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::{Color, Style};

    #[test]
    fn test_wrap_simple_text() {
        let result = wrap_text("hello world", 5);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_wrap_empty() {
        let result = wrap_text("", 10);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_display_width() {
        assert_eq!(display_width("hello"), 5);
        assert_eq!(display_width("日本語"), 6);
    }

    #[test]
    fn test_truncate_with_ellipsis() {
        assert_eq!(truncate_with_ellipsis("hello world", 8), "hello...");
        assert_eq!(truncate_with_ellipsis("short", 10), "short");
    }

    #[test]
    fn test_pad_to_width() {
        assert_eq!(pad_to_width("hello", 10), "hello     ");
        assert_eq!(pad_to_width("hello", 5), "hello");
    }

    #[test]
    fn test_word_wrap_line_preserves_style() {
        let style = Style::default().fg(Color::Cyan);
        let line = Line::from(vec![Span::styled("hello world foo bar", style)]);

        let wrapped = word_wrap_line(&line, 10);
        assert!(wrapped.len() >= 2);
    }
}
