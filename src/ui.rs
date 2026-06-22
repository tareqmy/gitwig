//! Rendering for the main list + status bar + help overlay.
//!
//! All drawing reads from `&App`; nothing here mutates state. Adding a new
//! keybinding means updating `HELP_LINES` here AND the status text below,
//! so the help overlay and the bottom bar stay aligned.
//!
//! Visual conventions live in the `theme` block at the top — keep visual
//! choices centralized so the look stays coherent.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Gauge, Padding, Paragraph, Wrap};

use crate::app::{App, ITEM_HEIGHT, Mode};
use crate::config::SortOrder;
use crate::repo::{ItemStatus, RepoSummary};

// ── Theme ──────────────────────────────────────────────────────────────────
// Colors are kept minimal so the app works on both dark and light terminal
// backgrounds. Plain text is left at the terminal's default foreground —
// never hard-code `White` / `Gray` / `DarkGray` for text, because what reads
// as "muted" on one background reads as invisible on the other. Use the
// helpers below: `muted_style()` for de-emphasis, bold for emphasis, and the
// accent colors only for true accents (selection, mode, warnings).

pub(crate) struct ThemeState {
    pub accent: Color,
    pub warning: Color,
    pub danger: Color,
    pub success: Color,
    pub border_type: BorderType,
}

pub(crate) static THEME: std::sync::RwLock<ThemeState> = std::sync::RwLock::new(ThemeState {
    accent: Color::Cyan,
    warning: Color::Yellow,
    danger: Color::Red,
    success: Color::Green,
    border_type: BorderType::Rounded,
});

#[allow(non_snake_case)]
pub(crate) fn ACCENT() -> Color {
    THEME.read().map(|l| l.accent).unwrap_or(Color::Cyan)
}
#[allow(non_snake_case)]
pub(crate) fn WARNING() -> Color {
    THEME.read().map(|l| l.warning).unwrap_or(Color::Yellow)
}
#[allow(non_snake_case)]
pub(crate) fn DANGER() -> Color {
    THEME.read().map(|l| l.danger).unwrap_or(Color::Red)
}
#[allow(non_snake_case)]
pub(crate) fn SUCCESS() -> Color {
    THEME.read().map(|l| l.success).unwrap_or(Color::Green)
}
#[allow(non_snake_case)]
pub(crate) fn CARD_BORDER() -> BorderType {
    THEME
        .read()
        .map(|l| l.border_type)
        .unwrap_or(BorderType::Rounded)
}

pub fn parse_color(s: &str) -> Color {
    match s.to_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" => Color::Gray,
        "darkgray" => Color::DarkGray,
        "lightred" => Color::LightRed,
        "lightgreen" => Color::LightGreen,
        "lightyellow" => Color::LightYellow,
        "lightblue" => Color::LightBlue,
        "lightmagenta" => Color::LightMagenta,
        "lightcyan" => Color::LightCyan,
        "white" => Color::White,
        _ => Color::Cyan,
    }
}

pub fn parse_border_type(s: &str) -> BorderType {
    match s.to_lowercase().as_str() {
        "plain" => BorderType::Plain,
        "rounded" => BorderType::Rounded,
        "double" => BorderType::Double,
        "thick" => BorderType::Thick,
        _ => BorderType::Rounded,
    }
}

pub fn update_theme(theme: &crate::config::ThemeConfig) {
    if let Ok(mut lock) = THEME.write() {
        lock.accent = parse_color(&theme.accent);
        lock.warning = parse_color(&theme.warning);
        lock.danger = parse_color(&theme.danger);
        lock.success = parse_color(&theme.success);
        lock.border_type = parse_border_type(&theme.border_type);
    }
}

/// Width of the per-item status zone on the right of each card. Wide
/// enough to fit a busy repo's worth of indicators ("● 99+ 99! 99? 99↑")
/// with a little breathing room. Right-aligned, so it crops on the left
/// for the unusual case of 3-digit counts on every indicator.
const STATUS_ZONE_WIDTH: u16 = 22;

/// Marker shown on the left edge of the selected card.
const SELECTION_MARK: &str = "▌ ";
const UNSELECTED_INDENT: &str = "  ";

/// "Muted" / secondary text. Uses the terminal's own foreground so it stays
/// readable on both light and dark backgrounds, then applies `DIM` to fade
/// it relative to primary text.
pub(crate) fn muted_style() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

/// Emphasized text. Bold over the terminal default — also theme-agnostic.
pub(crate) fn primary_style() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}

/// Accent-colored, bold. Used for keys in the status bar / help overlay,
/// and the app title.
pub(crate) fn accent_style() -> Style {
    Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)
}

/// Lines of the help overlay. Update this whenever a binding is added or
/// renamed — it is the user's only complete shortcut reference.
pub(crate) const HELP_LINES: &[(&str, &str)] = &[
    ("↑ [Up] / k", "Move selection up / scroll up"),
    ("↓ [Down] / j", "Move selection down / scroll down"),
    ("⇟ [PgDn]", "Jump one page down / page down"),
    ("⇞ [PgUp]", "Jump one page up / page up"),
    (
        "↵ [Enter]",
        "Open detail view for selected item / stage file",
    ),
    ("a", "Add a new item"),
    ("e", "Edit selected item"),
    ("d", "Delete selected item / branch (Branches) / tag (Tags)"),
    ("r", "Refresh status of selected item"),
    ("o / O", "Cycle sorting mode / Toggle reverse sorting"),
    ("g", "Launch gitui for selected repository"),
    ("s", "Open options/settings page"),
    (
        "⎋ [Esc]",
        "Cancel input, close dialog, leave detail view, or quit",
    ),
    ("⌫ [Backspace]", "Erase character while typing"),
    ("⇥ [Tab] / ⇧⇥", "Cycle detail view tabs"),
    ("w / W", "Cycle panel focus (Workspace / Branches tabs)"),
    ("c", "Commit changes (Workspace) / Create branch (Branches)"),
    ("⇧F", "Fetch selected branch (Branches tab)"),
    (
        "p",
        "Pull branch (Branches) / Push tag (Tags) / Toggle pin (List)",
    ),
    ("⇧P", "Push branch (Branches) / Push all tags (Tags)"),
    ("?", "Toggle this help overlay"),
    ("q", "Quit (also closes detail view)"),
    (
        "Left-Click",
        "Focus clicked panel / change tab (mouse support)",
    ),
    ("Left-Click+Drag", "Drag boundaries to resize split panels"),
];

/// Top-level draw entry point invoked from inside `terminal.draw`.
pub fn draw(
    f: &mut Frame,
    app: &App,
    area: Rect,
    inner_area: Rect,
    visible_count: usize,
    detail_areas: &mut crate::ui_detail::DetailAreas,
    main_areas: &mut Vec<Rect>,
) {
    draw_outer_frame(f, area, app);

    // Always reserve the bottom row for the status bar, regardless of mode.
    let (content_area, status_chunk) = content_and_status_chunks(inner_area, app.status_height());

    if matches!(
        app.mode,
        Mode::Detail
            | Mode::DetailHelp
            | Mode::CommitInput
            | Mode::BranchCreateInput
            | Mode::TagCreateInput
            | Mode::BranchDeleteConfirm
            | Mode::BranchPushConfirm
            | Mode::BranchMergeConfirm
            | Mode::BranchRebaseConfirm
            | Mode::BranchInteractiveRebaseConfirm
            | Mode::TagDeleteConfirm
            | Mode::TagPushConfirm
            | Mode::TagPushAllConfirm
            | Mode::StashDeleteConfirm
            | Mode::StashApplyConfirm
            | Mode::RemotePicker
            | Mode::CommitSearchInput
            | Mode::DiscardChangesConfirm
            | Mode::Inspect
    ) {
        if let Some(detail) = &app.current_detail {
            let item_name = app
                .config
                .items
                .get(app.selected_index)
                .map(String::as_str)
                .unwrap_or("");
            crate::ui_detail::draw(
                f,
                item_name,
                detail,
                &app.mode,
                &app.detail_focus,
                app.last_staging_focus,
                app.commit_selection,
                &app.commit_search_query,
                app.file_selection,
                &app.file_diff,
                app.diff_scroll,
                app.staging_file_selection,
                app.commit_details_scroll,
                app.local_branch_selection,
                app.remote_branch_selection,
                app.local_tag_selection,
                app.remote_selection,
                app.remote_picker_selection,
                app.stash_selection,
                app.stash_file_selection,
                app.file_list_selection,
                app.file_content_scroll,
                &app.visible_files,
                app.detail_tab,
                app.graph_scroll,
                app.help_scroll,
                detail_areas,
                &app.input_buffer,
                app.commit_editing,
                &app.branch_action_target,
                &app.tag_action_target_oid,
                &app.tag_delete_target,
                &app.tag_push_target,
                &app.discard_target,
                app.stash_apply_delete_after,
                app.commit_amend,
                app.commit_input_scroll,
                app.inspect_horizontal_split_pct,
                app.inspect_vertical_split_pct,
                app.workspace_main_split_pct,
                app.files_horizontal_split_pct,
                app.branches_horizontal_split_pct,
                app.stashes_horizontal_split_pct,
                app.stashes_vertical_split_pct,
                app.overview_horizontal_split_pct,
                content_area,
            );
        }
    } else if app.mode == Mode::Settings {
        draw_settings_page(f, app, content_area);
    } else if app.config.items.is_empty() {
        draw_empty_state(f, content_area);
    } else {
        let list_chunks = item_chunks(content_area, visible_count);
        *main_areas = list_chunks.clone();
        draw_items(f, app, &list_chunks);
    }

    draw_status_bar(f, app, status_chunk);

    if matches!(app.mode, Mode::Help) {
        draw_help_overlay(f, area, app.help_scroll);
    }

    if app.fetching {
        draw_progress_popup(f, area, app);
    }
}

fn draw_outer_frame(f: &mut Frame, area: Rect, app: &App) {
    let show_sort = matches!(
        app.mode,
        Mode::Normal | Mode::Adding | Mode::Editing | Mode::ConfirmDelete | Mode::Help
    );

    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(muted_style())
        .title(
            Line::from(vec![
                Span::raw(" "),
                Span::styled("Twig", accent_style()),
                Span::raw(" "),
            ])
            .alignment(Alignment::Left),
        );

    if show_sort {
        let sort_label = match app.config.sort_by {
            SortOrder::Custom => "Sort: Custom",
            SortOrder::Alphabetical => "Sort: Alphabetical",
            SortOrder::RecentVisit => "Sort: Recent Visit",
            SortOrder::LatestChanges => "Sort: Latest Changes",
        };
        let sort_label_with_dir = if app.config.sort_reverse {
            format!("{} (Rev)", sort_label)
        } else {
            sort_label.to_string()
        };

        block = block.title(
            Line::from(vec![
                Span::raw(" "),
                Span::styled(sort_label_with_dir, accent_style()),
                Span::raw(" "),
            ])
            .alignment(Alignment::Center),
        );
    }

    block = block.title(
        Line::from(format!(" v{} ", env!("CARGO_PKG_VERSION")))
            .style(muted_style())
            .alignment(Alignment::Right),
    );
    f.render_widget(block, area);
}

/// Reserve the bottom row for the status bar. The remainder is the
/// "content area" — list view, detail view, or anything else a mode
/// wants to draw.
fn content_and_status_chunks(inner_area: Rect, status_height: u16) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(status_height)])
        .split(inner_area);
    (chunks[0], chunks[1])
}

/// Within the content area, split into N item rows + a flex spacer so the
/// list is top-aligned and never pushes against the status bar.
fn item_chunks(content_area: Rect, visible_count: usize) -> Vec<Rect> {
    let mut constraints = vec![Constraint::Length(ITEM_HEIGHT); visible_count];
    constraints.push(Constraint::Min(0));

    let chunks: Vec<Rect> = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(content_area)
        .to_vec();
    // Drop the trailing spacer.
    chunks[..visible_count].to_vec()
}

fn draw_items(f: &mut Frame, app: &App, chunks: &[Rect]) {
    let upper = (app.scroll_top + chunks.len()).min(app.config.items.len());
    let visible_items = &app.config.items[app.scroll_top..upper];

    for (i, item) in visible_items.iter().enumerate() {
        let actual_index = i + app.scroll_top;
        let is_selected = actual_index == app.selected_index;
        let pending_delete = is_selected && matches!(app.mode, Mode::ConfirmDelete);
        let pending_edit = is_selected && matches!(app.mode, Mode::Editing);
        let is_pinned = app.config.pinned.contains(item);

        // Selected/pending cards use an accent color; unselected cards use
        // the terminal's default foreground (dimmed) so they stay legible
        // on both light and dark backgrounds.
        let border_style = if pending_delete {
            Style::default().fg(DANGER())
        } else if pending_edit {
            Style::default().fg(WARNING())
        } else if is_selected {
            Style::default().fg(ACCENT())
        } else if is_pinned {
            Style::default().fg(WARNING())
        } else {
            muted_style()
        };

        let (mark, mark_style, text_style) = if is_selected {
            (SELECTION_MARK, border_style, primary_style())
        } else {
            (UNSELECTED_INDENT, Style::default(), Style::default())
        };

        let border_type = if is_selected {
            BorderType::LightDoubleDashed
        } else {
            CARD_BORDER()
        };

        // Render the border block; split its inner rect into two rows:
        //   row 0 — item path (left) + status indicator (right)
        //   row 1 — branch name (left-aligned, muted)
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(border_type)
            .border_style(border_style)
            .padding(Padding::horizontal(1));
        let inner = block.inner(chunks[i]);
        f.render_widget(block, chunks[i]);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(inner);

        // Row 0: repo name + pin sign
        let name_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(4)])
            .split(rows[0]);

        let repo_name = std::path::Path::new(item)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(item.as_str());

        let fallback = ItemStatus::Missing;
        let status = app.statuses.get(actual_index).unwrap_or(&fallback);
        let is_git = matches!(status, ItemStatus::GitRepo(_));

        let mut spans = vec![Span::styled(mark, mark_style)];
        if is_git {
            spans.push(Span::styled(
                "⎇  ",
                muted_style().add_modifier(Modifier::BOLD),
            ));
        }
        spans.push(Span::styled(repo_name, text_style));
        let name_line = Line::from(spans);
        f.render_widget(Paragraph::new(name_line), name_cols[0]);

        if is_pinned {
            let pin_line = Line::from(Span::styled("📌", Style::default().fg(WARNING())))
                .alignment(Alignment::Right);
            f.render_widget(Paragraph::new(pin_line), name_cols[1]);
        }

        // Row 1: Left column (branch name) and Right column (status section)
        let row1_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(STATUS_ZONE_WIDTH)])
            .split(rows[1]);

        let branch = match status {
            ItemStatus::GitRepo(Some(s)) => s.branch.clone(),
            _ => None,
        };
        let branch_line = match branch {
            Some(b) => Line::from(vec![
                Span::raw(UNSELECTED_INDENT), // align with item text
                Span::styled("  ", muted_style()),
                Span::styled(b, Style::default().fg(ACCENT())),
            ]),
            None => Line::from(""),
        };
        f.render_widget(Paragraph::new(branch_line), row1_cols[0]);

        let status_line = status_indicator_line(status).alignment(Alignment::Right);
        f.render_widget(Paragraph::new(status_line), row1_cols[1]);
    }
}

/// Renders a centered empty-state message when no items are in the list.
fn draw_empty_state(f: &mut Frame, area: Rect) {
    // Vertical: push content to the upper-middle third of the area.
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Min(0),
            Constraint::Percentage(40),
        ])
        .split(area);

    let lines = vec![
        Line::from(vec![Span::styled(
            "No repositories tracked yet.",
            primary_style(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::raw("Press  "),
            Span::styled("a", accent_style()),
            Span::raw("  to add a repository or directory path"),
        ]),
        Line::from(vec![
            Span::raw("Press  "),
            Span::styled("e", accent_style()),
            Span::raw("  to edit the selected item"),
        ]),
        Line::from(vec![
            Span::raw("Press  "),
            Span::styled("d", accent_style()),
            Span::raw("  to delete the selected item"),
        ]),
        Line::from(vec![
            Span::raw("Press  "),
            Span::styled("?", accent_style()),
            Span::raw("  to see all shortcuts"),
        ]),
        Line::from(vec![
            Span::raw("Press  "),
            Span::styled("q", accent_style()),
            Span::raw("  to quit"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tip: ", muted_style()),
            Span::styled(
                "paths support ~ expansion  (e.g. ~/code/my-project)",
                muted_style(),
            ),
        ]),
    ];

    let para = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(para, vert[1]);
}

/// Renders the per-item status as a colored symbol + (for git repos) a
/// compact set of `N+` (staged), `N!` (modified), `N?` (untracked),
/// `N↑` (commits ahead), `N↓` (commits behind) suffixes. Only non-zero
/// counts are shown so the indicator stays compact for the common case.
fn status_indicator_line(status: &ItemStatus) -> Line<'static> {
    match status {
        ItemStatus::Missing => Line::from(vec![
            Span::styled("✕", Style::default().fg(DANGER())),
            Span::raw(" "),
            Span::styled("missing", muted_style()),
        ]),
        ItemStatus::Directory => Line::from(vec![
            Span::styled("○", Style::default().fg(WARNING())),
            Span::raw(" "),
            Span::styled("dir", muted_style()),
        ]),
        ItemStatus::GitRepo(None) => Line::from(vec![
            Span::styled("●", Style::default().fg(SUCCESS())),
            Span::raw(" "),
            Span::styled("?", muted_style()),
        ]),
        ItemStatus::GitRepo(Some(summary)) => repo_indicator_line(summary),
    }
}

fn repo_indicator_line(summary: &RepoSummary) -> Line<'static> {
    let mut spans = vec![Span::styled("●", Style::default().fg(SUCCESS()))];
    if summary.unchanged() {
        spans.push(Span::raw(" "));
        spans.push(Span::styled("clean", muted_style()));
        return Line::from(spans);
    }
    // Each (count, symbol, style) is rendered only if count > 0. The
    // ordering matches the Detail view's worktree section for consistency.
    let parts: [(usize, &str, Style); 5] = [
        (summary.staged, "+", Style::default().fg(ACCENT())),
        (summary.modified, "!", Style::default().fg(WARNING())),
        (summary.untracked, "?", muted_style()),
        (summary.ahead, "↑", primary_style()),
        (summary.behind, "↓", Style::default().fg(WARNING())),
    ];
    for (count, symbol, style) in parts {
        if count > 0 {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(format!("{}{}", count, symbol), style));
        }
    }
    Line::from(spans)
}

struct StatusEntry {
    spans: Vec<Span<'static>>,
}

impl StatusEntry {
    fn new(spans: Vec<Span<'static>>) -> Self {
        Self { spans }
    }

    fn width(&self) -> usize {
        self.spans.iter().map(|s| s.content.chars().count()).sum()
    }
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    match &app.mode {
        Mode::Settings => {
            let msg_spans = if let Some(msg) = &app.status_message {
                vec![Span::styled(format!("{} ", msg), accent_style())]
            } else if app.settings_editing {
                if app.settings_selected_index == 3 {
                    vec![Span::raw("Selecting theme... (Press Up/Down to choose)")]
                } else {
                    vec![Span::raw("Editing setting...")]
                }
            } else {
                vec![
                    Span::raw("Settings (Esc to exit) | Use "),
                    Span::styled("Enter", accent_style()),
                    Span::raw(" / "),
                    Span::styled("Space", accent_style()),
                    Span::raw(" to toggle/edit"),
                ]
            };
            let entries = if app.settings_editing {
                vec![
                    StatusEntry::new(vec![
                        Span::styled("↵ [Enter]", accent_style()),
                        Span::raw(" Save "),
                    ]),
                    StatusEntry::new(vec![
                        Span::styled("⎋ [Esc]", accent_style()),
                        Span::raw(" Cancel "),
                    ]),
                ]
            } else {
                vec![
                    StatusEntry::new(vec![
                        Span::styled("↑/↓ / k/j", accent_style()),
                        Span::raw(" Select "),
                    ]),
                    StatusEntry::new(vec![
                        Span::styled("↵ [Enter] / Space", accent_style()),
                        Span::raw(" Edit/Toggle "),
                    ]),
                    StatusEntry::new(vec![
                        Span::styled("⎋ [Esc] / q", accent_style()),
                        Span::raw(" Back "),
                    ]),
                ]
            };
            draw_status_layout(f, area, Some(msg_spans), entries, app.status_expanded);
        }
        Mode::Normal => {
            let (msg_spans, entries) = normal_status_entries(
                &app.status_message,
                app.config.sort_by,
                app.config.sort_reverse,
            );
            draw_status_layout(f, area, msg_spans, entries, app.status_expanded);
        }
        Mode::Adding => {
            draw_input_status(f, area, "Add", &app.input_buffer);
        }
        Mode::Editing => {
            draw_input_status(f, area, "Edit", &app.input_buffer);
        }
        Mode::ConfirmDelete => {
            let target = app
                .config
                .items
                .get(app.selected_index)
                .map(|s| s.as_str())
                .unwrap_or("");
            let (msg_spans, entries) = confirm_delete_entries(target);
            draw_status_layout(f, area, msg_spans, entries, app.status_expanded);
        }
        Mode::Help => {
            let (msg_spans, entries) = help_dismiss_entries();
            draw_status_layout(f, area, msg_spans, entries, app.status_expanded);
        }
        Mode::Detail => {
            let (msg_spans, entries) = detail_dismiss_entries(&app.status_message, app.detail_tab);
            draw_status_layout(f, area, msg_spans, entries, app.status_expanded);
        }

        Mode::DetailHelp => {
            let (msg_spans, entries) = detail_help_entries();
            draw_status_layout(f, area, msg_spans, entries, app.status_expanded);
        }
        Mode::CommitInput => {
            let (msg_spans, entries) = if app.commit_editing {
                commit_input_editing_entries()
            } else {
                commit_input_confirm_entries(app.commit_amend)
            };
            draw_status_layout(f, area, msg_spans, entries, app.status_expanded);
        }
        Mode::BranchCreateInput => {
            draw_input_status(f, area, "Create Branch", &app.input_buffer);
        }
        Mode::TagCreateInput => {
            draw_input_status(f, area, "Create Tag", &app.input_buffer);
        }
        Mode::BranchDeleteConfirm => {
            let (target, is_remote) = app
                .branch_action_target
                .as_ref()
                .map(|(name, remote)| (name.as_str(), *remote))
                .unwrap_or(("", false));
            let (msg_spans, entries) = confirm_branch_delete_entries(target, is_remote);
            draw_status_layout(f, area, msg_spans, entries, app.status_expanded);
        }
        Mode::BranchPushConfirm => {
            let target = app
                .branch_action_target
                .as_ref()
                .map(|(name, _)| name.as_str())
                .unwrap_or("");
            let (msg_spans, entries) = confirm_branch_push_entries(target);
            draw_status_layout(f, area, msg_spans, entries, app.status_expanded);
        }
        Mode::BranchMergeConfirm => {
            let (target, is_remote) = app
                .branch_action_target
                .as_ref()
                .map(|(name, remote)| (name.as_str(), *remote))
                .unwrap_or(("", false));
            let (msg_spans, entries) = confirm_branch_merge_entries(target, is_remote);
            draw_status_layout(f, area, msg_spans, entries, app.status_expanded);
        }
        Mode::BranchRebaseConfirm => {
            let (target, is_remote) = app
                .branch_action_target
                .as_ref()
                .map(|(name, remote)| (name.as_str(), *remote))
                .unwrap_or(("", false));
            let (msg_spans, entries) = confirm_branch_rebase_entries(target, is_remote);
            draw_status_layout(f, area, msg_spans, entries, app.status_expanded);
        }
        Mode::BranchInteractiveRebaseConfirm => {
            let (target, is_remote) = app
                .branch_action_target
                .as_ref()
                .map(|(name, remote)| (name.as_str(), *remote))
                .unwrap_or(("", false));
            let (msg_spans, entries) = confirm_branch_interactive_rebase_entries(target, is_remote);
            draw_status_layout(f, area, msg_spans, entries, app.status_expanded);
        }
        Mode::TagDeleteConfirm => {
            let (target, is_on_remote) = app
                .tag_delete_target
                .as_ref()
                .map(|(name, is_on_remote)| (name.as_str(), *is_on_remote))
                .unwrap_or(("", false));
            let (msg_spans, entries) = confirm_tag_delete_entries(target, is_on_remote);
            draw_status_layout(f, area, msg_spans, entries, app.status_expanded);
        }
        Mode::TagPushConfirm => {
            let target = app.tag_push_target.as_deref().unwrap_or("");
            let (msg_spans, entries) = confirm_tag_push_entries(target);
            draw_status_layout(f, area, msg_spans, entries, app.status_expanded);
        }
        Mode::TagPushAllConfirm => {
            let (msg_spans, entries) = confirm_tag_push_all_entries();
            draw_status_layout(f, area, msg_spans, entries, app.status_expanded);
        }
        Mode::StashDeleteConfirm => {
            let target = match &app.current_detail {
                Some(crate::repo::ItemDetail::Repo { info, .. }) => info
                    .stashes
                    .get(app.stash_selection)
                    .map(|s| format!("stash@{{{}}}", s.index))
                    .unwrap_or_else(|| "".to_string()),
                _ => "".to_string(),
            };
            let (msg_spans, entries) = confirm_stash_delete_entries(&target);
            draw_status_layout(f, area, msg_spans, entries, app.status_expanded);
        }
        Mode::StashApplyConfirm => {
            let target = match &app.current_detail {
                Some(crate::repo::ItemDetail::Repo { info, .. }) => info
                    .stashes
                    .get(app.stash_selection)
                    .map(|s| format!("stash@{{{}}}", s.index))
                    .unwrap_or_else(|| "".to_string()),
                _ => "".to_string(),
            };
            let (msg_spans, entries) =
                confirm_stash_apply_entries(&target, app.stash_apply_delete_after);
            draw_status_layout(f, area, msg_spans, entries, app.status_expanded);
        }
        Mode::RemotePicker => {
            let (msg_spans, entries) = remote_picker_status_entries();
            draw_status_layout(f, area, msg_spans, entries, app.status_expanded);
        }
        Mode::CommitSearchInput => {
            draw_input_status(f, area, "Search Commits", &app.input_buffer);
        }
        Mode::DiscardChangesConfirm => {
            let (target, staged) = app
                .discard_target
                .as_ref()
                .map(|(name, staged)| (name.as_str(), *staged))
                .unwrap_or(("", false));
            let (msg_spans, entries) = confirm_discard_changes_entries(target, staged);
            draw_status_layout(f, area, msg_spans, entries, app.status_expanded);
        }
        Mode::Inspect => {
            let (msg_spans, entries) = inspect_dismiss_entries(&app.status_message);
            draw_status_layout(f, area, msg_spans, entries, app.status_expanded);
        }
    }
}

fn detail_dismiss_entries(
    status_message: &Option<String>,
    detail_tab: usize,
) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let mut message_spans = None;
    if let Some(msg) = status_message {
        message_spans = Some(vec![Span::styled(format!("{} ", msg), accent_style())]);
    }

    let mut entries = Vec::new();
    let entries_data = match detail_tab {
        0 => vec![
            ("Home", "⎋/q"),
            ("Tabs", "Tab/1-8"),
            ("Cycle Focus", "w/W"),
            ("Navigate/Scroll", "↑↓"),
            ("Stage/Unstage", "↵"),
            ("Discard", "x"),
            ("Commit", "c"),
            ("Tag", "t"),
            ("Interactive Rebase", "i"),
            ("Filter", "/"),
            ("Help", "?"),
        ],
        1 => vec![
            ("Home", "⎋/q"),
            ("Tabs", "Tab/1-8"),
            ("Cycle Focus", "w/W"),
            ("Navigate/Scroll", "↑↓"),
            ("Expand/Collapse", "←/→"),
            ("Help", "?"),
        ],
        2 => vec![
            ("Home", "⎋/q"),
            ("Tabs", "Tab/1-8"),
            ("Scroll", "↑↓"),
            ("Help", "?"),
        ],
        3 => vec![
            ("Home", "⎋/q"),
            ("Tabs", "Tab/1-8"),
            ("Cycle Focus", "w/W"),
            ("Checkout", "↵"),
            ("Create", "c"),
            ("Delete", "d"),
            ("Merge", "m"),
            ("Rebase", "r"),
            ("Interactive Rebase", "i"),
            ("Fetch", "⇧F"),
            ("Pull", "p"),
            ("Push", "⇧P"),
            ("Navigate", "↑↓"),
            ("Focus L/R", "←/→"),
            ("Help", "?"),
        ],
        4 => vec![
            ("Home", "⎋/q"),
            ("Tabs", "Tab/1-8"),
            ("Checkout", "↵"),
            ("Navigate", "↑↓"),
            ("Push", "p"),
            ("Push All", "⇧P"),
            ("Delete", "d"),
            ("Help", "?"),
        ],
        5 => vec![
            ("Home", "⎋/q"),
            ("Tabs", "Tab/1-8"),
            ("Navigate", "↑↓"),
            ("Fetch", "f/F"),
            ("Help", "?"),
        ],
        6 => vec![
            ("Home", "⎋/q"),
            ("Tabs", "Tab/1-8"),
            ("Cycle Focus", "w/W"),
            ("Navigate", "↑↓"),
            ("Apply", "a"),
            ("Delete", "d"),
            ("Help", "?"),
        ],
        7 => vec![("Home", "⎋/q"), ("Tabs", "Tab/1-8"), ("Help", "?")],
        _ => vec![("Home", "⎋/q"), ("Tabs", "Tab/1-8"), ("Help", "?")],
    };
    for (i, (label, key)) in entries_data.iter().enumerate() {
        let mut spans = Vec::new();
        if i > 0 {
            spans.push(Span::styled(" ", muted_style()));
        }
        spans.push(Span::raw((*label).to_string()));
        spans.push(Span::raw(" "));
        spans.push(Span::styled("[", muted_style()));
        spans.push(Span::styled((*key).to_string(), accent_style()));
        spans.push(Span::styled("]", muted_style()));
        entries.push(StatusEntry::new(spans));
    }
    (message_spans, entries)
}

fn inspect_dismiss_entries(
    status_message: &Option<String>,
) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let mut message_spans = None;
    if let Some(msg) = status_message {
        message_spans = Some(vec![Span::styled(format!("{} ", msg), accent_style())]);
    }

    let mut entries = Vec::new();
    let entries_data = [
        ("Workspace", "⎋"),
        ("Cycle Focus", "w/W"),
        ("Select File", "↑↓/j/k"),
        ("Scroll Diff", "↑↓/j/k (focused)"),
        ("Help", "?"),
    ];
    for (i, (label, key)) in entries_data.iter().enumerate() {
        let mut spans = Vec::new();
        if i > 0 {
            spans.push(Span::styled(" ", muted_style()));
        }
        spans.push(Span::raw((*label).to_string()));
        spans.push(Span::raw(" "));
        spans.push(Span::styled("[", muted_style()));
        spans.push(Span::styled((*key).to_string(), accent_style()));
        spans.push(Span::styled("]", muted_style()));
        entries.push(StatusEntry::new(spans));
    }
    (message_spans, entries)
}

fn detail_help_entries() -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let entries = vec![StatusEntry::new(vec![
        Span::raw("Close Help"),
        Span::raw(" "),
        Span::styled("[", muted_style()),
        Span::styled("?/⎋/q", accent_style()),
        Span::styled("]", muted_style()),
    ])];
    (None, entries)
}

fn commit_input_editing_entries() -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Done Editing"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("⌃C", accent_style()),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Newline"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("↵", accent_style()),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel Commit"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Scroll"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("↑/↓", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (None, entries)
}

fn commit_input_confirm_entries(
    commit_amend: bool,
) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let amend_toggle_label = if commit_amend {
        "Amend: [Yes]"
    } else {
        "Amend: [No]"
    };
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Submit Commit"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("↵", accent_style()),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw(amend_toggle_label),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("a/space", accent_style()),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Edit Message"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("e", accent_style()),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("⎋/q", accent_style()),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Scroll"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("↑/↓", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (None, entries)
}

fn draw_status_layout(
    f: &mut Frame,
    area: Rect,
    message_spans: Option<Vec<Span<'static>>>,
    entries: Vec<StatusEntry>,
    status_expanded: bool,
) {
    let mut spans = Vec::new();
    spans.push(Span::raw(" "));

    let mut initial_width = 1;
    if let Some(ref msg) = message_spans {
        spans.extend(msg.clone());
        initial_width += msg.iter().map(|s| s.content.chars().count()).sum::<usize>();
    }

    let max_width = area.width as usize;

    if status_expanded {
        for entry in entries {
            spans.extend(entry.spans);
        }
        spans.push(Span::styled(" ", muted_style()));
        spans.push(Span::raw("Less"));
        spans.push(Span::raw(" "));
        spans.push(Span::styled("[", muted_style()));
        spans.push(Span::styled(".", accent_style()));
        spans.push(Span::styled("]", muted_style()));

        let para = Paragraph::new(Line::from(spans)).wrap(Wrap { trim: true });
        f.render_widget(para, area);
    } else {
        // Need to truncate whole entries. Leave space for " More [.]" which is 9 chars plus 2 safe buffer.
        let limit = max_width.saturating_sub(11);

        let mut fitted_entries = Vec::new();
        let mut current_width = initial_width;
        let mut truncated = false;

        for entry in entries {
            let w = entry.width();
            if current_width + w <= limit {
                current_width += w;
                fitted_entries.push(entry);
            } else {
                truncated = true;
                break;
            }
        }

        for entry in fitted_entries {
            spans.extend(entry.spans);
        }

        if truncated {
            spans.push(Span::styled(" ", muted_style()));
            spans.push(Span::raw("More"));
            spans.push(Span::raw(" "));
            spans.push(Span::styled("[", muted_style()));
            spans.push(Span::styled(".", accent_style()));
            spans.push(Span::styled("]", muted_style()));
        }

        let para = Paragraph::new(Line::from(spans));
        f.render_widget(para, area);
    }
}

fn normal_status_entries(
    status_message: &Option<String>,
    sort_by: SortOrder,
    sort_reverse: bool,
) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let mut message_spans = None;
    if let Some(msg) = status_message {
        message_spans = Some(vec![Span::styled(format!("{} ", msg), accent_style())]);
    }
    let sort_label = match sort_by {
        SortOrder::Custom => "Custom",
        SortOrder::Alphabetical => "Alphabetical",
        SortOrder::RecentVisit => "Recent",
        SortOrder::LatestChanges => "Changes",
    };
    let sort_dir = if sort_reverse { " (Rev)" } else { "" };
    let sort_key_label = format!("Sort: {}{}", sort_label, sort_dir);

    let entries_data = vec![
        ("Navigate", "↑↓"),
        ("Page", "⇟/⇞"),
        ("Detail", "↵"),
        ("gitui", "g"),
        (&sort_key_label, "o/O"),
        ("Add", "a"),
        ("Edit", "e"),
        ("Delete", "d"),
        ("Refresh", "r"),
        ("Pin", "p"),
        ("Help", "?"),
        ("Quit", "⎋/q"),
    ];
    let mut entries = Vec::new();
    for (i, (label, key)) in entries_data.iter().enumerate() {
        let mut spans = Vec::new();
        if i > 0 {
            spans.push(Span::styled(" ", muted_style()));
        }
        spans.push(Span::raw((*label).to_string()));
        spans.push(Span::raw(" "));
        spans.push(Span::styled("[", muted_style()));
        spans.push(Span::styled((*key).to_string(), accent_style()));
        spans.push(Span::styled("]", muted_style()));
        entries.push(StatusEntry::new(spans));
    }
    (message_spans, entries)
}

fn confirm_delete_entries(target: &str) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let message_spans = Some(vec![
        Span::raw("Delete "),
        Span::styled(
            format!("\"{}\"", target),
            Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
        ),
        Span::raw("? "),
    ]);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled(
                "y",
                Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn confirm_branch_delete_entries(
    target: &str,
    is_remote: bool,
) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let type_label = if is_remote {
        "remote-tracking branch"
    } else {
        "branch"
    };
    let message_spans = Some(vec![
        Span::raw("Delete "),
        Span::raw(type_label),
        Span::raw(" "),
        Span::styled(
            format!("\"{}\"", target),
            Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
        ),
        Span::raw("? "),
    ]);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled(
                "y",
                Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn confirm_discard_changes_entries(
    target: &str,
    staged: bool,
) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let area_label = if staged { "staged" } else { "unstaged" };
    let message_spans = Some(vec![
        Span::raw("Discard "),
        Span::raw(area_label),
        Span::raw(" changes in "),
        Span::styled(
            format!("\"{}\"", target),
            Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
        ),
        Span::raw("? "),
    ]);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled(
                "y",
                Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn confirm_branch_merge_entries(
    target: &str,
    is_remote: bool,
) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let type_label = if is_remote {
        "remote-tracking branch"
    } else {
        "branch"
    };
    let message_spans = Some(vec![
        Span::raw("Merge "),
        Span::raw(type_label),
        Span::raw(" "),
        Span::styled(
            format!("\"{}\"", target),
            Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" into current branch? "),
    ]);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled(
                "y",
                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn confirm_branch_rebase_entries(
    target: &str,
    is_remote: bool,
) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let type_label = if is_remote {
        "remote-tracking branch"
    } else {
        "branch"
    };
    let message_spans = Some(vec![
        Span::raw("Rebase current branch onto "),
        Span::raw(type_label),
        Span::raw(" "),
        Span::styled(
            format!("\"{}\"", target),
            Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
        ),
        Span::raw("? "),
    ]);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled(
                "y",
                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn confirm_branch_interactive_rebase_entries(
    target: &str,
    is_remote: bool,
) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let type_label = if is_remote {
        "remote-tracking branch"
    } else {
        "branch"
    };
    let message_spans = Some(vec![
        Span::raw("Interactively rebase current branch onto "),
        Span::raw(type_label),
        Span::raw(" "),
        Span::styled(
            format!("\"{}\"", target),
            Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
        ),
        Span::raw("? "),
    ]);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled(
                "y",
                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn confirm_stash_delete_entries(target: &str) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let message_spans = Some(vec![
        Span::raw("Delete stash "),
        Span::styled(
            format!("\"{}\"", target),
            Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
        ),
        Span::raw("? "),
    ]);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled(
                "y",
                Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn confirm_stash_apply_entries(
    target: &str,
    delete_after: bool,
) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let message_spans = Some(vec![
        Span::raw("Apply stash "),
        Span::styled(
            format!("\"{}\"", target),
            Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
        ),
        Span::raw("? "),
    ]);
    let delete_toggle_label = if delete_after {
        "Delete after apply: [Yes]"
    } else {
        "Delete after apply: [No]"
    };
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled(
                "y",
                Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw(delete_toggle_label.to_string()),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("d/space/a", accent_style()),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn remote_picker_status_entries() -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let message_spans = Some(vec![Span::raw("Select a remote to use for this operation")]);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Navigate"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("↑↓", accent_style()),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::raw(" Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("⏎", accent_style()),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::raw(" Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn confirm_branch_push_entries(target: &str) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let message_spans = Some(vec![
        Span::raw("Push branch "),
        Span::styled(
            format!("\"{}\"", target),
            Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
        ),
        Span::raw("? "),
    ]);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled(
                "y",
                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn help_dismiss_entries() -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let entries = vec![StatusEntry::new(vec![
        Span::raw("Close Help"),
        Span::raw(" "),
        Span::styled("[", muted_style()),
        Span::styled("?/⎋/q", accent_style()),
        Span::styled("]", muted_style()),
    ])];
    (None, entries)
}

/// Renders the input prompt for Add/Edit modes and places the real
/// terminal cursor at the end of the typed buffer.
fn draw_input_status(f: &mut Frame, area: Rect, verb: &str, buffer: &str) {
    let prefix = format!(" {} › ", verb);

    let spans = vec![
        Span::styled(
            prefix.clone(),
            Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
        ),
        Span::styled(buffer.to_string(), primary_style()),
        Span::styled(" ", muted_style()),
        Span::raw("Save"),
        Span::raw(" "),
        Span::styled("[", muted_style()),
        Span::styled("↵", accent_style()),
        Span::styled("]", muted_style()),
        Span::styled(" ", muted_style()),
        Span::raw("Cancel"),
        Span::raw(" "),
        Span::styled("[", muted_style()),
        Span::styled("⎋", accent_style()),
        Span::styled("]", muted_style()),
    ];
    let para = Paragraph::new(Line::from(spans));
    f.render_widget(para, area);

    // Place the real terminal cursor at the end of the input. Clamp to
    // the visible width so a long input doesn't push the cursor off-screen.
    let cursor_offset = (prefix.chars().count() + buffer.chars().count()) as u16;
    let cursor_x = area
        .x
        .saturating_add(cursor_offset.min(area.width.saturating_sub(1)));
    f.set_cursor_position(Position::new(cursor_x, area.y));
}

fn draw_settings_page(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(65, 75, area);

    // Draw background block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(accent_style())
        .title(
            Line::from(vec![
                Span::raw(" "),
                Span::styled("Settings", primary_style()),
                Span::raw(" "),
            ])
            .alignment(Alignment::Center),
        );

    f.render_widget(Clear, popup_area);
    f.render_widget(block.clone(), popup_area);

    let inner_rect = block.inner(popup_area);

    let mut items = Vec::new();

    for i in 0..5 {
        let is_selected = app.settings_selected_index == i;

        let label = match i {
            0 => "Poll Interval (ms)",
            1 => "Sort By",
            2 => "Sort Reverse",
            3 => "Theme Name",
            4 => "FZF Max Depth",
            _ => "",
        };

        let desc = match i {
            0 => "Event-loop poll interval in milliseconds. Sane range: 16-500.",
            1 => "Initial repository sorting criteria.",
            2 => "Reverse the order of repositories.",
            3 => "Active theme configuration name. Press Enter/Space to select from dropdown.",
            4 => "Maximum directory depth to search for git repositories.",
            _ => "",
        };

        let val_str = match i {
            0 => {
                if is_selected && app.settings_editing {
                    format!("{}█", app.input_buffer)
                } else {
                    app.config.poll_interval_ms.to_string()
                }
            }
            1 => {
                let s = match app.config.sort_by {
                    SortOrder::Alphabetical => "Alphabetical",
                    SortOrder::RecentVisit => "Recent Visit",
                    SortOrder::LatestChanges => "Latest Changes",
                    SortOrder::Custom => "Custom",
                };
                s.to_string()
            }
            2 => app.config.sort_reverse.to_string(),
            3 => {
                if is_selected && app.settings_editing {
                    if app.settings_theme_index < app.settings_theme_list.len() {
                        app.settings_theme_list[app.settings_theme_index].clone()
                    } else {
                        app.config.theme_name.clone()
                    }
                } else {
                    app.config.theme_name.clone()
                }
            }
            4 => {
                if is_selected && app.settings_editing {
                    format!("{}█", app.input_buffer)
                } else {
                    app.config.fzf.max_depth.to_string()
                }
            }
            _ => String::new(),
        };

        let prefix = if is_selected { " > " } else { "   " };

        let mut line_spans = vec![
            Span::styled(
                prefix,
                if is_selected {
                    accent_style()
                } else {
                    muted_style()
                },
            ),
            Span::styled(
                format!("{:<20}", label),
                if is_selected {
                    accent_style()
                } else {
                    primary_style()
                },
            ),
        ];

        if is_selected && app.settings_editing {
            let label = if i == 3 { " [Select]: " } else { " [Edit]: " };
            line_spans.push(Span::styled(label, muted_style()));
            line_spans.push(Span::styled(
                val_str,
                Style::default()
                    .fg(ACCENT())
                    .add_modifier(Modifier::UNDERLINED),
            ));
        } else {
            line_spans.push(Span::styled(" : ", muted_style()));
            line_spans.push(Span::styled(
                val_str,
                if is_selected {
                    accent_style()
                } else {
                    Style::default()
                },
            ));
        }

        items.push(Line::from(line_spans));
        items.push(Line::from(vec![
            Span::raw("     "),
            Span::styled(desc, muted_style()),
        ]));
        items.push(Line::from("")); // spacer
    }

    let paragraph = Paragraph::new(items)
        .block(Block::default().padding(Padding::horizontal(1)))
        .alignment(Alignment::Left);

    f.render_widget(paragraph, inner_rect);

    if app.settings_editing && app.settings_selected_index == 3 {
        // Draw the dropdown box
        let dropdown_width = 30;
        let dropdown_height = (app.settings_theme_list.len() + 2) as u16;

        // Position it near the theme name row
        // Theme name row index is 3, so its y coordinate starts at inner_rect.y + 9
        let dropdown_x = inner_rect.x + 25;
        let dropdown_y = inner_rect.y + 10;

        let dropdown_area = Rect::new(
            dropdown_x.min(area.right().saturating_sub(dropdown_width)),
            dropdown_y.min(area.bottom().saturating_sub(dropdown_height)),
            dropdown_width.min(area.width),
            dropdown_height.min(area.height),
        );

        let dropdown_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(accent_style())
            .title(Span::styled(" Select Theme ", accent_style()));

        f.render_widget(Clear, dropdown_area);
        f.render_widget(dropdown_block.clone(), dropdown_area);

        let dropdown_inner = dropdown_block.inner(dropdown_area);

        let mut theme_spans = Vec::new();
        for (idx, theme_name) in app.settings_theme_list.iter().enumerate() {
            let is_active = idx == app.settings_theme_index;
            let prefix = if is_active { "▶ " } else { "  " };
            let style = if is_active {
                accent_style().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            theme_spans.push(Line::from(Span::styled(
                format!("{}{}", prefix, theme_name),
                style,
            )));
        }

        let list = Paragraph::new(theme_spans);
        f.render_widget(list, dropdown_inner);
    }

    if app.settings_editing && app.settings_selected_index != 3 {
        let cursor_y = inner_rect.y + (app.settings_selected_index * 3) as u16;
        let cursor_x = inner_rect.x + 1 + 32 + app.input_buffer.chars().count() as u16;
        f.set_cursor_position(Position::new(cursor_x, cursor_y));
    }
}

fn draw_help_overlay(f: &mut Frame, area: Rect, scroll: usize) {
    let popup_area = centered_rect(60, 70, area);

    let key_width = HELP_LINES
        .iter()
        .map(|(k, _)| k.chars().count())
        .max()
        .unwrap_or(0);

    let mut lines: Vec<Line> = Vec::with_capacity(HELP_LINES.len() + 8);
    lines.push(Line::from(""));
    for (key, desc) in HELP_LINES {
        let padded_key = format!("{:>width$}", key, width = key_width);
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(padded_key, accent_style()),
            Span::raw("   "),
            Span::raw((*desc).to_string()),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Status indicators", primary_style()),
    ]));
    let pad_symbol = |sym: &'static str, color: Color, label: &'static str, desc: &'static str| {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(sym, Style::default().fg(color)),
            Span::raw(" "),
            Span::styled(format!("{:<8}", label), muted_style()),
            Span::raw(desc),
        ])
    };
    lines.push(pad_symbol(
        "●",
        SUCCESS(),
        "git",
        "Directory is a git repository",
    ));
    lines.push(pad_symbol(
        "○",
        WARNING(),
        "dir",
        "Directory exists but is not a git repo",
    ));
    lines.push(pad_symbol(
        "✕",
        DANGER(),
        "missing",
        "Path does not exist or is not a directory",
    ));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Repo state suffixes", primary_style()),
    ]));
    let pad_suffix = |sym: &'static str, style: Style, desc: &'static str| {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("N{}", sym), style),
            Span::raw("        "),
            Span::raw(desc),
        ])
    };
    lines.push(pad_suffix(
        "+",
        Style::default().fg(ACCENT()),
        "files staged for commit",
    ));
    lines.push(pad_suffix(
        "!",
        Style::default().fg(WARNING()),
        "files modified but not staged",
    ));
    lines.push(pad_suffix("?", muted_style(), "untracked files"));
    lines.push(pad_suffix(
        "↑",
        primary_style(),
        "commits ahead of upstream (need push)",
    ));
    lines.push(pad_suffix(
        "↓",
        Style::default().fg(WARNING()),
        "commits behind upstream",
    ));
    lines.push(Line::from(""));

    let help_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(Style::default().fg(ACCENT()))
        .title(
            Line::from(vec![
                Span::raw(" "),
                Span::styled("Shortcuts", accent_style()),
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

    if max_scroll > 0 {
        let mut scrollbar_state = ratatui::widgets::ScrollbarState::new(lines_len)
            .position(scroll)
            .viewport_content_length(inner_height);
        let scrollbar =
            ratatui::widgets::Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"))
                .thumb_style(Style::default().fg(ACCENT()));
        f.render_stateful_widget(scrollbar, popup_area, &mut scrollbar_state);
    }
}

/// Returns a `Rect` of `(percent_x, percent_y)` dimensions, centered inside `area`.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn confirm_tag_delete_entries(
    target: &str,
    is_on_remote: bool,
) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let mut spans = vec![
        Span::raw("Delete tag "),
        Span::styled(
            format!("\"{}\"", target),
            Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
        ),
        Span::raw("? "),
    ];
    if is_on_remote {
        spans.push(Span::styled(
            "(will also delete from remote) ",
            Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
        ));
    }
    let message_spans = Some(spans);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled(
                "y",
                Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn confirm_tag_push_entries(target: &str) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let message_spans = Some(vec![
        Span::raw("Push tag "),
        Span::styled(
            format!("\"{}\"", target),
            Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD),
        ),
        Span::raw("? "),
    ]);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled(
                "y",
                Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn confirm_tag_push_all_entries() -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let message_spans = Some(vec![
        Span::raw("Push "),
        Span::styled(
            "ALL",
            Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" local tags? "),
    ]);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled(
                "y",
                Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn draw_progress_popup(f: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(50, 15, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Network Operation", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(2));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // status text
            Constraint::Length(1), // spacer
            Constraint::Min(1),    // gauge
            Constraint::Length(1), // dismiss hint
        ])
        .split(inner);

    let status_text = app
        .status_message
        .as_deref()
        .unwrap_or("Executing Git network operation...");
    let status_para = Paragraph::new(Line::from(vec![Span::styled(status_text, muted_style())]));
    f.render_widget(status_para, chunks[0]);

    let gauge = Gauge::default()
        .block(Block::default().padding(Padding::ZERO))
        .gauge_style(Style::default().fg(ACCENT()))
        .percent(app.fetch_progress)
        .use_unicode(true);
    f.render_widget(gauge, chunks[2]);

    let hint = Paragraph::new(Line::from(vec![
        Span::styled("Press ", muted_style()),
        Span::styled("Esc", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" to dismiss if stuck", muted_style()),
    ]))
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(hint, chunks[3]);
}
