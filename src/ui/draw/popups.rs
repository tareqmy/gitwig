//! Overlay popup drawing (jump, label picker, global search, scanners, fuzzy searchers, and non-git warning).

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::*;
use crate::app::App;

pub fn draw_repo_jump_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = crate::ui::layout::centered_rect(60, 50, area);
    f.render_widget(ratatui::widgets::Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Jump to Repository", primary_style().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input box
            Constraint::Min(1),    // Match list
            Constraint::Length(1), // Hint
        ])
        .split(inner);

    // 1. Draw input box
    let input_block =
        Block::default().borders(Borders::ALL).border_style(muted_style()).title(" Search Query ");
    let input_p = Paragraph::new(Line::from(vec![
        Span::raw("> "),
        Span::styled(app.input_buffer.clone(), primary_style()),
    ]))
    .block(input_block);
    f.render_widget(input_p, chunks[0]);

    // 2. Draw matches
    let matches = app.get_jump_matches();
    let list_items: Vec<ratatui::widgets::ListItem> = matches
        .iter()
        .enumerate()
        .map(|(i, (_, path, name))| {
            let style = if i == app.repo_jump_selection {
                accent_style().add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                primary_style()
            };
            ratatui::widgets::ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(name.clone(), style),
                Span::raw("   "),
                Span::styled(path.clone(), muted_style()),
            ]))
        })
        .collect();

    let mut list_state = ratatui::widgets::ListState::default();
    if !matches.is_empty() {
        list_state.select(Some(app.repo_jump_selection));
    }
    f.render_stateful_widget(ratatui::widgets::List::new(list_items), chunks[1], &mut list_state);

    // 3. Draw hint
    let hint = Line::from(vec![
        Span::styled("Type to filter  ", muted_style()),
        Span::styled("↑↓ navigate  ", muted_style()),
        Span::styled("Enter", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" jump  ", muted_style()),
        Span::styled("Esc", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" cancel", muted_style()),
    ]);
    f.render_widget(Paragraph::new(hint), chunks[2]);
}

pub fn draw_label_picker_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = crate::ui::layout::centered_rect(50, 50, area);
    f.render_widget(ratatui::widgets::Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(Style::default().fg(ACCENT()))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Filter by Label", primary_style().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input box
            Constraint::Min(1),    // Label list
            Constraint::Length(1), // Hint
        ])
        .split(inner);

    let input_block =
        Block::default().borders(Borders::ALL).border_style(muted_style()).title(" Search Labels ");
    let input_p = Paragraph::new(Line::from(vec![
        Span::raw("> "),
        Span::styled(app.input_buffer.clone(), primary_style()),
    ]))
    .block(input_block);
    f.render_widget(input_p, chunks[0]);

    let matches = app.get_label_matches();
    if matches.len() == 1 && app.input_buffer.is_empty() {
        // Only the "All repositories" row exists: nothing is labeled yet.
        let empty_hint = Paragraph::new(Line::from(Span::styled(
            "No labels yet — press l on a repository to add labels",
            muted_style(),
        )));
        f.render_widget(empty_hint, chunks[1]);
    } else {
        let active_marker = if app.config.compatibility_mode { "* " } else { "● " };
        let quick = app.quick_labels();
        let list_items: Vec<ratatui::widgets::ListItem> = matches
            .iter()
            .enumerate()
            .map(|(i, (label, count))| {
                let marker =
                    if *label == app.state.active_label_filter { active_marker } else { "  " };
                // Quick-label slots get their key so the picker teaches the shortcut.
                let slot_key = label
                    .as_deref()
                    .and_then(|l| quick.iter().position(|(q, _)| q == l))
                    .and_then(crate::keybindings::Action::home_label_slot)
                    .map(|a| app.keybindings.format_action_keys(a, app.config.compatibility_mode));
                let slot_text =
                    slot_key.map(|k| format!("{} ", k)).unwrap_or_else(|| "  ".to_string());
                let name = label.clone().unwrap_or_else(|| "All repositories".to_string());
                let style = if i == app.label_picker_selection {
                    accent_style().add_modifier(Modifier::BOLD | Modifier::REVERSED)
                } else if label.is_none() {
                    muted_style()
                } else {
                    primary_style()
                };
                let repos_word = if *count == 1 { "repo" } else { "repos" };
                ratatui::widgets::ListItem::new(Line::from(vec![
                    Span::styled(marker, accent_style()),
                    Span::styled(slot_text, accent_style().add_modifier(Modifier::BOLD)),
                    Span::styled(name, style),
                    Span::styled(format!("   {} {}", count, repos_word), muted_style()),
                ]))
            })
            .collect();

        let mut list_state = ratatui::widgets::ListState::default();
        if !matches.is_empty() {
            list_state.select(Some(app.label_picker_selection.min(matches.len() - 1)));
        }
        f.render_stateful_widget(
            ratatui::widgets::List::new(list_items),
            chunks[1],
            &mut list_state,
        );
    }

    let hint = Line::from(vec![
        Span::styled("Type to filter  ", muted_style()),
        Span::styled("↑↓ ", muted_style()),
        Span::styled("Enter", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" select  ", muted_style()),
        Span::styled("→", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" settings  ", muted_style()),
        Span::styled("Del", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" forget  ", muted_style()),
        Span::styled("Esc", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" cancel", muted_style()),
    ]);
    f.render_widget(Paragraph::new(hint), chunks[2]);
}

pub fn draw_global_search_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = crate::ui::layout::centered_rect(85, 75, area);
    f.render_widget(ratatui::widgets::Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Global Code Search", primary_style().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input box
            Constraint::Min(1),    // Search results list
            Constraint::Length(1), // Key hint
        ])
        .split(inner);

    // 1. Draw search query input box
    let input_border_style =
        if app.global_search_focus_input { Style::default().fg(ACCENT()) } else { muted_style() };
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(input_border_style)
        .title(" Search Query ");
    let input_p = Paragraph::new(Line::from(vec![
        Span::raw("> "),
        Span::styled(app.input_buffer.clone(), primary_style()),
    ]))
    .block(input_block);
    f.render_widget(input_p, chunks[0]);

    // 2. Draw results list or loading state
    let list_border_style =
        if !app.global_search_focus_input { Style::default().fg(ACCENT()) } else { muted_style() };
    let list_block =
        Block::default().borders(Borders::ALL).border_style(list_border_style).title(" Results ");

    if app.global_search_running {
        let loading_p = Paragraph::new(Line::from(vec![Span::styled(
            " Searching across all tracked repositories...",
            accent_style().add_modifier(Modifier::BOLD),
        )]))
        .block(list_block);
        f.render_widget(loading_p, chunks[1]);
    } else if app.global_search_results.is_empty() {
        let empty_msg = if app.global_search_query.is_empty() {
            "Type a query and press Enter to start searching."
        } else {
            "No matching string patterns found."
        };
        let empty_p = Paragraph::new(Line::from(vec![Span::styled(
            format!(" {}", empty_msg),
            muted_style(),
        )]))
        .block(list_block);
        f.render_widget(empty_p, chunks[1]);
    } else {
        let list_items: Vec<ratatui::widgets::ListItem> = app
            .global_search_results
            .iter()
            .enumerate()
            .map(|(i, res)| {
                let is_selected = i == app.global_search_selection;
                let row_style = if is_selected && !app.global_search_focus_input {
                    accent_style().add_modifier(Modifier::BOLD | Modifier::REVERSED)
                } else {
                    primary_style()
                };

                let row_line = Line::from(vec![
                    Span::styled(
                        format!("[{}] ", res.repo_name),
                        if is_selected && !app.global_search_focus_input {
                            row_style
                        } else {
                            Style::default().fg(ratatui::style::Color::Green)
                        },
                    ),
                    Span::styled(
                        format!("{}:{} ", res.file_rel_path, res.line_number),
                        if is_selected && !app.global_search_focus_input {
                            row_style
                        } else {
                            Style::default().fg(ACCENT())
                        },
                    ),
                    Span::styled(res.line_content.clone(), row_style),
                ]);
                ratatui::widgets::ListItem::new(row_line)
            })
            .collect();

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(app.global_search_selection));
        f.render_stateful_widget(
            ratatui::widgets::List::new(list_items).block(list_block),
            chunks[1],
            &mut list_state,
        );
    }

    // 3. Draw instructions/hint
    let hint = Line::from(vec![
        Span::styled("Tab", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" switch focus  ", muted_style()),
        Span::styled("Enter", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(
            if app.global_search_focus_input { " search  " } else { " open repo  " },
            muted_style(),
        ),
        Span::styled("↑↓ navigate  ", muted_style()),
        Span::styled("Esc", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" close", muted_style()),
    ]);
    f.render_widget(Paragraph::new(hint), chunks[2]);
}

pub fn draw_repo_scan_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = crate::ui::layout::centered_rect(70, 60, area);
    f.render_widget(ratatui::widgets::Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title_text = if app.repo_scan_active {
        format!(" Scan & Add Repository (Discovered {}) ", app.scanned_repos.len())
    } else {
        format!(" Scan & Add Repository (Found {} total) ", app.scanned_repos.len())
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(title_text, primary_style().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input box
            Constraint::Min(1),    // Match list
            Constraint::Length(1), // Progress / Hint
        ])
        .split(inner);

    // 1. Draw input box
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(muted_style())
        .title(" Search Filter / Manual Path ");
    let input_p = Paragraph::new(Line::from(vec![
        Span::raw("> "),
        Span::styled(app.input_buffer.clone(), primary_style()),
    ]))
    .block(input_block);
    f.render_widget(input_p, chunks[0]);

    // 2. Draw matches
    let matches = app.get_scan_matches();
    let list_items: Vec<ratatui::widgets::ListItem> = matches
        .iter()
        .enumerate()
        .map(|(i, (name, path))| {
            let style = if i == app.repo_scan_selection {
                accent_style().add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                primary_style()
            };
            ratatui::widgets::ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(name.clone(), style),
                Span::raw("   "),
                Span::styled(path.clone(), muted_style()),
            ]))
        })
        .collect();

    let mut list_state = ratatui::widgets::ListState::default();
    if !matches.is_empty() {
        list_state.select(Some(app.repo_scan_selection));
    }
    f.render_stateful_widget(ratatui::widgets::List::new(list_items), chunks[1], &mut list_state);

    // 3. Draw progress and hint
    let status_text = if app.repo_scan_active {
        let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let idx = (app.repo_scan_count / 10) % spinner_chars.len();
        format!("{} Scanning: {} folders visited...", spinner_chars[idx], app.repo_scan_count)
    } else {
        "✓ Scanning completed.".to_string()
    };

    let hint = Line::from(vec![
        Span::styled(status_text, Style::default().fg(ACCENT())),
        Span::raw("  |  "),
        Span::styled("Type to filter/manual path  ", muted_style()),
        Span::styled("↑↓ navigate  ", muted_style()),
        Span::styled("Enter", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" add  ", muted_style()),
        Span::styled("Esc", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" cancel", muted_style()),
    ]);
    f.render_widget(Paragraph::new(hint), chunks[2]);
}

pub fn draw_bulk_add_scan_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = crate::ui::layout::centered_rect(70, 60, area);
    f.render_widget(ratatui::widgets::Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title_text = if app.repo_scan_active {
        format!(" Scan & Bulk Add (Discovered {}) ", app.scanned_repos.len())
    } else {
        format!(" Scan & Bulk Add (Found {} total) ", app.scanned_repos.len())
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(title_text, primary_style().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input box
            Constraint::Min(1),    // Match list
            Constraint::Length(1), // Progress / Hint
        ])
        .split(inner);

    // 1. Draw input box
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(muted_style())
        .title(" Search Filter / Manual Path ");
    let input_p = Paragraph::new(Line::from(vec![
        Span::raw("> "),
        Span::styled(app.input_buffer.clone(), primary_style()),
    ]))
    .block(input_block);
    f.render_widget(input_p, chunks[0]);

    // 2. Draw matches
    let matches = app.get_scan_matches();
    let list_items: Vec<ratatui::widgets::ListItem> = matches
        .iter()
        .enumerate()
        .map(|(i, (name, path))| {
            let style = if i == app.repo_scan_selection {
                accent_style().add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                primary_style()
            };
            ratatui::widgets::ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(name.clone(), style),
                Span::raw("   "),
                Span::styled(path.clone(), muted_style()),
            ]))
        })
        .collect();

    let mut list_state = ratatui::widgets::ListState::default();
    if !matches.is_empty() {
        list_state.select(Some(app.repo_scan_selection));
    }
    f.render_stateful_widget(ratatui::widgets::List::new(list_items), chunks[1], &mut list_state);

    // 3. Draw progress and hint
    let status_text = if app.repo_scan_active {
        let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let idx = (app.repo_scan_count / 10) % spinner_chars.len();
        format!("{} Scanning: {} folders visited...", spinner_chars[idx], app.repo_scan_count)
    } else {
        "✓ Scanning completed.".to_string()
    };

    let hint = Line::from(vec![
        Span::styled(status_text, Style::default().fg(ACCENT())),
        Span::raw("  |  "),
        Span::styled("Type to filter/manual path  ", muted_style()),
        Span::styled("↑↓ navigate  ", muted_style()),
        Span::styled("Enter", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" bulk add  ", muted_style()),
        Span::styled("Esc", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" cancel", muted_style()),
    ]);
    f.render_widget(Paragraph::new(hint), chunks[2]);
}

pub fn draw_branch_search_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = crate::ui::layout::centered_rect(60, 50, area);
    f.render_widget(ratatui::widgets::Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Fuzzy Branch Search", primary_style().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input box
            Constraint::Min(1),    // Match list
            Constraint::Length(1), // Hint
        ])
        .split(inner);

    // 1. Draw input box
    let input_block =
        Block::default().borders(Borders::ALL).border_style(muted_style()).title(" Search Branch ");
    let input_p = Paragraph::new(Line::from(vec![
        Span::raw("> "),
        Span::styled(app.input_buffer.clone(), primary_style()),
    ]))
    .block(input_block);
    f.render_widget(input_p, chunks[0]);

    // 2. Draw matches
    let matches = app.get_branch_search_matches();
    let list_items: Vec<ratatui::widgets::ListItem> = matches
        .iter()
        .enumerate()
        .map(|(i, (name, is_remote))| {
            let style = if i == app.branch_search_selection {
                accent_style().add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                primary_style()
            };
            let type_str = if *is_remote { "[remote] " } else { "[local] " };
            let type_color =
                if *is_remote { ratatui::style::Color::Red } else { ratatui::style::Color::Green };
            ratatui::widgets::ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(type_str, Style::default().fg(type_color)),
                Span::styled(name.clone(), style),
            ]))
        })
        .collect();

    let mut list_state = ratatui::widgets::ListState::default();
    if !matches.is_empty() {
        list_state.select(Some(app.branch_search_selection));
    }
    f.render_stateful_widget(ratatui::widgets::List::new(list_items), chunks[1], &mut list_state);

    // 3. Draw hint
    let hint = Line::from(vec![
        Span::styled("Type to filter  ", muted_style()),
        Span::styled("↑↓ navigate  ", muted_style()),
        Span::styled("Enter", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" checkout  ", muted_style()),
        Span::styled("Esc", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" cancel", muted_style()),
    ]);
    f.render_widget(Paragraph::new(hint), chunks[2]);
}

pub fn draw_file_search_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = crate::ui::layout::centered_rect(80, 60, area);
    f.render_widget(ratatui::widgets::Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Fuzzy File Search", primary_style().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input box
            Constraint::Min(1),    // Match list
            Constraint::Length(1), // Hint
        ])
        .split(inner);

    // 1. Draw input box
    let input_block =
        Block::default().borders(Borders::ALL).border_style(muted_style()).title(" Search File ");
    let input_p = Paragraph::new(Line::from(vec![
        Span::raw("> "),
        Span::styled(app.input_buffer.clone(), primary_style()),
    ]))
    .block(input_block);
    f.render_widget(input_p, chunks[0]);

    // 2. Draw matches
    let matches = app.get_file_search_matches();
    let list_items: Vec<ratatui::widgets::ListItem> = matches
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let style = if i == app.file_search_selection {
                accent_style().add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                primary_style()
            };
            ratatui::widgets::ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(path.clone(), style),
            ]))
        })
        .collect();

    let mut list_state = ratatui::widgets::ListState::default();
    if !matches.is_empty() {
        list_state.select(Some(app.file_search_selection));
    }
    f.render_stateful_widget(ratatui::widgets::List::new(list_items), chunks[1], &mut list_state);

    // 3. Draw hint
    let hint = Line::from(vec![
        Span::styled("Type to filter  ", muted_style()),
        Span::styled("↑↓ navigate  ", muted_style()),
        Span::styled("Enter", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" select  ", muted_style()),
        Span::styled("Esc", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" cancel", muted_style()),
    ]);
    f.render_widget(Paragraph::new(hint), chunks[2]);
}

pub fn draw_commit_fuzzy_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = crate::ui::layout::centered_rect(80, 60, area);
    f.render_widget(ratatui::widgets::Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Fuzzy Commit Search", primary_style().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input box
            Constraint::Min(1),    // Match list
            Constraint::Length(1), // Hint
        ])
        .split(inner);

    // 1. Draw input box
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(muted_style())
        .title(" Search Commit (summary / author / SHA) ");
    let input_p = Paragraph::new(Line::from(vec![
        Span::raw("> "),
        Span::styled(app.input_buffer.clone(), primary_style()),
    ]))
    .block(input_block);
    f.render_widget(input_p, chunks[0]);

    // 2. Draw matches
    let matches = app.get_commit_fuzzy_matches();
    let list_items: Vec<ratatui::widgets::ListItem> = matches
        .iter()
        .enumerate()
        .map(|(i, (_, sha, author, summary))| {
            let style = if i == app.commit_search_selection {
                accent_style().add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                primary_style()
            };
            let short_sha = if sha.len() >= 7 { &sha[..7] } else { sha };
            ratatui::widgets::ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("{} ", short_sha),
                    Style::default().fg(ratatui::style::Color::Yellow),
                ),
                Span::styled(
                    format!("({}) ", author),
                    Style::default().fg(ratatui::style::Color::Green),
                ),
                Span::styled(summary.clone(), style),
            ]))
        })
        .collect();

    let mut list_state = ratatui::widgets::ListState::default();
    if !matches.is_empty() {
        list_state.select(Some(app.commit_search_selection));
    }
    f.render_stateful_widget(ratatui::widgets::List::new(list_items), chunks[1], &mut list_state);

    // 3. Draw hint
    let hint = Line::from(vec![
        Span::styled("Type to filter  ", muted_style()),
        Span::styled("↑↓ navigate  ", muted_style()),
        Span::styled("Enter", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" select & jump  ", muted_style()),
        Span::styled("Esc", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" cancel", muted_style()),
    ]);
    f.render_widget(Paragraph::new(hint), chunks[2]);
}

pub fn draw_tag_search_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = crate::ui::layout::centered_rect(60, 50, area);
    f.render_widget(ratatui::widgets::Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Fuzzy Tag Search", primary_style().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input box
            Constraint::Min(1),    // Match list
            Constraint::Length(1), // Hint
        ])
        .split(inner);

    // 1. Draw input box
    let input_block =
        Block::default().borders(Borders::ALL).border_style(muted_style()).title(" Search Tag ");
    let input_p = Paragraph::new(Line::from(vec![
        Span::raw("> "),
        Span::styled(app.input_buffer.clone(), primary_style()),
    ]))
    .block(input_block);
    f.render_widget(input_p, chunks[0]);

    // 2. Draw matches
    let matches = app.get_tag_search_matches();
    let list_items: Vec<ratatui::widgets::ListItem> = matches
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let style = if i == app.tag_search_selection {
                accent_style().add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                primary_style()
            };
            ratatui::widgets::ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(name.clone(), style),
            ]))
        })
        .collect();

    let mut list_state = ratatui::widgets::ListState::default();
    if !matches.is_empty() {
        list_state.select(Some(app.tag_search_selection));
    }
    f.render_stateful_widget(ratatui::widgets::List::new(list_items), chunks[1], &mut list_state);

    // 3. Draw hint
    let hint = Line::from(vec![
        Span::styled("Type to filter  ", muted_style()),
        Span::styled("↑↓ navigate  ", muted_style()),
        Span::styled("Enter", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" checkout  ", muted_style()),
        Span::styled("Esc", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" cancel", muted_style()),
    ]);
    f.render_widget(Paragraph::new(hint), chunks[2]);
}

pub(super) fn draw_not_git_repo_popup(f: &mut Frame, area: Rect, app: &App) {
    use crate::ui::layout::centered_rect_fixed;
    use ratatui::style::{Color, Modifier};
    use ratatui::text::Text;
    use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

    let popup_area = centered_rect_fixed(56, 9, area);
    f.render_widget(Clear, popup_area);

    let is_compat = app.config.compatibility_mode;
    let icon = if is_compat { "[!] " } else { "⚠  " };
    let border_style = Style::default().fg(DANGER());

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(
            ratatui::text::Line::from(vec![
                ratatui::text::Span::raw(" "),
                ratatui::text::Span::styled(
                    format!("{}Not a Git Repository", icon),
                    border_style.add_modifier(Modifier::BOLD),
                ),
                ratatui::text::Span::raw(" "),
            ])
            .alignment(ratatui::layout::Alignment::Center),
        );

    let path_label = app.get_selected_item().cloned().unwrap_or_else(|| "<unknown>".to_string());

    let body = Text::from(vec![
        ratatui::text::Line::from(""),
        ratatui::text::Line::from(vec![ratatui::text::Span::styled(
            path_label,
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )]),
        ratatui::text::Line::from(""),
        ratatui::text::Line::from(vec![ratatui::text::Span::styled(
            "This entry is not a git repository.",
            Style::default(),
        )]),
        ratatui::text::Line::from(vec![ratatui::text::Span::styled(
            "Please remove it from your list.",
            muted_style(),
        )]),
        ratatui::text::Line::from(""),
        ratatui::text::Line::from(vec![
            ratatui::text::Span::styled("Press ", muted_style()),
            ratatui::text::Span::styled(
                "Esc / Enter / q",
                accent_style().add_modifier(Modifier::BOLD),
            ),
            ratatui::text::Span::styled(" to dismiss", muted_style()),
        ]),
    ]);

    f.render_widget(block.clone(), popup_area);
    let inner = block.inner(popup_area);
    f.render_widget(
        Paragraph::new(body)
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: true }),
        inner,
    );
}
