use super::*;

impl App {
    pub fn start_add(&mut self) {
        crate::debug_log::info("Initiating repository add");
        self.start_repo_scan();
    }

    pub fn start_edit(&mut self) {
        if let Some(current) = self.get_selected_item() {
            crate::debug_log::info(format!("Editing repository entry: {}", current));
            self.input_buffer = current.clone();
            self.mode = Mode::Editing;
        }
    }

    pub fn request_delete(&mut self) {
        if self.get_items_len() > 0 {
            self.mode = Mode::ConfirmDelete;
        }
    }

    pub fn commit_add(&mut self) {
        let trimmed = self.input_buffer.trim().to_string();
        self.commit_add_with_labels(trimmed, vec![]);
    }

    pub fn commit_add_with_labels(&mut self, path: String, labels: Vec<String>) {
        let trimmed = path.trim().to_string();
        if !trimmed.is_empty() {
            let new_expanded = repo::expand_tilde(&trimmed);
            let new_canonical = Self::canonical_path(&new_expanded);
            let already_exists = self.config.items.iter().any(|item| {
                let item_expanded = repo::expand_tilde(item);
                item.trim() == trimmed
                    || item_expanded == new_expanded
                    || Self::canonical_path(&item_expanded) == new_canonical
            });
            if already_exists {
                self.status_message = Some("Repository already added".to_string());
                self.input_buffer.clear();
                self.mode = Mode::Normal;
                return;
            }

            let status = repo::inspect_summary(&trimmed);
            self.statuses.push(status);
            self.config.items.push(trimmed.clone());
            self.original_items.push(trimmed.clone());

            if !labels.is_empty() {
                self.config.labels.insert(trimmed.clone(), labels);
            }

            self.sort_items_in_place();

            self.repo_search_query = None;
            if let Some(pos) = self.config.items.iter().position(|x| x == &trimmed) {
                self.selected_index = pos;
            } else {
                self.selected_index = self.config.items.len() - 1;
            }
            self.persist("Saved");
        }
        self.input_buffer.clear();
        self.mode = Mode::Normal;
    }

    pub fn commit_add_label_input(&mut self) {
        let labels_str = self.input_buffer.trim().to_string();
        let labels: Vec<String> = if labels_str.is_empty() {
            Vec::new()
        } else {
            labels_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        };
        let path = self.pending_add_repo.take().unwrap_or_default();
        self.commit_add_with_labels(path, labels);
    }

    pub fn add_repo_path(&mut self, path: String) {
        let trimmed = path.trim().to_string();
        if !trimmed.is_empty() {
            let new_expanded = repo::expand_tilde(&trimmed);
            let new_canonical = Self::canonical_path(&new_expanded);
            let already_exists = self.config.items.iter().any(|item| {
                let item_expanded = repo::expand_tilde(item);
                item.trim() == trimmed
                    || item_expanded == new_expanded
                    || Self::canonical_path(&item_expanded) == new_canonical
            });
            if already_exists {
                self.status_message = Some("Repository already added".to_string());
                return;
            }

            let status = repo::inspect_summary(&trimmed);
            self.statuses.push(status);
            self.config.items.push(trimmed.clone());
            self.original_items.push(trimmed.clone());

            self.sort_items_in_place();

            self.repo_search_query = None;
            if let Some(pos) = self.config.items.iter().position(|x| x == &trimmed) {
                self.selected_index = pos;
            } else {
                self.selected_index = self.config.items.len() - 1;
            }
            self.persist("Added repository");
        }
    }

    pub fn commit_edit(&mut self) {
        let trimmed = self.input_buffer.trim().to_string();
        if !trimmed.is_empty() {
            if let Some(orig_idx) = self.get_selected_item_index() {
                if orig_idx < self.config.items.len() {
                    let old_item = self.config.items[orig_idx].clone();

                    if let Some(pos) = self.original_items.iter().position(|x| x == &old_item) {
                        self.original_items[pos] = trimmed.clone();
                    }

                    if let Some(time) = self.config.visits.remove(&old_item) {
                        self.config.visits.insert(trimmed.clone(), time);
                    }

                    if self.config.pinned.remove(&old_item) {
                        self.config.pinned.insert(trimmed.clone());
                    }

                    self.config.items[orig_idx] = trimmed.clone();
                    self.statuses[orig_idx] = repo::inspect_summary(&trimmed);

                    self.sort_items_in_place();

                    self.repo_search_query = None;

                    if let Some(pos) = self.config.items.iter().position(|x| x == &trimmed) {
                        self.selected_index = pos;
                    }
                    self.persist("Saved");
                }
            }
        }
        self.input_buffer.clear();
        self.mode = Mode::Normal;
    }

    pub fn confirm_delete(&mut self) {
        if !self.multi_selected.is_empty() {
            let to_remove = self.multi_selected.clone();
            self.multi_selected.clear();

            for item in to_remove {
                if let Some(pos) = self.config.items.iter().position(|x| x == &item) {
                    self.config.items.remove(pos);
                    if pos < self.statuses.len() {
                        self.statuses.remove(pos);
                    }
                }
                if let Some(pos) = self.original_items.iter().position(|x| x == &item) {
                    self.original_items.remove(pos);
                }
                self.config.visits.remove(&item);
                self.config.pinned.remove(&item);
            }
            self.persist("Deleted selected repositories");
        } else if let Some(orig_idx) = self.get_selected_item_index() {
            if orig_idx < self.config.items.len() {
                let item = self.config.items.remove(orig_idx);
                if orig_idx < self.statuses.len() {
                    self.statuses.remove(orig_idx);
                }
                if let Some(pos) = self.original_items.iter().position(|x| x == &item) {
                    self.original_items.remove(pos);
                }
                self.config.visits.remove(&item);
                self.config.pinned.remove(&item);
                self.persist("Deleted");
            }
        }
        self.repo_search_query = None;
        self.selected_index = 0;
        self.mode = Mode::Normal;
    }

    pub fn start_edit_labels(&mut self) {
        if let Some(current) = self.get_selected_item() {
            crate::debug_log::info(format!("Editing labels for repository: {}", current));
            let current_labels = self
                .config
                .labels
                .get(current.as_str())
                .map(|lbls| lbls.join(", "))
                .unwrap_or_default();
            self.input_buffer = current_labels;
            self.mode = Mode::LabelInput;
        }
    }

    pub fn commit_edit_labels(&mut self) {
        let current = self.get_selected_item().cloned();
        if let Some(current) = current {
            let trimmed = self.input_buffer.trim();
            if trimmed.is_empty() {
                self.config.labels.remove(current.as_str());
            } else {
                let lbls: Vec<String> = trimmed
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if lbls.is_empty() {
                    self.config.labels.remove(current.as_str());
                } else {
                    self.config.labels.insert(current.clone(), lbls);
                }
            }
            self.persist("Saved labels");
        }
        self.input_buffer.clear();
        self.mode = Mode::Normal;
    }

    pub fn start_repo_scan(&mut self) {
        self.scanned_repos.clear();
        self.repo_scan_selection = 0;
        self.repo_scan_active = true;
        self.repo_scan_count = 0;
        self.input_buffer.clear();
        self.previous_mode = Some(self.mode);
        self.mode = Mode::RepoScanPicker;

        let start_dir = repo::expand_tilde(&self.config.scan.start_dir);
        let max_depth = self.config.scan.max_depth;
        let excludes = self.config.scan.excludes.clone();
        let tx = self.tx.clone();

        run_directory_scan(start_dir, max_depth, excludes, tx, true);
    }

    pub fn start_bulk_repo_scan(&mut self) {
        self.scanned_repos.clear();
        self.repo_scan_selection = 0;
        self.repo_scan_active = true;
        self.repo_scan_count = 0;
        self.input_buffer.clear();
        self.previous_mode = Some(self.mode);
        self.mode = Mode::BulkAddScanPicker;

        let start_dir = repo::expand_tilde(&self.config.scan.start_dir);
        let max_depth = self.config.scan.max_depth;
        let excludes = self.config.scan.excludes.clone();
        let tx = self.tx.clone();

        run_directory_scan(start_dir, max_depth, excludes, tx, false);
    }

    pub fn get_scan_matches(&self) -> Vec<(String, String)> {
        let query = self.input_buffer.to_lowercase();
        let mut results = Vec::new();

        if !self.input_buffer.trim().is_empty() {
            results.push((
                format!("[Use manual path: {}]", self.input_buffer.trim()),
                self.input_buffer.trim().to_string(),
            ));
        }

        if query.is_empty() {
            results.extend(self.scanned_repos.clone());
            return results;
        }

        let mut matches = Vec::new();
        for (name, path) in &self.scanned_repos {
            let name_lower = name.to_lowercase();
            let path_lower = path.to_lowercase();

            if name_lower.contains(&query) {
                let score = 1000 - (name_lower.len() - query.len());
                matches.push((score, name.clone(), path.clone()));
            } else if path_lower.contains(&query) {
                let score = 500 - (path_lower.len() - query.len());
                matches.push((score, name.clone(), path.clone()));
            } else {
                let mut name_chars = name_lower.chars();
                let mut matched = true;
                for qc in query.chars() {
                    if !name_chars.any(|nc| nc == qc) {
                        matched = false;
                        break;
                    }
                }
                if matched {
                    matches.push((100, name.clone(), path.clone()));
                }
            }
        }

        matches.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        results.extend(matches.into_iter().map(|(_, name, path)| (name, path)));
        results
    }

    pub fn start_branch_search(&mut self) {
        self.branch_search_selection = 0;
        self.input_buffer.clear();
        self.previous_mode = Some(self.mode);
        self.mode = Mode::BranchSearchInput;
    }

    pub fn get_branch_search_matches(&self) -> Vec<(String, bool)> {
        let mut results = Vec::new();
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let query = self.input_buffer.to_lowercase();

            let all_branches: Vec<(String, bool)> = info
                .local_branches
                .iter()
                .map(|b| (b.name.clone(), false))
                .chain(info.remote_branches.iter().map(|b| (b.name.clone(), true)))
                .collect();

            if query.is_empty() {
                return all_branches;
            }

            let mut matches = Vec::new();
            for (name, is_remote) in all_branches {
                let name_lower = name.to_lowercase();
                if name_lower.contains(&query) {
                    let score = 1000 - (name_lower.len() - query.len());
                    matches.push((score, name, is_remote));
                } else {
                    let mut name_chars = name_lower.chars();
                    let mut matched = true;
                    for qc in query.chars() {
                        if !name_chars.any(|nc| nc == qc) {
                            matched = false;
                            break;
                        }
                    }
                    if matched {
                        matches.push((100, name, is_remote));
                    }
                }
            }
            matches.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            results = matches.into_iter().map(|(_, name, is_remote)| (name, is_remote)).collect();
        }
        results
    }

    pub fn start_file_search(&mut self) {
        self.file_search_selection = 0;
        self.input_buffer.clear();
        self.previous_mode = Some(self.mode);
        self.mode = Mode::FileSearchInput;
    }

    pub fn get_file_search_matches(&self) -> Vec<String> {
        let mut results = Vec::new();
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let query = self.input_buffer.to_lowercase();
            let all_files = if let repo::TabData::Loaded(files) = &info.files {
                files.clone()
            } else {
                Vec::new()
            };

            if query.is_empty() {
                return all_files;
            }

            let mut matches = Vec::new();
            for file in all_files {
                let file_lower = file.to_lowercase();
                if file_lower.contains(&query) {
                    let score = 1000 - (file_lower.len() - query.len());
                    matches.push((score, file));
                } else {
                    let mut file_chars = file_lower.chars();
                    let mut matched = true;
                    for qc in query.chars() {
                        if !file_chars.any(|nc| nc == qc) {
                            matched = false;
                            break;
                        }
                    }
                    if matched {
                        matches.push((100, file));
                    }
                }
            }
            matches.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            results = matches.into_iter().map(|(_, file)| file).collect();
        }
        results
    }

    pub fn start_commit_fuzzy_search(&mut self) {
        self.commit_search_selection = 0;
        self.input_buffer.clear();
        self.previous_mode = Some(self.mode);
        self.mode = Mode::CommitFuzzySearch;
    }

    pub fn get_commit_fuzzy_matches(&self) -> Vec<(usize, String, String, String)> {
        let mut results = Vec::new();
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let query = self.input_buffer.to_lowercase();

            if query.is_empty() {
                return info
                    .commits
                    .iter()
                    .enumerate()
                    .map(|(idx, c)| (idx, c.id.clone(), c.author.clone(), c.summary.clone()))
                    .collect();
            }

            let mut matches = Vec::new();
            for (idx, c) in info.commits.iter().enumerate() {
                let summary_lower = c.summary.to_lowercase();
                let author_lower = c.author.to_lowercase();
                let id_lower = c.id.to_lowercase();

                if summary_lower.contains(&query) {
                    let score = 1000 - (summary_lower.len() - query.len());
                    matches.push((score, idx, c.id.clone(), c.author.clone(), c.summary.clone()));
                } else if author_lower.contains(&query) {
                    let score = 500 - (author_lower.len() - query.len());
                    matches.push((score, idx, c.id.clone(), c.author.clone(), c.summary.clone()));
                } else if id_lower.contains(&query) {
                    let score = 500 - (id_lower.len() - query.len());
                    matches.push((score, idx, c.id.clone(), c.author.clone(), c.summary.clone()));
                } else {
                    let mut summary_chars = summary_lower.chars();
                    let mut matched = true;
                    for qc in query.chars() {
                        if !summary_chars.any(|nc| nc == qc) {
                            matched = false;
                            break;
                        }
                    }
                    if matched {
                        matches.push((100, idx, c.id.clone(), c.author.clone(), c.summary.clone()));
                    }
                }
            }
            matches.sort_by(|a, b| b.0.cmp(&a.0).then(a.4.cmp(&b.4)));
            results = matches
                .into_iter()
                .map(|(_, idx, sha, author, summary)| (idx, sha, author, summary))
                .collect();
        }
        results
    }

    pub fn start_tag_search(&mut self) {
        self.tag_search_selection = 0;
        self.input_buffer.clear();
        self.previous_mode = Some(self.mode);
        self.mode = Mode::TagSearchInput;
    }

    pub fn get_tag_search_matches(&self) -> Vec<String> {
        let mut results = Vec::new();
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let query = self.input_buffer.to_lowercase();
            let all_tags = if let repo::TabData::Loaded(tags) = &info.local_tags {
                tags.iter().map(|t| t.name.clone()).collect()
            } else {
                Vec::new()
            };

            if query.is_empty() {
                return all_tags;
            }

            let mut matches = Vec::new();
            for tag in all_tags {
                let tag_lower = tag.to_lowercase();
                if tag_lower.contains(&query) {
                    let score = 1000 - (tag_lower.len() - query.len());
                    matches.push((score, tag));
                } else {
                    let mut tag_chars = tag_lower.chars();
                    let mut matched = true;
                    for qc in query.chars() {
                        if !tag_chars.any(|nc| nc == qc) {
                            matched = false;
                            break;
                        }
                    }
                    if matched {
                        matches.push((100, tag));
                    }
                }
            }
            matches.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            results = matches.into_iter().map(|(_, tag)| tag).collect();
        }
        results
    }

    pub fn trigger_global_search(&mut self) {
        let query = self.input_buffer.clone();
        if query.trim().is_empty() {
            return;
        }
        self.global_search_query = query.clone();
        self.global_search_running = true;
        self.global_search_results.clear();
        self.global_search_selection = 0;
        
        let tx = self.global_search_tx.clone();
        let repo_paths = self.config.items.clone();
        
        std::thread::spawn(move || {
            let mut handles = Vec::new();
            let query_lower = query.to_lowercase();
            
            for path_str in repo_paths {
                let path = std::path::PathBuf::from(&path_str);
                let query_lower = query_lower.clone();
                let path_str_clone = path_str.clone();
                
                let handle = std::thread::spawn(move || {
                    let mut local_results = Vec::new();
                    let repo_name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| path_str_clone.clone());
                        
                    fn walk_and_search(
                        current_dir: &std::path::Path,
                        base_dir: &std::path::Path,
                        repo_name: &str,
                        repo_path: &str,
                        query: &str,
                        results: &mut Vec<SearchResult>,
                    ) {
                        if let Ok(entries) = std::fs::read_dir(current_dir) {
                            for entry in entries.flatten() {
                                let path = entry.path();
                                let name = entry.file_name();
                                let name_str = name.to_string_lossy();
                                
                                if name_str.starts_with('.') 
                                    || name_str == "target" 
                                    || name_str == "node_modules"
                                    || name_str == "build"
                                    || name_str == "dist"
                                    || name_str == "vendor"
                                {
                                    continue;
                                }
                                
                                if path.is_dir() {
                                    walk_and_search(&path, base_dir, repo_name, repo_path, query, results);
                                } else if path.is_file() {
                                    if let Ok(meta) = entry.metadata() {
                                        if meta.len() > 1_000_000 {
                                            continue;
                                        }
                                    }
                                    if let Ok(content) = std::fs::read_to_string(&path) {
                                        for (i, line) in content.lines().enumerate() {
                                            if line.to_lowercase().contains(query) {
                                                let rel_path = path
                                                    .strip_prefix(base_dir)
                                                    .unwrap_or(&path)
                                                    .to_string_lossy()
                                                    .to_string();
                                                    
                                                results.push(SearchResult {
                                                    repo_name: repo_name.to_string(),
                                                    repo_path: repo_path.to_string(),
                                                    file_rel_path: rel_path,
                                                    line_number: i + 1,
                                                    line_content: line.trim().to_string(),
                                                });
                                                if results.len() > 100 {
                                                    return;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    walk_and_search(&path, &path, &repo_name, &path_str_clone, &query_lower, &mut local_results);
                    local_results
                });
                handles.push(handle);
            }
            
            let mut all_results = Vec::new();
            for h in handles {
                if let Ok(res) = h.join() {
                    all_results.extend(res);
                }
            }
            all_results.truncate(500);
            let _ = tx.send(all_results);
        });
    }

    pub fn select_global_search_result(&mut self) {
        if self.global_search_results.is_empty() || self.global_search_selection >= self.global_search_results.len() {
            return;
        }
        let result = self.global_search_results[self.global_search_selection].clone();
        if let Some(pos) = self.config.items.iter().position(|x| x == &result.repo_path) {
            self.jump_to_repo(pos);
            self.open_detail();
            self.input_buffer.clear();
            self.mode = Mode::Detail;
        }
    }
}
