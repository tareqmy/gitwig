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
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding};

use crate::app::{App, Mode};

// ── Theme ──────────────────────────────────────────────────────────────────
// Colors are kept minimal so the app works on both dark and light terminal
// backgrounds. Plain text is left at the terminal's default foreground —
// never hard-code `White` / `Gray` / `DarkGray` for text, because what reads
// as "muted" on one background reads as invisible on the other. Use the
// helpers below: `muted_style()` for de-emphasis, bold for emphasis, and the
// accent colors only for true accents (selection, mode, warnings).

pub use super::style::*;

mod helpers;
mod home;
mod popups;

pub(crate) use helpers::*;
pub use home::*;
pub use popups::*;

use home::{
    draw_compact_headers, draw_empty_state, draw_filter_empty_state, draw_home_header, draw_items,
    draw_label_badge, draw_pin_rules, draw_search_empty_state, home_header_layout, item_chunks,
};
use popups::draw_not_git_repo_popup;

#[cfg(test)]
mod tests;

/// Whether `mode` draws the repository detail view as its base (as opposed to
/// the home list, Settings, File History, or Debug Logs). `draw` routes these
/// modes to `ui_detail::draw`, which also paints their popups: a popup mode
/// missing here draws the home list and no popup at all. Also decides when the
/// home-list label tint may apply.
fn is_detail_base_mode(mode: &Mode) -> bool {
    matches!(
        mode,
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
            | Mode::SubmoduleUpdateConfirm
            | Mode::TagCheckoutConfirm
            | Mode::CommitCheckoutConfirm
            | Mode::BranchPushConfirm
            | Mode::BranchMergeConfirm
            | Mode::BranchMergeIntoConfirm
            | Mode::BranchRebaseConfirm
            | Mode::BranchInteractiveRebaseConfirm
            | Mode::TagDeleteConfirm
            | Mode::TagOverwriteConfirm
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
            | Mode::CommitHistoryPicker
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
            | Mode::BranchSearchInput
            | Mode::FileSearchInput
            | Mode::CommitFuzzySearch
            | Mode::TagSearchInput
            | Mode::ForgeCommentPathInput
            | Mode::ForgeCommentLineInput
            | Mode::ForgeCommentBodyInput
            | Mode::ForgeCheckoutConfirm
    )
}

struct ThemeGuard(Option<crate::config::ThemeConfig>);

impl Drop for ThemeGuard {
    fn drop(&mut self) {
        if let Some(ref original) = self.0 {
            crate::ui::update_theme(original);
        }
    }
}

fn snapshot_current_theme() -> crate::config::ThemeConfig {
    let lock = crate::ui::style::THEME.read().unwrap_or_else(std::sync::PoisonError::into_inner);
    crate::config::ThemeConfig {
        accent: crate::ui::style::format_color(lock.accent),
        warning: crate::ui::style::format_color(lock.warning),
        danger: crate::ui::style::format_color(lock.danger),
        success: crate::ui::style::format_color(lock.success),
        border_type: crate::ui::style::format_border_type(lock.border_type),
    }
}

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
    quick_label_area: &mut Option<Rect>,
) {
    *global_summary_area = None;
    *quick_label_area = None;
    let mut theme_guard = ThemeGuard(None);
    if is_detail_base_mode(&app.mode) {
        if let Some(repo_path) = app.active_repo_item() {
            if let Some(repo_theme) = app.repo_theme_cache.get(repo_path) {
                theme_guard.0 = Some(snapshot_current_theme());
                crate::ui::update_theme(repo_theme);
            }
        }
    }

    // Home list with an active label filter ("project view"): tint the whole
    // view with that label's theme when one is set. Skipped when a repo detail
    // view (which applies the repo's own resolved theme) or a non-home base
    // like Settings is showing, so the tint stays a home-list concept.
    if theme_guard.0.is_none()
        && app.loading_repo_path.is_none()
        && !is_detail_base_mode(&app.mode)
        && !matches!(app.mode, Mode::FileHistory | Mode::Settings | Mode::DebugLogs)
        && !(app.mode == Mode::UpdateConfirm && app.current_detail.is_some())
    {
        if let Some(label) = &app.state.active_label_filter {
            if let Some(label_theme) = app.label_theme_cache.get(label) {
                theme_guard.0 = Some(snapshot_current_theme());
                crate::ui::update_theme(label_theme);
            }
        }
    }

    draw_outer_frame(f, area, app);

    // Always reserve the bottom row for the status bar, regardless of mode.
    let (content_area, status_chunk) = content_and_status_chunks(inner_area, app.status_height());

    // The embedded terminal panel sits between the content and the status
    // bar in every base mode; the content shrinks to make room.
    let terminal_height = app.terminal_panel_outer_height(inner_area.height);
    let (content_area, terminal_area) = if terminal_height > 0 {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(terminal_height)])
            .split(content_area);
        (chunks[0], Some(chunks[1]))
    } else {
        (content_area, None)
    };
    app.terminal_panel_area.set(terminal_area);

    if app.loading_repo_path.is_some() {
        crate::popups::loading::draw_loading_screen(f, content_area, app);
    } else if is_detail_base_mode(&app.mode)
        || (app.mode == Mode::UpdateConfirm && app.current_detail.is_some())
    {
        if let Some(detail) = &app.current_detail {
            let item_name = app.active_repo_item().map(String::as_str).unwrap_or("");
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
                app.graph_selection,
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
        // Repositories exist but the active view is empty: keep the summary
        // tab bar visible (and clickable) so the user can see which filter is
        // active and switch away, instead of a dead-end onboarding screen.
        let header = home_header_layout(content_area, app);
        draw_home_header(f, &header, app, global_summary_area, quick_label_area);

        if let Some(ref query) = app.repo_search_query {
            draw_search_empty_state(f, header.body, query);
        } else {
            draw_filter_empty_state(f, header.body, app);
        }
    } else {
        let header = home_header_layout(content_area, app);
        draw_home_header(f, &header, app, global_summary_area, quick_label_area);
        let list_area_parent = header.body;

        let (header_area, list_area) =
            if app.effective_view_mode() == crate::config::HomeViewMode::Compact {
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

        let (list_chunks, pin_rules) = item_chunks(list_area, visible_count, app);
        *main_areas = list_chunks.clone();
        draw_items(f, app, &list_chunks);
        draw_pin_rules(f, app, &pin_rules);
    }

    if let Some(term_area) = terminal_area {
        crate::components::terminal_panel::draw_terminal_panel(f, app, term_area);
    }

    crate::components::cmd_bar::draw_status_bar(f, app, status_chunk);

    if matches!(app.mode, Mode::Help) {
        crate::popups::help::draw_help_overlay(f, app, area, app.help_scroll);
    }

    if matches!(app.mode, Mode::About) {
        crate::popups::about::draw_about_popup(f, area, app);
    }

    if matches!(app.mode, Mode::StatsDashboard) {
        crate::popups::stats::StatsPopup::draw(f, app, area);
    }

    if matches!(app.mode, Mode::NotGitRepo) {
        draw_not_git_repo_popup(f, area, app);
    }

    if matches!(app.mode, Mode::AddCwdRepoConfirm) {
        crate::popups::confirm::draw_add_cwd_repo_popup(f, area, app);
    }

    if matches!(app.mode, Mode::Legend) {
        crate::popups::legend::draw_legend_popup(f, area, app);
    }

    if matches!(app.mode, Mode::RepoSettings) {
        crate::popups::repo_settings::RepoSettingsPopup::draw(f, app, area);
    }

    if matches!(app.mode, Mode::GlobalSearch) {
        draw_global_search_popup(f, app, area);
    }

    if matches!(app.mode, Mode::RepoJump) {
        draw_repo_jump_popup(f, app, area);
    }

    if matches!(app.mode, Mode::LabelPicker) {
        draw_label_picker_popup(f, app, area);
    }

    if matches!(app.mode, Mode::LabelSettings) {
        crate::popups::label_settings::LabelSettingsPopup::draw(f, app, area);
    }

    if matches!(app.mode, Mode::RepoScanPicker) {
        draw_repo_scan_popup(f, app, area);
    }

    if matches!(app.mode, Mode::BulkAddScanPicker) {
        draw_bulk_add_scan_popup(f, app, area);
    }

    if matches!(app.mode, Mode::BranchSearchInput) {
        draw_branch_search_popup(f, app, area);
    }

    if matches!(app.mode, Mode::FileSearchInput) {
        draw_file_search_popup(f, app, area);
    }

    if matches!(app.mode, Mode::CommitFuzzySearch) {
        draw_commit_fuzzy_popup(f, app, area);
    }

    if matches!(app.mode, Mode::TagSearchInput) {
        draw_tag_search_popup(f, app, area);
    }

    if matches!(app.mode, Mode::ImportUrlInput | Mode::ImportDestInput | Mode::ImportNameInput) {
        crate::popups::import::draw_import_popup(f, area, app);
    }

    // The palette floats over whichever view is underneath (home or detail);
    // only an error popup outranks it.
    crate::popups::command_palette::CommandPalettePopup::draw(f, app, area);

    if let Some(ref err) = app.error_message {
        crate::popups::error::draw_error_popup(f, app, area, err);
    } else if app.fetching {
        crate::popups::loading::draw_progress_popup(f, area, app);
    }

    // The active-label badge lives on the frame's top border, but only while
    // the home header (summary bar) is showing beneath it.
    if global_summary_area.is_some() {
        draw_label_badge(f, area, app);
    } else {
        app.label_badge_area.set(None);
    }
}

fn draw_outer_frame(f: &mut Frame, area: Rect, app: &App) {
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

    let mut right_spans =
        vec![Span::styled(format!(" v{} ", env!("CARGO_PKG_VERSION")), muted_style())];
    if let Some(ref latest) = app.update_available {
        right_spans.insert(0, Span::raw(" "));
        let badge_text = if app.can_self_update() {
            format!("[Update to v{}]", latest)
        } else {
            format!("[New version v{}]", latest)
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
        right_spans.insert(0, Span::styled(spinner_chars[idx], accent_style()));
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
