//! Syntax Highlighting
//!
//! This module provides syntax highlighting for code blocks using syntect.

use once_cell::sync::Lazy;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use syntect::{
    easy::HighlightLines,
    highlighting::{Theme, ThemeSet},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

/// Global syntax set (loaded once)
static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);

/// Global theme set
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

/// Get the default theme
fn default_theme() -> &'static Theme {
    &THEME_SET.themes["base16-ocean.dark"]
}

/// Syntax highlighter for code blocks
#[derive(Debug, Clone)]
pub struct SyntaxHighlighter {
    /// Theme name to use
    theme_name: String,
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl SyntaxHighlighter {
    /// Create a new syntax highlighter
    pub fn new() -> Self {
        Self {
            theme_name: "base16-ocean.dark".to_string(),
        }
    }

    /// Create with a specific theme
    pub fn with_theme(theme_name: impl Into<String>) -> Self {
        Self {
            theme_name: theme_name.into(),
        }
    }

    /// Get the theme to use
    fn get_theme(&self) -> &Theme {
        THEME_SET
            .themes
            .get(&self.theme_name)
            .unwrap_or_else(default_theme)
    }

    /// Highlight code and return styled lines
    pub fn highlight(&self, code: &str, language: &str) -> Vec<Line<'static>> {
        let syntax = SYNTAX_SET
            .find_syntax_by_token(language)
            .or_else(|| SYNTAX_SET.find_syntax_by_extension(language))
            .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

        let theme = self.get_theme();
        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut lines = Vec::new();

        for line in LinesWithEndings::from(code) {
            match highlighter.highlight_line(line, &SYNTAX_SET) {
                Ok(ranges) => {
                    let spans: Vec<Span<'static>> = ranges
                        .into_iter()
                        .map(|(style, text)| {
                            let fg = Color::Rgb(
                                style.foreground.r,
                                style.foreground.g,
                                style.foreground.b,
                            );
                            Span::styled(
                                text.trim_end_matches('\n').to_string(),
                                Style::default().fg(fg),
                            )
                        })
                        .collect();
                    lines.push(Line::from(spans));
                }
                Err(_) => {
                    // Fallback to plain text
                    lines.push(Line::from(Span::styled(
                        line.trim_end_matches('\n').to_string(),
                        Style::default().fg(Color::Green),
                    )));
                }
            }
        }

        lines
    }

    /// Highlight a single line of code
    pub fn highlight_line(&self, line: &str, language: &str) -> Line<'static> {
        let lines = self.highlight(line, language);
        lines.into_iter().next().unwrap_or_default()
    }

    /// Get list of supported languages
    pub fn supported_languages() -> Vec<String> {
        SYNTAX_SET
            .syntaxes()
            .iter()
            .map(|s| s.name.clone())
            .collect()
    }

    /// Check if a language is supported
    pub fn is_supported(language: &str) -> bool {
        SYNTAX_SET.find_syntax_by_token(language).is_some()
            || SYNTAX_SET.find_syntax_by_extension(language).is_some()
    }
}

/// Map common language aliases to syntect names
pub fn normalize_language(lang: &str) -> &str {
    match lang.to_lowercase().as_str() {
        "js" | "javascript" => "JavaScript",
        "ts" | "typescript" => "TypeScript",
        "py" | "python" => "Python",
        "rb" | "ruby" => "Ruby",
        "rs" | "rust" => "Rust",
        "go" | "golang" => "Go",
        "c" => "C",
        "cpp" | "c++" => "C++",
        "cs" | "csharp" => "C#",
        "java" => "Java",
        "sh" | "bash" | "shell" => "Bourne Again Shell (bash)",
        "zsh" => "Bourne Again Shell (bash)",
        "json" => "JSON",
        "yaml" | "yml" => "YAML",
        "toml" => "TOML",
        "xml" => "XML",
        "html" => "HTML",
        "css" => "CSS",
        "sql" => "SQL",
        "md" | "markdown" => "Markdown",
        "dockerfile" => "Dockerfile",
        "makefile" | "make" => "Makefile",
        _ => lang,
    }
}

/// Highlight code with automatic language detection
pub fn highlight_code(code: &str, language: &str) -> Vec<Line<'static>> {
    let highlighter = SyntaxHighlighter::new();
    let normalized = normalize_language(language);
    highlighter.highlight(code, normalized)
}

/// Render a code block with borders
pub fn render_code_block(code: &str, language: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Header
    let header = if language.is_empty() {
        "```".to_string()
    } else {
        format!("```{}", language)
    };
    lines.push(Line::from(Span::styled(
        header,
        Style::default().fg(Color::DarkGray),
    )));

    // Highlighted code
    let highlighted = highlight_code(code, language);
    lines.extend(highlighted);

    // Footer
    lines.push(Line::from(Span::styled(
        "```",
        Style::default().fg(Color::DarkGray),
    )));

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlighter_creation() {
        let highlighter = SyntaxHighlighter::new();
        assert_eq!(highlighter.theme_name, "base16-ocean.dark");
    }

    #[test]
    fn test_highlight_rust() {
        let highlighter = SyntaxHighlighter::new();
        let code = "fn main() {\n    println!(\"Hello\");\n}";
        let lines = highlighter.highlight(code, "rust");

        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_highlight_python() {
        let highlighter = SyntaxHighlighter::new();
        let code = "def hello():\n    print('Hello')";
        let lines = highlighter.highlight(code, "python");

        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_highlight_unknown_language() {
        let highlighter = SyntaxHighlighter::new();
        let code = "some code";
        let lines = highlighter.highlight(code, "unknown_lang_xyz");

        // Should fall back to plain text
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_normalize_language() {
        assert_eq!(normalize_language("js"), "JavaScript");
        assert_eq!(normalize_language("ts"), "TypeScript");
        assert_eq!(normalize_language("py"), "Python");
        assert_eq!(normalize_language("rs"), "Rust");
    }

    #[test]
    fn test_is_supported() {
        assert!(SyntaxHighlighter::is_supported("rust"));
        assert!(SyntaxHighlighter::is_supported("python"));
        assert!(SyntaxHighlighter::is_supported("javascript"));
    }

    #[test]
    fn test_render_code_block() {
        let code = "let x = 1;";
        let lines = render_code_block(code, "rust");

        // Should have header, code, footer
        assert!(lines.len() >= 3);
    }

    #[test]
    fn test_highlight_code_helper() {
        let code = "console.log('hello');";
        let lines = highlight_code(code, "js");

        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_empty_code() {
        let highlighter = SyntaxHighlighter::new();
        let lines = highlighter.highlight("", "rust");

        assert!(lines.is_empty() || lines.len() == 1);
    }

    #[test]
    fn test_multiline_code() {
        let code = "fn main() {\n    let x = 1;\n    let y = 2;\n    println!(\"{}\", x + y);\n}";
        let lines = highlight_code(code, "rust");

        assert_eq!(lines.len(), 5);
    }
}
