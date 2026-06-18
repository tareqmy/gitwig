//! Full-screen detail view for the currently-selected item.
//!
//! Reads a snapshot prepared by `repo::inspect_detail` and renders it as a
//! padded paragraph in the body of the outer Twig frame. Drawing is pure —
//! all git2 work happens once in `App::open_detail`, not per frame.
//!
//! The view is divided into labelled sections (Overview, Repository, Sync,
//! Working Tree) separated by blank lines and introduced by an accent-coloured
//! `▍ Title` header line. Within "Working Tree", changed files are listed
//! under Staged / Unstaged / Untracked / Conflicts sub-headers.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Clear, Padding, Paragraph, Row, Table, Wrap};

use crate::repo::{FileEntry, HeadInfo, ItemDetail, RemoteInfo, RepoInfo, WorktreeChanges};
use crate::ui::{ACCENT, DANGER, SUCCESS, WARNING, accent_style, muted_style, primary_style};
use crate::app::DetailSection;

const FIELD_INDENT: &str = "  ";
/// Column width for the left-side field label — wide enough for "Upstream:".
const FIELD_LABEL_WIDTH: usize = 9;
/// Indent for file entries inside a working-tree sub-section.
const FILE_INDENT: &str = "      ";
/// Column width for the file-status label ("typechange" = 10 chars).
const FILE_LABEL_WIDTH: usize = 10;

use crate::app::Mode;

// ── Entry point ────────────────────────────────────────────────────────────

/// Renders the detail view into `area`.
pub fn draw(f: &mut Frame, item_name: &str, detail: &ItemDetail, mode: &Mode, focus: &DetailSection, area: Rect) {
    // Reserve one row at the top for the breadcrumb header ("Item: <name>").
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(area);

    // Extract branch name if this is a repo detail.
    let branch: Option<String> = match detail {
        ItemDetail::Repo { info, .. } => info.branch.clone(),
        _ => None,
    };

    // Split header into left (Item label) and right (branch name).
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(40)])
        .split(chunks[0]);

    let header_left = Paragraph::new(Line::from(vec![
        Span::raw(FIELD_INDENT),
        Span::styled(field_label("Item"), muted_style()),
        Span::styled(item_name.to_string(), accent_style()),
    ]));
    f.render_widget(header_left, header_chunks[0]);

    if let Some(branch_name) = branch {
        let header_right = Paragraph::new(Line::from(vec![
            Span::styled("⎇  ", muted_style()),
            Span::styled(branch_name, accent_style()),
            Span::raw("  "),
        ]))
        .alignment(Alignment::Right);
        f.render_widget(header_right, header_chunks[1]);
    }

    let body_area = chunks[1];

    match detail {
        ItemDetail::Repo { resolved, info } => {
            // Split body: top = recent commits, bottom = staging panels
            let detail_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(body_area);

            draw_detail_commits(f, info, *focus, detail_chunks[0]);
            draw_staging_panels(f, &info.changes, *focus, detail_chunks[1]);

            // Draw overview popup on top when requested
            if matches!(mode, Mode::DetailOverview) {
                draw_overview_popup(f, resolved, info, body_area);
            }
        }
        _ => {
            let body_lines = build_body(detail);
            let body = Paragraph::new(body_lines)
                .block(Block::default().padding(Padding::ZERO))
                .wrap(Wrap { trim: false });
            f.render_widget(body, body_area);
        }
    }
}

// ── Staging panels ─────────────────────────────────────────────────────────

/// Renders two side-by-side panels: "Staging Area" (left, split into Staged/Unstaged)
/// and "Staging Details" (right).
fn draw_staging_panels(f: &mut Frame, changes: &WorktreeChanges, focus: DetailSection, area: Rect) {
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Focus-aware border helpers.
    let left_focused  = focus == DetailSection::Staged || focus == DetailSection::Unstaged;
    let right_focused = focus == DetailSection::StagingDetails;

    // ── Left panel: outer border labelled "Staging Area" ──────────────────
    let left_border_style = if left_focused { Style::default().fg(ACCENT) } else { muted_style() };
    let left_outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(left_border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Staging Area", primary_style()),
            Span::raw(" "),
        ]));
    let left_inner = left_outer.inner(panels[0]);
    f.render_widget(left_outer, panels[0]);

    // Split left inner vertically: top = Staged, bottom = Unstaged
    let left_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(left_inner);

    draw_file_subpanel(
        f,
        "Staged",
        SUCCESS,
        &changes.staged,
        "Nothing staged",
        Borders::BOTTOM,
        focus == DetailSection::Staged,
        left_split[0],
    );
    draw_file_subpanel(
        f,
        "Unstaged",
        WARNING,
        &changes.unstaged,
        "No unstaged changes",
        Borders::empty(),
        focus == DetailSection::Unstaged,
        left_split[1],
    );

    // ── Right panel – Staging Details ─────────────────────────────────────
    let right_border_style = if right_focused { Style::default().fg(ACCENT) } else { muted_style() };
    let right_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(right_border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Staging Details", primary_style()),
            Span::raw(" "),
        ]));
    let right_inner = right_block.inner(panels[1]);
    f.render_widget(right_block, panels[1]);

    let v_center = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(right_inner);
    f.render_widget(
        Paragraph::new(Span::styled("Select a file to view its diff", muted_style()))
            .alignment(Alignment::Center),
        v_center[1],
    );
}

/// Renders a titled sub-panel listing `files`, or a centred placeholder if empty.
fn draw_file_subpanel(
    f: &mut Frame,
    title: &'static str,
    title_color: ratatui::style::Color,
    files: &[FileEntry],
    empty_msg: &'static str,
    borders: Borders,
    focused: bool,
    area: Rect,
) {
    // When focused, highlight the title in accent; border stays muted (contained inside outer).
    let title_style = if focused {
        Style::default().fg(ACCENT).add_modifier(ratatui::style::Modifier::BOLD)
    } else {
        Style::default().fg(title_color)
    };
    let border_style = if focused { Style::default().fg(ACCENT) } else { muted_style() };
    // Sub-panel block — bottom border separates Staged from Unstaged.
    let block = Block::default()
        .borders(borders)
        .border_style(border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(title, title_style),
            Span::raw("  "),
            Span::styled(
                format!("({})", files.len()),
                muted_style(),
            ),
            Span::raw(" "),
        ]));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if files.is_empty() {
        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(inner);
        f.render_widget(
            Paragraph::new(Span::styled(empty_msg, muted_style()))
                .alignment(Alignment::Center),
            v[1],
        );
        return;
    }

    let file_lines: Vec<Line<'static>> = files.iter().map(file_entry_line).collect();
    f.render_widget(
        Paragraph::new(file_lines).wrap(Wrap { trim: false }),
        inner,
    );
}

// ── Overview popup ─────────────────────────────────────────────────────────

/// Renders the repo overview as a floating popup centred over `area`.
fn draw_overview_popup(
    f: &mut Frame,
    resolved: &std::path::Path,
    info: &RepoInfo,
    area: Rect,
) {
    // Popup takes up ~70 % of the available space
    let popup_area = centred_rect(70, 70, area);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Overview", primary_style()),
            Span::raw("  "),
            Span::styled("o / Esc  close", muted_style()),
            Span::raw(" "),
        ]));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let body_lines = build_repo_body(resolved, info);
    let body = Paragraph::new(body_lines)
        .block(Block::default().padding(Padding::horizontal(1)))
        .wrap(Wrap { trim: false });
    f.render_widget(body, inner);
}

/// Returns a [`Rect`] that is `percent_x` wide and `percent_y` tall, centred in `r`.
fn centred_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_detail_commits(f: &mut Frame, info: &RepoInfo, focus: DetailSection, area: Rect) {
    let focused = focus == DetailSection::Commits;
    let border_style = if focused { Style::default().fg(ACCENT) } else { muted_style() };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Commits", primary_style()),
            Span::raw(" "),
        ]));

    if info.commits.is_empty() {
        let inner = block.inner(area);
        f.render_widget(block, area);
        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(inner);
        f.render_widget(
            Paragraph::new(Span::styled(
                "No commits yet / empty repository",
                muted_style(),
            ))
            .alignment(Alignment::Center),
            v[1],
        );
        return;
    }

    let header = Row::new(vec![
        Cell::from("ID"),
        Cell::from("Author"),
        Cell::from("Date"),
        Cell::from("Summary"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD).fg(ACCENT));

    let rows: Vec<Row> = info
        .commits
        .iter()
        .map(|commit| {
            Row::new(vec![
                Cell::from(Span::styled(
                    commit.id.clone(),
                    Style::default().fg(WARNING),
                )),
                Cell::from(Span::styled(commit.author.clone(), Style::default())),
                Cell::from(Span::styled(commit.when.clone(), muted_style())),
                Cell::from(Span::styled(commit.summary.clone(), Style::default())),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(9),  // "c7a45e2" + 2 padding
        Constraint::Length(18), // Author name
        Constraint::Length(16), // Date
        Constraint::Min(20),    // Summary
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .column_spacing(2);

    f.render_widget(table, area);
}

// ── Body builder ───────────────────────────────────────────────────────────

fn build_body(detail: &ItemDetail) -> Vec<Line<'static>> {
    match detail {
        ItemDetail::Missing { resolved } => {
            let mut lines = vec![];
            push_section_header(&mut lines, "Overview");
            lines.push(kind_line(
                "✕",
                DANGER,
                "Not a directory",
                "(path does not exist or isn't accessible)",
            ));
            lines.push(field_line(
                "Path",
                Span::raw(resolved.display().to_string()),
            ));
            lines
        }
        ItemDetail::Directory { resolved } => {
            let mut lines = vec![];
            push_section_header(&mut lines, "Overview");
            lines.push(kind_line(
                "○",
                WARNING,
                "Plain directory",
                "(exists, but no .git entry was found)",
            ));
            lines.push(field_line(
                "Path",
                Span::raw(resolved.display().to_string()),
            ));
            lines
        }
        ItemDetail::Error { resolved, message } => {
            let mut lines = vec![];
            push_section_header(&mut lines, "Overview");
            lines.push(kind_line(
                "⚠",
                WARNING,
                "Could not read repository",
                "(libgit2 reported an error — see below)",
            ));
            lines.push(field_line(
                "Path",
                Span::raw(resolved.display().to_string()),
            ));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::raw(FIELD_INDENT),
                Span::styled(message.clone(), Style::default().fg(DANGER)),
            ]));
            lines
        }
        ItemDetail::Repo { resolved, info } => build_repo_body(resolved, info),
    }
}

fn build_repo_body(resolved: &std::path::Path, info: &RepoInfo) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = vec![];

    // ── Overview ──────────────────────────────────────────────────────────
    push_section_header(&mut lines, "Overview");
    lines.push(kind_line(
        "●",
        SUCCESS,
        "Git repository",
        "(an inspectable libgit2 repo)",
    ));
    lines.push(field_line(
        "Path",
        Span::raw(resolved.display().to_string()),
    ));

    // ── Repository ────────────────────────────────────────────────────────
    push_section_header(&mut lines, "Repository");
    let branch = info
        .branch
        .clone()
        .unwrap_or_else(|| "(detached HEAD)".to_string());
    lines.push(field_line("Branch", Span::styled(branch, accent_style())));
    if let Some(head) = &info.head {
        append_head(&mut lines, head);
    } else {
        lines.push(field_line(
            "HEAD",
            Span::styled("(empty repository)", muted_style()),
        ));
    }

    // ── Sync ──────────────────────────────────────────────────────────────
    push_section_header(&mut lines, "Sync");
    append_sync(&mut lines, info);

    lines
}

// ── Section rendering ──────────────────────────────────────────────────────

/// Emits a blank line then `  ▍ Title` in accent + bold, then another blank.
fn push_section_header(lines: &mut Vec<Line<'static>>, title: &'static str) {
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw(FIELD_INDENT),
        Span::styled("▍ ", Style::default().fg(ACCENT)),
        Span::styled(title, primary_style()),
    ]));
    lines.push(Line::from(""));
}


// ── Repository section ─────────────────────────────────────────────────────

fn append_head(lines: &mut Vec<Line<'static>>, head: &HeadInfo) {
    lines.push(Line::from(vec![
        Span::raw(FIELD_INDENT),
        Span::styled(field_label("HEAD"), muted_style()),
        Span::styled(head.short_id.clone(), Style::default().fg(WARNING)),
        Span::raw("  "),
        Span::styled(head.summary.clone(), primary_style()),
    ]));
    lines.push(continuation_line(head.author.clone()));
    lines.push(continuation_line(head.when.clone()));
}

// ── Sync section ──────────────────────────────────────────────────────────

fn append_sync(lines: &mut Vec<Line<'static>>, info: &RepoInfo) {
    match &info.upstream {
        None => {
            lines.push(field_line(
                "Upstream",
                Span::styled("(not configured)", muted_style()),
            ));
            lines.push(field_line("Sync", Span::styled("—", muted_style())));
        }
        Some(name) => {
            lines.push(field_line(
                "Upstream",
                Span::styled(name.clone(), Style::default().fg(ACCENT)),
            ));
            let s = &info.summary;
            if s.is_synced() {
                lines.push(field_line(
                    "Sync",
                    Span::styled("in sync", Style::default().fg(SUCCESS)),
                ));
            } else {
                let mut spans = vec![
                    Span::raw(FIELD_INDENT),
                    Span::styled(field_label("Sync"), muted_style()),
                ];
                if s.ahead > 0 {
                    spans.push(Span::styled(format!("{} ahead", s.ahead), primary_style()));
                }
                if s.behind > 0 {
                    if s.ahead > 0 {
                        spans.push(Span::raw(", "));
                    }
                    spans.push(Span::styled(
                        format!("{} behind", s.behind),
                        Style::default().fg(WARNING),
                    ));
                }
                lines.push(Line::from(spans));
            }
        }
    }

    // Remotes listed under Sync
    if info.remotes.is_empty() {
        lines.push(field_line("Remotes", Span::styled("(none)", muted_style())));
    } else {
        lines.push(Line::from(""));
        for r in &info.remotes {
            lines.push(remote_line(r));
        }
    }
}

fn remote_line(remote: &RemoteInfo) -> Line<'static> {
    Line::from(vec![
        Span::raw("    "),
        Span::styled(format!("{:<8}", remote.name), Style::default().fg(ACCENT)),
        Span::raw("  "),
        Span::raw(remote.url.clone()),
    ])
}


fn file_entry_line(entry: &FileEntry) -> Line<'static> {
    let label_style = match entry.label {
        "new" => Style::default().fg(SUCCESS),
        "deleted" => Style::default().fg(DANGER),
        "conflict" => Style::default().fg(DANGER).add_modifier(Modifier::BOLD),
        "renamed" | "typechange" => Style::default().fg(ACCENT),
        "??" => muted_style(),
        _ => Style::default().fg(WARNING), // "modified"
    };
    Line::from(vec![
        Span::raw(FILE_INDENT),
        Span::styled(format!("{:<FILE_LABEL_WIDTH$}", entry.label), label_style),
        Span::styled(entry.path.clone(), muted_style()),
    ])
}

// ── Low-level line builders ────────────────────────────────────────────────

/// `"Field:   "` padded to `FIELD_LABEL_WIDTH`.
fn field_label(name: &str) -> String {
    let mut s = format!("{}:", name);
    while s.chars().count() < FIELD_LABEL_WIDTH {
        s.push(' ');
    }
    s
}

fn field_line(name: &'static str, value: Span<'static>) -> Line<'static> {
    Line::from(vec![
        Span::raw(FIELD_INDENT),
        Span::styled(field_label(name), muted_style()),
        value,
    ])
}

fn continuation_line(text: String) -> Line<'static> {
    Line::from(vec![
        Span::raw(FIELD_INDENT),
        Span::raw(" ".repeat(FIELD_LABEL_WIDTH)),
        Span::styled(text, muted_style()),
    ])
}

fn kind_line(
    symbol: &'static str,
    color: ratatui::style::Color,
    title: &'static str,
    sub: &'static str,
) -> Line<'static> {
    Line::from(vec![
        Span::raw(FIELD_INDENT),
        Span::styled(symbol, Style::default().fg(color)),
        Span::raw("  "),
        Span::styled(title, primary_style()),
        Span::raw("  "),
        Span::styled(sub, muted_style()),
    ])
}
