//! Picker popup for selecting historical commit messages.

use crate::app::{App, Mode};
use crate::ui::layout::centered_rect;
use crate::ui::style::{ACCENT, CARD_BORDER, accent_style, muted_style, primary_style};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, ListState, Padding, Paragraph, Wrap,
};

pub fn draw_commit_history_popup(f: &mut Frame, items: &[String], selection: usize, area: Rect) {
    let popup_area = centered_rect(50, 60, area);

    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Commit History", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, msg)| {
            let style = if i == selection {
                accent_style().add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                primary_style()
            };

            // Show only the first line of the commit message for the list
            let first_line = msg.lines().next().unwrap_or("").to_string();

            ListItem::new(Line::from(vec![Span::raw("  "), Span::styled(first_line, style)]))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(selection));
    f.render_stateful_widget(List::new(list_items), chunks[0], &mut list_state);

    let hint = Line::from(vec![
        Span::styled("↑↓ navigate  ", muted_style()),
        Span::styled("Enter", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" select  ", muted_style()),
        Span::styled("Esc", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" cancel", muted_style()),
    ]);
    f.render_widget(Paragraph::new(hint), chunks[1]);
}

use crossterm::event::{KeyCode, KeyEvent};
pub struct CommitHistoryPickerPopup;
impl CommitHistoryPickerPopup {
    pub fn handle_event(app: &mut crate::app::App, key: KeyEvent) -> bool {
        let code = key.code;
        match code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                app.queue.push(crate::queue::InternalEvent::CommitHistoryPickerUp)
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                app.queue.push(crate::queue::InternalEvent::CommitHistoryPickerDown)
            }
            KeyCode::Enter => {
                app.queue.push(crate::queue::InternalEvent::CommitHistoryPickerSelect)
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.queue.push(crate::queue::InternalEvent::CommitHistoryPickerCancel)
            }
            _ => {}
        }
        true
    }
}
