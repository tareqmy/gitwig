//! Tokenizer rules and style mapping for code syntax highlighting in previews/diffs.

use ratatui::text::Line;

pub fn highlight_code_line(content: &str) -> Line<'static> {
    // Basic placeholder for syntax highlighting.
    // In the future, this can be integrated with syntect or custom keyword rules.
    Line::from(content.to_string())
}
