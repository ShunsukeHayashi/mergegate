//! Pure update logic for `AppState`.

use crate::app::state::AppState;
use crate::domain::actions::AppAction;

/// Apply an action to the application state.
pub fn reduce(state: &mut AppState, action: &AppAction) {
    match action {
        AppAction::Quit => state.should_quit = true,
        AppAction::CancelStreaming => state.is_streaming = false,
        AppAction::Resize { .. } => {}
        AppAction::Tick => {}
        AppAction::ToggleSidebar => {}
        AppAction::ToggleAgentMode => {}
        AppAction::SendMessage { .. } => {}
        AppAction::ExecuteCommand { .. } => {}
        AppAction::ApproveTool { .. } => {}
        AppAction::KeyPressed(_) => {}
    }
}
