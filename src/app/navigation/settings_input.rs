//! Settings editing, text-input caret primitives, bulk add/import, and TOML persistence.

use super::super::*;

impl App {
    pub fn toggle_or_edit_setting(&mut self) {
        match self.settings_selected_index {
            0 => {
                self.settings_editing = true;
                self.set_input_buffer(self.config.poll_interval_ms.to_string());
            }
            1 => {
                self.config.sort_by = self.config.sort_by.next();
                if self.effective_sort_by() != SortOrder::Custom {
                    self.sort_items_in_place();
                }
                self.persist("Sort mode updated");
            }
            2 => {
                self.config.sort_reverse = !self.config.sort_reverse;
                if self.effective_sort_by() != SortOrder::Custom {
                    self.sort_items_in_place();
                }
                self.persist("Sort direction updated");
            }
            3 => {
                self.settings_theme_list = self.get_available_themes();
                self.settings_theme_index = self
                    .settings_theme_list
                    .iter()
                    .position(|t| t == &self.config.theme_name)
                    .unwrap_or(0);
                self.settings_editing = true;
            }
            4 => {
                self.settings_editing = true;
                self.set_input_buffer(self.config.scan.max_depth.to_string());
            }
            5 => {
                self.settings_editing = true;
                self.set_input_buffer(self.config.scan.start_dir.clone());
            }
            6 => {
                self.settings_editing = true;
                self.set_input_buffer(self.config.max_commits.to_string());
            }
            7 => {
                self.settings_editing = true;
                self.set_input_buffer(self.config.page_size.to_string());
            }
            8 => {
                self.settings_editing = true;
                self.set_input_buffer(self.config.scan.excludes.join(","));
            }
            9 => {
                self.settings_editing = true;
                self.set_input_buffer(self.config.git_app.clone());
            }
            10 => {
                // Scan Git Only is now always enabled; this setting has been removed.
            }
            12 => {
                self.config.compatibility_mode = !self.config.compatibility_mode;
                self.persist("Compatibility Mode updated");
            }
            13 => {
                self.config.resync_on_tab_change = !self.config.resync_on_tab_change;
                self.persist("Resync on Tab Change updated");
            }
            58 => {
                self.config.show_grouping = !self.config.show_grouping;
                self.persist("Show Grouping updated");
            }
            55 => {
                self.config.ssh_strict_host_checking = !self.config.ssh_strict_host_checking;
                gitwig_core::set_ssh_strict_host_checking(self.config.ssh_strict_host_checking);
                self.persist("SSH Strict Host Checking updated");
            }
            56 => {
                self.settings_editing = true;
                self.set_input_buffer(self.config.editor.clone());
            }
            60 => {
                self.settings_editing = true;
                self.set_input_buffer(self.config.auto_fetch_interval_mins.to_string());
            }
            84 => {
                self.settings_editing = true;
                self.set_input_buffer(self.config.fetch_timeout_secs.to_string());
            }
            61 => {
                self.settings_editing = true;
                self.set_input_buffer(self.config.watch_dirs.join(","));
            }
            62 => {
                self.config.show_system_stats = !self.config.show_system_stats;
                self.persist("Show CPU/MEM updated");
            }
            63 => {
                self.config.enable_commit_signatures = !self.config.enable_commit_signatures;
                self.persist("Commit signatures updated");
            }
            64 => {
                self.settings_editing = true;
                self.set_input_buffer(self.config.graph_max_commits.to_string());
            }
            65 => {
                self.settings_editing = true;
                self.set_input_buffer(self.config.detail_cache_ttl_secs.to_string());
            }
            66 => {
                self.settings_editing = true;
                self.set_input_buffer(self.config.tab_ttl_secs.to_string());
            }
            67 => {
                self.config.view_mode = self.config.view_mode.next();
                self.persist("View mode cycled");
            }
            82 => {
                self.settings_editing = true;
                self.set_input_buffer(self.config.tile_columns.to_string());
            }
            80 => {
                self.settings_editing = true;
                self.set_input_buffer(self.config.stale_threshold_months.to_string());
            }
            81 => {
                self.config.show_stale_projects = !self.config.show_stale_projects;
                self.persist("Show Stale Projects updated");
            }
            89 => {
                self.config.show_separators = !self.config.show_separators;
                self.persist("Show Separators updated");
            }
            11 => {
                self.config.prompt_cwd_repo = !self.config.prompt_cwd_repo;
                self.persist("Prompt To Track Launch Directory updated");
            }
            83 => {
                self.config.enable_watch_dirs = !self.config.enable_watch_dirs;
                self.persist("Enable Watch Directories updated");
                self.setup_watcher();
            }
            idx if idx >= 14 => {
                if let Some(action) = crate::keybindings::Action::from_index(idx) {
                    self.settings_editing = true;
                    self.set_input_buffer(self.keybindings.get_action_keys(action).join(", "));
                }
            }
            _ => {}
        }
    }

    pub fn commit_settings_edit(&mut self) {
        let trimmed = self.input_buffer.trim();
        match self.settings_selected_index {
            0 => {
                if let Ok(val) = trimmed.parse::<u64>() {
                    if val >= 10 {
                        self.config.poll_interval_ms = val;
                        self.persist("Poll interval updated");
                        self.settings_editing = false;
                        self.input_buffer.clear();
                    } else {
                        self.status_message =
                            Some("Poll interval must be at least 10ms".to_string());
                    }
                } else {
                    self.status_message = Some("Invalid integer".to_string());
                }
            }
            3 => {
                if self.settings_theme_index < self.settings_theme_list.len() {
                    let selected_theme =
                        self.settings_theme_list[self.settings_theme_index].clone();
                    self.config.theme_name = selected_theme.clone();
                    let themes_dir =
                        self.config_path.parent().unwrap_or(&self.config_path).join("themes");
                    let theme_path = themes_dir.join(format!("{}.theme", selected_theme));
                    if theme_path.exists() {
                        if let Ok(theme_contents) = std::fs::read_to_string(&theme_path) {
                            if let Ok(theme) =
                                toml::from_str::<crate::config::ThemeConfig>(&theme_contents)
                            {
                                self.config.theme = theme;
                                crate::ui::update_theme(&self.config.theme);
                                self.persist("Theme updated");
                                self.settings_editing = false;
                                return;
                            }
                        }
                    }
                    crate::ui::update_theme(&self.config.theme);
                    self.settings_editing = false;
                    self.persist("Theme updated");
                }
            }
            4 => {
                if let Ok(val) = trimmed.parse::<usize>() {
                    self.config.scan.max_depth = val;
                    self.persist("Scan max depth updated");
                    self.settings_editing = false;
                    self.input_buffer.clear();
                } else {
                    self.status_message = Some("Invalid integer".to_string());
                }
            }
            5 => {
                self.config.scan.start_dir = trimmed.to_string();
                self.persist("Scan start directory updated");
                self.settings_editing = false;
                self.input_buffer.clear();
            }
            6 => {
                if let Ok(val) = trimmed.parse::<usize>() {
                    self.config.max_commits = val;
                    self.persist("Max commits updated");
                    self.settings_editing = false;
                    self.input_buffer.clear();
                } else {
                    self.status_message = Some("Invalid integer".to_string());
                }
            }
            7 => {
                if let Ok(val) = trimmed.parse::<usize>() {
                    if val >= 1 {
                        self.config.page_size = val;
                        self.persist("Page size updated");
                        self.settings_editing = false;
                        self.input_buffer.clear();
                    } else {
                        self.status_message = Some("Page size must be at least 1".to_string());
                    }
                } else {
                    self.status_message = Some("Invalid integer".to_string());
                }
            }
            8 => {
                let mut excludes: Vec<String> = trimmed
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if excludes.is_empty() {
                    excludes = crate::config::default_scan_excludes();
                }
                self.config.scan.excludes = excludes;
                self.persist("Scan exclude folders updated");
                self.settings_editing = false;
                self.input_buffer.clear();
                self.setup_watcher();
            }
            9 => {
                let trimmed_app = trimmed.to_string();
                if !trimmed_app.is_empty() {
                    let allowed = ["git", "gitui", "lazygit"];
                    if allowed.contains(&trimmed_app.as_str()) {
                        self.config.git_app = trimmed_app;
                        self.persist("Preferred Git Client updated");
                        self.settings_editing = false;
                        self.input_buffer.clear();
                    } else {
                        self.status_message =
                            Some("Invalid client! Allowed values: git, gitui, lazygit".to_string());
                    }
                } else {
                    self.status_message = Some("Preferred Git Client cannot be empty".to_string());
                }
            }
            56 => {
                let trimmed_editor = trimmed.to_string();
                if !trimmed_editor.is_empty() {
                    self.config.editor = trimmed_editor;
                    self.persist("Editor Command updated");
                    self.settings_editing = false;
                    self.input_buffer.clear();
                } else {
                    self.status_message = Some("Editor Command cannot be empty".to_string());
                }
            }
            60 => {
                if let Ok(val) = trimmed.parse::<u64>() {
                    self.config.auto_fetch_interval_mins = val;
                    self.persist("Auto-fetch interval updated");
                    self.settings_editing = false;
                    self.input_buffer.clear();
                } else {
                    self.status_message = Some("Invalid integer".to_string());
                }
            }
            84 => {
                if let Ok(val) = trimmed.parse::<u64>() {
                    // 0 disables the limit; anything between 1 and 4 seconds is
                    // shorter than a normal TLS handshake and would fail every fetch.
                    if val == 0 || val >= 5 {
                        self.config.fetch_timeout_secs = val;
                        self.persist("Fetch timeout updated");
                        self.settings_editing = false;
                        self.input_buffer.clear();
                    } else {
                        self.status_message = Some(
                            "Fetch timeout must be 0 (disabled) or at least 5 seconds".to_string(),
                        );
                    }
                } else {
                    self.status_message = Some("Invalid integer".to_string());
                }
            }
            61 => {
                self.config.watch_dirs = trimmed
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                self.persist("Watch directories updated");
                self.settings_editing = false;
                self.input_buffer.clear();
                self.setup_watcher();
            }
            64 => {
                if let Ok(val) = trimmed.parse::<usize>() {
                    self.config.graph_max_commits = val;
                    self.persist("Graph max commits updated");
                    self.settings_editing = false;
                    self.input_buffer.clear();
                } else {
                    self.status_message = Some("Invalid integer".to_string());
                }
            }
            65 => {
                if let Ok(val) = trimmed.parse::<u64>() {
                    self.config.detail_cache_ttl_secs = val;
                    self.persist("Detail cache TTL updated");
                    self.settings_editing = false;
                    self.input_buffer.clear();
                } else {
                    self.status_message = Some("Invalid integer".to_string());
                }
            }
            66 => {
                if let Ok(val) = trimmed.parse::<u64>() {
                    self.config.tab_ttl_secs = val;
                    self.persist("Tab cache TTL updated");
                    self.settings_editing = false;
                    self.input_buffer.clear();
                } else {
                    self.status_message = Some("Invalid integer".to_string());
                }
            }
            82 => {
                if let Ok(val) = trimmed.parse::<usize>() {
                    self.config.tile_columns = val;
                    self.persist("Tile columns updated");
                    self.settings_editing = false;
                    self.input_buffer.clear();
                } else {
                    self.status_message = Some("Invalid integer".to_string());
                }
            }
            80 => {
                if let Ok(val) = trimmed.parse::<u32>() {
                    if val >= 1 {
                        self.config.stale_threshold_months = val;
                        self.persist("Stale threshold updated");
                        self.settings_editing = false;
                        self.input_buffer.clear();
                    } else {
                        self.status_message =
                            Some("Stale threshold must be at least 1 month".to_string());
                    }
                } else {
                    self.status_message = Some("Invalid integer".to_string());
                }
            }
            idx if idx >= 14 => {
                if let Some(action) = crate::keybindings::Action::from_index(idx) {
                    let keys: Vec<String> = trimmed
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();

                    let mut all_valid = true;
                    for k in &keys {
                        if crate::keybindings::parse_key(k).is_none() {
                            all_valid = false;
                            break;
                        }
                    }

                    if !all_valid {
                        self.status_message =
                            Some("One or more key mappings are invalid".to_string());
                    } else if let Some(conflicting_action) =
                        self.keybindings.find_conflict(action, &keys)
                    {
                        let conflicting_idx = conflicting_action.to_index();
                        let action_name = crate::popups::settings::get_label(conflicting_idx);
                        let section_name =
                            crate::popups::settings::get_action_section_name(conflicting_idx);
                        self.status_message = Some(format!(
                            "Conflict: key already bound to '{}' in '{}'",
                            action_name, section_name
                        ));
                        self.settings_editing = false;
                        self.input_buffer.clear();
                    } else {
                        self.keybindings.update_action_keys(action, keys);
                        let config_dir = self.config_path.parent().unwrap_or(&self.config_path);
                        if let Err(e) = self.keybindings.save(config_dir) {
                            self.status_message =
                                Some(format!("Failed to save keybindings: {}", e));
                        } else {
                            self.status_message = Some("Keybindings updated and saved".to_string());
                            self.settings_editing = false;
                            self.input_buffer.clear();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn cancel_settings_edit(&mut self) {
        self.settings_editing = false;
        self.input_buffer.clear();
    }

    pub fn get_available_themes(&self) -> Vec<String> {
        let mut themes = vec!["default".to_string()];
        let themes_dir = self.config_path.parent().unwrap_or(&self.config_path).join("themes");
        if themes_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(themes_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().is_some_and(|ext| ext == "theme") {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            let theme_name = stem.to_string();
                            if theme_name != "default" && !themes.contains(&theme_name) {
                                themes.push(theme_name);
                            }
                        }
                    }
                }
            }
        }
        themes.sort();
        themes
    }

    pub fn cancel_input(&mut self) {
        self.input_buffer.clear();
        self.sync_cursor(0);
        self.mode = Mode::Normal;
    }

    /// Seed the shared input buffer and park the caret at the end, which is
    /// where a user expects it when a field opens pre-filled.
    pub fn set_input_buffer(&mut self, text: String) {
        let len = text.chars().count();
        self.input_buffer = text;
        self.sync_cursor(len);
    }

    /// Record the caret and the buffer length it is valid for.
    fn sync_cursor(&mut self, at: usize) {
        let len = self.input_buffer.chars().count();
        self.input_cursor = at.min(len);
        self.input_cursor_len = len;
    }

    /// Caret position for the current buffer.
    ///
    /// If the buffer length no longer matches what the caret was synced
    /// against, something assigned `input_buffer` directly; the caret is then
    /// reported at the end rather than wherever it happened to be, so typing
    /// and backspace act on the end of the new text.
    pub fn input_cursor_clamped(&self) -> usize {
        let len = self.input_buffer.chars().count();
        if self.input_cursor_len != len { len } else { self.input_cursor.min(len) }
    }

    /// Byte offset of the (clamped) caret, for slicing `input_buffer`.
    fn input_cursor_byte(&self) -> usize {
        let chars = self.input_cursor_clamped();
        self.input_buffer
            .char_indices()
            .nth(chars)
            .map(|(i, _)| i)
            .unwrap_or(self.input_buffer.len())
    }

    pub fn input_left(&mut self) {
        let at = self.input_cursor_clamped().saturating_sub(1);
        self.sync_cursor(at);
    }

    pub fn input_right(&mut self) {
        let at = self.input_cursor_clamped() + 1;
        self.sync_cursor(at);
    }

    pub fn input_home(&mut self) {
        self.sync_cursor(0);
    }

    pub fn input_end(&mut self) {
        let len = self.input_buffer.chars().count();
        self.sync_cursor(len);
    }

    /// Delete the character *under* the caret, leaving the caret in place.
    pub fn input_delete(&mut self) {
        let at = self.input_cursor_clamped();
        if at < self.input_buffer.chars().count() {
            let byte = self.input_cursor_byte();
            self.input_buffer.remove(byte);
            self.sync_cursor(at);
        }
    }

    pub fn input_char(&mut self, c: char) {
        let byte = self.input_cursor_byte();
        let at = self.input_cursor_clamped() + 1;
        self.input_buffer.insert(byte, c);
        self.sync_cursor(at);
    }

    pub fn input_str(&mut self, s: &str) {
        if matches!(self.mode, Mode::CommitInput) && self.commit_popup.editing {
            self.commit_popup.insert_str(s);
            return;
        }
        let clean: String = s.chars().filter(|c| *c != '\r' && *c != '\n').collect();
        if matches!(self.mode, Mode::RepoSettings) && self.repo_settings_editing {
            if matches!(self.repo_settings_selected_index, 5..=7) {
                self.repo_settings_input.push_str(&clean);
            } else {
                self.repo_settings_input.extend(clean.chars().filter(char::is_ascii_digit));
            }
            return;
        }
        if matches!(self.mode, Mode::LabelSettings) && self.label_settings_editing {
            if self.label_settings_selected_index == 5 {
                self.label_settings_input.push_str(&clean);
            } else {
                self.label_settings_input.extend(clean.chars().filter(char::is_ascii_digit));
            }
            return;
        }
        if matches!(self.mode, Mode::DebugLogs) && self.debug_log_search_editing {
            if let Some(ref mut query) = self.debug_log_search_query {
                query.push_str(&clean);
                self.debug_log_scroll = 0;
            }
            return;
        }
        if matches!(self.mode, Mode::TagCreateInput) && self.tag_create_focus_message {
            self.tag_create_message.push_str(&clean);
            return;
        }
        if matches!(self.mode, Mode::GlobalSearch) && !self.global_search_focus_input {
            return;
        }
        let byte = self.input_cursor_byte();
        let at = self.input_cursor_clamped() + clean.chars().count();
        self.input_buffer.insert_str(byte, &clean);
        self.sync_cursor(at);
    }

    /// Delete the character *before* the caret and step back over it.
    pub fn input_backspace(&mut self) {
        let at = self.input_cursor_clamped();
        if at == 0 {
            return;
        }
        self.sync_cursor(at - 1);
        let byte = self.input_cursor_byte();
        self.input_buffer.remove(byte);
        self.sync_cursor(at - 1);
    }

    pub(in crate::app) fn canonical_path(p: &std::path::Path) -> PathBuf {
        match std::fs::canonicalize(p) {
            Ok(canon) => canon,
            Err(_) => p.to_path_buf(),
        }
    }

    pub fn start_bulk_add(&mut self) {
        crate::debug_log::info("Initiating bulk repository add");
        self.start_bulk_repo_scan();
    }

    pub fn commit_bulk_add(&mut self) {
        let trimmed = self.input_buffer.trim().to_string();
        self.input_buffer.clear();
        self.mode = Mode::Normal;
        if !trimmed.is_empty() {
            self.bulk_add_path_with_labels(trimmed, vec![]);
        }
    }

    pub fn commit_bulk_add_label_input(&mut self) {
        let labels_str = self.input_buffer.trim().to_string();
        let labels: Vec<String> = if labels_str.is_empty() {
            Vec::new()
        } else {
            labels_str.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
        };
        let path = self.pending_bulk_add_repo.take().unwrap_or_default();
        self.bulk_add_path_with_labels(path, labels);
        self.input_buffer.clear();
        self.mode = Mode::Normal;
    }

    pub fn bulk_add_path(&mut self, path: String) {
        self.bulk_add_path_with_labels(path, vec![]);
    }

    pub fn bulk_add_path_with_labels(&mut self, path: String, labels: Vec<String>) {
        let trimmed = path.trim().to_string();
        if trimmed.is_empty() {
            return;
        }

        let base_path = repo::expand_tilde(&trimmed);
        if !base_path.exists() {
            self.set_error(format!("Directory does not exist: {}", trimmed));
            return;
        }
        if !base_path.is_dir() {
            self.set_error(format!("Path is not a directory: {}", trimmed));
            return;
        }

        let entries = match std::fs::read_dir(&base_path) {
            Ok(read) => read,
            Err(e) => {
                self.set_error(format!("Failed to read directory: {}", e));
                return;
            }
        };

        let mut added_paths = Vec::new();

        for entry_opt in entries {
            let entry = match entry_opt {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            if path.is_dir() && path.join(".git").exists() {
                if let Some(sub_name) = path.file_name().and_then(|n| n.to_str()) {
                    let mut base_str = trimmed.clone();
                    if !base_str.ends_with(std::path::MAIN_SEPARATOR) {
                        base_str.push(std::path::MAIN_SEPARATOR);
                    }
                    let path_to_add = format!("{}{}", base_str, sub_name);
                    added_paths.push(path_to_add);
                }
            }
        }

        added_paths.sort();

        if added_paths.is_empty() {
            self.status_message = Some("No matching directories found to add".to_string());
            return;
        }

        let mut newly_added_count = 0;
        let mut first_new_path = None;
        for path_str in added_paths {
            let trimmed_path = path_str.trim().to_string();
            let new_expanded = repo::expand_tilde(&trimmed_path);
            let new_canonical = Self::canonical_path(&new_expanded);

            let already_exists = self.config.items.iter().any(|item| {
                let item_expanded = repo::expand_tilde(item);
                item.trim() == trimmed_path
                    || item_expanded == new_expanded
                    || Self::canonical_path(&item_expanded) == new_canonical
            });

            if !already_exists {
                let status = repo::inspect_summary(&trimmed_path);
                self.statuses.push(status);
                self.config.items.push(trimmed_path.clone());
                self.original_items.push(trimmed_path.clone());
                if !labels.is_empty() {
                    self.config.labels.insert(trimmed_path.clone(), labels.clone());
                }
                if first_new_path.is_none() {
                    first_new_path = Some(trimmed_path);
                }
                newly_added_count += 1;
            }
        }

        if newly_added_count > 0 {
            self.sort_items_in_place();
            self.repo_search_query = None;
            if let Some(ref target) = first_new_path {
                self.reveal_home_row(target);
            }
            self.persist(&format!("Added {} directories", newly_added_count));
        } else {
            self.status_message = Some("All discovered directories were already added".to_string());
        }
    }

    pub fn start_import_clone(&mut self) {
        let url = self.import_url.trim().to_string();
        let dest = self.import_dest.trim().to_string();
        let name = self.import_name.trim().to_string();

        if url.is_empty() || dest.is_empty() {
            self.set_error("Source URL and Destination path cannot be empty".to_string());
            self.mode = Mode::Normal;
            return;
        }

        let mut dest_path = repo::expand_tilde(&dest);
        if !name.is_empty() {
            dest_path.push(&name);
        }
        let dest_str = dest_path.to_string_lossy().to_string();

        crate::debug_log::info(format!(
            "Network Action: Cloning remote repository {} to {} (user triggered)",
            url, dest_str
        ));
        self.fetching = true;
        self.status_message = Some(format!("Cloning {}...", url));
        self.mode = Mode::Normal;

        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let res = (|| -> Result<String, String> {
                let dest_expanded = repo::expand_tilde(&dest_str);
                let _ = std::fs::create_dir_all(&dest_expanded);

                let trimmed_url = url.trim().to_lowercase();
                if trimmed_url.contains("ext:") || trimmed_url.contains("fd:") {
                    return Err("Malicious URL protocol rejected".to_string());
                }

                let mut cmd = crate::git_cmd::git_command();
                cmd.arg("clone").arg("--").arg(&url).arg(&dest_expanded);

                let output = cmd.output().map_err(|e| e.to_string())?;
                if output.status.success() {
                    Ok(dest_str)
                } else {
                    let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
                    Err(format!("Clone failed: {}", err))
                }
            })();

            let result = res.map_err(|e| format!("Failed to clone: {}", e));
            let _ = tx.send(BgEvent::CloneResult(result));
        });
    }

    pub fn close_dialog(&mut self) {
        self.mode = Mode::Normal;
    }

    /// Removes label settings whose label is no longer carried by any tracked
    /// repository. The app already treats a label as gone once no repo uses it
    /// (see `clear_label_filter_if_orphaned`); pruning its settings here keeps
    /// the two consistent and stops orphaned `label_configs` sections from
    /// silently reattaching if the label name is reused later.
    fn prune_orphaned_label_configs(&mut self) {
        if self.config.label_configs.is_empty() {
            return;
        }
        let live: std::collections::HashSet<&String> =
            self.config.items.iter().filter_map(|p| self.config.labels.get(p)).flatten().collect();
        self.config.label_configs.retain(|label, _| live.contains(label));
    }

    /// Persists `self.config` (and, since pruning may have touched it,
    /// `self.state`) and records a status message (success or the save
    /// error) for the next render.
    pub fn persist(&mut self, success_msg: &str) {
        self.prune_orphaned_label_configs();
        self.prune_label_slots();
        self.resolve_repo_themes();
        let saved = save_config(&self.config, &self.config_path)
            .and_then(|_| save_state(&self.state, &self.state_path()));
        match saved {
            Ok(()) => self.status_message = Some(success_msg.to_string()),
            Err(e) => self.set_error(format!("Save failed: {}", e)),
        }
        self.setup_watcher();
    }

    /// Persists only `self.state` — for changes that happen as a side effect
    /// of using the app (opening a repository, viewing a label) — so they
    /// never rewrite `config.toml` or restart the directory watcher.
    pub fn persist_state(&mut self, success_msg: &str) {
        self.prune_label_slots();
        match save_state(&self.state, &self.state_path()) {
            Ok(()) => self.status_message = Some(success_msg.to_string()),
            Err(e) => self.set_error(format!("Save failed: {}", e)),
        }
    }
}
