//! Shared drawing helpers: status bar input layout, line wrapping, tag confirm prompts, and process stats.

use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::*;
use crate::app::App;
use crate::components::cmd_bar::StatusEntry;

pub(crate) fn draw_input_status(
    f: &mut Frame,
    area: Rect,
    verb: &str,
    buffer: &str,
    is_compat: bool,
    cursor: usize,
) {
    let mut spans = Vec::new();

    // Add Mode Badge
    spans.push(Span::styled(
        "INPUT",
        Style::default().fg(ratatui::style::Color::Red).add_modifier(Modifier::BOLD),
    ));

    let mode_sep = if is_compat { " > " } else { " ⟩ " };
    spans.push(Span::styled(mode_sep, muted_style()));

    let prefix = format!("{} › ", verb);
    spans.push(Span::styled(
        prefix.clone(),
        Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::styled(buffer.to_string(), primary_style()));

    let separator = if is_compat { " > " } else { " ⟩ " };
    spans.push(Span::styled(separator, muted_style()));
    spans.push(Span::raw("Save"));
    spans.push(Span::raw(" "));
    spans.push(Span::styled("[", muted_style()));
    spans.push(Span::styled("↵", accent_style()));
    spans.push(Span::styled("]", muted_style()));

    spans.push(Span::styled(separator, muted_style()));
    spans.push(Span::raw("Cancel"));
    spans.push(Span::raw(" "));
    spans.push(Span::styled("[", muted_style()));
    let cancel_key = if is_compat { "Esc" } else { "⎋" };
    spans.push(Span::styled(cancel_key, accent_style()));
    spans.push(Span::styled("]", muted_style()));

    let para = Paragraph::new(Line::from(spans));
    f.render_widget(para, area);

    // Cursor position calculation includes the Mode Badge (5 chars) and Mode Sep (3 chars)
    let badge_offset = 5 + 3;
    // Caret sits at `cursor` characters into the buffer, not at its end, so the
    // terminal cursor tracks arrow-key movement.
    let caret = cursor.min(buffer.chars().count());
    let cursor_offset = (badge_offset + prefix.chars().count() + caret) as u16;
    let cursor_x = area.x.saturating_add(cursor_offset.min(area.width.saturating_sub(1)));
    f.set_cursor_position(Position::new(cursor_x, area.y));
}

pub(crate) fn wrap_str(s: &str, max_width: usize) -> Vec<String> {
    if s.is_empty() {
        return vec![String::new()];
    }
    let mut lines = Vec::new();
    for paragraph in s.lines() {
        let mut current_line = String::new();
        for word in paragraph.split_whitespace() {
            if current_line.is_empty() {
                if word.chars().count() > max_width {
                    let chars: Vec<char> = word.chars().collect();
                    let mut i = 0;
                    while i < chars.len() {
                        let end = (i + max_width).min(chars.len());
                        lines.push(chars[i..end].iter().collect());
                        i = end;
                    }
                } else {
                    current_line.push_str(word);
                }
            } else {
                let proposed_len = current_line.chars().count() + 1 + word.chars().count();
                if proposed_len <= max_width {
                    current_line.push(' ');
                    current_line.push_str(word);
                } else {
                    lines.push(current_line);
                    if word.chars().count() > max_width {
                        let chars: Vec<char> = word.chars().collect();
                        let mut i = 0;
                        while i < chars.len() {
                            let end = (i + max_width).min(chars.len());
                            lines.push(chars[i..end].iter().collect());
                            i = end;
                        }
                        current_line = String::new();
                    } else {
                        current_line = word.to_string();
                    }
                }
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }
    }
    if lines.is_empty() { vec![String::new()] } else { lines }
}

/// Pack comma-separated `val_str` items onto lines of at most `max_width` chars.
/// Items that are individually wider than `max_width` are hard-wrapped by
/// `wrap_str`. The function always returns at least one (possibly empty) entry.
pub(crate) fn wrap_excludes(val_str: &str, max_width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current_line = String::new();
    let parts: Vec<&str> = val_str.split(',').collect();
    for (idx, part) in parts.iter().enumerate() {
        let suffix = if idx + 1 < parts.len() { "," } else { "" };
        let item = format!("{}{}", part, suffix);

        if current_line.chars().count() + item.chars().count() > max_width {
            if !current_line.is_empty() {
                lines.push(current_line);
                current_line = String::new();
            }
            if item.chars().count() > max_width {
                let mut sub_chunks = wrap_str(&item, max_width);
                if let Some(last) = sub_chunks.pop() {
                    current_line = last;
                }
                lines.extend(sub_chunks);
            } else {
                current_line = item;
            }
        } else {
            current_line.push_str(&item);
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    if lines.is_empty() { vec![String::new()] } else { lines }
}

#[allow(clippy::needless_range_loop)]

/// Returns a `Rect` of `(percent_x, percent_y)` dimensions, centered inside `area`.

pub(crate) fn confirm_tag_delete_entries(
    target: &str,
    is_on_remote: bool,
) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let mut spans = vec![
        Span::raw("Delete tag "),
        Span::styled(
            format!("\"{}\"", target),
            Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
        ),
        Span::raw("? "),
    ];
    if is_on_remote {
        spans.push(Span::styled(
            "(will also delete from remote) ",
            Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
        ));
    }
    let message_spans = Some(spans);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("y", Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

pub(crate) fn confirm_tag_push_entries(
    target: &str,
) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let message_spans = Some(vec![
        Span::raw("Push tag "),
        Span::styled(
            format!("\"{}\"", target),
            Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD),
        ),
        Span::raw("? "),
    ]);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("y", Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

pub(crate) fn confirm_tag_push_all_entries() -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let message_spans = Some(vec![
        Span::raw("Push "),
        Span::styled("ALL", Style::default().fg(WARNING()).add_modifier(Modifier::BOLD)),
        Span::raw(" local tags? "),
    ]);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("y", Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

#[cfg(unix)]
#[allow(unsafe_code)]
pub(crate) fn get_process_stats(app: &App) -> (f64, f64) {
    if let Ok(mut guard) = app.cpu_tracker.lock() {
        let now = std::time::Instant::now();

        // If 2000 ms has not elapsed, return the cached values
        if let Some((_, prev_time, cached_cpu, cached_rss)) = *guard {
            if now.duration_since(prev_time) < std::time::Duration::from_millis(2000) {
                return (cached_rss, cached_cpu);
            }
        }

        let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
        let res = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
        if res != 0 {
            if let Some((_, _, cached_cpu, cached_rss)) = *guard {
                return (cached_rss, cached_cpu);
            }
            return (0.0, 0.0);
        }
        let usage = unsafe { usage.assume_init() };

        #[cfg(target_os = "macos")]
        let rss_bytes = usage.ru_maxrss as f64;
        #[cfg(not(target_os = "macos"))]
        let rss_bytes = (usage.ru_maxrss * 1024) as f64;
        let rss_mb = rss_bytes / (1024.0 * 1024.0);

        let user_sec = usage.ru_utime.tv_sec as f64 + (usage.ru_utime.tv_usec as f64 / 1_000_000.0);
        let sys_sec = usage.ru_stime.tv_sec as f64 + (usage.ru_stime.tv_usec as f64 / 1_000_000.0);
        let total_cpu_sec = user_sec + sys_sec;

        let mut cpu_pct = 0.0;
        if let Some((prev_cpu, prev_time, _, _)) = *guard {
            let delta_cpu = total_cpu_sec - prev_cpu;
            let delta_time = now.duration_since(prev_time).as_secs_f64();
            if delta_time > 0.0 {
                cpu_pct = (delta_cpu / delta_time) * 100.0;
            }
        }
        *guard = Some((total_cpu_sec, now, cpu_pct, rss_mb));

        (rss_mb, cpu_pct)
    } else {
        (0.0, 0.0)
    }
}

#[cfg(not(unix))]
pub(crate) fn get_process_stats(_app: &App) -> (f64, f64) {
    (0.0, 0.0)
}
