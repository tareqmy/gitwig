//! Full-screen detail view for the currently-selected item.
//!
//! Reads a snapshot prepared by `repo::inspect_detail` and renders it as a
//! padded paragraph in the body of the outer Gitwig frame. Drawing is pure —
//! all git2 work happens once in `App::open_detail`, not per frame.
//!
#![allow(unused_imports)]
//! The Detail View rendering module.
//! The view is divided into labelled sections (Overview, Repository, Sync,
//! Working Tree) separated by blank lines and introduced by an accent-coloured
//! `▍ Title` header line. Within "Working Tree", changed files are listed
//! under Staged / Unstaged / Untracked / Conflicts sub-headers.

use crate::ui::layout::centered_rect;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, Clear, List, ListItem, ListState, Padding, Paragraph, Row,
    Table, TableState, Wrap,
};

use crate::app::{DetailSection, Mode};
use crate::repo::{
    self, CommitEntry, DiffLine, DiffLineKind, FileEntry, ItemDetail, RemoteInfo, RepoInfo,
    WorktreeChanges,
};
use crate::ui::{
    ACCENT, CARD_BORDER, DANGER, SUCCESS, WARNING, accent_style, muted_style, primary_style,
};

const FIELD_INDENT: &str = "  ";
/// Column width for the left-side field label — wide enough for "Upstream:".
const FIELD_LABEL_WIDTH: usize = 9;
/// Indent for file entries inside a working-tree sub-section.
const FILE_INDENT: &str = "      ";
/// Column width for the file-status label ("T" = 1 char).
const FILE_LABEL_WIDTH: usize = 2;

pub fn error_style() -> Style {
    Style::default().fg(DANGER())
}

// ── Hit-test areas ─────────────────────────────────────────────────────────

/// Bounding boxes of the interactive panels in the detail view.
/// Recorded each frame so the mouse handler can do hit-testing without
/// duplicating the layout logic.
#[derive(Default, Clone, Copy)]
pub struct DetailAreas {
    /// Commits table (top panel).
    pub commits: Option<Rect>,
    /// Left side of the bottom panel.
    /// In the uncommitted view this is the outer Staging Area block;
    /// for a real commit it is the Changed Files block.
    pub bottom_left: Option<Rect>,
    /// Staged sub-panel (only in the uncommitted / staging view).
    pub staged_sub: Option<Rect>,
    /// Unstaged sub-panel (only in the uncommitted / staging view).
    pub unstaged_sub: Option<Rect>,
    /// Conflicts sub-panel (only in the uncommitted / staging view).
    pub conflicts_sub: Option<Rect>,
    /// Diff / Staging Details panel (right side of the bottom panel).
    pub bottom_right: Option<Rect>,
    /// Commit details panel (bottom-left bottom-half panel).
    pub commit_details: Option<Rect>,
    /// Local branches panel in Branches tab.
    pub local_branches: Option<Rect>,
    /// Remote branches panel in Branches tab.
    pub remote_branches: Option<Rect>,
    /// Local tags panel in Tags tab.
    pub local_tags: Option<Rect>,
    /// Remote tags panel in Tags tab.
    pub remote_tags: Option<Rect>,
    /// Bounding box of the tab bar itself.
    pub tab_bar: Option<Rect>,
    /// Bounding box of the files list.
    pub files: Option<Rect>,
    /// Bounding box of the file content preview.
    pub file_content: Option<Rect>,
    /// Bounding box of the remotes panel.
    pub remotes: Option<Rect>,
    /// Bounding box of the stashes panel.
    pub stashes: Option<Rect>,
    /// Bounding box of the stashed files panel.
    pub stashed_files: Option<Rect>,
    /// Bounding box of the horizontal splitter in inspect view.
    pub inspect_horizontal_splitter: Option<Rect>,
    /// Bounding box of the vertical splitter in left panel of inspect view.
    pub inspect_vertical_splitter: Option<Rect>,
    /// Bounding box of the main vertical splitter in workspace tab.
    pub workspace_main_splitter: Option<Rect>,
    /// Bounding box of the horizontal splitter in files tab.
    pub files_horizontal_splitter: Option<Rect>,
    /// Bounding box of the horizontal splitter in branches tab.
    pub branches_horizontal_splitter: Option<Rect>,
    /// Bounding box of the horizontal splitter in stashes tab.
    pub stashes_horizontal_splitter: Option<Rect>,
    /// Bounding box of the vertical splitter in left panel of stashes tab.
    pub stashes_vertical_splitter: Option<Rect>,
    /// Bounding box of the horizontal splitter in overview tab.
    pub overview_horizontal_splitter: Option<Rect>,
    /// Inner area of commits list.
    pub commits_inner: Option<Rect>,
    /// Inner area of staged files list.
    pub staged_sub_inner: Option<Rect>,
    /// Inner area of unstaged files list.
    pub unstaged_sub_inner: Option<Rect>,
    /// Inner area of conflicts files list.
    pub conflicts_sub_inner: Option<Rect>,
    /// Inner area of changed files list.
    pub changed_files_inner: Option<Rect>,
    /// Inner area of local branches list.
    pub local_branches_inner: Option<Rect>,
    /// Inner area of remote branches list.
    pub remote_branches_inner: Option<Rect>,
    /// Inner area of local tags list.
    pub local_tags_inner: Option<Rect>,
    /// Inner area of remotes list.
    pub remotes_inner: Option<Rect>,
    /// Inner area of stashes list.
    pub stashes_inner: Option<Rect>,
    /// Inner area of stashed files list.
    pub stashed_files_inner: Option<Rect>,
    /// Bounding box of the commit message popup.
    pub commit_popup: Option<Rect>,
    /// Bounding box of the parent area the commit popup was centered inside.
    pub commit_popup_parent: Option<Rect>,
}

/// Renders the detail view into `area` and records panel bounds in `areas`.
#[allow(clippy::too_many_arguments)]
pub fn draw(
    f: &mut Frame,
    item_name: &str,
    detail: &ItemDetail,
    mode: &Mode,
    focus: &DetailSection,
    last_staging_focus: DetailSection,
    commit_selection: usize,
    commit_search_query: &Option<String>,
    file_selection: usize,
    file_diff: &[DiffLine],
    diff_scroll: usize,
    staging_file_selection: usize,
    commit_details_scroll: usize,
    local_branch_selection: usize,
    remote_branch_selection: usize,
    local_tag_selection: usize,
    remote_selection: usize,
    remote_picker_selection: usize,
    stash_selection: usize,
    stash_file_selection: usize,
    file_list_selection: usize,
    file_content_scroll: usize,
    visible_files: &[crate::app::FileTreeItem],
    detail_tab: usize,
    graph_scroll: usize,
    help_scroll: usize,
    areas: &mut DetailAreas,
    input_buffer: &str,
    commit_editing: bool,
    branch_action_target: &Option<(String, bool)>,
    tag_action_target_oid: &Option<String>,
    tag_delete_target: &Option<(String, bool)>,
    tag_push_target: &Option<String>,
    discard_target: &Option<(String, bool)>,
    stash_apply_delete_after: bool,
    commit_amend: bool,
    commit_input_scroll: usize,
    inspect_horizontal_split_pct: u16,
    inspect_vertical_split_pct: u16,
    workspace_main_split_pct: u16,
    files_horizontal_split_pct: u16,
    branches_horizontal_split_pct: u16,
    stashes_horizontal_split_pct: u16,
    stashes_vertical_split_pct: u16,
    overview_horizontal_split_pct: u16,
    app: &crate::app::App,
    area: Rect,
) {
    if app.in_logs_ui
        && matches!(mode, Mode::Logs | Mode::LogsSearchInput | Mode::SearchColumnPicker)
    {
        if let ItemDetail::Repo { info, .. } = detail {
            let header_rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // Title
                    Constraint::Length(1), // Divider
                    Constraint::Min(0),    // Commits list
                ])
                .split(area);

            // Header: title left, branch right
            let header_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(header_rows[0]);

            let branch = info.branch.as_ref();
            let header_left = Paragraph::new(Line::from(vec![
                Span::styled("▍ ", Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("Logs - {}", item_name),
                    primary_style().add_modifier(Modifier::BOLD),
                ),
            ]));
            f.render_widget(header_left, header_chunks[0]);

            if let Some(branch_name) = branch {
                let header_right = Paragraph::new(Line::from(vec![
                    Span::styled("  ", muted_style()),
                    Span::styled(branch_name.as_str(), accent_style()),
                    Span::raw("  "),
                ]))
                .alignment(Alignment::Right);
                f.render_widget(header_right, header_chunks[1]);
            }

            // Divider
            {
                let w = area.width as usize;
                let outer = (w / 10).max(2);
                let inner = (w / 8).max(3);
                let centre = w.saturating_sub(outer * 2 + inner * 2);
                let divider_line = Line::from(vec![
                    Span::styled(
                        " ".repeat(outer),
                        Style::default()
                            .fg(ratatui::style::Color::DarkGray)
                            .add_modifier(Modifier::DIM),
                    ),
                    Span::styled("┄".repeat(inner), muted_style().add_modifier(Modifier::DIM)),
                    Span::styled("┈".repeat(centre), muted_style()),
                    Span::styled("┄".repeat(inner), muted_style().add_modifier(Modifier::DIM)),
                    Span::styled(
                        " ".repeat(outer),
                        Style::default()
                            .fg(ratatui::style::Color::DarkGray)
                            .add_modifier(Modifier::DIM),
                    ),
                ]);
                f.render_widget(Paragraph::new(divider_line), header_rows[1]);
            }

            areas.commits = Some(header_rows[2]);
            crate::components::commit_list::draw_logs_view(
                f,
                info,
                commit_selection,
                commit_search_query,
                app,
                header_rows[2],
            );

            // Draw SearchColumnPicker overlay if in that mode
            if matches!(mode, Mode::SearchColumnPicker) {
                crate::popups::search_columns::draw_search_column_picker(f, app, area);
            }
        }
        return;
    }

    if mode == &Mode::Inspect {
        if let ItemDetail::Repo { info, .. } = detail {
            let dirty = !info.changes.staged.is_empty()
                || !info.changes.unstaged.is_empty()
                || !info.changes.untracked.is_empty()
                || !info.changes.conflicted.is_empty();
            let is_uncommitted = dirty && commit_selection == 0 && !app.in_logs_ui;

            if is_uncommitted {
                crate::components::status_list::draw_staging_panels(
                    f,
                    &info.changes,
                    *focus,
                    last_staging_focus,
                    staging_file_selection,
                    file_diff,
                    diff_scroll,
                    areas,
                    inspect_horizontal_split_pct,
                    inspect_vertical_split_pct,
                    app,
                    area,
                );
                return;
            } else {
                if let Some(commit) = app.get_selected_commit() {
                    crate::components::diff::draw_inspect_window(
                        f,
                        commit,
                        *focus,
                        file_selection,
                        file_diff,
                        diff_scroll,
                        commit_details_scroll,
                        areas,
                        inspect_horizontal_split_pct,
                        inspect_vertical_split_pct,
                        app,
                        area,
                    );
                    return;
                }
            }
        }
    }

    // Extract branch name if this is a repo detail.
    let branch: Option<String> = match detail {
        ItemDetail::Repo { info, .. } => info.branch.clone(),
        _ => None,
    };

    let is_repo = matches!(detail, ItemDetail::Repo { .. });

    let (header_area, tab_bar_area, body_area) = if is_repo {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Header
                Constraint::Length(2), // Tab Bar
                Constraint::Min(0),    // Body Content
            ])
            .split(area);
        (chunks[0], Some(chunks[1]), chunks[2])
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Header
                Constraint::Min(0),    // Body Content
            ])
            .split(area);
        (chunks[0], None, chunks[1])
    };

    let header_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(header_area);

    // Split header into left (Item label) and right (branch name).
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(40)])
        .split(header_rows[0]);

    let header_left = Paragraph::new(Line::from(vec![
        Span::raw(FIELD_INDENT),
        Span::styled("⎇  ", muted_style().add_modifier(Modifier::BOLD)),
        Span::styled(item_name.to_string(), accent_style()),
    ]));
    f.render_widget(header_left, header_chunks[0]);

    if let Some(ref branch_name) = branch {
        let header_right = Paragraph::new(Line::from(vec![
            Span::styled("  ", muted_style()),
            Span::styled(branch_name, accent_style()),
            Span::raw("  "),
        ]))
        .alignment(Alignment::Right);
        f.render_widget(header_right, header_chunks[1]);
    }

    {
        let w = header_area.width as usize;
        // Zone widths: outer fade → inner fade → solid centre (mirrored)
        let outer = (w / 10).max(2);
        let inner = (w / 8).max(3);
        let centre = w.saturating_sub(outer * 2 + inner * 2);

        let fade_outer =
            Style::default().fg(ratatui::style::Color::DarkGray).add_modifier(Modifier::DIM);
        let fade_inner = muted_style().add_modifier(Modifier::DIM);
        let solid = muted_style();

        let divider_line = Line::from(vec![
            Span::styled(" ".repeat(outer), fade_outer),
            Span::styled("┄".repeat(inner), fade_inner),
            Span::styled("┈".repeat(centre), solid),
            Span::styled("┄".repeat(inner), fade_inner),
            Span::styled(" ".repeat(outer), fade_outer),
        ]);
        f.render_widget(Paragraph::new(divider_line), header_rows[1]);
    }

    if let Some(tab_area) = tab_bar_area {
        let tabs_data = [
            ("Workspace", "W", 1),
            ("Files", "F", 2),
            ("Graph", "G", 3),
            ("Branches", "B", 4),
            ("Tags", "T", 5),
            ("Remotes", "R", 6),
            ("Stashes", "S", 7),
            ("Overview", "O", 8),
        ];

        let use_short = tab_area.width < 124;
        let mut spans = vec![Span::raw("  ")];
        for (i, &(long_name, short_name, index)) in tabs_data.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw(" "));
            }
            let name = if use_short { short_name } else { long_name };
            let bullet = if detail_tab == i { "┃" } else { "│" };
            let style = if detail_tab == i {
                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)
            } else {
                Style::default().add_modifier(Modifier::DIM | Modifier::UNDERLINED)
            };
            spans.push(Span::styled(format!("{} {} [{}] {}", bullet, name, index, bullet), style));
        }
        let tab_line = Line::from(spans);
        f.render_widget(Paragraph::new(tab_line), tab_area);
        areas.tab_bar = Some(tab_area);
    }

    match detail {
        ItemDetail::Repo { resolved, info } => {
            if detail_tab == 0 {
                // Split body: top = recent commits, bottom = staging panels
                let detail_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(workspace_main_split_pct),
                        Constraint::Percentage(100 - workspace_main_split_pct),
                    ])
                    .split(body_area);

                // Record main vertical splitter boundary in workspace tab
                let split_row = body_area.y + detail_chunks[0].height;
                areas.workspace_main_splitter =
                    Some(Rect::new(body_area.x, split_row.saturating_sub(1), body_area.width, 2));

                let dirty = !info.changes.staged.is_empty()
                    || !info.changes.unstaged.is_empty()
                    || !info.changes.untracked.is_empty()
                    || !info.changes.conflicted.is_empty();
                let show_dirty = if dirty {
                    if let Some(query) = commit_search_query {
                        "<uncommitted>".contains(&query.to_lowercase())
                    } else {
                        true
                    }
                } else {
                    false
                };
                let is_uncommitted_row = show_dirty && commit_selection == 0;

                crate::components::commit_list::draw_detail_commits(
                    f,
                    info,
                    *focus,
                    commit_selection,
                    commit_search_query,
                    detail_chunks[0],
                    &app.commit_list.table_state,
                    areas,
                    app.commit_list.limit,
                );
                areas.commits = Some(detail_chunks[0]);

                if is_uncommitted_row {
                    // <uncommitted> row selected — show working-tree staging panels.
                    crate::components::status_list::draw_staging_panels(
                        f,
                        &info.changes,
                        *focus,
                        last_staging_focus,
                        staging_file_selection,
                        file_diff,
                        diff_scroll,
                        areas,
                        inspect_horizontal_split_pct,
                        inspect_vertical_split_pct,
                        app,
                        detail_chunks[1],
                    );
                } else {
                    // Real commit selected — show its changed files.
                    match app.get_selected_commit() {
                        Some(commit) => {
                            crate::components::file_tree::draw_commit_files_panel(
                                f,
                                commit,
                                *focus,
                                file_selection,
                                file_diff,
                                diff_scroll,
                                commit_details_scroll,
                                areas,
                                inspect_horizontal_split_pct,
                                inspect_vertical_split_pct,
                                app,
                                detail_chunks[1],
                            );
                        }
                        None => {
                            // Fallback: selection out of range, show staging panels.
                            crate::components::status_list::draw_staging_panels(
                                f,
                                &info.changes,
                                *focus,
                                last_staging_focus,
                                staging_file_selection,
                                file_diff,
                                diff_scroll,
                                areas,
                                inspect_horizontal_split_pct,
                                inspect_vertical_split_pct,
                                app,
                                detail_chunks[1],
                            );
                        }
                    }
                }
            } else if detail_tab == 1 {
                // Render Files view (tab 2, index 1)
                crate::components::file_tree::draw_files_view(
                    f,
                    resolved,
                    info,
                    visible_files,
                    *focus,
                    file_list_selection,
                    file_content_scroll,
                    areas,
                    files_horizontal_split_pct,
                    app,
                    body_area,
                );
            } else if detail_tab == 2 {
                // Render Graph view (tab 3, index 2)
                match &info.graph_lines {
                    repo::TabData::NotLoaded | repo::TabData::Loading => {
                        let block = Block::default()
                            .borders(Borders::ALL)
                            .border_type(CARD_BORDER())
                            .border_style(muted_style())
                            .title(Line::from(vec![
                                Span::raw(" "),
                                Span::styled("Branch History Graph", primary_style()),
                                Span::raw(" "),
                            ]))
                            .padding(Padding::uniform(1));
                        let inner = block.inner(body_area);
                        f.render_widget(block, body_area);

                        let loading_text = Paragraph::new("⟳ Loading graph...")
                            .style(muted_style())
                            .alignment(ratatui::layout::Alignment::Center);
                        let area = Rect::new(inner.x, inner.y + inner.height / 2, inner.width, 1);
                        f.render_widget(loading_text, area);
                    }
                    repo::TabData::Error(err) => {
                        let block = Block::default()
                            .borders(Borders::ALL)
                            .border_type(CARD_BORDER())
                            .border_style(error_style())
                            .title(Line::from(vec![
                                Span::raw(" "),
                                Span::styled("Branch History Graph - Error", error_style()),
                                Span::raw(" "),
                            ]))
                            .padding(Padding::uniform(1));
                        let inner = block.inner(body_area);
                        f.render_widget(block, body_area);

                        let error_text = Paragraph::new(format!("Error loading graph: {}", err))
                            .style(error_style())
                            .wrap(Wrap { trim: false });
                        f.render_widget(error_text, inner);
                    }
                    repo::TabData::Loaded(_) => {
                        let block = Block::default()
                            .borders(Borders::ALL)
                            .border_type(CARD_BORDER())
                            .border_style(muted_style())
                            .title(Line::from(vec![
                                Span::raw(" "),
                                Span::styled("Branch History Graph", primary_style()),
                                Span::raw("  "),
                                Span::styled(
                                    format!("({})", info.graph_lines.len()),
                                    muted_style(),
                                ),
                                Span::raw(" "),
                            ]))
                            .padding(Padding::uniform(1));

                        let inner = block.inner(body_area);
                        f.render_widget(block, body_area);

                        let visible_height = inner.height as usize;
                        let upper = (graph_scroll + visible_height).min(info.graph_lines.len());
                        let visible_lines = &info.graph_lines.as_slice()[graph_scroll..upper];

                        let mut list_lines = Vec::new();
                        for g_line in visible_lines {
                            list_lines.push(graph_line_spans(g_line));
                        }

                        let paragraph = Paragraph::new(list_lines).wrap(Wrap { trim: false });
                        f.render_widget(paragraph, inner);
                    }
                }
            } else if detail_tab == 3 {
                // Render Branches view (tab 4, index 3)
                crate::components::branch_list::draw_branches_view(
                    f,
                    info,
                    *focus,
                    local_branch_selection,
                    remote_branch_selection,
                    areas,
                    branches_horizontal_split_pct,
                    app,
                    body_area,
                );
            } else if detail_tab == 4 {
                // Render Tags view (tab 5, index 4)
                crate::components::tag_list::draw_tags_view(
                    f,
                    info,
                    *focus,
                    local_tag_selection,
                    info.remote_tags_loaded,
                    areas,
                    app,
                    body_area,
                );
            } else if detail_tab == 5 {
                // Render Remotes view (tab 6, index 5)
                crate::components::branch_list::draw_remotes_view(
                    f,
                    info,
                    *focus,
                    remote_selection,
                    areas,
                    app,
                    body_area,
                );
            } else if detail_tab == 6 {
                // Render Stashes view (tab 7, index 6)
                crate::components::stash_list::draw_stashes_view(
                    f,
                    info,
                    *focus,
                    stash_selection,
                    stash_file_selection,
                    file_diff,
                    diff_scroll,
                    areas,
                    stashes_horizontal_split_pct,
                    stashes_vertical_split_pct,
                    app,
                    body_area,
                );
            } else {
                // Render Overview tab (tab 8, index 7)
                draw_overview_tab(
                    f,
                    resolved,
                    info,
                    overview_horizontal_split_pct,
                    areas,
                    body_area,
                );
            }
            // Draw detail help overlay on top when requested.
            if matches!(mode, Mode::DetailHelp) {
                crate::popups::detail_help::draw_detail_help_overlay(
                    f,
                    app,
                    body_area,
                    help_scroll,
                );
            }
            // Draw commit popup on top when requested.
            if matches!(mode, Mode::CommitInput) {
                crate::popups::commit::draw_commit_popup(
                    f,
                    &app.commit_popup.input_buffer,
                    commit_editing,
                    commit_amend,
                    commit_input_scroll,
                    body_area,
                    app,
                    areas,
                );
            }
            // Draw search column picker popup on top when requested.
            if matches!(mode, Mode::SearchColumnPicker) {
                crate::popups::search_columns::draw_search_column_picker(f, app, body_area);
            }
            // Draw branch create popup on top when requested.
            if matches!(mode, Mode::BranchCreateInput) {
                crate::popups::create_branch::draw_branch_create_popup(
                    f,
                    input_buffer,
                    branch.as_deref(),
                    body_area,
                );
            }
            // Draw remote add name popup on top when requested.
            if matches!(mode, Mode::RemoteAddNameInput) {
                crate::popups::add_remote::draw_remote_add_name_popup(f, input_buffer, body_area);
            }
            // Draw remote add url popup on top when requested.
            if matches!(mode, Mode::RemoteAddUrlInput) {
                crate::popups::add_remote::draw_remote_add_url_popup(
                    f,
                    &app.remote_add_name,
                    input_buffer,
                    body_area,
                );
            }
            // Draw remote delete popup on top when requested.
            if matches!(mode, Mode::RemoteDeleteConfirm) {
                crate::popups::confirm::draw_remote_delete_popup(
                    f,
                    app.remote_action_target.as_deref().unwrap_or(""),
                    body_area,
                );
            }
            // Draw tag create popup on top when requested.
            if matches!(mode, Mode::TagCreateInput) {
                crate::popups::create_tag::draw_tag_create_popup(
                    f,
                    input_buffer,
                    tag_action_target_oid.as_deref(),
                    body_area,
                );
            }
            // Draw stash create popup on top when requested.
            if matches!(mode, Mode::StashCreateInput) {
                crate::popups::stash_msg::draw_stash_create_popup(f, input_buffer, body_area);
            }
            // Draw branch delete popup on top when requested.
            if matches!(mode, Mode::BranchDeleteConfirm) {
                crate::popups::confirm::draw_branch_delete_popup(
                    f,
                    branch_action_target,
                    body_area,
                );
            }
            // Draw branch push popup on top when requested.
            if matches!(mode, Mode::BranchPushConfirm) {
                crate::popups::confirm::draw_branch_push_popup(f, branch_action_target, body_area);
            }
            // Draw branch merge popup on top when requested.
            if matches!(mode, Mode::BranchMergeConfirm) {
                crate::popups::confirm::draw_branch_merge_popup(
                    f,
                    branch_action_target,
                    branch.as_deref(),
                    body_area,
                );
            }
            // Draw merge abort popup on top when requested.
            if matches!(mode, Mode::MergeAbortConfirm) {
                crate::popups::confirm::draw_merge_abort_confirm_popup(f, body_area);
            }
            // Draw merge continue popup on top when requested.
            if matches!(mode, Mode::MergeContinueConfirm) {
                crate::popups::confirm::draw_merge_continue_confirm_popup(f, body_area);
            }
            // Draw branch rebase popup on top when requested.
            if matches!(mode, Mode::BranchRebaseConfirm) {
                crate::popups::confirm::draw_branch_rebase_popup(
                    f,
                    branch_action_target,
                    branch.as_deref(),
                    body_area,
                );
            }
            // Draw branch interactive rebase popup on top when requested.
            if matches!(mode, Mode::BranchInteractiveRebaseConfirm) {
                crate::popups::confirm::draw_branch_interactive_rebase_popup(
                    f,
                    branch_action_target,
                    branch.as_deref(),
                    body_area,
                );
            }
            // Draw tag delete popup on top when requested.
            if matches!(mode, Mode::TagDeleteConfirm) {
                crate::popups::confirm::draw_tag_delete_popup(f, tag_delete_target, body_area);
            }
            // Draw tag push popup on top when requested.
            if matches!(mode, Mode::TagPushConfirm) {
                crate::popups::confirm::draw_tag_push_popup(f, tag_push_target, body_area);
            }
            // Draw tag push all popup on top when requested.
            if matches!(mode, Mode::TagPushAllConfirm) {
                crate::popups::confirm::draw_tag_push_all_popup(
                    f,
                    app.remote_action_target.as_deref(),
                    body_area,
                );
            }
            // Draw cherry-pick popup on top when requested.
            if matches!(mode, Mode::CherryPickConfirm) {
                crate::popups::confirm::draw_cherry_pick_popup(
                    f,
                    &app.cherry_pick_target,
                    branch.as_deref(),
                    app,
                    body_area,
                );
            }
            // Draw revert popup on top when requested.
            if matches!(mode, Mode::RevertConfirm) {
                crate::popups::confirm::draw_revert_popup(
                    f,
                    &app.revert_target,
                    branch.as_deref(),
                    body_area,
                );
            }
            // Draw stash delete popup on top when requested.
            if matches!(mode, Mode::StashDeleteConfirm) {
                let stash_name = match detail {
                    ItemDetail::Repo { info, .. } => info
                        .stashes
                        .get(stash_selection)
                        .map(|s| format!("stash@{{{}}}: {}", s.index, s.message)),
                    _ => None,
                };
                crate::popups::confirm::draw_stash_delete_popup(f, &stash_name, body_area);
            }
            // Draw stash apply popup on top when requested.
            if matches!(mode, Mode::StashApplyConfirm) {
                let stash_name = match detail {
                    ItemDetail::Repo { info, .. } => info
                        .stashes
                        .get(stash_selection)
                        .map(|s| format!("stash@{{{}}}: {}", s.index, s.message)),
                    _ => None,
                };
                crate::popups::confirm::draw_stash_apply_popup(
                    f,
                    &stash_name,
                    stash_apply_delete_after,
                    body_area,
                );
            }
            // Draw remote picker popup on top when requested.
            if matches!(mode, Mode::RemotePicker) {
                if let ItemDetail::Repo { info, .. } = detail {
                    crate::popups::remote_picker::draw_remote_picker_popup(
                        f,
                        info.remotes.as_slice(),
                        remote_picker_selection,
                        body_area,
                    );
                }
            }
            // Draw discard changes popup on top when requested.
            if matches!(mode, Mode::DiscardChangesConfirm) {
                crate::popups::confirm::draw_discard_changes_popup(f, discard_target, body_area);
            }
            // Draw branch checkout popup on top when requested.
            if matches!(mode, Mode::BranchCheckoutConfirm) {
                crate::popups::confirm::draw_branch_checkout_popup(
                    f,
                    branch_action_target,
                    body_area,
                );
            }
            // Draw tag checkout popup on top when requested.
            if matches!(mode, Mode::TagCheckoutConfirm) {
                crate::popups::confirm::draw_tag_checkout_popup(
                    f,
                    &app.tag_checkout_target,
                    body_area,
                );
            }
        }
        _ => {
            let body_lines = build_body(app, detail);
            let body = Paragraph::new(body_lines)
                .block(Block::default().padding(Padding::ZERO))
                .wrap(Wrap { trim: false });
            f.render_widget(body, body_area);
        }
    }
}

// ── Commit files panel ─────────────────────────────────────────────────────

/// Renders the bottom panel for a selected real commit:
/// left = changed-file list (with selection), right = diff panel.

// ── Staging panels ─────────────────────────────────────────────────────────

/// Renders two side-by-side panels: \"Staging Area\" (left, split into Staged/Unstaged)
/// and \"Staging Details\" (right, diff of selected file).

/// Renders a titled sub-panel listing `files`, or a centred placeholder if empty.
/// `selection` — when `Some(idx)` the panel is focused and the file at `idx` is highlighted.

// ── Overview popup ─────────────────────────────────────────────────────────

/// Renders the repo overview as a floating popup centred over `area`.
fn draw_overview_tab(
    f: &mut Frame,
    resolved: &std::path::Path,
    info: &RepoInfo,
    overview_horizontal_split_pct: u16,
    areas: &mut DetailAreas,
    area: Rect,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(overview_horizontal_split_pct),
            Constraint::Percentage(100 - overview_horizontal_split_pct),
        ])
        .split(area);

    // Record horizontal splitter boundary in overview tab
    let split_col = area.x + chunks[0].width;
    areas.overview_horizontal_splitter =
        Some(Rect::new(split_col.saturating_sub(1), area.y, 2, area.height));

    let left_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(muted_style())
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Overview", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));

    let body_lines = build_repo_body(resolved, info);
    let body = Paragraph::new(body_lines).block(left_block).wrap(Wrap { trim: false });
    f.render_widget(body, chunks[0]);

    let right_title =
        if info.committer_stats_limit_reached { "Stats (last 10k commits)" } else { "Stats" };

    let right_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(muted_style())
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(right_title, primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));

    match &info.committer_stats {
        repo::TabData::NotLoaded | repo::TabData::Loading => {
            let inner = right_block.inner(chunks[1]);
            f.render_widget(right_block, chunks[1]);
            let loading_text = Paragraph::new("⟳ Loading stats...")
                .style(muted_style())
                .alignment(ratatui::layout::Alignment::Center);
            let center_area = Rect::new(inner.x, inner.y + inner.height / 2, inner.width, 1);
            f.render_widget(loading_text, center_area);
        }
        repo::TabData::Error(err) => {
            let right_block = right_block.border_style(error_style());
            let inner = right_block.inner(chunks[1]);
            f.render_widget(right_block, chunks[1]);
            let error_text = Paragraph::new(format!("Error loading stats: {}", err))
                .style(error_style())
                .wrap(Wrap { trim: false });
            f.render_widget(error_text, inner);
        }
        repo::TabData::Loaded(_) => {
            let stats_lines = build_committer_stats_lines(info);
            let stats_body =
                Paragraph::new(stats_lines).block(right_block).wrap(Wrap { trim: false });
            f.render_widget(stats_body, chunks[1]);
        }
    }
}

fn build_committer_stats_lines(info: &RepoInfo) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = vec![];

    push_section_header(&mut lines, "Committer Statistics");

    if info.committer_stats.is_empty() {
        lines.push(Line::from(vec![
            Span::raw(FIELD_INDENT),
            Span::styled("(no commits / unborn branch)", muted_style()),
        ]));
    } else {
        for stat in info.committer_stats.iter() {
            let mut stat_spans = vec![
                Span::raw(FIELD_INDENT),
                Span::styled("● ", Style::default().fg(ACCENT())),
                Span::styled(stat.name.clone(), primary_style()),
            ];

            if stat.email != "?" && !stat.email.is_empty() {
                stat_spans.push(Span::styled(format!(" <{}>", stat.email), muted_style()));
            }

            stat_spans.push(Span::styled("  ➔  ", muted_style()));
            stat_spans.push(Span::styled(
                format!("{} commit{}", stat.count, if stat.count == 1 { "" } else { "s" }),
                Style::default().fg(SUCCESS()),
            ));

            lines.push(Line::from(stat_spans));
        }
    }

    lines
}

/// Returns a [`Rect`] that is `percent_x` wide and `percent_y` tall, centred in `r`.

// Detail help overlay configuration is now dynamically generated in detail_help.rs

/// Renders a floating shortcut reference overlay centred over `area`.

// ── Body builder ───────────────────────────────────────────────────────────

fn build_body(app: &crate::app::App, detail: &ItemDetail) -> Vec<Line<'static>> {
    match detail {
        ItemDetail::Missing { resolved } => {
            let mut lines = vec![];
            push_section_header(&mut lines, "Overview");
            lines.push(kind_line(
                app.sym("close"),
                DANGER(),
                "Not a directory",
                "(path does not exist or isn't accessible)",
            ));
            lines.push(field_line("Path", Span::raw(resolved.display().to_string())));
            lines
        }
        ItemDetail::Directory { resolved } => {
            let mut lines = vec![];
            push_section_header(&mut lines, "Overview");
            lines.push(kind_line(
                app.sym("bullet_empty"),
                WARNING(),
                "Plain directory",
                "(exists, but no .git entry was found)",
            ));
            lines.push(field_line("Path", Span::raw(resolved.display().to_string())));
            lines
        }
        ItemDetail::Error { resolved, message } => {
            let mut lines = vec![];
            push_section_header(&mut lines, "Overview");
            lines.push(kind_line(
                app.sym("warning").trim(),
                WARNING(),
                "Could not read repository",
                "(libgit2 reported an error — see below)",
            ));
            lines.push(field_line("Path", Span::raw(resolved.display().to_string())));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::raw(FIELD_INDENT),
                Span::styled(message.clone(), Style::default().fg(DANGER())),
            ]));
            lines
        }
        ItemDetail::Repo { resolved, info } => build_repo_body(resolved, info),
    }
}

fn build_repo_body(resolved: &std::path::Path, info: &RepoInfo) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = vec![];

    // ── General ───────────────────────────────────────────────────────────
    push_section_header(&mut lines, "General");
    lines.push(kind_line("●", SUCCESS(), "Git Repository", "(inspectable libgit2)"));
    lines.push(field_line("Path", Span::raw(resolved.display().to_string())));
    let branch = info.branch.clone().unwrap_or_else(|| "(detached HEAD)".to_string());
    lines.push(field_line("Branch", Span::styled(branch, accent_style())));

    // ── HEAD Commit ───────────────────────────────────────────────────────
    push_section_header(&mut lines, "HEAD Commit");
    if let Some(head) = &info.head {
        lines.push(field_line(
            "Hash",
            Span::styled(head.short_id.clone(), Style::default().fg(WARNING())),
        ));
        lines.push(field_line("Message", Span::styled(head.summary.clone(), primary_style())));
        lines.push(field_line("Author", Span::raw(head.author.clone())));
        lines.push(field_line("Date", Span::raw(head.when.clone())));
    } else {
        lines.push(field_line("HEAD", Span::styled("(empty repository)", muted_style())));
    }

    // ── Sync ──────────────────────────────────────────────────────────────
    push_section_header(&mut lines, "Sync");
    append_sync(&mut lines, info);

    lines
}

// ── Section rendering ──────────────────────────────────────────────────────

/// Emits a blank line then `  ▍ Title` in accent + bold, then another blank.
fn push_section_header(lines: &mut Vec<Line<'static>>, title: &'static str) {
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw(FIELD_INDENT),
        Span::styled("▍ ", Style::default().fg(ACCENT())),
        Span::styled(title, primary_style()),
    ]));
    lines.push(Line::from(""));
}

// ── Sync section ──────────────────────────────────────────────────────────

fn append_sync(lines: &mut Vec<Line<'static>>, info: &RepoInfo) {
    match &info.upstream {
        None => {
            lines.push(field_line("Upstream", Span::styled("(not configured)", muted_style())));
            lines.push(field_line("Sync", Span::styled("—", muted_style())));
        }
        Some(name) => {
            lines.push(field_line(
                "Upstream",
                Span::styled(name.clone(), Style::default().fg(ACCENT())),
            ));
            let s = &info.summary;
            if s.is_synced() {
                lines.push(field_line(
                    "Sync",
                    Span::styled("in sync", Style::default().fg(SUCCESS())),
                ));
            } else {
                let mut spans =
                    vec![Span::raw(FIELD_INDENT), Span::styled(field_label("Sync"), muted_style())];
                if s.ahead > 0 {
                    spans.push(Span::styled(format!("{} ahead", s.ahead), primary_style()));
                }
                if s.behind > 0 {
                    if s.ahead > 0 {
                        spans.push(Span::raw(", "));
                    }
                    spans.push(Span::styled(
                        format!("{} behind", s.behind),
                        Style::default().fg(WARNING()),
                    ));
                }
                lines.push(Line::from(spans));
            }
        }
    }

    // Remotes listed under Sync
    if info.remotes.is_empty() {
        lines.push(field_line("Remotes", Span::styled("(none)", muted_style())));
    } else {
        lines.push(Line::from(""));
        for r in info.remotes.iter() {
            lines.push(remote_line(r));
        }
    }
}

fn remote_line(remote: &RemoteInfo) -> Line<'static> {
    Line::from(vec![
        Span::raw("    "),
        Span::styled(format!("{:<8}", remote.name), Style::default().fg(ACCENT())),
        Span::raw("  "),
        Span::raw(remote.url.clone()),
    ])
}

pub fn file_entry_line(entry: &FileEntry) -> Line<'static> {
    let label_style = match entry.label {
        "N" => Style::default().fg(SUCCESS()),
        "D" => Style::default().fg(DANGER()),
        "C" => Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
        "R" | "T" => Style::default().fg(ACCENT()),
        "?" => muted_style(),
        _ => Style::default().fg(WARNING()), // "M"
    };
    Line::from(vec![
        Span::raw(FILE_INDENT),
        Span::styled(format!("{:<FILE_LABEL_WIDTH$}", entry.label), label_style),
        Span::styled(entry.path.clone(), muted_style()),
    ])
}

// ── Low-level line builders ────────────────────────────────────────────────

/// `"Field:   "` padded to `FIELD_LABEL_WIDTH`.
fn field_label(name: &str) -> String {
    let mut s = format!("{}:", name);
    while s.chars().count() < FIELD_LABEL_WIDTH {
        s.push(' ');
    }
    s
}

fn field_line(name: &'static str, value: Span<'static>) -> Line<'static> {
    Line::from(vec![Span::raw(FIELD_INDENT), Span::styled(field_label(name), muted_style()), value])
}

fn kind_line(
    symbol: &'static str,
    color: ratatui::style::Color,
    title: &'static str,
    sub: &'static str,
) -> Line<'static> {
    Line::from(vec![
        Span::raw(FIELD_INDENT),
        Span::styled(symbol, Style::default().fg(color)),
        Span::raw("  "),
        Span::styled(title, primary_style()),
        Span::raw("  "),
        Span::styled(sub, muted_style()),
    ])
}

/// Renders a commit confirmation/input popup centered over `area`.
#[allow(clippy::too_many_arguments)]

fn graph_line_spans(line: &crate::repo::GraphLine) -> Line<'static> {
    let mut spans = Vec::new();

    // 1. Graph characters
    spans.push(Span::styled(line.graph.clone(), muted_style()));

    if let Some(ref c) = line.commit {
        // 2. Commit OID (short hash)
        let short_hash = if c.oid.len() >= 7 { &c.oid[0..7] } else { &c.oid };
        spans.push(Span::styled(format!("{} ", short_hash), accent_style()));

        // Verification signature status badge
        if !c.signature_status.is_empty() && c.signature_status != "N" {
            let (sig_char, sig_style) = match c.signature_status.as_str() {
                "G" => ("✓ ", Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)),
                "B" => ("✗ ", Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
                "U" | "X" | "Y" | "R" => ("✓ ", Style::default().fg(WARNING())),
                _ => ("? ", muted_style()),
            };
            spans.push(Span::styled(sig_char, sig_style));
        }

        // 3. Decorations (refs)
        if !c.decoration.is_empty() {
            let dec = c.decoration.trim();
            let dec_content = if dec.starts_with('(') && dec.ends_with(')') {
                &dec[1..dec.len() - 1]
            } else {
                dec
            };

            spans.push(Span::styled("(", muted_style()));
            let mut first = true;
            for ref_item in dec_content.split(", ") {
                if !first {
                    spans.push(Span::styled(", ", muted_style()));
                }
                first = false;

                if let Some(stripped) = ref_item.strip_prefix("HEAD -> ") {
                    spans.push(Span::styled(
                        "HEAD -> ",
                        Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD),
                    ));
                    spans.push(Span::styled(
                        stripped.to_string(),
                        Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
                    ));
                } else if let Some(stripped) = ref_item.strip_prefix("tag: ") {
                    spans.push(Span::styled("tag: ", Style::default().fg(WARNING())));
                    spans.push(Span::styled(
                        stripped.to_string(),
                        Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
                    ));
                } else if ref_item.contains('/') {
                    spans.push(Span::styled(ref_item.to_string(), Style::default().fg(DANGER())));
                } else {
                    spans.push(Span::styled(ref_item.to_string(), Style::default().fg(SUCCESS())));
                }
            }
            spans.push(Span::styled(") ", muted_style()));
        }

        // 4. Commit Summary
        spans.push(Span::styled(c.summary.clone(), primary_style()));

        // 5. Author and Date
        spans.push(Span::styled(" - ", muted_style()));
        spans.push(Span::styled(c.author.clone(), muted_style()));
        spans.push(Span::styled(format!(" ({})", c.date), muted_style()));
    }

    Line::from(spans)
}

#[allow(clippy::too_many_arguments)]

pub fn read_file_content(path: &std::path::Path) -> Result<String, std::io::Error> {
    use std::io::Read;
    let file = std::fs::File::open(path)?;
    let mut buffer = Vec::new();
    file.take(100_000).read_to_end(&mut buffer)?;
    let content = String::from_utf8(buffer)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(content)
}
