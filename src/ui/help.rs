use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::keymap::HELP_GROUPS;
use super::theme::{ACCENT, ALT, TEXT};

pub fn render_help_overlay(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(70, 80, area);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .title(Span::styled(" Keys ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)));

    let mut lines = vec![];
    for group in HELP_GROUPS {
        lines.push(Line::from(Span::styled(
            group.title,
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )));
        for hint in group.hints {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<16} ", hint.keys), Style::default().fg(ALT)),
                Span::styled(hint.label, Style::default().fg(TEXT)),
            ]));
        }
        lines.push(Line::from(""));
    }

    let para = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(para, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
            ratatui::layout::Constraint::Percentage(percent_y),
            ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
            ratatui::layout::Constraint::Percentage(percent_x),
            ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])
        [1]
}
