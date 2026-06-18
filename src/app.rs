//! Application state and the main run loop.
//!
//! `App` owns everything mutable about a session: the current config, where
//! to persist it back to, where the cursor is, what mode we're in, and any
//! transient status message. The drawing layer (`ui`) reads `App` but never
//! mutates it. Key handling (`input`) calls back into `App` methods so the
//! state-mutation logic stays in one place.

use std::error::Error;
use std::path::PathBuf;

use crossterm::event::{self, Event};
use ratatui::Terminal;
use ratatui::layout::{Margin, Rect};

use crate::config::{Config, save_config};
use crate::input;
use crate::repo::{self, DiffLine, ItemDetail, ItemStatus};
use crate::ui;

/// Height of each item row inside the bordered list area.
/// Borders (top + bottom) take 2 rows; the remaining 2 inner rows hold
/// the item path and the branch name respectively.
pub const ITEM_HEIGHT: u16 = 4;

/// Height of the status/help bar at the bottom of the screen.
pub const STATUS_HEIGHT: u16 = 1;

/// Interaction modes for the item list. The mode dictates how keystrokes
/// are interpreted and what guidance the status bar shows.
pub enum Mode {
    /// Browsing the list. Navigation + add/edit/delete shortcuts are active.
    Normal,
    /// Typing a new item to append. Enter commits, Esc cancels.
    Adding,
    /// Typing replacement text for the selected item. Enter commits, Esc cancels.
    Editing,
    /// Asking the user to confirm deletion of the selected item.
    ConfirmDelete,
    /// Showing the full shortcut reference as a centered overlay.
    Help,
    /// Showing the full-screen detail view for the selected item.
    Detail,
    /// Showing the repo overview popup inside the detail view (triggered by 'o').
    DetailOverview,
    /// Showing the shortcut reference overlay inside the detail view (triggered by '?').
    DetailHelp,
}

/// Which panel in the detail view currently has keyboard focus.
/// Tab cycles through them in order.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DetailSection {
    Commits,
    Staged,
    Unstaged,
    StagingDetails,
}

impl DetailSection {
    /// Advance to the next section in the cycle.
    pub fn next(self) -> Self {
        match self {
            Self::Commits => Self::Staged,
            Self::Staged => Self::Unstaged,
            Self::Unstaged => Self::StagingDetails,
            Self::StagingDetails => Self::Commits,
        }
    }
}

/// All mutable session state.
pub struct App {
    pub config: Config,
    pub config_path: PathBuf,
    /// Filesystem classification per item, parallel to `config.items`.
    /// Recomputed on add/edit/delete so it never drifts from the list.
    pub statuses: Vec<ItemStatus>,
    pub selected_index: usize,
    pub scroll_top: usize,
    pub mode: Mode,
    pub input_buffer: String,
    pub status_message: Option<String>,
    /// Populated when entering `Mode::Detail`, cleared when leaving. The
    /// detail snapshot is taken once on open (not re-fetched per frame)
    /// so opening a slow repo only costs one git2 call.
    pub current_detail: Option<ItemDetail>,
    /// Which panel is focused inside the detail view.
    pub detail_focus: DetailSection,
    /// Selected row index inside the Commits panel (0 = top row).
    pub commit_selection: usize,
    /// Selected file index inside the Changed Files panel.
    pub file_selection: usize,
    /// Cached unified-diff lines for the currently selected file.
    pub file_diff: Vec<DiffLine>,
}

impl App {
    pub fn new(config: Config, config_path: PathBuf) -> Self {
        let statuses = config
            .items
            .iter()
            .map(|s| repo::inspect_summary(s))
            .collect();
        Self {
            config,
            config_path,
            statuses,
            selected_index: 0,
            scroll_top: 0,
            mode: Mode::Normal,
            input_buffer: String::new(),
            status_message: None,
            current_detail: None,
            detail_focus: DetailSection::Commits,
            commit_selection: 0,
            file_selection: 0,
            file_diff: Vec::new(),
        }
    }

    /// Ensure `selected_index` is a valid index into `config.items`.
    pub fn clamp_selection(&mut self) {
        if self.config.items.is_empty() {
            self.selected_index = 0;
        } else if self.selected_index >= self.config.items.len() {
            self.selected_index = self.config.items.len() - 1;
        }
    }

    /// Ensure the scroll window doesn't extend past the end of the list.
    pub fn clamp_scroll(&mut self, visible_count: usize) {
        let max_scroll = self.config.items.len().saturating_sub(visible_count);
        if self.scroll_top > max_scroll {
            self.scroll_top = max_scroll;
        }
    }

    pub fn move_down(&mut self, visible_count: usize) {
        if self.selected_index + 1 < self.config.items.len() {
            self.selected_index += 1;
            let bottom = self.scroll_top + visible_count;
            if self.selected_index >= bottom {
                self.scroll_top = self.scroll_top.saturating_add(1);
            }
        }
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            if self.selected_index < self.scroll_top {
                self.scroll_top = self.scroll_top.saturating_sub(1);
            }
        }
    }

    /// Jump the selection forward by one page (= `visible_count` items).
    /// The scroll window advances by the same amount so the newly selected
    /// item is always at the top of the visible area.
    pub fn page_down(&mut self, visible_count: usize) {
        let last = self.config.items.len().saturating_sub(1);
        self.selected_index = (self.selected_index + visible_count).min(last);
        // Align scroll so the selection lands at the top of the viewport,
        // then let clamp_scroll cap it at the list end.
        self.scroll_top = self.selected_index;
    }

    /// Jump the selection backward by one page (= `visible_count` items).
    pub fn page_up(&mut self, visible_count: usize) {
        self.selected_index = self.selected_index.saturating_sub(visible_count);
        self.scroll_top = self.selected_index;
    }

    pub fn start_add(&mut self) {
        self.input_buffer.clear();
        self.mode = Mode::Adding;
    }

    pub fn start_edit(&mut self) {
        if let Some(current) = self.config.items.get(self.selected_index) {
            self.input_buffer = current.clone();
            self.mode = Mode::Editing;
        }
    }

    pub fn request_delete(&mut self) {
        if !self.config.items.is_empty() {
            self.mode = Mode::ConfirmDelete;
        }
    }

    pub fn open_help(&mut self) {
        self.mode = Mode::Help;
    }

    /// Re-runs the cheap filesystem inspection for the selected item and
    /// updates its status indicator. Surfaces a transient "Refreshed" /
    /// "Refresh failed" message in the status bar so the user knows the
    /// keystroke landed (the indicator alone may not visibly change).
    pub fn refresh_selected_status(&mut self) {
        let Some(item) = self.config.items.get(self.selected_index) else {
            return;
        };
        let new_status = repo::inspect_summary(item);
        if let Some(slot) = self.statuses.get_mut(self.selected_index) {
            *slot = new_status;
        }
        self.status_message = Some("Refreshed".to_string());
    }

    /// Snapshot the selected item's filesystem/git state and enter the
    /// Detail view. The snapshot is held in `current_detail` for as long
    /// as the view is open; closing clears it.
    pub fn open_detail(&mut self) {
        if let Some(item) = self.config.items.get(self.selected_index) {
            self.current_detail = Some(repo::inspect_detail(item));
            self.detail_focus = DetailSection::Commits;
            self.commit_selection = 0;
            self.file_selection = 0;
            self.file_diff.clear();
            self.mode = Mode::Detail;
            self.refresh_file_diff();
        }
    }

    /// Advance focus to the next detail panel (Tab key).
    pub fn cycle_detail_focus(&mut self) {
        self.detail_focus = self.detail_focus.next();
        if !self.is_uncommitted_selected() && self.detail_focus == DetailSection::Unstaged {
            self.detail_focus = self.detail_focus.next();
        }
        // Pre-load the diff when landing on the Changed Files panel.
        if matches!(
            self.detail_focus,
            DetailSection::Staged | DetailSection::Unstaged
        ) {
            self.refresh_file_diff();
        }
    }

    /// Move commit selection up one row.
    pub fn detail_commit_up(&mut self) {
        self.commit_selection = self.commit_selection.saturating_sub(1);
        self.file_selection = 0;
        self.refresh_file_diff();
    }

    /// Move commit selection down one row, clamped to the last visible row.
    pub fn detail_commit_down(&mut self) {
        let total = self.commit_total();
        if total > 0 && self.commit_selection + 1 < total {
            self.commit_selection += 1;
        }
        self.file_selection = 0;
        self.refresh_file_diff();
    }

    /// Jump commit selection up by `page` rows.
    pub fn detail_commit_page_up(&mut self, page: usize) {
        self.commit_selection = self.commit_selection.saturating_sub(page);
        self.file_selection = 0;
        self.refresh_file_diff();
    }

    /// Jump commit selection down by `page` rows, clamped to the last row.
    pub fn detail_commit_page_down(&mut self, page: usize) {
        let total = self.commit_total();
        if total > 0 {
            self.commit_selection = (self.commit_selection + page).min(total - 1);
        }
        self.file_selection = 0;
        self.refresh_file_diff();
    }

    /// Move file selection up one row in the Changed Files panel.
    pub fn detail_file_up(&mut self) {
        self.file_selection = self.file_selection.saturating_sub(1);
        self.refresh_file_diff();
    }

    /// Move file selection down one row in the Changed Files panel.
    pub fn detail_file_down(&mut self) {
        let total = self.file_total();
        if total > 0 && self.file_selection + 1 < total {
            self.file_selection += 1;
        }
        self.refresh_file_diff();
    }

    /// Total number of rows in the Commits panel (dirty row + real commits).
    fn commit_total(&self) -> usize {
        match &self.current_detail {
            Some(ItemDetail::Repo { info, .. }) => {
                let dirty = !info.changes.staged.is_empty()
                    || !info.changes.unstaged.is_empty()
                    || !info.changes.untracked.is_empty()
                    || !info.changes.conflicted.is_empty();
                info.commits.len() + usize::from(dirty)
            }
            _ => 0,
        }
    }

    /// Total files in the currently-selected commit's Changed Files panel.
    fn file_total(&self) -> usize {
        match &self.current_detail {
            Some(ItemDetail::Repo { info, .. }) => {
                let dirty = !info.changes.staged.is_empty()
                    || !info.changes.unstaged.is_empty()
                    || !info.changes.untracked.is_empty()
                    || !info.changes.conflicted.is_empty();
                // Uncommitted row (staging area) has no file list.
                if dirty && self.commit_selection == 0 {
                    return 0;
                }
                let idx = if dirty {
                    self.commit_selection.saturating_sub(1)
                } else {
                    self.commit_selection
                };
                info.commits.get(idx).map(|c| c.files.len()).unwrap_or(0)
            }
            _ => 0,
        }
    }

    pub fn is_uncommitted_selected(&self) -> bool {
        match &self.current_detail {
            Some(ItemDetail::Repo { info, .. }) => {
                let dirty = !info.changes.staged.is_empty()
                    || !info.changes.unstaged.is_empty()
                    || !info.changes.untracked.is_empty()
                    || !info.changes.conflicted.is_empty();
                dirty && self.commit_selection == 0
            }
            _ => false,
        }
    }

    fn current_diff_params(&self) -> Option<(PathBuf, String, String)> {
        match &self.current_detail {
            Some(ItemDetail::Repo { resolved, info }) => {
                let dirty = !info.changes.staged.is_empty()
                    || !info.changes.unstaged.is_empty()
                    || !info.changes.untracked.is_empty()
                    || !info.changes.conflicted.is_empty();
                if dirty && self.commit_selection == 0 {
                    return None;
                }
                let commit_idx = if dirty {
                    self.commit_selection.saturating_sub(1)
                } else {
                    self.commit_selection
                };
                let commit = info.commits.get(commit_idx)?;
                let file = commit.files.get(self.file_selection)?;
                Some((resolved.clone(), commit.oid.clone(), file.path.clone()))
            }
            _ => None,
        }
    }

    pub fn refresh_file_diff(&mut self) {
        if let Some((repo_path, commit_oid, file_path)) = self.current_diff_params() {
            self.file_diff = repo::get_commit_file_diff(&repo_path, &commit_oid, &file_path);
        } else {
            self.file_diff.clear();
        }
    }

    pub fn close_detail(&mut self) {
        self.current_detail = None;
        self.mode = Mode::Normal;
    }

    /// Opens the repo overview popup while staying in the detail view.
    pub fn open_overview_popup(&mut self) {
        self.mode = Mode::DetailOverview;
    }

    /// Closes the overview popup and returns to the normal detail view.
    pub fn close_overview_popup(&mut self) {
        self.mode = Mode::Detail;
    }

    /// Opens the shortcut help overlay inside the detail view.
    pub fn open_detail_help(&mut self) {
        self.mode = Mode::DetailHelp;
    }

    /// Closes the detail help overlay and returns to the normal detail view.
    pub fn close_detail_help(&mut self) {
        self.mode = Mode::Detail;
    }

    pub fn cancel_input(&mut self) {
        self.input_buffer.clear();
        self.mode = Mode::Normal;
    }

    pub fn input_char(&mut self, c: char) {
        self.input_buffer.push(c);
    }

    pub fn input_backspace(&mut self) {
        self.input_buffer.pop();
    }

    pub fn commit_add(&mut self) {
        let trimmed = self.input_buffer.trim().to_string();
        if !trimmed.is_empty() {
            self.statuses.push(repo::inspect_summary(&trimmed));
            self.config.items.push(trimmed);
            self.selected_index = self.config.items.len() - 1;
            self.persist("Saved");
        }
        self.input_buffer.clear();
        self.mode = Mode::Normal;
    }

    pub fn commit_edit(&mut self) {
        let trimmed = self.input_buffer.trim().to_string();
        if !trimmed.is_empty()
            && let Some(slot) = self.config.items.get_mut(self.selected_index)
        {
            *slot = trimmed.clone();
            if let Some(slot) = self.statuses.get_mut(self.selected_index) {
                *slot = repo::inspect_summary(&trimmed);
            }
            self.persist("Saved");
        }
        self.input_buffer.clear();
        self.mode = Mode::Normal;
    }

    pub fn confirm_delete(&mut self) {
        if self.selected_index < self.config.items.len() {
            self.config.items.remove(self.selected_index);
            if self.selected_index < self.statuses.len() {
                self.statuses.remove(self.selected_index);
            }
            self.persist("Deleted");
        }
        self.mode = Mode::Normal;
    }

    pub fn close_dialog(&mut self) {
        self.mode = Mode::Normal;
    }

    /// Persists `self.config` and records a status message (success or
    /// the save error) for the next render.
    fn persist(&mut self, success_msg: &str) {
        self.status_message = match save_config(&self.config, &self.config_path) {
            Ok(()) => Some(success_msg.to_string()),
            Err(e) => Some(format!("Save failed: {}", e)),
        };
    }
}

/// Main event loop: compute layout, draw, poll input, repeat.
pub fn run<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
) -> Result<(), Box<dyn Error>>
where
    <B as ratatui::backend::Backend>::Error: 'static,
{
    loop {
        app.clamp_selection();

        let size = terminal.size()?;
        let area = Rect::new(0, 0, size.width, size.height);
        let inner_area = area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        });

        let available_height = inner_area.height.saturating_sub(STATUS_HEIGHT);
        let visible_count =
            (available_height / ITEM_HEIGHT).min(app.config.items.len() as u16) as usize;
        app.clamp_scroll(visible_count);

        terminal.draw(|f| ui::draw(f, &app, area, inner_area, visible_count))?;

        // Transient feedback disappears after one frame.
        app.status_message = None;

        if event::poll(std::time::Duration::from_millis(
            app.config.poll_interval_ms,
        ))? && let Event::Key(key) = event::read()?
            && !input::handle_key(&mut app, key.code, visible_count)
        {
            return Ok(());
        }
    }
}
