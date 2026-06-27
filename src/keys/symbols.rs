pub struct KeySymbols {
    pub enter: &'static str,
    pub esc: &'static str,
    pub tab: &'static str,
    pub backtab: &'static str,
    pub up: &'static str,
    pub down: &'static str,
    pub left: &'static str,
    pub right: &'static str,
    pub page_up: &'static str,
    pub page_down: &'static str,
    pub home: &'static str,
    pub end: &'static str,
}

impl Default for KeySymbols {
    fn default() -> Self {
        Self {
            enter: "⏎",
            esc: "⎋",
            tab: "⇥",
            backtab: "⇧⇥",
            up: "↑",
            down: "↓",
            left: "←",
            right: "→",
            page_up: "⇞",
            page_down: "⇟",
            home: "↖",
            end: "↘",
        }
    }
}
