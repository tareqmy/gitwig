use crate::app::{App, DetailSection, Mode, Splitter};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Position;

pub fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    let is_click = mouse.kind == MouseEventKind::Down(MouseButton::Left);
    let is_drag = mouse.kind == MouseEventKind::Drag(MouseButton::Left);
    let is_release = mouse.kind == MouseEventKind::Up(MouseButton::Left);
    let is_scroll_up = mouse.kind == MouseEventKind::ScrollUp;
    let is_scroll_down = mouse.kind == MouseEventKind::ScrollDown;

    if !is_click && !is_drag && !is_release && !is_scroll_up && !is_scroll_down {
        return;
    }

    if app.error_message.is_some() {
        if is_click {
            app.error_message = None;
        }
        return;
    }

    if app.fetching || app.loading_repo_path.is_some() {
        return;
    }

    let pos = Position { x: mouse.column, y: mouse.row };

    let areas = app.detail_areas;

    // Handle splitter dragging
    if let Some(splitter) = app.active_drag_splitter {
        if is_release {
            app.active_drag_splitter = None;
        } else if is_drag {
            match splitter {
                Splitter::InspectHorizontal => {
                    if let (Some(left), Some(right)) = (areas.bottom_left, areas.bottom_right) {
                        let start_x = areas.commit_details.map(|r| r.x).unwrap_or(left.x);
                        let total_width = (right.x + right.width).saturating_sub(start_x);
                        if total_width > 0 {
                            let relative_x = pos.x.saturating_sub(start_x);
                            let pct = ((relative_x as f32 / total_width as f32) * 100.0) as u16;
                            app.inspect_horizontal_split_pct = pct.clamp(15, 85);
                        }
                    }
                }
                Splitter::InspectVertical => {
                    let mut start_y = None;
                    let mut total_height = None;

                    if let (Some(top), Some(bottom)) = (areas.staged_sub, areas.unstaged_sub) {
                        start_y = Some(top.y);
                        total_height = Some((bottom.y + bottom.height).saturating_sub(top.y));
                    } else if let (Some(top), Some(bottom)) =
                        (areas.commit_details, areas.bottom_left)
                    {
                        start_y = Some(top.y);
                        total_height = Some((bottom.y + bottom.height).saturating_sub(top.y));
                    }

                    if let (Some(sy), Some(th)) = (start_y, total_height) {
                        if th > 0 {
                            let relative_y = pos.y.saturating_sub(sy);
                            let pct = ((relative_y as f32 / th as f32) * 100.0) as u16;
                            app.inspect_vertical_split_pct = pct.clamp(15, 85);
                        }
                    }
                }
                Splitter::WorkspaceMain => {
                    if let (Some(top), Some(bottom)) = (areas.commits, areas.bottom_right) {
                        let start_y = top.y;
                        let total_height = (bottom.y + bottom.height).saturating_sub(start_y);
                        if total_height > 0 {
                            let relative_y = pos.y.saturating_sub(start_y);
                            let pct = ((relative_y as f32 / total_height as f32) * 100.0) as u16;
                            app.workspace_main_split_pct = pct.clamp(15, 85);
                        }
                    }
                }
                Splitter::FilesHorizontal => {
                    if let (Some(left), Some(right)) = (areas.files, areas.file_content) {
                        let start_x = left.x;
                        let total_width = (right.x + right.width).saturating_sub(start_x);
                        if total_width > 0 {
                            let relative_x = pos.x.saturating_sub(start_x);
                            let pct = ((relative_x as f32 / total_width as f32) * 100.0) as u16;
                            app.files_horizontal_split_pct = pct.clamp(15, 85);
                        }
                    }
                }
                Splitter::BranchesHorizontal => {
                    if let (Some(left), Some(right)) = (areas.local_branches, areas.remote_branches)
                    {
                        let start_x = left.x;
                        let total_width = (right.x + right.width).saturating_sub(start_x);
                        if total_width > 0 {
                            let relative_x = pos.x.saturating_sub(start_x);
                            let pct = ((relative_x as f32 / total_width as f32) * 100.0) as u16;
                            app.branches_horizontal_split_pct = pct.clamp(15, 85);
                        }
                    }
                }
                Splitter::StashesHorizontal => {
                    if let (Some(stashes), Some(right)) = (areas.stashes, areas.bottom_right) {
                        let start_x = stashes.x;
                        let total_width = (right.x + right.width).saturating_sub(start_x);
                        if total_width > 0 {
                            let relative_x = pos.x.saturating_sub(start_x);
                            let pct = ((relative_x as f32 / total_width as f32) * 100.0) as u16;
                            app.stashes_horizontal_split_pct = pct.clamp(15, 85);
                        }
                    }
                }
                Splitter::StashesVertical => {
                    if let (Some(top), Some(bottom)) = (areas.stashes, areas.stashed_files) {
                        let start_y = top.y;
                        let total_height = (bottom.y + bottom.height).saturating_sub(start_y);
                        if total_height > 0 {
                            let relative_y = pos.y.saturating_sub(start_y);
                            let pct = ((relative_y as f32 / total_height as f32) * 100.0) as u16;
                            app.stashes_vertical_split_pct = pct.clamp(15, 85);
                        }
                    }
                }
                Splitter::OverviewHorizontal => {
                    if let Some(tab_bar) = areas.tab_bar {
                        let start_x = tab_bar.x;
                        let total_width = tab_bar.width;
                        if total_width > 0 {
                            let relative_x = pos.x.saturating_sub(start_x);
                            let pct = ((relative_x as f32 / total_width as f32) * 100.0) as u16;
                            app.overview_horizontal_split_pct = pct.clamp(15, 85);
                        }
                    }
                }
                Splitter::CommitPopupWidth => {
                    if let Some(parent) = areas.commit_popup_parent {
                        if parent.width > 0 {
                            let center_x = parent.x + parent.width / 2;
                            let half_width = (pos.x as i16 - center_x as i16).unsigned_abs();
                            let new_width = 2 * half_width;
                            let pct = ((new_width as f32 / parent.width as f32) * 100.0) as u16;
                            app.commit_popup_width_pct = pct.clamp(20, 98);
                        }
                    }
                }
                Splitter::CommitPopupHeight => {
                    if let Some(parent) = areas.commit_popup_parent {
                        if parent.height > 0 {
                            let center_y = parent.y + parent.height / 2;
                            let half_height = (pos.y as i16 - center_y as i16).unsigned_abs();
                            let new_height = 2 * half_height;
                            let pct = ((new_height as f32 / parent.height as f32) * 100.0) as u16;
                            app.commit_popup_height_pct = pct.clamp(20, 95);
                        }
                    }
                }
                Splitter::CommitPopupBoth => {
                    if let Some(parent) = areas.commit_popup_parent {
                        if parent.width > 0 && parent.height > 0 {
                            let center_x = parent.x + parent.width / 2;
                            let half_width = (pos.x as i16 - center_x as i16).unsigned_abs();
                            let new_width = 2 * half_width;
                            let pct_x = ((new_width as f32 / parent.width as f32) * 100.0) as u16;
                            app.commit_popup_width_pct = pct_x.clamp(20, 98);

                            let center_y = parent.y + parent.height / 2;
                            let half_height = (pos.y as i16 - center_y as i16).unsigned_abs();
                            let new_height = 2 * half_height;
                            let pct_y = ((new_height as f32 / parent.height as f32) * 100.0) as u16;
                            app.commit_popup_height_pct = pct_y.clamp(20, 95);
                        }
                    }
                }
            }
        }
        return;
    }

    if is_click {
        if app.mode == Mode::CommitInput {
            if let Some(rect) = areas.commit_popup {
                let on_left = pos.x == rect.x;
                let on_right = pos.x == rect.x + rect.width - 1;
                let on_top = pos.y == rect.y;
                let on_bottom = pos.y == rect.y + rect.height - 1;

                if (on_left || on_right) && (on_top || on_bottom) {
                    if app.commit_popup.maximized {
                        if let Some(parent) = areas.commit_popup_parent {
                            if parent.width > 0 && parent.height > 0 {
                                app.commit_popup_width_pct =
                                    ((rect.width as f32 / parent.width as f32) * 100.0) as u16;
                                app.commit_popup_height_pct =
                                    ((rect.height as f32 / parent.height as f32) * 100.0) as u16;
                            }
                        }
                        app.commit_popup.maximized = false;
                    }
                    app.active_drag_splitter = Some(Splitter::CommitPopupBoth);
                    return;
                } else if on_left || on_right {
                    if app.commit_popup.maximized {
                        if let Some(parent) = areas.commit_popup_parent {
                            if parent.width > 0 && parent.height > 0 {
                                app.commit_popup_width_pct =
                                    ((rect.width as f32 / parent.width as f32) * 100.0) as u16;
                                app.commit_popup_height_pct =
                                    ((rect.height as f32 / parent.height as f32) * 100.0) as u16;
                            }
                        }
                        app.commit_popup.maximized = false;
                    }
                    app.active_drag_splitter = Some(Splitter::CommitPopupWidth);
                    return;
                } else if on_top || on_bottom {
                    if app.commit_popup.maximized {
                        if let Some(parent) = areas.commit_popup_parent {
                            if parent.width > 0 && parent.height > 0 {
                                app.commit_popup_width_pct =
                                    ((rect.width as f32 / parent.width as f32) * 100.0) as u16;
                                app.commit_popup_height_pct =
                                    ((rect.height as f32 / parent.height as f32) * 100.0) as u16;
                            }
                        }
                        app.commit_popup.maximized = false;
                    }
                    app.active_drag_splitter = Some(Splitter::CommitPopupHeight);
                    return;
                }
            }
        }

        if let Some(rect) = areas.inspect_horizontal_splitter {
            if rect.contains(pos) {
                app.active_drag_splitter = Some(Splitter::InspectHorizontal);
                return;
            }
        }
        if let Some(rect) = areas.inspect_vertical_splitter {
            if rect.contains(pos) {
                app.active_drag_splitter = Some(Splitter::InspectVertical);
                return;
            }
        }
        if let Some(rect) = areas.workspace_main_splitter {
            if rect.contains(pos) {
                app.active_drag_splitter = Some(Splitter::WorkspaceMain);
                return;
            }
        }
        if let Some(rect) = areas.files_horizontal_splitter {
            if rect.contains(pos) {
                app.active_drag_splitter = Some(Splitter::FilesHorizontal);
                return;
            }
        }
        if let Some(rect) = areas.branches_horizontal_splitter {
            if rect.contains(pos) {
                app.active_drag_splitter = Some(Splitter::BranchesHorizontal);
                return;
            }
        }
        if let Some(rect) = areas.stashes_horizontal_splitter {
            if rect.contains(pos) {
                app.active_drag_splitter = Some(Splitter::StashesHorizontal);
                return;
            }
        }
        if let Some(rect) = areas.stashes_vertical_splitter {
            if rect.contains(pos) {
                app.active_drag_splitter = Some(Splitter::StashesVertical);
                return;
            }
        }
        if let Some(rect) = areas.overview_horizontal_splitter {
            if rect.contains(pos) {
                app.active_drag_splitter = Some(Splitter::OverviewHorizontal);
                return;
            }
        }
    }

    if is_drag || is_release {
        return;
    }

    if app.mode == Mode::Settings {
        return;
    }

    if app.mode == Mode::About {
        return;
    }

    if app.mode == Mode::Help || app.mode == Mode::DetailHelp {
        if is_scroll_up {
            app.help_scroll_up();
        } else if is_scroll_down {
            app.help_scroll_down();
        }
        return;
    }

    if app.mode == Mode::CommitInput {
        if is_scroll_up {
            app.commit_input_scroll_up();
        } else if is_scroll_down {
            app.commit_input_scroll_down();
        }
        return;
    }

    if app.mode == Mode::Normal {
        if is_click {
            for (i, rect) in app.main_areas.iter().enumerate() {
                if rect.contains(pos) {
                    let actual_index = i + app.scroll_top;
                    if actual_index < app.get_items_len() {
                        let now = std::time::Instant::now();
                        let is_double_click = if let Some((last_time, last_idx)) = app.last_click {
                            last_idx == actual_index
                                && now.duration_since(last_time).as_millis() < 400
                        } else {
                            false
                        };

                        if is_double_click {
                            app.selected_index = actual_index;
                            app.open_detail();
                            app.last_click = None;
                        } else {
                            app.selected_index = actual_index;
                            app.last_click = Some((now, actual_index));
                        }
                    }
                    return;
                }
            }
        } else {
            let visible_count = app.main_areas.len();
            if is_scroll_up {
                app.move_up();
            } else if is_scroll_down {
                app.move_down(visible_count);
            }
        }
        return;
    }

    // Only handle detail modes beyond this point.
    if !matches!(app.mode, Mode::Detail | Mode::DetailHelp | Mode::Inspect | Mode::Logs) {
        return;
    }

    let areas = app.detail_areas;

    // Handle tab switching if the user clicks on the tab bar.
    if app.mode == Mode::Detail {
        if let Some(rect) = areas.tab_bar {
            if rect.contains(pos) {
                if is_click {
                    let click_x = pos.x - rect.x;
                    let use_short = rect.width < 124;
                    let tabs_data = [
                        ("Details", "D", 0),
                        ("Files", "F", 1),
                        ("Graph", "G", 2),
                        ("Branches", "B", 3),
                        ("Tags", "T", 4),
                        ("Remotes", "R", 5),
                        ("Stashes", "S", 6),
                        ("Overview", "O", 7),
                    ];
                    let mut current_offset = 2;
                    for &(long_name, short_name, tab_index) in &tabs_data {
                        let name = if use_short { short_name } else { long_name };
                        let tab_width = name.len() + 8;
                        if click_x >= current_offset && click_x < current_offset + tab_width as u16
                        {
                            app.detail_tab = tab_index;
                            match tab_index {
                                0 => app.detail_focus = DetailSection::Commits,
                                1 => app.detail_focus = DetailSection::Files,
                                2 => {}
                                3 => app.detail_focus = DetailSection::LocalBranches,
                                4 => {
                                    app.detail_focus = DetailSection::LocalTags;
                                }
                                5 => {
                                    app.detail_focus = DetailSection::Remotes;
                                    let remote_name = if let Some(crate::repo::ItemDetail::Repo {
                                        info,
                                        ..
                                    }) = &app.current_detail
                                    {
                                        info.remotes
                                            .get(app.branch_list.remote_selection)
                                            .or_else(|| info.remotes.first())
                                            .map(|r| r.name.clone())
                                    } else {
                                        None
                                    };
                                    if let Some(name) = remote_name {
                                        app.fetch_remote(&name);
                                    }
                                }
                                6 => app.detail_focus = DetailSection::Stashes,
                                7 => app.detail_focus = DetailSection::Commits,
                                _ => {}
                            }
                            if app.config.resync_on_tab_change {
                                app.resync_detail();
                            }
                            break;
                        }
                        current_offset += tab_width as u16 + 1;
                    }
                }
                return;
            }
        }
    }

    // Graph view scroll (tab 3, index 2)
    if app.detail_tab == 2 {
        if let Some(rect) = areas.tab_bar {
            if pos.y >= rect.y + rect.height {
                if is_scroll_up {
                    app.graph_scroll_up();
                } else if is_scroll_down {
                    app.graph_scroll_down();
                }
                return;
            }
        }
    }

    // Staged sub-panel (inside Staging Area left block) — check before bottom_left
    // so the more-specific sub-panels win.
    if let Some(rect) = areas.staged_sub {
        if rect.contains(pos) {
            if is_click {
                app.detail_focus = DetailSection::Staged;
                app.last_staging_focus = DetailSection::Staged;

                let mut clicked_file = false;
                if let Some(inner) = areas.staged_sub_inner {
                    if inner.contains(pos) {
                        let clicked_row = (pos.y - inner.y) as usize;
                        let offset = if app.detail_focus == DetailSection::Staged {
                            app.status_list.staged_list_state.borrow().offset()
                        } else {
                            0
                        };
                        let actual_idx = offset + clicked_row;
                        let total = match &app.current_detail {
                            Some(crate::repo::ItemDetail::Repo { info, .. }) => {
                                info.changes.staged.len()
                            }
                            _ => 0,
                        };
                        if actual_idx < total {
                            app.status_list.staging_file_selection = actual_idx;
                            clicked_file = true;
                        }
                    }
                }

                if !clicked_file {
                    let total = match &app.current_detail {
                        Some(crate::repo::ItemDetail::Repo { info, .. }) => {
                            info.changes.staged.len()
                        }
                        _ => 0,
                    };
                    if total > 0 {
                        app.status_list.staging_file_selection =
                            app.status_list.staging_file_selection.min(total - 1);
                    } else {
                        app.status_list.staging_file_selection = 0;
                    }
                }
                app.diff.diff_scroll = 0;
                app.refresh_staging_diff();
            } else if is_scroll_up {
                app.detail_focus = DetailSection::Staged;
                app.last_staging_focus = DetailSection::Staged;
                if app.is_uncommitted_selected() {
                    app.staging_file_up();
                } else {
                    app.detail_file_up();
                }
            } else if is_scroll_down {
                app.detail_focus = DetailSection::Staged;
                app.last_staging_focus = DetailSection::Staged;
                if app.is_uncommitted_selected() {
                    app.staging_file_down();
                } else {
                    app.detail_file_down();
                }
            }
            return;
        }
    }
    // Unstaged sub-panel.
    if let Some(rect) = areas.unstaged_sub {
        if rect.contains(pos) {
            if is_click {
                app.detail_focus = DetailSection::Unstaged;
                app.last_staging_focus = DetailSection::Unstaged;

                let mut clicked_file = false;
                if let Some(inner) = areas.unstaged_sub_inner {
                    if inner.contains(pos) {
                        let clicked_row = (pos.y - inner.y) as usize;
                        let offset = if app.detail_focus == DetailSection::Unstaged {
                            app.status_list.unstaged_list_state.borrow().offset()
                        } else {
                            0
                        };
                        let actual_idx = offset + clicked_row;
                        let total = match &app.current_detail {
                            Some(crate::repo::ItemDetail::Repo { info, .. }) => {
                                info.changes.unstaged.len()
                            }
                            _ => 0,
                        };
                        if actual_idx < total {
                            app.status_list.staging_file_selection = actual_idx;
                            clicked_file = true;
                        }
                    }
                }

                if !clicked_file {
                    let total = match &app.current_detail {
                        Some(crate::repo::ItemDetail::Repo { info, .. }) => {
                            info.changes.unstaged.len()
                        }
                        _ => 0,
                    };
                    if total > 0 {
                        app.status_list.staging_file_selection =
                            app.status_list.staging_file_selection.min(total - 1);
                    } else {
                        app.status_list.staging_file_selection = 0;
                    }
                }
                app.diff.diff_scroll = 0;
                app.refresh_staging_diff();
            } else if is_scroll_up {
                app.detail_focus = DetailSection::Unstaged;
                app.last_staging_focus = DetailSection::Unstaged;
                if app.is_uncommitted_selected() {
                    app.staging_file_up();
                } else {
                    app.detail_file_up();
                }
            } else if is_scroll_down {
                app.detail_focus = DetailSection::Unstaged;
                app.last_staging_focus = DetailSection::Unstaged;
                if app.is_uncommitted_selected() {
                    app.staging_file_down();
                } else {
                    app.detail_file_down();
                }
            }
            return;
        }
    }
    // Conflicts sub-panel.
    if let Some(rect) = areas.conflicts_sub {
        if rect.contains(pos) {
            if is_click {
                app.detail_focus = DetailSection::Conflicts;
                app.last_staging_focus = DetailSection::Conflicts;

                let mut clicked_file = false;
                if let Some(inner) = areas.conflicts_sub_inner {
                    if inner.contains(pos) {
                        let clicked_row = (pos.y - inner.y) as usize;
                        let offset = if app.detail_focus == DetailSection::Conflicts {
                            app.status_list.conflicts_list_state.borrow().offset()
                        } else {
                            0
                        };
                        let actual_idx = offset + clicked_row;
                        let total = match &app.current_detail {
                            Some(crate::repo::ItemDetail::Repo { info, .. }) => {
                                info.changes.conflicted.len()
                            }
                            _ => 0,
                        };
                        if actual_idx < total {
                            app.status_list.conflict_file_selection = actual_idx;
                            clicked_file = true;
                        }
                    }
                }

                if !clicked_file {
                    let total = match &app.current_detail {
                        Some(crate::repo::ItemDetail::Repo { info, .. }) => {
                            info.changes.conflicted.len()
                        }
                        _ => 0,
                    };
                    if total > 0 {
                        app.status_list.conflict_file_selection =
                            app.status_list.conflict_file_selection.min(total - 1);
                    } else {
                        app.status_list.conflict_file_selection = 0;
                    }
                }
                app.diff.diff_scroll = 0;
                app.refresh_staging_diff();
            } else if is_scroll_up {
                app.detail_focus = DetailSection::Conflicts;
                app.last_staging_focus = DetailSection::Conflicts;
                let total = match &app.current_detail {
                    Some(crate::repo::ItemDetail::Repo { info, .. }) => {
                        info.changes.conflicted.len()
                    }
                    _ => 0,
                };
                if total > 0 {
                    app.status_list.conflict_file_selection =
                        app.status_list.conflict_file_selection.saturating_sub(1);
                    app.refresh_staging_diff();
                }
            } else if is_scroll_down {
                app.detail_focus = DetailSection::Conflicts;
                app.last_staging_focus = DetailSection::Conflicts;
                let total = match &app.current_detail {
                    Some(crate::repo::ItemDetail::Repo { info, .. }) => {
                        info.changes.conflicted.len()
                    }
                    _ => 0,
                };
                if total > 0 {
                    app.status_list.conflict_file_selection =
                        (app.status_list.conflict_file_selection + 1).min(total - 1);
                    app.refresh_staging_diff();
                }
            }
            return;
        }
    }
    // Commit details sub-panel (inside Changed Files / Commit Details left block).
    if let Some(rect) = areas.commit_details {
        if rect.contains(pos) {
            if is_click {
                if app.detail_focus != DetailSection::CommitDetails {
                    app.detail_focus = DetailSection::CommitDetails;
                }
            } else if is_scroll_up {
                app.detail_focus = DetailSection::CommitDetails;
                app.commit_list.details_scroll_up();
            } else if is_scroll_down {
                app.detail_focus = DetailSection::CommitDetails;
                app.commit_list.details_scroll_down();
            }
            return;
        }
    }

    // Bottom-left panel (Staging Area outer block or Changed Files).
    if let Some(rect) = areas.bottom_left {
        if rect.contains(pos) {
            if is_click {
                app.detail_focus = DetailSection::Staged;
                if app.is_uncommitted_selected() {
                    let total = app.staging_file_total();
                    if total > 0 {
                        app.status_list.staging_file_selection =
                            app.status_list.staging_file_selection.min(total - 1);
                    } else {
                        app.status_list.staging_file_selection = 0;
                    }
                    app.diff.diff_scroll = 0;
                    app.refresh_staging_diff();
                } else {
                    let mut clicked_file = false;
                    if let Some(inner) = areas.changed_files_inner {
                        if inner.contains(pos) {
                            let clicked_row = (pos.y - inner.y) as usize;
                            let offset = app.status_list.changed_files_list_state.borrow().offset();
                            let actual_idx = offset + clicked_row;
                            let total = app.file_total();
                            if actual_idx < total {
                                app.status_list.file_selection = actual_idx;
                                clicked_file = true;
                            }
                        }
                    }
                    if !clicked_file {
                        let total = app.file_total();
                        if total > 0 {
                            app.status_list.file_selection =
                                app.status_list.file_selection.min(total - 1);
                        } else {
                            app.status_list.file_selection = 0;
                        }
                    }
                    app.diff.diff_scroll = 0;
                    app.refresh_file_diff();
                }
            } else if is_scroll_up {
                app.detail_focus = DetailSection::Staged;
                if app.is_uncommitted_selected() {
                    app.staging_file_up();
                } else {
                    app.detail_file_up();
                }
            } else if is_scroll_down {
                app.detail_focus = DetailSection::Staged;
                if app.is_uncommitted_selected() {
                    app.staging_file_down();
                } else {
                    app.detail_file_down();
                }
            }
            return;
        }
    }
    // Right panel (Diff / Staging Details / Conflict Diff).
    if let Some(rect) = areas.bottom_right {
        if rect.contains(pos) {
            if is_click {
                if app.detail_focus == DetailSection::Conflicts
                    || app.detail_focus == DetailSection::ConflictDiff
                    || app.last_staging_focus == DetailSection::Conflicts
                {
                    if app.detail_focus == DetailSection::Conflicts {
                        app.last_staging_focus = DetailSection::Conflicts;
                    }
                    app.detail_focus = DetailSection::ConflictDiff;
                } else if app.detail_focus != DetailSection::StagingDetails {
                    if app.detail_focus == DetailSection::Staged
                        || app.detail_focus == DetailSection::Unstaged
                    {
                        app.last_staging_focus = app.detail_focus;
                    }
                    app.detail_focus = DetailSection::StagingDetails;
                    app.diff.diff_scroll = 0;
                }
            } else if is_scroll_up {
                if app.detail_focus == DetailSection::Conflicts
                    || app.detail_focus == DetailSection::ConflictDiff
                    || app.last_staging_focus == DetailSection::Conflicts
                {
                    app.detail_focus = DetailSection::ConflictDiff;
                    app.diff.diff_scroll_up();
                } else {
                    app.detail_focus = DetailSection::StagingDetails;
                    app.diff.diff_scroll_up();
                }
            } else if is_scroll_down {
                if app.detail_focus == DetailSection::Conflicts
                    || app.detail_focus == DetailSection::ConflictDiff
                    || app.last_staging_focus == DetailSection::Conflicts
                {
                    app.detail_focus = DetailSection::ConflictDiff;
                    app.diff.diff_scroll_down();
                } else {
                    app.detail_focus = DetailSection::StagingDetails;
                    app.diff.diff_scroll_down();
                }
            }
            return;
        }
    }
    // Commits panel (top).
    if let Some(rect) = areas.commits {
        if rect.contains(pos) {
            if is_click {
                app.detail_focus = DetailSection::Commits;
                if let Some(inner) = areas.commits_inner {
                    if pos.y > inner.y {
                        let row_clicked = (pos.y - inner.y - 1) as usize;
                        let offset = app.commit_list.table_state.borrow().offset();
                        let actual_idx = offset + row_clicked;
                        let total = app.commit_total();
                        if actual_idx < total {
                            app.commit_list.selection = actual_idx;
                            app.status_list.file_selection = 0;
                            app.status_list.staging_file_selection = 0;
                            app.diff.diff_scroll = 0;
                            app.refresh_file_diff();
                        }
                    }
                }
            } else if is_scroll_up {
                app.detail_focus = DetailSection::Commits;
                app.detail_commit_up();
            } else if is_scroll_down {
                app.detail_focus = DetailSection::Commits;
                app.detail_commit_down();
            }
        }
    }
    // Local branches panel (inside Branches view).
    if let Some(rect) = areas.local_branches {
        if rect.contains(pos) {
            if is_click {
                app.detail_focus = DetailSection::LocalBranches;
                if let Some(inner) = areas.local_branches_inner {
                    if inner.contains(pos) {
                        let clicked_row = (pos.y - inner.y) as usize;
                        let offset = app.branch_list.local_branch_list_state.borrow().offset();
                        let actual_idx = offset + clicked_row;
                        let total = match &app.current_detail {
                            Some(crate::repo::ItemDetail::Repo { info, .. }) => {
                                info.local_branches.len()
                            }
                            _ => 0,
                        };
                        if actual_idx < total {
                            app.branch_list.local_branch_selection = actual_idx;
                        }
                    }
                }
            } else if is_scroll_up {
                app.detail_focus = DetailSection::LocalBranches;
                app.local_branch_up();
            } else if is_scroll_down {
                app.detail_focus = DetailSection::LocalBranches;
                app.local_branch_down();
            }
        }
    }
    // Remote branches panel (inside Branches view).
    if let Some(rect) = areas.remote_branches {
        if rect.contains(pos) {
            if is_click {
                app.detail_focus = DetailSection::RemoteBranches;
                if let Some(inner) = areas.remote_branches_inner {
                    if inner.contains(pos) {
                        let clicked_row = (pos.y - inner.y) as usize;
                        let offset = app.branch_list.remote_branch_list_state.borrow().offset();
                        let actual_idx = offset + clicked_row;
                        let total = match &app.current_detail {
                            Some(crate::repo::ItemDetail::Repo { info, .. }) => {
                                info.remote_branches.len()
                            }
                            _ => 0,
                        };
                        if actual_idx < total {
                            app.branch_list.remote_branch_selection = actual_idx;
                        }
                    }
                }
            } else if is_scroll_up {
                app.detail_focus = DetailSection::RemoteBranches;
                app.remote_branch_up();
            } else if is_scroll_down {
                app.detail_focus = DetailSection::RemoteBranches;
                app.remote_branch_down();
            }
        }
    }
    // Local tags panel (inside Tags view).
    if let Some(rect) = areas.local_tags {
        if rect.contains(pos) {
            if is_click {
                app.detail_focus = DetailSection::LocalTags;
                if let Some(inner) = areas.local_tags_inner {
                    if inner.contains(pos) {
                        let clicked_row = (pos.y - inner.y) as usize;
                        let offset = app.tag_list.local_tag_list_state.borrow().offset();
                        let actual_idx = offset + clicked_row;
                        let total = match &app.current_detail {
                            Some(crate::repo::ItemDetail::Repo { info, .. }) => {
                                info.local_tags.len()
                            }
                            _ => 0,
                        };
                        if actual_idx < total {
                            app.tag_list.local_tag_selection = actual_idx;
                        }
                    }
                }
            } else if is_scroll_up {
                app.detail_focus = DetailSection::LocalTags;
                app.local_tag_up();
            } else if is_scroll_down {
                app.detail_focus = DetailSection::LocalTags;
                app.local_tag_down();
            }
        }
    }
    // Files list panel (inside Files view).
    if let Some(rect) = areas.files {
        if rect.contains(pos) {
            if is_click {
                if app.detail_focus != DetailSection::Files {
                    app.detail_focus = DetailSection::Files;
                }
            } else if is_scroll_up {
                app.detail_focus = DetailSection::Files;
                app.file_list_up();
            } else if is_scroll_down {
                app.detail_focus = DetailSection::Files;
                app.file_list_down();
            }
        }
    }
    // File content preview panel (inside Files view).
    if let Some(rect) = areas.file_content {
        if rect.contains(pos) {
            if is_click {
                if app.detail_focus != DetailSection::FileContent {
                    app.detail_focus = DetailSection::FileContent;
                }
            } else if is_scroll_up {
                app.detail_focus = DetailSection::FileContent;
                app.file_tree.file_content_scroll_up();
            } else if is_scroll_down {
                app.detail_focus = DetailSection::FileContent;
                app.file_tree.file_content_scroll_down();
            }
        }
    }
    // Remotes list panel.
    if let Some(rect) = areas.remotes {
        if rect.contains(pos) {
            if is_click {
                app.detail_focus = DetailSection::Remotes;
                if let Some(inner) = areas.remotes_inner {
                    if inner.contains(pos) {
                        let clicked_row = (pos.y - inner.y) as usize;
                        let offset = app.branch_list.remote_list_state.borrow().offset();
                        let actual_idx = offset + clicked_row;
                        let total = match &app.current_detail {
                            Some(crate::repo::ItemDetail::Repo { info, .. }) => info.remotes.len(),
                            _ => 0,
                        };
                        if actual_idx < total {
                            app.branch_list.remote_selection = actual_idx;
                        }
                    }
                }
            } else if is_scroll_up {
                app.detail_focus = DetailSection::Remotes;
                app.remote_up();
            } else if is_scroll_down {
                app.detail_focus = DetailSection::Remotes;
                app.remote_down();
            }
        }
    }
    // Stashes list panel.
    if let Some(rect) = areas.stashes {
        if rect.contains(pos) {
            if is_click {
                app.detail_focus = DetailSection::Stashes;
                if let Some(inner) = areas.stashes_inner {
                    if inner.contains(pos) {
                        let clicked_row = (pos.y - inner.y) as usize;
                        let offset = app.stash_list.stash_list_state.borrow().offset();
                        let actual_idx = offset + clicked_row;
                        let total = match &app.current_detail {
                            Some(crate::repo::ItemDetail::Repo { info, .. }) => info.stashes.len(),
                            _ => 0,
                        };
                        if actual_idx < total {
                            app.stash_list.stash_selection = actual_idx;
                            app.stash_list.stash_file_selection = 0;
                            app.diff.diff_scroll = 0;
                            app.refresh_file_diff();
                        }
                    }
                }
            } else if is_scroll_up {
                app.detail_focus = DetailSection::Stashes;
                app.stash_up();
            } else if is_scroll_down {
                app.detail_focus = DetailSection::Stashes;
                app.stash_down();
            }
        }
    }
    // Stashed files panel.
    if let Some(rect) = areas.stashed_files {
        if rect.contains(pos) {
            if is_click {
                app.detail_focus = DetailSection::StashedFiles;
                if let Some(inner) = areas.stashed_files_inner {
                    if inner.contains(pos) {
                        let clicked_row = (pos.y - inner.y) as usize;
                        let offset = app.stash_list.stash_file_list_state.borrow().offset();
                        let actual_idx = offset + clicked_row;
                        let total = match &app.current_detail {
                            Some(crate::repo::ItemDetail::Repo { info, .. }) => info
                                .stashes
                                .get(app.stash_list.stash_selection)
                                .map(|s| s.files.len())
                                .unwrap_or(0),
                            _ => 0,
                        };
                        if actual_idx < total {
                            app.stash_list.stash_file_selection = actual_idx;
                            app.diff.diff_scroll = 0;
                            app.refresh_file_diff();
                        }
                    }
                }
            } else if is_scroll_up {
                app.detail_focus = DetailSection::StashedFiles;
                app.stash_file_up();
            } else if is_scroll_down {
                app.detail_focus = DetailSection::StashedFiles;
                app.stash_file_down();
            }
        }
    }
}
