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
) {
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
        draw_empty_state(f, content_area);
    } else if app.get_items_len() == 0 {
        if let Some(ref query) = app.repo_search_query {
            draw_search_empty_state(f, content_area, query);
        } else {
            draw_empty_state(f, content_area);
        }
    } else {
        let list_chunks = item_chunks(content_area, visible_count, app);
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

    if matches!(app.mode, Mode::RepoSettings) {
        crate::popups::repo_settings::RepoSettingsPopup::draw(f, app, area);
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
        right_spans.insert(
            0,
            Span::styled(
                format!("[Update to v{}]", latest),
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
    let mut constraints = vec![Constraint::Length(app.item_height()); visible_count];
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
    let filtered_items = app.get_filtered_items();
    let upper = (app.scroll_top + chunks.len()).min(filtered_items.len());
    let visible_items = &filtered_items[app.scroll_top..upper];

    for (i, &(actual_index, item)) in visible_items.iter().enumerate() {
        let display_index = i + app.scroll_top;
        let is_selected = display_index == app.selected_index;
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
            (app.sym("selection_mark"), border_style, primary_style())
        } else {
            (UNSELECTED_INDENT, Style::default(), Style::default())
        };

        let repo_name = std::path::Path::new(item)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(item.as_str());

        let fallback = ItemStatus::Missing;
        let status = app.statuses.get(actual_index).unwrap_or(&fallback);
        let is_git = matches!(status, ItemStatus::GitRepo(_));

        if app.config.compact_view {
            let row_style = if is_selected {
                Style::default().bg(ACCENT()).fg(ratatui::style::Color::Black)
            } else if is_pinned {
                Style::default().fg(WARNING())
            } else {
                Style::default()
            };

            // Draw selection background block
            if is_selected {
                f.render_widget(Block::default().style(row_style), chunks[i]);
            }

            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(28),
                    Constraint::Length(STATUS_ZONE_WIDTH),
                ])
                .split(chunks[i]);

            // 1. Repo name and labels
            let mut left_spans =
                vec![Span::styled(mark, if is_selected { row_style } else { mark_style })];
            if is_git {
                left_spans.push(Span::styled(
                    app.sym("git_repo"),
                    if is_selected {
                        row_style
                    } else {
                        muted_style().add_modifier(Modifier::BOLD)
                    },
                ));
            }
            left_spans.push(Span::styled(
                repo_name,
                if is_selected { row_style.add_modifier(Modifier::BOLD) } else { text_style },
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
                    if is_selected { row_style } else { state_style },
                ));
            }
            if let Some(lbls) = app.config.labels.get(item) {
                for lbl in lbls {
                    left_spans.push(Span::raw(" "));
                    left_spans.push(Span::styled(
                        format!("[{}]", lbl),
                        if is_selected {
                            row_style
                        } else {
                            Style::default().fg(ACCENT()).add_modifier(Modifier::DIM)
                        },
                    ));
                }
            }
            if is_pinned {
                left_spans.push(Span::raw(" "));
                left_spans.push(Span::styled(
                    app.sym("pinned").trim(),
                    if is_selected { row_style } else { Style::default().fg(WARNING()) },
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
                                if is_selected { row_style } else { muted_style() },
                            ),
                            Span::styled(
                                b.clone(),
                                if is_selected {
                                    row_style.add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default().fg(ACCENT())
                                },
                            ),
                        ];
                        if let Some(time) = s.last_commit_time {
                            parts.push(Span::styled(
                                format!(" ({})", format_relative_time(time)),
                                if is_selected { row_style } else { muted_style() },
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
            let status_line = status_indicator_line(app, status).alignment(Alignment::Right);
            let status_line = if is_selected {
                let mut mapped_spans = Vec::new();
                for span in &status_line.spans {
                    mapped_spans.push(Span::styled(span.content.to_string(), row_style));
                }
                Line::from(mapped_spans).alignment(Alignment::Right)
            } else {
                status_line
            };
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
                .constraints([Constraint::Length(1), Constraint::Min(0)])
                .split(inner);

            let name_cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(0), Constraint::Length(4)])
                .split(rows[0]);

            let mut spans = vec![Span::styled(mark, mark_style)];
            if is_git {
                spans.push(Span::styled(
                    app.sym("git_repo"),
                    muted_style().add_modifier(Modifier::BOLD),
                ));
            }
            spans.push(Span::styled(repo_name, text_style));
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

            if is_pinned {
                let pin_line = Line::from(Span::styled(
                    app.sym("pinned").trim(),
                    Style::default().fg(WARNING()),
                ))
                .alignment(Alignment::Right);
                f.render_widget(Paragraph::new(pin_line), name_cols[1]);
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
                            parts.push(Span::styled(format_relative_time(time), muted_style()));
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

            let status_line = status_indicator_line(app, status).alignment(Alignment::Right);
            f.render_widget(Paragraph::new(status_line), row1_cols[1]);
        }
    }
}

/// Renders a centered empty-state message when no items are in the list.
fn draw_empty_state(f: &mut Frame, area: Rect) {
    // Vertical: push content to the upper-middle third of the area.
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(25), Constraint::Min(0), Constraint::Percentage(40)])
        .split(area);

    let lines = vec![
        Line::from(vec![Span::styled("No repositories tracked yet.", primary_style())]),
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
            Span::styled("paths support ~ expansion  (e.g. ~/code/my-project)", muted_style()),
        ]),
    ];

    let para = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(para, vert[1]);
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
fn status_indicator_line(app: &App, status: &ItemStatus) -> Line<'static> {
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
    // Each (count, symbol, style) is rendered only if count > 0. The
    // ordering matches the Detail view's worktree section for consistency.
    let parts = [
        (summary.staged, "+", Style::default().fg(ACCENT())),
        (summary.modified, "!", Style::default().fg(WARNING())),
        (summary.untracked, "?", muted_style()),
        (summary.conflicted, app.sym("action").trim(), Style::default().fg(DANGER())),
        (summary.ahead, app.sym("up"), primary_style()),
        (summary.behind, app.sym("down"), Style::default().fg(WARNING())),
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
}
