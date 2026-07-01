use super::*;

impl App {
    pub fn start_add(&mut self) {
        crate::debug_log::info("Initiating repository add");
        if !self.config.fzf.enabled {
            self.mode = Mode::Adding;
            self.commit_popup.input_buffer.clear();
        } else if !self.is_fzf_installed() {
            self.mode = Mode::Adding;
            self.commit_popup.input_buffer.clear();
            self.status_message =
                Some("fzf is not installed. Falling back to manual add.".to_string());
        } else {
            self.pending_fzf = true;
        }
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
                self.commit_popup.input_buffer.clear();
                self.mode = Mode::Normal;
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
            self.persist("Saved");
        }
        self.commit_popup.input_buffer.clear();
        self.mode = Mode::Normal;
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
        self.commit_popup.input_buffer.clear();
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
}
