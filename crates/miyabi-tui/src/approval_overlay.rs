//! Approval Overlay - Tool/action approval UI
//!
//! Following Codex patterns for approval dialogs.
//! Features:
//! - Clear action description
//! - Risk level indication
//! - Approve/Reject with keyboard
//! - Details expansion

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Risk level for approval requests
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    /// Low risk - informational
    Low,
    /// Medium risk - may modify files
    Medium,
    /// High risk - system modifications
    High,
    /// Critical risk - destructive operations
    Critical,
}

impl RiskLevel {
    /// Get color for this risk level
    pub fn color(&self) -> Color {
        match self {
            RiskLevel::Low => Color::Green,
            RiskLevel::Medium => Color::Yellow,
            RiskLevel::High => Color::Rgb(255, 165, 0), // Orange
            RiskLevel::Critical => Color::Red,
        }
    }

    /// Get label for this risk level
    pub fn label(&self) -> &'static str {
        match self {
            RiskLevel::Low => "LOW",
            RiskLevel::Medium => "MEDIUM",
            RiskLevel::High => "HIGH",
            RiskLevel::Critical => "CRITICAL",
        }
    }

    /// Get icon for this risk level
    pub fn icon(&self) -> &'static str {
        match self {
            RiskLevel::Low => "○",
            RiskLevel::Medium => "◐",
            RiskLevel::High => "●",
            RiskLevel::Critical => "⚠",
        }
    }
}

/// Approval request data
#[derive(Debug, Clone)]
pub struct ApprovalRequest {
    /// Request ID
    pub id: String,
    /// Action title
    pub title: String,
    /// Action description
    pub description: String,
    /// Tool/command name
    pub tool_name: String,
    /// Tool arguments or parameters
    pub arguments: String,
    /// Risk level
    pub risk_level: RiskLevel,
    /// Additional details
    pub details: Vec<String>,
    /// Whether to show details expanded
    pub show_details: bool,
}

impl ApprovalRequest {
    /// Create a new approval request
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: String::new(),
            tool_name: String::new(),
            arguments: String::new(),
            risk_level: RiskLevel::Medium,
            details: Vec::new(),
            show_details: false,
        }
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set tool name
    pub fn tool_name(mut self, name: impl Into<String>) -> Self {
        self.tool_name = name.into();
        self
    }

    /// Set arguments
    pub fn arguments(mut self, args: impl Into<String>) -> Self {
        self.arguments = args.into();
        self
    }

    /// Set risk level
    pub fn risk_level(mut self, level: RiskLevel) -> Self {
        self.risk_level = level;
        self
    }

    /// Add detail line
    pub fn add_detail(mut self, detail: impl Into<String>) -> Self {
        self.details.push(detail.into());
        self
    }

    /// Set details
    pub fn details(mut self, details: Vec<String>) -> Self {
        self.details = details;
        self
    }
}

/// Approval overlay state
pub struct ApprovalOverlay {
    /// Current request
    request: Option<ApprovalRequest>,
    /// Whether overlay is visible
    visible: bool,
    /// Selected button (0 = Approve, 1 = Reject, 2 = Details)
    selected: usize,
    /// Auto-approve enabled
    auto_approve: bool,
    /// Pending requests queue
    queue: Vec<ApprovalRequest>,
}

impl ApprovalOverlay {
    /// Create a new approval overlay
    pub fn new() -> Self {
        Self {
            request: None,
            visible: false,
            selected: 0, // Default to Approve
            auto_approve: false,
            queue: Vec::new(),
        }
    }

    /// Show approval request
    pub fn show(&mut self, request: ApprovalRequest) {
        if self.auto_approve {
            // Auto-approve, skip showing
            return;
        }

        if self.visible {
            // Queue if already showing
            self.queue.push(request);
        } else {
            self.request = Some(request);
            self.visible = true;
            self.selected = 0;
        }
    }

    /// Hide overlay
    pub fn hide(&mut self) {
        self.visible = false;
        self.request = None;

        // Show next in queue
        if let Some(next) = self.queue.pop() {
            self.show(next);
        }
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Set auto-approve mode
    pub fn set_auto_approve(&mut self, enabled: bool) {
        self.auto_approve = enabled;
    }

    /// Get current request
    pub fn current_request(&self) -> Option<&ApprovalRequest> {
        self.request.as_ref()
    }

    /// Handle key event
    pub fn handle_key(&mut self, key: KeyEvent) -> ApprovalAction {
        if !self.visible {
            return ApprovalAction::None;
        }

        match key.code {
            // Approve shortcuts
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let request_id = self.request.as_ref().map(|r| r.id.clone());
                self.hide();
                ApprovalAction::Approve(request_id.unwrap_or_default())
            }
            KeyCode::Enter if self.selected == 0 => {
                let request_id = self.request.as_ref().map(|r| r.id.clone());
                self.hide();
                ApprovalAction::Approve(request_id.unwrap_or_default())
            }

            // Reject shortcuts
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                let request_id = self.request.as_ref().map(|r| r.id.clone());
                self.hide();
                ApprovalAction::Reject(request_id.unwrap_or_default())
            }
            KeyCode::Enter if self.selected == 1 => {
                let request_id = self.request.as_ref().map(|r| r.id.clone());
                self.hide();
                ApprovalAction::Reject(request_id.unwrap_or_default())
            }

            // Toggle details
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if let Some(req) = &mut self.request {
                    req.show_details = !req.show_details;
                }
                ApprovalAction::None
            }
            KeyCode::Enter if self.selected == 2 => {
                if let Some(req) = &mut self.request {
                    req.show_details = !req.show_details;
                }
                ApprovalAction::None
            }

            // Navigation
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Tab
                if key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                self.selected = self.selected.saturating_sub(1);
                ApprovalAction::None
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
                let max = if self
                    .request
                    .as_ref()
                    .map(|r| !r.details.is_empty())
                    .unwrap_or(false)
                {
                    2
                } else {
                    1
                };
                self.selected = (self.selected + 1).min(max);
                ApprovalAction::None
            }

            // Always approve from now on
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.auto_approve = true;
                let request_id = self.request.as_ref().map(|r| r.id.clone());
                self.hide();
                ApprovalAction::ApproveAll(request_id.unwrap_or_default())
            }

            _ => ApprovalAction::None,
        }
    }

    /// Render the overlay
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let request = match &self.request {
            Some(r) => r,
            None => return,
        };

        // Calculate popup dimensions
        let popup_width = area.width.min(70);
        let base_height = if request.show_details && !request.details.is_empty() {
            18 + request.details.len().min(5) as u16
        } else {
            16
        };
        let popup_height = area.height.min(base_height);

        let popup_area = Rect {
            x: (area.width - popup_width) / 2 + area.x,
            y: (area.height - popup_height) / 2 + area.y,
            width: popup_width,
            height: popup_height,
        };

        // Clear background
        frame.render_widget(Clear, popup_area);

        // Main block
        let title = format!(" {} Approval Required ", request.risk_level.icon());
        let block = Block::default()
            .title(Span::styled(
                title,
                Style::default()
                    .fg(request.risk_level.color())
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(request.risk_level.color()));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        // Layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Risk indicator
                Constraint::Length(3), // Title
                Constraint::Min(4),    // Content
                Constraint::Length(3), // Buttons
            ])
            .split(inner);

        // Risk level indicator
        self.render_risk_indicator(frame, chunks[0], request);

        // Title
        self.render_title(frame, chunks[1], request);

        // Content
        self.render_content(frame, chunks[2], request);

        // Buttons
        self.render_buttons(frame, chunks[3], request);
    }

    /// Render risk indicator
    fn render_risk_indicator(&self, frame: &mut Frame, area: Rect, request: &ApprovalRequest) {
        let indicator = Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {} ", request.risk_level.label()),
                Style::default()
                    .fg(Color::Black)
                    .bg(request.risk_level.color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                &request.tool_name,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        frame.render_widget(indicator, area);
    }

    /// Render title
    fn render_title(&self, frame: &mut Frame, area: Rect, request: &ApprovalRequest) {
        let title = Paragraph::new(Line::from(Span::styled(
            &request.title,
            Style::default()
                .fg(Color::Rgb(192, 202, 245))
                .add_modifier(Modifier::BOLD),
        )))
        .wrap(Wrap { trim: false });
        frame.render_widget(title, area);
    }

    /// Render content
    fn render_content(&self, frame: &mut Frame, area: Rect, request: &ApprovalRequest) {
        let mut lines = Vec::new();

        // Description
        if !request.description.is_empty() {
            lines.push(Line::from(Span::styled(
                &request.description,
                Style::default().fg(Color::Rgb(169, 177, 214)),
            )));
            lines.push(Line::from(""));
        }

        // Arguments
        if !request.arguments.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "Command: ",
                Style::default().fg(Color::Rgb(86, 95, 137)),
            )]));

            // Wrap long arguments
            let arg_lines: Vec<&str> = request.arguments.lines().collect();
            for arg_line in arg_lines.iter().take(3) {
                lines.push(Line::from(Span::styled(
                    format!("  {}", arg_line),
                    Style::default().fg(Color::Rgb(192, 202, 245)),
                )));
            }
            if arg_lines.len() > 3 {
                lines.push(Line::from(Span::styled(
                    "  ...",
                    Style::default().fg(Color::Rgb(86, 95, 137)),
                )));
            }
        }

        // Details (if expanded)
        if request.show_details && !request.details.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Details:",
                Style::default().fg(Color::Rgb(86, 95, 137)),
            )));
            for detail in request.details.iter().take(5) {
                lines.push(Line::from(Span::styled(
                    format!("  • {}", detail),
                    Style::default().fg(Color::Rgb(169, 177, 214)),
                )));
            }
        }

        let content = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(content, area);
    }

    /// Render buttons
    fn render_buttons(&self, frame: &mut Frame, area: Rect, request: &ApprovalRequest) {
        let has_details = !request.details.is_empty();

        let mut buttons = vec![];

        // Approve button
        let approve_style = if self.selected == 0 {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        buttons.push(Span::styled(" [Y] Approve ", approve_style));
        buttons.push(Span::raw("  "));

        // Reject button
        let reject_style = if self.selected == 1 {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Red)
        };
        buttons.push(Span::styled(" [N] Reject ", reject_style));

        // Details button (if available)
        if has_details {
            buttons.push(Span::raw("  "));
            let details_style = if self.selected == 2 {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            };
            let details_label = if request.show_details {
                " [D] Hide Details "
            } else {
                " [D] Show Details "
            };
            buttons.push(Span::styled(details_label, details_style));
        }

        let button_line = Paragraph::new(Line::from(buttons)).alignment(Alignment::Center);
        frame.render_widget(button_line, area);
    }

    /// Get queue size
    pub fn queue_size(&self) -> usize {
        self.queue.len()
    }
}

impl Default for ApprovalOverlay {
    fn default() -> Self {
        Self::new()
    }
}

/// Action returned by approval overlay
#[derive(Debug, Clone, PartialEq)]
pub enum ApprovalAction {
    /// No action
    None,
    /// Approve request
    Approve(String),
    /// Reject request
    Reject(String),
    /// Approve all (enable auto-approve)
    ApproveAll(String),
}

/// Helper to create common approval requests
pub struct ApprovalBuilder {
    request: ApprovalRequest,
}

impl ApprovalBuilder {
    /// Create builder for shell command
    pub fn shell_command(command: impl Into<String>) -> Self {
        let cmd = command.into();
        let risk = if cmd.contains("rm ") || cmd.contains("sudo") {
            RiskLevel::High
        } else if cmd.contains("mv ") || cmd.contains("cp ") {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        Self {
            request: ApprovalRequest::new(
                uuid::Uuid::new_v4().to_string(),
                "Execute Shell Command",
            )
            .tool_name("bash")
            .arguments(&cmd)
            .risk_level(risk)
            .description("The AI wants to run a shell command"),
        }
    }

    /// Create builder for file write
    pub fn file_write(path: impl Into<String>) -> Self {
        let p = path.into();
        Self {
            request: ApprovalRequest::new(uuid::Uuid::new_v4().to_string(), "Write File")
                .tool_name("write")
                .arguments(&p)
                .risk_level(RiskLevel::Medium)
                .description("The AI wants to write to a file"),
        }
    }

    /// Create builder for file edit
    pub fn file_edit(path: impl Into<String>) -> Self {
        let p = path.into();
        Self {
            request: ApprovalRequest::new(uuid::Uuid::new_v4().to_string(), "Edit File")
                .tool_name("edit")
                .arguments(&p)
                .risk_level(RiskLevel::Low)
                .description("The AI wants to edit a file"),
        }
    }

    /// Add detail
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.request.details.push(detail.into());
        self
    }

    /// Set risk level
    pub fn risk(mut self, level: RiskLevel) -> Self {
        self.request.risk_level = level;
        self
    }

    /// Build the request
    pub fn build(self) -> ApprovalRequest {
        self.request
    }
}

/// Batch approval manager
pub struct BatchApproval {
    /// Requests to approve
    requests: Vec<ApprovalRequest>,
    /// Current index
    current: usize,
    /// Approved IDs
    approved: Vec<String>,
    /// Rejected IDs
    rejected: Vec<String>,
}

impl BatchApproval {
    /// Create new batch approval
    pub fn new(requests: Vec<ApprovalRequest>) -> Self {
        Self {
            requests,
            current: 0,
            approved: Vec::new(),
            rejected: Vec::new(),
        }
    }

    /// Get current request
    pub fn current(&self) -> Option<&ApprovalRequest> {
        self.requests.get(self.current)
    }

    /// Approve current
    pub fn approve_current(&mut self) -> bool {
        if let Some(req) = self.requests.get(self.current) {
            self.approved.push(req.id.clone());
            self.current += 1;
            true
        } else {
            false
        }
    }

    /// Reject current
    pub fn reject_current(&mut self) -> bool {
        if let Some(req) = self.requests.get(self.current) {
            self.rejected.push(req.id.clone());
            self.current += 1;
            true
        } else {
            false
        }
    }

    /// Approve all remaining
    pub fn approve_all(&mut self) {
        while self.current < self.requests.len() {
            self.approved.push(self.requests[self.current].id.clone());
            self.current += 1;
        }
    }

    /// Check if done
    pub fn is_done(&self) -> bool {
        self.current >= self.requests.len()
    }

    /// Get results
    pub fn results(&self) -> (&[String], &[String]) {
        (&self.approved, &self.rejected)
    }

    /// Get progress
    pub fn progress(&self) -> (usize, usize) {
        (self.current, self.requests.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // RiskLevel tests
    #[test]
    fn test_risk_level_color() {
        assert_eq!(RiskLevel::Low.color(), Color::Green);
        assert_eq!(RiskLevel::Medium.color(), Color::Yellow);
        assert_eq!(RiskLevel::High.color(), Color::Rgb(255, 165, 0));
        assert_eq!(RiskLevel::Critical.color(), Color::Red);
    }

    #[test]
    fn test_risk_level_label() {
        assert_eq!(RiskLevel::Low.label(), "LOW");
        assert_eq!(RiskLevel::Medium.label(), "MEDIUM");
        assert_eq!(RiskLevel::High.label(), "HIGH");
        assert_eq!(RiskLevel::Critical.label(), "CRITICAL");
    }

    #[test]
    fn test_risk_level_icon() {
        assert_eq!(RiskLevel::Low.icon(), "○");
        assert_eq!(RiskLevel::Medium.icon(), "◐");
        assert_eq!(RiskLevel::High.icon(), "●");
        assert_eq!(RiskLevel::Critical.icon(), "⚠");
    }

    // ApprovalRequest tests
    #[test]
    fn test_request_creation() {
        let request = ApprovalRequest::new("req-1", "Test Request");
        assert_eq!(request.id, "req-1");
        assert_eq!(request.title, "Test Request");
        assert!(request.description.is_empty());
        assert!(request.tool_name.is_empty());
        assert!(request.arguments.is_empty());
        assert_eq!(request.risk_level, RiskLevel::Medium);
        assert!(request.details.is_empty());
        assert!(!request.show_details);
    }

    #[test]
    fn test_request_description() {
        let request = ApprovalRequest::new("1", "Title").description("Test description");
        assert_eq!(request.description, "Test description");
    }

    #[test]
    fn test_request_tool_name() {
        let request = ApprovalRequest::new("1", "Title").tool_name("bash");
        assert_eq!(request.tool_name, "bash");
    }

    #[test]
    fn test_request_arguments() {
        let request = ApprovalRequest::new("1", "Title").arguments("echo hello");
        assert_eq!(request.arguments, "echo hello");
    }

    #[test]
    fn test_request_risk_level() {
        let request = ApprovalRequest::new("1", "Title").risk_level(RiskLevel::Critical);
        assert_eq!(request.risk_level, RiskLevel::Critical);
    }

    #[test]
    fn test_request_add_detail() {
        let request = ApprovalRequest::new("1", "Title")
            .add_detail("Detail 1")
            .add_detail("Detail 2");
        assert_eq!(request.details.len(), 2);
        assert_eq!(request.details[0], "Detail 1");
        assert_eq!(request.details[1], "Detail 2");
    }

    #[test]
    fn test_request_details() {
        let request =
            ApprovalRequest::new("1", "Title").details(vec!["A".to_string(), "B".to_string()]);
        assert_eq!(request.details.len(), 2);
    }

    // ApprovalOverlay tests
    #[test]
    fn test_overlay_creation() {
        let overlay = ApprovalOverlay::new();
        assert!(!overlay.is_visible());
        assert!(overlay.current_request().is_none());
        assert_eq!(overlay.queue_size(), 0);
    }

    #[test]
    fn test_overlay_show() {
        let mut overlay = ApprovalOverlay::new();
        let request = ApprovalRequest::new("req-1", "Test");

        overlay.show(request);
        assert!(overlay.is_visible());
        assert!(overlay.current_request().is_some());
        assert_eq!(overlay.current_request().unwrap().id, "req-1");
    }

    #[test]
    fn test_overlay_hide() {
        let mut overlay = ApprovalOverlay::new();
        overlay.show(ApprovalRequest::new("1", "Test"));

        overlay.hide();
        assert!(!overlay.is_visible());
        assert!(overlay.current_request().is_none());
    }

    #[test]
    fn test_overlay_queue() {
        let mut overlay = ApprovalOverlay::new();
        overlay.show(ApprovalRequest::new("1", "First"));
        overlay.show(ApprovalRequest::new("2", "Second"));

        assert_eq!(overlay.queue_size(), 1);
        assert_eq!(overlay.current_request().unwrap().id, "1");

        // Hide shows next from queue
        overlay.hide();
        assert!(overlay.is_visible());
        assert_eq!(overlay.current_request().unwrap().id, "2");
        assert_eq!(overlay.queue_size(), 0);
    }

    #[test]
    fn test_overlay_auto_approve() {
        let mut overlay = ApprovalOverlay::new();
        overlay.set_auto_approve(true);

        overlay.show(ApprovalRequest::new("1", "Test"));
        assert!(!overlay.is_visible());
    }

    #[test]
    fn test_overlay_handle_key_approve_y() {
        let mut overlay = ApprovalOverlay::new();
        overlay.show(ApprovalRequest::new("req-1", "Test"));

        let action = overlay.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty()));
        assert_eq!(action, ApprovalAction::Approve("req-1".to_string()));
        assert!(!overlay.is_visible());
    }

    #[test]
    fn test_overlay_handle_key_approve_enter() {
        let mut overlay = ApprovalOverlay::new();
        overlay.show(ApprovalRequest::new("req-1", "Test"));

        let action = overlay.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
        assert_eq!(action, ApprovalAction::Approve("req-1".to_string()));
    }

    #[test]
    fn test_overlay_handle_key_reject_n() {
        let mut overlay = ApprovalOverlay::new();
        overlay.show(ApprovalRequest::new("req-1", "Test"));

        let action = overlay.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::empty()));
        assert_eq!(action, ApprovalAction::Reject("req-1".to_string()));
        assert!(!overlay.is_visible());
    }

    #[test]
    fn test_overlay_handle_key_reject_esc() {
        let mut overlay = ApprovalOverlay::new();
        overlay.show(ApprovalRequest::new("req-1", "Test"));

        let action = overlay.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert_eq!(action, ApprovalAction::Reject("req-1".to_string()));
    }

    #[test]
    fn test_overlay_handle_key_toggle_details() {
        let mut overlay = ApprovalOverlay::new();
        let request = ApprovalRequest::new("1", "Test").add_detail("Detail");
        overlay.show(request);

        assert!(!overlay.current_request().unwrap().show_details);

        overlay.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::empty()));
        assert!(overlay.current_request().unwrap().show_details);

        overlay.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::empty()));
        assert!(!overlay.current_request().unwrap().show_details);
    }

    #[test]
    fn test_overlay_handle_key_navigation() {
        let mut overlay = ApprovalOverlay::new();
        let request = ApprovalRequest::new("1", "Test").add_detail("Detail");
        overlay.show(request);

        // Initially selected = 0 (Approve)
        overlay.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::empty()));
        // Now selected = 1 (Reject)

        overlay.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::empty()));
        // Now selected = 2 (Details)

        overlay.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::empty()));
        // Back to 1
    }

    #[test]
    fn test_overlay_handle_key_approve_all() {
        let mut overlay = ApprovalOverlay::new();
        overlay.show(ApprovalRequest::new("req-1", "Test"));

        let action = overlay.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
        assert_eq!(action, ApprovalAction::ApproveAll("req-1".to_string()));
        assert!(overlay.auto_approve);
    }

    #[test]
    fn test_overlay_handle_key_not_visible() {
        let mut overlay = ApprovalOverlay::new();
        let action = overlay.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty()));
        assert_eq!(action, ApprovalAction::None);
    }

    // ApprovalAction tests
    #[test]
    fn test_approval_action_enum() {
        let _none = ApprovalAction::None;
        let _approve = ApprovalAction::Approve("id".to_string());
        let _reject = ApprovalAction::Reject("id".to_string());
        let _approve_all = ApprovalAction::ApproveAll("id".to_string());
    }

    // ApprovalBuilder tests
    #[test]
    fn test_builder_shell_command() {
        let request = ApprovalBuilder::shell_command("echo hello").build();
        assert_eq!(request.tool_name, "bash");
        assert_eq!(request.arguments, "echo hello");
        assert_eq!(request.risk_level, RiskLevel::Low);
    }

    #[test]
    fn test_builder_shell_command_high_risk() {
        let request = ApprovalBuilder::shell_command("rm -rf /tmp/test").build();
        assert_eq!(request.risk_level, RiskLevel::High);

        let request = ApprovalBuilder::shell_command("sudo apt update").build();
        assert_eq!(request.risk_level, RiskLevel::High);
    }

    #[test]
    fn test_builder_shell_command_medium_risk() {
        let request = ApprovalBuilder::shell_command("mv file1 file2").build();
        assert_eq!(request.risk_level, RiskLevel::Medium);

        let request = ApprovalBuilder::shell_command("cp file1 file2").build();
        assert_eq!(request.risk_level, RiskLevel::Medium);
    }

    #[test]
    fn test_builder_file_write() {
        let request = ApprovalBuilder::file_write("/path/to/file").build();
        assert_eq!(request.tool_name, "write");
        assert_eq!(request.arguments, "/path/to/file");
        assert_eq!(request.risk_level, RiskLevel::Medium);
    }

    #[test]
    fn test_builder_file_edit() {
        let request = ApprovalBuilder::file_edit("/path/to/file").build();
        assert_eq!(request.tool_name, "edit");
        assert_eq!(request.arguments, "/path/to/file");
        assert_eq!(request.risk_level, RiskLevel::Low);
    }

    #[test]
    fn test_builder_detail() {
        let request = ApprovalBuilder::shell_command("ls")
            .detail("Detail 1")
            .detail("Detail 2")
            .build();
        assert_eq!(request.details.len(), 2);
    }

    #[test]
    fn test_builder_risk() {
        let request = ApprovalBuilder::shell_command("ls")
            .risk(RiskLevel::Critical)
            .build();
        assert_eq!(request.risk_level, RiskLevel::Critical);
    }

    // BatchApproval tests
    #[test]
    fn test_batch_creation() {
        let requests = vec![
            ApprovalRequest::new("1", "First"),
            ApprovalRequest::new("2", "Second"),
        ];
        let batch = BatchApproval::new(requests);

        assert!(!batch.is_done());
        assert_eq!(batch.progress(), (0, 2));
        assert!(batch.current().is_some());
        assert_eq!(batch.current().unwrap().id, "1");
    }

    #[test]
    fn test_batch_approve_current() {
        let requests = vec![
            ApprovalRequest::new("1", "First"),
            ApprovalRequest::new("2", "Second"),
        ];
        let mut batch = BatchApproval::new(requests);

        assert!(batch.approve_current());
        assert_eq!(batch.progress(), (1, 2));
        assert_eq!(batch.current().unwrap().id, "2");

        let (approved, rejected) = batch.results();
        assert_eq!(approved, &["1".to_string()]);
        assert!(rejected.is_empty());
    }

    #[test]
    fn test_batch_reject_current() {
        let requests = vec![
            ApprovalRequest::new("1", "First"),
            ApprovalRequest::new("2", "Second"),
        ];
        let mut batch = BatchApproval::new(requests);

        assert!(batch.reject_current());
        assert_eq!(batch.progress(), (1, 2));

        let (approved, rejected) = batch.results();
        assert!(approved.is_empty());
        assert_eq!(rejected, &["1".to_string()]);
    }

    #[test]
    fn test_batch_approve_all() {
        let requests = vec![
            ApprovalRequest::new("1", "First"),
            ApprovalRequest::new("2", "Second"),
            ApprovalRequest::new("3", "Third"),
        ];
        let mut batch = BatchApproval::new(requests);

        batch.approve_all();
        assert!(batch.is_done());

        let (approved, rejected) = batch.results();
        assert_eq!(approved.len(), 3);
        assert!(rejected.is_empty());
    }

    #[test]
    fn test_batch_is_done() {
        let requests = vec![ApprovalRequest::new("1", "First")];
        let mut batch = BatchApproval::new(requests);

        assert!(!batch.is_done());
        batch.approve_current();
        assert!(batch.is_done());
    }

    #[test]
    fn test_batch_empty() {
        let batch = BatchApproval::new(vec![]);
        assert!(batch.is_done());
        assert!(batch.current().is_none());
    }

    #[test]
    fn test_batch_mixed_approvals() {
        let requests = vec![
            ApprovalRequest::new("1", "First"),
            ApprovalRequest::new("2", "Second"),
            ApprovalRequest::new("3", "Third"),
        ];
        let mut batch = BatchApproval::new(requests);

        batch.approve_current(); // Approve 1
        batch.reject_current(); // Reject 2
        batch.approve_current(); // Approve 3

        let (approved, rejected) = batch.results();
        assert_eq!(approved, &["1".to_string(), "3".to_string()]);
        assert_eq!(rejected, &["2".to_string()]);
    }
}
