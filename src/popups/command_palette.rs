//! Command palette: a fuzzy-findable list of every action available in the
//! current view, run by name instead of by remembering its key.
//!
//! The palette is an overlay, not a [`Mode`]: it sits on top of the home list
//! or the repository view, owns the keyboard while open, and leaves the view
//! underneath untouched. Running an entry re-dispatches the action through
//! the same `input::handle_key` path its key would take, with
//! `App::forced_action` set so the matching `is_bound` check fires whatever
//! the user has bound the action to. The palette and the keyboard can
//! therefore never disagree about what an action does.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Padding, Paragraph};

use crate::app::{App, Mode};
use crate::keybindings::{Action, parse_key};
use crate::ui::layout::centered_rect;
use crate::ui::style::{ACCENT, CARD_BORDER, accent_style, muted_style, primary_style};

/// Actions offered on the home screen, in the order they are listed before
/// any query is typed.
const HOME_ACTIONS: &[Action] = &[
    Action::HomeOpenDetail,
    Action::HomeAddRepo,
    Action::HomeBulkAdd,
    Action::HomeImportRepo,
    Action::HomeEditRepo,
    Action::HomeDeleteRepo,
    Action::HomeEditLabels,
    Action::HomeLabelPicker,
    Action::HomeSearchRepo,
    Action::HomeJumpPicker,
    Action::HomeGlobalSearch,
    Action::HomeRefresh,
    Action::HomeFetchAll,
    Action::HomeFetchDetails,
    Action::HomeCycleSort,
    Action::HomeToggleSortReverse,
    Action::HomeCycleFilter,
    Action::HomeCycleFilterBack,
    Action::HomeCycleViewMode,
    Action::HomeTogglePin,
    Action::HomeToggleStar,
    Action::HomeSelect,
    Action::HomeYankPath,
    Action::HomeOpenGitApp,
    Action::HomeOpenTerminal,
    Action::HomeOpenExternalShell,
    Action::HomeOpenSettings,
    Action::HomeOpenStatsDashboard,
    Action::HomeCheckUpdate,
    Action::HomeOpenDebugLogs,
    Action::HomeSymbolsHelp,
    Action::HomeAbout,
];

/// Global actions that the home-screen dispatcher handles.
const HOME_GLOBAL_ACTIONS: &[Action] =
    &[Action::Help, Action::ToggleStatusBar, Action::ToggleTerminalPanel, Action::Close];

/// Actions offered in every tab of the repository view.
const DETAIL_ACTIONS: &[Action] = &[
    Action::Overview,
    Action::RefreshDetail,
    Action::CycleTabForward,
    Action::CycleTabBackward,
    Action::ToggleAdvancedTabs,
    Action::GoToTab1,
    Action::GoToTab2,
    Action::GoToTab3,
    Action::GoToTab4,
    Action::GoToTab5,
    Action::GoToTab6,
    Action::GoToTab7,
    Action::CycleFocusForward,
    Action::CycleFocusBackward,
    Action::GrowPanel,
    Action::ShrinkPanel,
    Action::DetailHelp,
    Action::CloseDetail,
];

/// Global actions that the detail dispatcher handles (`Help` is the home
/// overlay; the detail view has `DetailHelp` instead).
const DETAIL_GLOBAL_ACTIONS: &[Action] =
    &[Action::ToggleStatusBar, Action::ToggleTerminalPanel, Action::Close];

/// The actions specific to detail tab `tab`, with the group name shown beside
/// them. Mirrors the `detail_tab` routing in `tabs::route_detail_event`.
fn tab_actions(tab: usize) -> Option<(&'static str, &'static [Action])> {
    Some(match tab {
        0 => (
            "Workspace",
            &[
                Action::WorkspaceCommit,
                Action::WorkspaceCommitAmend,
                Action::WorkspaceStage,
                Action::WorkspaceStageAll,
                Action::WorkspaceDiscard,
                Action::WorkspaceDiscardAll,
                Action::WorkspaceStashUI,
                Action::WorkspaceFuzzySearch,
                Action::WorkspaceColumnPicker,
                Action::WorkspaceLogsView,
                Action::WorkspaceLoadMore,
                Action::WorkspaceCheckout,
                Action::WorkspaceCreateBranch,
                Action::WorkspaceCreateTag,
                Action::WorkspaceYankHash,
                Action::WorkspaceRevert,
                Action::WorkspaceCherryPick,
                Action::WorkspaceInteractiveRebase,
            ],
        ),
        1 => (
            "Files",
            &[
                Action::FilesSearch,
                Action::FilesEditor,
                Action::FilesHistory,
                Action::FilesBlame,
                Action::FilesLineNumbers,
                Action::FilesFullScreen,
                Action::FilesExpand,
                Action::FilesCollapse,
            ],
        ),
        3 => (
            "Branches",
            &[
                Action::BranchesCheckout,
                Action::BranchesCreate,
                Action::BranchesDelete,
                Action::BranchesMerge,
                Action::BranchesMergeInto,
                Action::BranchesRebase,
                Action::BranchesInteractiveRebase,
                Action::BranchesPull,
                Action::BranchesPush,
                Action::BranchesSearch,
            ],
        ),
        4 => (
            "Tags",
            &[
                Action::TagsCheckout,
                Action::TagsDelete,
                Action::TagsPush,
                Action::TagsPushAll,
                Action::TagsFetch,
                Action::TagsSearch,
            ],
        ),
        5 => ("Remotes", &[Action::RemotesAdd, Action::RemotesDelete, Action::RemotesFetch]),
        6 => ("Stashes", &[Action::StashesApply, Action::StashesCreate, Action::StashesDelete]),
        7 => (
            "Worktrees",
            &[
                Action::WorktreesOpen,
                Action::WorktreesAdd,
                Action::WorktreesDelete,
                Action::WorktreesLock,
                Action::WorktreesPrune,
            ],
        ),
        8 => ("Submodules", &[Action::SubmodulesAdd, Action::SubmodulesDelete]),
        9 => ("Reflog", &[Action::ReflogCheckout]),
        10 | 11 => (
            "Forge",
            &[
                Action::ForgeCheckout,
                Action::ForgeOpenBrowser,
                Action::ForgeToggleAssigned,
                Action::ForgeAddComment,
            ],
        ),
        _ => return None,
    })
}

/// One runnable row of the palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteEntry {
    pub action: Action,
    /// The context the action belongs to (`Home`, `Branches`, `Global`, …).
    pub group: &'static str,
    /// The action's description from the keybindings config.
    pub label: String,
    /// The action's current keys, formatted for display. `None` only if an
    /// action has no default keys at all (an empty custom list falls back to
    /// the defaults, so in practice every action is bound).
    pub keys: Option<String>,
}

impl PaletteEntry {
    fn new(app: &App, group: &'static str, action: Action) -> Self {
        let label = app.keybindings.get_action_description(action);
        let keys = app.keybindings.format_action_keys(action, app.config.compatibility_mode);
        let keys = if keys == "-" { None } else { Some(keys) };
        Self { action, group, label, keys }
    }
}

/// The open palette: its entries (fixed at open time), the typed query and
/// the highlighted match.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandPalette {
    pub query: String,
    pub selection: usize,
    pub entries: Vec<PaletteEntry>,
}

impl CommandPalette {
    /// Builds the palette for whatever `app` is showing: the home actions on
    /// the home screen, or the repository actions plus the active tab's own
    /// actions in the detail view, each followed by the global actions that
    /// dispatcher handles.
    pub fn for_context(app: &App) -> Self {
        let mut entries = Vec::new();
        let mut push = |group: &'static str, actions: &[Action]| {
            entries.extend(actions.iter().map(|&a| PaletteEntry::new(app, group, a)));
        };
        match app.mode {
            Mode::Detail => {
                if let Some((group, actions)) = tab_actions(app.detail_tab) {
                    push(group, actions);
                }
                push("Repository", DETAIL_ACTIONS);
                push("Global", DETAIL_GLOBAL_ACTIONS);
            }
            _ => {
                push("Home", HOME_ACTIONS);
                push("Global", HOME_GLOBAL_ACTIONS);
            }
        }
        Self { query: String::new(), selection: 0, entries }
    }

    /// Indices of the entries matching the query, best match first. An empty
    /// query lists everything in context order.
    pub fn matches(&self) -> Vec<usize> {
        let query = self.query.trim().to_lowercase();
        if query.is_empty() {
            return (0..self.entries.len()).collect();
        }
        let mut scored: Vec<(i32, usize)> = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| score(e, &query).map(|s| (s, i)))
            .collect();
        // Stable: equal scores keep context order.
        scored.sort_by_key(|&(score, _)| std::cmp::Reverse(score));
        scored.into_iter().map(|(_, i)| i).collect()
    }

    /// The highlighted entry, if any match exists.
    pub fn selected(&self) -> Option<&PaletteEntry> {
        self.matches().get(self.selection).and_then(|&i| self.entries.get(i))
    }
}

/// How well `entry` matches `query` (already lower-cased): a label prefix
/// beats a label substring, which beats a group-qualified substring, which
/// beats a scattered subsequence of the label. `None` when it does not match.
fn score(entry: &PaletteEntry, query: &str) -> Option<i32> {
    let label = entry.label.to_lowercase();
    let group = entry.group.to_lowercase();
    if label.starts_with(query) {
        return Some(400 - label.len() as i32);
    }
    if let Some(pos) = label.find(query) {
        return Some(300 - pos as i32);
    }
    if group.contains(query) || format!("{} {}", group, label).contains(query) {
        return Some(200);
    }
    let mut chars = label.chars();
    if query.chars().all(|q| chars.any(|c| c == q)) {
        return Some(100 - label.len() as i32);
    }
    None
}

impl App {
    /// Opens the command palette over the current view. Only the home screen
    /// and the repository view have one; elsewhere the key just says so.
    pub fn open_command_palette(&mut self) {
        if !matches!(self.mode, Mode::Normal | Mode::Detail) {
            self.status_message = Some(
                "The command palette is available on the home screen and in the repository view"
                    .to_string(),
            );
            return;
        }
        self.command_palette = Some(CommandPalette::for_context(self));
    }

    pub fn close_command_palette(&mut self) {
        self.command_palette = None;
    }

    /// Runs `action` exactly as pressing its key would, by sending the key
    /// through the normal dispatcher with `forced_action` set so only this
    /// action's `is_bound` check matches. A binding the key parser rejects
    /// still runs: the dispatcher then sees a null key that no raw match can
    /// claim, and the forced check does the routing. Returns `false` when the
    /// action quit the app.
    pub fn run_action(&mut self, action: Action, visible_count: usize) -> bool {
        let key = self
            .keybindings
            .get_action_keys(action)
            .first()
            .and_then(|k| parse_key(k))
            .map(|(code, mods)| KeyEvent::new(code, mods))
            .unwrap_or_else(|| KeyEvent::new(KeyCode::Null, KeyModifiers::empty()));
        self.forced_action = Some(action);
        let keep_running = crate::input::handle_key(self, key, visible_count);
        self.forced_action = None;
        keep_running
    }
}

pub struct CommandPalettePopup;

impl CommandPalettePopup {
    /// Handles a key while the palette is open. Returns `false` only when the
    /// action that was run quit the app.
    pub fn handle_event(app: &mut App, key: KeyEvent, visible_count: usize) -> bool {
        if app.is_bound(Action::CommandPalette, key) {
            app.close_command_palette();
            return true;
        }
        let page = app.config.page_size.max(1);
        let Some(palette) = app.command_palette.as_mut() else {
            return true;
        };
        let count = palette.matches().len();
        let last = count.saturating_sub(1);
        match key.code {
            KeyCode::Esc => app.close_command_palette(),
            KeyCode::Up => palette.selection = palette.selection.saturating_sub(1),
            KeyCode::Down => palette.selection = (palette.selection + 1).min(last),
            KeyCode::PageUp => palette.selection = palette.selection.saturating_sub(page),
            KeyCode::PageDown => palette.selection = (palette.selection + page).min(last),
            KeyCode::Home => palette.selection = 0,
            KeyCode::End => palette.selection = last,
            KeyCode::Backspace => {
                palette.query.pop();
                palette.selection = 0;
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                palette.query.push(c);
                palette.selection = 0;
            }
            KeyCode::Enter => {
                let action = palette.selected().map(|e| e.action);
                app.close_command_palette();
                if let Some(action) = action {
                    return app.run_action(action, visible_count);
                }
            }
            _ => {}
        }
        true
    }

    pub fn draw(f: &mut Frame, app: &App, area: Rect) {
        let Some(palette) = app.command_palette.as_ref() else {
            return;
        };
        let popup_area = centered_rect(60, 60, area);
        f.render_widget(Clear, popup_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(Style::default().fg(ACCENT()))
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Command Palette", primary_style().add_modifier(Modifier::BOLD)),
                Span::raw(" "),
            ]))
            .padding(Padding::horizontal(1));
        let inner = block.inner(popup_area);
        f.render_widget(block, popup_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1), Constraint::Length(1)])
            .split(inner);

        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_style(muted_style())
            .title(" Search Actions ");
        let input = Paragraph::new(Line::from(vec![
            Span::raw("> "),
            Span::styled(palette.query.clone(), primary_style()),
        ]))
        .block(input_block);
        f.render_widget(input, chunks[0]);

        let matches = palette.matches();
        if matches.is_empty() {
            let empty = Paragraph::new(Line::from(Span::styled(
                "No actions match — try another word",
                muted_style(),
            )));
            f.render_widget(empty, chunks[1]);
        } else {
            let width = chunks[1].width as usize;
            let group_width = palette.entries.iter().map(|e| e.group.len()).max().unwrap_or(0) + 2;
            let items: Vec<ListItem> = matches
                .iter()
                .enumerate()
                .filter_map(|(row, &idx)| palette.entries.get(idx).map(|e| (row, e)))
                .map(|(row, entry)| {
                    let keys = entry.keys.clone().unwrap_or_else(|| "no key".to_string());
                    let used = group_width + entry.label.chars().count() + keys.chars().count();
                    let pad = width.saturating_sub(used).max(2);
                    let label_style = if row == palette.selection {
                        accent_style().add_modifier(Modifier::BOLD | Modifier::REVERSED)
                    } else {
                        primary_style()
                    };
                    let keys_style = if entry.keys.is_some() {
                        accent_style().add_modifier(Modifier::BOLD)
                    } else {
                        muted_style()
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("{:<w$}", entry.group, w = group_width),
                            muted_style(),
                        ),
                        Span::styled(entry.label.clone(), label_style),
                        Span::raw(" ".repeat(pad)),
                        Span::styled(keys, keys_style),
                    ]))
                })
                .collect();
            let mut state = ListState::default();
            state.select(Some(palette.selection.min(matches.len() - 1)));
            f.render_stateful_widget(List::new(items), chunks[1], &mut state);
        }

        let hint = Line::from(vec![
            Span::styled("Type to filter  ", muted_style()),
            Span::styled("↑↓ navigate  ", muted_style()),
            Span::styled("Enter", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" run  ", muted_style()),
            Span::styled("Esc", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" cancel", muted_style()),
        ]);
        f.render_widget(Paragraph::new(hint), chunks[2]);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn entry(group: &'static str, label: &str) -> PaletteEntry {
        PaletteEntry { action: Action::Help, group, label: label.to_string(), keys: None }
    }

    #[test]
    fn matching_ranks_prefix_over_substring_over_group_over_subsequence() {
        let palette = CommandPalette {
            query: "push".to_string(),
            selection: 0,
            entries: vec![
                entry("Branches", "Pull the current branch"),
                entry("Tags", "Force push all tags"),
                entry("Branches", "Push the current branch"),
                entry("Home", "Pin under sort hue"), // p-u-s-h as a subsequence
                entry("Global", "Quit"),
            ],
        };
        let ranked: Vec<&str> =
            palette.matches().into_iter().map(|i| palette.entries[i].label.as_str()).collect();
        assert_eq!(
            ranked,
            vec!["Push the current branch", "Force push all tags", "Pin under sort hue"]
        );

        let by_group = CommandPalette { query: "branches".to_string(), ..palette.clone() };
        let ranked: Vec<&str> =
            by_group.matches().into_iter().map(|i| by_group.entries[i].group).collect();
        assert_eq!(ranked, vec!["Branches", "Branches"]);

        let none = CommandPalette { query: "zzz".to_string(), ..palette.clone() };
        assert!(none.matches().is_empty());
        assert!(none.selected().is_none());

        let all = CommandPalette { query: "  ".to_string(), ..palette };
        assert_eq!(all.matches(), vec![0, 1, 2, 3, 4]);
        assert_eq!(all.selected().map(|e| e.label.as_str()), Some("Pull the current branch"));
    }

    #[test]
    fn draw_renders_query_entries_keys_and_highlight() {
        let config = crate::config::Config {
            items: vec!["/path/to/repo".to_string()],
            ..Default::default()
        };
        let mut app = App::new(config, std::path::PathBuf::from("dummy_palette.toml"));
        app.open_command_palette();
        let palette = app.command_palette.as_mut().unwrap();
        palette.query = "sett".to_string();
        let expected = app.keybindings.get_action_description(Action::HomeOpenSettings);

        let backend = ratatui::backend::TestBackend::new(100, 30);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|f| CommandPalettePopup::draw(f, &app, Rect::new(0, 0, 100, 30))).unwrap();

        let buffer = terminal.backend().buffer();
        let rows: Vec<String> = (0..30)
            .map(|y| (0..100).map(|x| buffer[(x, y)].symbol().to_string()).collect::<String>())
            .collect();
        let screen = rows.join("\n");
        assert!(screen.contains("Command Palette"), "screen:\n{}", screen);
        assert!(screen.contains("> sett"), "screen:\n{}", screen);
        assert!(screen.contains(&expected), "screen:\n{}", screen);
        assert!(screen.contains("Enter"), "screen:\n{}", screen);

        // The top match is highlighted and carries its key at the row's end.
        let (y, row) = rows.iter().enumerate().find(|(_, r)| r.contains(&expected)).unwrap();
        assert!(row.contains("Home"), "row: {}", row);
        let key_at = row.rfind(" s ").expect("key column present");
        assert!(key_at > row.find(&expected).unwrap(), "row: {}", row);
        let x = row.find(&expected).unwrap() as u16;
        assert!(buffer[(x, y as u16)].style().add_modifier.contains(Modifier::REVERSED));

        // No match: the list gives way to a hint instead of an empty box.
        app.command_palette.as_mut().unwrap().query = "zzz".to_string();
        terminal.draw(|f| CommandPalettePopup::draw(f, &app, Rect::new(0, 0, 100, 30))).unwrap();
        let buffer = terminal.backend().buffer();
        let screen: String = (0..30)
            .map(|y| (0..100).map(|x| buffer[(x, y)].symbol().to_string()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(screen.contains("No actions match"), "screen:\n{}", screen);

        // Nothing is drawn when the palette is closed.
        app.close_command_palette();
        terminal.draw(|f| CommandPalettePopup::draw(f, &app, Rect::new(0, 0, 100, 30))).unwrap();
        let buffer = terminal.backend().buffer();
        let screen: String = (0..30)
            .map(|y| (0..100).map(|x| buffer[(x, y)].symbol().to_string()).collect::<String>())
            .collect();
        assert!(!screen.contains("Command Palette"));
    }

    #[test]
    fn every_tab_group_is_covered_or_deliberately_empty() {
        // Graph (2) has no tab-specific actions; every other routed tab does.
        assert!(tab_actions(2).is_none());
        for tab in [0, 1, 3, 4, 5, 6, 7, 8, 9, 10, 11] {
            let (group, actions) = tab_actions(tab).unwrap();
            assert!(!group.is_empty());
            assert!(!actions.is_empty(), "tab {} lists no actions", tab);
        }
        assert!(tab_actions(12).is_none());
    }
}
