//! Full-screen detail view for the currently-selected item.
//!
//! Reads a snapshot prepared by `repo::inspect_detail` and renders it as a
//! padded paragraph in the body of the outer Twig frame. Drawing is pure —
//! all git2 work happens once in `App::open_detail`, not per frame.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Padding, Paragraph, Wrap};

use crate::repo::{HeadInfo, ItemDetail, RemoteInfo, RepoInfo, RepoSummary};
use crate::ui::{ACCENT, DANGER, SUCCESS, WARNING, accent_style, muted_style, primary_style};

const FIELD_INDENT: &str = "  ";
/// Column width for the left-side field label — picked to align longest
/// label ("Working") with a trailing colon and space.
const FIELD_LABEL_WIDTH: usize = 9;

/// Renders the detail view into `area`. The caller is responsible for
/// passing the area inside the outer frame (so the frame's border stays
/// intact).
pub fn draw(f: &mut Frame, item_name: &str, detail: &ItemDetail, area: Rect) {
    // Reserve one row at the top for the breadcrumb-like header
    // ("Item: <name>") so it doesn't get scrolled away if the body is long.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(area);

    let header = Paragraph::new(Line::from(vec![
        Span::raw(FIELD_INDENT),
        Span::styled(label("Item"), muted_style()),
        Span::styled(item_name.to_string(), accent_style()),
    ]));
    f.render_widget(header, chunks[0]);

    let body_lines = build_body(detail);
    let body = Paragraph::new(body_lines)
        .block(Block::default().padding(Padding::ZERO))
        .wrap(Wrap { trim: false });
    f.render_widget(body, chunks[1]);
}

fn build_body(detail: &ItemDetail) -> Vec<Line<'static>> {
    match detail {
        ItemDetail::Missing { resolved } => {
            vec![
                field_line("Path", Span::raw(resolved.display().to_string())),
                Line::from(""),
                kind_line(
                    "✕",
                    DANGER,
                    "Not a directory",
                    "(path does not exist or isn't accessible)",
                ),
            ]
        }
        ItemDetail::Directory { resolved } => {
            vec![
                field_line("Path", Span::raw(resolved.display().to_string())),
                Line::from(""),
                kind_line(
                    "○",
                    WARNING,
                    "Plain directory",
                    "(exists, but no .git entry was found)",
                ),
            ]
        }
        ItemDetail::Error { resolved, message } => {
            vec![
                field_line("Path", Span::raw(resolved.display().to_string())),
                Line::from(""),
                kind_line(
                    "⚠",
                    WARNING,
                    "Could not read repository",
                    "(libgit2 reported an error — see below)",
                ),
                Line::from(""),
                Line::from(vec![
                    Span::raw(FIELD_INDENT),
                    Span::styled(message.clone(), Style::default().fg(DANGER)),
                ]),
            ]
        }
        ItemDetail::Repo { resolved, info } => {
            let mut lines = vec![
                field_line("Path", Span::raw(resolved.display().to_string())),
                Line::from(""),
                kind_line(
                    "●",
                    SUCCESS,
                    "Git repository",
                    "(an inspectable libgit2 repo)",
                ),
                Line::from(""),
            ];
            append_repo_body(&mut lines, info);
            lines
        }
    }
}

fn append_repo_body(lines: &mut Vec<Line<'static>>, info: &RepoInfo) {
    // Branch
    let branch = info
        .branch
        .clone()
        .unwrap_or_else(|| "(detached HEAD)".to_string());
    lines.push(field_line("Branch", Span::styled(branch, accent_style())));

    // HEAD
    if let Some(head) = &info.head {
        append_head(lines, head);
    } else {
        lines.push(field_line(
            "HEAD",
            Span::styled("(empty repository)", muted_style()),
        ));
    }

    // Upstream + sync
    lines.push(Line::from(""));
    append_upstream(lines, info);

    // Remotes
    lines.push(Line::from(""));
    if info.remotes.is_empty() {
        lines.push(field_line("Remotes", Span::styled("(none)", muted_style())));
    } else {
        lines.push(field_line(
            "Remotes",
            Span::styled(format!("{}", info.remotes.len()), primary_style()),
        ));
        for r in &info.remotes {
            lines.push(remote_line(r));
        }
    }

    // Worktree status
    lines.push(Line::from(""));
    append_worktree(lines, &info.summary);
}

fn append_upstream(lines: &mut Vec<Line<'static>>, info: &RepoInfo) {
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
                let mut spans = Vec::new();
                if s.ahead > 0 {
                    spans.push(Span::styled(format!("{} ahead", s.ahead), primary_style()));
                }
                if s.behind > 0 {
                    if !spans.is_empty() {
                        spans.push(Span::raw(", "));
                    }
                    spans.push(Span::styled(
                        format!("{} behind", s.behind),
                        Style::default().fg(WARNING),
                    ));
                }
                lines.push(Line::from(
                    [
                        vec![
                            Span::raw(FIELD_INDENT),
                            Span::styled(label("Sync"), muted_style()),
                        ],
                        spans,
                    ]
                    .concat(),
                ));
            }
        }
    }
}

fn append_head(lines: &mut Vec<Line<'static>>, head: &HeadInfo) {
    lines.push(Line::from(vec![
        Span::raw(FIELD_INDENT),
        Span::styled(label("HEAD"), muted_style()),
        Span::styled(head.short_id.clone(), Style::default().fg(WARNING)),
        Span::raw("  "),
        Span::styled(head.summary.clone(), primary_style()),
    ]));
    lines.push(continuation_line(head.author.clone()));
    lines.push(continuation_line(head.when.clone()));
}

fn remote_line(remote: &RemoteInfo) -> Line<'static> {
    Line::from(vec![
        Span::raw("    "),
        Span::styled(format!("{:<8}", remote.name), Style::default().fg(ACCENT)),
        Span::raw("  "),
        Span::raw(remote.url.clone()),
    ])
}

fn append_worktree(lines: &mut Vec<Line<'static>>, status: &RepoSummary) {
    if status.is_clean() {
        lines.push(field_line(
            "Working",
            Span::styled("clean", Style::default().fg(SUCCESS)),
        ));
        return;
    }
    lines.push(field_line(
        "Working",
        Span::styled("changes pending", primary_style()),
    ));
    if status.staged > 0 {
        lines.push(count_line("Staged", status.staged, ACCENT));
    }
    if status.modified > 0 {
        lines.push(count_line("Modified", status.modified, WARNING));
    }
    if status.untracked > 0 {
        lines.push(count_line_muted("Untracked", status.untracked));
    }
    if status.conflicted > 0 {
        lines.push(count_line("Conflicted", status.conflicted, DANGER));
    }
}

/// `"Field    "` with the trailing colon + space, padded to `FIELD_LABEL_WIDTH`.
fn label(name: &str) -> String {
    let mut s = format!("{}:", name);
    while s.chars().count() < FIELD_LABEL_WIDTH {
        s.push(' ');
    }
    s
}

fn field_line(name: &'static str, value: Span<'static>) -> Line<'static> {
    Line::from(vec![
        Span::raw(FIELD_INDENT),
        Span::styled(label(name), muted_style()),
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

fn count_line(name: &'static str, count: usize, color: ratatui::style::Color) -> Line<'static> {
    Line::from(vec![
        Span::raw("    "),
        Span::styled(format!("{:<11}", name), muted_style()),
        Span::styled(format!("{}", count), Style::default().fg(color)),
    ])
}

fn count_line_muted(name: &'static str, count: usize) -> Line<'static> {
    Line::from(vec![
        Span::raw("    "),
        Span::styled(format!("{:<11}", name), muted_style()),
        Span::styled(format!("{}", count), muted_style()),
    ])
}
