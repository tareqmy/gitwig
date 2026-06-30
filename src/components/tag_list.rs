#[derive(Default)]
pub struct TagListComponent {
    pub queue: crate::queue::Queue,
    pub local_tag_selection: usize,
    pub remote_tag_selection: usize,
    pub local_tag_list_state: std::cell::RefCell<ratatui::widgets::ListState>,
}
use crate::app::{App, DetailSection, Mode};
use crate::repo;
use crate::repo::FileEntry;
use crate::repo::RemoteInfo;
use crate::repo::{CommitEntry, DiffLine, RepoInfo, WorktreeChanges};
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

pub fn draw_tags_view(
    f: &mut Frame,
    info: &RepoInfo,
    focus: DetailSection,
    local_tag_selection: usize,
    remote_tags_loaded: bool,
    areas: &mut DetailAreas,
    app: &crate::app::App,
    area: Rect,
) {
    if info.local_tags.is_loading() || info.local_tags.is_not_loaded() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(muted_style())
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Tags", primary_style()),
                Span::raw(" "),
            ]))
            .padding(Padding::uniform(1));
        let inner = block.inner(area);
        f.render_widget(block, area);
        let loading_text = Paragraph::new("⟳ Loading tags...")
            .style(muted_style())
            .alignment(ratatui::layout::Alignment::Center);
        let center_area = Rect::new(inner.x, inner.y + inner.height / 2, inner.width, 1);
        f.render_widget(loading_text, center_area);
        return;
    }
    if let repo::TabData::Error(err) = &info.local_tags {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(error_style())
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Tags - Error", error_style()),
                Span::raw(" "),
            ]))
            .padding(Padding::uniform(1));
        let inner = block.inner(area);
        f.render_widget(block, area);
        let error_text = Paragraph::new(format!("Error loading tags: {}", err))
            .style(error_style())
            .wrap(Wrap { trim: false });
        f.render_widget(error_text, inner);
        return;
    }

    areas.local_tags = Some(area);
    areas.remote_tags = None;

    // ── Local Tags Panel ──
    let local_focused = focus == DetailSection::LocalTags;
    let local_border_style =
        if local_focused { Style::default().fg(ACCENT()) } else { muted_style() };
    let local_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(local_border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Local Tags", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::uniform(1));

    let local_items: Vec<ListItem> = info
        .local_tags
        .iter()
        .map(|t| {
            let mut spans = vec![Span::styled("  ", Style::default())];
            if !t.short_sha.is_empty() {
                spans.push(Span::styled(format!("[{}]", t.short_sha), accent_style()));
                spans.push(Span::raw("  "));
            }
            spans.push(Span::styled(t.name.clone(), primary_style()));
            if !t.short_message.is_empty() {
                spans.push(Span::raw(" "));
                spans.push(Span::styled("·", muted_style()));
                spans.push(Span::raw(" "));
                spans.push(Span::styled(t.short_message.clone(), muted_style()));
            }

            let is_pushed = if info.remotes.is_empty() {
                true
            } else if remote_tags_loaded {
                info.remote_tags.iter().any(|rt| rt.name == t.name)
            } else {
                true
            };

            if !is_pushed {
                spans.push(Span::raw("  "));
                spans.push(Span::styled("unpushed", Style::default().fg(WARNING())));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let inner = local_block.inner(area);
    areas.local_tags_inner = Some(inner);

    let local_list = List::new(local_items)
        .block(local_block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut local_state = app.tag_list.local_tag_list_state.borrow_mut();
    if local_focused {
        local_state.select(Some(local_tag_selection));
    } else {
        local_state.select(None);
    }
    f.render_stateful_widget(local_list, area, &mut *local_state);
}

impl TagListComponent {
    pub fn new(queue: crate::queue::Queue) -> Self {
        Self { queue, ..Default::default() }
    }
}

use crate::components::{Component, DrawableComponent, EventState};
use crate::queue::InternalEvent;
use crossterm::event::{Event, KeyCode};

impl DrawableComponent for TagListComponent {
    fn draw(&self, _f: &mut ratatui::Frame, _rect: ratatui::layout::Rect) -> std::io::Result<()> {
        Ok(())
    }
}

impl Component for TagListComponent {
    fn event(&mut self, ev: &Event) -> std::io::Result<EventState> {
        if let Event::Key(key) = ev {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    self.queue.push(InternalEvent::TagUp);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    self.queue.push(InternalEvent::TagDown);
                    return Ok(EventState::Consumed);
                }
                KeyCode::PageUp => {
                    self.queue.push(InternalEvent::TagPageUp);
                    return Ok(EventState::Consumed);
                }
                KeyCode::PageDown => {
                    self.queue.push(InternalEvent::TagPageDown);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Home => {
                    self.queue.push(InternalEvent::TagTop);
                    return Ok(EventState::Consumed);
                }
                KeyCode::End => {
                    self.queue.push(InternalEvent::TagBottom);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Enter => {
                    self.queue.push(InternalEvent::CheckoutTag);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Char('D') => {
                    self.queue.push(InternalEvent::RequestDeleteTag);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Char('p') => {
                    self.queue.push(InternalEvent::RequestPushTag);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Char('P') => {
                    self.queue.push(InternalEvent::RequestPushAllTags);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Char('f') | KeyCode::Char('F') => {
                    self.queue.push(InternalEvent::FetchRemoteTags);
                    return Ok(EventState::Consumed);
                }
                _ => {}
            }
        }
        Ok(EventState::NotConsumed)
    }
}
