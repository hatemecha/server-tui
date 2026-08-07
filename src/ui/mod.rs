pub mod components;
pub mod dashboard;
pub mod diagnostics;
pub mod dialogs;
pub mod help;
pub mod layout;
pub mod logs;
pub mod processes;
pub mod selection;
pub mod services;
pub mod settings;
pub mod status;
pub mod storage;
pub mod theme;
pub mod viewport;

use ratatui::Frame;

use crate::app::state::AppState;
use crate::app::Screen;
use crate::ui::layout::draw_shell;

pub fn draw(frame: &mut Frame<'_>, state: &AppState) {
    if state.too_small() {
        layout::draw_too_small(frame, state);
        return;
    }
    draw_shell(frame, state, |frame, area, state| match state.screen {
        Screen::Dashboard => dashboard::draw(frame, area, state),
        Screen::Processes => processes::draw(frame, area, state),
        Screen::Services => services::draw(frame, area, state),
        Screen::Logs => logs::draw(frame, area, state),
        Screen::Storage => storage::draw(frame, area, state),
        Screen::Diagnostics => diagnostics::draw(frame, area, state),
        Screen::Settings => settings::draw(frame, area, state),
    });
    if state.dialog.is_some() {
        dialogs::draw(frame, state);
    }
}
