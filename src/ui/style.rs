//! Dynamic theme manager containing styles, color parsing, and border styling constants.

use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::BorderType;

pub struct ThemeState {
    pub accent: Color,
    pub warning: Color,
    pub danger: Color,
    pub success: Color,
    pub border_type: BorderType,
}

pub static THEME: std::sync::RwLock<ThemeState> = std::sync::RwLock::new(ThemeState {
    accent: Color::Cyan,
    warning: Color::Yellow,
    danger: Color::Red,
    success: Color::Green,
    border_type: BorderType::Rounded,
});

#[allow(non_snake_case)]
pub fn ACCENT() -> Color {
    THEME.read().map(|l| l.accent).unwrap_or(Color::Cyan)
}
#[allow(non_snake_case)]
pub fn WARNING() -> Color {
    THEME.read().map(|l| l.warning).unwrap_or(Color::Yellow)
}
#[allow(non_snake_case)]
pub fn DANGER() -> Color {
    THEME.read().map(|l| l.danger).unwrap_or(Color::Red)
}
#[allow(non_snake_case)]
pub fn SUCCESS() -> Color {
    THEME.read().map(|l| l.success).unwrap_or(Color::Green)
}
#[allow(non_snake_case)]
pub fn CARD_BORDER() -> BorderType {
    THEME.read().map(|l| l.border_type).unwrap_or(BorderType::Rounded)
}

pub fn parse_color(s: &str) -> Color {
    match s.to_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" => Color::Gray,
        "darkgray" => Color::DarkGray,
        "lightred" => Color::LightRed,
        "lightgreen" => Color::LightGreen,
        "lightyellow" => Color::LightYellow,
        "lightblue" => Color::LightBlue,
        "lightmagenta" => Color::LightMagenta,
        "lightcyan" => Color::LightCyan,
        "white" => Color::White,
        _ => Color::Cyan,
    }
}

pub fn parse_border_type(s: &str) -> BorderType {
    match s.to_lowercase().as_str() {
        "plain" => BorderType::Plain,
        "rounded" => BorderType::Rounded,
        "double" => BorderType::Double,
        "thick" => BorderType::Thick,
        _ => BorderType::Rounded,
    }
}

pub fn format_color(color: Color) -> String {
    match color {
        Color::Black => "black".to_string(),
        Color::Red => "red".to_string(),
        Color::Green => "green".to_string(),
        Color::Yellow => "yellow".to_string(),
        Color::Blue => "blue".to_string(),
        Color::Magenta => "magenta".to_string(),
        Color::Cyan => "cyan".to_string(),
        Color::Gray => "gray".to_string(),
        Color::DarkGray => "darkgray".to_string(),
        Color::LightRed => "lightred".to_string(),
        Color::LightGreen => "lightgreen".to_string(),
        Color::LightYellow => "lightyellow".to_string(),
        Color::LightBlue => "lightblue".to_string(),
        Color::LightMagenta => "lightmagenta".to_string(),
        Color::LightCyan => "lightcyan".to_string(),
        Color::White => "white".to_string(),
        _ => "cyan".to_string(),
    }
}

pub fn format_border_type(border: BorderType) -> String {
    match border {
        BorderType::Plain => "plain".to_string(),
        BorderType::Rounded => "rounded".to_string(),
        BorderType::Double => "double".to_string(),
        BorderType::Thick => "thick".to_string(),
        _ => "rounded".to_string(),
    }
}

pub fn update_theme(theme: &crate::config::ThemeConfig) {
    if let Ok(mut lock) = THEME.write() {
        lock.accent = parse_color(&theme.accent);
        lock.warning = parse_color(&theme.warning);
        lock.danger = parse_color(&theme.danger);
        lock.success = parse_color(&theme.success);
        lock.border_type = parse_border_type(&theme.border_type);
    }
}

/// "Muted" / secondary text. Uses the terminal's own foreground so it stays
/// readable on both light and dark backgrounds, then applies `DIM` to fade
/// it relative to primary text.
pub fn muted_style() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

/// Emphasized text. Bold over the terminal default — also theme-agnostic.
pub fn primary_style() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}

/// Accent-colored, bold. Used for keys in the status bar / help overlay,
/// and the app title.
pub fn accent_style() -> Style {
    Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)
}
