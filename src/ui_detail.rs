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

use crate::app::DetailSection;
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

use crate::app::Mode;

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
    /// Bounding box of the tab bar itself.
    pub tab_bar: Option<Rect>,
    /// Bounding box of the files list.
    pub files: Option<Rect>,
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
    file_selection: usize,
    file_diff: &[DiffLine],
    diff_scroll: usize,
    staging_file_selection: usize,
    commit_details_scroll: usize,
    local_branch_selection: usize,
    remote_branch_selection: usize,
    file_list_selection: usize,
    visible_files: &[crate::app::FileTreeItem],
    detail_tab: usize,
    graph_scroll: usize,
    areas: &mut DetailAreas,
    input_buffer: &str,
    commit_editing: bool,
    branch_action_target: &Option<(String, bool)>,
    area: Rect,
) {
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

    // Split header into left (Item label) and right (branch name).
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(40)])
        .split(header_area);

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

    if let Some(tab_area) = tab_bar_area {
        let (style_details, style_graph, style_branches, style_files) = if detail_tab == 0 {
            (
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                Style::default().add_modifier(Modifier::DIM | Modifier::UNDERLINED),
                Style::default().add_modifier(Modifier::DIM | Modifier::UNDERLINED),
                Style::default().add_modifier(Modifier::DIM | Modifier::UNDERLINED),
            )
        } else if detail_tab == 1 {
            (
                Style::default().add_modifier(Modifier::DIM | Modifier::UNDERLINED),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                Style::default().add_modifier(Modifier::DIM | Modifier::UNDERLINED),
                Style::default().add_modifier(Modifier::DIM | Modifier::UNDERLINED),
            )
        } else if detail_tab == 2 {
            (
                Style::default().add_modifier(Modifier::DIM | Modifier::UNDERLINED),
                Style::default().add_modifier(Modifier::DIM | Modifier::UNDERLINED),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                Style::default().add_modifier(Modifier::DIM | Modifier::UNDERLINED),
            )
        } else {
            (
                Style::default().add_modifier(Modifier::DIM | Modifier::UNDERLINED),
                Style::default().add_modifier(Modifier::DIM | Modifier::UNDERLINED),
                Style::default().add_modifier(Modifier::DIM | Modifier::UNDERLINED),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )
        };

        let details_bullet = if detail_tab == 0 { "●" } else { "○" };
        let graph_bullet = if detail_tab == 1 { "●" } else { "○" };
        let branches_bullet = if detail_tab == 2 { "●" } else { "○" };
        let files_bullet = if detail_tab == 3 { "●" } else { "○" };

        let tab_line = Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{} Details [1]", details_bullet), style_details),
            Span::raw("    "),
            Span::styled(format!("{} Graph [2]", graph_bullet), style_graph),
            Span::raw("    "),
            Span::styled(format!("{} Branches [3]", branches_bullet), style_branches),
            Span::raw("    "),
            Span::styled(format!("{} Files [4]", files_bullet), style_files),
        ]);
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
                let is_uncommitted_row = dirty && commit_selection == 0;

                draw_detail_commits(f, info, *focus, commit_selection, detail_chunks[0]);
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
                                detail_chunks[1],
                            );
                        }
                    }
                }
            } else if detail_tab == 1 {
                // Render Graph view
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(CARD_BORDER)
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
            } else if detail_tab == 2 {
                // Render Branches view (tab 3, index 2)
                draw_branches_view(
                    f,
                    info,
                    *focus,
                    local_branch_selection,
                    remote_branch_selection,
                    areas,
                    body_area,
                );
            } else {
                // Render Files view (tab 4, index 3)
                draw_files_view(
                    f,
                    visible_files,
                    *focus,
                    file_list_selection,
                    areas,
                    body_area,
                );
            }

            // Draw overview popup on top when requested.
            if matches!(mode, Mode::DetailOverview) {
                draw_overview_popup(f, resolved, info, body_area);
            }
            // Draw detail help overlay on top when requested.
            if matches!(mode, Mode::DetailHelp) {
                draw_detail_help_overlay(f, body_area);
            }
            // Draw commit popup on top when requested.
            if matches!(mode, Mode::CommitInput) {
                draw_commit_popup(f, input_buffer, commit_editing, body_area);
            }
            // Draw branch create popup on top when requested.
            if matches!(mode, Mode::BranchCreateInput) {
                draw_branch_create_popup(f, input_buffer, branch.as_deref(), body_area);
            }
            // Draw branch delete popup on top when requested.
            if matches!(mode, Mode::BranchDeleteConfirm) {
                draw_branch_delete_popup(f, branch_action_target, body_area);
            }
            // Draw branch push popup on top when requested.
            if matches!(mode, Mode::BranchPushConfirm) {
                draw_branch_push_popup(f, branch_action_target, body_area);
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
        Style::default().fg(ACCENT)
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
    let details_focused = focus == DetailSection::CommitDetails;
    let details_border_style = if details_focused {
        Style::default().fg(ACCENT)
    } else {
        muted_style()
    };
    let details_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(details_border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Commit Details", primary_style()),
            if details_focused && commit_details_scroll > 0 {
                Span::styled(
                    format!("  ↕ line {}", commit_details_scroll + 1),
                    muted_style(),
                )
            } else {
                Span::raw("")
            },
            if details_focused {
                Span::styled("  ↑↓ scroll", muted_style())
            } else {
                Span::raw("")
            },
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));
    let details_inner = details_block.inner(left_chunks[1]);
    f.render_widget(details_block, left_chunks[1]);

    let mut detail_lines = Vec::new();

    // Hash
    detail_lines.push(Line::from(vec![
        Span::styled("Hash:   ", primary_style()),
        Span::raw(&commit.oid),
    ]));

    // Author
    detail_lines.push(Line::from(vec![
        Span::styled("Author: ", primary_style()),
        Span::raw(&commit.author),
    ]));

    // Date
    detail_lines.push(Line::from(vec![
        Span::styled("Date:   ", primary_style()),
        Span::raw(format!("{} ({})", commit.date, commit.when)),
    ]));

    // Labels
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
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
            };
            ref_spans.push(Span::styled(r.clone(), style));
        }
        detail_lines.push(Line::from(ref_spans));
    }

    // Empty separator
    detail_lines.push(Line::from(""));

    // Message
    detail_lines.push(Line::from(vec![Span::styled("Message:", primary_style())]));
    for line in commit.message.lines() {
        detail_lines.push(Line::from(vec![Span::raw(line.to_string())]));
    }

    let details_paragraph = Paragraph::new(detail_lines)
        .scroll((commit_details_scroll as u16, 0))
        .wrap(Wrap { trim: true });
    f.render_widget(details_paragraph, details_inner);

    // ── Right: diff panel ─────────────────────────────────────────────────
    let right_border_style = if right_focused {
        Style::default().fg(ACCENT)
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
                    DiffLineKind::Added => Style::default().fg(SUCCESS),
                    DiffLineKind::Removed => Style::default().fg(DANGER),
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
    area: Rect,
) {
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    areas.bottom_left = Some(panels[0]);
    areas.bottom_right = Some(panels[1]);
    areas.commit_details = None;

    // Focus-aware border helpers.
    let left_focused = focus == DetailSection::Staged || focus == DetailSection::Unstaged;
    let right_focused = focus == DetailSection::StagingDetails;

    // ── Left panel: outer border labelled "Staging Area" ──────────────────
    let left_border_style = if left_focused {
        Style::default().fg(ACCENT)
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
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(left_inner);

    areas.staged_sub = Some(left_split[0]);
    areas.unstaged_sub = Some(left_split[1]);

    draw_file_subpanel(
        f,
        "Staged",
        SUCCESS,
        &changes.staged,
        "Nothing staged",
        Borders::BOTTOM,
        focus == DetailSection::Staged,
        if focus == DetailSection::Staged {
            Some(staging_file_selection)
        } else {
            None
        },
        left_split[0],
    );
    draw_file_subpanel(
        f,
        "Unstaged",
        WARNING,
        &changes.unstaged,
        "No unstaged changes",
        Borders::empty(),
        focus == DetailSection::Unstaged,
        if focus == DetailSection::Unstaged {
            Some(staging_file_selection)
        } else {
            None
        },
        left_split[1],
    );

    // ── Right panel – Staging Details ─────────────────────────────────────
    let right_border_style = if right_focused {
        Style::default().fg(ACCENT)
    } else {
        muted_style()
    };
    let selected_file_name: Option<String> = {
        let files = match focus {
            DetailSection::Staged => Some(&changes.staged),
            DetailSection::Unstaged => Some(&changes.unstaged),
            _ => None,
        };
        files
            .and_then(|f| f.get(staging_file_selection))
            .map(|e| e.path.clone())
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
                    DiffLineKind::Added => Style::default().fg(SUCCESS),
                    DiffLineKind::Removed => Style::default().fg(DANGER),
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
            .fg(ACCENT)
            .add_modifier(ratatui::style::Modifier::BOLD)
    } else {
        Style::default().fg(title_color)
    };
    let border_style = if focused {
        Style::default().fg(ACCENT)
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
fn draw_overview_popup(f: &mut Frame, resolved: &std::path::Path, info: &RepoInfo, area: Rect) {
    // Popup takes up ~70 % of the available space
    let popup_area = centred_rect(70, 70, area);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Overview", primary_style()),
            Span::raw(" "),
        ]));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let body_lines = build_repo_body(resolved, info);
    let body = Paragraph::new(body_lines)
        .block(Block::default().padding(Padding::horizontal(1)))
        .wrap(Wrap { trim: false });
    f.render_widget(body, inner);
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

const DETAIL_HELP_LINES: &[(&str, &str)] = &[
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
    ("w / W", "Cycle panel focus (Details / Branches tabs)"),
    ("← / →", "Focus Local/Remote branch (Branches tab)"),
    ("← / → or < / >", "Collapse/Expand folder (Files tab)"),
    (
        "↵ [Enter]",
        "Stage/Unstage file, or Checkout selected branch",
    ),
    ("c", "Commit changes (Details) / Create branch (Branches)"),
    ("d", "Delete selected branch (Branches tab)"),
    ("o", "Show repo overview popup"),
    ("1", "Go to Details tab"),
    ("2", "Go to Graph View tab"),
    ("3", "Go to Branches tab"),
    ("4", "Go to Files tab"),
    ("⇧F [Shift+F]", "Fetch selected local branch's upstream"),
    ("p", "Pull selected local branch from remote"),
    ("⇧P [Shift+P]", "Push selected local branch to remote"),
    ("? / ⎋ [Esc]", "Close this help"),
    ("q / ⎋ [Esc]", "Back to repository list"),
    (
        "Left-Click",
        "Focus clicked panel / change tab (mouse support)",
    ),
];

/// Renders a floating shortcut reference overlay centred over `area`.
fn draw_detail_help_overlay(f: &mut Frame, area: Rect) {
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
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            Span::raw((*desc).to_string()),
        ]));
    }
    lines.push(Line::from(""));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Detail Shortcuts", primary_style()),
            Span::raw("  "),
            Span::styled("? / Esc  close", muted_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));

    let para = Paragraph::new(lines).block(block);
    f.render_widget(para, popup_area);
}

fn draw_detail_commits(
    f: &mut Frame,
    info: &RepoInfo,
    focus: DetailSection,
    commit_selection: usize,
    area: Rect,
) {
    let focused = focus == DetailSection::Commits;
    let border_style = if focused {
        Style::default().fg(ACCENT)
    } else {
        muted_style()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Commits", primary_style()),
            Span::raw(" "),
        ]));

    // Dirty = any uncommitted change in staged / unstaged / untracked / conflicted.
    let dirty = !info.changes.staged.is_empty()
        || !info.changes.unstaged.is_empty()
        || !info.changes.untracked.is_empty()
        || !info.changes.conflicted.is_empty();

    // Show empty placeholder only when truly empty (no commits AND clean).
    if info.commits.is_empty() && !dirty {
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
    .style(Style::default().add_modifier(Modifier::BOLD).fg(ACCENT));

    // Prepend a virtual "uncommitted changes" row when the worktree is dirty.
    let mut rows: Vec<Row> = Vec::new();
    if dirty {
        rows.push(Row::new(vec![
            Cell::from(Span::styled("-", muted_style())),
            Cell::from(Span::styled("-", muted_style())),
            Cell::from(Span::styled("-", muted_style())),
            Cell::from(Span::styled("<uncommitted>", Style::default().fg(WARNING))),
        ]));
    }
    rows.extend(info.commits.iter().map(|commit| {
        // Build the summary cell: optional ref badges then the commit message.
        let mut spans: Vec<Span<'static>> = Vec::new();
        for r in &commit.refs {
            let (label, style) = if let Some(tag) = r.strip_prefix("tag:") {
                // Tag — yellow
                (
                    format!("[{}]", tag),
                    Style::default().fg(WARNING).add_modifier(Modifier::BOLD),
                )
            } else if let Some(remote) = r.strip_prefix("remote:") {
                // Remote tracking branch — green
                (
                    format!("[{}]", remote),
                    Style::default().fg(SUCCESS).add_modifier(Modifier::BOLD),
                )
            } else {
                // Local branch — cyan
                (
                    format!("[{}]", r),
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                )
            };
            spans.push(Span::styled(label, style));
            spans.push(Span::raw(" "));
        }
        spans.push(Span::styled(commit.summary.clone(), Style::default()));

        Row::new(vec![
            Cell::from(Span::styled(
                commit.id.clone(),
                Style::default().fg(WARNING),
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
                DANGER,
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
                WARNING,
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
                WARNING,
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
                Span::styled(message.clone(), Style::default().fg(DANGER)),
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
        SUCCESS,
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
            Span::styled(head.short_id.clone(), Style::default().fg(WARNING)),
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
        Span::styled("▍ ", Style::default().fg(ACCENT)),
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
                Span::styled(name.clone(), Style::default().fg(ACCENT)),
            ));
            let s = &info.summary;
            if s.is_synced() {
                lines.push(field_line(
                    "Sync",
                    Span::styled("in sync", Style::default().fg(SUCCESS)),
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
                        Style::default().fg(WARNING),
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
        Span::styled(format!("{:<8}", remote.name), Style::default().fg(ACCENT)),
        Span::raw("  "),
        Span::raw(remote.url.clone()),
    ])
}

fn file_entry_line(entry: &FileEntry) -> Line<'static> {
    let label_style = match entry.label {
        "N" => Style::default().fg(SUCCESS),
        "D" => Style::default().fg(DANGER),
        "C" => Style::default().fg(DANGER).add_modifier(Modifier::BOLD),
        "R" | "T" => Style::default().fg(ACCENT),
        "?" => muted_style(),
        _ => Style::default().fg(WARNING), // "M"
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
fn draw_commit_popup(f: &mut Frame, input_buffer: &str, editing: bool, area: Rect) {
    let popup_area = centred_rect(60, 25, area);
    f.render_widget(Clear, popup_area);

    let border_color = if editing { ACCENT } else { WARNING };
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
    } else {
        Paragraph::new(input_buffer).wrap(Wrap { trim: true })
    };

    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);
    f.render_widget(text, inner_area);

    if editing {
        let lines: Vec<&str> = input_buffer.split('\n').collect();
        let last_line = lines.last().copied().unwrap_or("");
        let line_count = lines.len();
        let cursor_y = inner_area
            .y
            .saturating_add(line_count.saturating_sub(1) as u16)
            .min(
                inner_area
                    .y
                    .saturating_add(inner_area.height.saturating_sub(1)),
            );
        let cursor_offset = last_line.chars().count() as u16;
        let cursor_x = inner_area
            .x
            .saturating_add(cursor_offset.min(inner_area.width.saturating_sub(1)));
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
                        Style::default().fg(SUCCESS).add_modifier(Modifier::BOLD),
                    ));
                    spans.push(Span::styled(
                        stripped.to_string(),
                        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                    ));
                } else if let Some(stripped) = ref_item.strip_prefix("tag: ") {
                    spans.push(Span::styled("tag: ", Style::default().fg(WARNING)));
                    spans.push(Span::styled(
                        stripped.to_string(),
                        Style::default().fg(WARNING).add_modifier(Modifier::BOLD),
                    ));
                } else if ref_item.contains('/') {
                    spans.push(Span::styled(
                        ref_item.to_string(),
                        Style::default().fg(DANGER),
                    ));
                } else {
                    spans.push(Span::styled(
                        ref_item.to_string(),
                        Style::default().fg(SUCCESS),
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
        Style::default().fg(ACCENT)
    } else {
        muted_style()
    };
    let local_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER)
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
                Style::default().fg(SUCCESS).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let prefix = if b.is_head { " " } else { "  " };
            let mut spans = vec![
                Span::styled(prefix, Style::default().fg(SUCCESS)),
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
        Style::default().fg(ACCENT)
    } else {
        muted_style()
    };
    let remote_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER)
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

fn draw_files_view(
    f: &mut Frame,
    visible_files: &[crate::app::FileTreeItem],
    focus: DetailSection,
    file_list_selection: usize,
    areas: &mut DetailAreas,
    area: Rect,
) {
    areas.files = Some(area);

    let focused = focus == DetailSection::Files;
    let border_style = if focused {
        Style::default().fg(ACCENT)
    } else {
        muted_style()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER)
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
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = ListState::default();
    state.select(Some(file_list_selection));

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_branch_create_popup(
    f: &mut Frame,
    input_buffer: &str,
    base_branch: Option<&str>,
    area: Rect,
) {
    let popup_area = centred_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT);
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

    let border_style = Style::default().fg(DANGER);
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
                Style::default().fg(DANGER).add_modifier(Modifier::BOLD),
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

fn draw_branch_push_popup(f: &mut Frame, target: &Option<(String, bool)>, area: Rect) {
    let popup_area = centred_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT);
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
