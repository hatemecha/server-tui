//! Reusable list/table viewport: keeps the selected row visible.

use ratatui::widgets::{ListState, TableState};

/// Viewport scrolling state for list-like screens.
#[derive(Debug, Clone, Copy, Default)]
pub struct ViewportState {
    /// Zero-based selected index.
    pub selected: usize,
    /// First visible row index.
    pub offset: usize,
}

impl ViewportState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clamp_to_len(&mut self, len: usize) {
        self.clamp_to_len_visible(len, 20);
    }

    /// Clamp selection using a real visible row count (prefer over `clamp_to_len`).
    pub fn clamp_to_len_visible(&mut self, len: usize, visible: usize) {
        if len == 0 {
            self.selected = 0;
            self.offset = 0;
            return;
        }
        if self.selected >= len {
            self.selected = len - 1;
        }
        self.ensure_visible(len, visible.max(1));
    }

    /// Ensure selected index stays within `[offset, offset + visible)`.
    pub fn ensure_visible(&mut self, len: usize, visible: usize) {
        if len == 0 {
            self.selected = 0;
            self.offset = 0;
            return;
        }
        let visible = visible.max(1);
        if self.selected >= len {
            self.selected = len - 1;
        }
        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset.saturating_add(visible) {
            self.offset = self.selected.saturating_add(1).saturating_sub(visible);
        }
        let max_offset = len.saturating_sub(visible);
        if self.offset > max_offset {
            self.offset = max_offset;
        }
    }

    pub fn set_selected(&mut self, idx: usize, len: usize, visible: usize) {
        if len == 0 {
            self.selected = 0;
            self.offset = 0;
            return;
        }
        self.selected = idx.min(len - 1);
        self.ensure_visible(len, visible);
    }

    pub fn move_up(&mut self, len: usize, visible: usize) {
        if len == 0 {
            return;
        }
        if self.selected > 0 {
            self.selected -= 1;
        }
        self.ensure_visible(len, visible);
    }

    pub fn move_down(&mut self, len: usize, visible: usize) {
        if len == 0 {
            return;
        }
        if self.selected + 1 < len {
            self.selected += 1;
        }
        self.ensure_visible(len, visible);
    }

    pub fn page_up(&mut self, len: usize, visible: usize) {
        if len == 0 {
            return;
        }
        let step = visible.max(1);
        self.selected = self.selected.saturating_sub(step);
        self.ensure_visible(len, visible);
    }

    pub fn page_down(&mut self, len: usize, visible: usize) {
        if len == 0 {
            return;
        }
        let step = visible.max(1);
        self.selected = (self.selected + step).min(len - 1);
        self.ensure_visible(len, visible);
    }

    pub fn home(&mut self, len: usize, visible: usize) {
        if len == 0 {
            return;
        }
        self.selected = 0;
        self.ensure_visible(len, visible);
    }

    pub fn end(&mut self, len: usize, visible: usize) {
        if len == 0 {
            return;
        }
        self.selected = len - 1;
        self.ensure_visible(len, visible);
    }

    /// Display like `37 / 184` (1-based selected) or `0 / 0` when empty.
    pub fn position_label(&self, len: usize) -> String {
        if len == 0 {
            "0 / 0".into()
        } else {
            format!("{} / {}", self.selected + 1, len)
        }
    }

    pub fn to_table_state(&self) -> TableState {
        let mut state = TableState::default();
        state.select(Some(self.selected));
        // ratatui TableState uses offset via set_offset in newer versions;
        // keep selected in sync; ViewportState.offset is authoritative for our ensure_visible.
        state
    }

    pub fn to_list_state(&self) -> ListState {
        let mut state = ListState::default();
        state.select(Some(self.selected));
        state
    }
}

/// Guess content rows from terminal height when a precise layout height is unknown.
pub fn content_rows_from_terminal(height: u16) -> usize {
    // Header(~3) + footer(~2) + borders(~2) → leave remaining for lists.
    height.saturating_sub(7).max(3) as usize
}

/// Apply viewport offset into TableState when the API supports it.
pub fn sync_table_state(vp: &ViewportState, state: &mut TableState) {
    state.select(Some(vp.selected));
    // TableState::offset exists in ratatui 0.29+.
    *state.offset_mut() = vp.offset;
}

pub fn sync_list_state(vp: &ViewportState, state: &mut ListState) {
    state.select(Some(vp.selected));
    *state.offset_mut() = vp.offset;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_visible_keeps_selection_in_window() {
        let mut vp = ViewportState {
            selected: 0,
            offset: 0,
        };
        vp.set_selected(25, 100, 10);
        assert_eq!(vp.selected, 25);
        assert!(vp.selected >= vp.offset);
        assert!(vp.selected < vp.offset + 10);
    }

    #[test]
    fn move_down_scrolls() {
        let mut vp = ViewportState::new();
        for _ in 0..15 {
            vp.move_down(50, 10);
        }
        assert_eq!(vp.selected, 15);
        assert_eq!(vp.offset, 6);
    }

    #[test]
    fn page_and_home_end() {
        let mut vp = ViewportState::new();
        vp.page_down(100, 10);
        assert_eq!(vp.selected, 10);
        vp.end(100, 10);
        assert_eq!(vp.selected, 99);
        assert_eq!(vp.offset, 90);
        vp.home(100, 10);
        assert_eq!(vp.selected, 0);
        assert_eq!(vp.offset, 0);
        vp.page_up(100, 10);
        assert_eq!(vp.selected, 0);
    }

    #[test]
    fn empty_list_safe() {
        let mut vp = ViewportState {
            selected: 5,
            offset: 2,
        };
        vp.clamp_to_len(0);
        assert_eq!(vp.selected, 0);
        assert_eq!(vp.offset, 0);
        assert_eq!(vp.position_label(0), "0 / 0");
    }

    #[test]
    fn position_label_one_based() {
        let vp = ViewportState {
            selected: 36,
            offset: 30,
        };
        assert_eq!(vp.position_label(184), "37 / 184");
    }
}
