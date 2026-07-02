//! Rendering for the main list + status bar + help overlay.
//!
//! All drawing reads from `&App`; nothing here mutates state. Adding a new
//! keybinding means updating the help line list helpers (under `src/popups/help.rs`
//! or `src/popups/detail_help.rs`) AND the status bar text (under `src/components/cmd_bar.rs`),
//! so the help overlay and the bottom bar stay aligned.
//!
//! Visual conventions live in the `theme` block at the top — keep visual
//! choices centralized so the look stays coherent.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Padding, Paragraph};

use crate::app::{App, Mode};
use crate::components::cmd_bar::StatusEntry;
use crate::config::SortOrder;
use crate::repo::{ItemStatus, RepoState, RepoSummary, format_relative_time};

// ── Theme ──────────────────────────────────────────────────────────────────
// Colors are kept minimal so the app works on both dark and light terminal
// backgrounds. Plain text is left at the terminal's default foreground —
// never hard-code `White` / `Gray` / `DarkGray` for text, because what reads
// as "muted" on one background reads as invisible on the other. Use the
// helpers below: `muted_style()` for de-emphasis, bold for emphasis, and the
// accent colors only for true accents (selection, mode, warnings).

pub use super::style::*;

/// Width of the per-item status zone on the right of each card. Wide
/// enough to fit a busy repo's worth of indicators ("● 99+ 99! 99? 99↑")
/// with a little breathing room. Right-aligned, so it crops on the left
/// for the unusual case of 3-digit counts on every indicator.
const STATUS_ZONE_WIDTH: u16 = 22;

const UNSELECTED_INDENT: &str = "  ";

/// Top-level draw entry point invoked from inside `terminal.draw`.
pub fn draw(
    f: &mut Frame,
    app: &App,
    area: Rect,
    inner_area: Rect,
    visible_count: usize,
    detail_areas: &mut crate::ui_detail::DetailAreas,
    main_areas: &mut Vec<Rect>,
    global_summary_area: &mut Option<Rect>,
) {
    *global_summary_area = None;
    let mut swapped_theme = None;
    if let Some(repo_path) = app.get_selected_item() {
        if matches!(
            app.mode,
            Mode::Detail
                | Mode::DetailHelp
                | Mode::CommitInput
                | Mode::BranchCreateInput
                | Mode::TagCreateInput
                | Mode::StashingUI
                | Mode::WorktreeAddBranchInput
                | Mode::WorktreeAddPathInput
                | Mode::WorktreeLockReasonInput
                | Mode::WorktreeRemoveConfirm
                | Mode::BranchDeleteConfirm
                | Mode::BranchCheckoutConfirm
                | Mode::SubmoduleAddUrlInput
                | Mode::SubmoduleAddPathInput
                | Mode::SubmoduleDeleteConfirm
                | Mode::TagCheckoutConfirm
                | Mode::BranchPushConfirm
                | Mode::BranchMergeConfirm
                | Mode::BranchRebaseConfirm
                | Mode::BranchInteractiveRebaseConfirm
                | Mode::TagDeleteConfirm
                | Mode::TagPushConfirm
                | Mode::TagPushAllConfirm
                | Mode::StashDeleteConfirm
                | Mode::StashApplyConfirm
                | Mode::CherryPickConfirm
                | Mode::RevertConfirm
                | Mode::MergeAbortConfirm
                | Mode::MergeContinueConfirm
                | Mode::StashCreateInput
                | Mode::RemotePicker
                | Mode::CommitSearchInput
                | Mode::DiscardChangesConfirm
                | Mode::Inspect
                | Mode::SearchColumnPicker
                | Mode::Logs
                | Mode::LogsSearchInput
                | Mode::RemoteAddNameInput
                | Mode::RemoteAddUrlInput
                | Mode::RemoteDeleteConfirm
                | Mode::RepoSettings
                | Mode::Overview
        ) {
            if let Some(repo_theme) = app.repo_theme_cache.get(repo_path) {
                // Save current theme state
                let current_theme_config = {
                    let lock =
                        crate::ui::style::THEME.read().expect("theme lock should be acquired");
                    crate::config::ThemeConfig {
                        accent: crate::ui::style::format_color(lock.accent),
                        warning: crate::ui::style::format_color(lock.warning),
                        danger: crate::ui::style::format_color(lock.danger),
                        success: crate::ui::style::format_color(lock.success),
                        border_type: crate::ui::style::format_border_type(lock.border_type),
                    }
                };

                // Swap with repo-specific theme
                crate::ui::update_theme(repo_theme);
                swapped_theme = Some(current_theme_config);
            }
        }
    }

    draw_outer_frame(f, area, app);

    // Always reserve the bottom row for the status bar, regardless of mode.
    let (content_area, status_chunk) = content_and_status_chunks(inner_area, app.status_height());

    if app.loading_repo_path.is_some() {
        crate::popups::loading::draw_loading_screen(f, content_area, app);
    } else if matches!(
        app.mode,
        Mode::Detail
            | Mode::DetailHelp
            | Mode::CommitInput
            | Mode::BranchCreateInput
            | Mode::TagCreateInput
            | Mode::StashingUI
            | Mode::WorktreeAddBranchInput
            | Mode::WorktreeAddPathInput
            | Mode::WorktreeLockReasonInput
            | Mode::WorktreeRemoveConfirm
            | Mode::BranchDeleteConfirm
            | Mode::BranchCheckoutConfirm
            | Mode::SubmoduleAddUrlInput
            | Mode::SubmoduleAddPathInput
            | Mode::SubmoduleDeleteConfirm
            | Mode::TagCheckoutConfirm
            | Mode::BranchPushConfirm
            | Mode::BranchMergeConfirm
            | Mode::BranchRebaseConfirm
            | Mode::BranchInteractiveRebaseConfirm
            | Mode::TagDeleteConfirm
            | Mode::TagPushConfirm
            | Mode::TagPushAllConfirm
            | Mode::StashDeleteConfirm
            | Mode::StashApplyConfirm
            | Mode::CherryPickConfirm
            | Mode::RevertConfirm
            | Mode::MergeAbortConfirm
            | Mode::MergeContinueConfirm
            | Mode::StashCreateInput
            | Mode::RemotePicker
            | Mode::CommitSearchInput
            | Mode::DiscardChangesConfirm
            | Mode::Inspect
            | Mode::SearchColumnPicker
            | Mode::Logs
            | Mode::LogsSearchInput
            | Mode::RemoteAddNameInput
            | Mode::RemoteAddUrlInput
            | Mode::RemoteDeleteConfirm
            | Mode::RepoSettings
            | Mode::Overview
    ) || (app.mode == Mode::UpdateConfirm && app.current_detail.is_some())
    {
        if let Some(detail) = &app.current_detail {
            let item_name = app.get_selected_item().map(String::as_str).unwrap_or("");
            crate::ui_detail::draw(
                f,
                item_name,
                detail,
                &app.mode,
                &app.detail_focus,
                app.last_staging_focus,
                app.commit_list.selection,
                &app.commit_list.search_query,
                app.status_list.file_selection,
                &app.diff.file_diff,
                app.diff.diff_scroll,
                app.status_list.staging_file_selection,
                app.commit_list.details_scroll,
                app.branch_list.local_branch_selection,
                app.branch_list.remote_branch_selection,
                app.tag_list.local_tag_selection,
                app.branch_list.remote_selection,
                app.remote_picker_selection,
                app.stash_list.stash_selection,
                app.stash_list.stash_file_selection,
                app.file_tree.file_list_selection,
                app.file_tree.file_content_scroll,
                &app.file_tree.visible_files,
                app.detail_tab,
                app.graph_scroll,
                app.help_scroll,
                detail_areas,
                &app.input_buffer,
                app.commit_popup.editing,
                &app.branch_action_target,
                &app.tag_action_target_oid,
                &app.tag_delete_target,
                &app.tag_push_target,
                &app.discard_target,
                app.stash_apply_delete_after,
                app.commit_popup.amend,
                app.commit_input_scroll,
                app.inspect_horizontal_split_pct,
                app.inspect_vertical_split_pct,
                app.workspace_main_split_pct,
                app.files_horizontal_split_pct,
                app.branches_horizontal_split_pct,
                app.stashes_horizontal_split_pct,
                app.stashes_vertical_split_pct,
                app.overview_horizontal_split_pct,
                app,
                content_area,
            );
        }
    } else if app.mode == Mode::FileHistory {
        crate::tabs::FileHistoryTab::draw_file_history(f, app, content_area);
    } else if app.mode == Mode::Settings {
        crate::popups::settings::draw_settings_page(f, app, content_area);
    } else if app.mode == Mode::DebugLogs {
        crate::popups::debug::draw_debug_logs(f, app, content_area);
    } else if app.config.items.is_empty() {
        draw_empty_state(f, content_area, app);
    } else if app.get_items_len() == 0 {
        if let Some(ref query) = app.repo_search_query {
            draw_search_empty_state(f, content_area, query);
        } else {
            draw_empty_state(f, content_area, app);
        }
    } else {
        let layout_parts = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Global summary bar
                Constraint::Length(1), // Spacer
                Constraint::Min(0),    // Rest of content
            ])
            .split(content_area);

        draw_global_summary_bar(f, layout_parts[0], app);
        *global_summary_area = Some(layout_parts[0]);
        let list_area_parent = layout_parts[2];

        let (header_area, list_area) = if app.config.compact_view {
            let parts = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(0)])
                .split(list_area_parent);
            (Some(parts[0]), parts[1])
        } else {
            (None, list_area_parent)
        };

        if let Some(hdr) = header_area {
            draw_compact_headers(f, hdr, app);
        }

        let list_chunks = item_chunks(list_area, visible_count, app);
        *main_areas = list_chunks.clone();
        draw_items(f, app, &list_chunks);
    }

    crate::components::cmd_bar::draw_status_bar(f, app, status_chunk);

    if matches!(app.mode, Mode::Help) {
        crate::popups::help::draw_help_overlay(f, app, area, app.help_scroll);
    }

    if matches!(app.mode, Mode::About) {
        crate::popups::about::draw_about_popup(f, area, app);
    }

    if matches!(app.mode, Mode::Legend) {
        crate::popups::legend::draw_legend_popup(f, area, app);
    }

    if matches!(app.mode, Mode::RepoSettings) {
        crate::popups::repo_settings::RepoSettingsPopup::draw(f, app, area);
    }

    if matches!(app.mode, Mode::RepoJump) {
        draw_repo_jump_popup(f, app, area);
    }

    if matches!(app.mode, Mode::RepoScanPicker) {
        draw_repo_scan_popup(f, app, area);
    }

    if matches!(app.mode, Mode::BranchSearchInput) {
        draw_branch_search_popup(f, app, area);
    }

    if matches!(app.mode, Mode::ImportUrlInput | Mode::ImportDestInput | Mode::ImportNameInput) {
        crate::popups::import::draw_import_popup(f, area, app);
    }

    if let Some(ref err) = app.error_message {
        crate::popups::error::draw_error_popup(f, app, area, err);
    } else if app.fetching {
        crate::popups::loading::draw_progress_popup(f, area, app);
    }

    if let Some(original_theme) = swapped_theme {
        crate::ui::update_theme(&original_theme);
    }
}

fn draw_outer_frame(f: &mut Frame, area: Rect, app: &App) {
    let show_sort = matches!(
        app.mode,
        Mode::Normal
            | Mode::Adding
            | Mode::Editing
            | Mode::ConfirmDelete
            | Mode::Help
            | Mode::About
            | Mode::Legend
            | Mode::BulkAddInput
    );

    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(muted_style())
        .title(
            Line::from(vec![
                Span::raw(" "),
                Span::styled("Gitwig", accent_style()),
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

    let mut right_spans =
        vec![Span::styled(format!(" v{} ", env!("CARGO_PKG_VERSION")), muted_style())];
    if let Some(ref latest) = app.update_available {
        right_spans.insert(0, Span::raw(" "));
        let badge_text = if app.is_msi_install() {
            format!("[New version v{}]", latest)
        } else {
            format!("[Update to v{}]", latest)
        };
        right_spans.insert(
            0,
            Span::styled(
                badge_text,
                Style::default().fg(ratatui::style::Color::LightGreen).add_modifier(Modifier::BOLD),
            ),
        );
    }
    if app.implicit_network_count > 0 {
        let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let idx = ((app.fetch_progress / 10) % 10) as usize;
        right_spans.insert(0, Span::raw(" "));
        right_spans.insert(
            0,
            Span::styled(
                spinner_chars[idx],
                Style::default().fg(ratatui::style::Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        );
    }
    block = block.title(Line::from(right_spans).alignment(Alignment::Right));
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
fn item_chunks(content_area: Rect, visible_count: usize, app: &App) -> Vec<Rect> {
    let rows = app.get_home_rows();
    let upper = (app.scroll_top + visible_count).min(rows.len());
    let visible_rows = &rows[app.scroll_top..upper];

    let mut constraints = Vec::new();
    for row in visible_rows {
        let h = match row {
            crate::app::HomeRow::GroupHeader { .. } => {
                if app.config.compact_view {
                    1
                } else {
                    2
                }
            }
            crate::app::HomeRow::Repo { path, .. } => {
                if app.config.compact_view {
                    1
                } else {
                    let has_note = app
                        .config
                        .repo_configs
                        .get(path)
                        .and_then(|cfg| cfg.note.as_ref())
                        .is_some();
                    if has_note { 5 } else { 4 }
                }
            }
        };
        constraints.push(Constraint::Length(h));
    }
    constraints.push(Constraint::Min(0));

    let chunks: Vec<Rect> = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(content_area)
        .to_vec();
    chunks[..visible_count].to_vec()
}

fn draw_global_summary_bar(f: &mut Frame, area: Rect, app: &App) {
    let mut total_repos = 0;
    let mut dirty_count = 0;
    let mut ahead_count = 0;
    let mut stale_count = 0;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let stale_threshold = 30 * 24 * 60 * 60; // 30 days

    for status in &app.statuses {
        match status {
            ItemStatus::GitRepo(Some(summary)) => {
                total_repos += 1;
                if !summary.is_clean() {
                    dirty_count += 1;
                }
                if summary.ahead > 0 {
                    ahead_count += 1;
                }
                if let Some(t) = summary.last_commit_time {
                    if now - t > stale_threshold {
                        stale_count += 1;
                    }
                }
            }
            ItemStatus::GitRepo(None) => {
                total_repos += 1;
            }
            _ => {}
        }
    }

    let dot = Span::styled("  •  ", muted_style());
    let repos_style = if app.global_filter.is_none() {
        primary_style().fg(ACCENT()).add_modifier(Modifier::UNDERLINED | Modifier::BOLD)
    } else {
        primary_style().fg(ACCENT())
    };
    let repos_label_style = if app.global_filter.is_none() {
        primary_style().add_modifier(Modifier::UNDERLINED | Modifier::BOLD)
    } else {
        muted_style()
    };
    let mut spans = vec![
        Span::styled(format!(" {} ", total_repos), repos_style),
        Span::styled("repos", repos_label_style),
    ];

    spans.push(dot.clone());
    let is_dirty_active = app.global_filter == Some(crate::app::GlobalFilter::Dirty);
    let dirty_color = if dirty_count > 0 { WARNING() } else { SUCCESS() };
    let dirty_style = if is_dirty_active {
        primary_style().fg(dirty_color).add_modifier(Modifier::UNDERLINED | Modifier::BOLD)
    } else {
        primary_style().fg(dirty_color)
    };
    let dirty_label_style = if is_dirty_active {
        primary_style().fg(dirty_color).add_modifier(Modifier::UNDERLINED | Modifier::BOLD)
    } else {
        muted_style()
    };
    spans.push(Span::styled(format!(" {} ", dirty_count), dirty_style));
    spans.push(Span::styled("dirty", dirty_label_style));

    spans.push(dot.clone());
    let is_ahead_active = app.global_filter == Some(crate::app::GlobalFilter::Ahead);
    let ahead_color = if ahead_count > 0 { ACCENT() } else { ratatui::style::Color::Gray };
    let ahead_style = if is_ahead_active {
        primary_style().fg(ahead_color).add_modifier(Modifier::UNDERLINED | Modifier::BOLD)
    } else {
        primary_style().fg(ahead_color)
    };
    let ahead_label_style = if is_ahead_active {
        primary_style().fg(ahead_color).add_modifier(Modifier::UNDERLINED | Modifier::BOLD)
    } else {
        muted_style()
    };
    spans.push(Span::styled(format!(" {} ", ahead_count), ahead_style));
    spans.push(Span::styled("ahead", ahead_label_style));

    spans.push(dot);
    let is_stale_active = app.global_filter == Some(crate::app::GlobalFilter::Stale);
    let stale_color = if stale_count > 0 { DANGER() } else { SUCCESS() };
    let stale_style = if is_stale_active {
        primary_style().fg(stale_color).add_modifier(Modifier::UNDERLINED | Modifier::BOLD)
    } else {
        primary_style().fg(stale_color)
    };
    let stale_label_style = if is_stale_active {
        primary_style().fg(stale_color).add_modifier(Modifier::UNDERLINED | Modifier::BOLD)
    } else {
        muted_style()
    };
    spans.push(Span::styled(format!(" {} ", stale_count), stale_style));
    spans.push(Span::styled("stale", stale_label_style));

    let line = Line::from(spans).alignment(Alignment::Center);
    f.render_widget(Paragraph::new(line), area);
}

fn draw_compact_headers(f: &mut Frame, area: Rect, _app: &App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(28),
            Constraint::Length(STATUS_ZONE_WIDTH),
        ])
        .split(area);

    let header_style = muted_style().add_modifier(Modifier::BOLD);

    let col_repo = Line::from(vec![Span::raw("   "), Span::styled("REPOSITORY", header_style)]);
    let col_branch = Line::from(Span::styled("ACTIVE BRANCH", header_style));
    let col_status = Line::from(Span::styled("STATUS", header_style)).alignment(Alignment::Right);

    f.render_widget(Paragraph::new(col_repo), cols[0]);
    f.render_widget(Paragraph::new(col_branch), cols[1]);
    f.render_widget(Paragraph::new(col_status), cols[2]);
}

fn draw_items(f: &mut Frame, app: &App, chunks: &[Rect]) {
    let rows = app.get_home_rows();
    let upper = (app.scroll_top + chunks.len()).min(rows.len());
    let visible_rows = &rows[app.scroll_top..upper];

    for (i, row) in visible_rows.iter().enumerate() {
        let display_index = i + app.scroll_top;
        let is_selected = display_index == app.selected_index;

        match row {
            crate::app::HomeRow::GroupHeader { name, count, collapsed } => {
                let arrow = if *collapsed { "▶" } else { "▼" };
                let header_text = format!("{} [{}] ({} repos)", arrow, name, count);
                let style = if is_selected {
                    Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)
                } else {
                    muted_style().add_modifier(Modifier::BOLD)
                };
                let mut block = Block::default();
                if !app.config.compact_view {
                    let border_style =
                        if is_selected { Style::default().fg(ACCENT()) } else { muted_style() };
                    block = block.borders(Borders::BOTTOM).border_style(border_style);
                }

                let p = Paragraph::new(Line::from(vec![
                    Span::styled(
                        if is_selected { app.sym("selection_mark") } else { UNSELECTED_INDENT },
                        if is_selected { Style::default().fg(ACCENT()) } else { Style::default() },
                    ),
                    Span::styled(header_text, style),
                ]))
                .block(block);
                f.render_widget(p, chunks[i]);
            }
            crate::app::HomeRow::Repo { actual_index, path: item, .. } => {
                let actual_index = *actual_index;
                let fallback = ItemStatus::Missing;
                let status = app.statuses.get(actual_index).unwrap_or(&fallback);
                let is_git = matches!(status, ItemStatus::GitRepo(_));

                let mut is_partial = false;
                if let ItemStatus::GitRepo(Some(summary)) = status {
                    if summary.staged > 0 && (summary.modified > 0 || summary.untracked > 0) {
                        is_partial = true;
                    }
                }

                let pending_delete = is_selected && matches!(app.mode, Mode::ConfirmDelete);
                let pending_edit = is_selected && matches!(app.mode, Mode::Editing);
                let is_pinned = app.config.pinned.contains(item);
                let is_starred = app.config.starred.contains(item);

                // Selected/pending cards use an accent color; unselected cards use
                // the terminal's default foreground (dimmed) so they stay legible
                // on both light and dark backgrounds.
                let border_style = if pending_delete {
                    Style::default().fg(DANGER())
                } else if pending_edit {
                    Style::default().fg(WARNING())
                } else if is_selected {
                    Style::default().fg(ACCENT())
                } else if is_partial || is_pinned {
                    Style::default().fg(WARNING())
                } else {
                    muted_style()
                };

                let (mark, mark_style, text_style) = if is_selected {
                    (app.sym("selection_mark"), border_style, primary_style())
                } else {
                    (UNSELECTED_INDENT, Style::default(), Style::default())
                };

                let repo_name = std::path::Path::new(item)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(item.as_str());

                if app.config.compact_view {
                    let cols = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Min(0),
                            Constraint::Length(28),
                            Constraint::Length(STATUS_ZONE_WIDTH),
                        ])
                        .split(chunks[i]);

                    // 1. Repo name and labels
                    let mut left_spans = vec![Span::styled(mark, mark_style)];
                    if is_git {
                        left_spans.push(Span::styled(
                            app.sym("git_repo"),
                            if is_selected {
                                primary_style().add_modifier(Modifier::BOLD)
                            } else {
                                muted_style().add_modifier(Modifier::BOLD)
                            },
                        ));
                    }
                    let mut name_prefix = String::new();
                    if !app.multi_selected.is_empty() {
                        if app.multi_selected.contains(item) {
                            name_prefix.push_str("[x] ");
                        } else {
                            name_prefix.push_str("[ ] ");
                        }
                    }
                    left_spans.push(Span::styled(
                        format!("{}{}", name_prefix, repo_name),
                        if is_selected {
                            Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)
                        } else {
                            text_style
                        },
                    ));
                    if let ItemStatus::GitRepo(Some(summary)) = status {
                        let (state_str, state_style) = match summary.state {
                            RepoState::Merge => (
                                " ⚠ MERGE_HEAD",
                                Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
                            ),
                            RepoState::Rebase => (
                                " 🚧 REBASING",
                                Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
                            ),
                            RepoState::CherryPick => (
                                " ⚡ CHERRY-PICK",
                                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
                            ),
                            RepoState::Revert => (
                                " ⚡ REVERTING",
                                Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
                            ),
                            RepoState::Bisect => (
                                " 🔍 BISECTING",
                                Style::default()
                                    .fg(ratatui::style::Color::Magenta)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            RepoState::ApplyMailbox => (
                                " 📬 APPLYING",
                                Style::default()
                                    .fg(ratatui::style::Color::Cyan)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            RepoState::Clean => (" ✓ CLEAN", muted_style()),
                        };
                        left_spans.push(Span::styled(
                            state_str,
                            if is_selected {
                                state_style
                                    .remove_modifier(Modifier::DIM)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                state_style
                            },
                        ));
                        if summary.staged > 0 && (summary.modified > 0 || summary.untracked > 0) {
                            left_spans.push(Span::styled(
                                " ⚠ PARTIAL",
                                Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
                            ));
                        }
                    }
                    if let Some(lbls) = app.config.labels.get(item) {
                        for lbl in lbls {
                            left_spans.push(Span::raw(" "));
                            left_spans.push(Span::styled(
                                format!("[{}]", lbl),
                                if is_selected {
                                    Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default().fg(ACCENT()).add_modifier(Modifier::DIM)
                                },
                            ));
                        }
                    }
                    let is_starred = app.config.starred.contains(item);
                    if is_starred {
                        left_spans.push(Span::raw(" "));
                        left_spans.push(Span::styled(
                            app.sym("star").trim(),
                            if is_selected {
                                Style::default()
                                    .fg(ratatui::style::Color::Yellow)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(ratatui::style::Color::Yellow)
                            },
                        ));
                    }
                    if is_pinned {
                        left_spans.push(Span::raw(" "));
                        left_spans.push(Span::styled(
                            app.sym("pinned").trim(),
                            if is_selected {
                                Style::default().fg(WARNING()).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(WARNING())
                            },
                        ));
                    }
                    f.render_widget(Paragraph::new(Line::from(left_spans)), cols[0]);

                    // 2. Branch and relative time
                    let branch_line = match status {
                        ItemStatus::GitRepo(Some(s)) => {
                            if let Some(b) = &s.branch {
                                let mut parts = vec![
                                    Span::styled(
                                        format!("{} ", app.sym("branch")),
                                        if is_selected {
                                            primary_style().add_modifier(Modifier::BOLD)
                                        } else {
                                            muted_style()
                                        },
                                    ),
                                    Span::styled(
                                        b.clone(),
                                        if is_selected {
                                            Style::default()
                                                .fg(ACCENT())
                                                .add_modifier(Modifier::BOLD)
                                        } else {
                                            Style::default().fg(ACCENT())
                                        },
                                    ),
                                ];
                                if let Some(time) = s.last_commit_time {
                                    parts.push(Span::styled(
                                        format!(" ({})", format_relative_time(time)),
                                        if is_selected {
                                            primary_style().add_modifier(Modifier::BOLD)
                                        } else {
                                            muted_style()
                                        },
                                    ));
                                }
                                Line::from(parts)
                            } else {
                                Line::from("")
                            }
                        }
                        _ => Line::from(""),
                    };
                    f.render_widget(Paragraph::new(branch_line), cols[1]);

                    // 3. Status indicator line
                    let status_line =
                        status_indicator_line(app, status, item).alignment(Alignment::Right);
                    f.render_widget(Paragraph::new(status_line), cols[2]);
                } else {
                    let border_type =
                        if is_selected { BorderType::LightDoubleDashed } else { CARD_BORDER() };

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
                        .constraints([
                            Constraint::Length(1),
                            Constraint::Length(1),
                            Constraint::Min(0),
                        ])
                        .split(inner);

                    let name_cols = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Min(0), Constraint::Length(8)])
                        .split(rows[0]);

                    let mut spans = vec![Span::styled(mark, mark_style)];
                    if is_git {
                        spans.push(Span::styled(
                            app.sym("git_repo"),
                            muted_style().add_modifier(Modifier::BOLD),
                        ));
                    }
                    let mut name_prefix = String::new();
                    if !app.multi_selected.is_empty() {
                        if app.multi_selected.contains(item) {
                            name_prefix.push_str("[x] ");
                        } else {
                            name_prefix.push_str("[ ] ");
                        }
                    }
                    spans.push(Span::styled(format!("{}{}", name_prefix, repo_name), text_style));
                    if let ItemStatus::GitRepo(Some(summary)) = status {
                        let (state_str, state_style) = match summary.state {
                            RepoState::Merge => (
                                " ⚠ MERGE_HEAD",
                                Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
                            ),
                            RepoState::Rebase => (
                                " 🚧 REBASING",
                                Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
                            ),
                            RepoState::CherryPick => (
                                " ⚡ CHERRY-PICK",
                                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
                            ),
                            RepoState::Revert => (
                                " ⚡ REVERTING",
                                Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
                            ),
                            RepoState::Bisect => (
                                " 🔍 BISECTING",
                                Style::default()
                                    .fg(ratatui::style::Color::Magenta)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            RepoState::ApplyMailbox => (
                                " 📬 APPLYING",
                                Style::default()
                                    .fg(ratatui::style::Color::Cyan)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            RepoState::Clean => (" ✓ CLEAN", muted_style()),
                        };
                        spans.push(Span::styled(state_str, state_style));
                        if summary.staged > 0 && (summary.modified > 0 || summary.untracked > 0) {
                            spans.push(Span::styled(
                                " ⚠ PARTIAL",
                                Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
                            ));
                        }
                    }
                    if let Some(lbls) = app.config.labels.get(item) {
                        for lbl in lbls {
                            spans.push(Span::raw(" "));
                            spans.push(Span::styled(
                                format!("[{}]", lbl),
                                Style::default().fg(ACCENT()).add_modifier(Modifier::DIM),
                            ));
                        }
                    }
                    let name_line = Line::from(spans);
                    f.render_widget(Paragraph::new(name_line), name_cols[0]);

                    let mut right_spans = Vec::new();
                    if is_starred {
                        right_spans.push(Span::styled(
                            format!("{} ", app.sym("star").trim()),
                            Style::default().fg(ratatui::style::Color::Yellow),
                        ));
                    }
                    if is_pinned {
                        right_spans.push(Span::styled(
                            app.sym("pinned").trim(),
                            Style::default().fg(WARNING()),
                        ));
                    }
                    if !right_spans.is_empty() {
                        let icon_line = Line::from(right_spans).alignment(Alignment::Right);
                        f.render_widget(Paragraph::new(icon_line), name_cols[1]);
                    }

                    // Row 1: Left column (branch name) and Right column (status section)
                    let row1_cols = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Min(0), Constraint::Length(STATUS_ZONE_WIDTH)])
                        .split(rows[1]);

                    let branch_line = match status {
                        ItemStatus::GitRepo(Some(s)) => {
                            if let Some(b) = &s.branch {
                                let mut parts = vec![
                                    Span::raw(UNSELECTED_INDENT),
                                    Span::styled(format!("{} ", app.sym("branch")), muted_style()),
                                    Span::styled(b.clone(), Style::default().fg(ACCENT())),
                                ];
                                if let Some(time) = s.last_commit_time {
                                    parts.push(Span::raw(" ("));
                                    parts.push(Span::styled(
                                        format_relative_time(time),
                                        muted_style(),
                                    ));
                                    parts.push(Span::raw(")"));
                                }
                                Line::from(parts)
                            } else {
                                Line::from("")
                            }
                        }
                        _ => Line::from(""),
                    };
                    f.render_widget(Paragraph::new(branch_line), row1_cols[0]);

                    let status_line =
                        status_indicator_line(app, status, item).alignment(Alignment::Right);
                    f.render_widget(Paragraph::new(status_line), row1_cols[1]);

                    let note_line = if let Some(repo_cfg) = app.config.repo_configs.get(item) {
                        if let Some(ref note) = repo_cfg.note {
                            Line::from(vec![
                                Span::raw(UNSELECTED_INDENT),
                                Span::styled("✎ ", muted_style()),
                                Span::styled(
                                    note.clone(),
                                    muted_style().add_modifier(Modifier::ITALIC),
                                ),
                            ])
                        } else {
                            Line::from("")
                        }
                    } else {
                        Line::from("")
                    };
                    f.render_widget(Paragraph::new(note_line), rows[2]);
                }
            }
        }
    }
}

/// Renders a centered empty-state message when no items are in the list.
fn draw_empty_state(f: &mut Frame, area: Rect, app: &App) {
    let popup_area = crate::ui::layout::centered_rect_fixed(70, 16, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(accent_style())
        .title(
            Line::from(vec![
                Span::raw(" "),
                Span::styled("Gitwig Onboarding", primary_style().add_modifier(Modifier::BOLD)),
                Span::raw(" "),
            ])
            .alignment(Alignment::Center),
        )
        .padding(Padding::vertical(1));

    let inner = block.inner(popup_area);
    f.render_widget(ratatui::widgets::Clear, popup_area);
    f.render_widget(block, popup_area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header title
            Constraint::Length(1), // Description
            Constraint::Length(1), // Spacer
            Constraint::Length(7), // Shortcuts
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Tip
        ])
        .split(inner);

    let is_compat = app.config.compatibility_mode;
    let welcome_glyph = if is_compat { "Welcome to Gitwig!" } else { "Welcome to Gitwig! 🌿" };
    let header =
        Line::from(Span::styled(welcome_glyph, primary_style().add_modifier(Modifier::BOLD)))
            .alignment(Alignment::Center);
    f.render_widget(Paragraph::new(header), layout[0]);

    let desc = Line::from(Span::styled(
        "Get started by tracking your first Git repository.",
        muted_style(),
    ))
    .alignment(Alignment::Center);
    f.render_widget(Paragraph::new(desc), layout[1]);

    let format_shortcut = |key: &str, desc: &str| -> Line<'static> {
        Line::from(vec![
            Span::raw("   "),
            Span::styled(format!("{:>4}", key), accent_style().add_modifier(Modifier::BOLD)),
            Span::styled("  ⟩  ", muted_style()),
            Span::raw(desc.to_string()),
        ])
    };

    let shortcut_lines = vec![
        format_shortcut("a", "Add a new local repository path"),
        format_shortcut("A", "Bulk add repositories in a directory"),
        format_shortcut("i", "Import a remote repository URL"),
        format_shortcut("?", "Open full shortcuts help reference"),
        format_shortcut("s", "Configure global settings"),
        format_shortcut("q", "Quit the application"),
    ];
    f.render_widget(Paragraph::new(shortcut_lines), layout[3]);

    let tip = Line::from(vec![
        Span::styled("Tip: ", primary_style().fg(WARNING())),
        Span::styled("paths support ~ expansion (e.g. ~/projects/my-repo)", muted_style()),
    ])
    .alignment(Alignment::Center);
    f.render_widget(Paragraph::new(tip), layout[5]);
}

/// Renders a centered empty-state message when search matches no repositories.
fn draw_search_empty_state(f: &mut Frame, area: Rect, query: &str) {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(25), Constraint::Min(0), Constraint::Percentage(40)])
        .split(area);

    let lines = vec![
        Line::from(vec![Span::styled(
            format!("No repositories matching '{}'.", query),
            primary_style(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::raw("Press  "),
            Span::styled("Esc", accent_style()),
            Span::raw("  to clear the search filter"),
        ]),
    ];

    let p = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(p, vert[1]);
}

/// Renders the per-item status as a colored symbol + (for git repos) a
/// compact set of `N+` (staged), `N!` (modified), `N?` (untracked),
/// `N↑` (commits ahead), `N↓` (commits behind) suffixes. Only non-zero
/// counts are shown so the indicator stays compact for the common case.
fn status_indicator_line(app: &App, status: &ItemStatus, item: &str) -> Line<'static> {
    if app.bulk_fetching.contains(item) {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let index = ((millis / 80) % 10) as usize;
        let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"][index];
        return Line::from(vec![
            Span::styled(
                format!("{} ", spinner),
                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("fetching...", Style::default().fg(ACCENT())),
        ]);
    }
    if let Some(res) = app.bulk_fetch_results.get(item) {
        return match res {
            Ok(_) => Line::from(vec![
                Span::styled("✓ ", Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)),
                Span::styled("done", Style::default().fg(SUCCESS())),
            ]),
            Err(_) => Line::from(vec![
                Span::styled("✗ ", Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
                Span::styled("failed", Style::default().fg(DANGER())),
            ]),
        };
    }

    match status {
        ItemStatus::Missing => Line::from(vec![
            Span::styled(app.sym("close"), Style::default().fg(DANGER())),
            Span::raw(" "),
            Span::styled("missing", muted_style()),
        ]),
        ItemStatus::Directory => Line::from(vec![
            Span::styled(app.sym("bullet_empty"), Style::default().fg(WARNING())),
            Span::raw(" "),
            Span::styled("dir", muted_style()),
        ]),
        ItemStatus::GitRepo(None) => Line::from(vec![
            Span::styled(app.sym("bullet_filled"), Style::default().fg(SUCCESS())),
            Span::raw(" "),
            Span::styled("?", muted_style()),
        ]),
        ItemStatus::GitRepo(Some(summary)) => repo_indicator_line(app, summary),
    }
}

fn repo_indicator_line(app: &App, summary: &RepoSummary) -> Line<'static> {
    let dot_color = if summary.conflicted > 0 { DANGER() } else { SUCCESS() };
    let mut spans = vec![Span::styled(app.sym("bullet_filled"), Style::default().fg(dot_color))];
    if summary.unchanged() {
        spans.push(Span::raw(" "));
        spans.push(Span::styled("clean", muted_style()));
        return Line::from(spans);
    }
    let ahead_style = if summary.ahead <= 3 {
        Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)
    } else if summary.ahead <= 10 {
        Style::default().fg(WARNING()).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)
    };

    let behind_style = if summary.behind <= 5 {
        Style::default().fg(WARNING()).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)
    };

    // Each (count, symbol, style) is rendered only if count > 0. The
    // ordering matches the Detail view's worktree section for consistency.
    let parts = [
        (summary.staged, "+", Style::default().fg(ACCENT())),
        (summary.modified, "!", Style::default().fg(WARNING())),
        (summary.untracked, "?", muted_style()),
        (summary.conflicted, app.sym("action").trim(), Style::default().fg(DANGER())),
        (summary.ahead, app.sym("up"), ahead_style),
        (summary.behind, app.sym("down"), behind_style),
    ];
    for (count, symbol, style) in parts {
        if count > 0 {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(format!("{}{}", count, symbol), style));
        }
    }
    Line::from(spans)
}

pub(crate) fn draw_input_status(
    f: &mut Frame,
    area: Rect,
    verb: &str,
    buffer: &str,
    is_compat: bool,
) {
    let mut spans = Vec::new();

    // Add Mode Badge
    spans.push(Span::styled(
        "INPUT",
        Style::default().fg(ratatui::style::Color::Red).add_modifier(Modifier::BOLD),
    ));

    let mode_sep = if is_compat { " > " } else { " ⟩ " };
    spans.push(Span::styled(mode_sep, muted_style()));

    let prefix = format!("{} › ", verb);
    spans.push(Span::styled(
        prefix.clone(),
        Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::styled(buffer.to_string(), primary_style()));

    let separator = if is_compat { " > " } else { " ⟩ " };
    spans.push(Span::styled(separator, muted_style()));
    spans.push(Span::raw("Save"));
    spans.push(Span::raw(" "));
    spans.push(Span::styled("[", muted_style()));
    spans.push(Span::styled("↵", accent_style()));
    spans.push(Span::styled("]", muted_style()));

    spans.push(Span::styled(separator, muted_style()));
    spans.push(Span::raw("Cancel"));
    spans.push(Span::raw(" "));
    spans.push(Span::styled("[", muted_style()));
    let cancel_key = if is_compat { "Esc" } else { "⎋" };
    spans.push(Span::styled(cancel_key, accent_style()));
    spans.push(Span::styled("]", muted_style()));

    let para = Paragraph::new(Line::from(spans));
    f.render_widget(para, area);

    // Cursor position calculation includes the Mode Badge (5 chars) and Mode Sep (3 chars)
    let badge_offset = 5 + 3;
    let cursor_offset = (badge_offset + prefix.chars().count() + buffer.chars().count()) as u16;
    let cursor_x = area.x.saturating_add(cursor_offset.min(area.width.saturating_sub(1)));
    f.set_cursor_position(Position::new(cursor_x, area.y));
}

pub(crate) fn wrap_str(s: &str, max_width: usize) -> Vec<String> {
    if s.is_empty() {
        return vec![String::new()];
    }
    let mut chunks = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let end = (i + max_width).min(chars.len());
        chunks.push(chars[i..end].iter().collect());
        i = end;
    }
    chunks
}

/// Pack comma-separated `val_str` items onto lines of at most `max_width` chars.
/// Items that are individually wider than `max_width` are hard-wrapped by
/// `wrap_str`. The function always returns at least one (possibly empty) entry.
pub(crate) fn wrap_excludes(val_str: &str, max_width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current_line = String::new();
    let parts: Vec<&str> = val_str.split(',').collect();
    for (idx, part) in parts.iter().enumerate() {
        let suffix = if idx + 1 < parts.len() { "," } else { "" };
        let item = format!("{}{}", part, suffix);

        if current_line.chars().count() + item.chars().count() > max_width {
            if !current_line.is_empty() {
                lines.push(current_line);
                current_line = String::new();
            }
            if item.chars().count() > max_width {
                let mut sub_chunks = wrap_str(&item, max_width);
                if let Some(last) = sub_chunks.pop() {
                    current_line = last;
                }
                lines.extend(sub_chunks);
            } else {
                current_line = item;
            }
        } else {
            current_line.push_str(&item);
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    if lines.is_empty() { vec![String::new()] } else { lines }
}

#[allow(clippy::needless_range_loop)]

/// Returns a `Rect` of `(percent_x, percent_y)` dimensions, centered inside `area`.

pub(crate) fn confirm_tag_delete_entries(
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
            Span::styled("y", Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
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

pub(crate) fn confirm_tag_push_entries(
    target: &str,
) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
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
            Span::styled("y", Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)),
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

pub(crate) fn confirm_tag_push_all_entries() -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let message_spans = Some(vec![
        Span::raw("Push "),
        Span::styled("ALL", Style::default().fg(WARNING()).add_modifier(Modifier::BOLD)),
        Span::raw(" local tags? "),
    ]);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("y", Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)),
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

#[cfg(unix)]
#[allow(unsafe_code)]
pub(crate) fn get_process_stats(app: &App) -> (f64, f64) {
    if let Ok(mut guard) = app.cpu_tracker.lock() {
        let now = std::time::Instant::now();

        // If 2000 ms has not elapsed, return the cached values
        if let Some((_, prev_time, cached_cpu, cached_rss)) = *guard {
            if now.duration_since(prev_time) < std::time::Duration::from_millis(2000) {
                return (cached_rss, cached_cpu);
            }
        }

        let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
        let res = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
        if res != 0 {
            if let Some((_, _, cached_cpu, cached_rss)) = *guard {
                return (cached_rss, cached_cpu);
            }
            return (0.0, 0.0);
        }
        let usage = unsafe { usage.assume_init() };

        #[cfg(target_os = "macos")]
        let rss_bytes = usage.ru_maxrss as f64;
        #[cfg(not(target_os = "macos"))]
        let rss_bytes = (usage.ru_maxrss * 1024) as f64;
        let rss_mb = rss_bytes / (1024.0 * 1024.0);

        let user_sec = usage.ru_utime.tv_sec as f64 + (usage.ru_utime.tv_usec as f64 / 1_000_000.0);
        let sys_sec = usage.ru_stime.tv_sec as f64 + (usage.ru_stime.tv_usec as f64 / 1_000_000.0);
        let total_cpu_sec = user_sec + sys_sec;

        let mut cpu_pct = 0.0;
        if let Some((prev_cpu, prev_time, _, _)) = *guard {
            let delta_cpu = total_cpu_sec - prev_cpu;
            let delta_time = now.duration_since(prev_time).as_secs_f64();
            if delta_time > 0.0 {
                cpu_pct = (delta_cpu / delta_time) * 100.0;
            }
        }
        *guard = Some((total_cpu_sec, now, cpu_pct, rss_mb));

        (rss_mb, cpu_pct)
    } else {
        (0.0, 0.0)
    }
}

#[cfg(not(unix))]
pub(crate) fn get_process_stats(_app: &App) -> (f64, f64) {
    (0.0, 0.0)
}

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
        Span::styled(status_text, Style::default().fg(ratatui::style::Color::Cyan)),
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::app::{App, DetailSection};
    use crate::components::cmd_bar::{detail_dismiss_entries, inspect_dismiss_entries};
    use crate::config::{Config, FzfConfig, SortOrder, ThemeConfig};
    use crate::repo::{FileEntry, ItemDetail, RepoInfo};
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_inspect_status_bar_shortcuts() {
        let config = Config {
            items: vec![],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            labels: std::collections::HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
            ..Default::default()
        };
        let mut app = App::new(config, PathBuf::from("dummy_path.toml"));

        // 1. Setup dirty working tree detail
        let mut info = RepoInfo::default();
        info.changes.staged.push(FileEntry { path: "file.txt".to_string(), label: "M" });

        app.current_detail =
            Some(ItemDetail::Repo { resolved: PathBuf::from("/dummy"), info: Box::new(info) });
        app.commit_list.selection = 0; // selection = uncommitted
        app.in_logs_ui = false;

        // A) Staged focus -> Unstage File [↵]
        app.detail_focus = DetailSection::Staged;
        let (_, entries) = inspect_dismiss_entries(&app);
        let entry_labels: Vec<String> = entries
            .iter()
            .map(|entry| {
                entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join("")
            })
            .collect();
        assert!(entry_labels.iter().any(|label| label.contains("Unstage File [↵]")));
        assert!(entry_labels.iter().any(|label| label.contains("Unstage All [a]")));
        assert!(entry_labels.iter().any(|label| label.contains("Discard [x]")));
        assert!(entry_labels.iter().any(|label| label.contains("Discard All [X]")));
        assert!(entry_labels.iter().any(|label| label.contains("Commit/Amend [c/C]")));

        // B) Unstaged focus -> Stage File [↵]
        app.detail_focus = DetailSection::Unstaged;
        let (_, entries) = inspect_dismiss_entries(&app);
        let entry_labels: Vec<String> = entries
            .iter()
            .map(|entry| {
                entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join("")
            })
            .collect();
        assert!(entry_labels.iter().any(|label| label.contains("Stage File [↵]")));
        assert!(entry_labels.iter().any(|label| label.contains("Stage All [a]")));
        assert!(entry_labels.iter().any(|label| label.contains("Discard [x]")));
        assert!(entry_labels.iter().any(|label| label.contains("Discard All [X]")));

        // C) StagingDetails with last_staging_focus == Staged -> Unstage Hunk [↵]
        app.detail_focus = DetailSection::StagingDetails;
        app.last_staging_focus = DetailSection::Staged;
        let (_, entries) = inspect_dismiss_entries(&app);
        let entry_labels: Vec<String> = entries
            .iter()
            .map(|entry| {
                entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join("")
            })
            .collect();
        assert!(entry_labels.iter().any(|label| label.contains("Unstage Hunk [↵]")));

        // D) StagingDetails with last_staging_focus == Unstaged -> Stage Hunk [↵]
        app.detail_focus = DetailSection::StagingDetails;
        app.last_staging_focus = DetailSection::Unstaged;
        let (_, entries) = inspect_dismiss_entries(&app);
        let entry_labels: Vec<String> = entries
            .iter()
            .map(|entry| {
                entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join("")
            })
            .collect();
        assert!(entry_labels.iter().any(|label| label.contains("Stage Hunk [↵]")));
        assert!(entry_labels.iter().any(|label| label.contains("Discard Hunk [x/Del]")));

        // D2) StagingDetails with last_staging_focus == Unstaged and diff_line_mode == true -> Stage Line [↵] & Discard Line [x/Del] & Hunk Mode [l]
        app.diff.diff_line_mode = true;
        let (_, entries) = inspect_dismiss_entries(&app);
        let entry_labels: Vec<String> = entries
            .iter()
            .map(|entry| {
                entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join("")
            })
            .collect();
        assert!(entry_labels.iter().any(|label| label.contains("Stage Line [↵]")));
        assert!(entry_labels.iter().any(|label| label.contains("Discard Line [x/Del]")));
        assert!(entry_labels.iter().any(|label| label.contains("Hunk Mode [l]")));

        // D3) Full screen diff mode -> Commit [c]
        app.inspect_full_diff = true;
        let (_, entries_full) = inspect_dismiss_entries(&app);
        let entry_labels_full: Vec<String> = entries_full
            .iter()
            .map(|entry| {
                entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join("")
            })
            .collect();
        assert!(entry_labels_full.iter().any(|label| label.contains("Commit/Amend [c/C]")));

        // E) If in_logs_ui is true, it should NOT render any staging entry
        app.inspect_full_diff = false;
        app.in_logs_ui = true;
        let (_, entries) = inspect_dismiss_entries(&app);
        let entry_labels: Vec<String> = entries
            .iter()
            .map(|entry| {
                entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join("")
            })
            .collect();
        assert!(
            !entry_labels.iter().any(|label| label.contains("Stage") || label.contains("Unstage"))
        );

        // F) Conflicts focus in Inspect mode -> Accept Ours [o] etc.
        app.in_logs_ui = false;
        app.detail_focus = DetailSection::Conflicts;
        let (_, entries_c) = inspect_dismiss_entries(&app);
        let entry_labels_c: Vec<String> = entries_c
            .iter()
            .map(|entry| {
                entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join("")
            })
            .collect();
        assert!(entry_labels_c.iter().any(|label| label.contains("Accept Ours [o]")));
        assert!(entry_labels_c.iter().any(|label| label.contains("Accept Theirs [t]")));
        assert!(entry_labels_c.iter().any(|label| label.contains("Mark Resolved [r]")));
        assert!(entry_labels_c.iter().any(|label| label.contains("Abort Merge [A]")));
        assert!(entry_labels_c.iter().any(|label| label.contains("Continue Merge [C]")));
        assert!(entry_labels_c.iter().any(|label| label.contains("Inspect [↵/→]")));

        // G) ConflictDiff focus in Inspect mode -> Accept Ours [o] etc.
        app.detail_focus = DetailSection::ConflictDiff;
        let (_, entries_cd) = inspect_dismiss_entries(&app);
        let entry_labels_cd: Vec<String> = entries_cd
            .iter()
            .map(|entry| {
                entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join("")
            })
            .collect();
        assert!(entry_labels_cd.iter().any(|label| label.contains("Accept Ours [o]")));
        assert!(entry_labels_cd.iter().any(|label| label.contains("Accept Theirs [t]")));
        assert!(entry_labels_cd.iter().any(|label| label.contains("Mark Resolved [r]")));
        assert!(entry_labels_cd.iter().any(|label| label.contains("Abort Merge [A]")));
        assert!(entry_labels_cd.iter().any(|label| label.contains("Continue Merge [C]")));
        assert!(entry_labels_cd.iter().any(|label| label.contains("Scroll Diff [↑↓/⇟⇞]")));
    }

    #[test]
    fn test_detail_dismiss_entries_shortcuts() {
        let config = Config {
            items: vec![],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            labels: std::collections::HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
            ..Default::default()
        };
        let mut app = App::new(config, PathBuf::from("dummy_path.toml"));

        // Tab 0: Workspace, Commits focus (default)
        app.detail_tab = 0;
        app.detail_focus = DetailSection::Commits;
        let (_, entries_w) = detail_dismiss_entries(&app);
        let entry_labels_w: Vec<String> = entries_w
            .iter()
            .map(|entry| {
                entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join("")
            })
            .collect();
        assert!(entry_labels_w.iter().any(|label| label.contains("Inspect [↵/→]")));
        assert!(entry_labels_w.iter().any(|label| label.contains("Tag [t]")));
        assert!(entry_labels_w.iter().any(|label| label.contains("Load More [G]")));
        assert!(entry_labels_w.iter().any(|label| label.contains("Yank Hash [y]")));

        // Setup uncommitted changes mock for Tab 0 uncommitted shortcuts
        let mut info = RepoInfo::default();
        info.changes.staged.push(FileEntry { path: "file.txt".to_string(), label: "M" });
        info.changes.unstaged.push(FileEntry { path: "other.txt".to_string(), label: "M" });
        app.current_detail =
            Some(ItemDetail::Repo { resolved: PathBuf::from("/dummy"), info: Box::new(info) });
        app.commit_list.selection = 0; // selection = uncommitted

        // Tab 0: Workspace, Staged focus
        app.detail_focus = DetailSection::Staged;
        let (_, entries_s) = detail_dismiss_entries(&app);
        let entry_labels_s: Vec<String> = entries_s
            .iter()
            .map(|entry| {
                entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join("")
            })
            .collect();
        assert!(entry_labels_s.iter().any(|label| label.contains("Inspect [→]")));
        assert!(entry_labels_s.iter().any(|label| label.contains("Unstage All [a]")));
        assert!(entry_labels_s.iter().any(|label| label.contains("Discard All [X]")));
        assert!(!entry_labels_s.iter().any(|label| label.contains("Tag [t]")));

        // Tab 0: Workspace, Unstaged focus
        app.detail_focus = DetailSection::Unstaged;
        let (_, entries_u) = detail_dismiss_entries(&app);
        let entry_labels_u: Vec<String> = entries_u
            .iter()
            .map(|entry| {
                entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join("")
            })
            .collect();
        assert!(entry_labels_u.iter().any(|label| label.contains("Stage All [a]")));
        assert!(entry_labels_u.iter().any(|label| label.contains("Discard All [X]")));

        // Tab 0: Workspace, StagingDetails focus
        app.detail_focus = DetailSection::StagingDetails;
        let (_, entries_sd) = detail_dismiss_entries(&app);
        let entry_labels_sd: Vec<String> = entries_sd
            .iter()
            .map(|entry| {
                entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join("")
            })
            .collect();
        assert!(entry_labels_sd.iter().any(|label| label.contains("Inspect [→]")));
        assert!(!entry_labels_sd.iter().any(|label| label.contains("Tag [t]")));

        // Tab 1: Files - Files Focus
        app.detail_tab = 1;
        app.detail_focus = DetailSection::Files;
        let (_, entries_f1) = detail_dismiss_entries(&app);
        let entry_labels_f1: Vec<String> = entries_f1
            .iter()
            .map(|entry| {
                entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join("")
            })
            .collect();
        assert!(entry_labels_f1.iter().any(|label| label.contains("Fuzzy Find [f]")));
        assert!(entry_labels_f1.iter().any(|label| label.contains("Expand/Collapse [←/→]")));
        assert!(entry_labels_f1.iter().any(|label| label.contains("History [⇧H]")));
        assert!(!entry_labels_f1.iter().any(|label| label.contains("Open in Editor [e/o]")));

        // Add a file tree item that is a directory
        app.file_tree.visible_files.push(crate::app::FileTreeItem {
            name: "src".to_string(),
            full_path: "src".to_string(),
            is_dir: true,
            depth: 0,
            is_expanded: true,
        });
        app.file_tree.file_list_selection = 0;
        let (_, entries_f1_dir) = detail_dismiss_entries(&app);
        let entry_labels_f1_dir: Vec<String> = entries_f1_dir
            .iter()
            .map(|entry| {
                entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join("")
            })
            .collect();
        assert!(!entry_labels_f1_dir.iter().any(|label| label.contains("Open in Editor [e/o]")));

        // Add a file tree item that is a file
        app.file_tree.visible_files.push(crate::app::FileTreeItem {
            name: "main.rs".to_string(),
            full_path: "src/main.rs".to_string(),
            is_dir: false,
            depth: 1,
            is_expanded: false,
        });
        app.file_tree.file_list_selection = 1;
        let (_, entries_f1_file) = detail_dismiss_entries(&app);
        let entry_labels_f1_file: Vec<String> = entries_f1_file
            .iter()
            .map(|entry| {
                entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join("")
            })
            .collect();
        assert!(entry_labels_f1_file.iter().any(|label| label.contains("Open in Editor [e/o]")));

        // Tab 1: Files - FileContent Focus
        app.detail_focus = DetailSection::FileContent;
        let (_, entries_f2) = detail_dismiss_entries(&app);
        let entry_labels_f2: Vec<String> = entries_f2
            .iter()
            .map(|entry| {
                entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join("")
            })
            .collect();
        assert!(!entry_labels_f2.iter().any(|label| label.contains("Fuzzy Find [f]")));
        assert!(!entry_labels_f2.iter().any(|label| label.contains("Expand/Collapse [←/→]")));
        assert!(!entry_labels_f2.iter().any(|label| label.contains("History [⇧H]")));
        assert!(entry_labels_f2.iter().any(|label| label.contains("Full Screen [→]")));
        assert!(entry_labels_f2.iter().any(|label| label.contains("Open in Editor [e/o]")));

        app.inspect_full_diff = true;
        let (_, entries_f2_full) = detail_dismiss_entries(&app);
        let entry_labels_f2_full: Vec<String> = entries_f2_full
            .iter()
            .map(|entry| {
                entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join("")
            })
            .collect();
        assert!(
            entry_labels_f2_full.iter().any(|label| label.contains("Exit Full Screen [←/⎋/q]"))
        );
        app.inspect_full_diff = false;

        // Tab 3: Branches - LocalBranches Focus
        app.detail_tab = 3;
        app.detail_focus = DetailSection::LocalBranches;
        let (_, entries_b1) = detail_dismiss_entries(&app);
        let entry_labels_b1: Vec<String> = entries_b1
            .iter()
            .map(|entry| {
                entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join("")
            })
            .collect();
        assert!(entry_labels_b1.iter().any(|label| label.contains("Fetch [f/F]")));
        assert!(entry_labels_b1.iter().any(|label| label.contains("Pull [p]")));
        assert!(entry_labels_b1.iter().any(|label| label.contains("Push [⇧P]")));

        // Tab 3: Branches - RemoteBranches Focus
        app.detail_focus = DetailSection::RemoteBranches;
        let (_, entries_b2) = detail_dismiss_entries(&app);
        let entry_labels_b2: Vec<String> = entries_b2
            .iter()
            .map(|entry| {
                entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join("")
            })
            .collect();
        assert!(!entry_labels_b2.iter().any(|label| label.contains("Fetch [f/F]")));
        assert!(!entry_labels_b2.iter().any(|label| label.contains("Pull [p]")));
        assert!(!entry_labels_b2.iter().any(|label| label.contains("Push [⇧P]")));

        // Tab 6: Stashes - Stashes Focus
        app.detail_tab = 6;
        app.detail_focus = DetailSection::Stashes;
        let (_, entries_s1) = detail_dismiss_entries(&app);
        let entry_labels_s1: Vec<String> = entries_s1
            .iter()
            .map(|entry| {
                entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join("")
            })
            .collect();
        assert!(entry_labels_s1.iter().any(|label| label.contains("Apply [a]")));
        assert!(entry_labels_s1.iter().any(|label| label.contains("Delete [D]")));

        // Tab 6: Stashes - StashedFiles Focus
        app.detail_focus = DetailSection::StashedFiles;
        let (_, entries_s2) = detail_dismiss_entries(&app);
        let entry_labels_s2: Vec<String> = entries_s2
            .iter()
            .map(|entry| {
                entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join("")
            })
            .collect();
        assert!(!entry_labels_s2.iter().any(|label| label.contains("Apply [a]")));
        assert!(!entry_labels_s2.iter().any(|label| label.contains("Delete [D]")));

        // Tab 4: Tags
        app.detail_tab = 4;
        let (_, entries_tags) = detail_dismiss_entries(&app);
        let entry_labels_tags: Vec<String> = entries_tags
            .iter()
            .map(|entry| {
                entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join("")
            })
            .collect();
        assert!(entry_labels_tags.iter().any(|label| label.contains("Fetch [f/F]")));
    }

    #[test]
    fn test_wrap_str() {
        let wrapped = wrap_str("node_modules,target,build,dist", 10);
        assert_eq!(
            wrapped,
            vec!["node_modul".to_string(), "es,target,".to_string(), "build,dist".to_string(),]
        );

        let wrapped_empty = wrap_str("", 10);
        assert_eq!(wrapped_empty, vec!["".to_string()]);
    }

    #[test]
    fn test_wrap_excludes() {
        // Items that fit together on one line stay together
        let w = wrap_excludes("node_modules,target", 30);
        assert_eq!(w, vec!["node_modules,target"]);

        // Items that overflow wrap to the next line
        let w = wrap_excludes("node_modules,target,build,dist", 20);
        // "node_modules," = 13, "target," = 7 → 20 fits on line 1
        // "build," = 6, "dist" = 4 → fits on line 2
        assert_eq!(w, vec!["node_modules,target,", "build,dist"]);

        // Single very long item gets hard-wrapped
        let w = wrap_excludes("averylongnamehere", 10);
        assert_eq!(w, vec!["averylongn", "amehere"]);

        // Empty string returns a single empty line
        let w = wrap_excludes("", 20);
        assert_eq!(w, vec![""]);
    }

    #[test]
    fn test_settings_val_str() {
        let config = Config {
            editor: "vim".to_string(),
            ssh_strict_host_checking: true,
            ..Default::default()
        };
        let mut app = App::new(config, PathBuf::from("dummy.toml"));

        // Setting index 55: SSH Strict Host Checking
        let val_ssh = crate::popups::settings::get_val_str(&app, 55);
        assert_eq!(val_ssh, "true");

        // Setting index 56: Editor Command
        app.settings_selected_index = 56;
        app.settings_editing = false;
        let val_editor = crate::popups::settings::get_val_str(&app, 56);
        assert_eq!(val_editor, "vim");

        // During editing
        app.settings_editing = true;
        app.input_buffer = "nano".to_string();
        let val_editor_edit = crate::popups::settings::get_val_str(&app, 56);
        assert_eq!(val_editor_edit, "nano█");
    }

    #[test]
    fn test_repo_indicator_line_divergence() {
        let config = Config::default();
        let app = App::new(config, PathBuf::from("dummy.toml"));

        // Case 1: Ahead <= 3 (Green)
        let summary1 = RepoSummary {
            ahead: 2,
            behind: 0,
            staged: 0,
            modified: 0,
            untracked: 0,
            conflicted: 0,
            branch: None,
            state: RepoState::Clean,
            last_commit_time: None,
        };
        let line1 = repo_indicator_line(&app, &summary1);
        let ahead_span = line1.spans.iter().find(|s| s.content.contains(app.sym("up"))).unwrap();
        assert_eq!(ahead_span.style.fg, Some(SUCCESS()));

        // Case 2: Ahead <= 10 (Yellow)
        let summary2 = RepoSummary {
            ahead: 7,
            behind: 0,
            staged: 0,
            modified: 0,
            untracked: 0,
            conflicted: 0,
            branch: None,
            state: RepoState::Clean,
            last_commit_time: None,
        };
        let line2 = repo_indicator_line(&app, &summary2);
        let ahead_span2 = line2.spans.iter().find(|s| s.content.contains(app.sym("up"))).unwrap();
        assert_eq!(ahead_span2.style.fg, Some(WARNING()));

        // Case 3: Behind <= 5 (Yellow)
        let summary3 = RepoSummary {
            ahead: 0,
            behind: 3,
            staged: 0,
            modified: 0,
            untracked: 0,
            conflicted: 0,
            branch: None,
            state: RepoState::Clean,
            last_commit_time: None,
        };
        let line3 = repo_indicator_line(&app, &summary3);
        let behind_span = line3.spans.iter().find(|s| s.content.contains(app.sym("down"))).unwrap();
        assert_eq!(behind_span.style.fg, Some(WARNING()));

        // Case 4: Behind > 5 (Red)
        let summary4 = RepoSummary {
            ahead: 0,
            behind: 8,
            staged: 0,
            modified: 0,
            untracked: 0,
            conflicted: 0,
            branch: None,
            state: RepoState::Clean,
            last_commit_time: None,
        };
        let line4 = repo_indicator_line(&app, &summary4);
        let behind_span2 =
            line4.spans.iter().find(|s| s.content.contains(app.sym("down"))).unwrap();
        assert_eq!(behind_span2.style.fg, Some(DANGER()));
    }

    #[test]
    fn test_draw_global_summary_bar() {
        let config = Config {
            items: vec!["/path/to/repo_a".to_string(), "/path/to/repo_b".to_string()],
            ..Default::default()
        };
        let mut app = App::new(config, PathBuf::from("dummy_path.toml"));

        let summary_clean = RepoSummary {
            branch: Some("main".to_string()),
            staged: 0,
            modified: 0,
            untracked: 0,
            conflicted: 0,
            ahead: 0,
            behind: 0,
            state: RepoState::Clean,
            last_commit_time: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64,
            ),
        };

        let summary_dirty = RepoSummary {
            branch: Some("main".to_string()),
            staged: 1,
            modified: 0,
            untracked: 0,
            conflicted: 0,
            ahead: 2,
            behind: 0,
            state: RepoState::Clean,
            last_commit_time: Some(0), // very stale
        };

        app.statuses = vec![
            ItemStatus::GitRepo(Some(summary_clean)),
            ItemStatus::GitRepo(Some(summary_dirty)),
        ];

        let backend = ratatui::backend::TestBackend::new(80, 1);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                draw_global_summary_bar(f, Rect::new(0, 0, 80, 1), &app);
            })
            .unwrap();

        // Verify correct rendering of stats in the buffer
        let buffer = terminal.backend().buffer();
        let text: String = (0..80).map(|x| buffer[(x, 0)].symbol()).collect();
        let trimmed = text.trim();
        assert!(trimmed.contains("2 repos"), "Buffer contents: {}", trimmed);
        assert!(trimmed.contains("1 dirty"), "Buffer contents: {}", trimmed);
        assert!(trimmed.contains("1 ahead"), "Buffer contents: {}", trimmed);
        assert!(trimmed.contains("1 stale"), "Buffer contents: {}", trimmed);
    }

    #[test]
    fn test_draw_partial_uncommitted_badge() {
        let config = Config { items: vec!["/path/to/repo_a".to_string()], ..Default::default() };
        let mut app = App::new(config, PathBuf::from("dummy_path.toml"));

        // Has both staged and unstaged (modified) changes -> should show PARTIAL badge
        let summary_partial = RepoSummary {
            branch: Some("main".to_string()),
            staged: 1,
            modified: 1,
            untracked: 0,
            conflicted: 0,
            ahead: 0,
            behind: 0,
            state: RepoState::Clean,
            last_commit_time: None,
        };

        app.statuses = vec![ItemStatus::GitRepo(Some(summary_partial))];

        let backend = ratatui::backend::TestBackend::new(80, 5);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let chunks = vec![Rect::new(0, 0, 80, 4)];
                draw_items(f, &app, &chunks);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut found_partial = false;
        for y in 0..5 {
            let row_text: String = (0..80).map(|x| buffer[(x, y)].symbol()).collect();
            if row_text.contains("PARTIAL") {
                found_partial = true;
                break;
            }
        }
        assert!(found_partial, "Expected to find PARTIAL badge in card rendering");
    }
}
