//! Full-screen detail view for the currently-selected item.
//!
//! Reads a snapshot prepared by `repo::inspect_detail` and renders it as a
//! padded paragraph in the body of the outer Twig frame. Drawing is pure —
//! all git2 work happens once in `App::open_detail`, not per frame.
//!
//! The view is divided into labelled sections (Overview, Repository, Sync,
//! Working Tree) separated by blank lines and introduced by an accent-coloured
//! `▍ Title` header line. Within "Working Tree", changed files are listed
//! under Staged / Unstaged / Untracked / Conflicts sub-headers.

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
    CommitEntry, DiffLine, DiffLineKind, FileEntry, ItemDetail, RemoteInfo, RepoInfo,
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
}

/// Renders the detail view into `area` and records panel bounds in `areas`.
#[allow(clippy::too_many_arguments)]
pub fn draw(
    f: &mut Frame,
    item_name: &str,
    detail: &ItemDetail,
    mode: &Mode,
    focus: &DetailSection,
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
    area: Rect,
) {
    if mode == &Mode::Inspect {
        if let ItemDetail::Repo { info, .. } = detail {
            let dirty = !info.changes.staged.is_empty()
                || !info.changes.unstaged.is_empty()
                || !info.changes.untracked.is_empty()
                || !info.changes.conflicted.is_empty();
            let is_uncommitted = dirty && commit_selection == 0;

            if is_uncommitted {
                draw_staging_panels(
                    f,
                    &info.changes,
                    *focus,
                    staging_file_selection,
                    file_diff,
                    diff_scroll,
                    areas,
                    inspect_horizontal_split_pct,
                    inspect_vertical_split_pct,
                    area,
                );
                return;
            } else {
                let commit_idx = if dirty {
                    commit_selection.saturating_sub(1)
                } else {
                    commit_selection
                };
                if let Some(commit) = info.commits.get(commit_idx) {
                    draw_inspect_window(
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
        Span::styled("⎇  ", muted_style()),
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

        let fade_outer = Style::default()
            .fg(ratatui::style::Color::DarkGray)
            .add_modifier(Modifier::DIM);
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
            spans.push(Span::styled(
                format!("{} {} [{}] {}", bullet, name, index, bullet),
                style,
            ));
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
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(body_area);

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

                draw_detail_commits(
                    f,
                    info,
                    *focus,
                    commit_selection,
                    commit_search_query,
                    detail_chunks[0],
                );
                areas.commits = Some(detail_chunks[0]);

                if is_uncommitted_row {
                    // <uncommitted> row selected — show working-tree staging panels.
                    draw_staging_panels(
                        f,
                        &info.changes,
                        *focus,
                        staging_file_selection,
                        file_diff,
                        diff_scroll,
                        areas,
                        40,
                        50,
                        detail_chunks[1],
                    );
                } else {
                    // Real commit selected — show its changed files.
                    let commit_idx = if dirty {
                        commit_selection.saturating_sub(1)
                    } else {
                        commit_selection
                    };
                    match info.commits.get(commit_idx) {
                        Some(commit) => {
                            draw_commit_files_panel(
                                f,
                                commit,
                                *focus,
                                file_selection,
                                file_diff,
                                diff_scroll,
                                commit_details_scroll,
                                areas,
                                detail_chunks[1],
                            );
                        }
                        None => {
                            // Fallback: selection out of range, show staging panels.
                            draw_staging_panels(
                                f,
                                &info.changes,
                                *focus,
                                staging_file_selection,
                                file_diff,
                                diff_scroll,
                                areas,
                                40,
                                50,
                                detail_chunks[1],
                            );
                        }
                    }
                }
            } else if detail_tab == 1 {
                // Render Files view (tab 2, index 1)
                draw_files_view(
                    f,
                    resolved,
                    info,
                    visible_files,
                    *focus,
                    file_list_selection,
                    areas,
                    body_area,
                );
            } else if detail_tab == 2 {
                // Render Graph view (tab 3, index 2)
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

                let visible_height = inner.height as usize;
                let upper = (graph_scroll + visible_height).min(info.graph_lines.len());
                let visible_lines = &info.graph_lines[graph_scroll..upper];

                let mut list_lines = Vec::new();
                for g_line in visible_lines {
                    list_lines.push(graph_line_spans(g_line));
                }

                let paragraph = Paragraph::new(list_lines).wrap(Wrap { trim: false });
                f.render_widget(paragraph, inner);
            } else if detail_tab == 3 {
                // Render Branches view (tab 4, index 3)
                draw_branches_view(
                    f,
                    info,
                    *focus,
                    local_branch_selection,
                    remote_branch_selection,
                    areas,
                    body_area,
                );
            } else if detail_tab == 4 {
                // Render Tags view (tab 5, index 4)
                draw_tags_view(
                    f,
                    info,
                    *focus,
                    local_tag_selection,
                    info.remote_tags_loaded,
                    areas,
                    body_area,
                );
            } else if detail_tab == 5 {
                // Render Remotes view (tab 6, index 5)
                draw_remotes_view(f, info, *focus, remote_selection, areas, body_area);
            } else if detail_tab == 6 {
                // Render Stashes view (tab 7, index 6)
                draw_stashes_view(
                    f,
                    info,
                    *focus,
                    stash_selection,
                    stash_file_selection,
                    file_diff,
                    diff_scroll,
                    areas,
                    body_area,
                );
            } else {
                // Render Overview tab (tab 8, index 7)
                draw_overview_tab(f, resolved, info, body_area);
            }
            // Draw detail help overlay on top when requested.
            if matches!(mode, Mode::DetailHelp) {
                draw_detail_help_overlay(f, body_area, help_scroll);
            }
            // Draw commit popup on top when requested.
            if matches!(mode, Mode::CommitInput) {
                draw_commit_popup(
                    f,
                    input_buffer,
                    commit_editing,
                    commit_amend,
                    commit_input_scroll,
                    body_area,
                );
            }
            // Draw branch create popup on top when requested.
            if matches!(mode, Mode::BranchCreateInput) {
                draw_branch_create_popup(f, input_buffer, branch.as_deref(), body_area);
            }
            // Draw tag create popup on top when requested.
            if matches!(mode, Mode::TagCreateInput) {
                draw_tag_create_popup(f, input_buffer, tag_action_target_oid.as_deref(), body_area);
            }
            // Draw branch delete popup on top when requested.
            if matches!(mode, Mode::BranchDeleteConfirm) {
                draw_branch_delete_popup(f, branch_action_target, body_area);
            }
            // Draw branch push popup on top when requested.
            if matches!(mode, Mode::BranchPushConfirm) {
                draw_branch_push_popup(f, branch_action_target, body_area);
            }
            // Draw branch merge popup on top when requested.
            if matches!(mode, Mode::BranchMergeConfirm) {
                draw_branch_merge_popup(f, branch_action_target, branch.as_deref(), body_area);
            }
            // Draw branch rebase popup on top when requested.
            if matches!(mode, Mode::BranchRebaseConfirm) {
                draw_branch_rebase_popup(f, branch_action_target, branch.as_deref(), body_area);
            }
            // Draw branch interactive rebase popup on top when requested.
            if matches!(mode, Mode::BranchInteractiveRebaseConfirm) {
                draw_branch_interactive_rebase_popup(
                    f,
                    branch_action_target,
                    branch.as_deref(),
                    body_area,
                );
            }
            // Draw tag delete popup on top when requested.
            if matches!(mode, Mode::TagDeleteConfirm) {
                draw_tag_delete_popup(f, tag_delete_target, body_area);
            }
            // Draw tag push popup on top when requested.
            if matches!(mode, Mode::TagPushConfirm) {
                draw_tag_push_popup(f, tag_push_target, body_area);
            }
            // Draw tag push all popup on top when requested.
            if matches!(mode, Mode::TagPushAllConfirm) {
                draw_tag_push_all_popup(f, body_area);
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
                draw_stash_delete_popup(f, &stash_name, body_area);
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
                draw_stash_apply_popup(f, &stash_name, stash_apply_delete_after, body_area);
            }
            // Draw remote picker popup on top when requested.
            if matches!(mode, Mode::RemotePicker) {
                if let ItemDetail::Repo { info, .. } = detail {
                    draw_remote_picker_popup(f, &info.remotes, remote_picker_selection, body_area);
                }
            }
            // Draw discard changes popup on top when requested.
            if matches!(mode, Mode::DiscardChangesConfirm) {
                draw_discard_changes_popup(f, discard_target, body_area);
            }
        }
        _ => {
            let body_lines = build_body(detail);
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
#[allow(clippy::too_many_arguments)]
fn draw_commit_files_panel(
    f: &mut Frame,
    commit: &CommitEntry,
    focus: DetailSection,
    file_selection: usize,
    file_diff: &[DiffLine],
    diff_scroll: usize,
    commit_details_scroll: usize,
    areas: &mut DetailAreas,
    area: Rect,
) {
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    areas.bottom_right = Some(panels[1]);
    areas.staged_sub = None;
    areas.unstaged_sub = None;

    let left_focused = focus == DetailSection::Staged || focus == DetailSection::Unstaged;
    let right_focused = focus == DetailSection::StagingDetails;

    // Split left panel horizontally: top is Changed Files list, bottom is Commit Details
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(panels[0]);

    areas.bottom_left = Some(left_chunks[0]);
    areas.commit_details = Some(left_chunks[1]);

    // ── Left Top: changed files ───────────────────────────────────────────────
    let left_border_style = if left_focused {
        Style::default().fg(ACCENT())
    } else {
        muted_style()
    };
    let left_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(left_border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Changed Files", primary_style()),
            Span::raw("  "),
            Span::styled(format!("({})", commit.files.len()), muted_style()),
            Span::raw(" "),
        ]));
    let left_inner = left_block.inner(left_chunks[0]);
    f.render_widget(left_block, left_chunks[0]);

    if commit.files.is_empty() {
        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(left_inner);
        f.render_widget(
            Paragraph::new(Span::styled("No files changed", muted_style()))
                .alignment(Alignment::Center),
            v[1],
        );
    } else {
        let items: Vec<ListItem> = commit
            .files
            .iter()
            .map(|f| ListItem::new(file_entry_line(f)))
            .collect();
        let list =
            List::new(items).highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        let mut state = ListState::default();
        if left_focused {
            state.select(Some(file_selection));
        }
        f.render_stateful_widget(list, left_inner, &mut state);
    }

    // ── Left Bottom: commit details ───────────────────────────────────────────
    draw_commit_details_widget(
        f,
        commit,
        focus == DetailSection::CommitDetails,
        commit_details_scroll,
        left_chunks[1],
    );
    areas.commit_details = Some(left_chunks[1]);

    // ── Right: diff panel ─────────────────────────────────────────────────
    let right_border_style = if right_focused {
        Style::default().fg(ACCENT())
    } else {
        muted_style()
    };
    let right_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(right_border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Diff", primary_style()),
            if right_focused && diff_scroll > 0 {
                Span::styled(format!("  ↕ line {}", diff_scroll + 1), muted_style())
            } else {
                Span::raw("")
            },
            if right_focused {
                Span::styled("  ↑↓ scroll", muted_style())
            } else {
                Span::raw("")
            },
            Span::raw(" "),
        ]));
    let right_inner = right_block.inner(panels[1]);
    f.render_widget(right_block, panels[1]);

    if file_diff.is_empty() {
        let v_center = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(45),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(right_inner);
        f.render_widget(
            Paragraph::new(Span::styled(
                "Select a file to view its diff",
                muted_style(),
            ))
            .alignment(Alignment::Center),
            v_center[1],
        );
    } else {
        let diff_spans: Vec<Line> = file_diff
            .iter()
            .map(|line| {
                let style = match line.kind {
                    DiffLineKind::Added => Style::default().fg(SUCCESS()),
                    DiffLineKind::Removed => Style::default().fg(DANGER()),
                    DiffLineKind::Header => Style::default().fg(ratatui::style::Color::Cyan),
                    DiffLineKind::Context => Style::default(),
                };
                Line::from(Span::styled(line.content.clone(), style))
            })
            .collect();
        f.render_widget(
            Paragraph::new(diff_spans)
                .scroll((diff_scroll as u16, 0))
                .wrap(Wrap { trim: false }),
            right_inner,
        );
    }
}

// ── Staging panels ─────────────────────────────────────────────────────────

/// Renders two side-by-side panels: \"Staging Area\" (left, split into Staged/Unstaged)
/// and \"Staging Details\" (right, diff of selected file).
#[allow(clippy::too_many_arguments)]
fn draw_staging_panels(
    f: &mut Frame,
    changes: &WorktreeChanges,
    focus: DetailSection,
    staging_file_selection: usize,
    file_diff: &[DiffLine],
    diff_scroll: usize,
    areas: &mut DetailAreas,
    inspect_horizontal_split_pct: u16,
    inspect_vertical_split_pct: u16,
    area: Rect,
) {
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(inspect_horizontal_split_pct),
            Constraint::Percentage(100 - inspect_horizontal_split_pct),
        ])
        .split(area);

    areas.bottom_left = Some(panels[0]);
    areas.bottom_right = Some(panels[1]);
    areas.commit_details = None;

    // Record horizontal splitter boundary
    let split_col = area.x + panels[0].width;
    areas.inspect_horizontal_splitter = Some(Rect::new(
        split_col.saturating_sub(1),
        area.y,
        2,
        area.height,
    ));

    // Focus-aware border helpers.
    let left_focused = focus == DetailSection::Staged || focus == DetailSection::Unstaged;
    let right_focused = focus == DetailSection::StagingDetails;

    // ── Left panel: outer border labelled "Staging Area" ──────────────────
    let left_border_style = if left_focused {
        Style::default().fg(ACCENT())
    } else {
        muted_style()
    };
    let left_outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(left_border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Staging Area", primary_style()),
            Span::raw(" "),
        ]));
    let left_inner = left_outer.inner(panels[0]);
    f.render_widget(left_outer, panels[0]);

    // Split left inner vertically: top = Staged, bottom = Unstaged
    let left_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(inspect_vertical_split_pct),
            Constraint::Percentage(100 - inspect_vertical_split_pct),
        ])
        .split(left_inner);

    // Record vertical splitter boundary in left inner
    let split_row = left_inner.y + left_split[0].height;
    areas.inspect_vertical_splitter = Some(Rect::new(
        left_inner.x,
        split_row.saturating_sub(1),
        left_inner.width,
        2,
    ));

    areas.staged_sub = Some(left_split[0]);
    areas.unstaged_sub = Some(left_split[1]);

    draw_file_subpanel(
        f,
        "Staged",
        SUCCESS(),
        &changes.staged,
        "Nothing staged",
        Borders::BOTTOM,
        focus == DetailSection::Staged,
        if focus == DetailSection::Staged {
            Some(staging_file_selection)
        } else if focus == DetailSection::Commits && !changes.staged.is_empty() {
            Some(0)
        } else {
            None
        },
        left_split[0],
    );
    draw_file_subpanel(
        f,
        "Unstaged",
        WARNING(),
        &changes.unstaged,
        "No unstaged changes",
        Borders::empty(),
        focus == DetailSection::Unstaged,
        if focus == DetailSection::Unstaged {
            Some(staging_file_selection)
        } else if focus == DetailSection::Commits
            && changes.staged.is_empty()
            && !changes.unstaged.is_empty()
        {
            Some(0)
        } else {
            None
        },
        left_split[1],
    );

    // ── Right panel – Staging Details ─────────────────────────────────────
    let right_border_style = if right_focused {
        Style::default().fg(ACCENT())
    } else {
        muted_style()
    };
    let selected_file_name: Option<String> = {
        let (files, idx) = match focus {
            DetailSection::Staged => (Some(&changes.staged), staging_file_selection),
            DetailSection::Unstaged => (Some(&changes.unstaged), staging_file_selection),
            _ => {
                if !changes.staged.is_empty() {
                    (Some(&changes.staged), 0)
                } else if !changes.unstaged.is_empty() {
                    (Some(&changes.unstaged), 0)
                } else {
                    (None, 0)
                }
            }
        };
        files.and_then(|f| f.get(idx)).map(|e| e.path.clone())
    };
    let right_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(right_border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Staging Details", primary_style()),
            if let Some(ref name) = selected_file_name {
                Span::styled(format!("  {}", name), muted_style())
            } else {
                Span::raw("")
            },
            Span::raw(" "),
        ]));
    let right_inner = right_block.inner(panels[1]);
    f.render_widget(right_block, panels[1]);

    if file_diff.is_empty() {
        let v_center = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(45),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(right_inner);
        f.render_widget(
            Paragraph::new(Span::styled(
                "Select a file to view its diff",
                muted_style(),
            ))
            .alignment(Alignment::Center),
            v_center[1],
        );
    } else {
        let diff_spans: Vec<Line> = file_diff
            .iter()
            .map(|line| {
                let style = match line.kind {
                    DiffLineKind::Added => Style::default().fg(SUCCESS()),
                    DiffLineKind::Removed => Style::default().fg(DANGER()),
                    DiffLineKind::Header => Style::default().fg(ratatui::style::Color::Cyan),
                    DiffLineKind::Context => Style::default(),
                };
                Line::from(Span::styled(line.content.clone(), style))
            })
            .collect();
        f.render_widget(
            Paragraph::new(diff_spans)
                .scroll((diff_scroll as u16, 0))
                .wrap(Wrap { trim: false }),
            right_inner,
        );
    }
}

/// Renders a titled sub-panel listing `files`, or a centred placeholder if empty.
/// `selection` — when `Some(idx)` the panel is focused and the file at `idx` is highlighted.
#[allow(clippy::too_many_arguments)]
fn draw_file_subpanel(
    f: &mut Frame,
    title: &'static str,
    title_color: ratatui::style::Color,
    files: &[FileEntry],
    empty_msg: &'static str,
    borders: Borders,
    focused: bool,
    selection: Option<usize>,
    area: Rect,
) {
    // When focused, highlight the title in accent; border stays muted (contained inside outer).
    let title_style = if focused {
        Style::default()
            .fg(ACCENT())
            .add_modifier(ratatui::style::Modifier::BOLD)
    } else {
        Style::default().fg(title_color)
    };
    let border_style = if focused {
        Style::default().fg(ACCENT())
    } else {
        muted_style()
    };
    // Sub-panel block — bottom border separates Staged from Unstaged.
    let block = Block::default()
        .borders(borders)
        .border_style(border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(title, title_style),
            Span::raw("  "),
            Span::styled(format!("({})", files.len()), muted_style()),
            Span::raw(" "),
        ]));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if files.is_empty() {
        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(inner);
        f.render_widget(
            Paragraph::new(Span::styled(empty_msg, muted_style())).alignment(Alignment::Center),
            v[1],
        );
        return;
    }

    if let Some(sel_idx) = selection {
        // Focused: render as a selectable list with highlight.
        let items: Vec<ListItem> = files
            .iter()
            .map(|e| ListItem::new(file_entry_line(e)))
            .collect();
        let list =
            List::new(items).highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        let mut state = ListState::default();
        state.select(Some(sel_idx));
        f.render_stateful_widget(list, inner, &mut state);
    } else {
        // Not focused: plain paragraph.
        let file_lines: Vec<Line<'static>> = files.iter().map(file_entry_line).collect();
        f.render_widget(Paragraph::new(file_lines).wrap(Wrap { trim: false }), inner);
    }
}

// ── Overview popup ─────────────────────────────────────────────────────────

/// Renders the repo overview as a floating popup centred over `area`.
fn draw_overview_tab(f: &mut Frame, resolved: &std::path::Path, info: &RepoInfo, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

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
    let body = Paragraph::new(body_lines)
        .block(left_block)
        .wrap(Wrap { trim: false });
    f.render_widget(body, chunks[0]);

    let right_title = if info.committer_stats_limit_reached {
        "Stats (last 10k commits)"
    } else {
        "Stats"
    };

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

    let stats_lines = build_committer_stats_lines(info);
    let stats_body = Paragraph::new(stats_lines)
        .block(right_block)
        .wrap(Wrap { trim: false });
    f.render_widget(stats_body, chunks[1]);
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
        for stat in &info.committer_stats {
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
                format!(
                    "{} commit{}",
                    stat.count,
                    if stat.count == 1 { "" } else { "s" }
                ),
                Style::default().fg(SUCCESS()),
            ));

            lines.push(Line::from(stat_spans));
        }
    }

    lines
}

/// Returns a [`Rect`] that is `percent_x` wide and `percent_y` tall, centred in `r`.
fn centred_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

// ── Detail help overlay ────────────────────────────────────────────────────

pub(crate) const DETAIL_HELP_LINES: &[(&str, &str)] = &[
    (
        "↑ [Up] / k",
        "Select previous commit / file / branch / file tree item",
    ),
    (
        "↓ [Down] / j",
        "Select next commit / file / branch / file tree item",
    ),
    ("⇞ [PgUp]", "Jump 10 rows up"),
    ("⇟ [PgDn]", "Jump 10 rows down"),
    ("⇥ [Tab] / ⇧⇥", "Cycle detail view tabs"),
    ("w / W", "Cycle panel focus (Workspace / Branches tabs)"),
    ("← / →", "Focus Local/Remote branch (Branches tab)"),
    ("← / → or < / >", "Collapse/Expand folder (Files tab)"),
    (
        "↵ [Enter]",
        "Stage/Unstage file, Checkout branch, or Checkout tag",
    ),
    ("c", "Commit changes (Workspace) / Create branch (Branches)"),
    ("t", "Create tag (Workspace tab commits list)"),
    (
        "i",
        "Interactive rebase from selected commit (Workspace tab commits list)",
    ),
    ("/", "Filter commits list by search query (Workspace tab)"),
    ("d", "Delete selected branch (Branches) / tag (Tags)"),
    ("m", "Merge selected branch into current branch (Branches)"),
    ("r", "Rebase current branch onto selected branch (Branches)"),
    ("1", "Go to Workspace tab"),
    ("2", "Go to Files tab"),
    ("3", "Go to Graph View tab"),
    ("4", "Go to Branches tab"),
    ("5", "Go to Tags tab"),
    ("6", "Go to Remotes tab"),
    ("7", "Go to Stashes tab"),
    ("8", "Go to Overview tab"),
    ("f / F", "Fetch selected remote (Remotes tab)"),
    ("⇧F [Shift+F]", "Fetch selected local branch's upstream"),
    ("p", "Pull branch (Branches) / Push tag (Tags)"),
    (
        "⇧P [Shift+P]",
        "Push branch (Branches) / Push all tags (Tags)",
    ),
    ("? / ⎋ [Esc]", "Close this help"),
    ("q / ⎋ [Esc]", "Back to repository list"),
    (
        "→ [Right]",
        "Inspect selected commit (Workspace commits list)",
    ),
    ("⎋ [Esc]", "Back to workspace commits list (Inspect mode)"),
    (
        "Left-Click",
        "Focus clicked panel / change tab (mouse support)",
    ),
];

/// Renders a floating shortcut reference overlay centred over `area`.
fn draw_detail_help_overlay(f: &mut Frame, area: Rect, scroll: usize) {
    let popup_area = centred_rect(60, 55, area);
    f.render_widget(Clear, popup_area);

    let key_width = DETAIL_HELP_LINES
        .iter()
        .map(|(k, _)| k.chars().count())
        .max()
        .unwrap_or(0);

    let mut lines: Vec<Line> = vec![Line::from("")];
    for (key, desc) in DETAIL_HELP_LINES {
        let padded = format!("{:>width$}", key, width = key_width);
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                padded,
                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            Span::raw((*desc).to_string()),
        ]));
    }
    lines.push(Line::from(""));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
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
    let para = Paragraph::new(lines)
        .block(block)
        .scroll((scroll as u16, 0));
    f.render_widget(para, popup_area);

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

fn draw_detail_commits(
    f: &mut Frame,
    info: &RepoInfo,
    focus: DetailSection,
    commit_selection: usize,
    commit_search_query: &Option<String>,
    area: Rect,
) {
    let focused = focus == DetailSection::Commits;
    let border_style = if focused {
        Style::default().fg(ACCENT())
    } else {
        muted_style()
    };

    let title_spans = if let Some(q) = commit_search_query {
        vec![
            Span::raw(" "),
            Span::styled("Commits (Filter: ", primary_style()),
            Span::styled(q.clone(), accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(")", primary_style()),
            Span::raw(" "),
        ]
    } else {
        vec![
            Span::raw(" "),
            Span::styled("Commits", primary_style()),
            Span::raw(" "),
        ]
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(Line::from(title_spans));

    // Filter commits based on search query
    let filtered_commits: Vec<&crate::repo::CommitEntry> = if let Some(query) = commit_search_query
    {
        let q = query.to_lowercase();
        info.commits
            .iter()
            .filter(|c| {
                c.id.to_lowercase().contains(&q)
                    || c.author.to_lowercase().contains(&q)
                    || c.when.to_lowercase().contains(&q)
                    || c.summary.to_lowercase().contains(&q)
            })
            .collect()
    } else {
        info.commits.iter().collect()
    };

    // Dirty = any uncommitted change in staged / unstaged / untracked / conflicted.
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

    // Show empty placeholder only when truly empty (no commits AND clean).
    if filtered_commits.is_empty() && !show_dirty {
        let inner = block.inner(area);
        f.render_widget(block, area);
        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(inner);
        f.render_widget(
            Paragraph::new(Span::styled(
                "No commits yet / empty repository",
                muted_style(),
            ))
            .alignment(Alignment::Center),
            v[1],
        );
        return;
    }

    let header = Row::new(vec![
        Cell::from("ID"),
        Cell::from("Author"),
        Cell::from("Date"),
        Cell::from("Summary"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD).fg(ACCENT()));

    // Prepend a virtual "uncommitted changes" row when the worktree is dirty.
    let mut rows: Vec<Row> = Vec::new();
    if show_dirty {
        rows.push(Row::new(vec![
            Cell::from(Span::styled("-", muted_style())),
            Cell::from(Span::styled("-", muted_style())),
            Cell::from(Span::styled("-", muted_style())),
            Cell::from(Span::styled(
                "<uncommitted>",
                Style::default().fg(WARNING()),
            )),
        ]));
    }
    rows.extend(filtered_commits.iter().map(|commit| {
        // Build the summary cell: optional ref badges then the commit message.
        let mut spans: Vec<Span<'static>> = Vec::new();
        for r in &commit.refs {
            let (label, style) = if let Some(tag) = r.strip_prefix("tag:") {
                // Tag — yellow
                (
                    format!("[{}]", tag),
                    Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
                )
            } else if let Some(remote) = r.strip_prefix("remote:") {
                // Remote tracking branch — green
                (
                    format!("[{}]", remote),
                    Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD),
                )
            } else {
                // Local branch — cyan
                (
                    format!("[{}]", r),
                    Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
                )
            };
            spans.push(Span::styled(label, style));
            spans.push(Span::raw(" "));
        }
        spans.push(Span::styled(commit.summary.clone(), Style::default()));

        Row::new(vec![
            Cell::from(Span::styled(
                commit.id.clone(),
                Style::default().fg(WARNING()),
            )),
            Cell::from(Span::styled(commit.author.clone(), Style::default())),
            Cell::from(Span::styled(commit.when.clone(), muted_style())),
            Cell::from(Line::from(spans)),
        ])
    }));

    let widths = [
        Constraint::Length(9),  // "c7a45e2" + 2 padding
        Constraint::Length(18), // Author name
        Constraint::Length(16), // Date
        Constraint::Min(20),    // Summary
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .column_spacing(2);

    let mut state = TableState::default();
    if focused {
        state.select(Some(commit_selection));
    }
    f.render_stateful_widget(table, area, &mut state);
}

// ── Body builder ───────────────────────────────────────────────────────────

fn build_body(detail: &ItemDetail) -> Vec<Line<'static>> {
    match detail {
        ItemDetail::Missing { resolved } => {
            let mut lines = vec![];
            push_section_header(&mut lines, "Overview");
            lines.push(kind_line(
                "✕",
                DANGER(),
                "Not a directory",
                "(path does not exist or isn't accessible)",
            ));
            lines.push(field_line(
                "Path",
                Span::raw(resolved.display().to_string()),
            ));
            lines
        }
        ItemDetail::Directory { resolved } => {
            let mut lines = vec![];
            push_section_header(&mut lines, "Overview");
            lines.push(kind_line(
                "○",
                WARNING(),
                "Plain directory",
                "(exists, but no .git entry was found)",
            ));
            lines.push(field_line(
                "Path",
                Span::raw(resolved.display().to_string()),
            ));
            lines
        }
        ItemDetail::Error { resolved, message } => {
            let mut lines = vec![];
            push_section_header(&mut lines, "Overview");
            lines.push(kind_line(
                "⚠",
                WARNING(),
                "Could not read repository",
                "(libgit2 reported an error — see below)",
            ));
            lines.push(field_line(
                "Path",
                Span::raw(resolved.display().to_string()),
            ));
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
    lines.push(kind_line(
        "●",
        SUCCESS(),
        "Git Repository",
        "(inspectable libgit2)",
    ));
    lines.push(field_line(
        "Path",
        Span::raw(resolved.display().to_string()),
    ));
    let branch = info
        .branch
        .clone()
        .unwrap_or_else(|| "(detached HEAD)".to_string());
    lines.push(field_line("Branch", Span::styled(branch, accent_style())));

    // ── HEAD Commit ───────────────────────────────────────────────────────
    push_section_header(&mut lines, "HEAD Commit");
    if let Some(head) = &info.head {
        lines.push(field_line(
            "Hash",
            Span::styled(head.short_id.clone(), Style::default().fg(WARNING())),
        ));
        lines.push(field_line(
            "Message",
            Span::styled(head.summary.clone(), primary_style()),
        ));
        lines.push(field_line("Author", Span::raw(head.author.clone())));
        lines.push(field_line("Date", Span::raw(head.when.clone())));
    } else {
        lines.push(field_line(
            "HEAD",
            Span::styled("(empty repository)", muted_style()),
        ));
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
            lines.push(field_line(
                "Upstream",
                Span::styled("(not configured)", muted_style()),
            ));
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
                let mut spans = vec![
                    Span::raw(FIELD_INDENT),
                    Span::styled(field_label("Sync"), muted_style()),
                ];
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
        for r in &info.remotes {
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

fn file_entry_line(entry: &FileEntry) -> Line<'static> {
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
    Line::from(vec![
        Span::raw(FIELD_INDENT),
        Span::styled(field_label(name), muted_style()),
        value,
    ])
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
fn draw_commit_popup(
    f: &mut Frame,
    input_buffer: &str,
    editing: bool,
    commit_amend: bool,
    scroll: usize,
    area: Rect,
) {
    let popup_area = centred_rect(60, 25, area);
    f.render_widget(Clear, popup_area);

    let border_color = if editing { ACCENT() } else { WARNING() };
    let border_style = Style::default().fg(border_color);

    let title_text = if editing {
        " Commit Message "
    } else {
        " Confirm Commit "
    };

    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled(title_text, primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let text = if input_buffer.is_empty() {
        Paragraph::new(Span::styled("(type commit message here...)", muted_style()))
            .wrap(Wrap { trim: true })
            .scroll((scroll as u16, 0))
    } else {
        Paragraph::new(input_buffer)
            .wrap(Wrap { trim: true })
            .scroll((scroll as u16, 0))
    };

    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // Split inner area vertically: top is the commit message text area, bottom is the amend option.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner_area);

    f.render_widget(text, chunks[0]);

    // Render the amend checkbox.
    let checkbox = if commit_amend { "[X]" } else { "[ ]" };
    let checkbox_style = if commit_amend {
        Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)
    } else {
        muted_style()
    };
    let checkbox_line = Line::from(vec![
        Span::styled(format!("{} ", checkbox), checkbox_style),
        Span::styled("Amend last commit", primary_style()),
        if !editing {
            Span::styled(" (toggle: [a/space])", muted_style())
        } else {
            Span::raw("")
        },
    ]);
    f.render_widget(Paragraph::new(checkbox_line), chunks[1]);

    if editing {
        let lines: Vec<&str> = input_buffer.split('\n').collect();
        let last_line = lines.last().copied().unwrap_or("");
        let line_count = lines.len();
        let cursor_y = chunks[0]
            .y
            .saturating_add(line_count.saturating_sub(1) as u16)
            .min(
                chunks[0]
                    .y
                    .saturating_add(chunks[0].height.saturating_sub(1)),
            );
        let cursor_offset = last_line.chars().count() as u16;
        let cursor_x = chunks[0]
            .x
            .saturating_add(cursor_offset.min(chunks[0].width.saturating_sub(1)));
        f.set_cursor_position(ratatui::layout::Position::new(cursor_x, cursor_y));
    }
}

fn graph_line_spans(line: &crate::repo::GraphLine) -> Line<'static> {
    let mut spans = Vec::new();

    // 1. Graph characters
    spans.push(Span::styled(line.graph.clone(), muted_style()));

    if let Some(ref c) = line.commit {
        // 2. Commit OID (short hash)
        let short_hash = if c.oid.len() >= 7 {
            &c.oid[0..7]
        } else {
            &c.oid
        };
        spans.push(Span::styled(format!("{} ", short_hash), accent_style()));

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
                    spans.push(Span::styled(
                        ref_item.to_string(),
                        Style::default().fg(DANGER()),
                    ));
                } else {
                    spans.push(Span::styled(
                        ref_item.to_string(),
                        Style::default().fg(SUCCESS()),
                    ));
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

fn draw_branches_view(
    f: &mut Frame,
    info: &RepoInfo,
    focus: DetailSection,
    local_branch_selection: usize,
    remote_branch_selection: usize,
    areas: &mut DetailAreas,
    area: Rect,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let left_area = chunks[0];
    let right_area = chunks[1];

    areas.local_branches = Some(left_area);
    areas.remote_branches = Some(right_area);

    // ── Local Branches Panel ──
    let local_focused = focus == DetailSection::LocalBranches;
    let local_border_style = if local_focused {
        Style::default().fg(ACCENT())
    } else {
        muted_style()
    };
    let local_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(local_border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Local Branches", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::uniform(1));

    let local_items: Vec<ListItem> = info
        .local_branches
        .iter()
        .map(|b| {
            let style = if b.is_head {
                Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let prefix = if b.is_head { " " } else { "  " };
            let mut spans = vec![
                Span::styled(prefix, Style::default().fg(SUCCESS())),
                Span::styled(b.name.clone(), style),
            ];
            if !b.short_sha.is_empty() {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(format!("[{}]", b.short_sha), accent_style()));
                if !b.short_message.is_empty() {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled("·", muted_style()));
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(b.short_message.clone(), muted_style()));
                }
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let local_list = List::new(local_items)
        .block(local_block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut local_state = ListState::default();
    if local_focused {
        local_state.select(Some(local_branch_selection));
    }
    f.render_stateful_widget(local_list, left_area, &mut local_state);

    // ── Remote Branches Panel ──
    let remote_focused = focus == DetailSection::RemoteBranches;
    let remote_border_style = if remote_focused {
        Style::default().fg(ACCENT())
    } else {
        muted_style()
    };
    let remote_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(remote_border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Remote Branches", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::uniform(1));

    let remote_items: Vec<ListItem> = info
        .remote_branches
        .iter()
        .map(|b| {
            let mut spans = vec![
                Span::raw("  "),
                Span::styled(b.name.clone(), primary_style()),
            ];
            if !b.short_sha.is_empty() {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(format!("[{}]", b.short_sha), accent_style()));
                if !b.short_message.is_empty() {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled("·", muted_style()));
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(b.short_message.clone(), muted_style()));
                }
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let remote_list = List::new(remote_items)
        .block(remote_block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut remote_state = ListState::default();
    if remote_focused {
        remote_state.select(Some(remote_branch_selection));
    }
    f.render_stateful_widget(remote_list, right_area, &mut remote_state);
}

#[allow(clippy::too_many_arguments)]
fn draw_files_view(
    f: &mut Frame,
    resolved: &std::path::Path,
    info: &RepoInfo,
    visible_files: &[crate::app::FileTreeItem],
    focus: DetailSection,
    file_list_selection: usize,
    areas: &mut DetailAreas,
    area: Rect,
) {
    areas.files = Some(area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    let focused = focus == DetailSection::Files;
    let border_style = if focused {
        Style::default().fg(ACCENT())
    } else {
        muted_style()
    };

    let left_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Repository Files", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::uniform(1));

    let items: Vec<ListItem> = visible_files
        .iter()
        .map(|item| {
            let indent = "  ".repeat(item.depth);
            let (prefix, style) = if item.is_dir {
                if item.is_expanded {
                    ("▼ ", primary_style())
                } else {
                    ("> ", primary_style())
                }
            } else {
                ("  🗎 ", muted_style())
            };

            ListItem::new(Line::from(vec![
                Span::raw(indent),
                Span::styled(prefix, style),
                Span::styled(item.name.clone(), primary_style()),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(left_block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = ListState::default();
    state.select(Some(file_list_selection));

    f.render_stateful_widget(list, chunks[0], &mut state);

    // Right panel: file preview or folder contents
    if let Some(selected_item) = visible_files.get(file_list_selection) {
        if selected_item.is_dir {
            // Selected item is a directory: list its direct contents
            let folder_name = if selected_item.name.is_empty() {
                "/"
            } else {
                &selected_item.name
            };
            let right_block = Block::default()
                .borders(Borders::ALL)
                .border_type(CARD_BORDER())
                .border_style(muted_style())
                .title(Line::from(vec![
                    Span::raw(" "),
                    Span::styled(format!("Contents of {}", folder_name), primary_style()),
                    Span::raw(" "),
                ]))
                .padding(Padding::uniform(1));

            let prefix = if selected_item.full_path.is_empty() {
                "".to_string()
            } else {
                format!("{}/", selected_item.full_path)
            };

            let mut direct_children = std::collections::BTreeSet::new();
            for f_path in &info.files {
                if f_path.starts_with(&prefix) {
                    let relative = &f_path[prefix.len()..];
                    if let Some(idx) = relative.find('/') {
                        let subdir = &relative[..idx];
                        direct_children.insert((subdir.to_string(), true));
                    } else {
                        direct_children.insert((relative.to_string(), false));
                    }
                }
            }

            let mut children_vec: Vec<(String, bool)> = direct_children.into_iter().collect();
            children_vec.sort_by(|a, b| match (a.1, b.1) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.0.cmp(&b.0),
            });

            let mut lines = Vec::new();
            if children_vec.is_empty() {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("(empty folder)", muted_style()),
                ]));
            } else {
                for (name, is_dir) in children_vec {
                    if is_dir {
                        lines.push(Line::from(vec![
                            Span::raw("  "),
                            Span::styled("📁 ", Style::default().fg(ACCENT())),
                            Span::styled(name, primary_style()),
                        ]));
                    } else {
                        lines.push(Line::from(vec![
                            Span::raw("  "),
                            Span::styled("🗎 ", muted_style()),
                            Span::raw(name),
                        ]));
                    }
                }
            }

            let body = Paragraph::new(lines)
                .block(right_block)
                .wrap(Wrap { trim: false });
            f.render_widget(body, chunks[1]);
        } else {
            // Selected item is a file: show its contents
            let right_block = Block::default()
                .borders(Borders::ALL)
                .border_type(CARD_BORDER())
                .border_style(muted_style())
                .title(Line::from(vec![
                    Span::raw(" "),
                    Span::styled(
                        format!("Content of {}", selected_item.name),
                        primary_style(),
                    ),
                    Span::raw(" "),
                ]))
                .padding(Padding::uniform(1));

            let file_path = resolved.join(&selected_item.full_path);
            let content_text = match read_file_content(&file_path) {
                Ok(content) => content,
                Err(e) => format!("Could not read file: {}", e),
            };
            let body = Paragraph::new(content_text)
                .block(right_block)
                .wrap(Wrap { trim: false });
            f.render_widget(body, chunks[1]);
        }
    } else {
        // Fallback: render an empty block on the right
        let right_block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(muted_style())
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Content", primary_style()),
                Span::raw(" "),
            ]));
        f.render_widget(right_block, chunks[1]);
    }
}

fn read_file_content(path: &std::path::Path) -> Result<String, std::io::Error> {
    use std::io::Read;
    let file = std::fs::File::open(path)?;
    let mut buffer = Vec::new();
    file.take(100_000).read_to_end(&mut buffer)?;
    let content = String::from_utf8(buffer)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(content)
}

fn draw_branch_create_popup(
    f: &mut Frame,
    input_buffer: &str,
    base_branch: Option<&str>,
    area: Rect,
) {
    let popup_area = centred_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Create Branch", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let base_name = base_branch.unwrap_or("HEAD");
    let content = vec![
        Line::from(vec![
            Span::styled("Base: ", muted_style()),
            Span::styled(base_name, primary_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("New Branch Name: ", muted_style()),
            Span::styled(input_buffer, primary_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let paragraph = Paragraph::new(content);
    f.render_widget(paragraph, inner_area);

    let cursor_y = inner_area.y.saturating_add(2).min(
        inner_area
            .y
            .saturating_add(inner_area.height.saturating_sub(1)),
    );
    let label_width = "New Branch Name: ".chars().count() as u16;
    let cursor_offset = label_width.saturating_add(input_buffer.chars().count() as u16);
    let cursor_x = inner_area.x.saturating_add(cursor_offset).min(
        inner_area
            .x
            .saturating_add(inner_area.width.saturating_sub(1)),
    );
    f.set_cursor_position(ratatui::layout::Position::new(cursor_x, cursor_y));
}

fn draw_branch_delete_popup(f: &mut Frame, target: &Option<(String, bool)>, area: Rect) {
    let popup_area = centred_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(DANGER());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Delete Branch", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let (branch_name, is_remote) = match target {
        Some((name, remote)) => (name.as_str(), *remote),
        None => ("", false),
    };

    let type_label = if is_remote {
        "remote-tracking branch"
    } else {
        "branch"
    };
    let content = vec![
        Line::from(vec![
            Span::styled("Are you sure you want to delete the ", primary_style()),
            Span::styled(type_label, accent_style()),
            Span::raw(":"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                branch_name,
                Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Confirm: ", muted_style()),
            Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" / Cancel: ", muted_style()),
            Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(content).block(block);
    f.render_widget(paragraph, popup_area);
}

fn draw_discard_changes_popup(f: &mut Frame, target: &Option<(String, bool)>, area: Rect) {
    let popup_area = centred_rect(60, 20, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(DANGER());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Discard Changes", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let (file_path, staged) = match target {
        Some((path, staged)) => (path.as_str(), *staged),
        None => ("", false),
    };

    let area_label = if staged { "staged" } else { "unstaged" };
    let content = vec![
        Line::from(vec![
            Span::styled("Are you sure you want to discard ", primary_style()),
            Span::styled(area_label, accent_style()),
            Span::styled(" changes in:", primary_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                file_path,
                Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "This operation is destructive and cannot be undone.",
            Style::default().fg(DANGER()),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Confirm: ", muted_style()),
            Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" / Cancel: ", muted_style()),
            Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(content).block(block);
    f.render_widget(paragraph, popup_area);
}

fn draw_branch_merge_popup(
    f: &mut Frame,
    target: &Option<(String, bool)>,
    current_branch: Option<&str>,
    area: Rect,
) {
    let popup_area = centred_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Merge Branch", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let (branch_name, is_remote) = match target {
        Some((name, remote)) => (name.as_str(), *remote),
        None => ("", false),
    };

    let type_label = if is_remote {
        "remote-tracking branch"
    } else {
        "branch"
    };

    let current = current_branch.unwrap_or("HEAD");

    let content = vec![
        Line::from(vec![
            Span::styled("Are you sure you want to merge the ", primary_style()),
            Span::styled(type_label, accent_style()),
            Span::raw(":"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                branch_name,
                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("into the current branch ", primary_style()),
            Span::styled(
                format!("'{}'", current),
                accent_style().add_modifier(Modifier::BOLD),
            ),
            Span::raw("?"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Confirm: ", muted_style()),
            Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" / Cancel: ", muted_style()),
            Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(content).block(block);
    f.render_widget(paragraph, popup_area);
}

fn draw_branch_rebase_popup(
    f: &mut Frame,
    target: &Option<(String, bool)>,
    current_branch: Option<&str>,
    area: Rect,
) {
    let popup_area = centred_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Rebase Branch", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let (branch_name, is_remote) = match target {
        Some((name, remote)) => (name.as_str(), *remote),
        None => ("", false),
    };

    let type_label = if is_remote {
        "remote-tracking branch"
    } else {
        "branch"
    };

    let current = current_branch.unwrap_or("HEAD");

    let content = vec![
        Line::from(vec![
            Span::styled("Are you sure you want to rebase the ", primary_style()),
            Span::styled(
                format!("current branch '{}'", current),
                accent_style().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("onto the ", primary_style()),
            Span::styled(type_label, accent_style()),
            Span::raw(":"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                branch_name,
                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Confirm: ", muted_style()),
            Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" / Cancel: ", muted_style()),
            Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(content).block(block);
    f.render_widget(paragraph, popup_area);
}

fn draw_branch_interactive_rebase_popup(
    f: &mut Frame,
    target: &Option<(String, bool)>,
    current_branch: Option<&str>,
    area: Rect,
) {
    let popup_area = centred_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Interactive Rebase Branch", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let (branch_name, is_remote) = match target {
        Some((name, remote)) => (name.as_str(), *remote),
        None => ("", false),
    };

    let type_label = if is_remote {
        "remote-tracking branch"
    } else {
        "branch"
    };

    let current = current_branch.unwrap_or("HEAD");

    let content = vec![
        Line::from(vec![
            Span::styled(
                "Are you sure you want to interactively rebase the ",
                primary_style(),
            ),
            Span::styled(
                format!("current branch '{}'", current),
                accent_style().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("onto the ", primary_style()),
            Span::styled(type_label, accent_style()),
            Span::raw(":"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                branch_name,
                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Confirm: ", muted_style()),
            Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" / Cancel: ", muted_style()),
            Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(content).block(block);
    f.render_widget(paragraph, popup_area);
}

fn draw_remote_picker_popup(f: &mut Frame, remotes: &[RemoteInfo], selection: usize, area: Rect) {
    let popup_area = centred_rect(50, 60, area);

    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Select Remote", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // Split inner: list on top, hint at bottom.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let items: Vec<ListItem> = remotes
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let style = if i == selection {
                accent_style().add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                primary_style()
            };
            ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(r.name.clone(), style),
                Span::styled("  ", muted_style()),
                Span::styled(r.url.clone(), muted_style()),
            ]))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(selection));
    f.render_stateful_widget(List::new(items), chunks[0], &mut list_state);

    let hint = Line::from(vec![
        Span::styled("↑↓ navigate  ", muted_style()),
        Span::styled("Enter", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" confirm  ", muted_style()),
        Span::styled("Esc", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" cancel", muted_style()),
    ]);
    f.render_widget(Paragraph::new(hint), chunks[1]);
}

fn draw_branch_push_popup(f: &mut Frame, target: &Option<(String, bool)>, area: Rect) {
    let popup_area = centred_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Push Branch", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let branch_name = match target {
        Some((name, _)) => name.as_str(),
        None => "",
    };

    let content = vec![
        Line::from(vec![
            Span::styled("Are you sure you want to push branch ", primary_style()),
            Span::styled(branch_name, accent_style().add_modifier(Modifier::BOLD)),
            Span::raw("?"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Confirm: ", muted_style()),
            Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" / Cancel: ", muted_style()),
            Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(content).block(block);
    f.render_widget(paragraph, popup_area);
}

fn draw_tag_create_popup(
    f: &mut Frame,
    input_buffer: &str,
    target_commit_oid: Option<&str>,
    area: Rect,
) {
    let popup_area = centred_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Create Tag", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let commit_hash = target_commit_oid
        .map(|oid| if oid.len() >= 7 { &oid[..7] } else { oid })
        .unwrap_or("unknown");
    let content = vec![
        Line::from(vec![
            Span::styled("Target Commit: ", muted_style()),
            Span::styled(commit_hash, primary_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tag Name: ", muted_style()),
            Span::styled(input_buffer, primary_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let paragraph = Paragraph::new(content);
    f.render_widget(paragraph, inner_area);

    let cursor_y = inner_area.y.saturating_add(2).min(
        inner_area
            .y
            .saturating_add(inner_area.height.saturating_sub(1)),
    );
    let label_width = "Tag Name: ".chars().count() as u16;
    let cursor_offset = label_width.saturating_add(input_buffer.chars().count() as u16);
    let cursor_x = inner_area.x.saturating_add(cursor_offset).min(
        inner_area
            .x
            .saturating_add(inner_area.width.saturating_sub(1)),
    );
    f.set_cursor_position(ratatui::layout::Position::new(cursor_x, cursor_y));
}

fn draw_tags_view(
    f: &mut Frame,
    info: &RepoInfo,
    focus: DetailSection,
    local_tag_selection: usize,
    remote_tags_loaded: bool,
    areas: &mut DetailAreas,
    area: Rect,
) {
    areas.local_tags = Some(area);
    areas.remote_tags = None;

    // ── Local Tags Panel ──
    let local_focused = focus == DetailSection::LocalTags;
    let local_border_style = if local_focused {
        Style::default().fg(ACCENT())
    } else {
        muted_style()
    };
    let local_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(local_border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Local Tags", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::uniform(1));

    let local_items: Vec<ListItem> = info
        .local_tags
        .iter()
        .map(|t| {
            let mut spans = vec![Span::styled("  ", Style::default())];
            if !t.short_sha.is_empty() {
                spans.push(Span::styled(format!("[{}]", t.short_sha), accent_style()));
                spans.push(Span::raw("  "));
            }
            spans.push(Span::styled(t.name.clone(), primary_style()));
            if !t.short_message.is_empty() {
                spans.push(Span::raw(" "));
                spans.push(Span::styled("·", muted_style()));
                spans.push(Span::raw(" "));
                spans.push(Span::styled(t.short_message.clone(), muted_style()));
            }

            let is_pushed = if info.remotes.is_empty() {
                true
            } else if remote_tags_loaded {
                info.remote_tags.iter().any(|rt| rt.name == t.name)
            } else {
                true
            };

            if !is_pushed {
                spans.push(Span::raw("  "));
                spans.push(Span::styled("unpushed", Style::default().fg(WARNING())));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let local_list = List::new(local_items)
        .block(local_block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut local_state = ListState::default();
    if local_focused {
        local_state.select(Some(local_tag_selection));
    }
    f.render_stateful_widget(local_list, area, &mut local_state);
}

fn draw_remotes_view(
    f: &mut Frame,
    info: &RepoInfo,
    focus: DetailSection,
    remote_selection: usize,
    areas: &mut DetailAreas,
    area: Rect,
) {
    areas.remotes = Some(area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    let left_area = chunks[0];
    let right_area = chunks[1];

    let focused = focus == DetailSection::Remotes;
    let border_style = if focused {
        Style::default().fg(ACCENT())
    } else {
        muted_style()
    };

    // ── Remotes List Panel ──
    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Remotes", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::uniform(1));

    let list_items: Vec<ListItem> = info
        .remotes
        .iter()
        .map(|r| {
            ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(r.name.clone(), primary_style()),
            ]))
        })
        .collect();

    let list = List::new(list_items)
        .block(list_block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut list_state = ListState::default();
    if focused {
        list_state.select(Some(remote_selection));
    }
    f.render_stateful_widget(list, left_area, &mut list_state);

    // ── Remote Details Panel ──
    let details_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(muted_style())
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Remote Details", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::uniform(1));

    let mut details_lines = Vec::new();
    if let Some(selected_remote) = info.remotes.get(remote_selection) {
        details_lines.push(Line::from(""));
        details_lines.push(Line::from(vec![
            Span::styled("  Name:      ", accent_style()),
            Span::styled(selected_remote.name.clone(), primary_style()),
        ]));
        details_lines.push(Line::from(vec![
            Span::styled("  Fetch URL: ", accent_style()),
            Span::raw(selected_remote.url.clone()),
        ]));
        if let Some(push_url) = &selected_remote.push_url {
            details_lines.push(Line::from(vec![
                Span::styled("  Push URL:  ", accent_style()),
                Span::raw(push_url.clone()),
            ]));
        } else {
            details_lines.push(Line::from(vec![
                Span::styled("  Push URL:  ", accent_style()),
                Span::raw(selected_remote.url.clone()),
            ]));
        }
        details_lines.push(Line::from(""));
        details_lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Refspecs:", primary_style()),
        ]));
        for spec in &selected_remote.refspecs {
            details_lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(spec.clone(), muted_style()),
            ]));
        }
    } else {
        details_lines.push(Line::from(""));
        details_lines.push(Line::from(Span::styled(
            "  No remotes configured",
            muted_style(),
        )));
    }

    let details_paragraph = Paragraph::new(details_lines).block(details_block);
    f.render_widget(details_paragraph, right_area);
}

fn draw_tag_delete_popup(f: &mut Frame, target: &Option<(String, bool)>, area: Rect) {
    let popup_area = centred_rect(55, 25, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(DANGER());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Delete Tag", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let (tag_name, is_on_remote) = match target {
        Some((name, is_on_remote)) => (name.as_str(), *is_on_remote),
        None => ("", false),
    };

    let mut content = vec![
        Line::from(vec![
            Span::styled("Are you sure you want to delete the tag ", primary_style()),
            Span::styled(tag_name, accent_style()),
            Span::raw("?"),
        ]),
        Line::from(""),
    ];

    if is_on_remote {
        content.push(Line::from(vec![
            Span::styled(
                "Warning: ",
                Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
            ),
            Span::raw(
                "This tag is also present on the remote and will be deleted from the remote.",
            ),
        ]));
        content.push(Line::from(""));
    }

    content.push(Line::from(vec![
        Span::styled("Confirm: ", muted_style()),
        Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" / Cancel: ", muted_style()),
        Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
    ]));

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, popup_area);
}

fn draw_stash_delete_popup(f: &mut Frame, target: &Option<String>, area: Rect) {
    let popup_area = centred_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(DANGER());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Delete Stash", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let stash_name = match target {
        Some(name) => name.as_str(),
        None => "",
    };

    let content = vec![
        Line::from(vec![Span::styled(
            "Are you sure you want to delete the stash:",
            primary_style(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                stash_name,
                Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Confirm: ", muted_style()),
            Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" / Cancel: ", muted_style()),
            Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, popup_area);
}

fn draw_stash_apply_popup(f: &mut Frame, target: &Option<String>, delete_after: bool, area: Rect) {
    let popup_area = centred_rect(55, 25, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Apply Stash", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let stash_name = match target {
        Some(name) => name.as_str(),
        None => "",
    };

    let mut content = vec![
        Line::from(vec![Span::styled(
            "Are you sure you want to apply the stash:",
            primary_style(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                stash_name,
                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];

    let delete_after_style = if delete_after {
        Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(ratatui::style::Color::DarkGray)
    };

    let checkbox = if delete_after { "[X]" } else { "[ ]" };

    content.push(Line::from(vec![
        Span::styled(format!("  {} ", checkbox), delete_after_style),
        Span::styled("Delete stash after applying", primary_style()),
        Span::styled(" (toggle: [d/space/a])", muted_style()),
    ]));

    content.push(Line::from(""));

    content.push(Line::from(vec![
        Span::styled("Confirm: ", muted_style()),
        Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" / Cancel: ", muted_style()),
        Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
    ]));

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, popup_area);
}

fn draw_tag_push_popup(f: &mut Frame, target: &Option<String>, area: Rect) {
    let popup_area = centred_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(SUCCESS());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Push Tag", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let tag_name = target.as_deref().unwrap_or("");

    let content = vec![
        Line::from(vec![
            Span::styled("Are you sure you want to push the tag ", primary_style()),
            Span::styled(tag_name, accent_style()),
            Span::raw(" to remote?"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Confirm: ", muted_style()),
            Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" / Cancel: ", muted_style()),
            Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, popup_area);
}

fn draw_tag_push_all_popup(f: &mut Frame, area: Rect) {
    let popup_area = centred_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(SUCCESS());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Push All Tags", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let content = vec![
        Line::from(vec![
            Span::styled("Are you sure you want to push ", primary_style()),
            Span::styled(
                "ALL local tags",
                Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to remote?", primary_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Confirm: ", muted_style()),
            Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" / Cancel: ", muted_style()),
            Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, popup_area);
}

#[allow(clippy::too_many_arguments)]
fn draw_stashes_view(
    f: &mut Frame,
    info: &RepoInfo,
    focus: DetailSection,
    stash_selection: usize,
    stash_file_selection: usize,
    file_diff: &[DiffLine],
    diff_scroll: usize,
    areas: &mut DetailAreas,
    area: Rect,
) {
    areas.bottom_left = None;
    areas.bottom_right = None;
    areas.commits = None;
    areas.local_branches = None;
    areas.remote_branches = None;
    areas.local_tags = None;
    areas.remote_tags = None;
    areas.files = None;
    areas.remotes = None;

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    let left_area = chunks[0];
    let right_area = chunks[1];

    areas.bottom_right = Some(right_area);

    // Split left area horizontally: top = Stashes list, bottom = Stashed files
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(left_area);

    areas.stashes = Some(left_chunks[0]);
    areas.stashed_files = Some(left_chunks[1]);

    // ── Stashes List Panel ──
    let stashes_focused = focus == DetailSection::Stashes;
    let stashes_border_style = if stashes_focused {
        Style::default().fg(ACCENT())
    } else {
        muted_style()
    };

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(stashes_border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Stashes", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::uniform(1));

    let list_items: Vec<ListItem> = info
        .stashes
        .iter()
        .map(|s| {
            ListItem::new(Line::from(vec![Span::styled(
                format!("  stash@{{{}}}: {}", s.index, s.message),
                primary_style(),
            )]))
        })
        .collect();

    let list = List::new(list_items)
        .block(list_block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut list_state = ListState::default();
    if stashes_focused || !info.stashes.is_empty() {
        list_state.select(Some(stash_selection));
    }
    f.render_stateful_widget(list, left_chunks[0], &mut list_state);

    // ── Stashed Files List Panel ──
    let files_focused = focus == DetailSection::StashedFiles;
    let selected_stash = info.stashes.get(stash_selection);
    let stashed_files = selected_stash.map(|s| s.files.as_slice()).unwrap_or(&[]);

    draw_file_subpanel(
        f,
        "Stashed Files",
        WARNING(),
        stashed_files,
        "No files in this stash",
        Borders::ALL,
        files_focused,
        if files_focused || !stashed_files.is_empty() {
            Some(stash_file_selection)
        } else {
            None
        },
        left_chunks[1],
    );

    // ── Right panel: Diff/Stash Details ──
    let diff_focused = focus == DetailSection::StagingDetails;
    let right_border_style = if diff_focused {
        Style::default().fg(ACCENT())
    } else {
        muted_style()
    };

    let selected_file_name: Option<String> = stashed_files
        .get(stash_file_selection)
        .map(|e| e.path.clone());

    let right_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(right_border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Stash Diff", primary_style()),
            if let Some(ref name) = selected_file_name {
                Span::styled(format!("  {}", name), muted_style())
            } else {
                Span::raw("")
            },
            Span::raw(" "),
        ]))
        .padding(Padding::uniform(1));

    let right_inner = right_block.inner(right_area);
    f.render_widget(right_block, right_area);

    if file_diff.is_empty() {
        let v_center = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(45),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(right_inner);
        f.render_widget(
            Paragraph::new(Span::styled(
                "Select a file to view its diff",
                muted_style(),
            ))
            .alignment(Alignment::Center),
            v_center[1],
        );
    } else {
        let diff_spans: Vec<Line> = file_diff
            .iter()
            .map(|line| {
                let style = match line.kind {
                    DiffLineKind::Added => Style::default().fg(SUCCESS()),
                    DiffLineKind::Removed => Style::default().fg(DANGER()),
                    DiffLineKind::Header => Style::default().fg(ratatui::style::Color::Cyan),
                    DiffLineKind::Context => Style::default(),
                };
                Line::from(Span::styled(line.content.clone(), style))
            })
            .collect();
        f.render_widget(
            Paragraph::new(diff_spans)
                .scroll((diff_scroll as u16, 0))
                .wrap(Wrap { trim: false }),
            right_inner,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_inspect_window(
    f: &mut Frame,
    commit: &CommitEntry,
    focus: DetailSection,
    file_selection: usize,
    file_diff: &[DiffLine],
    diff_scroll: usize,
    commit_details_scroll: usize,
    areas: &mut DetailAreas,
    inspect_horizontal_split_pct: u16,
    inspect_vertical_split_pct: u16,
    area: Rect,
) {
    // Body: divided vertically: Left panel, Right panel
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(inspect_horizontal_split_pct),
            Constraint::Percentage(100 - inspect_horizontal_split_pct),
        ])
        .split(area);

    // Record horizontal splitter boundary
    let split_col = area.x + panels[0].width;
    areas.inspect_horizontal_splitter = Some(Rect::new(
        split_col.saturating_sub(1),
        area.y,
        2,
        area.height,
    ));

    // Split left panel vertically: top is Commit Details, bottom is Changed Files
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(inspect_vertical_split_pct),
            Constraint::Percentage(100 - inspect_vertical_split_pct),
        ])
        .split(panels[0]);

    // Record vertical splitter boundary in left panel
    let split_row = panels[0].y + left_chunks[0].height;
    areas.inspect_vertical_splitter = Some(Rect::new(
        panels[0].x,
        split_row.saturating_sub(1),
        panels[0].width,
        2,
    ));

    // Record panel areas for mouse hit testing/scrolling
    areas.commit_details = Some(left_chunks[0]);
    areas.bottom_left = Some(left_chunks[1]);
    areas.bottom_right = Some(panels[1]);

    let details_focused = focus == DetailSection::CommitDetails;
    let left_focused = focus == DetailSection::Staged;
    let right_focused = focus == DetailSection::StagingDetails;

    // ── Left Top: Commit Info (Commit Details) ─────────────────────────
    draw_commit_details_widget(
        f,
        commit,
        details_focused,
        commit_details_scroll,
        left_chunks[0],
    );

    // ── Left Bottom: Changed Files ─────────────────────────────────────
    let left_border_style = if left_focused {
        Style::default().fg(ACCENT())
    } else {
        muted_style()
    };
    let left_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(left_border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Changed Files", primary_style()),
            Span::raw("  "),
            Span::styled(format!("({})", commit.files.len()), muted_style()),
            Span::raw(" "),
        ]));
    let left_inner = left_block.inner(left_chunks[1]);
    f.render_widget(left_block, left_chunks[1]);

    if commit.files.is_empty() {
        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(left_inner);
        f.render_widget(
            Paragraph::new(Span::styled("No files changed", muted_style()))
                .alignment(Alignment::Center),
            v[1],
        );
    } else {
        let items: Vec<ListItem> = commit
            .files
            .iter()
            .map(|f| ListItem::new(file_entry_line(f)))
            .collect();
        let list =
            List::new(items).highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        let mut state = ListState::default();
        state.select(Some(file_selection));
        f.render_stateful_widget(list, left_inner, &mut state);
    }

    // ── Right: Diff ───────────────────────────────────────────────────
    let right_border_style = if right_focused {
        Style::default().fg(ACCENT())
    } else {
        muted_style()
    };
    let right_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(right_border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Diff", primary_style()),
            if right_focused && diff_scroll > 0 {
                Span::styled(format!("  ↕ line {}", diff_scroll + 1), muted_style())
            } else {
                Span::raw("")
            },
            if right_focused {
                Span::styled("  ↑↓ scroll", muted_style())
            } else {
                Span::raw("")
            },
            Span::raw(" "),
        ]));
    let right_inner = right_block.inner(panels[1]);
    f.render_widget(right_block, panels[1]);

    if file_diff.is_empty() {
        let v_center = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(45),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(right_inner);
        f.render_widget(
            Paragraph::new(Span::styled(
                "Select a file to view its diff",
                muted_style(),
            ))
            .alignment(Alignment::Center),
            v_center[1],
        );
    } else {
        let diff_spans: Vec<Line> = file_diff
            .iter()
            .map(|line| {
                let style = match line.kind {
                    DiffLineKind::Added => Style::default().fg(SUCCESS()),
                    DiffLineKind::Removed => Style::default().fg(DANGER()),
                    DiffLineKind::Header => Style::default().fg(ratatui::style::Color::Cyan),
                    DiffLineKind::Context => Style::default(),
                };
                Line::from(Span::styled(line.content.clone(), style))
            })
            .collect();
        f.render_widget(
            Paragraph::new(diff_spans)
                .scroll((diff_scroll as u16, 0))
                .wrap(Wrap { trim: false }),
            right_inner,
        );
    }
}

fn draw_commit_details_widget(
    f: &mut Frame,
    commit: &CommitEntry,
    focused: bool,
    scroll: usize,
    area: Rect,
) {
    let border_style = if focused {
        Style::default().fg(ACCENT())
    } else {
        muted_style()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Commit Info", primary_style()),
            if focused && scroll > 0 {
                Span::styled(format!("  ↕ line {}", scroll + 1), muted_style())
            } else {
                Span::raw("")
            },
            if focused {
                Span::styled("  ↑↓ scroll", muted_style())
            } else {
                Span::raw("")
            },
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines = Vec::new();

    // Hash
    lines.push(Line::from(vec![
        Span::styled("Hash:   ", primary_style()),
        Span::raw(&commit.oid),
    ]));

    // Author
    lines.push(Line::from(vec![
        Span::styled("Author: ", primary_style()),
        Span::raw(&commit.author),
    ]));

    // Date
    lines.push(Line::from(vec![
        Span::styled("Date:   ", primary_style()),
        Span::raw(format!("{} ({})", commit.date, commit.when)),
    ]));

    // Refs
    if !commit.refs.is_empty() {
        let mut ref_spans = vec![Span::styled("Refs:   ", primary_style())];
        for (idx, r) in commit.refs.iter().enumerate() {
            if idx > 0 {
                ref_spans.push(Span::raw(", "));
            }
            let style = if r.starts_with("tag:") {
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)
            };
            ref_spans.push(Span::styled(r.clone(), style));
        }
        lines.push(Line::from(ref_spans));
    }

    // Empty separator
    lines.push(Line::from(""));

    // Message
    lines.push(Line::from(vec![Span::styled("Message:", primary_style())]));
    for line in commit.message.lines() {
        lines.push(Line::from(vec![Span::raw(line.to_string())]));
    }

    let paragraph = Paragraph::new(lines)
        .scroll((scroll as u16, 0))
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, inner);
}
