//! Logging utilities for Miyabi
//!
//! Provides structured logging with multiple output formats

use tracing_subscriber::{fmt, layer::SubscriberExt, prelude::*, EnvFilter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// Human-readable colored output
    Pretty,
    /// Compact format for CI/CD
    Compact,
    /// JSON format for structured parsing
    Json,
}

/// Logger configuration
#[derive(Debug, Clone)]
pub struct LoggerConfig {
    /// Log level
    pub level: LogLevel,
    /// Log format
    pub format: LogFormat,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: LogFormat::Pretty,
        }
    }
}

impl From<&str> for LogLevel {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "trace" => LogLevel::Trace,
            "debug" => LogLevel::Debug,
            "info" => LogLevel::Info,
            "warn" => LogLevel::Warn,
            "error" => LogLevel::Error,
            _ => LogLevel::Info,
        }
    }
}

impl From<LogLevel> for &'static str {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }
}

impl From<&str> for LogFormat {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "json" => LogFormat::Json,
            "compact" => LogFormat::Compact,
            _ => LogFormat::Pretty,
        }
    }
}

/// Initialize logging with default configuration
pub fn init_logger(level: LogLevel) {
    let config = LoggerConfig {
        level,
        ..Default::default()
    };
    init_logger_with_config(config);
}

/// Initialize logging with custom configuration
pub fn init_logger_with_config(config: LoggerConfig) {
    let level_str: &'static str = config.level.into();
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level_str));

    let subscriber = tracing_subscriber::registry().with(filter);

    match config.format {
        LogFormat::Pretty => {
            let fmt_layer = fmt::layer()
                .pretty()
                .with_target(true)
                .with_thread_ids(false)
                .with_line_number(true);
            subscriber.with(fmt_layer).init();
        }
        LogFormat::Compact => {
            let fmt_layer = fmt::layer()
                .compact()
                .with_target(false)
                .with_thread_ids(false);
            subscriber.with(fmt_layer).init();
        }
        LogFormat::Json => {
            let fmt_layer = fmt::layer()
                .json()
                .with_current_span(true)
                .with_span_list(true);
            subscriber.with(fmt_layer).init();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevel::from("trace"), LogLevel::Trace);
        assert_eq!(LogLevel::from("debug"), LogLevel::Debug);
        assert_eq!(LogLevel::from("info"), LogLevel::Info);
        assert_eq!(LogLevel::from("warn"), LogLevel::Warn);
        assert_eq!(LogLevel::from("error"), LogLevel::Error);
        assert_eq!(LogLevel::from("invalid"), LogLevel::Info);
    }

    #[test]
    fn test_log_level_to_str() {
        let level_str: &'static str = LogLevel::Trace.into();
        assert_eq!(level_str, "trace");

        let level_str: &'static str = LogLevel::Info.into();
        assert_eq!(level_str, "info");
    }

    #[test]
    fn test_log_format_from_str() {
        assert_eq!(LogFormat::from("json"), LogFormat::Json);
        assert_eq!(LogFormat::from("compact"), LogFormat::Compact);
        assert_eq!(LogFormat::from("pretty"), LogFormat::Pretty);
        assert_eq!(LogFormat::from("invalid"), LogFormat::Pretty);
    }

    #[test]
    fn test_logger_config_default() {
        let config = LoggerConfig::default();
        assert_eq!(config.level, LogLevel::Info);
        assert_eq!(config.format, LogFormat::Pretty);
    }
}
