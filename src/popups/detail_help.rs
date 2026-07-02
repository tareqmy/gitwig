use crate::app::{App, DetailSection, Mode};
use crate::repo::RemoteInfo;
use crate::ui::layout::{centered_rect, centered_rect_fixed};
use crate::ui::style::{
    ACCENT, CARD_BORDER, DANGER, SUCCESS, WARNING, accent_style, muted_style, parse_color,
    primary_style,
};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, Clear, Gauge, List, ListItem, Padding, Paragraph, Row, Table,
    Wrap,
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

pub fn get_detail_help_lines(app: &App, usable_width: usize) -> Vec<Line<'_>> {
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
            "General Navigation",
            vec![
                ("↑ [Up] / k", "Select previous commit / file / branch / file tree item"),
                ("↓ [Down] / j", "Select next commit / file / branch / file tree item"),
                ("⇞ [PgUp]", "Jump page size rows up"),
                ("⇟ [PgDn]", "Jump page size rows down"),
                ("Home", "Scroll to top / go to first item"),
                ("End", "Scroll to bottom / go to last item"),
                ("⇥ [Tab] / ⇧⇥", "Cycle detail view tabs"),
                ("w / W", "Cycle panel focus forward (w) / backward (W)"),
                ("R", "Resync current tab state"),
                ("? / ⎋ [Esc]", "Close this help"),
                ("q / ⎋ [Esc]", "Back to repository list"),
            ],
        ),
        (
            "Tab Direct Navigation",
            vec![
                ("1", "Go to Workspace tab"),
                ("2", "Go to Files tab"),
                ("3", "Go to Graph View tab"),
                ("4", "Go to Branches tab"),
                ("5", "Go to Tags tab"),
                ("6", "Go to Remotes tab"),
                ("7", "Go to Stashes tab"),
                ("8", "Go to Worktrees tab"),
                ("9", "Go to Submodules tab"),
                ("0", "Go to Reflog tab"),
            ],
        ),
        (
            "Workspace & Inspection",
            vec![
                (
                    "↵ [Enter]",
                    "Stage/Unstage file, Checkout branch, Checkout tag, or Inspect commit",
                ),
                ("→ [Right]", "Inspect selected commit (Workspace commits list)"),
                ("⎋ [Esc]", "Back to workspace commits list (Inspect mode)"),
                ("c / C", "Commit (c) / Amend last commit (C)"),
                ("t", "Create tag (Workspace commits list)"),
                ("y", "Yank selected commit hash"),
                ("a", "Stage/Unstage All"),
                ("x", "Discard changes in selected file"),
                ("X", "Discard all changes in repository"),
                ("s", "Stash uncommitted changes"),
                ("v", "Show repository Overview (from any tab)"),
                ("f", "Open search logs picker"),
                ("l", "Open Logs view (Full screen commits list)"),
                ("G", "Load more commits (Workspace / Logs view)"),
                ("s", "Set repository theme (Overview only)"),
            ],
        ),
        (
            "Files Tab",
            vec![
                ("← / → or < / >", "Collapse/Expand folder"),
                ("f", "Fuzzy find files"),
                ("⇧H [Shift+H]", "Show file history"),
                ("e / o", "Open file in terminal editor"),
            ],
        ),
        (
            "Branches & Tags Tab",
            vec![
                ("← / →", "Focus Local/Remote branch (Branches tab)"),
                ("c", "Create branch"),
                ("⇧D [Shift+D]", "Delete selected branch / tag"),
                ("m", "Merge selected branch into current branch"),
                ("r", "Rebase current branch onto selected branch / Interactive rebase"),
                ("p", "Pull branch (Branches) / Push tag (Tags)"),
                ("⇧P [Shift+P]", "Push branch (Branches) / Push all tags (Tags)"),
                ("f / F", "Fetch remote (Branches / Tags tabs)"),
            ],
        ),
        (
            "Remotes, Stashes & Worktrees Tabs",
            vec![
                ("a", "Apply stash / Add worktree"),
                ("⇧D [Shift+D]", "Delete stash / Remove worktree"),
                ("l", "Toggle lock status (Worktrees tab only)"),
                ("p", "Prune worktree metadata (Worktrees tab only)"),
                ("↵ [Enter]", "Open worktree in new context (Worktrees tab only)"),
                ("f / F", "Fetch selected remote (Remotes tab)"),
            ],
        ),
        (
            "Conflict Resolution",
            vec![
                ("o", "Accept OURS version of conflict"),
                ("t", "Accept THEIRS version of conflict"),
                ("r", "Mark conflict as resolved"),
                ("A", "Abort the merge"),
                ("C", "Continue the merge"),
            ],
        ),
        (
            "Reflog Tab",
            vec![("↵ [Enter] / Space", "Checkout the commit OID of the selected reflog entry")],
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
            let k_str = format_key(key);
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

    lines.push(Line::from(""));
    lines
}

pub fn get_detail_help_lines_len(app: &App, width: u16) -> usize {
    let popup_width = (width * 80) / 100;
    let usable_width = popup_width.saturating_sub(4) as usize;
    get_detail_help_lines(app, usable_width).len()
}

pub fn draw_detail_help_overlay(f: &mut Frame, app: &App, area: Rect, scroll: usize) {
    let popup_area = centered_rect(80, 55, area);
    f.render_widget(Clear, popup_area);

    let usable_width = popup_area.width.saturating_sub(4) as usize;
    let lines = get_detail_help_lines(app, usable_width);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(Style::default().fg(ACCENT()))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Detail Shortcuts", primary_style()),
            Span::raw("  "),
            Span::styled("? / Esc  close", muted_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));

    let inner_height = popup_area.height.saturating_sub(2) as usize;
    let max_scroll = lines.len().saturating_sub(inner_height);
    let scroll = scroll.min(max_scroll);

    let lines_len = lines.len();
    let para = Paragraph::new(lines).block(block).scroll((scroll as u16, 0));
    f.render_widget(para, popup_area);

    crate::ui::scrollbar::draw_vertical_scrollbar(f, popup_area, scroll, lines_len, inner_height);
}
