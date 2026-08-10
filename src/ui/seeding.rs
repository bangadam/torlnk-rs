use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::download::types::{SeedItem, SeedStatus};
use super::theme::{ALT, BAD, GOOD, ICON_UP, TEXT};
use crate::util::format::{clean_text, format_bytes, format_bytes_per_sec, truncate};

pub fn render_seeding(
    frame: &mut Frame,
    area: Rect,
    seeds: &[SeedItem],
    list_state: &mut ListState,
) {
    if seeds.is_empty() {
        let msg = Paragraph::new("  Nothing seeding yet.\n  Completed downloads appear here and seed automatically.")
            .style(Style::default().fg(ALT))
            .wrap(Wrap { trim: true });
        frame.render_widget(msg, area);
        return;
    }

    let items: Vec<ListItem> = seeds
        .iter()
        .map(|s| {
            let name = clean_text(&s.name);
            let status_icon = match s.status {
                SeedStatus::Seeding => ICON_UP,
                SeedStatus::Paused => "⏸",
                SeedStatus::Missing => "⚠",
            };
            let status_color = match s.status {
                SeedStatus::Seeding => GOOD,
                SeedStatus::Paused => ALT,
                SeedStatus::Missing => BAD,
            };

            let speed = format_bytes_per_sec(s.upload_speed);
            let uploaded = format_bytes(s.uploaded);
            let peers = s.peers;
            let size = format_bytes(s.size_bytes);

            let name_line = Line::from(vec![
                Span::styled(format!("{} ", status_icon), Style::default().fg(status_color)),
                Span::styled(truncate(&name, 50), Style::default().fg(TEXT)),
            ]);

            let stats_line = Line::from(vec![
                Span::styled(format!("{:>10} ", size), Style::default().fg(ALT)),
                Span::styled(format!("{}{} ", ICON_UP, speed), Style::default().fg(GOOD)),
                Span::styled(format!("up {} ", uploaded), Style::default().fg(ALT)),
                Span::styled(format!("{}{}", "•", peers), Style::default().fg(TEXT)),
            ]);

            ListItem::new(vec![name_line, stats_line])
        })
        .collect();

    let list = List::new(items)
        .highlight_symbol("❯ ")
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_stateful_widget(list, area, list_state);
}
