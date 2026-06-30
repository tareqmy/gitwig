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
pub fn draw_branch_delete_popup(f: &mut Frame, target: &Option<(String, bool)>, area: Rect) {
    let popup_area = centered_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(DANGER());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Delete Branch", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let (branch_name, is_remote) = match target {
        Some((name, remote)) => (name.as_str(), *remote),
        None => ("", false),
    };

    let type_label = if is_remote { "remote branch (from the remote server)" } else { "branch" };
    let content = vec![
        Line::from(vec![
            Span::styled("Are you sure you want to delete the ", primary_style()),
            Span::styled(type_label, accent_style()),
            Span::raw(":"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(branch_name, Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Confirm: ", muted_style()),
            Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" / Cancel: ", muted_style()),
            Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(content).block(block);
    f.render_widget(paragraph, popup_area);
}

pub fn draw_branch_checkout_popup(f: &mut Frame, target: &Option<(String, bool)>, area: Rect) {
    let popup_area = centered_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Checkout Branch", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let (branch_name, is_remote) = match target {
        Some((name, remote)) => (name.as_str(), *remote),
        None => ("", false),
    };

    let type_label = if is_remote { "remote-tracking branch" } else { "branch" };
    let content = vec![
        Line::from(vec![
            Span::styled("Are you sure you want to checkout the ", primary_style()),
            Span::styled(type_label, accent_style()),
            Span::raw(":"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(branch_name, Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Confirm: ", muted_style()),
            Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" / Cancel: ", muted_style()),
            Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(content).block(block);
    f.render_widget(paragraph, popup_area);
}

pub fn draw_tag_checkout_popup(f: &mut Frame, target: &Option<String>, area: Rect) {
    let popup_area = centered_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Checkout Tag", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let tag_name = target.as_deref().unwrap_or("");

    let content = vec![
        Line::from(vec![Span::styled(
            "Are you sure you want to checkout the tag (detached HEAD):",
            primary_style(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(tag_name, Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Confirm: ", muted_style()),
            Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" / Cancel: ", muted_style()),
            Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(content).block(block);
    f.render_widget(paragraph, popup_area);
}

pub fn draw_discard_changes_popup(f: &mut Frame, target: &Option<(String, bool)>, area: Rect) {
    let popup_area = centered_rect(60, 20, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(DANGER());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Discard Changes", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let (file_path, staged) = match target {
        Some((path, staged)) => (path.as_str(), *staged),
        None => ("", false),
    };

    let area_label = if staged { "staged" } else { "unstaged" };
    let content = vec![
        Line::from(vec![
            Span::styled("Are you sure you want to discard ", primary_style()),
            Span::styled(area_label, accent_style()),
            Span::styled(" changes in:", primary_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(file_path, Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "This operation is destructive and cannot be undone.",
            Style::default().fg(DANGER()),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Confirm: ", muted_style()),
            Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" / Cancel: ", muted_style()),
            Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(content).block(block);
    f.render_widget(paragraph, popup_area);
}

pub fn draw_branch_merge_popup(
    f: &mut Frame,
    target: &Option<(String, bool)>,
    current_branch: Option<&str>,
    area: Rect,
) {
    let popup_area = centered_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Merge Branch", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let (branch_name, is_remote) = match target {
        Some((name, remote)) => (name.as_str(), *remote),
        None => ("", false),
    };

    let type_label = if is_remote { "remote-tracking branch" } else { "branch" };

    let current = current_branch.unwrap_or("HEAD");

    let content = vec![
        Line::from(vec![
            Span::styled("Are you sure you want to merge the ", primary_style()),
            Span::styled(type_label, accent_style()),
            Span::raw(":"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(branch_name, Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("into the current branch ", primary_style()),
            Span::styled(format!("'{}'", current), accent_style().add_modifier(Modifier::BOLD)),
            Span::raw("?"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Confirm: ", muted_style()),
            Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" / Cancel: ", muted_style()),
            Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(content).block(block);
    f.render_widget(paragraph, popup_area);
}

pub fn draw_merge_abort_confirm_popup(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(45, 12, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(DANGER());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Abort Merge", Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(title)
        .padding(Padding::uniform(1));

    let content = vec![
        Line::from(Span::styled("Are you sure you want to abort the merge?", primary_style())),
        Line::from(""),
        Line::from(Span::styled("All unresolved conflict changes will be lost.", muted_style())),
        Line::from(""),
        Line::from(vec![
            Span::styled("Confirm: ", muted_style()),
            Span::styled("y", Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
            Span::styled(" / Cancel: ", muted_style()),
            Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(content).block(block);
    f.render_widget(paragraph, popup_area);
}

pub fn draw_merge_continue_confirm_popup(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(45, 12, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(SUCCESS());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Continue Merge", Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(title)
        .padding(Padding::uniform(1));

    let content = vec![
        Line::from(Span::styled("Are you sure you want to commit the merge?", primary_style())),
        Line::from(""),
        Line::from(Span::styled("This will finalize the merge commit.", muted_style())),
        Line::from(""),
        Line::from(vec![
            Span::styled("Confirm: ", muted_style()),
            Span::styled("y", Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)),
            Span::styled(" / Cancel: ", muted_style()),
            Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(content).block(block);
    f.render_widget(paragraph, popup_area);
}

pub fn draw_branch_rebase_popup(
    f: &mut Frame,
    target: &Option<(String, bool)>,
    current_branch: Option<&str>,
    area: Rect,
) {
    let popup_area = centered_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Rebase Branch", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let (branch_name, is_remote) = match target {
        Some((name, remote)) => (name.as_str(), *remote),
        None => ("", false),
    };

    let type_label = if is_remote { "remote-tracking branch" } else { "branch" };

    let current = current_branch.unwrap_or("HEAD");

    let content = vec![
        Line::from(vec![
            Span::styled("Are you sure you want to rebase the ", primary_style()),
            Span::styled(
                format!("current branch '{}'", current),
                accent_style().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("onto the ", primary_style()),
            Span::styled(type_label, accent_style()),
            Span::raw(":"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(branch_name, Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Confirm: ", muted_style()),
            Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" / Cancel: ", muted_style()),
            Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(content).block(block);
    f.render_widget(paragraph, popup_area);
}

pub fn draw_branch_interactive_rebase_popup(
    f: &mut Frame,
    target: &Option<(String, bool)>,
    current_branch: Option<&str>,
    area: Rect,
) {
    let popup_area = centered_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Interactive Rebase Branch", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let (branch_name, is_remote) = match target {
        Some((name, remote)) => (name.as_str(), *remote),
        None => ("", false),
    };

    let type_label = if is_remote { "remote-tracking branch" } else { "branch" };

    let current = current_branch.unwrap_or("HEAD");

    let content = vec![
        Line::from(vec![
            Span::styled("Are you sure you want to interactively rebase the ", primary_style()),
            Span::styled(
                format!("current branch '{}'", current),
                accent_style().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("onto the ", primary_style()),
            Span::styled(type_label, accent_style()),
            Span::raw(":"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(branch_name, Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Confirm: ", muted_style()),
            Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" / Cancel: ", muted_style()),
            Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(content).block(block);
    f.render_widget(paragraph, popup_area);
}

pub fn draw_branch_push_popup(f: &mut Frame, target: &Option<(String, bool)>, area: Rect) {
    let popup_area = centered_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Push Branch", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let branch_name = match target {
        Some((name, _)) => name.as_str(),
        None => "",
    };

    let content = vec![
        Line::from(vec![
            Span::styled("Are you sure you want to push branch ", primary_style()),
            Span::styled(branch_name, accent_style().add_modifier(Modifier::BOLD)),
            Span::raw("?"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Confirm: ", muted_style()),
            Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" / Cancel: ", muted_style()),
            Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(content).block(block);
    f.render_widget(paragraph, popup_area);
}

pub fn draw_tag_delete_popup(f: &mut Frame, target: &Option<(String, bool)>, area: Rect) {
    let popup_area = centered_rect(55, 25, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(DANGER());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Delete Tag", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let (tag_name, is_on_remote) = match target {
        Some((name, is_on_remote)) => (name.as_str(), *is_on_remote),
        None => ("", false),
    };

    let mut content = vec![
        Line::from(vec![
            Span::styled("Are you sure you want to delete the tag ", primary_style()),
            Span::styled(tag_name, accent_style()),
            Span::raw("?"),
        ]),
        Line::from(""),
    ];

    if is_on_remote {
        content.push(Line::from(vec![
            Span::styled("Warning: ", Style::default().fg(WARNING()).add_modifier(Modifier::BOLD)),
            Span::raw(
                "This tag is also present on the remote and will be deleted from the remote.",
            ),
        ]));
        content.push(Line::from(""));
    }

    content.push(Line::from(vec![
        Span::styled("Confirm: ", muted_style()),
        Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" / Cancel: ", muted_style()),
        Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
    ]));

    let paragraph = Paragraph::new(content).block(block).wrap(Wrap { trim: false });
    f.render_widget(paragraph, popup_area);
}

pub fn draw_stash_delete_popup(f: &mut Frame, target: &Option<String>, area: Rect) {
    let popup_area = centered_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(DANGER());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Delete Stash", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let stash_name = match target {
        Some(name) => name.as_str(),
        None => "",
    };

    let content = vec![
        Line::from(vec![Span::styled(
            "Are you sure you want to delete the stash:",
            primary_style(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(stash_name, Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Confirm: ", muted_style()),
            Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" / Cancel: ", muted_style()),
            Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(content).block(block).wrap(Wrap { trim: false });
    f.render_widget(paragraph, popup_area);
}

pub fn draw_stash_apply_popup(
    f: &mut Frame,
    target: &Option<String>,
    delete_after: bool,
    area: Rect,
) {
    let popup_area = centered_rect(55, 25, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Apply Stash", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let stash_name = match target {
        Some(name) => name.as_str(),
        None => "",
    };

    let mut content = vec![
        Line::from(vec![Span::styled(
            "Are you sure you want to apply the stash:",
            primary_style(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(stash_name, Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
    ];

    let delete_after_style = if delete_after {
        Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)
    } else {
        muted_style()
    };

    let checkbox = if delete_after { "[X]" } else { "[ ]" };

    content.push(Line::from(vec![
        Span::styled(format!("  {} ", checkbox), delete_after_style),
        Span::styled("Delete stash after applying", primary_style()),
        Span::styled(" (toggle: [d/space/a])", muted_style()),
    ]));

    content.push(Line::from(""));

    content.push(Line::from(vec![
        Span::styled("Confirm: ", muted_style()),
        Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" / Cancel: ", muted_style()),
        Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
    ]));

    let paragraph = Paragraph::new(content).block(block).wrap(Wrap { trim: false });
    f.render_widget(paragraph, popup_area);
}

pub fn draw_cherry_pick_popup(
    f: &mut Frame,
    target: &Option<(String, String)>,
    current_branch: Option<&str>,
    app: &crate::app::App,
    area: Rect,
) {
    let popup_area = centered_rect(60, 75, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Cherry-pick Commit", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(2));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // Split inner: top contains details (2 rows), bottom contains list/dropdown.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Details (2 lines + spacer)
            Constraint::Min(1),    // Local branches dropdown/list
            Constraint::Length(1), // Help / shortcuts hint
        ])
        .split(inner);

    let (commit_oid, summary) = match target {
        Some((oid, sum)) => (oid.as_str(), sum.as_str()),
        None => ("", ""),
    };
    let source_branch = current_branch.unwrap_or("HEAD");

    let details_content = vec![
        Line::from(vec![
            Span::styled("Commit: ", muted_style()),
            Span::styled(format!("{:.7}", commit_oid), accent_style().add_modifier(Modifier::BOLD)),
            Span::raw(" ("),
            Span::styled(summary, primary_style()),
            Span::raw(")"),
        ]),
        Line::from(vec![
            Span::styled("Taken From: ", muted_style()),
            Span::styled(source_branch, primary_style().add_modifier(Modifier::BOLD)),
        ]),
    ];
    f.render_widget(Paragraph::new(details_content), chunks[0]);

    // Destination branches dropdown (List)
    let items: Vec<ListItem> = if app.cherry_pick_dest_branches.is_empty() {
        vec![ListItem::new(Line::from(vec![Span::styled(
            "  (No local branches found)",
            muted_style(),
        )]))]
    } else {
        app.cherry_pick_dest_branches
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let is_selected = i == app.cherry_pick_dest_selection;
                let style = if is_selected {
                    accent_style().add_modifier(Modifier::BOLD | Modifier::REVERSED)
                } else {
                    primary_style()
                };
                let prefix = if is_selected { "▸ " } else { "  " };
                ListItem::new(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(b.clone(), style),
                ]))
            })
            .collect()
    };

    let mut list_state = ListState::default();
    list_state.select(Some(app.cherry_pick_dest_selection));

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(muted_style())
        .title("Select Destination Branch");

    let list_area = chunks[1];
    f.render_stateful_widget(List::new(items).block(list_block), list_area, &mut list_state);

    let hint = Line::from(vec![
        Span::styled("↑↓ / j k navigate  ", muted_style()),
        Span::styled("Enter", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" confirm  ", muted_style()),
        Span::styled("Esc / q", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" cancel", muted_style()),
    ]);
    f.render_widget(Paragraph::new(hint), chunks[2]);
}

pub fn draw_revert_popup(
    f: &mut Frame,
    target: &Option<(String, String)>,
    current_branch: Option<&str>,
    area: Rect,
) {
    let popup_area = centered_rect(55, 25, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Revert Commit", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let (commit_oid, summary) = match target {
        Some((oid, sum)) => (oid.as_str(), sum.as_str()),
        None => ("", ""),
    };

    let current = current_branch.unwrap_or("HEAD");

    let content = vec![
        Line::from(vec![
            Span::styled("Are you sure you want to revert commit ", primary_style()),
            Span::styled(format!("{:.7}", commit_oid), accent_style().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("on branch ", primary_style()),
            Span::styled(format!("'{}'", current), accent_style().add_modifier(Modifier::BOLD)),
            Span::raw("?"),
        ]),
        Line::from(""),
        Line::from(vec![Span::raw("  "), Span::styled(summary, primary_style())]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Confirm: ", muted_style()),
            Span::styled("y", Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)),
            Span::styled(" / Cancel: ", muted_style()),
            Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(content).block(block).wrap(Wrap { trim: false });
    f.render_widget(paragraph, popup_area);
}

pub fn draw_tag_push_popup(f: &mut Frame, target: &Option<String>, area: Rect) {
    let popup_area = centered_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(SUCCESS());
    let title =
        Line::from(vec![Span::raw(" "), Span::styled("Push Tag", primary_style()), Span::raw(" ")]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let tag_name = target.as_deref().unwrap_or("");

    let content = vec![
        Line::from(vec![
            Span::styled("Are you sure you want to push the tag ", primary_style()),
            Span::styled(tag_name, accent_style()),
            Span::raw(" to remote?"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Confirm: ", muted_style()),
            Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" / Cancel: ", muted_style()),
            Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(content).block(block).wrap(Wrap { trim: false });
    f.render_widget(paragraph, popup_area);
}

pub fn draw_tag_push_all_popup(f: &mut Frame, remote: Option<&str>, area: Rect) {
    let popup_area = centered_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(SUCCESS());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Push All Tags", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let remote_str = remote.unwrap_or("remote");

    let content = vec![
        Line::from(vec![
            Span::styled("Are you sure you want to push ", primary_style()),
            Span::styled(
                "ALL local tags",
                Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![Span::styled(format!("to remote '{}'?", remote_str), primary_style())]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Confirm: ", muted_style()),
            Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" / Cancel: ", muted_style()),
            Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(content).block(block).wrap(Wrap { trim: false });
    f.render_widget(paragraph, popup_area);
}

pub fn draw_remote_delete_popup(f: &mut Frame, remote_name: &str, area: Rect) {
    let popup_area = centered_rect(50, 15, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(DANGER());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Remove Remote", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let content = vec![
        Line::from(vec![
            Span::raw("Are you sure you want to remove remote "),
            Span::styled(remote_name, Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
            Span::raw("?"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("All remote-tracking branches for ", muted_style()),
            Span::styled(remote_name, primary_style()),
            Span::styled(" will be deleted.", muted_style()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("Confirm: [y]  Cancel: [n/Esc]", muted_style())]),
    ];

    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let paragraph = Paragraph::new(content);
    f.render_widget(paragraph, inner_area);
}

use crate::components::{Component, DrawableComponent, EventState};
use crate::queue::{InternalEvent, Queue};
use crossterm::event::{Event, KeyCode, KeyEvent};

pub struct ConfirmPopup {
    pub queue: Queue,
}

impl ConfirmPopup {
    pub fn new(queue: Queue) -> Self {
        Self { queue }
    }

    pub fn handle_event(app: &mut crate::app::App, key: KeyEvent) -> bool {
        let code = key.code;
        match app.mode {
            crate::app::Mode::CherryPickConfirm => match code {
                KeyCode::Up | KeyCode::Char('k') => {
                    app.cherry_pick_dest_selection =
                        app.cherry_pick_dest_selection.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if !app.cherry_pick_dest_branches.is_empty() {
                        app.cherry_pick_dest_selection = (app.cherry_pick_dest_selection + 1)
                            .min(app.cherry_pick_dest_branches.len().saturating_sub(1));
                    }
                }
                KeyCode::Enter => app.confirm_cherry_pick(),
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => app.cancel_cherry_pick(),
                _ => {}
            },
            crate::app::Mode::StashApplyConfirm => match code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    app.confirm_stash_apply()
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_stash_apply(),
                KeyCode::Char('D') | KeyCode::Char(' ') | KeyCode::Char('A') => {
                    app.toggle_stash_apply_delete();
                }
                _ => {}
            },
            _ => {}
        }
        true
    }
}

impl DrawableComponent for ConfirmPopup {
    fn draw(&self, _f: &mut ratatui::Frame, _rect: ratatui::layout::Rect) -> std::io::Result<()> {
        Ok(())
    }
}

impl Component for ConfirmPopup {
    fn event(&mut self, ev: &Event) -> std::io::Result<EventState> {
        if let Event::Key(key) = ev {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    self.queue.push(InternalEvent::ConfirmYes);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.queue.push(InternalEvent::ConfirmNo);
                    return Ok(EventState::Consumed);
                }
                _ => {}
            }
        }
        Ok(EventState::NotConsumed)
    }
}

pub fn draw_update_confirm_popup(f: &mut Frame, latest_version: &str, area: Rect) {
    let popup_area = centered_rect(50, 18, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(SUCCESS());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled(
            "Update Available",
            Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let content = vec![
        Line::from(vec![Span::styled("A new version of Gitwig is available:", primary_style())]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Current Version: ", muted_style()),
            Span::styled(env!("CARGO_PKG_VERSION"), primary_style().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Latest Version:  ", muted_style()),
            Span::styled(
                latest_version,
                Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Would you like to trigger the update now?",
            primary_style(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Confirm: ", muted_style()),
            Span::styled("y", Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)),
            Span::styled(" / Cancel: ", muted_style()),
            Span::styled("n", accent_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(content).block(block).wrap(Wrap { trim: false });
    f.render_widget(paragraph, popup_area);
}
