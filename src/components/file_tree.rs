#[derive(Default)]
pub struct FileTreeComponent {
    pub queue: crate::queue::Queue,
    pub expanded_folders: std::collections::HashSet<String>,
    pub visible_files: Vec<crate::app::FileTreeItem>,
    pub file_list_selection: usize,
    pub file_content_scroll: usize,
    pub file_list_state: std::cell::RefCell<ratatui::widgets::ListState>,
    pub show_blame: bool,
    pub file_blame: Option<Vec<crate::repo::BlameEntry>>,
    pub blamed_file_path: Option<String>,
    pub show_line_numbers: bool,
}
use crate::app::{App, DetailSection, Mode};
use crate::components::commit_list::draw_commit_details_widget;
use crate::repo;
use crate::repo::FileEntry;
use crate::repo::{CommitEntry, DiffLine, RepoInfo, WorktreeChanges};
use crate::repo::{DiffLineKind, RemoteInfo};
use crate::ui::layout::{centered_rect, centered_rect_fixed};
use crate::ui::style::{
    ACCENT, CARD_BORDER, DANGER, SUCCESS, WARNING, accent_style, muted_style, parse_color,
    primary_style,
};
use crate::ui_detail::{DetailAreas, error_style, file_entry_line, read_file_content};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, Clear, Gauge, List, ListItem, ListState, Padding, Paragraph,
    Row, Table, Wrap,
};

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
                let (prefix, prefix_style, name_style) = if item.is_dir {
                    if item.is_expanded {
                        (
                            files_tab_sym("folder_tree_expanded"),
                            Style::default().fg(ACCENT()),
                            Style::default().fg(ACCENT()),
                        )
                    } else {
                        (
                            files_tab_sym("folder_tree_collapsed"),
                            Style::default().fg(ACCENT()),
                            Style::default().fg(ACCENT()),
                        )
                    }
                } else {
                    (files_tab_sym("file_tree"), muted_style(), primary_style())
                };

                let mut spans = vec![
                    Span::raw(indent),
                    Span::styled(prefix, prefix_style),
                    Span::styled(item.name.clone(), name_style),
                ];
                if !item.is_dir && info.lfs_files.contains(&item.full_path) {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled("[LFS]", accent_style()));
                }
                ListItem::new(Line::from(spans))
            })
            .collect();

        let left_inner = left_block.inner(chunks[0]);
        areas.files_inner = Some(left_inner);

        let list = List::new(items)
            .block(left_block)
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        let mut state = app.file_tree.file_list_state.borrow_mut();
        state.select(Some(file_list_selection));

        f.render_stateful_widget(list, chunks[0], &mut *state);
    }

    // Right panel: file preview or folder contents
    if let Some(selected_item) = visible_files.get(file_list_selection) {
        if selected_item.is_dir {
            // Selected item is a directory: list its direct contents
            let folder_name = if selected_item.name.is_empty() { "/" } else { &selected_item.name };
            let right_focused = focus == DetailSection::FileContent;
            let right_border_style =
                if right_focused { Style::default().fg(ACCENT()) } else { muted_style() };

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

            let total_lines = if children_vec.is_empty() { 1 } else { children_vec.len() };
            let file_content_scroll = file_content_scroll.min(total_lines.saturating_sub(1));

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
                            Span::styled(files_tab_sym("folder"), Style::default().fg(ACCENT())),
                            Span::raw(" "),
                            Span::styled(name, Style::default().fg(ACCENT())),
                        ]));
                    } else {
                        lines.push(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(files_tab_sym("file"), muted_style()),
                            Span::raw(" "),
                            Span::styled(name, primary_style()),
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

            let file_path = resolved.join(&selected_item.full_path);
            let content_text = match read_file_content(&file_path) {
                Ok(content) => content,
                Err(e) => format!("Could not read file: {}", e),
            };
            let lines: Vec<Line> =
                content_text.lines().map(crate::ui::syntax::highlight_code_line).collect();

            let file_content_scroll = file_content_scroll.min(lines.len().saturating_sub(1));

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

            if files_full_screen && (app.file_tree.show_line_numbers || app.file_tree.show_blame) {
                let inner_area = right_block.inner(chunks[1]);
                f.render_widget(right_block, chunks[1]);

                let max_line_digits = lines.len().to_string().len().max(3);
                let line_no_col_width = (max_line_digits + 2) as u16;

                let mut constraints = Vec::new();
                if app.file_tree.show_blame {
                    constraints.push(Constraint::Length(28));
                }
                if app.file_tree.show_line_numbers {
                    constraints.push(Constraint::Length(line_no_col_width));
                }
                constraints.push(Constraint::Min(0));

                let sub_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(constraints)
                    .split(inner_area);

                let mut chunk_idx = 0;

                if app.file_tree.show_blame {
                    let mut blame_lines = Vec::new();
                    for i in 1..=lines.len() {
                        let mut spans = Vec::new();
                        if let Some(ref blames) = app.file_tree.file_blame {
                            if let Some(entry) = blames.iter().find(|e| e.line_no == i) {
                                let first_name =
                                    entry.author.split_whitespace().next().unwrap_or("");
                                let author = if first_name.len() > 8 {
                                    format!("{}...", &first_name[..5])
                                } else {
                                    format!("{:>8}", first_name)
                                };
                                spans.push(Span::styled(
                                    format!("{:<7} ", entry.commit_id),
                                    Style::default().fg(ACCENT()),
                                ));
                                spans.push(Span::styled(format!("{:<8} ", author), muted_style()));
                                spans.push(Span::styled(format!("{} ", entry.date), muted_style()));
                            } else {
                                spans.push(Span::styled("        ".to_string(), muted_style()));
                                spans.push(Span::styled("         ".to_string(), muted_style()));
                                spans.push(Span::styled("           ".to_string(), muted_style()));
                            }
                        } else {
                            spans.push(Span::styled("        ".to_string(), muted_style()));
                            spans.push(Span::styled("         ".to_string(), muted_style()));
                            spans.push(Span::styled("           ".to_string(), muted_style()));
                        }
                        blame_lines.push(Line::from(spans));
                    }
                    let blame_body =
                        Paragraph::new(blame_lines).scroll((file_content_scroll as u16, 0));
                    f.render_widget(blame_body, sub_chunks[chunk_idx]);
                    chunk_idx += 1;
                }

                if app.file_tree.show_line_numbers {
                    let mut line_no_lines = Vec::new();
                    for i in 1..=lines.len() {
                        line_no_lines.push(Line::from(vec![Span::styled(
                            format!("{:>width$} │", i, width = max_line_digits),
                            muted_style(),
                        )]));
                    }
                    let line_no_body =
                        Paragraph::new(line_no_lines).scroll((file_content_scroll as u16, 0));
                    f.render_widget(line_no_body, sub_chunks[chunk_idx]);
                    chunk_idx += 1;
                }

                let body = Paragraph::new(lines)
                    .wrap(Wrap { trim: false })
                    .scroll((file_content_scroll as u16, 0));
                f.render_widget(body, sub_chunks[chunk_idx]);
            } else {
                let body = Paragraph::new(lines)
                    .block(right_block)
                    .wrap(Wrap { trim: false })
                    .scroll((file_content_scroll as u16, 0));
                f.render_widget(body, chunks[1]);
            }
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
        .border_type(CARD_BORDER())
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
        let empty_set = std::collections::HashSet::new();
        let lfs_files =
            if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
                &info.lfs_files
            } else {
                &empty_set
            };
        let items: Vec<ListItem> = commit
            .files
            .iter()
            .map(|f| ListItem::new(file_entry_line(f, lfs_files.contains(&f.path))))
            .collect();
        let list =
            List::new(items).highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        let mut state = app.status_list.changed_files_list_state.borrow_mut();
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
        .border_type(CARD_BORDER())
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
                    DiffLineKind::Header => Style::default().fg(ACCENT()),
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

impl FileTreeComponent {
    pub fn file_content_scroll_up(&mut self) {
        self.file_content_scroll = self.file_content_scroll.saturating_sub(1);
    }
    pub fn file_content_scroll_down(&mut self) {
        self.file_content_scroll = self.file_content_scroll.saturating_add(1);
    }
    pub fn file_content_scroll_page_up(&mut self, page: usize) {
        self.file_content_scroll = self.file_content_scroll.saturating_sub(page);
    }
    pub fn file_content_scroll_page_down(&mut self, page: usize) {
        self.file_content_scroll = self.file_content_scroll.saturating_add(page);
    }
    pub fn file_content_scroll_to_top(&mut self) {
        self.file_content_scroll = 0;
    }
    pub fn file_content_scroll_to_bottom(&mut self) {
        self.file_content_scroll = usize::MAX;
    }
}

impl FileTreeComponent {
    pub fn new(queue: crate::queue::Queue) -> Self {
        Self { queue, show_line_numbers: true, ..Default::default() }
    }
}

use crate::components::{Component, DrawableComponent, EventState};
use crate::queue::InternalEvent;
use crossterm::event::{Event, KeyCode};

impl DrawableComponent for FileTreeComponent {
    fn draw(&self, _f: &mut ratatui::Frame, _rect: ratatui::layout::Rect) -> std::io::Result<()> {
        Ok(())
    }
}

impl Component for FileTreeComponent {
    fn event(&mut self, ev: &Event) -> std::io::Result<EventState> {
        if let Event::Key(key) = ev {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    self.queue.push(InternalEvent::FileTreeUp);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    self.queue.push(InternalEvent::FileTreeDown);
                    return Ok(EventState::Consumed);
                }
                KeyCode::PageUp => {
                    self.queue.push(InternalEvent::FileTreePageUp);
                    return Ok(EventState::Consumed);
                }
                KeyCode::PageDown => {
                    self.queue.push(InternalEvent::FileTreePageDown);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Home => {
                    self.queue.push(InternalEvent::FileTreeTop);
                    return Ok(EventState::Consumed);
                }
                KeyCode::End => {
                    self.queue.push(InternalEvent::FileTreeBottom);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Enter | KeyCode::Right => {
                    self.queue.push(InternalEvent::ToggleFolderExpanded);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Left => {
                    self.queue.push(InternalEvent::CollapseAllFolders);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Char('x') | KeyCode::Char('X') => {
                    self.queue.push(InternalEvent::RequestDiscardFile);
                    return Ok(EventState::Consumed);
                }
                _ => {}
            }
        }
        Ok(EventState::NotConsumed)
    }
}

fn files_tab_sym(key: &str) -> &'static str {
    match key {
        "folder_tree_expanded" => "v ",
        "folder_tree_collapsed" => "> ",
        "file_tree" => "  -  ",
        "folder" => "[D]",
        "file" => "[F]",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn test_file_tree_component() {
        let queue = crate::queue::Queue::default();
        let mut component = FileTreeComponent::new(queue.clone());

        component.file_content_scroll_up();
        component.file_content_scroll_down();
        component.file_content_scroll_page_up(1);
        component.file_content_scroll_page_down(1);
        component.file_content_scroll_to_top();
        component.file_content_scroll_to_bottom();

        let key = |code: KeyCode| Event::Key(KeyEvent::new(code, KeyModifiers::empty()));

        // Test event methods
        assert!(component.event(&key(KeyCode::Up)).unwrap().is_consumed());
        assert!(component.event(&key(KeyCode::Down)).unwrap().is_consumed());
        assert!(component.event(&key(KeyCode::PageUp)).unwrap().is_consumed());
        assert!(component.event(&key(KeyCode::PageDown)).unwrap().is_consumed());
        assert!(component.event(&key(KeyCode::Home)).unwrap().is_consumed());
        assert!(component.event(&key(KeyCode::End)).unwrap().is_consumed());
        assert!(component.event(&key(KeyCode::Enter)).unwrap().is_consumed());
        assert!(component.event(&key(KeyCode::Left)).unwrap().is_consumed());
        assert!(component.event(&key(KeyCode::Char('x'))).unwrap().is_consumed());

        // Test draw functions
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let _ = component.draw(f, Rect::new(0, 0, 80, 24));

                let info = RepoInfo {
                    files: repo::TabData::Loaded(vec!["file.txt".to_string()]),
                    ..RepoInfo::default()
                };
                let visible = vec![
                    crate::app::FileTreeItem {
                        name: "file.txt".to_string(),
                        full_path: "file.txt".to_string(),
                        is_dir: false,
                        is_expanded: false,
                        depth: 0,
                    },
                    crate::app::FileTreeItem {
                        name: "dir".to_string(),
                        full_path: "dir".to_string(),
                        is_dir: true,
                        is_expanded: true,
                        depth: 0,
                    },
                ];
                let mut areas = DetailAreas::default();
                let config = crate::config::Config::default();
                let app = App::new(config, std::path::PathBuf::from("test.toml"));

                draw_files_view(
                    f,
                    std::path::Path::new("/tmp"),
                    &info,
                    &visible,
                    DetailSection::Files,
                    0,
                    0,
                    &mut areas,
                    50,
                    &app,
                    Rect::new(0, 0, 80, 24),
                );

                let commit = CommitEntry {
                    id: "abc".to_string(),
                    oid: "abc".to_string(),
                    author: "author".to_string(),
                    when: "10 mins ago".to_string(),
                    date: "2026-07-02".to_string(),
                    summary: "summary".to_string(),
                    message: "msg".to_string(),
                    refs: vec![],
                    files: vec![],
                    signature_status: "G".to_string(),
                };
                draw_commit_files_panel(
                    f,
                    &commit,
                    DetailSection::Staged,
                    0,
                    &[],
                    0,
                    0,
                    &mut areas,
                    50,
                    50,
                    &app,
                    Rect::new(0, 0, 80, 24),
                );
            })
            .unwrap();
    }
}
