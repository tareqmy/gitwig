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
use crate::ui;

/// Height of each item row inside the bordered list area.
pub const ITEM_HEIGHT: u16 = 3;

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
}

/// All mutable session state.
pub struct App {
    pub config: Config,
    pub config_path: PathBuf,
    pub selected_index: usize,
    pub scroll_top: usize,
    pub mode: Mode,
    pub input_buffer: String,
    pub status_message: Option<String>,
}

impl App {
    pub fn new(config: Config, config_path: PathBuf) -> Self {
        Self {
            config,
            config_path,
            selected_index: 0,
            scroll_top: 0,
            mode: Mode::Normal,
            input_buffer: String::new(),
            status_message: None,
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
            *slot = trimmed;
            self.persist("Saved");
        }
        self.input_buffer.clear();
        self.mode = Mode::Normal;
    }

    pub fn confirm_delete(&mut self) {
        if self.selected_index < self.config.items.len() {
            self.config.items.remove(self.selected_index);
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

        if event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && !input::handle_key(&mut app, key.code, visible_count)
        {
            return Ok(());
        }
    }
}
