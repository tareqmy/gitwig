use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect, Margin, Position};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap, Padding, Gauge, List, ListItem, ListState, Table, Row, Cell};
use crate::app::{App, Mode, DetailSection};
use crate::repo::{RemoteInfo, DiffLineKind};
use crate::ui::style::{accent_style, muted_style, primary_style, ACCENT, CARD_BORDER, DANGER, SUCCESS, WARNING, parse_color};
use crate::ui::layout::{centered_rect, centered_rect_fixed};
use crate::ui_detail::{error_style, read_file_content, file_entry_line, DetailAreas};
use crate::repo::FileEntry;
use crate::repo;
use crate::components::commit_list::draw_commit_details_widget;
use crate::repo::{RepoInfo, CommitEntry, DiffLine, WorktreeChanges};


pub fn draw_files_view(
    f: &mut Frame,
    resolved: &std::path::Path,
    info: &RepoInfo,
    visible_files: &[crate::app::FileTreeItem],
    focus: DetailSection,
    file_list_selection: usize,
    file_content_scroll: usize,
    areas: &mut DetailAreas,
    files_horizontal_split_pct: u16,
    app: &crate::app::App,
    area: Rect,
) {
    if info.files.is_loading() || info.files.is_not_loaded() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(muted_style())
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Files", primary_style()),
                Span::raw(" "),
            ]))
            .padding(Padding::uniform(1));
        let inner = block.inner(area);
        f.render_widget(block, area);
        let loading_text = Paragraph::new("⟳ Loading files...")
            .style(muted_style())
            .alignment(ratatui::layout::Alignment::Center);
        let center_area = Rect::new(inner.x, inner.y + inner.height / 2, inner.width, 1);
        f.render_widget(loading_text, center_area);
        return;
    }
    if let repo::TabData::Error(err) = &info.files {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(error_style())
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Files - Error", error_style()),
                Span::raw(" "),
            ]))
            .padding(Padding::uniform(1));
        let inner = block.inner(area);
        f.render_widget(block, area);
        let error_text = Paragraph::new(format!("Error loading files: {}", err))
            .style(error_style())
            .wrap(Wrap { trim: false });
        f.render_widget(error_text, inner);
        return;
    }

    let files_full_screen = app.inspect_full_diff;
    let chunks = if files_full_screen {
        let left_rect = Rect::new(area.x, area.y, 0, area.height);
        vec![left_rect, area]
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(files_horizontal_split_pct),
                Constraint::Percentage(100 - files_horizontal_split_pct),
            ])
            .split(area)
            .to_vec()
    };

    areas.files = Some(chunks[0]);
    areas.file_content = Some(chunks[1]);

    // Record horizontal splitter boundary in files tab
    if files_full_screen {
        areas.files_horizontal_splitter = None;
    } else {
        let split_col = area.x + chunks[0].width;
        areas.files_horizontal_splitter =
            Some(Rect::new(split_col.saturating_sub(1), area.y, 2, area.height));
    }

    if !files_full_screen {
        let focused = focus == DetailSection::Files;
        let border_style = if focused { Style::default().fg(ACCENT()) } else { muted_style() };

        let left_block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(border_style)
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Repository Files", primary_style()),
                Span::raw(" "),
            ]))
            .padding(Padding::uniform(1));

        let items: Vec<ListItem> = visible_files
            .iter()
            .map(|item| {
                let indent = "  ".repeat(item.depth);
                let (prefix, style) = if item.is_dir {
                    if item.is_expanded {
                        (app.sym("folder_tree_expanded"), primary_style())
                    } else {
                        (app.sym("folder_tree_collapsed"), primary_style())
                    }
                } else {
                    (app.sym("file_tree"), muted_style())
                };

                ListItem::new(Line::from(vec![
                    Span::raw(indent),
                    Span::styled(prefix, style),
                    Span::styled(item.name.clone(), primary_style()),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(left_block)
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        let mut state = ListState::default();
        state.select(Some(file_list_selection));

        f.render_stateful_widget(list, chunks[0], &mut state);
    }

    // Right panel: file preview or folder contents
    if let Some(selected_item) = visible_files.get(file_list_selection) {
        if selected_item.is_dir {
            // Selected item is a directory: list its direct contents
            let folder_name = if selected_item.name.is_empty() { "/" } else { &selected_item.name };
            let right_focused = focus == DetailSection::FileContent;
            let right_border_style =
                if right_focused { Style::default().fg(ACCENT()) } else { muted_style() };

            let mut title_spans = vec![
                Span::raw(" "),
                Span::styled(format!("Contents of {}", folder_name), primary_style()),
            ];
            if right_focused && file_content_scroll > 0 {
                title_spans.push(Span::styled(
                    format!("  ↕ line {}", file_content_scroll + 1),
                    muted_style(),
                ));
            }
            title_spans.push(Span::raw(" "));

            let right_block = Block::default()
                .borders(Borders::ALL)
                .border_type(CARD_BORDER())
                .border_style(right_border_style)
                .title(Line::from(title_spans))
                .padding(Padding::uniform(1));

            let prefix = if selected_item.full_path.is_empty() {
                "".to_string()
            } else {
                format!("{}/", selected_item.full_path)
            };

            let mut direct_children = std::collections::BTreeSet::new();
            for f_path in info.files.iter() {
                if f_path.starts_with(&prefix) {
                    let relative = &f_path[prefix.len()..];
                    if let Some(idx) = relative.find('/') {
                        let subdir = &relative[..idx];
                        direct_children.insert((subdir.to_string(), true));
                    } else {
                        direct_children.insert((relative.to_string(), false));
                    }
                }
            }

            let mut children_vec: Vec<(String, bool)> = direct_children.into_iter().collect();
            children_vec.sort_by(|a, b| match (a.1, b.1) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.0.cmp(&b.0),
            });

            let mut lines = Vec::new();
            if children_vec.is_empty() {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("(empty folder)", muted_style()),
                ]));
            } else {
                for (name, is_dir) in children_vec {
                    if is_dir {
                        lines.push(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(app.sym("folder"), Style::default().fg(ACCENT())),
                            Span::styled(name, primary_style()),
                        ]));
                    } else {
                        lines.push(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(app.sym("file"), muted_style()),
                            Span::raw(name),
                        ]));
                    }
                }
            }

            let body = Paragraph::new(lines)
                .block(right_block)
                .wrap(Wrap { trim: false })
                .scroll((file_content_scroll as u16, 0));
            f.render_widget(body, chunks[1]);
        } else {
            // Selected item is a file: show its contents
            let right_focused = focus == DetailSection::FileContent;
            let right_border_style =
                if right_focused { Style::default().fg(ACCENT()) } else { muted_style() };

            let mut title_spans = vec![
                Span::raw(" "),
                Span::styled(format!("Content of {}", selected_item.name), primary_style()),
            ];
            if right_focused && file_content_scroll > 0 {
                title_spans.push(Span::styled(
                    format!("  ↕ line {}", file_content_scroll + 1),
                    muted_style(),
                ));
            }
            title_spans.push(Span::raw(" "));

            let right_block = Block::default()
                .borders(Borders::ALL)
                .border_type(CARD_BORDER())
                .border_style(right_border_style)
                .title(Line::from(title_spans))
                .padding(Padding::uniform(1));

            let file_path = resolved.join(&selected_item.full_path);
            let content_text = match read_file_content(&file_path) {
                Ok(content) => content,
                Err(e) => format!("Could not read file: {}", e),
            };
            let body = Paragraph::new(content_text)
                .block(right_block)
                .wrap(Wrap { trim: false })
                .scroll((file_content_scroll as u16, 0));
            f.render_widget(body, chunks[1]);
        }
    } else {
        // Fallback: render an empty block on the right
        let right_block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(muted_style())
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Content", primary_style()),
                Span::raw(" "),
            ]));
        f.render_widget(right_block, chunks[1]);
    }
}

pub fn draw_commit_files_panel(
    f: &mut Frame,
    commit: &CommitEntry,
    focus: DetailSection,
    file_selection: usize,
    file_diff: &[DiffLine],
    diff_scroll: usize,
    commit_details_scroll: usize,
    areas: &mut DetailAreas,
    inspect_horizontal_split_pct: u16,
    inspect_vertical_split_pct: u16,
    app: &crate::app::App,
    area: Rect,
) {
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(inspect_horizontal_split_pct),
            Constraint::Percentage(100 - inspect_horizontal_split_pct),
        ])
        .split(area);

    areas.bottom_right = Some(panels[1]);
    areas.staged_sub = None;
    areas.unstaged_sub = None;

    // Record horizontal splitter boundary
    let split_col = area.x + panels[0].width;
    areas.inspect_horizontal_splitter =
        Some(Rect::new(split_col.saturating_sub(1), area.y, 2, area.height));

    let left_focused = focus == DetailSection::Staged || focus == DetailSection::Unstaged;
    let right_focused = focus == DetailSection::StagingDetails;

    // Split left panel vertically: top is Changed Files list, bottom is Commit Details
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(inspect_vertical_split_pct),
            Constraint::Percentage(100 - inspect_vertical_split_pct),
        ])
        .split(panels[0]);

    // Record vertical splitter boundary
    let split_row = panels[0].y + left_chunks[0].height;
    areas.inspect_vertical_splitter =
        Some(Rect::new(panels[0].x, split_row.saturating_sub(1), panels[0].width, 2));

    areas.bottom_left = Some(left_chunks[0]);
    areas.commit_details = Some(left_chunks[1]);

    // ── Left Top: changed files ───────────────────────────────────────────────
    let left_border_style =
        if left_focused { Style::default().fg(ACCENT()) } else { muted_style() };
    let left_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(left_border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Changed Files", primary_style()),
            Span::raw("  "),
            Span::styled(format!("({})", commit.files.len()), muted_style()),
            Span::raw(" "),
        ]));
    let left_inner = left_block.inner(left_chunks[0]);
    areas.changed_files_inner = Some(left_inner);
    f.render_widget(left_block, left_chunks[0]);

    if commit.files.is_empty() {
        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(40), Constraint::Length(1), Constraint::Min(0)])
            .split(left_inner);
        f.render_widget(
            Paragraph::new(Span::styled("No files changed", muted_style()))
                .alignment(Alignment::Center),
            v[1],
        );
    } else {
        let items: Vec<ListItem> =
            commit.files.iter().map(|f| ListItem::new(file_entry_line(f))).collect();
        let list =
            List::new(items).highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        let mut state = app.changed_files_list_state.borrow_mut();
        if left_focused {
            state.select(Some(file_selection));
        } else {
            state.select(None);
        }
        f.render_stateful_widget(list, left_inner, &mut *state);
    }

    // ── Left Bottom: commit details ───────────────────────────────────────────
    draw_commit_details_widget(
        f,
        commit,
        focus == DetailSection::CommitDetails,
        commit_details_scroll,
        left_chunks[1],
    );
    areas.commit_details = Some(left_chunks[1]);

    // ── Right: diff panel ─────────────────────────────────────────────────
    let right_border_style =
        if right_focused { Style::default().fg(ACCENT()) } else { muted_style() };
    let right_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(right_border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Diff", primary_style()),
            if right_focused && diff_scroll > 0 {
                Span::styled(format!("  ↕ line {}", diff_scroll + 1), muted_style())
            } else {
                Span::raw("")
            },
            if right_focused {
                Span::styled(format!("  {} scroll", app.sym("up_down")), muted_style())
            } else {
                Span::raw("")
            },
            Span::raw(" "),
        ]));
    let right_inner = right_block.inner(panels[1]);
    f.render_widget(right_block, panels[1]);

    if file_diff.is_empty() {
        let v_center = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(45), Constraint::Length(1), Constraint::Min(0)])
            .split(right_inner);
        f.render_widget(
            Paragraph::new(Span::styled("Select a file to view its diff", muted_style()))
                .alignment(Alignment::Center),
            v_center[1],
        );
    } else {
        let diff_spans: Vec<Line> = file_diff
            .iter()
            .map(|line| {
                let style = match line.kind {
                    DiffLineKind::Added => Style::default().fg(SUCCESS()),
                    DiffLineKind::Removed => Style::default().fg(DANGER()),
                    DiffLineKind::Header => Style::default().fg(ratatui::style::Color::Cyan),
                    DiffLineKind::Context => Style::default(),
                    DiffLineKind::ConflictOurs => {
                        Style::default().fg(ratatui::style::Color::LightRed)
                    }
                    DiffLineKind::ConflictTheirs => {
                        Style::default().fg(ratatui::style::Color::LightBlue)
                    }
                    DiffLineKind::ConflictSeparator => Style::default()
                        .fg(ratatui::style::Color::Yellow)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                };
                Line::from(Span::styled(line.content.clone(), style))
            })
            .collect();
        f.render_widget(
            Paragraph::new(diff_spans).scroll((diff_scroll as u16, 0)).wrap(Wrap { trim: false }),
            right_inner,
        );
    }
}

