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
    // The label filter is sticky (Esc does not clear it), so keep a permanent
    // chip visible whenever it is active.
    if let Some(label) = &app.config.active_label_filter {
        let label_key = app.keybindings.format_action_keys(
            crate::keybindings::Action::HomeLabelPicker,
            app.config.compatibility_mode,
        );
        let mut spans = vec![
            Span::styled("Label: ", muted_style()),
            Span::styled(format!("{} ", label), accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(format!("({} to change) ", label_key), muted_style()),
        ];
        if let Some(existing) = message_spans.take() {
            spans.extend(existing);
        }
        message_spans = Some(spans);
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
    let cycle_filter_key = k(crate::keybindings::Action::HomeCycleFilter);
    let label_picker_key = k(crate::keybindings::Action::HomeLabelPicker);
    let jump_key = k(crate::keybindings::Action::HomeJumpPicker);
    let add_key = k(crate::keybindings::Action::HomeAddRepo);
    let bulk_add_key = k(crate::keybindings::Action::HomeBulkAdd);
    let import_key = k(crate::keybindings::Action::HomeImportRepo);
    let edit_key = k(crate::keybindings::Action::HomeEditRepo);
    let delete_key = k(crate::keybindings::Action::HomeDeleteRepo);
    let labels_key = k(crate::keybindings::Action::HomeEditLabels);
    let refresh_key = k(crate::keybindings::Action::HomeRefresh);
    let fetch_key = k(crate::keybindings::Action::HomeFetchAll);
    let fetch_details_key = k(crate::keybindings::Action::HomeFetchDetails);
    let select_key = k(crate::keybindings::Action::HomeSelect);
    let pin_key = k(crate::keybindings::Action::HomeTogglePin);
    let star_key = k(crate::keybindings::Action::HomeToggleStar);
    let yank_key = k(crate::keybindings::Action::HomeYankPath);
    let update_key = k(crate::keybindings::Action::HomeCheckUpdate);
    let debug_key = k(crate::keybindings::Action::HomeOpenDebugLogs);
    let about_key = k(crate::keybindings::Action::HomeAbout);
    let compact_key = k(crate::keybindings::Action::HomeCycleViewMode);
    let settings_key = k(crate::keybindings::Action::HomeOpenSettings);
    let global_search_key = k(crate::keybindings::Action::HomeGlobalSearch);
    let stats_key = k(crate::keybindings::Action::HomeOpenStatsDashboard);
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
        ("Code Search", &global_search_key),
        ("Filter", &cycle_filter_key),
        ("Label Filter", &label_picker_key),
        ("Jump Picker", &jump_key),
        ("Add", &add_key),
        ("Bulk Add", &bulk_add_key),
        ("Import", &import_key),
        ("Edit", &edit_key),
        ("Delete", &delete_key),
        ("Labels", &labels_key),
        ("Refresh", &refresh_key),
        ("Fetch All", &fetch_key),
        ("Fetch Error", &fetch_details_key),
        ("Select", &select_key),
        ("Pin", &pin_key),
        ("Star", &star_key),
        ("Yank Path", &yank_key),
        ("Check Update", &update_key),
        ("Settings", &settings_key),
        ("Debug Logs", &debug_key),
        ("About", &about_key),
        ("Stats", &stats_key),
        ("View Mode", &compact_key),
        ("Legend", &legend_key),
        ("Help", &help_key),
        ("Quit", &quit_key),
    ];
    let entries = super::build_status_entries(&entries_data);
    (message_spans, entries)
}
