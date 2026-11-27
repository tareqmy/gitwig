//! Home screen repository list, summary bar, quick-label strip, and status indicator rendering.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Padding, Paragraph};

use super::*;
use crate::app::{App, Mode};
use crate::repo::{ItemStatus, RepoState, RepoSummary, format_relative_time};

/// Width of the per-item status zone on the right of each card. Wide
/// enough to fit a busy repo's worth of indicators ("● 99+ 99! 99? 99↑")
/// with a little breathing room. Right-aligned, so it crops on the left
/// for the unusual case of 3-digit counts on every indicator.
pub(super) const STATUS_ZONE_WIDTH: u16 = 22;

pub(super) const UNSELECTED_INDENT: &str = "  ";

/// Within the content area, split into N item rows + a flex spacer so the
/// list is top-aligned and never pushes against the status bar. Returns one
/// rect per visible row, and the one-line rects of the rules that close a
/// pinned block ([`App::home_pin_separators`]); the visible-row count in the
/// main loop reserves the same lines.
pub(super) fn item_chunks(
    content_area: Rect,
    visible_count: usize,
    app: &App,
) -> (Vec<Rect>, Vec<Rect>) {
    let rows = app.get_home_rows();
    let separators = app.home_pin_separators(&rows);
    let upper = (app.scroll_top + visible_count).min(rows.len());
    let visible_rows = &rows[app.scroll_top..upper];
    let visible_separators = &separators[app.scroll_top..upper];

    if app.effective_view_mode() != crate::config::HomeViewMode::Tile {
        let mut constraints = Vec::new();
        for (row, &separator) in visible_rows.iter().zip(visible_separators) {
            let h = match row {
                crate::app::HomeRow::GroupHeader { .. } => {
                    if app.effective_view_mode() == crate::config::HomeViewMode::Compact {
                        1
                    } else {
                        2
                    }
                }
                crate::app::HomeRow::Repo { path, .. } => {
                    if app.effective_view_mode() == crate::config::HomeViewMode::Compact {
                        1
                    } else {
                        let has_note = app
                            .config
                            .repo_configs
                            .get(path)
                            .and_then(|cfg| cfg.note.as_ref())
                            .is_some();
                        if has_note { 5 } else { 4 }
                    }
                }
            };
            constraints.push(Constraint::Length(h));
            if separator {
                constraints.push(Constraint::Length(1));
            }
        }
        constraints.push(Constraint::Min(0));

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(content_area);
        let mut rects = Vec::with_capacity(visible_rows.len());
        let mut rules = Vec::new();
        let mut next = 0;
        for &separator in visible_separators {
            rects.push(chunks[next]);
            next += 1;
            if separator {
                rules.push(chunks[next]);
                next += 1;
            }
        }
        return (rects, rules);
    }

    let cols = app.get_tile_cols();
    let mut final_rects = Vec::with_capacity(visible_rows.len());
    let mut rules = Vec::new();

    let mut row_heights = Vec::new();
    let mut current_row_items = 0;

    for (row, &separator) in visible_rows.iter().zip(visible_separators) {
        match row {
            crate::app::HomeRow::GroupHeader { .. } => {
                if current_row_items > 0 {
                    row_heights.push(4);
                    current_row_items = 0;
                }
                row_heights.push(2);
            }
            crate::app::HomeRow::Repo { .. } => {
                current_row_items += 1;
                if separator {
                    // The rule spans the grid: the pinned block's last tile
                    // ends its grid row, and the rule takes the line below.
                    row_heights.push(4);
                    row_heights.push(1);
                    current_row_items = 0;
                } else if current_row_items == cols {
                    row_heights.push(4);
                    current_row_items = 0;
                }
            }
        }
    }
    if current_row_items > 0 {
        row_heights.push(4);
    }
    row_heights.push(0);

    let constraints: Vec<Constraint> = row_heights
        .iter()
        .map(|&h| if h == 0 { Constraint::Min(0) } else { Constraint::Length(h as u16) })
        .collect();

    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(content_area);

    let mut row_idx = 0;
    let mut current_col = 0;

    let tile_constraints: Vec<Constraint> =
        (0..cols).map(|_| Constraint::Ratio(1, cols as u32)).collect();

    for (row, &separator) in visible_rows.iter().zip(visible_separators) {
        match row {
            crate::app::HomeRow::GroupHeader { .. } => {
                if current_col > 0 {
                    row_idx += 1;
                    current_col = 0;
                }
                final_rects.push(vertical_chunks[row_idx]);
                row_idx += 1;
            }
            crate::app::HomeRow::Repo { .. } => {
                let row_rect = vertical_chunks[row_idx];
                let horizontal_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(tile_constraints.clone())
                    .split(row_rect);

                final_rects.push(horizontal_chunks[current_col]);

                current_col += 1;
                if separator {
                    current_col = 0;
                    row_idx += 1;
                    rules.push(vertical_chunks[row_idx]);
                    row_idx += 1;
                } else if current_col == cols {
                    current_col = 0;
                    row_idx += 1;
                }
            }
        }
    }

    (final_rects, rules)
}

/// Faint rules under the rows that close a pinned block, so the pinned
/// repositories the sort keeps at the top read as their own group. Lighter
/// than the header rules on purpose: the rule glyph in every other cell,
/// dimmed, indented to the card text and stopping short of the right edge.
pub(super) fn draw_pin_rules(f: &mut Frame, app: &App, rules: &[Rect]) {
    let indent = UNSELECTED_INDENT.len() as u16 + 1;
    for rect in rules {
        let width = rect.width.saturating_sub(indent * 2) as usize;
        if width == 0 {
            continue;
        }
        let dash = app.sym("rule");
        let rule: String = (0..width).map(|i| if i % 2 == 0 { dash } else { " " }).collect();
        let area = Rect::new(rect.x + indent, rect.y, width as u16, 1);
        f.render_widget(Paragraph::new(Line::from(Span::styled(rule, rule_style()))), area);
    }
}

/// Aggregate counts backing the home summary tab bar.
pub struct SummaryCounts {
    pub total_repos: usize,
    pub visible: usize,
    pub dirty: usize,
    pub ahead: usize,
    pub stale: usize,
    pub archived: usize,
}

pub fn summary_counts(app: &App) -> SummaryCounts {
    let mut counts =
        SummaryCounts { total_repos: 0, visible: 0, dirty: 0, ahead: 0, stale: 0, archived: 0 };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let stale_threshold = (app.config.stale_threshold_months as i64) * 30 * 24 * 60 * 60;

    // Scope the dashboard to the active label filter so the tabs describe the
    // selected project, not the whole workspace.
    let label_ok = |idx: usize| match &app.state.active_label_filter {
        Some(label) => app
            .config
            .items
            .get(idx)
            .and_then(|path| app.config.labels.get(path))
            .is_some_and(|lbls| lbls.contains(label)),
        None => true,
    };

    for (idx, status) in app.statuses.iter().enumerate() {
        if !label_ok(idx) {
            continue;
        }
        if app.config.items.get(idx).is_some_and(|p| app.config.archived.contains(p)) {
            counts.archived += 1;
            continue;
        }
        match status {
            ItemStatus::GitRepo(Some(summary)) => {
                counts.total_repos += 1;
                if !summary.is_clean() {
                    counts.dirty += 1;
                }
                if summary.ahead > 0 {
                    counts.ahead += 1;
                }
                if let Some(t) = summary.last_commit_time {
                    if now - t > stale_threshold {
                        counts.stale += 1;
                    }
                }
            }
            ItemStatus::GitRepo(None) => {
                counts.total_repos += 1;
            }
            _ => {}
        }
    }

    counts.visible = app.get_filtered_items().len();
    counts
}

/// Divider drawn between summary tabs; its width is part of the click math.
pub const SUMMARY_TAB_DIVIDER: &str = "│";

/// Badge naming the sticky label filter (` ● label `), set into the outer
/// frame's top border where the sort caption used to sit; `None` when no
/// label filter is active.
pub fn label_badge_text(app: &App) -> Option<String> {
    app.state
        .active_label_filter
        .as_ref()
        .map(|label| format!(" {} {} ", app.sym("bullet_filled"), label))
}

/// Draws the active-label badge centered on the frame's top border and
/// records its rect in `app.label_badge_area` so a click on it reopens the
/// label picker. Called only while the home header is showing.
pub(super) fn draw_label_badge(f: &mut Frame, area: Rect, app: &App) {
    let Some(text) = label_badge_text(app) else {
        app.label_badge_area.set(None);
        return;
    };
    // Keep off the frame corners; center the badge on the border line.
    let usable = area.width.saturating_sub(2);
    let width = (text.chars().count() as u16).min(usable);
    let x = area.x + 1 + (usable - width) / 2;
    let badge = Rect::new(x, area.y, width, 1);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            text,
            accent_style().add_modifier(Modifier::REVERSED | Modifier::BOLD),
        ))),
        badge,
    );
    app.label_badge_area.set(Some(badge));
}

/// Caption of each summary tab as (count, label) halves, in click order:
/// repos, dirty, ahead, stale, archived. Mouse hit-testing in `mouse.rs`
/// measures these exact strings, so the rendered tabs and the click zones
/// cannot drift apart.
pub fn summary_tab_parts(app: &App, counts: &SummaryCounts) -> [(String, String); 5] {
    let repos_count = if counts.visible < counts.total_repos {
        format!(" {}/{} ", counts.visible, counts.total_repos)
    } else {
        format!(" {} ", counts.total_repos)
    };
    let stale_label = if !app.config.show_stale_projects && counts.stale > 0 {
        "stale (hidden)"
    } else {
        "stale"
    };
    [
        (repos_count, "repos ".to_string()),
        (format!(" {} ", counts.dirty), "dirty ".to_string()),
        (format!(" {} ", counts.ahead), "ahead ".to_string()),
        (format!(" {} ", counts.stale), format!("{} ", stale_label)),
        (format!(" {} ", counts.archived), "archived ".to_string()),
    ]
}

pub(super) fn draw_global_summary_bar(f: &mut Frame, area: Rect, app: &App) {
    let counts = summary_counts(app);
    let parts = summary_tab_parts(app, &counts);

    let colors = [
        ACCENT(),
        if counts.dirty > 0 { WARNING() } else { SUCCESS() },
        if counts.ahead > 0 { ACCENT() } else { SUCCESS() },
        if counts.stale > 0 { DANGER() } else { SUCCESS() },
        if counts.archived > 0 { ACCENT() } else { SUCCESS() },
    ];
    let active = [
        app.global_filter.is_none(),
        app.global_filter == Some(crate::app::GlobalFilter::Dirty),
        app.global_filter == Some(crate::app::GlobalFilter::Ahead),
        app.global_filter == Some(crate::app::GlobalFilter::Stale),
        app.global_filter == Some(crate::app::GlobalFilter::Archived),
    ];

    let mut spans = Vec::new();
    for (i, (count_text, label_text)) in parts.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(SUMMARY_TAB_DIVIDER, muted_style()));
        }
        if active[i] {
            let tab_style =
                primary_style().fg(colors[i]).add_modifier(Modifier::REVERSED | Modifier::BOLD);
            spans.push(Span::styled(count_text.clone(), tab_style));
            spans.push(Span::styled(label_text.clone(), tab_style));
        } else {
            spans.push(Span::styled(count_text.clone(), primary_style().fg(colors[i])));
            spans.push(Span::styled(label_text.clone(), muted_style()));
        }
    }

    let line = Line::from(spans).alignment(Alignment::Center);
    f.render_widget(Paragraph::new(line), area);
}

/// Divider drawn between quick-label chips (the same glyph as the summary
/// tabs, so the two header rows read as one unit); part of the click math.
pub const QUICK_LABEL_DIVIDER: &str = SUMMARY_TAB_DIVIDER;

/// Caption of each quick-label chip as (key, name) halves, in slot order.
/// Mouse hit-testing in `mouse.rs` measures these exact strings, so the
/// rendered chips and the click zones cannot drift apart.
pub fn quick_label_parts(app: &App) -> Vec<(String, String)> {
    app.quick_labels()
        .into_iter()
        .enumerate()
        .map(|(slot, (label, _))| {
            let key = crate::keybindings::Action::home_label_slot(slot)
                .map(|a| app.keybindings.format_action_keys(a, app.config.compatibility_mode))
                .unwrap_or_default();
            (format!(" {} ", key), format!("{} ", label))
        })
        .collect()
}

/// Home header rows: the summary bar, the quick-label chip strip (only once
/// a label has been viewed, set off from the summary bar by a thin rule
/// unless `show_separators` is off), the closing rule that carries the sort
/// caption, and the body below it.
pub(super) struct HomeHeaderLayout {
    pub(super) summary: Rect,
    pub(super) chips: Option<Rect>,
    pub(super) sort_rule: Rect,
    pub(super) body: Rect,
}

pub(super) fn home_header_layout(area: Rect, app: &App) -> HomeHeaderLayout {
    if app.quick_labels().is_empty() {
        let parts = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Global summary bar
                Constraint::Length(1), // Closing rule with the sort caption
                Constraint::Min(0),    // Body
            ])
            .split(area);
        HomeHeaderLayout { summary: parts[0], chips: None, sort_rule: parts[1], body: parts[2] }
    } else if !app.config.show_separators {
        let parts = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Global summary bar
                Constraint::Length(1), // Quick-label chips
                Constraint::Length(1), // Sort caption
                Constraint::Min(0),    // Body
            ])
            .split(area);
        HomeHeaderLayout {
            summary: parts[0],
            chips: Some(parts[1]),
            sort_rule: parts[2],
            body: parts[3],
        }
    } else {
        let parts = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Global summary bar
                Constraint::Length(1), // Rule between summary and chips
                Constraint::Length(1), // Quick-label chips
                Constraint::Length(1), // Closing rule with the sort caption
                Constraint::Min(0),    // Body
            ])
            .split(area);
        HomeHeaderLayout {
            summary: parts[0],
            chips: Some(parts[2]),
            sort_rule: parts[3],
            body: parts[4],
        }
    }
}

/// Caption for the active list sort, e.g. `Sort: Alphabetical (Rev)`. When the
/// active label filter overrides the sort, the label is named so it is clear
/// the caption (and the `o`/`O` keys) refer to that label's setting.
pub fn sort_caption(app: &App) -> String {
    let mut caption = format!("Sort: {}", app.effective_sort_by().display_name());
    if app.effective_sort_reverse() {
        caption.push_str(" (Rev)");
    }
    if let Some(label) = app.sort_override_label() {
        caption.push_str(&format!(" · {}", label));
    }
    caption
}

/// Draws every header row and records the clickable rects for `mouse.rs`.
pub(super) fn draw_home_header(
    f: &mut Frame,
    header: &HomeHeaderLayout,
    app: &App,
    global_summary_area: &mut Option<Rect>,
    quick_label_area: &mut Option<Rect>,
) {
    draw_global_summary_bar(f, header.summary, app);
    *global_summary_area = Some(header.summary);
    if let Some(chips) = header.chips {
        if app.config.show_separators {
            draw_header_rule(f, Rect::new(chips.x, chips.y.saturating_sub(1), chips.width, 1), app);
        }
        draw_quick_label_bar(f, chips, app);
        *quick_label_area = Some(chips);
    }
    draw_sort_rule(f, header.sort_rule, app);
}

/// Style of the header rules: dimmed, and drawn with a light dashed glyph so
/// they separate the rows without competing with the frame border.
fn rule_style() -> Style {
    muted_style()
}

/// Thin muted rule drawn between the summary bar and the quick-label strip.
fn draw_header_rule(f: &mut Frame, area: Rect, app: &App) {
    let rule = app.sym("rule").repeat(area.width as usize);
    f.render_widget(Paragraph::new(Line::from(Span::styled(rule, rule_style()))), area);
}

/// Closing rule under the header with the sort caption set into it, the way
/// the outer frame carries its titles. With `show_separators` off only the
/// caption is drawn.
pub(super) fn draw_sort_rule(f: &mut Frame, area: Rect, app: &App) {
    let border_set = ratatui::symbols::border::Set {
        horizontal_top: if app.config.show_separators { app.sym("rule") } else { " " },
        ..ratatui::symbols::border::PLAIN
    };
    let block = Block::default()
        .borders(Borders::TOP)
        .border_set(border_set)
        .border_style(rule_style())
        .title(
            Line::from(vec![
                Span::raw(" "),
                Span::styled(sort_caption(app), accent_style()),
                Span::raw(" "),
            ])
            .alignment(Alignment::Center),
        );
    f.render_widget(block, area);
}

/// One-row strip of the quick-label slots as numbered chips
/// (`1 web │ 2 api │ 3 infra`, in the order the labels were first viewed)
/// under the summary bar, styled like the summary tabs: divider-separated
/// segments with the active label drawn as a highlighted block.
pub(super) fn draw_quick_label_bar(f: &mut Frame, area: Rect, app: &App) {
    let parts = quick_label_parts(app);
    let labels = app.quick_labels();
    let mut spans = Vec::new();
    for (i, ((key_text, name_text), (label, _))) in parts.iter().zip(labels.iter()).enumerate() {
        if i > 0 {
            spans.push(Span::styled(QUICK_LABEL_DIVIDER, muted_style()));
        }
        if app.state.active_label_filter.as_deref() == Some(label.as_str()) {
            let chip = accent_style().add_modifier(Modifier::REVERSED | Modifier::BOLD);
            spans.push(Span::styled(key_text.clone(), chip));
            spans.push(Span::styled(name_text.clone(), chip));
        } else {
            // Key in accent so the shortcut pops; name in the default
            // foreground so the label itself stays readable.
            spans.push(Span::styled(key_text.clone(), accent_style().add_modifier(Modifier::BOLD)));
            spans.push(Span::styled(name_text.clone(), primary_style()));
        }
    }
    let line = Line::from(spans).alignment(Alignment::Center);
    f.render_widget(Paragraph::new(line), area);
}

/// Width of the compact view's "Active Branch" column.
///
/// Historically a fixed 28 columns, which truncated long branch names even on
/// wide terminals. Now grows to fit the longest visible branch line (branch
/// symbol + name + relative-time suffix), floored at the legacy 28 so narrow
/// terminals render exactly as before, and capped so the repository column
/// always keeps room. Both the header row and the item rows size their
/// columns through this, so they can never drift out of alignment.
pub(super) fn compact_branch_col_width(app: &App, total_width: u16) -> u16 {
    const MIN_WIDTH: u16 = 28;
    /// Columns reserved for the repository-name column at minimum.
    const MIN_REPO_COL: u16 = 30;

    let mut desired = MIN_WIDTH as usize;
    for (idx, _) in app.get_filtered_items() {
        if let Some(ItemStatus::GitRepo(Some(s))) = app.statuses.get(idx) {
            if let Some(b) = &s.branch {
                let time_len = s
                    .last_commit_time
                    .map(|t| format_relative_time(t).chars().count() + 3) // " (…)"
                    .unwrap_or(0);
                // branch symbol + space (2) + name + time + breathing room
                desired = desired.max(2 + b.chars().count() + time_len + 1);
            }
        }
    }

    let cap = total_width.saturating_sub(STATUS_ZONE_WIDTH + MIN_REPO_COL).max(MIN_WIDTH);
    (desired.min(u16::MAX as usize) as u16).clamp(MIN_WIDTH, cap)
}

pub(super) fn draw_compact_headers(f: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(compact_branch_col_width(app, area.width)),
            Constraint::Length(STATUS_ZONE_WIDTH),
        ])
        .split(area);

    let header_style = muted_style().add_modifier(Modifier::BOLD);

    let col_repo = Line::from(vec![Span::raw("   "), Span::styled("REPOSITORY", header_style)]);
    let col_branch = Line::from(Span::styled("ACTIVE BRANCH", header_style));
    let col_status = Line::from(Span::styled("STATUS", header_style)).alignment(Alignment::Right);

    f.render_widget(Paragraph::new(col_repo), cols[0]);
    f.render_widget(Paragraph::new(col_branch), cols[1]);
    f.render_widget(Paragraph::new(col_status), cols[2]);
}

pub(super) fn draw_items(f: &mut Frame, app: &App, chunks: &[Rect]) {
    let rows = app.get_home_rows();
    let upper = (app.scroll_top + chunks.len()).min(rows.len());
    let visible_rows = &rows[app.scroll_top..upper];
    let branch_col_width =
        compact_branch_col_width(app, chunks.first().map(|c| c.width).unwrap_or(0));

    for (i, row) in visible_rows.iter().enumerate() {
        let display_index = i + app.scroll_top;
        let is_selected = display_index == app.selected_index;

        match row {
            crate::app::HomeRow::GroupHeader { name, count, collapsed } => {
                let arrow = if *collapsed { "▶" } else { "▼" };
                let header_text = format!("{} [{}] ({} repos)", arrow, name, count);
                let style = if is_selected {
                    Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)
                } else {
                    muted_style().add_modifier(Modifier::BOLD)
                };
                let mut block = Block::default();
                if app.effective_view_mode() != crate::config::HomeViewMode::Compact {
                    let border_style =
                        if is_selected { Style::default().fg(ACCENT()) } else { muted_style() };
                    block = block.borders(Borders::BOTTOM).border_style(border_style);
                }

                let p = Paragraph::new(Line::from(vec![
                    Span::styled(
                        if is_selected { app.sym("selection_mark") } else { UNSELECTED_INDENT },
                        if is_selected { Style::default().fg(ACCENT()) } else { Style::default() },
                    ),
                    Span::styled(header_text, style),
                ]))
                .block(block);
                f.render_widget(p, chunks[i]);
            }
            crate::app::HomeRow::Repo { actual_index, path: item, .. } => {
                let actual_index = *actual_index;
                let fallback = ItemStatus::Missing;
                let status = app.statuses.get(actual_index).unwrap_or(&fallback);
                let is_git = matches!(status, ItemStatus::GitRepo(_));

                let mut is_partial = false;
                if let ItemStatus::GitRepo(Some(summary)) = status {
                    if summary.staged > 0 && (summary.modified > 0 || summary.untracked > 0) {
                        is_partial = true;
                    }
                }

                let pending_delete = is_selected && matches!(app.mode, Mode::ConfirmDelete);
                let pending_edit = is_selected && matches!(app.mode, Mode::Editing);
                let is_pinned = app.config.pinned.contains(item);
                let is_starred = app.config.starred.contains(item);
                let is_archived = app.config.archived.contains(item);

                // Selected/pending cards use an accent color; unselected cards use
                // the terminal's default foreground (dimmed) so they stay legible
                // on both light and dark backgrounds.
                let border_style = if pending_delete {
                    Style::default().fg(DANGER())
                } else if pending_edit {
                    Style::default().fg(WARNING())
                } else if is_selected {
                    Style::default().fg(ACCENT())
                } else if is_partial || is_pinned {
                    Style::default().fg(WARNING())
                } else {
                    muted_style()
                };

                let (mark, mark_style, text_style) = if is_selected {
                    (app.sym("selection_mark"), border_style, primary_style())
                } else {
                    (UNSELECTED_INDENT, Style::default(), Style::default())
                };

                let repo_name = std::path::Path::new(item)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(item.as_str());

                if app.effective_view_mode() == crate::config::HomeViewMode::Compact {
                    let cols = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Min(0),
                            Constraint::Length(branch_col_width),
                            Constraint::Length(STATUS_ZONE_WIDTH),
                        ])
                        .split(chunks[i]);

                    // 1. Repo name and labels
                    let mut left_spans = vec![Span::styled(mark, mark_style)];
                    if is_git {
                        left_spans.push(Span::styled(
                            app.sym("git_repo"),
                            if is_selected {
                                primary_style().add_modifier(Modifier::BOLD)
                            } else {
                                muted_style().add_modifier(Modifier::BOLD)
                            },
                        ));
                    }
                    let mut name_prefix = String::new();
                    if !app.multi_selected.is_empty() {
                        if app.multi_selected.contains(item) {
                            name_prefix.push_str("[x] ");
                        } else {
                            name_prefix.push_str("[ ] ");
                        }
                    }
                    left_spans.push(Span::styled(
                        format!("{}{}", name_prefix, repo_name),
                        if is_selected {
                            Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)
                        } else {
                            text_style
                        },
                    ));
                    if let ItemStatus::GitRepo(Some(summary)) = status {
                        let (state_str, state_style) = match summary.state {
                            RepoState::Merge => (
                                " ⚠ MERGE_HEAD",
                                Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
                            ),
                            RepoState::Rebase => (
                                " 🚧 REBASING",
                                Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
                            ),
                            RepoState::CherryPick => (
                                " ⚡ CHERRY-PICK",
                                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
                            ),
                            RepoState::Revert => (
                                " ⚡ REVERTING",
                                Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
                            ),
                            RepoState::Bisect => (
                                " 🔍 BISECTING",
                                Style::default()
                                    .fg(ratatui::style::Color::Magenta)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            RepoState::ApplyMailbox => (
                                " 📬 APPLYING",
                                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
                            ),
                            RepoState::Clean => (" ✓ CLEAN", muted_style()),
                        };
                        left_spans.push(Span::styled(
                            state_str,
                            if is_selected {
                                state_style
                                    .remove_modifier(Modifier::DIM)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                state_style
                            },
                        ));
                        if summary.staged > 0 && (summary.modified > 0 || summary.untracked > 0) {
                            left_spans.push(Span::styled(
                                " ⚠ PARTIAL",
                                Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
                            ));
                        }
                    }
                    if let Some(lbls) = app.config.labels.get(item) {
                        for lbl in lbls {
                            left_spans.push(Span::raw(" "));
                            left_spans.push(Span::styled(
                                format!("[{}]", lbl),
                                if is_selected {
                                    Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default().fg(ACCENT()).add_modifier(Modifier::DIM)
                                },
                            ));
                        }
                    }
                    let is_starred = app.config.starred.contains(item);
                    if is_starred {
                        left_spans.push(Span::raw(" "));
                        left_spans.push(Span::styled(
                            app.sym("star").trim(),
                            if is_selected {
                                Style::default()
                                    .fg(ratatui::style::Color::Yellow)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(ratatui::style::Color::Yellow)
                            },
                        ));
                    }
                    if is_pinned {
                        left_spans.push(Span::raw(" "));
                        left_spans.push(Span::styled(
                            app.sym("pinned").trim(),
                            if is_selected {
                                Style::default().fg(WARNING()).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(WARNING())
                            },
                        ));
                    }
                    if is_archived {
                        left_spans.push(Span::raw(" "));
                        left_spans.push(Span::styled(app.sym("archived").trim(), muted_style()));
                    }
                    f.render_widget(Paragraph::new(Line::from(left_spans)), cols[0]);

                    // 2. Branch and relative time
                    let branch_line = match status {
                        ItemStatus::GitRepo(Some(s)) => {
                            if let Some(b) = &s.branch {
                                let mut parts = vec![
                                    Span::styled(
                                        format!("{} ", app.sym("branch")),
                                        if is_selected {
                                            primary_style().add_modifier(Modifier::BOLD)
                                        } else {
                                            muted_style()
                                        },
                                    ),
                                    Span::styled(
                                        b.clone(),
                                        if is_selected {
                                            Style::default()
                                                .fg(ACCENT())
                                                .add_modifier(Modifier::BOLD)
                                        } else {
                                            Style::default().fg(ACCENT())
                                        },
                                    ),
                                ];
                                if let Some(time) = s.last_commit_time {
                                    parts.push(Span::styled(
                                        format!(" ({})", format_relative_time(time)),
                                        if is_selected {
                                            primary_style().add_modifier(Modifier::BOLD)
                                        } else {
                                            muted_style()
                                        },
                                    ));
                                }
                                Line::from(parts)
                            } else {
                                Line::from("")
                            }
                        }
                        _ => Line::from(""),
                    };
                    f.render_widget(Paragraph::new(branch_line), cols[1]);

                    // 3. Status indicator line
                    let status_line =
                        status_indicator_line(app, status, item).alignment(Alignment::Right);
                    f.render_widget(Paragraph::new(status_line), cols[2]);
                } else {
                    let border_type =
                        if is_selected { BorderType::LightDoubleDashed } else { CARD_BORDER() };

                    // Render the border block; split its inner rect into two rows:
                    //   row 0 — item path (left) + status indicator (right)
                    //   row 1 — branch name (left-aligned, muted)
                    let block = Block::default()
                        .borders(Borders::ALL)
                        .border_type(border_type)
                        .border_style(border_style)
                        .padding(Padding::horizontal(1));
                    let inner = block.inner(chunks[i]);
                    f.render_widget(block, chunks[i]);

                    let rows = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(1),
                            Constraint::Length(1),
                            Constraint::Min(0),
                        ])
                        .split(inner);

                    let name_cols = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Min(0), Constraint::Length(8)])
                        .split(rows[0]);

                    let mut spans = vec![Span::styled(mark, mark_style)];
                    if is_git {
                        spans.push(Span::styled(
                            app.sym("git_repo"),
                            muted_style().add_modifier(Modifier::BOLD),
                        ));
                    }
                    let mut name_prefix = String::new();
                    if !app.multi_selected.is_empty() {
                        if app.multi_selected.contains(item) {
                            name_prefix.push_str("[x] ");
                        } else {
                            name_prefix.push_str("[ ] ");
                        }
                    }
                    spans.push(Span::styled(format!("{}{}", name_prefix, repo_name), text_style));
                    if let ItemStatus::GitRepo(Some(summary)) = status {
                        let (state_str, state_style) = match summary.state {
                            RepoState::Merge => (
                                " ⚠ MERGE_HEAD",
                                Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
                            ),
                            RepoState::Rebase => (
                                " 🚧 REBASING",
                                Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
                            ),
                            RepoState::CherryPick => (
                                " ⚡ CHERRY-PICK",
                                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
                            ),
                            RepoState::Revert => (
                                " ⚡ REVERTING",
                                Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
                            ),
                            RepoState::Bisect => (
                                " 🔍 BISECTING",
                                Style::default()
                                    .fg(ratatui::style::Color::Magenta)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            RepoState::ApplyMailbox => (
                                " 📬 APPLYING",
                                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
                            ),
                            RepoState::Clean => (" ✓ CLEAN", muted_style()),
                        };
                        spans.push(Span::styled(state_str, state_style));
                        if summary.staged > 0 && (summary.modified > 0 || summary.untracked > 0) {
                            spans.push(Span::styled(
                                " ⚠ PARTIAL",
                                Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
                            ));
                        }
                    }
                    if let Some(lbls) = app.config.labels.get(item) {
                        for lbl in lbls {
                            spans.push(Span::raw(" "));
                            spans.push(Span::styled(
                                format!("[{}]", lbl),
                                Style::default().fg(ACCENT()).add_modifier(Modifier::DIM),
                            ));
                        }
                    }
                    let name_line = Line::from(spans);
                    f.render_widget(Paragraph::new(name_line), name_cols[0]);

                    let mut right_spans = Vec::new();
                    if is_starred {
                        right_spans.push(Span::styled(
                            format!("{} ", app.sym("star").trim()),
                            Style::default().fg(ratatui::style::Color::Yellow),
                        ));
                    }
                    if is_pinned {
                        right_spans.push(Span::styled(
                            app.sym("pinned").trim(),
                            Style::default().fg(WARNING()),
                        ));
                    }
                    if is_archived {
                        if !right_spans.is_empty() {
                            right_spans.push(Span::raw(" "));
                        }
                        right_spans.push(Span::styled(app.sym("archived").trim(), muted_style()));
                    }
                    if !right_spans.is_empty() {
                        let icon_line = Line::from(right_spans).alignment(Alignment::Right);
                        f.render_widget(Paragraph::new(icon_line), name_cols[1]);
                    }

                    // Row 1: Left column (branch name) and Right column (status section)
                    let row1_cols = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Min(0), Constraint::Length(STATUS_ZONE_WIDTH)])
                        .split(rows[1]);

                    let branch_line = match status {
                        ItemStatus::GitRepo(Some(s)) => {
                            if let Some(b) = &s.branch {
                                let mut parts = vec![
                                    Span::raw(UNSELECTED_INDENT),
                                    Span::styled(format!("{} ", app.sym("branch")), muted_style()),
                                    Span::styled(b.clone(), Style::default().fg(ACCENT())),
                                ];
                                if let Some(time) = s.last_commit_time {
                                    parts.push(Span::raw(" ("));
                                    parts.push(Span::styled(
                                        format_relative_time(time),
                                        muted_style(),
                                    ));
                                    parts.push(Span::raw(")"));
                                }
                                Line::from(parts)
                            } else {
                                Line::from("")
                            }
                        }
                        _ => Line::from(""),
                    };
                    f.render_widget(Paragraph::new(branch_line), row1_cols[0]);

                    let status_line =
                        status_indicator_line(app, status, item).alignment(Alignment::Right);
                    f.render_widget(Paragraph::new(status_line), row1_cols[1]);

                    let note_line = if let Some(repo_cfg) = app.config.repo_configs.get(item) {
                        if let Some(ref note) = repo_cfg.note {
                            Line::from(vec![
                                Span::raw(UNSELECTED_INDENT),
                                Span::styled("✎ ", muted_style()),
                                Span::styled(
                                    note.clone(),
                                    muted_style().add_modifier(Modifier::ITALIC),
                                ),
                            ])
                        } else {
                            Line::from("")
                        }
                    } else {
                        Line::from("")
                    };
                    f.render_widget(Paragraph::new(note_line), rows[2]);
                }
            }
        }
    }
}

/// Renders a centered empty-state message when no items are in the list.
pub(super) fn draw_empty_state(f: &mut Frame, area: Rect, app: &App) {
    let popup_area = crate::ui::layout::centered_rect_fixed(70, 16, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(accent_style())
        .title(
            Line::from(vec![
                Span::raw(" "),
                Span::styled("Gitwig Onboarding", primary_style().add_modifier(Modifier::BOLD)),
                Span::raw(" "),
            ])
            .alignment(Alignment::Center),
        )
        .padding(Padding::vertical(1));

    let inner = block.inner(popup_area);
    f.render_widget(ratatui::widgets::Clear, popup_area);
    f.render_widget(block, popup_area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header title
            Constraint::Length(1), // Description
            Constraint::Length(1), // Spacer
            Constraint::Length(7), // Shortcuts
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Tip
        ])
        .split(inner);

    let is_compat = app.config.compatibility_mode;
    let mut header_spans =
        vec![Span::styled("Welcome to Gitwig!", primary_style().add_modifier(Modifier::BOLD))];
    if !is_compat {
        // Copper HEAD ring from the brand mark; omitted in compatibility mode
        // because U+25CB is ambiguous-width and can misalign on narrow terminals.
        header_spans.push(Span::styled(" ○", Style::default().fg(BRAND_COPPER())));
    }
    let header = Line::from(header_spans).alignment(Alignment::Center);
    f.render_widget(Paragraph::new(header), layout[0]);

    let desc = Line::from(Span::styled(
        "Get started by tracking your first Git repository.",
        muted_style(),
    ))
    .alignment(Alignment::Center);
    f.render_widget(Paragraph::new(desc), layout[1]);

    let format_shortcut = |key: &str, desc: &str| -> Line<'static> {
        Line::from(vec![
            Span::raw("   "),
            Span::styled(format!("{:>4}", key), accent_style().add_modifier(Modifier::BOLD)),
            Span::styled("  ⟩  ", muted_style()),
            Span::raw(desc.to_string()),
        ])
    };

    let shortcut_lines = vec![
        format_shortcut("a", "Add a new local repository path"),
        format_shortcut("A", "Bulk add repositories in a directory"),
        format_shortcut("i", "Import a remote repository URL"),
        format_shortcut("?", "Open full shortcuts help reference"),
        format_shortcut("s", "Configure global settings"),
        format_shortcut("q", "Quit the application"),
    ];
    f.render_widget(Paragraph::new(shortcut_lines), layout[3]);

    let tip = Line::from(vec![
        Span::styled("Tip: ", primary_style().fg(WARNING())),
        Span::styled("paths support ~ expansion (e.g. ~/projects/my-repo)", muted_style()),
    ])
    .alignment(Alignment::Center);
    f.render_widget(Paragraph::new(tip), layout[5]);
}

/// Renders a centered empty-state message when search matches no repositories.
pub(super) fn draw_search_empty_state(f: &mut Frame, area: Rect, query: &str) {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(25), Constraint::Min(0), Constraint::Percentage(40)])
        .split(area);

    let lines = vec![
        Line::from(vec![Span::styled(
            format!("No repositories matching '{}'.", query),
            primary_style(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::raw("Press  "),
            Span::styled("Esc", accent_style()),
            Span::raw("  to clear the search filter"),
        ]),
    ];

    let p = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(p, vert[1]);
}

/// Renders a centered message when repositories exist but the active filters
/// (summary tab, sticky label filter, or hidden stale projects) leave nothing
/// to show. Drawn below the summary tab bar, which stays visible.
pub(super) fn draw_filter_empty_state(f: &mut Frame, area: Rect, app: &App) {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(25), Constraint::Min(0), Constraint::Percentage(40)])
        .split(area);

    let tab_name = match app.global_filter {
        Some(crate::app::GlobalFilter::Dirty) => Some("dirty"),
        Some(crate::app::GlobalFilter::Ahead) => Some("ahead"),
        Some(crate::app::GlobalFilter::Stale) => Some("stale"),
        Some(crate::app::GlobalFilter::Archived) => Some("archived"),
        None => None,
    };

    let all_archived = !app.config.items.is_empty()
        && app.config.items.iter().all(|p| app.config.archived.contains(p));
    let all_label_archived = app.state.active_label_filter.as_ref().is_some_and(|label| {
        let mut has_any = false;
        let all = app
            .config
            .items
            .iter()
            .filter(|p| {
                app.config.labels.get(*p).is_some_and(|lbls| lbls.iter().any(|l| l == label))
            })
            .all(|p| {
                has_any = true;
                app.config.archived.contains(p)
            });
        has_any && all
    });
    let headline = match (tab_name, &app.state.active_label_filter) {
        (Some(tab), Some(label)) => {
            format!("No {} repositories with label '{}'.", tab, label)
        }
        (Some(tab), None) => format!("No repositories in the {} tab right now.", tab),
        (None, Some(label)) if all_label_archived => {
            format!("All repositories with label '{}' are currently archived.", label)
        }
        (None, Some(label)) => format!("No repositories with label '{}'.", label),
        (None, None) if all_archived => "All repositories are currently archived.".to_string(),
        (None, None) => "All repositories are currently hidden as stale.".to_string(),
    };

    let mut lines = vec![
        Line::from(Span::styled(headline, primary_style().add_modifier(Modifier::BOLD))),
        Line::from(""),
    ];

    if tab_name.is_some() || all_archived || all_label_archived {
        lines.push(Line::from(vec![
            Span::raw("Press  "),
            Span::styled("Tab", accent_style()),
            Span::raw("  to switch tabs, or  "),
            Span::styled("Esc", accent_style()),
            Span::raw("  to show all repositories"),
        ]));
    }
    if app.state.active_label_filter.is_some() {
        let label_key = app.keybindings.format_action_keys(
            crate::keybindings::Action::HomeLabelPicker,
            app.config.compatibility_mode,
        );
        lines.push(Line::from(vec![
            Span::raw("Press  "),
            Span::styled(label_key, accent_style()),
            Span::raw("  to change or clear the label filter"),
        ]));
    }
    if tab_name.is_none() && app.state.active_label_filter.is_none() {
        lines.push(Line::from(vec![
            Span::raw("Enable  "),
            Span::styled("Show Stale Projects", accent_style()),
            Span::raw("  in Settings to see them"),
        ]));
    }

    let p = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(p, vert[1]);
}

/// Renders the per-item status as a colored symbol + (for git repos) a
/// compact set of `N+` (staged), `N!` (modified), `N?` (untracked),
/// `N↑` (commits ahead), `N↓` (commits behind) suffixes. Only non-zero
/// counts are shown so the indicator stays compact for the common case.
fn status_indicator_line(app: &App, status: &ItemStatus, item: &str) -> Line<'static> {
    if app.bulk_fetching.contains(item) {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let index = ((millis / 80) % 10) as usize;
        let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"][index];
        return Line::from(vec![
            Span::styled(
                format!("{} ", spinner),
                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("fetching...", Style::default().fg(ACCENT())),
        ]);
    }
    // The fetch outcome only replaces the normal indicator while the result is
    // fresh (i.e. until `bulk_fetch_completed_at` expires). Failures are retained
    // past that point so they stay inspectable, but a stale failure must not keep
    // hiding the repository's real staged/modified/ahead/behind counts — after the
    // window it degrades to a small trailing marker instead.
    let fetch_result = app.bulk_fetch_results.get(item);
    // Also treat a round still in flight as fresh, so a repo that finishes early
    // shows its outcome immediately instead of waiting for the slowest remote.
    let fresh = app.bulk_fetch_completed_at.is_some() || !app.bulk_fetching.is_empty();

    if let Some(res) = fetch_result {
        match res {
            Ok(_) if fresh => {
                return Line::from(vec![
                    Span::styled("✓ ", Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)),
                    Span::styled("done", Style::default().fg(SUCCESS())),
                ]);
            }
            // A bare "failed" tells the user nothing actionable, so show the
            // classified cause. A missing remote is a configuration fact rather
            // than a fault, so it is toned down to a warning.
            Err(_) if fresh => {
                let (color, label) = fetch_failure_style(app, item);
                return Line::from(vec![
                    Span::styled("✗ ", Style::default().fg(color).add_modifier(Modifier::BOLD)),
                    Span::styled(label, Style::default().fg(color)),
                ]);
            }
            _ => {}
        }
    }

    let mut line = match status {
        ItemStatus::Loading => Line::from(vec![
            Span::styled(app.sym("bullet_empty"), muted_style()),
            Span::raw(" "),
            Span::styled("loading...", muted_style()),
        ]),
        ItemStatus::Missing => Line::from(vec![
            Span::styled(app.sym("close"), Style::default().fg(DANGER())),
            Span::raw(" "),
            Span::styled("missing", muted_style()),
        ]),
        ItemStatus::Directory => Line::from(vec![
            Span::styled(app.sym("bullet_empty"), Style::default().fg(WARNING())),
            Span::raw(" "),
            Span::styled("dir", muted_style()),
        ]),
        ItemStatus::GitRepo(None) => Line::from(vec![
            Span::styled(app.sym("bullet_filled"), Style::default().fg(SUCCESS())),
            Span::raw(" "),
            Span::styled("?", muted_style()),
        ]),
        ItemStatus::GitRepo(Some(summary)) => repo_indicator_line(app, summary),
    };

    // Stale failure: keep the real status visible, but append a compact marker so
    // the user can still tell this repository is not talking to its remote.
    if matches!(fetch_result, Some(Err(_))) {
        let (color, _) = fetch_failure_style(app, item);
        line.spans.push(Span::raw(" "));
        line.spans.push(Span::styled("✗", Style::default().fg(color).add_modifier(Modifier::BOLD)));
    }

    line
}

/// Colour and compact label for a repository's recorded fetch failure.
///
/// "No remote" is a configuration fact rather than a fault, so it renders as a
/// warning instead of an error.
fn fetch_failure_style(app: &App, item: &str) -> (ratatui::style::Color, &'static str) {
    let failure = app
        .bulk_fetch_failures
        .get(item)
        .copied()
        .unwrap_or(crate::fetch_error::FetchFailure::Unknown);
    let color =
        if failure == crate::fetch_error::FetchFailure::NoRemote { WARNING() } else { DANGER() };
    (color, failure.label())
}

pub(super) fn repo_indicator_line(app: &App, summary: &RepoSummary) -> Line<'static> {
    let dot_color = if summary.conflicted > 0 { DANGER() } else { SUCCESS() };
    let mut spans = vec![Span::styled(app.sym("bullet_filled"), Style::default().fg(dot_color))];
    if summary.unchanged() {
        spans.push(Span::raw(" "));
        spans.push(Span::styled("clean", muted_style()));
        return Line::from(spans);
    }
    let ahead_style = if summary.ahead <= 3 {
        Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)
    } else if summary.ahead <= 10 {
        Style::default().fg(WARNING()).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)
    };

    let behind_style = if summary.behind <= 5 {
        Style::default().fg(WARNING()).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)
    };

    // Each (count, symbol, style) is rendered only if count > 0. The
    // ordering matches the Detail view's worktree section for consistency.
    let parts = [
        (summary.staged, "+", Style::default().fg(ACCENT())),
        (summary.modified, "!", Style::default().fg(WARNING())),
        (summary.untracked, "?", muted_style()),
        (summary.conflicted, app.sym("action").trim(), Style::default().fg(DANGER())),
        (summary.ahead, app.sym("up"), ahead_style),
        (summary.behind, app.sym("down"), behind_style),
    ];
    for (count, symbol, style) in parts {
        if count > 0 {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(format!("{}{}", count, symbol), style));
        }
    }
    Line::from(spans)
}
