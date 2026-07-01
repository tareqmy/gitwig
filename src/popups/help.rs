use crate::app::{App, Mode};
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

pub fn get_help_lines(app: &App) -> Vec<Line<'_>> {
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

    let categories: Vec<(&str, Vec<(&str, &str)>)> = vec![
        (
            "Global & Navigation",
            vec![
                ("↑ [Up] / k", "Move selection up / scroll up"),
                ("↓ [Down] / j", "Move selection down / scroll down"),
                ("⇞ [PgUp]", "Jump one page up"),
                ("⇟ [PgDn]", "Jump one page down"),
                ("Home", "Go to top / scroll to top"),
                ("End", "Go to bottom / scroll to bottom"),
                ("⎋ [Esc]", "Cancel input, close dialog, leave detail view, or quit"),
                ("?", "Toggle this help overlay"),
                ("V", "Show about popup / creator profile"),
                ("h", "Show signs & symbols legend popup"),
                ("q", "Quit (also closes detail view)"),
            ],
        ),
        (
            "Main List Operations",
            vec![
                ("a", "Add a new repository"),
                ("A", "Bulk add folders in a directory"),
                ("i", "Import remote repository"),
                ("e", "Edit selected item"),
                ("D", "Delete selected item"),
                ("f", "Enter repository search mode"),
                ("R", "Refresh status of selected item"),
                ("o / O", "Cycle sorting mode / Toggle reverse sorting"),
                ("v", "Toggle between standard cards and compact 1-row view"),
                ("p", "Toggle pin status of selected item"),
                ("g", "Launch preferred Git client for selected repository"),
                ("s", "Open options/settings page"),
                ("l", "Edit labels of selected item"),
                ("d", "Open debug logs panel"),
                ("u", "Check for application updates manually"),
                ("⌫ [Backspace]", "Erase character while typing"),
            ],
        ),
        (
            "Repository Detail View",
            vec![
                ("↵ [Enter] / → [Right]", "Open detail view / Stage file"),
                ("⇥ [Tab] / ⇧⇥", "Cycle detail view tabs"),
                ("w / W", "Cycle panel focus forward (w) / backward (W)"),
                ("R", "Resync active tab (Detail)"),
            ],
        ),
        (
            "Git Operations (Detail)",
            vec![
                (
                    "c / C",
                    "Commit (c) / Amend last commit (C) (Workspace) / Create branch (Branches)",
                ),
                ("s", "Stash changes (Workspace changes or Stashes tab)"),
                ("p", "Pull branch (Branches) / Push tag (Tags)"),
                ("⇧P", "Push branch (Branches) / Push all tags (Tags)"),
                ("f / F", "Fetch remote (Branches / Tags / Remotes tabs)"),
                ("⇧H", "Show file history (Files tab)"),
                ("e / o", "Open file in editor (Files tab)"),
            ],
        ),
        (
            "Mouse Interactions",
            vec![
                ("Left-Click", "Focus clicked panel / change tab (mouse support)"),
                ("Left-Click+Drag", "Drag boundaries to resize split panels"),
            ],
        ),
    ];

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
            let k_str = format_key(key);
            let padded_key = format!("{:>width$}", k_str, width = max_key_width);
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(padded_key, accent_style()),
                Span::raw("   "),
                Span::raw(desc.to_string()),
            ]));
        }
    }

    // Status Indicators section
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
        Span::styled("STATUS INDICATORS", title_style),
    ]));

    let mut pad_symbol = |sym: &str, color: Color, label: &'static str, desc: &'static str| {
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(sym.to_string(), Style::default().fg(color)),
            Span::raw(" "),
            Span::styled(format!("{:<8}", label), muted_style()),
            Span::raw(desc),
        ]));
    };

    pad_symbol(app.sym("bullet_filled"), SUCCESS(), "git", "Directory is a git repository");
    pad_symbol(app.sym("bullet_empty"), WARNING(), "dir", "Directory exists but is not a git repo");
    pad_symbol(app.sym("close"), DANGER(), "missing", "Path does not exist or is not a directory");

    // Repo state suffixes section
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(prefix, if is_compat { Style::default() } else { accent_style() }),
        Span::styled("REPO STATE SUFFIXES", title_style),
    ]));

    let mut pad_suffix = |sym: &str, style: Style, desc: &'static str| {
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(format!("N{}", sym), style),
            Span::raw("        "),
            Span::raw(desc),
        ]));
    };

    pad_suffix("+", Style::default().fg(ACCENT()), "files staged for commit");
    pad_suffix("!", Style::default().fg(WARNING()), "files modified but not staged");
    pad_suffix("?", muted_style(), "untracked files");
    pad_suffix(app.sym("up"), primary_style(), "commits ahead of upstream (need push)");
    pad_suffix(app.sym("down"), Style::default().fg(WARNING()), "commits behind upstream");

    lines.push(Line::from(""));
    lines
}

pub fn get_help_lines_len(app: &App) -> usize {
    get_help_lines(app).len()
}

pub fn draw_help_overlay(f: &mut Frame, app: &App, area: Rect, scroll: usize) {
    let popup_area = centered_rect(60, 70, area);

    let lines = get_help_lines(app);

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
    let help = Paragraph::new(lines)
        .block(help_block)
        .wrap(Wrap { trim: false })
        .scroll((scroll as u16, 0));

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
        match code {
            KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.close_dialog();
            }
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
        match code {
            KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.close_detail_help();
            }
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
