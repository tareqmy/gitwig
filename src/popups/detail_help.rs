//! Short-cut keys guide popup for active pane options in the detail inspect view.

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

    let tab_fwd =
        app.keybindings.format_action_keys(crate::keybindings::Action::CycleTabForward, is_compat);
    let tab_bwd =
        app.keybindings.format_action_keys(crate::keybindings::Action::CycleTabBackward, is_compat);
    let cycle_tabs_key = format!("{} / {}", tab_fwd, tab_bwd);

    let toggle_tabs_key = app
        .keybindings
        .format_action_keys(crate::keybindings::Action::ToggleAdvancedTabs, is_compat);

    let focus_fwd = app
        .keybindings
        .format_action_keys(crate::keybindings::Action::CycleFocusForward, is_compat);
    let focus_bwd = app
        .keybindings
        .format_action_keys(crate::keybindings::Action::CycleFocusBackward, is_compat);
    let focus_key = format!("{} / {}", focus_fwd, focus_bwd);

    let resync_key =
        app.keybindings.format_action_keys(crate::keybindings::Action::RefreshDetail, is_compat);

    let help_key =
        app.keybindings.format_action_keys(crate::keybindings::Action::DetailHelp, is_compat);
    let esc_key =
        app.keybindings.format_action_keys(crate::keybindings::Action::CloseDetail, is_compat);
    let close_help_key = format!("{} / {}", help_key, esc_key);
    let back_key = esc_key;

    let p1 = app.keybindings.format_action_keys(crate::keybindings::Action::GoToTab1, is_compat);
    let p7 = app.keybindings.format_action_keys(crate::keybindings::Action::GoToTab7, is_compat);
    let primary_nums = format!("{}-{}", p1, p7);

    let a1 = app.keybindings.format_action_keys(crate::keybindings::Action::GoToTab1, is_compat);
    let a5 = app.keybindings.format_action_keys(crate::keybindings::Action::GoToTab5, is_compat);
    let advanced_nums = format!("{}-{}", a1, a5);

    let make_cat = |title: &'static str,
                    items: Vec<(&str, &'static str)>|
     -> (&str, Vec<(String, &'static str)>) {
        let mapped = items.into_iter().map(|(k, d)| (k.to_string(), d)).collect();
        (title, mapped)
    };

    let mut categories: Vec<(&str, Vec<(String, &str)>)> = vec![
        (
            "General Navigation",
            vec![
                (
                    "↑ [Up] / k".to_string(),
                    "Select previous commit / file / branch / file tree item",
                ),
                ("↓ [Down] / j".to_string(), "Select next commit / file / branch / file tree item"),
                ("⇞ [PgUp]".to_string(), "Jump page size rows up"),
                ("⇟ [PgDn]".to_string(), "Jump page size rows down"),
                ("Home".to_string(), "Scroll to top / go to first item"),
                ("End".to_string(), "Scroll to bottom / go to last item"),
                (cycle_tabs_key, "Cycle tabs within active group"),
                (toggle_tabs_key, "Toggle between Primary and Advanced tab groups"),
                (focus_key, "Cycle panel focus forward / backward"),
                (resync_key, "Resync current tab state"),
                (close_help_key, "Close this help"),
                (back_key, "Back to repository list"),
            ],
        ),
        (
            "Tab Direct Navigation",
            vec![
                (
                    format!("Primary [{}]", primary_nums),
                    "Workspace (1), Files (2), Graph (3), Branches (4), Tags (5), Remotes (6), Stashes (7)",
                ),
                (
                    format!("Advanced [{}]", advanced_nums),
                    "Worktrees (1), Submodules (2), Reflog (3), Forge (4), PRs (5) (accessible when Advanced group is active)",
                ),
            ],
        ),
    ];

    categories.push(make_cat(
        "Workspace & Inspection",
        vec![
            ("↵ [Enter]", "Stage/Unstage file, Checkout branch, Checkout tag, or Inspect commit"),
            ("o", "Checkout selected commit (Workspace commits list)"),
            ("→ [Right]", "Inspect selected commit (Workspace commits list)"),
            ("⎋ [Esc]", "Back to workspace commits list (Inspect mode)"),
            ("c / C", "Commit (c) / Amend last commit (C)"),
            ("t", "Create tag (Workspace commits list)"),
            ("y", "Yank selected commit hash"),
            ("a", "Stage/Unstage All"),
            ("x", "Discard changes in selected file"),
            ("X", "Discard all changes in repository"),
            ("s", "Stash uncommitted changes"),
            ("v", "Revert selected commit (Workspace commits list)"),
            ("O", "Show repository Overview (from any tab)"),
            ("/", "Fuzzy search commits (History panel)"),
            ("f", "Open search logs picker"),
            ("l", "Open Logs view (Full screen commits list)"),
            ("G", "Load more commits (Workspace / Logs view)"),
            ("s", "Set repository theme (Overview only)"),
        ],
    ));

    categories.push(make_cat(
        "Files Tab",
        vec![
            ("← / → or < / >", "Collapse/Expand folder"),
            ("/", "Fuzzy find files"),
            ("⇧H [Shift+H]", "Show file history"),
            ("e / o", "Open file in terminal editor"),
        ],
    ));

    categories.push(make_cat(
        "Branches & Tags Tab",
        vec![
            ("← / →", "Focus Local/Remote branch (Branches tab)"),
            ("c", "Create branch"),
            ("⇧D [Shift+D]", "Delete selected branch / tag"),
            ("m", "Merge selected branch into current branch"),
            ("r", "Rebase current branch onto selected branch / Interactive rebase"),
            ("p", "Pull branch (Branches) / Push tag (Tags)"),
            ("⇧P [Shift+P]", "Push branch (Branches) / Push all tags (Tags)"),
            ("/", "Fuzzy search branches / tags"),
            ("F", "Fetch remote (Branches / Tags tabs)"),
        ],
    ));

    categories.push(make_cat(
        "Remotes, Stashes & Worktrees Tabs",
        vec![
            ("a", "Apply stash / Add worktree"),
            ("⇧D [Shift+D]", "Delete stash / Remove worktree"),
            ("l", "Toggle lock status (Worktrees tab only)"),
            ("p", "Prune worktree metadata (Worktrees tab only)"),
            ("↵ [Enter]", "Open worktree in new context (Worktrees tab only)"),
            ("f / F", "Fetch selected remote (Remotes tab)"),
        ],
    ));

    categories.push(make_cat(
        "Conflict Resolution",
        vec![
            ("o", "Accept OURS version of conflict"),
            ("t", "Accept THEIRS version of conflict"),
            ("r", "Mark conflict as resolved"),
            ("A", "Abort the merge"),
            ("C", "Continue the merge"),
        ],
    ));

    categories.push(make_cat(
        "Reflog Tab",
        vec![("↵ [Enter] / Space", "Checkout the commit OID of the selected reflog entry")],
    ));

    categories.push(make_cat(
        "Forge Integration (Issues & PRs)",
        vec![
            ("↵ [Enter]", "Checkout branch linked to selected issue or checkout PR branch"),
            ("o", "Open selected issue or PR in web browser"),
            ("a", "Toggle between all issues and assigned issues (Issues tab only)"),
            ("n", "Add line comment to selected PR (PRs tab only)"),
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

    let compat = app.config.compatibility_mode;
    let help_key =
        app.keybindings.format_action_keys(crate::keybindings::Action::DetailHelp, compat);
    let close_key =
        app.keybindings.format_action_keys(crate::keybindings::Action::CloseDetail, compat);
    let title_close = format!("{} / {}  close", help_key, close_key);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(Style::default().fg(ACCENT()))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Detail Shortcuts", primary_style()),
            Span::raw("  "),
            Span::styled(title_close, muted_style()),
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
