//! Short-cut keys guide popup for active pane options in the detail inspect view.

use crate::app::{App, DetailSection, Mode};
use crate::keybindings::Action;
use crate::popups::help::nav_key_caption;
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

    let grow_key =
        app.keybindings.format_action_keys(crate::keybindings::Action::GrowPanel, is_compat);
    let shrink_key =
        app.keybindings.format_action_keys(crate::keybindings::Action::ShrinkPanel, is_compat);
    let resize_key = format!("{} / {}", grow_key, shrink_key);

    let resync_key =
        app.keybindings.format_action_keys(crate::keybindings::Action::RefreshDetail, is_compat);
    let status_bar_key =
        app.keybindings.format_action_keys(crate::keybindings::Action::ToggleStatusBar, is_compat);
    let terminal_toggle_key = app
        .keybindings
        .format_action_keys(crate::keybindings::Action::ToggleTerminalPanel, is_compat);
    let palette_key =
        app.keybindings.format_action_keys(crate::keybindings::Action::CommandPalette, is_compat);

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

    // Every configurable action is rendered from the live keybindings so a
    // rebinding never leaves this overlay lying; rows that stay literal are
    // matched as literal key codes by their handlers and say so below.
    let k =
        |action: crate::keybindings::Action| nav_key_caption(&app.keybindings, action, is_compat);
    // Joins the captions of several actions that share one row, collapsing
    // duplicates (most tabs bind the same key for the same kind of action).
    let join_keys = |actions: &[crate::keybindings::Action]| -> String {
        let mut seen: Vec<String> = Vec::new();
        for action in actions {
            let caption = k(*action);
            if !seen.contains(&caption) {
                seen.push(caption);
            }
        }
        seen.join(" / ")
    };

    let make_cat = |title: &'static str,
                    items: Vec<(String, &'static str)>|
     -> (&str, Vec<(String, &'static str)>) { (title, items) };

    let mut categories: Vec<(&str, Vec<(String, &str)>)> = vec![
        (
            "General Navigation",
            vec![
                (
                    k(Action::DetailMoveUp),
                    "Select previous commit / file / branch / file tree item",
                ),
                (k(Action::DetailMoveDown), "Select next commit / file / branch / file tree item"),
                (k(Action::DetailPageUp), "Jump page size rows up"),
                (k(Action::DetailPageDown), "Jump page size rows down"),
                (k(Action::DetailHome), "Scroll to top / go to first item"),
                (k(Action::DetailEnd), "Scroll to bottom / go to last item"),
                (cycle_tabs_key, "Cycle tabs within active group"),
                (toggle_tabs_key, "Toggle between Primary and Advanced tab groups"),
                (focus_key, "Cycle panel focus forward / backward"),
                (resize_key, "Grow / shrink the focused panel (where the layout splits)"),
                (resync_key, "Resync current tab state"),
                (status_bar_key, "Toggle status bar visibility"),
                (palette_key, "Open the command palette: run any action by name"),
                (terminal_toggle_key, "Toggle the embedded terminal panel"),
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
                    "Worktrees (1), Submodules (2), Reflog (3), Issues (4), PRs (5) (accessible when Advanced group is active)",
                ),
            ],
        ),
    ];

    let commit_keys =
        format!("{} / {}", k(Action::WorkspaceCommit), k(Action::WorkspaceCommitAmend));
    categories.push(make_cat(
        "Workspace & Inspection",
        vec![
            (k(Action::WorkspaceStage), "Stage / Unstage selected file (Workspace files lists)"),
            (k(Action::HomeOpenDetail), "Inspect selected commit (Workspace commits list)"),
            (k(Action::WorkspaceCheckout), "Checkout selected commit (Workspace commits list)"),
            (k(Action::CloseDetail), "Back to workspace commits list (Inspect mode)"),
            (commit_keys, "Commit / Amend last commit"),
            (k(Action::WorkspaceCreateTag), "Create tag (Workspace commits list)"),
            (
                k(Action::WorkspaceCreateBranch),
                "Create branch at selected commit (Workspace commits list)",
            ),
            (k(Action::WorkspaceCherryPick), "Cherry-pick selected commit (Workspace commits list)"),
            (
                k(Action::WorkspaceInteractiveRebase),
                "Interactive rebase from selected commit (Workspace commits list)",
            ),
            (k(Action::WorkspaceYankHash), "Yank selected commit hash"),
            (k(Action::WorkspaceStageAll), "Stage/Unstage All"),
            (k(Action::WorkspaceDiscard), "Discard changes in selected file"),
            (k(Action::WorkspaceDiscardAll), "Discard all changes in repository"),
            (
                k(Action::WorkspaceStashUI),
                "Open the stashing panel (Workspace lists); inside the Overview overlay the same key opens Repository Settings",
            ),
            (k(Action::WorkspaceRevert), "Revert selected commit (Workspace commits list)"),
            (k(Action::Overview), "Show repository Overview (from any tab)"),
            (k(Action::WorkspaceFuzzySearch), "Fuzzy search commits (History panel)"),
            (
                k(Action::WorkspaceColumnPicker),
                "Open search column picker (choose SHA/Message/Author/Date)",
            ),
            (k(Action::WorkspaceLogsView), "Open Logs view (Full screen commits list)"),
            (k(Action::WorkspaceLoadMore), "Load more commits (Workspace / Logs view)"),
            // The Overview overlay matches Tab / w / W as literal key codes
            // (src/input.rs, Mode::Overview), so this row stays literal.
            ("⇥ [Tab] / w / W".to_string(), "Cycle pane focus (Overview only)"),
        ],
    ));

    categories.push(make_cat(
        "Diff & Hunk Staging (Workspace diff / Inspect)",
        vec![
            (k(Action::DiffLineMode), "Toggle line-by-line stage/discard mode"),
            (k(Action::DiffStage), "Stage selected hunk/line"),
            (k(Action::DiffUnstage), "Unstage selected hunk/line"),
            (k(Action::DiffDiscard), "Discard selected hunk/line (immediate, no confirmation)"),
        ],
    ));

    // The file tree (src/components/file_tree.rs) matches → / ↵ (toggle),
    // ← (collapse all) and x / X (discard) as literal key codes; ← / Esc to
    // leave the full-screen viewer is literal ← plus the CloseDetail binding.
    let files_expand_key = format!("→ / ↵ / {}", k(Action::FilesExpand));
    let files_full_screen_key = k(Action::FilesFullScreen);
    let files_exit_full_screen_key = format!("← / {}", k(Action::CloseDetail));
    categories.push(make_cat(
        "Files Tab",
        vec![
            (files_expand_key, "Expand/toggle selected folder"),
            (k(Action::FilesCollapse), "Collapse selected folder"),
            ("←".to_string(), "Collapse all folders"),
            (k(Action::FilesSearch), "Fuzzy find files"),
            (k(Action::FilesBlame), "Toggle git blame panel"),
            (k(Action::FilesLineNumbers), "Toggle line numbers in content viewer"),
            ("x / X".to_string(), "Discard changes in selected file"),
            (k(Action::FilesHistory), "Show file history"),
            (k(Action::FilesEditor), "Open file in terminal editor"),
            (files_full_screen_key, "Show the content viewer full screen (content viewer focused)"),
            (files_exit_full_screen_key, "Leave the full-screen content viewer"),
        ],
    ));

    // Fetch and Add Remote on the Branches tab are matched as literal f / F
    // and a / A in src/components/branch_list.rs; every Tags-tab key comes
    // from the `[tags]` bindings (src/tabs/tags.rs).
    let branches_checkout_key = join_keys(&[Action::BranchesCheckout, Action::TagsCheckout]);
    let branches_delete_key = join_keys(&[Action::BranchesDelete, Action::TagsDelete]);
    let branches_pull_key = join_keys(&[Action::BranchesPull, Action::TagsPush]);
    let branches_push_key = join_keys(&[Action::BranchesPush, Action::TagsPushAll]);
    let branches_search_key = join_keys(&[Action::BranchesSearch, Action::TagsSearch]);
    categories.push(make_cat(
        "Branches & Tags Tab",
        vec![
            ("← / →".to_string(), "Focus Local/Remote branch (Branches tab)"),
            (branches_checkout_key, "Checkout selected branch / tag"),
            (k(Action::BranchesCreate), "Create branch"),
            (branches_delete_key, "Delete selected branch / tag"),
            (k(Action::BranchesMerge), "Merge selected branch into current branch"),
            (
                k(Action::BranchesMergeInto),
                "Checkout selected branch and merge the current branch into it",
            ),
            (k(Action::BranchesRebase), "Rebase current branch onto selected branch"),
            (
                k(Action::BranchesInteractiveRebase),
                "Interactive rebase of current branch onto selected branch",
            ),
            (branches_pull_key, "Pull branch (Branches) / Push tag (Tags)"),
            (branches_push_key, "Push branch (Branches) / Push all tags (Tags)"),
            (branches_search_key, "Fuzzy search branches / tags"),
            ("f / F".to_string(), "Fetch remote (Branches tab)"),
            (k(Action::TagsFetch), "Fetch remote tags (Tags tab)"),
            ("a / A".to_string(), "Add new remote (Branches tab)"),
        ],
    ));

    let misc_add_key = join_keys(&[
        Action::StashesApply,
        Action::WorktreesAdd,
        Action::RemotesAdd,
        Action::SubmodulesAdd,
    ]);
    let misc_delete_key = join_keys(&[
        Action::StashesDelete,
        Action::WorktreesDelete,
        Action::RemotesDelete,
        Action::SubmodulesDelete,
    ]);
    categories.push(make_cat(
        "Remotes, Stashes, Worktrees & Submodules Tabs",
        vec![
            (misc_add_key, "Apply stash / Add worktree / Add remote / Add submodule"),
            (k(Action::StashesCreate), "Create new stash (Stashes tab)"),
            (misc_delete_key, "Delete stash / Remove worktree / Delete remote / Delete submodule"),
            (k(Action::WorktreesLock), "Toggle lock status (Worktrees tab only)"),
            (k(Action::WorktreesPrune), "Prune worktree metadata (Worktrees tab only)"),
            (k(Action::WorktreesOpen), "Open worktree in new context (Worktrees tab only)"),
            (k(Action::RemotesFetch), "Fetch selected remote (Remotes tab)"),
        ],
    ));

    categories.push(make_cat(
        "Conflict Resolution",
        vec![
            (k(Action::ConflictOurs), "Accept OURS version of conflict"),
            (k(Action::ConflictTheirs), "Accept THEIRS version of conflict"),
            (k(Action::ConflictResolve), "Mark conflict as resolved"),
            (k(Action::ConflictAbort), "Abort the merge"),
            (k(Action::ConflictContinue), "Continue the merge"),
            (
                k(Action::ConflictMergeTool),
                "Open external mergetool (Conflicts file list / ConflictDiff pane)",
            ),
        ],
    ));

    categories.push(make_cat(
        "Reflog Tab",
        vec![(
            k(Action::ReflogCheckout),
            "Checkout the commit OID of the selected reflog entry (asks confirmation)",
        )],
    ));

    categories.push(make_cat(
        "Forge Integration (Issues & PRs)",
        vec![
            (
                k(Action::ForgeCheckout),
                "Checkout branch linked to selected issue or checkout PR branch (asks confirmation)",
            ),
            (k(Action::ForgeOpenBrowser), "Open selected issue or PR in web browser"),
            (
                k(Action::ForgeToggleAssigned),
                "Toggle between all issues and assigned issues (Issues tab only)",
            ),
            (k(Action::ForgeAddComment), "Add line comment to selected PR (PRs tab only)"),
        ],
    ));

    categories.push(make_cat(
        "Mouse Interactions",
        vec![
            ("Left-Click".to_string(), "Focus clicked panel / change tab (mouse support)"),
            ("Left-Click+Drag".to_string(), "Drag boundaries to resize split panels"),
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
