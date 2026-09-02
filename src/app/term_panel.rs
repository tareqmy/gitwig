//! Embedded terminal panel state transitions and geometry.
//!
//! The panel is global: it renders above the status bar in every base mode.
//! A single shell session is kept alive across hide/show; it ends when the
//! shell exits or the app quits.

use ratatui::layout::Rect;

use super::App;
use crate::repo;

impl App {
    /// Open (and focus) the terminal panel, spawning the shell on first use
    /// in the selected repository's directory. A live session is reused
    /// regardless of the current selection.
    pub fn open_terminal_panel(&mut self) {
        if self.terminal_panel.session.is_none() {
            let Some(path) = self.terminal_target_path() else {
                self.status_message = Some("No repository selected".to_string());
                return;
            };
            let label = path.file_name().and_then(|n| n.to_str()).unwrap_or("shell").to_string();
            let (rows, cols) = self.terminal_grid_estimate();
            match crate::terminal_session::TerminalSession::spawn(&path, label, rows, cols) {
                Ok(session) => self.terminal_panel.session = Some(session),
                Err(e) => {
                    self.set_error(e);
                    return;
                }
            }
        }
        self.terminal_panel.visible = true;
        self.terminal_focused = true;
    }

    /// Hide the panel; the shell session keeps running in the background.
    pub fn hide_terminal_panel(&mut self) {
        self.terminal_panel.visible = false;
        self.terminal_focused = false;
    }

    /// Toggle from anywhere: show+focus when hidden, hide when visible.
    pub fn toggle_terminal_panel(&mut self) {
        if self.terminal_panel.visible {
            self.hide_terminal_panel();
        } else {
            self.open_terminal_panel();
        }
    }

    /// Drop the session (killing the shell if needed) and hide the panel.
    pub fn close_terminal_session(&mut self) {
        self.terminal_panel.session = None;
        self.hide_terminal_panel();
    }

    /// Directory the shell starts in: the detail view's repo when one is
    /// open, otherwise the home selection.
    fn terminal_target_path(&self) -> Option<std::path::PathBuf> {
        if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail {
            return Some(resolved.clone());
        }
        self.get_selected_item().map(|item| repo::expand_tilde(item))
    }
}

impl App {
    /// Outer panel height (including borders) actually used this frame;
    /// 0 when the panel is hidden. Keeps at least 5 rows of content visible.
    pub fn terminal_panel_outer_height(&self, inner_area_height: u16) -> u16 {
        if !self.terminal_panel.visible {
            return 0;
        }
        let budget = inner_area_height.saturating_sub(self.status_height()).saturating_sub(5);
        self.terminal_panel.height.clamp(5, budget.max(5))
    }

    /// PTY grid (rows, cols) for the panel's inner rect this frame; `None`
    /// when the panel is hidden.
    pub fn terminal_grid_size(&self, inner_area: Rect) -> Option<(u16, u16)> {
        let outer = self.terminal_panel_outer_height(inner_area.height);
        if outer == 0 {
            return None;
        }
        Some((outer.saturating_sub(2), inner_area.width.saturating_sub(2)))
    }

    /// Best-effort initial grid before the first draw; corrected by the
    /// per-frame resize immediately afterwards.
    fn terminal_grid_estimate(&self) -> (u16, u16) {
        let (width, height) = crossterm::terminal::size().unwrap_or((80, 24));
        let inner = Rect::new(0, 0, width.saturating_sub(2), height.saturating_sub(2));
        // The panel may be closed at this instant; compute with it open.
        let budget = inner.height.saturating_sub(self.status_height()).saturating_sub(5);
        let outer = self.terminal_panel.height.clamp(5, budget.max(5));
        (outer.saturating_sub(2), inner.width.saturating_sub(2))
    }
}
