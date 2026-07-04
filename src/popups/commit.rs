use crate::components::{Component, DrawableComponent, EventState};
use crate::queue::{InternalEvent, Queue};
use crossterm::event::{Event, KeyCode, KeyModifiers};
use std::cell::Cell;

pub struct CommitPopup {
    pub queue: Queue,
    pub input_buffer: String,
    pub editing: bool,
    pub amend: bool,
    pub scroll: Cell<usize>,
    pub maximized: bool,
    pub width_pct: u16,
    pub height_pct: u16,
    pub cursor_idx: usize,
    pub last_height: Cell<usize>,
}

impl CommitPopup {
    pub fn new(queue: Queue) -> Self {
        Self {
            queue,
            input_buffer: String::new(),
            editing: true,
            amend: false,
            scroll: Cell::new(0),
            maximized: false,
            width_pct: 60,
            height_pct: 60,
            cursor_idx: 0,
            last_height: Cell::new(8),
        }
    }

    pub fn char_idx_to_line_col(&self, char_idx: usize) -> (usize, usize) {
        char_idx_to_line_col(&self.input_buffer, char_idx)
    }

    pub fn line_col_to_char_idx(&self, line: usize, col: usize) -> usize {
        line_col_to_char_idx(&self.input_buffer, line, col)
    }

    pub fn adjust_scroll(&mut self) {
        let (line, _) = self.char_idx_to_line_col(self.cursor_idx);
        let height = self.last_height.get().max(2);
        let scroll_val = self.scroll.get();
        if line < scroll_val {
            self.scroll.set(line);
        } else if line >= scroll_val + height {
            self.scroll.set(line - height + 1);
        }
    }

    fn insert_char(&mut self, c: char) {
        let mut idx = 0;
        let mut new_str = String::with_capacity(self.input_buffer.len() + 4);
        for ch in self.input_buffer.chars() {
            if idx == self.cursor_idx {
                new_str.push(c);
            }
            new_str.push(ch);
            idx += 1;
        }
        if idx == self.cursor_idx {
            new_str.push(c);
        }
        self.input_buffer = new_str;
        self.cursor_idx += 1;
        self.adjust_scroll();
    }

    fn backspace(&mut self) {
        if self.cursor_idx > 0 {
            let mut idx = 0;
            let mut new_str = String::with_capacity(self.input_buffer.len());
            for ch in self.input_buffer.chars() {
                if idx != self.cursor_idx - 1 {
                    new_str.push(ch);
                }
                idx += 1;
            }
            self.input_buffer = new_str;
            self.cursor_idx -= 1;
            self.adjust_scroll();
        }
    }

    fn delete_char(&mut self) {
        if self.cursor_idx < self.input_buffer.chars().count() {
            let mut idx = 0;
            let mut new_str = String::with_capacity(self.input_buffer.len());
            for ch in self.input_buffer.chars() {
                if idx != self.cursor_idx {
                    new_str.push(ch);
                }
                idx += 1;
            }
            self.input_buffer = new_str;
            self.adjust_scroll();
        }
    }

    fn delete_word_before_cursor(&mut self) {
        if self.cursor_idx > 0 {
            let chars: Vec<char> = self.input_buffer.chars().collect();
            let mut idx = self.cursor_idx;
            while idx > 0 && chars[idx - 1].is_whitespace() {
                idx -= 1;
            }
            while idx > 0 && !chars[idx - 1].is_whitespace() {
                idx -= 1;
            }
            let delete_count = self.cursor_idx - idx;
            if delete_count > 0 {
                let mut new_str = String::new();
                for (i, &ch) in chars.iter().enumerate() {
                    if i < idx || i >= self.cursor_idx {
                        new_str.push(ch);
                    }
                }
                self.input_buffer = new_str;
                self.cursor_idx = idx;
                self.adjust_scroll();
            }
        }
    }

    fn kill_line_after_cursor(&mut self) {
        let chars: Vec<char> = self.input_buffer.chars().collect();
        let mut idx = self.cursor_idx;
        while idx < chars.len() && chars[idx] != '\n' {
            idx += 1;
        }
        if idx == self.cursor_idx && idx < chars.len() && chars[idx] == '\n' {
            idx += 1;
        }
        let delete_count = idx - self.cursor_idx;
        if delete_count > 0 {
            let mut new_str = String::new();
            for (i, &ch) in chars.iter().enumerate() {
                if i < self.cursor_idx || i >= idx {
                    new_str.push(ch);
                }
            }
            self.input_buffer = new_str;
            self.adjust_scroll();
        }
    }

    fn move_up(&mut self) {
        let (line, col) = self.char_idx_to_line_col(self.cursor_idx);
        if line > 0 {
            self.cursor_idx = self.line_col_to_char_idx(line - 1, col);
            self.adjust_scroll();
        }
    }

    fn move_down(&mut self) {
        let (line, col) = self.char_idx_to_line_col(self.cursor_idx);
        self.cursor_idx = self.line_col_to_char_idx(line + 1, col);
        self.adjust_scroll();
    }

    fn move_home(&mut self) {
        let (line, _) = self.char_idx_to_line_col(self.cursor_idx);
        self.cursor_idx = self.line_col_to_char_idx(line, 0);
        self.adjust_scroll();
    }

    fn move_end(&mut self) {
        let (line, _) = self.char_idx_to_line_col(self.cursor_idx);
        self.cursor_idx = self.line_col_to_char_idx(line, usize::MAX);
        self.adjust_scroll();
    }
}

pub fn char_idx_to_line_col(text: &str, char_idx: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;
    for (i, c) in text.chars().enumerate() {
        if i == char_idx {
            return (line, col);
        }
        if c == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (line, col)
}

pub fn line_col_to_char_idx(text: &str, line: usize, col: usize) -> usize {
    let mut current_line = 0;
    let mut current_col = 0;
    let mut line_start_char_idx = 0;
    for (i, c) in text.chars().enumerate() {
        if current_line == line {
            if current_col == col {
                return i;
            }
            if c == '\n' {
                return i;
            }
        }
        if c == '\n' {
            current_line += 1;
            current_col = 0;
            line_start_char_idx = i + 1;
        } else {
            current_col += 1;
        }
    }
    if current_line == line {
        return line_start_char_idx + col.min(current_col);
    }
    text.chars().count()
}

impl DrawableComponent for CommitPopup {
    fn draw(&self, _f: &mut ratatui::Frame, _rect: ratatui::layout::Rect) -> std::io::Result<()> {
        Ok(())
    }
}

impl Component for CommitPopup {
    fn event(&mut self, ev: &Event) -> std::io::Result<EventState> {
        if let Event::Key(key) = ev {
            if self.editing {
                match key.code {
                    KeyCode::Esc => {
                        self.queue.push(InternalEvent::ClosePopup);
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.queue.push(InternalEvent::Commit);
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.amend = !self.amend;
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.editing = false;
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.maximized = !self.maximized;
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.input_buffer.clear();
                        self.cursor_idx = 0;
                        self.scroll.set(0);
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.kill_line_after_cursor();
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.delete_word_before_cursor();
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if self.cursor_idx > 0 {
                            self.cursor_idx -= 1;
                            self.adjust_scroll();
                        }
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if self.cursor_idx < self.input_buffer.chars().count() {
                            self.cursor_idx += 1;
                            self.adjust_scroll();
                        }
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.move_up();
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.move_down();
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Char(c) => {
                        self.insert_char(c);
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Backspace => {
                        self.backspace();
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Delete => {
                        self.delete_char();
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Enter => {
                        self.insert_char('\n');
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Left => {
                        if self.cursor_idx > 0 {
                            self.cursor_idx -= 1;
                            self.adjust_scroll();
                        }
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Right => {
                        if self.cursor_idx < self.input_buffer.chars().count() {
                            self.cursor_idx += 1;
                            self.adjust_scroll();
                        }
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Up => {
                        self.move_up();
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Down => {
                        self.move_down();
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Home => {
                        self.move_home();
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::End => {
                        self.move_end();
                        return Ok(EventState::Consumed);
                    }
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                        self.queue.push(InternalEvent::ClosePopup);
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Char('e') | KeyCode::Char('E') => {
                        self.editing = true;
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Char('a') | KeyCode::Char(' ') => {
                        self.amend = !self.amend;
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Char('d')
                    | KeyCode::Char('D')
                    | KeyCode::Char('m')
                    | KeyCode::Char('M') => {
                        self.maximized = !self.maximized;
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Enter => {
                        self.queue.push(InternalEvent::Commit);
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                        self.scroll.set(self.scroll.get().saturating_sub(1));
                        return Ok(EventState::Consumed);
                    }
                    KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                        self.scroll.set(self.scroll.get().saturating_add(1));
                        return Ok(EventState::Consumed);
                    }
                    _ => {}
                }
            }
        }
        Ok(EventState::NotConsumed)
    }
}

use crate::app::{App, Mode};
use crate::repo::RemoteInfo;
use crate::ui::layout::{centered_rect, centered_rect_fixed};
use crate::ui::style::{
    ACCENT, CARD_BORDER, DANGER, SUCCESS, WARNING, accent_style, muted_style, parse_color,
    primary_style,
};
use crate::ui_detail::DetailAreas;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap,
};

pub fn draw_commit_popup(
    f: &mut Frame,
    input_buffer: &str,
    editing: bool,
    commit_amend: bool,
    _scroll: usize,
    area: Rect,
    app: &crate::app::App,
    areas: &mut DetailAreas,
) {
    let popup_area = if app.commit_popup.maximized {
        let width = area.width.saturating_sub(20).max(area.width.min(40));
        let height = area.height.saturating_sub(20).max(area.height.min(15));
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        Rect::new(x, y, width, height)
    } else {
        centered_rect(app.commit_popup_width_pct, app.commit_popup_height_pct, area)
    };
    areas.commit_popup = Some(popup_area);
    areas.commit_popup_parent = Some(area);
    f.render_widget(Clear, popup_area);

    let border_color = if editing { ACCENT() } else { WARNING() };
    let border_style = Style::default().fg(border_color);

    let title_text = if editing {
        if commit_amend { " Amend Commit Message " } else { " Commit Message " }
    } else {
        if commit_amend { " Confirm Amend Commit " } else { " Confirm Commit " }
    };

    let title =
        Line::from(vec![Span::raw(" "), Span::styled(title_text, primary_style()), Span::raw(" ")]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // Split inner area vertically: top is the commit message text area, bottom is the amend option.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner_area);

    // Sync app.commit_popup.last_height so adjust_scroll works correctly on next event
    let text_area_height = chunks[0].height as usize;
    app.commit_popup.last_height.set(text_area_height);

    let scroll_val = app.commit_popup.scroll.get();
    let text = if input_buffer.is_empty() {
        Paragraph::new(Span::styled("(type commit message here...)", muted_style()))
            .wrap(Wrap { trim: true })
            .scroll((scroll_val as u16, 0))
    } else {
        Paragraph::new(input_buffer)
            .wrap(Wrap { trim: true })
            .scroll((scroll_val as u16, 0))
    };

    f.render_widget(text, chunks[0]);

    // Render the amend checkbox.
    let checkbox = if commit_amend { "[X]" } else { "[ ]" };
    let checkbox_style = if commit_amend {
        Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)
    } else {
        muted_style()
    };
    let checkbox_line = Line::from(vec![
        Span::styled(format!("{} ", checkbox), checkbox_style),
        Span::styled("Amend last commit", primary_style()),
        if !editing {
            Span::styled(" (toggle: [a/space])", muted_style())
        } else {
            Span::styled(" (toggle: [⌃A])", muted_style())
        },
    ]);
    f.render_widget(Paragraph::new(checkbox_line), chunks[1]);

    if editing {
        let (cursor_line, cursor_col) = char_idx_to_line_col(input_buffer, app.commit_popup.cursor_idx);
        let cursor_y = chunks[0]
            .y
            .saturating_add(cursor_line.saturating_sub(scroll_val) as u16);
        let max_y = chunks[0].y.saturating_add(chunks[0].height.saturating_sub(1));
        let cursor_y = cursor_y.min(max_y);

        let cursor_x = chunks[0]
            .x
            .saturating_add(cursor_col as u16);
        let max_x = chunks[0].x.saturating_add(chunks[0].width.saturating_sub(1));
        let cursor_x = cursor_x.min(max_x);

        f.set_cursor_position(ratatui::layout::Position::new(cursor_x, cursor_y));
    }
}

pub struct GenericInputPopup {
    pub queue: Queue,
}

impl GenericInputPopup {
    pub fn new(queue: Queue) -> Self {
        Self { queue }
    }
}

impl DrawableComponent for GenericInputPopup {
    fn draw(&self, _f: &mut ratatui::Frame, _rect: ratatui::layout::Rect) -> std::io::Result<()> {
        Ok(())
    }
}

impl Component for GenericInputPopup {
    fn event(&mut self, ev: &Event) -> std::io::Result<EventState> {
        if let Event::Key(key) = ev {
            match key.code {
                KeyCode::Esc => {
                    self.queue.push(InternalEvent::InputEsc);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Enter => {
                    self.queue.push(InternalEvent::InputEnter);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Backspace => {
                    self.queue.push(InternalEvent::InputBackspace);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Char(c) => {
                    self.queue.push(InternalEvent::InputChar(c));
                    return Ok(EventState::Consumed);
                }
                _ => {}
            }
        }
        Ok(EventState::NotConsumed)
    }
}
