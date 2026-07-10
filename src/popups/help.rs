use crate::app::{App, Mode};
use crate::keybindings::Action;
use crate::repo::RemoteInfo;
use crate::ui::layout::{centered_rect, centered_rect_fixed};
use crate::ui::style::{
    ACCENT, CARD_BORDER, DANGER, SUCCESS, WARNING, accent_style, muted_style, parse_color,
    primary_style,
};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, Gauge, List, ListItem, ListState, Padding, Paragraph, Wrap,
};

fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line.push_str(word);
        } else if current_line.chars().count() + 1 + word.chars().count() <= max_width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

pub fn get_help_lines(app: &App, usable_width: usize) -> Vec<Line<'_>> {
    let mut lines = Vec::new();
    let is_compat = app.config.compatibility_mode;

    let format_key = |k: &str| {
        let mut key = k.to_string();
        if is_compat {
            key = key
                .replace("↑", "^")
                .replace("↓", "v")
                .replace("⇟", "PgDn")
                .replace("⇞", "PgUp")
                .replace("↵", "Enter")
                .replace("→", ">")
                .replace("⎋", "Esc")
                .replace("⌫", "Backspace")
                .replace("⇥", "Tab")
                .replace("⇧⇥", "Shift+Tab")
                .replace("⇧", "Shift+");
        }
        key
    };

    let kb = &app.keybindings;

    let help_key = kb.format_action_keys(Action::Help, is_compat);
    let about_key = kb.format_action_keys(Action::HomeAbout, is_compat);
    let legend_key = kb.format_action_keys(Action::HomeSymbolsHelp, is_compat);
    let close_key = kb.format_action_keys(Action::CloseDetail, is_compat);
    let quit_key = kb.format_action_keys(Action::Close, is_compat);

    let add_key = kb.format_action_keys(Action::HomeAddRepo, is_compat);
    let bulk_add_key = kb.format_action_keys(Action::HomeBulkAdd, is_compat);
    let import_key = kb.format_action_keys(Action::HomeImportRepo, is_compat);
    let edit_key = kb.format_action_keys(Action::HomeEditRepo, is_compat);
    let delete_key = kb.format_action_keys(Action::HomeDeleteRepo, is_compat);
    let search_key = kb.format_action_keys(Action::HomeSearchRepo, is_compat);
    let refresh_key = kb.format_action_keys(Action::HomeRefresh, is_compat);
    let sort_key = format!(
        "{}/{}",
        kb.format_action_keys(Action::HomeCycleSort, is_compat),
        kb.format_action_keys(Action::HomeToggleSortReverse, is_compat)
    );
    let compact_key = kb.format_action_keys(Action::HomeToggleCompactView, is_compat);
    let pin_key = kb.format_action_keys(Action::HomeTogglePin, is_compat);
    let settings_key = kb.format_action_keys(Action::HomeOpenSettings, is_compat);
    let labels_key = kb.format_action_keys(Action::HomeEditLabels, is_compat);
    let debug_key = kb.format_action_keys(Action::HomeOpenDebugLogs, is_compat);
    let update_key = kb.format_action_keys(Action::HomeCheckUpdate, is_compat);

    let open_detail_key = kb.format_action_keys(Action::HomeOpenDetail, is_compat);

    // New actions
    let terminal_key = kb.format_action_keys(Action::HomeOpenTerminal, is_compat);
    let star_key = kb.format_action_keys(Action::HomeToggleStar, is_compat);
    let yank_key = kb.format_action_keys(Action::HomeYankPath, is_compat);
    let jump_picker_key = kb.format_action_keys(Action::HomeJumpPicker, is_compat);
    let fetch_all_key = kb.format_action_keys(Action::HomeFetchAll, is_compat);
    let select_key = kb.format_action_keys(Action::HomeSelect, is_compat);
    let global_search_key = kb.format_action_keys(Action::HomeGlobalSearch, is_compat);

    let make_cat = |title: &'static str,
                    items: Vec<(&str, &'static str)>|
     -> (&str, Vec<(String, &'static str)>) {
        let mapped = items.into_iter().map(|(k, d)| (k.to_string(), d)).collect();
        (title, mapped)
    };

    let mut categories: Vec<(&str, Vec<(String, &str)>)> = vec![
        (
            "Global & Navigation",
            vec![
                ("↑ [Up] / k".to_string(), "Move selection up / scroll up"),
                ("↓ [Down] / j".to_string(), "Move selection down / scroll down"),
                ("⇞ [PgUp]".to_string(), "Jump one page up"),
                ("⇟ [PgDn]".to_string(), "Jump one page down"),
                ("Home".to_string(), "Go to top / scroll to top"),
                ("End".to_string(), "Go to bottom / scroll to bottom"),
                (
                    close_key,
                    "Cancel input, close dialog, clear search, cancel selections, or leave detail view",
                ),
                (help_key, "Toggle this help overlay"),
                (about_key, "Show about popup / creator profile"),
                (legend_key, "Show signs & symbols legend popup"),
                (quit_key, "Quit application"),
            ],
        ),
        (
            "Main List Operations",
            vec![
                (open_detail_key, "Open Detail view for selected repository"),
                (add_key, "Add a new repository"),
                (bulk_add_key, "Bulk add folders in a directory"),
                (import_key, "Import remote repository"),
                (edit_key, "Edit selected item"),
                (delete_key, "Delete selected item"),
                (search_key, "Enter repository search mode"),
                (global_search_key, "Open global code search popup overlay"),
                (refresh_key, "Refresh status of selected item"),
                (fetch_all_key, "Bulk fetch all tracked repositories concurrently"),
                (sort_key, "Cycle sorting mode / Toggle reverse sorting"),
                (compact_key, "Toggle between standard cards and compact 1-row view"),
                (pin_key, "Toggle pin status of selected item"),
                (star_key, "Toggle Favorite / Star status of selected item"),
                (select_key, "Toggle selection of item for batch operations"),
                (yank_key, "Yank absolute path of selected item to clipboard"),
                (jump_picker_key, "Open fuzzy Jump-to-Repo picker overlay"),
                (
                    "← / → / Space / Enter".to_string(),
                    "Collapse/expand label group (on group header)",
                ),
                ("g".to_string(), "Launch preferred Git client for selected repository"),
                (terminal_key, "Spawn a new shell (Terminal) in the selected repository"),
                (settings_key, "Open options/settings page"),
                (labels_key, "Edit labels of selected item"),
                (debug_key, "Open debug logs panel"),
                (update_key, "Check for application updates manually"),
                ("⌫ [Backspace]".to_string(), "Erase character while typing"),
            ],
        ),
    ];


    categories.push(make_cat(
        "Debug Logs Panel",
        vec![
            ("Esc / q", "Exit debug logs panel"),
            ("c / C / x", "Clear all debug logs"),
            ("/", "Fuzzy search/filter debug logs"),
            ("Enter", "Focus/lock list scrolling"),
            ("↑ / ↓ / j / k", "Scroll log entries"),
        ],
    ));

    categories.push(make_cat(
        "Mouse Interactions",
        vec![
            ("Left-Click", "Focus clicked panel / change tab (mouse support)"),
            ("Left-Click+Drag", "Drag boundaries to resize split panels"),
        ],
    ));

    // Find max key width for aligned display
    let mut max_key_width = 0;
    for (_, keys) in &categories {
        for (key, _) in keys {
            let width = format_key(key).chars().count();
            if width > max_key_width {
                max_key_width = width;
            }
        }
    }

    let desc_width = usable_width.saturating_sub(4 + max_key_width + 3);

    // Render Categories
    for (cat_title, keys) in categories {
        lines.push(Line::from(""));

        let title_style = if is_compat {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            primary_style().add_modifier(Modifier::BOLD)
        };

        let prefix = if is_compat { "=== " } else { "■ " };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(prefix, if is_compat { Style::default() } else { accent_style() }),
            Span::styled(cat_title.to_uppercase(), title_style),
        ]));

        for (key, desc) in keys {
            let k_str = format_key(&key);
            let padded_key = format!("{:>width$}", k_str, width = max_key_width);
            let desc_lines = wrap_text(desc, desc_width);
            for (idx, desc_line) in desc_lines.into_iter().enumerate() {
                if idx == 0 {
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(padded_key.clone(), accent_style()),
                        Span::raw("   "),
                        Span::raw(desc_line),
                    ]));
                } else {
                    let indent = " ".repeat(4 + max_key_width + 3);
                    lines.push(Line::from(vec![Span::raw(indent), Span::raw(desc_line)]));
                }
            }
        }
    }

    lines
}

pub fn get_help_lines_len(app: &App, width: u16) -> usize {
    let popup_width = (width * 80) / 100;
    let usable_width = popup_width.saturating_sub(4) as usize;
    get_help_lines(app, usable_width).len()
}

pub fn draw_help_overlay(f: &mut Frame, app: &App, area: Rect, scroll: usize) {
    let popup_area = centered_rect(80, 70, area);
    let usable_width = popup_area.width.saturating_sub(4) as usize;

    let lines = get_help_lines(app, usable_width);

    let help_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(Style::default().fg(ACCENT()))
        .title(
            Line::from(vec![
                Span::raw(" "),
                Span::styled("Shortcuts & Legend", accent_style()),
                Span::raw(" "),
            ])
            .alignment(Alignment::Left),
        )
        .padding(Padding::horizontal(1));

    let inner_height = popup_area.height.saturating_sub(2) as usize;
    let max_scroll = lines.len().saturating_sub(inner_height);
    let scroll = scroll.min(max_scroll);

    let lines_len = lines.len();
    let help = Paragraph::new(lines).block(help_block).scroll((scroll as u16, 0));

    // Clear wipes the underlying cells so the list doesn't bleed through.
    f.render_widget(Clear, popup_area);
    f.render_widget(help, popup_area);

    crate::ui::scrollbar::draw_vertical_scrollbar(f, popup_area, scroll, lines_len, inner_height);
}

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
pub struct HelpPopup;
impl HelpPopup {
    pub fn handle_event(app: &mut crate::app::App, key: KeyEvent) -> bool {
        let code = key.code;
        if app.is_bound(Action::Help, key) || app.is_bound(Action::CloseDetail, key) {
            app.close_dialog();
            return true;
        }
        match code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                app.help_scroll_up();
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                app.help_scroll_down();
            }
            KeyCode::PageUp => {
                app.help_scroll_page_up(app.config.page_size);
            }
            KeyCode::PageDown => {
                app.help_scroll_page_down(app.config.page_size);
            }
            KeyCode::Home => {
                app.help_scroll_to_top();
            }
            KeyCode::End => {
                app.help_scroll_to_bottom();
            }
            _ => {}
        }
        false
    }
}

pub struct DetailHelpPopup;
impl DetailHelpPopup {
    pub fn handle_event(app: &mut crate::app::App, key: KeyEvent) -> bool {
        let code = key.code;
        if app.is_bound(Action::DetailHelp, key) || app.is_bound(Action::CloseDetail, key) {
            app.close_detail_help();
            return true;
        }
        match code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                app.help_scroll_up();
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                app.help_scroll_down();
            }
            KeyCode::PageUp => {
                app.help_scroll_page_up(app.config.page_size);
            }
            KeyCode::PageDown => {
                app.help_scroll_page_down(app.config.page_size);
            }
            KeyCode::Home => {
                app.help_scroll_to_top();
            }
            KeyCode::End => {
                app.help_scroll_to_bottom();
            }
            _ => {}
        }
        false
    }
}
