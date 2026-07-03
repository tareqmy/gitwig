use crate::app::{App, DetailSection, GlobalFilter, Mode, Splitter};
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

    if is_click && pos.y == 0 {
        if let Some(ref latest) = app.update_available {
            let (mut width, _) = crossterm::terminal::size().unwrap_or((80, 24));
            if width == 0 {
                width = 80;
            }
            let len_version = format!(" v{} ", env!("CARGO_PKG_VERSION")).chars().count();
            let len_badge = if app.can_self_update() {
                format!("[Update to v{}]", latest).chars().count()
            } else {
                format!("[New version v{}]", latest).chars().count()
            };
            let len_total = len_version + len_badge + 1;

            let start_x = (width as usize).saturating_sub(len_total + 2);
            let end_x = (width as usize).saturating_sub(len_version + 2);

            if (pos.x as usize) >= start_x && (pos.x as usize) <= end_x {
                app.trigger_self_update();
                return;
            }
        }
    }

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

    if app.mode == Mode::Legend {
        if is_scroll_up {
            app.legend_scroll_up();
        } else if is_scroll_down {
            app.legend_scroll_down();
        }
        return;
    }

    if app.mode == Mode::Overview {
        if is_click {
            if let Some(rect) = areas.overview {
                if rect.contains(pos) {
                    app.overview_focus = crate::app::OverviewFocus::Overview;
                    return;
                }
            }
            if let Some(rect) = areas.stats {
                if rect.contains(pos) {
                    app.overview_focus = crate::app::OverviewFocus::Stats;
                    return;
                }
            }
        } else if is_scroll_up {
            if let Some(rect) = areas.overview {
                if rect.contains(pos) {
                    app.overview_scroll = app.overview_scroll.saturating_sub(1);
                    return;
                }
            }
            if let Some(rect) = areas.stats {
                if rect.contains(pos) {
                    app.stats_scroll = app.stats_scroll.saturating_sub(1);
                    return;
                }
            }
        } else if is_scroll_down {
            if let Some(rect) = areas.overview {
                if rect.contains(pos) {
                    app.overview_scroll = app.overview_scroll.saturating_add(1);
                    return;
                }
            }
            if let Some(rect) = areas.stats {
                if rect.contains(pos) {
                    app.stats_scroll = app.stats_scroll.saturating_add(1);
                    return;
                }
            }
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
            if let Some(summary_rect) = app.global_summary_area {
                if summary_rect.contains(pos) {
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
                            crate::repo::ItemStatus::GitRepo(Some(summary)) => {
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
                            crate::repo::ItemStatus::GitRepo(None) => {
                                total_repos += 1;
                            }
                            _ => {}
                        }
                    }

                    let repos_width = format!(" {} ", total_repos).len() + 5;
                    let dirty_width = format!(" {} ", dirty_count).len() + 5;
                    let ahead_width = format!(" {} ", ahead_count).len() + 5;
                    let stale_width = format!(" {} ", stale_count).len() + 5;
                    let spacer_width = 5;

                    let total_width = repos_width
                        + spacer_width
                        + dirty_width
                        + spacer_width
                        + ahead_width
                        + spacer_width
                        + stale_width;

                    let start_x = summary_rect.x
                        + (summary_rect.width.saturating_sub(total_width as u16) / 2);
                    if pos.x >= start_x && pos.x < start_x + total_width as u16 {
                        let click_offset = (pos.x - start_x) as usize;
                        if click_offset < repos_width {
                            app.global_filter = None;
                            app.selected_index = 0;
                            app.scroll_top = 0;
                            return;
                        } else if click_offset < repos_width + spacer_width {
                            // Spacer click
                        } else if click_offset < repos_width + spacer_width + dirty_width {
                            if app.global_filter == Some(GlobalFilter::Dirty) {
                                app.global_filter = None;
                            } else {
                                app.global_filter = Some(GlobalFilter::Dirty);
                            }
                            app.selected_index = 0;
                            app.scroll_top = 0;
                            return;
                        } else if click_offset
                            < repos_width + spacer_width + dirty_width + spacer_width
                        {
                            // Spacer click
                        } else if click_offset
                            < repos_width + spacer_width + dirty_width + spacer_width + ahead_width
                        {
                            if app.global_filter == Some(GlobalFilter::Ahead) {
                                app.global_filter = None;
                            } else {
                                app.global_filter = Some(GlobalFilter::Ahead);
                            }
                            app.selected_index = 0;
                            app.scroll_top = 0;
                            return;
                        } else if click_offset
                            < repos_width
                                + spacer_width
                                + dirty_width
                                + spacer_width
                                + ahead_width
                                + spacer_width
                        {
                            // Spacer click
                        } else {
                            if app.global_filter == Some(GlobalFilter::Stale) {
                                app.global_filter = None;
                            } else {
                                app.global_filter = Some(GlobalFilter::Stale);
                            }
                            app.selected_index = 0;
                            app.scroll_top = 0;
                            return;
                        }
                    }
                }
            }

            for (i, rect) in app.main_areas.iter().enumerate() {
                if rect.contains(pos) {
                    let actual_index = i + app.scroll_top;
                    let rows = app.get_home_rows();
                    if let Some(row) = rows.get(actual_index) {
                        let now = std::time::Instant::now();
                        let is_double_click = if let Some((last_time, last_idx)) = app.last_click {
                            last_idx == actual_index
                                && now.duration_since(last_time).as_millis() < 400
                        } else {
                            false
                        };

                        match row {
                            crate::app::HomeRow::Repo {
                                actual_index: original_index,
                                path: item,
                                ..
                            } => {
                                let original_index = *original_index;
                                let rect_y =
                                    if app.config.compact_view { rect.y } else { rect.y + 1 };

                                if pos.y == rect_y {
                                    let mut current_x =
                                        if app.config.compact_view { rect.x } else { rect.x + 2 };

                                    let mark = if actual_index == app.selected_index {
                                        app.sym("selection_mark")
                                    } else {
                                        "  "
                                    };
                                    current_x += mark.chars().count() as u16;

                                    let fallback = crate::repo::ItemStatus::Missing;
                                    let status =
                                        app.statuses.get(original_index).unwrap_or(&fallback);
                                    let is_git =
                                        matches!(status, crate::repo::ItemStatus::GitRepo(_));
                                    if is_git {
                                        current_x += app.sym("git_repo").chars().count() as u16;
                                    }

                                    let repo_name = std::path::Path::new(item.as_str())
                                        .file_name()
                                        .and_then(|s| s.to_str())
                                        .unwrap_or(item.as_str());
                                    current_x += repo_name.chars().count() as u16;

                                    if let crate::repo::ItemStatus::GitRepo(Some(summary)) = status
                                    {
                                        let state_str = match summary.state {
                                            crate::repo::RepoState::Merge => " ⚠ MERGE_HEAD",
                                            crate::repo::RepoState::Rebase => " 🚧 REBASING",
                                            crate::repo::RepoState::CherryPick => " ⚡ CHERRY-PICK",
                                            crate::repo::RepoState::Revert => " ⚡ REVERTING",
                                            crate::repo::RepoState::Bisect => " 🔍 BISECTING",
                                            crate::repo::RepoState::ApplyMailbox => " 📬 APPLYING",
                                            crate::repo::RepoState::Clean => " ✓ CLEAN",
                                        };
                                        current_x += state_str.chars().count() as u16;

                                        if summary.staged > 0
                                            && (summary.modified > 0 || summary.untracked > 0)
                                        {
                                            current_x += " ⚠ PARTIAL".chars().count() as u16;
                                        }
                                    }

                                    if let Some(lbls) = app.config.labels.get(item.as_str()) {
                                        for lbl in lbls {
                                            current_x += 1;

                                            let label_width = lbl.chars().count() as u16 + 2;
                                            let label_start = current_x;
                                            let label_end = current_x + label_width;

                                            if pos.x >= label_start && pos.x < label_end {
                                                crate::debug_log::info(format!(
                                                    "Clicked label: {}",
                                                    lbl
                                                ));
                                                app.repo_search_query = Some(lbl.clone());
                                                app.selected_index = 0;
                                                app.scroll_top = 0;
                                                return;
                                            }
                                            current_x = label_end;
                                        }
                                    }
                                }

                                if is_double_click {
                                    app.selected_index = actual_index;
                                    app.open_detail();
                                    app.last_click = None;
                                } else {
                                    app.selected_index = actual_index;
                                    app.last_click = Some((now, actual_index));
                                }
                            }
                            crate::app::HomeRow::GroupHeader { name, collapsed, .. } => {
                                let collapsed = *collapsed;
                                if is_double_click {
                                    let name = name.clone();
                                    if collapsed {
                                        app.collapsed_groups.remove(&name);
                                    } else {
                                        app.collapsed_groups.insert(name);
                                    }
                                    app.clamp_selection();
                                    app.last_click = None;
                                } else {
                                    app.selected_index = actual_index;
                                    app.last_click = Some((now, actual_index));
                                }
                            }
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
    if !matches!(
        app.mode,
        Mode::Detail | Mode::DetailHelp | Mode::Inspect | Mode::Logs | Mode::Overview
    ) {
        return;
    }

    let areas = app.detail_areas;

    // Handle tab switching if the user clicks on the tab bar.
    if app.mode == Mode::Detail {
        if let Some(rect) = areas.tab_bar {
            if rect.contains(pos) {
                if is_click {
                    let click_x = pos.x - rect.x;
                    let tabs_data = [
                        ("Workspace", "WS", "W", 0),
                        ("Files", "Fi", "F", 1),
                        ("Graph", "Gr", "G", 2),
                        ("Branches", "Br", "B", 3),
                        ("Tags", "Tg", "T", 4),
                        ("Remotes", "Rm", "R", 5),
                        ("Stashes", "St", "S", 6),
                        ("Worktrees", "WT", "W", 7),
                        ("Submodules", "SM", "S", 8),
                        ("Reflog", "Rf", "R", 9),
                    ];
                    let width_long: usize =
                        11 + tabs_data.iter().map(|t| t.0.len() + 8).sum::<usize>();
                    let width_medium: usize =
                        11 + tabs_data.iter().map(|t| t.1.len() + 8).sum::<usize>();
                    let name_format = if rect.width as usize >= width_long {
                        0
                    } else if rect.width as usize >= width_medium {
                        1
                    } else {
                        2
                    };
                    let mut current_offset = 2;
                    for &(long_name, medium_name, short_name, tab_index) in &tabs_data {
                        let name = match name_format {
                            0 => long_name,
                            1 => medium_name,
                            _ => short_name,
                        };
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
                                }
                                6 => app.detail_focus = DetailSection::Stashes,
                                7 => app.detail_focus = DetailSection::Worktrees,
                                8 => app.detail_focus = DetailSection::Submodules,
                                9 => app.detail_focus = DetailSection::Reflog,
                                _ => {}
                            }
                            if app.get_current_resync_on_tab_change() {
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

    // Graph view scroll & click (tab 3, index 2)
    if app.detail_tab == 2 {
        if let Some(rect) = areas.tab_bar {
            if pos.y >= rect.y + rect.height {
                if is_scroll_up {
                    app.graph_select_up();
                    return;
                } else if is_scroll_down {
                    app.graph_select_down();
                    return;
                }
            }
        }
        if let Some(rect) = areas.graph {
            if rect.contains(pos) {
                if is_click {
                    if let Some(inner) = areas.graph_inner {
                        if inner.contains(pos) {
                            let clicked_row = (pos.y - inner.y) as usize;
                            let actual_idx = app.graph_scroll + clicked_row;
                            let total = match &app.current_detail {
                                Some(crate::repo::ItemDetail::Repo { info, .. }) => {
                                    info.graph_lines.len()
                                }
                                _ => 0,
                            };
                            if actual_idx < total {
                                app.graph_selection = actual_idx;
                            }
                        }
                    }
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
                app.detail_focus = DetailSection::Files;
                if let Some(inner) = areas.files_inner {
                    if inner.contains(pos) {
                        let clicked_row = (pos.y - inner.y) as usize;
                        let offset = app.file_tree.file_list_state.borrow().offset();
                        let actual_idx = offset + clicked_row;
                        let total = app.file_tree.visible_files.len();
                        if actual_idx < total {
                            app.file_tree.file_list_selection = actual_idx;
                            app.file_tree.file_content_scroll = 0;
                        }
                    }
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
    // Worktrees list panel.
    if let Some(rect) = areas.worktrees {
        if rect.contains(pos) {
            if is_click {
                app.detail_focus = DetailSection::Worktrees;
                if let Some(inner) = areas.worktrees_inner {
                    if inner.contains(pos) {
                        // Offset by 2 lines to account for table header (1 line) and bottom margin (1 line)
                        let header_height = 2;
                        if pos.y >= inner.y + header_height {
                            let clicked_row = (pos.y - (inner.y + header_height)) as usize;
                            let actual_idx = clicked_row;
                            let total = match &app.current_detail {
                                Some(crate::repo::ItemDetail::Repo { info, .. }) => {
                                    if let crate::repo::TabData::Loaded(wts) = &info.worktrees {
                                        wts.len()
                                    } else {
                                        0
                                    }
                                }
                                _ => 0,
                            };
                            if actual_idx < total {
                                app.worktree_selection = actual_idx;
                            }
                        }
                    }
                }
            } else if is_scroll_up {
                app.detail_focus = DetailSection::Worktrees;
                app.worktree_selection = app.worktree_selection.saturating_sub(1);
            } else if is_scroll_down {
                app.detail_focus = DetailSection::Worktrees;
                if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
                    if let crate::repo::TabData::Loaded(wts) = &info.worktrees {
                        let wts_count = wts.len();
                        app.worktree_selection =
                            (app.worktree_selection + 1).min(wts_count.saturating_sub(1));
                    }
                }
            }
        }
    }
}
