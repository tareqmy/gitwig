//! Picker popup for selecting target remote for sync/fetch operations.

use crate::app::{App, Mode};
use crate::repo::RemoteInfo;
use crate::ui::layout::{centered_rect, centered_rect_fixed};
use crate::ui::style::{
    ACCENT, CARD_BORDER, DANGER, SUCCESS, WARNING, accent_style, muted_style, parse_color,
    primary_style,
};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, Gauge, List, ListItem, ListState, Padding, Paragraph, Wrap,
};

use crate::ui::*;
pub fn draw_remote_picker_popup(
    f: &mut Frame,
    remotes: &[RemoteInfo],
    selection: usize,
    area: Rect,
) {
    let popup_area = centered_rect(50, 60, area);

    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Select Remote", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // Split inner: list on top, hint at bottom.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let items: Vec<ListItem> = remotes
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let style = if i == selection {
                accent_style().add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                primary_style()
            };
            ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(r.name.clone(), style),
                Span::styled("  ", muted_style()),
                Span::styled(r.url.clone(), muted_style()),
            ]))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(selection));
    f.render_stateful_widget(List::new(items), chunks[0], &mut list_state);

    let hint = Line::from(vec![
        Span::styled("↑↓ navigate  ", muted_style()),
        Span::styled("Enter", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" confirm  ", muted_style()),
        Span::styled("Esc", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" cancel", muted_style()),
    ]);
    f.render_widget(Paragraph::new(hint), chunks[1]);
}

use crossterm::event::{KeyCode, KeyEvent};
pub struct RemotePickerPopup;
impl RemotePickerPopup {
    pub fn handle_event(app: &mut crate::app::App, key: KeyEvent) -> bool {
        let code = key.code;
        match code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => app.remote_picker_up(),
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => app.remote_picker_down(),
            KeyCode::Enter => app.confirm_remote_picker(),
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => app.cancel_remote_picker(),
            _ => {}
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn test_remote_picker_popup_events() {
        let config = crate::config::Config::default();
        let mut app = crate::app::App::new(config, std::path::PathBuf::from("test.toml"));

        // Setup mock remotes list in current_detail
        app.current_detail = Some(crate::repo::ItemDetail::Repo {
            resolved: std::path::PathBuf::from("test"),
            info: Box::new(crate::repo::RepoInfo {
                remotes: crate::repo::TabData::Loaded(vec![
                    crate::repo::RemoteInfo {
                        name: "origin".to_string(),
                        url: "url1".to_string(),
                        push_url: None,
                        refspecs: vec![],
                    },
                    crate::repo::RemoteInfo {
                        name: "upstream".to_string(),
                        url: "url2".to_string(),
                        push_url: None,
                        refspecs: vec![],
                    },
                ]),
                ..Default::default()
            }),
        });
        app.remote_picker_selection = 0;

        let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

        // Test Up/Down key handling
        RemotePickerPopup::handle_event(&mut app, key_event(KeyCode::Down));
        assert_eq!(app.remote_picker_selection, 1);

        RemotePickerPopup::handle_event(&mut app, key_event(KeyCode::Up));
        assert_eq!(app.remote_picker_selection, 0);

        // Test Enter confirm
        RemotePickerPopup::handle_event(&mut app, key_event(KeyCode::Enter));

        // Test Esc cancel
        RemotePickerPopup::handle_event(&mut app, key_event(KeyCode::Esc));
    }
}
