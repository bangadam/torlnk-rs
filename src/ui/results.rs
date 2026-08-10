use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use super::state::SearchResult;
use super::theme::{source_color, ALT, DIM, GOOD, ICON_DOWN, ICON_PEER, TEXT, WARN};
use crate::util::format::{clean_text, format_bytes, format_count, format_relative, truncate};

pub fn render_results(
    frame: &mut Frame,
    area: Rect,
    results: &[SearchResult],
    list_state: &mut ListState,
    query: &str,
    searching: bool,
    hide_dead: bool,
) {
    let filtered: Vec<&SearchResult> = results
        .iter()
        .filter(|r| !hide_dead || r.result.seeders > 0)
        .collect();

    // Empty / searching states
    if filtered.is_empty() {
        let msg = if searching {
            "  ◌ Searching across sources..."
        } else if !query.is_empty() {
            "  No results found. Try a different search term."
        } else {
            "  Start by pressing / and typing what you want to find."
        };
        let para = Paragraph::new(msg)
            .style(Style::default().fg(ALT))
            .wrap(Wrap { trim: true });
        frame.render_widget(para, area);
        return;
    }

    // Column header
    let header = Line::from(vec![
        Span::styled(" SOURCE   ", Style::default().fg(DIM)),
        Span::styled("NAME", Style::default().fg(DIM)),
        Span::styled(format!("{:>10}", "SIZE"), Style::default().fg(DIM)),
        Span::styled(format!("{:>7}", "SEEDS"), Style::default().fg(DIM)),
        Span::styled(format!("{:>8}", "ADDED"), Style::default().fg(DIM)),
    ]);
    let header_area = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);

    frame.render_widget(
        Paragraph::new(header),
        header_area[0],
    );

    let list_area = header_area[1];

    let items: Vec<ListItem> = filtered
        .iter()
        .map(|r| {
            let name = clean_text(&r.result.name);
            let name_span = Span::styled(
                truncate(&name, 50),
                Style::default().fg(TEXT),
            );

            let size = format_bytes(r.result.size_bytes);
            let size_span = Span::styled(
                format!("{:>10} ", size),
                Style::default().fg(ALT),
            );

            let seeders = format_count(r.result.seeders);
            let seeder_span = Span::styled(
                format!("{:>6} ", seeders),
                Style::default().fg(if r.result.seeders > 0 { GOOD } else { WARN }),
            );

            let src_color = source_color(&r.result.source);
            let tag_span = Span::styled(
                format!(" [{:>5}]  ", r.result.source.tag()),
                Style::default().fg(src_color),
            );

            let relative = format_relative(r.result.added.unwrap_or(0));
            let date_span = if relative.is_empty() {
                Span::raw("        ")
            } else {
                Span::styled(format!("{:>8}", relative), Style::default().fg(ALT))
            };

            ListItem::new(Line::from(vec![tag_span, name_span, size_span, seeder_span, date_span]))
        })
        .collect();

    let list = List::new(items)
        .highlight_symbol("❯ ")
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_stateful_widget(list, list_area, list_state);
}

use ratatui::layout::{Constraint, Layout};
