//! PR code review comments input modal wizard.

use crate::app::{App, Mode};
use crate::ui::layout::centered_rect;
use crate::ui::style::{ACCENT, CARD_BORDER, muted_style, primary_style};
use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap};

pub fn draw_forge_comment_popup(f: &mut Frame, app: &App, input_buffer: &str, area: Rect) {
    let popup_area = centered_rect(60, 25, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Add PR Line Comment", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let mut content = Vec::new();
    match app.mode {
        Mode::ForgeCommentPathInput => {
            content.push(Line::from(vec![Span::styled(
                "Step 1: Enter File Path (relative to repo root)",
                primary_style(),
            )]));
            content.push(Line::from(""));
            content.push(Line::from(vec![
                Span::styled("File Path: ", muted_style()),
                Span::styled(input_buffer, Style::default()),
            ]));
        }
        Mode::ForgeCommentLineInput => {
            content.push(Line::from(vec![
                Span::styled("File: ", muted_style()),
                Span::styled(&app.forge_comment_path, primary_style()),
            ]));
            content.push(Line::from(""));
            content
                .push(Line::from(vec![Span::styled("Step 2: Enter Line Number", primary_style())]));
            content.push(Line::from(""));
            content.push(Line::from(vec![
                Span::styled("Line Number: ", muted_style()),
                Span::styled(input_buffer, Style::default()),
            ]));
        }
        Mode::ForgeCommentBodyInput => {
            content.push(Line::from(vec![
                Span::styled("File: ", muted_style()),
                Span::styled(&app.forge_comment_path, primary_style()),
            ]));
            content.push(Line::from(vec![
                Span::styled("Line: ", muted_style()),
                Span::styled(app.forge_comment_line.to_string(), primary_style()),
            ]));
            content.push(Line::from(""));
            content.push(Line::from(vec![Span::styled(
                "Step 3: Enter Comment Body",
                primary_style(),
            )]));
            content.push(Line::from(""));
            content.push(Line::from(vec![
                Span::styled("Comment: ", muted_style()),
                Span::styled(input_buffer, Style::default()),
            ]));
        }
        _ => {}
    }

    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let paragraph = Paragraph::new(content).wrap(Wrap { trim: false });
    f.render_widget(paragraph, inner_area);

    // Set cursor
    let label_width = match app.mode {
        Mode::ForgeCommentPathInput => "File Path: ".chars().count(),
        Mode::ForgeCommentLineInput => "Line Number: ".chars().count(),
        Mode::ForgeCommentBodyInput => "Comment: ".chars().count(),
        _ => 0,
    } as u16;

    let row_offset = match app.mode {
        Mode::ForgeCommentPathInput => 2,
        Mode::ForgeCommentLineInput => 4,
        Mode::ForgeCommentBodyInput => 5,
        _ => 0,
    };

    let cursor_y = inner_area
        .y
        .saturating_add(row_offset)
        .min(inner_area.y.saturating_add(inner_area.height.saturating_sub(1)));
    let cursor_offset = label_width.saturating_add(input_buffer.chars().count() as u16);
    let cursor_x = inner_area
        .x
        .saturating_add(cursor_offset)
        .min(inner_area.x.saturating_add(inner_area.width.saturating_sub(1)));
    f.set_cursor_position(Position::new(cursor_x, cursor_y));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_forge_comment_popup() {
        let config = crate::config::Config::default();
        let mut app = App::new(config, std::path::PathBuf::from("test.toml"));

        app.mode = Mode::ForgeCommentPathInput;
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                draw_forge_comment_popup(f, &app, "some/path.txt", Rect::new(0, 0, 80, 24));
            })
            .unwrap();

        app.mode = Mode::ForgeCommentLineInput;
        app.forge_comment_path = "some/path.txt".to_string();
        terminal
            .draw(|f| {
                draw_forge_comment_popup(f, &app, "42", Rect::new(0, 0, 80, 24));
            })
            .unwrap();

        app.mode = Mode::ForgeCommentBodyInput;
        app.forge_comment_line = 42;
        terminal
            .draw(|f| {
                draw_forge_comment_popup(f, &app, "looks good", Rect::new(0, 0, 80, 24));
            })
            .unwrap();
    }
}
