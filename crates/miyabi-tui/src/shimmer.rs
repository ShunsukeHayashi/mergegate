//! Shimmer - Loading animation effects
//!
//! Following Codex patterns for loading states.
//! Features:
//! - Skeleton loading
//! - Animated shimmer effect
//! - Progress indicators
//! - Spinner variants

use std::time::{Duration, Instant};

use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Shimmer animation state
pub struct ShimmerState {
    /// Start time for animation
    start: Instant,
    /// Animation duration
    duration: Duration,
    /// Whether animation loops
    looping: bool,
}

impl ShimmerState {
    /// Create a new shimmer state
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            duration: Duration::from_millis(1500),
            looping: true,
        }
    }

    /// Set animation duration
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Set looping
    pub fn looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }

    /// Get current animation progress (0.0 - 1.0)
    pub fn progress(&self) -> f64 {
        let elapsed = self.start.elapsed();
        if self.looping {
            let cycles = elapsed.as_secs_f64() / self.duration.as_secs_f64();
            cycles.fract()
        } else {
            (elapsed.as_secs_f64() / self.duration.as_secs_f64()).min(1.0)
        }
    }

    /// Reset animation
    pub fn reset(&mut self) {
        self.start = Instant::now();
    }
}

impl Default for ShimmerState {
    fn default() -> Self {
        Self::new()
    }
}

/// Shimmer effect types
#[derive(Debug, Clone, Copy)]
pub enum ShimmerEffect {
    /// Left-to-right wave
    Wave,
    /// Pulsing opacity
    Pulse,
    /// Gradient sweep
    Gradient,
    /// Typing dots
    Dots,
}

/// Skeleton loading widget
pub struct SkeletonLoader {
    /// Number of lines
    lines: usize,
    /// Line widths (as percentages)
    widths: Vec<u16>,
    /// Animation state
    state: ShimmerState,
    /// Effect type
    effect: ShimmerEffect,
    /// Base color
    base_color: Color,
    /// Highlight color
    highlight_color: Color,
}

impl SkeletonLoader {
    /// Create a new skeleton loader
    pub fn new(lines: usize) -> Self {
        let mut widths = Vec::with_capacity(lines);
        for i in 0..lines {
            // Vary widths for realistic look
            let width = match i % 4 {
                0 => 90,
                1 => 70,
                2 => 85,
                _ => 60,
            };
            widths.push(width);
        }

        Self {
            lines,
            widths,
            state: ShimmerState::new(),
            effect: ShimmerEffect::Wave,
            base_color: Color::Rgb(45, 50, 80),
            highlight_color: Color::Rgb(65, 72, 104),
        }
    }

    /// Set specific line widths
    pub fn widths(mut self, widths: Vec<u16>) -> Self {
        self.widths = widths;
        self.lines = self.widths.len();
        self
    }

    /// Set effect
    pub fn effect(mut self, effect: ShimmerEffect) -> Self {
        self.effect = effect;
        self
    }

    /// Set colors
    pub fn colors(mut self, base: Color, highlight: Color) -> Self {
        self.base_color = base;
        self.highlight_color = highlight;
        self
    }

    /// Get animation state
    pub fn state(&self) -> &ShimmerState {
        &self.state
    }

    /// Get mutable animation state
    pub fn state_mut(&mut self) -> &mut ShimmerState {
        &mut self.state
    }

    /// Render the skeleton loader
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let progress = self.state.progress();

        for i in 0..self.lines.min(area.height as usize) {
            let y = area.y + i as u16;
            let width_percent = self.widths.get(i).copied().unwrap_or(80);
            let bar_width = (area.width as u32 * width_percent as u32 / 100) as u16;

            let bar_area = Rect {
                x: area.x,
                y,
                width: bar_width.min(area.width),
                height: 1,
            };

            self.render_bar(frame, bar_area, progress, i);
        }
    }

    /// Render a single skeleton bar
    fn render_bar(&self, frame: &mut Frame, area: Rect, progress: f64, _index: usize) {
        let width = area.width as usize;
        if width == 0 {
            return;
        }

        let chars: Vec<Span> = match self.effect {
            ShimmerEffect::Wave => {
                // Wave effect: highlight moves across
                let wave_pos = (progress * width as f64) as usize;
                let wave_width = width / 4;

                (0..width)
                    .map(|x| {
                        let dist = x.abs_diff(wave_pos);
                        let color = if dist < wave_width {
                            let blend = 1.0 - (dist as f64 / wave_width as f64);
                            blend_colors(self.base_color, self.highlight_color, blend)
                        } else {
                            self.base_color
                        };
                        Span::styled("█", Style::default().fg(color))
                    })
                    .collect()
            }
            ShimmerEffect::Pulse => {
                // Pulse effect: entire bar pulses
                let intensity = (progress * std::f64::consts::PI * 2.0).sin() * 0.5 + 0.5;
                let color = blend_colors(self.base_color, self.highlight_color, intensity);
                vec![Span::styled(
                    "█".repeat(width),
                    Style::default().fg(color),
                )]
            }
            ShimmerEffect::Gradient => {
                // Gradient sweep
                let offset = (progress * width as f64 * 2.0) as usize;
                (0..width)
                    .map(|x| {
                        let pos = (x + offset) % (width * 2);
                        let intensity = if pos < width {
                            pos as f64 / width as f64
                        } else {
                            1.0 - ((pos - width) as f64 / width as f64)
                        };
                        let color = blend_colors(self.base_color, self.highlight_color, intensity);
                        Span::styled("█", Style::default().fg(color))
                    })
                    .collect()
            }
            ShimmerEffect::Dots => {
                // Dots loading effect (for text lines)
                let color = blend_colors(
                    self.base_color,
                    self.highlight_color,
                    (progress * std::f64::consts::PI).sin(),
                );
                vec![Span::styled(
                    "─".repeat(width),
                    Style::default().fg(color),
                )]
            }
        };

        let line = Line::from(chars);
        let paragraph = Paragraph::new(line);
        frame.render_widget(paragraph, area);
    }
}

/// Spinner widget
pub struct Spinner {
    /// Spinner style
    style: SpinnerStyle,
    /// Animation state
    state: ShimmerState,
    /// Label
    label: String,
    /// Color
    color: Color,
}

/// Spinner animation styles
#[derive(Debug, Clone, Copy)]
pub enum SpinnerStyle {
    /// Classic dots (...)
    Dots,
    /// Braille spinner
    Braille,
    /// Line spinner
    Line,
    /// Arrow spinner
    Arrow,
    /// Block spinner
    Block,
    /// Moon phases
    Moon,
    /// Custom frames
    Custom,
}

impl SpinnerStyle {
    /// Get animation frames
    fn frames(&self) -> &[&str] {
        match self {
            SpinnerStyle::Dots => &["   ", ".  ", ".. ", "...", ".. ", ".  "],
            SpinnerStyle::Braille => &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            SpinnerStyle::Line => &["-", "\\", "|", "/"],
            SpinnerStyle::Arrow => &["←", "↖", "↑", "↗", "→", "↘", "↓", "↙"],
            SpinnerStyle::Block => &["▖", "▘", "▝", "▗"],
            SpinnerStyle::Moon => &["🌑", "🌒", "🌓", "🌔", "🌕", "🌖", "🌗", "🌘"],
            SpinnerStyle::Custom => &["●", "○"],
        }
    }
}

impl Spinner {
    /// Create a new spinner
    pub fn new() -> Self {
        Self {
            style: SpinnerStyle::Braille,
            state: ShimmerState::new().duration(Duration::from_millis(100)),
            label: String::new(),
            color: Color::Cyan,
        }
    }

    /// Set spinner style
    pub fn style(mut self, style: SpinnerStyle) -> Self {
        self.style = style;
        self
    }

    /// Set label
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Set color
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Get animation state
    pub fn state(&self) -> &ShimmerState {
        &self.state
    }

    /// Get mutable animation state
    pub fn state_mut(&mut self) -> &mut ShimmerState {
        &mut self.state
    }

    /// Render the spinner
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let frames = self.style.frames();
        let frame_count = frames.len();
        let progress = self.state.progress();
        let frame_idx = (progress * frame_count as f64) as usize % frame_count;

        let mut spans = vec![Span::styled(
            frames[frame_idx],
            Style::default().fg(self.color),
        )];

        if !self.label.is_empty() {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                &self.label,
                Style::default().fg(Color::Rgb(192, 202, 245)),
            ));
        }

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line);
        frame.render_widget(paragraph, area);
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

/// Progress bar widget
pub struct ProgressBar {
    /// Current progress (0.0 - 1.0)
    progress: f64,
    /// Label
    label: String,
    /// Show percentage
    show_percent: bool,
    /// Bar character
    bar_char: char,
    /// Empty character
    empty_char: char,
    /// Fill color
    fill_color: Color,
    /// Empty color
    empty_color: Color,
}

impl ProgressBar {
    /// Create a new progress bar
    pub fn new(progress: f64) -> Self {
        Self {
            progress: progress.clamp(0.0, 1.0),
            label: String::new(),
            show_percent: true,
            bar_char: '█',
            empty_char: '░',
            fill_color: Color::Cyan,
            empty_color: Color::Rgb(45, 50, 80),
        }
    }

    /// Set label
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Set show percentage
    pub fn show_percent(mut self, show: bool) -> Self {
        self.show_percent = show;
        self
    }

    /// Set bar characters
    pub fn chars(mut self, bar: char, empty: char) -> Self {
        self.bar_char = bar;
        self.empty_char = empty;
        self
    }

    /// Set colors
    pub fn colors(mut self, fill: Color, empty: Color) -> Self {
        self.fill_color = fill;
        self.empty_color = empty;
        self
    }

    /// Render the progress bar
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let mut available_width = area.width as usize;

        // Calculate space for percentage
        let percent_width = if self.show_percent { 5 } else { 0 };
        available_width = available_width.saturating_sub(percent_width);

        // Calculate space for label
        let label_width = if !self.label.is_empty() {
            self.label.len() + 1
        } else {
            0
        };
        available_width = available_width.saturating_sub(label_width);

        let filled = (self.progress * available_width as f64) as usize;
        let empty = available_width.saturating_sub(filled);

        let mut spans = vec![];

        // Label
        if !self.label.is_empty() {
            spans.push(Span::styled(
                format!("{} ", self.label),
                Style::default().fg(Color::Rgb(192, 202, 245)),
            ));
        }

        // Bar
        spans.push(Span::styled(
            self.bar_char.to_string().repeat(filled),
            Style::default().fg(self.fill_color),
        ));
        spans.push(Span::styled(
            self.empty_char.to_string().repeat(empty),
            Style::default().fg(self.empty_color),
        ));

        // Percentage
        if self.show_percent {
            spans.push(Span::styled(
                format!(" {:>3}%", (self.progress * 100.0) as u32),
                Style::default().fg(Color::Rgb(192, 202, 245)),
            ));
        }

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line);
        frame.render_widget(paragraph, area);
    }
}

/// Typing indicator (thinking animation)
pub struct TypingIndicator {
    /// Animation state
    state: ShimmerState,
    /// Label
    label: String,
    /// Color
    color: Color,
}

impl TypingIndicator {
    /// Create a new typing indicator
    pub fn new() -> Self {
        Self {
            state: ShimmerState::new().duration(Duration::from_millis(500)),
            label: "Thinking".to_string(),
            color: Color::Cyan,
        }
    }

    /// Set label
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Set color
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Get animation state
    pub fn state(&self) -> &ShimmerState {
        &self.state
    }

    /// Get mutable animation state
    pub fn state_mut(&mut self) -> &mut ShimmerState {
        &mut self.state
    }

    /// Render the typing indicator
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let progress = self.state.progress();
        let dots = match (progress * 4.0) as usize % 4 {
            0 => "",
            1 => ".",
            2 => "..",
            _ => "...",
        };

        let spans = vec![
            Span::styled(&self.label, Style::default().fg(self.color)),
            Span::styled(dots, Style::default().fg(self.color)),
        ];

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line);
        frame.render_widget(paragraph, area);
    }
}

impl Default for TypingIndicator {
    fn default() -> Self {
        Self::new()
    }
}

/// Countdown timer widget
pub struct Countdown {
    /// Target duration
    duration: Duration,
    /// Start time
    start: Instant,
    /// Label
    label: String,
    /// Color
    color: Color,
}

impl Countdown {
    /// Create a new countdown
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            start: Instant::now(),
            label: String::new(),
            color: Color::Cyan,
        }
    }

    /// Set label
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Set color
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Reset countdown
    pub fn reset(&mut self) {
        self.start = Instant::now();
    }

    /// Check if countdown is complete
    pub fn is_complete(&self) -> bool {
        self.start.elapsed() >= self.duration
    }

    /// Get remaining time
    pub fn remaining(&self) -> Duration {
        self.duration.saturating_sub(self.start.elapsed())
    }

    /// Render the countdown
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let remaining = self.remaining();
        let secs = remaining.as_secs();
        let mins = secs / 60;
        let secs = secs % 60;

        let time_str = if mins > 0 {
            format!("{}:{:02}", mins, secs)
        } else {
            format!("{}s", secs)
        };

        let mut spans = vec![];

        if !self.label.is_empty() {
            spans.push(Span::styled(
                format!("{} ", self.label),
                Style::default().fg(Color::Rgb(192, 202, 245)),
            ));
        }

        spans.push(Span::styled(
            time_str,
            Style::default().fg(self.color).add_modifier(Modifier::BOLD),
        ));

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line);
        frame.render_widget(paragraph, area);
    }
}

/// Blend two colors
fn blend_colors(a: Color, b: Color, t: f64) -> Color {
    match (a, b) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
            let r = (r1 as f64 * (1.0 - t) + r2 as f64 * t) as u8;
            let g = (g1 as f64 * (1.0 - t) + g2 as f64 * t) as u8;
            let b = (b1 as f64 * (1.0 - t) + b2 as f64 * t) as u8;
            Color::Rgb(r, g, b)
        }
        _ => if t < 0.5 { a } else { b },
    }
}

/// Loading state for components
#[derive(Debug, Clone)]
pub enum LoadingState {
    /// Not loading
    Idle,
    /// Loading with message
    Loading(String),
    /// Loading with progress
    Progress { message: String, progress: f64 },
    /// Complete
    Complete,
    /// Error
    Error(String),
}

impl LoadingState {
    /// Check if loading
    pub fn is_loading(&self) -> bool {
        matches!(self, LoadingState::Loading(_) | LoadingState::Progress { .. })
    }

    /// Check if complete
    pub fn is_complete(&self) -> bool {
        matches!(self, LoadingState::Complete)
    }

    /// Check if error
    pub fn is_error(&self) -> bool {
        matches!(self, LoadingState::Error(_))
    }
}

/// Loading overlay component
pub struct LoadingOverlay {
    /// Loading state
    state: LoadingState,
    /// Spinner
    spinner: Spinner,
    /// Show overlay
    visible: bool,
}

impl LoadingOverlay {
    /// Create a new loading overlay
    pub fn new() -> Self {
        Self {
            state: LoadingState::Idle,
            spinner: Spinner::new(),
            visible: false,
        }
    }

    /// Show loading
    pub fn show(&mut self, message: impl Into<String>) {
        self.state = LoadingState::Loading(message.into());
        self.visible = true;
        self.spinner.state_mut().reset();
    }

    /// Show with progress
    pub fn show_progress(&mut self, message: impl Into<String>, progress: f64) {
        self.state = LoadingState::Progress {
            message: message.into(),
            progress,
        };
        self.visible = true;
    }

    /// Hide loading
    pub fn hide(&mut self) {
        self.visible = false;
        self.state = LoadingState::Idle;
    }

    /// Set complete
    pub fn complete(&mut self) {
        self.state = LoadingState::Complete;
    }

    /// Set error
    pub fn error(&mut self, message: impl Into<String>) {
        self.state = LoadingState::Error(message.into());
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Render the loading overlay
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Semi-transparent overlay would go here if supported
        // For now, just render the loading indicator

        let popup_width = area.width.min(40);
        let popup_height = 5;

        let popup_area = Rect {
            x: (area.width - popup_width) / 2 + area.x,
            y: (area.height - popup_height) / 2 + area.y,
            width: popup_width,
            height: popup_height,
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        match &self.state {
            LoadingState::Loading(message) => {
                let spinner = Spinner::new().label(message.clone());
                spinner.render(frame, inner);
            }
            LoadingState::Progress { message, progress } => {
                let bar = ProgressBar::new(*progress).label(message.clone());
                bar.render(frame, inner);
            }
            LoadingState::Complete => {
                let line = Line::from(Span::styled(
                    "✓ Complete",
                    Style::default().fg(Color::Green),
                ));
                let paragraph = Paragraph::new(line).alignment(Alignment::Center);
                frame.render_widget(paragraph, inner);
            }
            LoadingState::Error(message) => {
                let line = Line::from(Span::styled(
                    format!("✗ {}", message),
                    Style::default().fg(Color::Red),
                ));
                let paragraph = Paragraph::new(line).alignment(Alignment::Center);
                frame.render_widget(paragraph, inner);
            }
            LoadingState::Idle => {}
        }
    }
}

impl Default for LoadingOverlay {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shimmer_state_creation() {
        let state = ShimmerState::new();
        assert!(state.looping);
        assert_eq!(state.duration, Duration::from_millis(1500));
    }

    #[test]
    fn test_shimmer_state_duration() {
        let state = ShimmerState::new().duration(Duration::from_secs(2));
        assert_eq!(state.duration, Duration::from_secs(2));
    }

    #[test]
    fn test_shimmer_state_looping() {
        let state = ShimmerState::new().looping(false);
        assert!(!state.looping);
    }

    #[test]
    fn test_shimmer_state_progress() {
        let state = ShimmerState::new();
        let progress = state.progress();
        assert!(progress >= 0.0 && progress <= 1.0);
    }

    #[test]
    fn test_shimmer_state_reset() {
        let mut state = ShimmerState::new();
        std::thread::sleep(Duration::from_millis(10));
        state.reset();
        let progress = state.progress();
        assert!(progress < 0.1); // Should be near start
    }

    #[test]
    fn test_skeleton_loader_creation() {
        let loader = SkeletonLoader::new(5);
        assert_eq!(loader.lines, 5);
        assert_eq!(loader.widths.len(), 5);
    }

    #[test]
    fn test_skeleton_loader_widths() {
        let loader = SkeletonLoader::new(3).widths(vec![100, 80, 60]);
        assert_eq!(loader.widths, vec![100, 80, 60]);
        assert_eq!(loader.lines, 3);
    }

    #[test]
    fn test_skeleton_loader_effect() {
        let loader = SkeletonLoader::new(3).effect(ShimmerEffect::Pulse);
        assert!(matches!(loader.effect, ShimmerEffect::Pulse));
    }

    #[test]
    fn test_skeleton_loader_colors() {
        let loader = SkeletonLoader::new(3).colors(Color::Red, Color::Blue);
        assert_eq!(loader.base_color, Color::Red);
        assert_eq!(loader.highlight_color, Color::Blue);
    }

    #[test]
    fn test_spinner_creation() {
        let spinner = Spinner::new();
        assert_eq!(spinner.color, Color::Cyan);
        assert!(spinner.label.is_empty());
    }

    #[test]
    fn test_spinner_style() {
        let spinner = Spinner::new().style(SpinnerStyle::Dots);
        assert!(matches!(spinner.style, SpinnerStyle::Dots));
    }

    #[test]
    fn test_spinner_label() {
        let spinner = Spinner::new().label("Loading...");
        assert_eq!(spinner.label, "Loading...");
    }

    #[test]
    fn test_spinner_color() {
        let spinner = Spinner::new().color(Color::Green);
        assert_eq!(spinner.color, Color::Green);
    }

    #[test]
    fn test_spinner_style_frames() {
        assert!(!SpinnerStyle::Dots.frames().is_empty());
        assert!(!SpinnerStyle::Braille.frames().is_empty());
        assert!(!SpinnerStyle::Line.frames().is_empty());
        assert!(!SpinnerStyle::Arrow.frames().is_empty());
        assert!(!SpinnerStyle::Block.frames().is_empty());
        assert!(!SpinnerStyle::Moon.frames().is_empty());
    }

    #[test]
    fn test_progress_bar_creation() {
        let bar = ProgressBar::new(0.5);
        assert_eq!(bar.progress, 0.5);
        assert!(bar.show_percent);
    }

    #[test]
    fn test_progress_bar_clamp() {
        let bar = ProgressBar::new(1.5);
        assert_eq!(bar.progress, 1.0);

        let bar = ProgressBar::new(-0.5);
        assert_eq!(bar.progress, 0.0);
    }

    #[test]
    fn test_progress_bar_label() {
        let bar = ProgressBar::new(0.5).label("Downloading");
        assert_eq!(bar.label, "Downloading");
    }

    #[test]
    fn test_progress_bar_show_percent() {
        let bar = ProgressBar::new(0.5).show_percent(false);
        assert!(!bar.show_percent);
    }

    #[test]
    fn test_progress_bar_chars() {
        let bar = ProgressBar::new(0.5).chars('=', '-');
        assert_eq!(bar.bar_char, '=');
        assert_eq!(bar.empty_char, '-');
    }

    #[test]
    fn test_typing_indicator_creation() {
        let indicator = TypingIndicator::new();
        assert_eq!(indicator.label, "Thinking");
        assert_eq!(indicator.color, Color::Cyan);
    }

    #[test]
    fn test_typing_indicator_label() {
        let indicator = TypingIndicator::new().label("Processing");
        assert_eq!(indicator.label, "Processing");
    }

    #[test]
    fn test_countdown_creation() {
        let countdown = Countdown::new(Duration::from_secs(10));
        assert_eq!(countdown.duration, Duration::from_secs(10));
        assert!(!countdown.is_complete());
    }

    #[test]
    fn test_countdown_label() {
        let countdown = Countdown::new(Duration::from_secs(10)).label("Time left");
        assert_eq!(countdown.label, "Time left");
    }

    #[test]
    fn test_countdown_remaining() {
        let countdown = Countdown::new(Duration::from_secs(10));
        let remaining = countdown.remaining();
        assert!(remaining <= Duration::from_secs(10));
    }

    #[test]
    fn test_loading_state_is_loading() {
        assert!(LoadingState::Loading("test".to_string()).is_loading());
        assert!(LoadingState::Progress {
            message: "test".to_string(),
            progress: 0.5
        }
        .is_loading());
        assert!(!LoadingState::Idle.is_loading());
        assert!(!LoadingState::Complete.is_loading());
    }

    #[test]
    fn test_loading_state_is_complete() {
        assert!(LoadingState::Complete.is_complete());
        assert!(!LoadingState::Idle.is_complete());
        assert!(!LoadingState::Loading("test".to_string()).is_complete());
    }

    #[test]
    fn test_loading_state_is_error() {
        assert!(LoadingState::Error("error".to_string()).is_error());
        assert!(!LoadingState::Idle.is_error());
        assert!(!LoadingState::Complete.is_error());
    }

    #[test]
    fn test_loading_overlay_creation() {
        let overlay = LoadingOverlay::new();
        assert!(!overlay.is_visible());
        assert!(matches!(overlay.state, LoadingState::Idle));
    }

    #[test]
    fn test_loading_overlay_show() {
        let mut overlay = LoadingOverlay::new();
        overlay.show("Loading data");
        assert!(overlay.is_visible());
        assert!(matches!(overlay.state, LoadingState::Loading(_)));
    }

    #[test]
    fn test_loading_overlay_show_progress() {
        let mut overlay = LoadingOverlay::new();
        overlay.show_progress("Downloading", 0.5);
        assert!(overlay.is_visible());
        assert!(matches!(overlay.state, LoadingState::Progress { .. }));
    }

    #[test]
    fn test_loading_overlay_hide() {
        let mut overlay = LoadingOverlay::new();
        overlay.show("Loading");
        overlay.hide();
        assert!(!overlay.is_visible());
        assert!(matches!(overlay.state, LoadingState::Idle));
    }

    #[test]
    fn test_loading_overlay_complete() {
        let mut overlay = LoadingOverlay::new();
        overlay.show("Loading");
        overlay.complete();
        assert!(matches!(overlay.state, LoadingState::Complete));
    }

    #[test]
    fn test_loading_overlay_error() {
        let mut overlay = LoadingOverlay::new();
        overlay.show("Loading");
        overlay.error("Failed to load");
        assert!(matches!(overlay.state, LoadingState::Error(_)));
    }

    #[test]
    fn test_blend_colors_rgb() {
        let result = blend_colors(
            Color::Rgb(0, 0, 0),
            Color::Rgb(255, 255, 255),
            0.5,
        );
        assert!(matches!(result, Color::Rgb(127, 127, 127)));
    }

    #[test]
    fn test_blend_colors_non_rgb() {
        let result = blend_colors(Color::Red, Color::Blue, 0.3);
        assert_eq!(result, Color::Red); // t < 0.5

        let result = blend_colors(Color::Red, Color::Blue, 0.7);
        assert_eq!(result, Color::Blue); // t >= 0.5
    }
}
