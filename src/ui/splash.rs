use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::theme::{ACCENT, ALT, BRIGHT, DIM, GOOD, TEXT, WARN};

pub fn render_splash(frame: &mut Frame, area: Rect, update_version: Option<&str>, recovered: bool) {
    let logo_lines = vec![
        Line::from(Span::styled(
            "  ╔╦╗╦╔╦╗╦═╗╔╦╗╔═╗",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "   ║ ║║║║╠╦╝ ║ ║╣ ",
            Style::default().fg(BRIGHT),
        )),
        Line::from(Span::styled(
            "   ╩ ╩╩ ╩╩╚═ ╩ ╚═╝",
            Style::default().fg(ALT),
        )),
    ];

    let mut lines = logo_lines;
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Find and download torrents, straight from your terminal.",
        Style::default().fg(TEXT),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Getting started:",
        Style::default().fg(DIM),
    )));
    lines.push(Line::from(vec![
        Span::styled("   1. ", Style::default().fg(GOOD)),
        Span::styled("Press ", Style::default().fg(TEXT)),
        Span::styled("/", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(" and type a movie, show, or game name", Style::default().fg(TEXT)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("   2. ", Style::default().fg(GOOD)),
        Span::styled("Browse results from multiple sources", Style::default().fg(TEXT)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("   3. ", Style::default().fg(GOOD)),
        Span::styled("Press ", Style::default().fg(TEXT)),
        Span::styled("d", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(" on any result to start downloading", Style::default().fg(TEXT)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("   4. ", Style::default().fg(GOOD)),
        Span::styled("Press ", Style::default().fg(TEXT)),
        Span::styled("?", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(" any time for the full key list", Style::default().fg(TEXT)),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  press / to search  ·  ? for help  ·  q to quit",
        Style::default().fg(ALT),
    )));

    if let Some(v) = update_version {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  update available: {}", v),
            Style::default().fg(WARN),
        )));
    }

    if recovered {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  recovered from a crashed start · downloads paused",
            Style::default().fg(WARN),
        )));
    }

    let para = Paragraph::new(lines).alignment(Alignment::Left);
    frame.render_widget(para, area);
}

pub fn render_logo(frame: &mut Frame, area: Rect) {
    let line = Line::from(Span::styled(
        "torlnk",
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
    ));
    let para = Paragraph::new(line);
    frame.render_widget(para, area);
}
