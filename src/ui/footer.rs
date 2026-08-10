use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::keymap::{footer_hints, Hint};
use super::state::{NoticeLevel, Region, Section};
use super::theme::{ACCENT, ALT, BAD, DIM, GOOD, TEXT, WARN};

const FOOTER_BG: Color = Color::Rgb(40, 36, 50);

pub fn render_footer(
    frame: &mut Frame,
    area: Rect,
    region: Region,
    section: Section,
) {
    let hints = footer_hints(region, section);
    let mut spans: Vec<Span> = vec![];
    for (i, h) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" │ ", Style::default().fg(DIM)));
        }
        spans.push(Span::styled(
            format!(" {} ", h.keys),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(h.label.to_string(), Style::default().fg(TEXT)));
    }

    let line = Line::from(spans);
    let para = Paragraph::new(line).style(Style::default().bg(FOOTER_BG));
    frame.render_widget(para, area);
}

/// Render a transient notice message replacing the footer.
pub fn render_notice(frame: &mut Frame, area: Rect, msg: &str, level: NoticeLevel) {
    let (icon, color, bg) = match level {
        NoticeLevel::Success => ("✓", GOOD, Color::Rgb(30, 50, 38)),
        NoticeLevel::Error => ("✗", BAD, Color::Rgb(55, 30, 36)),
        NoticeLevel::Warn => ("⚠", WARN, Color::Rgb(52, 44, 30)),
        NoticeLevel::Info => ("•", ACCENT, FOOTER_BG),
    };

    let line = Line::from(vec![
        Span::styled(format!(" {} ", icon), Style::default().fg(color).add_modifier(Modifier::BOLD)),
        Span::styled(msg, Style::default().fg(color).add_modifier(Modifier::BOLD)),
        Span::raw(" "),
    ]);
    let para = Paragraph::new(line).style(Style::default().bg(bg));
    frame.render_widget(para, area);
}
