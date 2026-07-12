//! Remote repository clone and import paths config wizard.

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

pub fn draw_import_popup(f: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect_fixed(65, 12, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(accent_style())
        .title(
            Line::from(vec![
                Span::raw(" "),
                Span::styled("Import Remote Repository", primary_style()),
                Span::raw(" "),
            ])
            .alignment(Alignment::Center),
        );

    f.render_widget(block.clone(), popup_area);
    let inner = block.inner(popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Length(2), // URL field
            Constraint::Length(2), // Destination Path field
            Constraint::Length(2), // Optional Name field
            Constraint::Min(1),    // Help hint
        ])
        .split(inner);

    // Render URL
    let url_style = if matches!(app.mode, Mode::ImportUrlInput) {
        Style::default().fg(ACCENT())
    } else {
        Style::default()
    };
    let url_val =
        if matches!(app.mode, Mode::ImportUrlInput) { &app.input_buffer } else { &app.import_url };
    let url_para = Paragraph::new(Line::from(vec![
        Span::styled("Source URL: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(url_val, url_style),
    ]));
    f.render_widget(url_para, chunks[0]);

    // Render Dest
    let dest_style = if matches!(app.mode, Mode::ImportDestInput) {
        Style::default().fg(ACCENT())
    } else {
        Style::default()
    };
    let dest_val = if matches!(app.mode, Mode::ImportDestInput) {
        &app.input_buffer
    } else {
        &app.import_dest
    };
    let dest_para = Paragraph::new(Line::from(vec![
        Span::styled("Dest Path:  ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(dest_val, dest_style),
    ]));
    f.render_widget(dest_para, chunks[1]);

    // Render Name
    let name_style = if matches!(app.mode, Mode::ImportNameInput) {
        Style::default().fg(ACCENT())
    } else {
        Style::default()
    };
    let name_val = if matches!(app.mode, Mode::ImportNameInput) {
        &app.input_buffer
    } else {
        &app.import_name
    };
    let name_para = Paragraph::new(Line::from(vec![
        Span::styled("Repo Name:  ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(name_val, name_style),
    ]));
    f.render_widget(name_para, chunks[2]);

    // Hint
    let current_step = match app.mode {
        Mode::ImportUrlInput => "Step 1: Enter Remote Git URL (Press Enter)",
        Mode::ImportDestInput => "Step 2: Enter Local Destination Path (Press Enter)",
        Mode::ImportNameInput => "Step 3: Enter Optional Folder Name (Press Enter to Clone)",
        _ => "",
    };
    let hint_para = Paragraph::new(Line::from(vec![
        Span::styled(current_step, muted_style()),
        Span::raw(" | "),
        Span::styled("Esc", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" to go back/cancel", muted_style()),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(hint_para, chunks[3]);

    let cursor_y = match app.mode {
        Mode::ImportUrlInput => chunks[0].y,
        Mode::ImportDestInput => chunks[1].y,
        Mode::ImportNameInput => chunks[2].y,
        _ => 0,
    };
    if cursor_y > 0 {
        let prefix_len = 12;
        let cursor_offset = (prefix_len + app.input_buffer.chars().count()) as u16;
        let cursor_x = chunks[0].x.saturating_add(cursor_offset.min(inner.width.saturating_sub(1)));
        f.set_cursor_position(Position::new(cursor_x, cursor_y));
    }
}

use crossterm::event::{KeyCode, KeyEvent};
pub struct ImportPopup;
impl ImportPopup {
    pub fn handle_event(app: &mut crate::app::App, key: KeyEvent) -> bool {
        let code = key.code;
        match app.mode {
            Mode::ImportUrlInput => match code {
                KeyCode::Esc => {
                    crate::debug_log::info("Cancelled repository import");
                    app.mode = Mode::Normal;
                    app.input_buffer.clear();
                }
                KeyCode::Enter => {
                    app.import_url = app.input_buffer.clone();
                    app.input_buffer.clear();

                    let repo_name = if let Some(last) = app.import_url.split('/').next_back() {
                        let name = last.trim_end_matches(".git");
                        if name.is_empty() { "repo".to_string() } else { name.to_string() }
                    } else {
                        "repo".to_string()
                    };

                    if let Some(home) = dirs::home_dir() {
                        app.input_buffer = home.join(&repo_name).to_string_lossy().to_string();
                    } else {
                        app.input_buffer = format!("./{}", repo_name);
                    }

                    app.mode = Mode::ImportDestInput;
                }
                KeyCode::Backspace => app.input_backspace(),
                KeyCode::Char(c) => app.input_char(c),
                _ => {}
            },
            Mode::ImportDestInput => match code {
                KeyCode::Esc => {
                    app.mode = Mode::ImportUrlInput;
                    app.input_buffer = app.import_url.clone();
                }
                KeyCode::Enter => {
                    app.import_dest = app.input_buffer.clone();
                    app.input_buffer.clear();

                    let repo_name = if let Some(last) = app.import_url.split('/').next_back() {
                        let name = last.trim_end_matches(".git");
                        if name.is_empty() { "repo".to_string() } else { name.to_string() }
                    } else {
                        "repo".to_string()
                    };
                    app.input_buffer = repo_name;
                    app.mode = Mode::ImportNameInput;
                }
                KeyCode::Backspace => app.input_backspace(),
                KeyCode::Char(c) => app.input_char(c),
                _ => {}
            },
            Mode::ImportNameInput => match code {
                KeyCode::Esc => {
                    app.mode = Mode::ImportDestInput;
                    app.input_buffer = app.import_dest.clone();
                }
                KeyCode::Enter => {
                    app.import_name = app.input_buffer.clone();
                    app.input_buffer.clear();
                    app.start_import_clone();
                }
                KeyCode::Backspace => app.input_backspace(),
                KeyCode::Char(c) => app.input_char(c),
                _ => {}
            },
            _ => {}
        }
        true
    }
}
