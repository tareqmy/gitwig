use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone)]
pub struct KeyList {
    pub quit: KeyEvent,
    pub exit_popup: KeyEvent,
    pub enter: KeyEvent,
    pub move_up: KeyEvent,
    pub move_down: KeyEvent,
    pub move_left: KeyEvent,
    pub move_right: KeyEvent,
    pub page_up: KeyEvent,
    pub page_down: KeyEvent,
    pub home: KeyEvent,
    pub end: KeyEvent,
    pub tab_next: KeyEvent,
    pub tab_prev: KeyEvent,
    // Add all other keys here
}

impl Default for KeyList {
    fn default() -> Self {
        Self {
            quit: KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty()),
            exit_popup: KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
            enter: KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
            move_up: KeyEvent::new(KeyCode::Up, KeyModifiers::empty()),
            move_down: KeyEvent::new(KeyCode::Down, KeyModifiers::empty()),
            move_left: KeyEvent::new(KeyCode::Left, KeyModifiers::empty()),
            move_right: KeyEvent::new(KeyCode::Right, KeyModifiers::empty()),
            page_up: KeyEvent::new(KeyCode::PageUp, KeyModifiers::empty()),
            page_down: KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty()),
            home: KeyEvent::new(KeyCode::Home, KeyModifiers::empty()),
            end: KeyEvent::new(KeyCode::End, KeyModifiers::empty()),
            tab_next: KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()),
            tab_prev: KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_list_default() {
        let _ = KeyList::default();
    }
}
