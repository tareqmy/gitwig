//! Keystroke dispatch.
//!
//! `handle_key` reads `app.mode` and routes the keystroke to the
//! appropriate `App` method. Returns `false` when the user has asked to
//! quit, `true` otherwise.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Position;

use crate::app::{App, DetailSection, Mode};

/// Dispatch a key press. Returns `false` if the user requested quit.
pub fn handle_key(app: &mut App, key: KeyEvent, visible_count: usize) -> bool {
    let code = key.code;
    let detail_focus = app.detail_focus; // Copy before borrow in match
    match &app.mode {
        Mode::Normal => match code {
            KeyCode::Char('q') => return false,
            KeyCode::Down | KeyCode::Char('j') => app.move_down(visible_count),
            KeyCode::Up | KeyCode::Char('k') => app.move_up(),
            KeyCode::PageDown => app.page_down(visible_count),
            KeyCode::PageUp => app.page_up(visible_count),
            KeyCode::Char('a') => app.start_add(),
            KeyCode::Char('e') => app.start_edit(),
            KeyCode::Char('d') => app.request_delete(),
            KeyCode::Char('?') => app.open_help(),
            KeyCode::Char('r') => app.refresh_selected_status(),
            KeyCode::Enter => app.open_detail(),
            _ => {}
        },
        Mode::Adding => match code {
            KeyCode::Esc => app.cancel_input(),
            KeyCode::Enter => app.commit_add(),
            KeyCode::Backspace => app.input_backspace(),
            KeyCode::Char(c) => app.input_char(c),
            _ => {}
        },
        Mode::Editing => match code {
            KeyCode::Esc => app.cancel_input(),
            KeyCode::Enter => app.commit_edit(),
            KeyCode::Backspace => app.input_backspace(),
            KeyCode::Char(c) => app.input_char(c),
            _ => {}
        },
        Mode::ConfirmDelete => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_delete(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.close_dialog(),
            _ => {}
        },
        Mode::Help => match code {
            KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.close_dialog();
            }
            _ => {}
        },
        Mode::Detail => match code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => app.close_detail(),
            KeyCode::Char('o') => app.open_overview_popup(),
            KeyCode::Char('?') => app.open_detail_help(),
            KeyCode::Char('c') | KeyCode::Char('C') => app.start_commit(),
            KeyCode::Tab => app.cycle_detail_focus(),
            KeyCode::Up | KeyCode::Char('k') if detail_focus == DetailSection::Commits => {
                app.detail_commit_up()
            }
            KeyCode::Down | KeyCode::Char('j') if detail_focus == DetailSection::Commits => {
                app.detail_commit_down()
            }
            KeyCode::PageUp if detail_focus == DetailSection::Commits => {
                app.detail_commit_page_up(10)
            }
            KeyCode::PageDown if detail_focus == DetailSection::Commits => {
                app.detail_commit_page_down(10)
            }
            KeyCode::Up | KeyCode::Char('k')
                if detail_focus == DetailSection::Staged
                    || detail_focus == DetailSection::Unstaged =>
            {
                if app.is_uncommitted_selected() {
                    app.staging_file_up()
                } else {
                    app.detail_file_up()
                }
            }
            KeyCode::Down | KeyCode::Char('j')
                if detail_focus == DetailSection::Staged
                    || detail_focus == DetailSection::Unstaged =>
            {
                if app.is_uncommitted_selected() {
                    app.staging_file_down()
                } else {
                    app.detail_file_down()
                }
            }
            KeyCode::Enter
                if detail_focus == DetailSection::Staged && app.is_uncommitted_selected() =>
            {
                app.unstage_selected_file()
            }
            KeyCode::Enter
                if detail_focus == DetailSection::Unstaged && app.is_uncommitted_selected() =>
            {
                app.stage_selected_file()
            }
            KeyCode::Up | KeyCode::Char('k') if detail_focus == DetailSection::StagingDetails => {
                app.diff_scroll_up()
            }
            KeyCode::Down | KeyCode::Char('j') if detail_focus == DetailSection::StagingDetails => {
                app.diff_scroll_down()
            }
            KeyCode::PageUp if detail_focus == DetailSection::StagingDetails => {
                app.diff_scroll_page_up(10)
            }
            KeyCode::PageDown if detail_focus == DetailSection::StagingDetails => {
                app.diff_scroll_page_down(10)
            }
            KeyCode::Up | KeyCode::Char('k') if detail_focus == DetailSection::CommitDetails => {
                app.commit_details_scroll_up()
            }
            KeyCode::Down | KeyCode::Char('j') if detail_focus == DetailSection::CommitDetails => {
                app.commit_details_scroll_down()
            }
            _ => {}
        },
        Mode::DetailOverview => match code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Char('o') => {
                app.close_overview_popup();
            }
            _ => {}
        },
        Mode::DetailHelp => match code {
            KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.close_detail_help();
            }
            _ => {}
        },
        Mode::CommitInput => {
            if app.commit_editing {
                match code {
                    KeyCode::Esc => app.cancel_commit(),
                    KeyCode::Enter => app.input_char('\n'),
                    KeyCode::Char('c') | KeyCode::Char('C')
                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        app.commit_done_editing()
                    }
                    KeyCode::Backspace => app.input_backspace(),
                    KeyCode::Char(c) => app.input_char(c),
                    _ => {}
                }
            } else {
                match code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => app.cancel_commit(),
                    KeyCode::Char('c') | KeyCode::Char('C') => app.commit_git_changes(),
                    KeyCode::Char('e') | KeyCode::Char('E') => app.commit_start_editing(),
                    _ => {}
                }
            }
        }
    }
    true
}

/// Dispatch a mouse event.
///
/// Only left-button clicks on panel borders/content in `Mode::Detail` are
/// acted upon — all other events are silently ignored so scrolling and
/// selection still feel natural.
pub fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    // Only handle left-button press-down in detail mode.
    if !matches!(
        app.mode,
        Mode::Detail | Mode::DetailOverview | Mode::DetailHelp
    ) {
        return;
    }
    if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
        return;
    }

    let pos = Position {
        x: mouse.column,
        y: mouse.row,
    };
    let areas = &app.detail_areas;

    // Staged sub-panel (inside Staging Area left block) — check before bottom_left
    // so the more-specific sub-panels win.
    if let Some(rect) = areas.staged_sub {
        if rect.contains(pos) {
            if app.detail_focus != DetailSection::Staged {
                app.detail_focus = DetailSection::Staged;
                app.staging_file_selection = 0;
                app.diff_scroll = 0;
                app.refresh_staging_diff();
            }
            return;
        }
    }
    // Unstaged sub-panel.
    if let Some(rect) = areas.unstaged_sub {
        if rect.contains(pos) {
            if app.detail_focus != DetailSection::Unstaged {
                app.detail_focus = DetailSection::Unstaged;
                app.staging_file_selection = 0;
                app.diff_scroll = 0;
                app.refresh_staging_diff();
            }
            return;
        }
    }
    // Commit details sub-panel (inside Changed Files / Commit Details left block).
    if let Some(rect) = areas.commit_details {
        if rect.contains(pos) {
            if app.detail_focus != DetailSection::CommitDetails {
                app.detail_focus = DetailSection::CommitDetails;
            }
            return;
        }
    }

    // Bottom-left panel (Staging Area outer block or Changed Files).
    if let Some(rect) = areas.bottom_left {
        if rect.contains(pos) {
            // When sub-panels are not shown (real commit), treat the whole
            // left block as the Staged (Changed Files) focus.
            if app.detail_focus != DetailSection::Staged {
                app.detail_focus = DetailSection::Staged;
                app.diff_scroll = 0;
                if app.is_uncommitted_selected() {
                    app.staging_file_selection = 0;
                    app.refresh_staging_diff();
                } else {
                    app.refresh_file_diff();
                }
            }
            return;
        }
    }
    // Right panel (Diff / Staging Details).
    if let Some(rect) = areas.bottom_right {
        if rect.contains(pos) {
            if app.detail_focus != DetailSection::StagingDetails {
                app.detail_focus = DetailSection::StagingDetails;
                app.diff_scroll = 0;
            }
            return;
        }
    }
    // Commits panel (top).
    if let Some(rect) = areas.commits {
        if rect.contains(pos) && app.detail_focus != DetailSection::Commits {
            app.detail_focus = DetailSection::Commits;
        }
    }
}
