use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    // Global
    ToggleStatusBar,
    Help,
    Close,

    // Home Page
    HomeMoveDown,
    HomeMoveUp,
    HomePageDown,
    HomePageUp,
    HomeHome,
    HomeEnd,
    HomeAddRepo,
    HomeBulkAdd,
    HomeEditRepo,
    HomeDeleteRepo,
    HomeOpenDebugLogs,
    HomeEditLabels,
    HomeAbout,
    HomeRefresh,
    HomeCycleSort,
    HomeToggleSortReverse,
    HomeTogglePin,
    HomeOpenSettings,
    HomeImportRepo,
    HomeOpenGitApp,
    HomeSearchRepo,
    HomeOpenDetail,

    // Detail / Workspace Tab Navigation
    CloseDetail,
    DetailHelp,
    CycleFocusForward,
    CycleFocusBackward,
    RefreshDetail,
    CycleTabForward,
    CycleTabBackward,
    GoToTab1,
    GoToTab2,
    GoToTab3,
    GoToTab4,
    GoToTab5,
    GoToTab6,
    GoToTab7,
    GoToTab8,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct GlobalKeybindings {
    pub toggle_status_bar: Option<Vec<String>>,
    pub help: Option<Vec<String>>,
    pub close: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct HomeKeybindings {
    pub move_down: Option<Vec<String>>,
    pub move_up: Option<Vec<String>>,
    pub page_down: Option<Vec<String>>,
    pub page_up: Option<Vec<String>>,
    pub home: Option<Vec<String>>,
    pub end: Option<Vec<String>>,
    pub add_repo: Option<Vec<String>>,
    pub bulk_add: Option<Vec<String>>,
    pub edit_repo: Option<Vec<String>>,
    pub delete_repo: Option<Vec<String>>,
    pub open_debug_logs: Option<Vec<String>>,
    pub edit_labels: Option<Vec<String>>,
    pub about: Option<Vec<String>>,
    pub refresh: Option<Vec<String>>,
    pub cycle_sort: Option<Vec<String>>,
    pub toggle_sort_reverse: Option<Vec<String>>,
    pub toggle_pin: Option<Vec<String>>,
    pub open_settings: Option<Vec<String>>,
    pub import_repo: Option<Vec<String>>,
    pub open_git_app: Option<Vec<String>>,
    pub search_repo: Option<Vec<String>>,
    pub open_detail: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct NavigationKeybindings {
    pub close_detail: Option<Vec<String>>,
    pub detail_help: Option<Vec<String>>,
    pub cycle_focus_forward: Option<Vec<String>>,
    pub cycle_focus_backward: Option<Vec<String>>,
    pub refresh_detail: Option<Vec<String>>,
    pub cycle_tab_forward: Option<Vec<String>>,
    pub cycle_tab_backward: Option<Vec<String>>,
    pub go_to_tab_1: Option<Vec<String>>,
    pub go_to_tab_2: Option<Vec<String>>,
    pub go_to_tab_3: Option<Vec<String>>,
    pub go_to_tab_4: Option<Vec<String>>,
    pub go_to_tab_5: Option<Vec<String>>,
    pub go_to_tab_6: Option<Vec<String>>,
    pub go_to_tab_7: Option<Vec<String>>,
    pub go_to_tab_8: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct KeybindingsConfig {
    #[serde(default)]
    pub global: GlobalKeybindings,
    #[serde(default)]
    pub home: HomeKeybindings,
    #[serde(default)]
    pub navigation: NavigationKeybindings,
}

pub fn parse_key(s: &str) -> Option<(KeyCode, KeyModifiers)> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let parts: Vec<&str> = s.split('-').collect();
    let mut modifiers = KeyModifiers::empty();
    let key_str = if parts.len() > 1 {
        for part in &parts[..parts.len() - 1] {
            match part.to_lowercase().as_str() {
                "ctrl" | "control" => modifiers.insert(KeyModifiers::CONTROL),
                "alt" | "meta" => modifiers.insert(KeyModifiers::ALT),
                "shift" => modifiers.insert(KeyModifiers::SHIFT),
                _ => {}
            }
        }
        parts.last().cloned().unwrap_or("")
    } else {
        s
    };

    let key_lower = key_str.to_lowercase();
    let code = match key_lower.as_str() {
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "enter" | "return" => KeyCode::Enter,
        "esc" | "escape" => KeyCode::Esc,
        "backspace" => KeyCode::Backspace,
        "tab" => KeyCode::Tab,
        "backtab" => KeyCode::BackTab,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" | "pgup" => KeyCode::PageUp,
        "pagedown" | "pgdn" => KeyCode::PageDown,
        "delete" | "del" => KeyCode::Delete,
        "insert" | "ins" => KeyCode::Insert,
        "space" => KeyCode::Char(' '),
        _ => {
            if let Some(c) = key_str.chars().next() {
                if key_str.len() == 1 {
                    KeyCode::Char(c)
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
    };

    Some((code, modifiers))
}

pub fn keys_equal(
    code_a: KeyCode,
    mods_a: KeyModifiers,
    code_b: KeyCode,
    mods_b: KeyModifiers,
) -> bool {
    if let (KeyCode::Char(c_a), KeyCode::Char(c_b)) = (code_a, code_b) {
        if c_a != c_b {
            return false;
        }
        let mask = KeyModifiers::CONTROL | KeyModifiers::ALT;
        return (mods_a & mask) == (mods_b & mask);
    }

    if code_a != code_b {
        return false;
    }

    let mask = KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT;
    (mods_a & mask) == (mods_b & mask)
}

impl KeybindingsConfig {
    pub fn default_config() -> Self {
        Self {
            global: GlobalKeybindings {
                toggle_status_bar: Some(vec![".".to_string()]),
                help: Some(vec!["?".to_string()]),
                close: Some(vec!["esc".to_string(), "q".to_string()]),
            },
            home: HomeKeybindings {
                move_down: Some(vec!["j".to_string(), "down".to_string()]),
                move_up: Some(vec!["k".to_string(), "up".to_string()]),
                page_down: Some(vec!["pagedown".to_string()]),
                page_up: Some(vec!["pageup".to_string()]),
                home: Some(vec!["home".to_string()]),
                end: Some(vec!["end".to_string()]),
                add_repo: Some(vec!["a".to_string()]),
                bulk_add: Some(vec!["A".to_string()]),
                edit_repo: Some(vec!["e".to_string()]),
                delete_repo: Some(vec!["D".to_string()]),
                open_debug_logs: Some(vec!["d".to_string()]),
                edit_labels: Some(vec!["l".to_string()]),
                about: Some(vec!["v".to_string(), "V".to_string()]),
                refresh: Some(vec!["R".to_string()]),
                cycle_sort: Some(vec!["o".to_string()]),
                toggle_sort_reverse: Some(vec!["O".to_string()]),
                toggle_pin: Some(vec!["p".to_string()]),
                open_settings: Some(vec!["s".to_string()]),
                import_repo: Some(vec!["i".to_string()]),
                open_git_app: Some(vec!["g".to_string()]),
                search_repo: Some(vec!["f".to_string()]),
                open_detail: Some(vec!["enter".to_string(), "right".to_string()]),
            },
            navigation: NavigationKeybindings {
                close_detail: Some(vec!["esc".to_string(), "q".to_string(), "Q".to_string()]),
                detail_help: Some(vec!["?".to_string()]),
                cycle_focus_forward: Some(vec!["w".to_string()]),
                cycle_focus_backward: Some(vec!["W".to_string()]),
                refresh_detail: Some(vec!["R".to_string()]),
                cycle_tab_forward: Some(vec!["tab".to_string()]),
                cycle_tab_backward: Some(vec!["backtab".to_string()]),
                go_to_tab_1: Some(vec!["1".to_string()]),
                go_to_tab_2: Some(vec!["2".to_string()]),
                go_to_tab_3: Some(vec!["3".to_string()]),
                go_to_tab_4: Some(vec!["4".to_string()]),
                go_to_tab_5: Some(vec!["5".to_string()]),
                go_to_tab_6: Some(vec!["6".to_string()]),
                go_to_tab_7: Some(vec!["7".to_string()]),
                go_to_tab_8: Some(vec!["8".to_string()]),
            },
        }
    }

    pub fn get_default_keys(action: Action) -> Vec<String> {
        let defaults = Self::default_config();
        defaults.get_action_keys(action)
    }

    pub fn get_action_keys(&self, action: Action) -> Vec<String> {
        let keys_opt = match action {
            // Global
            Action::ToggleStatusBar => self.global.toggle_status_bar.as_ref(),
            Action::Help => self.global.help.as_ref(),
            Action::Close => self.global.close.as_ref(),

            // Home
            Action::HomeMoveDown => self.home.move_down.as_ref(),
            Action::HomeMoveUp => self.home.move_up.as_ref(),
            Action::HomePageDown => self.home.page_down.as_ref(),
            Action::HomePageUp => self.home.page_up.as_ref(),
            Action::HomeHome => self.home.home.as_ref(),
            Action::HomeEnd => self.home.end.as_ref(),
            Action::HomeAddRepo => self.home.add_repo.as_ref(),
            Action::HomeBulkAdd => self.home.bulk_add.as_ref(),
            Action::HomeEditRepo => self.home.edit_repo.as_ref(),
            Action::HomeDeleteRepo => self.home.delete_repo.as_ref(),
            Action::HomeOpenDebugLogs => self.home.open_debug_logs.as_ref(),
            Action::HomeEditLabels => self.home.edit_labels.as_ref(),
            Action::HomeAbout => self.home.about.as_ref(),
            Action::HomeRefresh => self.home.refresh.as_ref(),
            Action::HomeCycleSort => self.home.cycle_sort.as_ref(),
            Action::HomeToggleSortReverse => self.home.toggle_sort_reverse.as_ref(),
            Action::HomeTogglePin => self.home.toggle_pin.as_ref(),
            Action::HomeOpenSettings => self.home.open_settings.as_ref(),
            Action::HomeImportRepo => self.home.import_repo.as_ref(),
            Action::HomeOpenGitApp => self.home.open_git_app.as_ref(),
            Action::HomeSearchRepo => self.home.search_repo.as_ref(),
            Action::HomeOpenDetail => self.home.open_detail.as_ref(),

            // Navigation
            Action::CloseDetail => self.navigation.close_detail.as_ref(),
            Action::DetailHelp => self.navigation.detail_help.as_ref(),
            Action::CycleFocusForward => self.navigation.cycle_focus_forward.as_ref(),
            Action::CycleFocusBackward => self.navigation.cycle_focus_backward.as_ref(),
            Action::RefreshDetail => self.navigation.refresh_detail.as_ref(),
            Action::CycleTabForward => self.navigation.cycle_tab_forward.as_ref(),
            Action::CycleTabBackward => self.navigation.cycle_tab_backward.as_ref(),
            Action::GoToTab1 => self.navigation.go_to_tab_1.as_ref(),
            Action::GoToTab2 => self.navigation.go_to_tab_2.as_ref(),
            Action::GoToTab3 => self.navigation.go_to_tab_3.as_ref(),
            Action::GoToTab4 => self.navigation.go_to_tab_4.as_ref(),
            Action::GoToTab5 => self.navigation.go_to_tab_5.as_ref(),
            Action::GoToTab6 => self.navigation.go_to_tab_6.as_ref(),
            Action::GoToTab7 => self.navigation.go_to_tab_7.as_ref(),
            Action::GoToTab8 => self.navigation.go_to_tab_8.as_ref(),
        };

        keys_opt.cloned().unwrap_or_default()
    }

    pub fn matches(&self, action: Action, key: KeyEvent) -> bool {
        let user_keys = self.get_action_keys(action);
        let mut matched = false;
        let mut has_valid_user_binding = false;

        for key_str in &user_keys {
            if let Some((code, mods)) = parse_key(key_str) {
                has_valid_user_binding = true;
                if keys_equal(key.code, key.modifiers, code, mods) {
                    matched = true;
                }
            }
        }

        if has_valid_user_binding {
            return matched;
        }

        // Fallback to default
        let default_keys = Self::get_default_keys(action);
        for key_str in &default_keys {
            if let Some((code, mods)) = parse_key(key_str) {
                if keys_equal(key.code, key.modifiers, code, mods) {
                    return true;
                }
            }
        }

        false
    }

    pub fn check_conflicts(&self) {
        let mut home_map: HashMap<(KeyCode, KeyModifiers), Vec<Action>> = HashMap::new();
        let home_actions = [
            Action::HomeMoveDown,
            Action::HomeMoveUp,
            Action::HomePageDown,
            Action::HomePageUp,
            Action::HomeHome,
            Action::HomeEnd,
            Action::HomeAddRepo,
            Action::HomeBulkAdd,
            Action::HomeEditRepo,
            Action::HomeDeleteRepo,
            Action::HomeOpenDebugLogs,
            Action::HomeEditLabels,
            Action::HomeAbout,
            Action::HomeRefresh,
            Action::HomeCycleSort,
            Action::HomeToggleSortReverse,
            Action::HomeTogglePin,
            Action::HomeOpenSettings,
            Action::HomeImportRepo,
            Action::HomeOpenGitApp,
            Action::HomeSearchRepo,
            Action::HomeOpenDetail,
        ];

        for action in &home_actions {
            let keys = self.get_action_keys(*action);
            for k in &keys {
                if let Some((code, mods)) = parse_key(k) {
                    home_map.entry((code, mods)).or_default().push(*action);
                }
            }
        }

        for (key, actions) in home_map {
            if actions.len() > 1 {
                crate::debug_log::warn(format!(
                    "Keybind conflict detected for key {:?}: mapped to multiple actions {:?}",
                    key, actions
                ));
            }
        }
    }

    pub fn load(config_dir: &Path) -> Self {
        let keybindings_path = config_dir.join("keybindings.toml");
        if keybindings_path.exists() {
            if let Ok(contents) = std::fs::read_to_string(&keybindings_path) {
                if let Ok(cfg) = toml::from_str::<KeybindingsConfig>(&contents) {
                    cfg.check_conflicts();
                    return cfg;
                }
            }
        }

        let default_cfg = Self::default_config();
        if let Ok(serialized) = toml::to_string_pretty(&default_cfg) {
            let _ = std::fs::write(&keybindings_path, serialized);
        }
        default_cfg
    }
}
