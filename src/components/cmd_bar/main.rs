//! Normal dashboard, settings, and main views status bar entry generation.

use super::StatusEntry;
use crate::app::{App, Mode};
use crate::config::SortOrder;
use crate::ui::style::{accent_style, muted_style, primary_style};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

pub(crate) fn normal_status_entries(app: &App) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let mut message_spans = None;
    if let Some(msg) = &app.status_message {
        message_spans = Some(vec![Span::styled(format!("{} ", msg), accent_style())]);
    } else if let Some(query) = &app.repo_search_query {
        message_spans = Some(vec![
            Span::styled("Filtered by: ", muted_style()),
            Span::styled(format!("\"{}\" ", query), accent_style()),
            Span::styled("(Esc to clear) ", muted_style()),
        ]);
    }
    let sort_label = match app.config.sort_by {
        SortOrder::Custom => "Custom",
        SortOrder::Alphabetical => "Alphabetical",
        SortOrder::RecentVisit => "Recent",
        SortOrder::LatestChanges => "Changes",
    };
    let sort_dir = if app.config.sort_reverse { " (Rev)" } else { "" };
    let sort_key_label = format!("Sort: {}{}", sort_label, sort_dir);

    let compat = app.config.compatibility_mode;
    let kb = &app.keybindings;
    let k = |a| kb.format_action_keys(a, compat);

    let detail_key = k(crate::keybindings::Action::HomeOpenDetail);
    let git_app_key = k(crate::keybindings::Action::HomeOpenGitApp);
    let terminal_key = k(crate::keybindings::Action::HomeOpenTerminal);
    let sort_key = format!(
        "{}/{}",
        k(crate::keybindings::Action::HomeCycleSort),
        k(crate::keybindings::Action::HomeToggleSortReverse)
    );
    let search_key = k(crate::keybindings::Action::HomeSearchRepo);
    let jump_key = k(crate::keybindings::Action::HomeJumpPicker);
    let add_key = k(crate::keybindings::Action::HomeAddRepo);
    let bulk_add_key = k(crate::keybindings::Action::HomeBulkAdd);
    let import_key = k(crate::keybindings::Action::HomeImportRepo);
    let edit_key = k(crate::keybindings::Action::HomeEditRepo);
    let delete_key = k(crate::keybindings::Action::HomeDeleteRepo);
    let labels_key = k(crate::keybindings::Action::HomeEditLabels);
    let refresh_key = k(crate::keybindings::Action::HomeRefresh);
    let fetch_key = k(crate::keybindings::Action::HomeFetchAll);
    let select_key = k(crate::keybindings::Action::HomeSelect);
    let pin_key = k(crate::keybindings::Action::HomeTogglePin);
    let star_key = k(crate::keybindings::Action::HomeToggleStar);
    let yank_key = k(crate::keybindings::Action::HomeYankPath);
    let update_key = k(crate::keybindings::Action::HomeCheckUpdate);
    let debug_key = k(crate::keybindings::Action::HomeOpenDebugLogs);
    let about_key = k(crate::keybindings::Action::HomeAbout);
    let compact_key = k(crate::keybindings::Action::HomeToggleCompactView);
    let legend_key = k(crate::keybindings::Action::HomeSymbolsHelp);
    let help_key = k(crate::keybindings::Action::Help);
    let quit_key = k(crate::keybindings::Action::Close);

    let entries_data = vec![
        ("Navigate", "↑↓"),
        ("Page", "⇟/⇞"),
        ("Jump", "Home/End"),
        ("Detail", &detail_key),
        (&app.config.git_app, &git_app_key),
        ("Terminal", &terminal_key),
        (&sort_key_label, &sort_key),
        ("Find", &search_key),
        ("Jump Picker", &jump_key),
        ("Add", &add_key),
        ("Bulk Add", &bulk_add_key),
        ("Import", &import_key),
        ("Edit", &edit_key),
        ("Delete", &delete_key),
        ("Labels", &labels_key),
        ("Refresh", &refresh_key),
        ("Fetch All", &fetch_key),
        ("Select", &select_key),
        ("Pin", &pin_key),
        ("Star", &star_key),
        ("Yank Path", &yank_key),
        ("Check Update", &update_key),
        ("Debug Logs", &debug_key),
        ("About", &about_key),
        ("Compact", &compact_key),
        ("Legend", &legend_key),
        ("Help", &help_key),
        ("Quit", &quit_key),
    ];
    let mut entries = Vec::new();
    for (i, (label, key)) in entries_data.iter().enumerate() {
        let mut spans = Vec::new();
        if i > 0 {
            spans.push(Span::styled(" ", muted_style()));
        }
        spans.push(Span::raw((*label).to_string()));
        spans.push(Span::raw(" "));
        spans.push(Span::styled("[", muted_style()));
        spans.push(Span::styled((*key).to_string(), accent_style()));
        spans.push(Span::styled("]", muted_style()));
        entries.push(StatusEntry::new(spans));
    }
    (message_spans, entries)
}
