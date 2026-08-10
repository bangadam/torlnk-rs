use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::download::types::{DownloadStatus, QueueItem};
use super::theme::{BAD, GOOD, ICON_BAR, ICON_DOWN, ICON_PAUSE, TEXT, WARN, ALT};
use crate::util::format::{clean_text, format_bytes, format_bytes_per_sec, format_eta_short, truncate};

pub fn render_downloads(
    frame: &mut Frame,
    area: Rect,
    items: &[QueueItem],
    list_state: &mut ListState,
) {
    if items.is_empty() {
        let msg = Paragraph::new("  No downloads yet.\n  Press / to search, then d on any result to download.")
            .style(Style::default().fg(ALT))
            .wrap(Wrap { trim: true });
        frame.render_widget(msg, area);
        return;
    }

    // Split: active downloads on top, recently completed below
    let active: Vec<&QueueItem> = items.iter().filter(|it| it.status != DownloadStatus::Completed).collect();

    let list_items: Vec<ListItem> = active
        .iter()
        .map(|it| {
            let name = clean_text(&it.name);
            let status_icon = match it.status {
                DownloadStatus::Downloading => ICON_DOWN,
                DownloadStatus::Queued => "⏳",
                DownloadStatus::Paused => ICON_PAUSE,
                DownloadStatus::Failed => "✗",
                DownloadStatus::Completed => "✓",
            };
            let status_color = match it.status {
                DownloadStatus::Downloading => GOOD,
                DownloadStatus::Queued => WARN,
                DownloadStatus::Paused => ALT,
                DownloadStatus::Failed => BAD,
                DownloadStatus::Completed => GOOD,
            };

            let progress_bar = render_progress_bar(it.progress);
            let speed = format_bytes_per_sec(it.speed);
            let eta = it.eta.map(format_eta_short).unwrap_or_default();
            let peers = it.peers;
            let size = format_bytes(it.total_bytes);

            let name_line = Line::from(vec![
                Span::styled(format!("{} ", status_icon), Style::default().fg(status_color)),
                Span::styled(truncate(&name, 50), Style::default().fg(TEXT)),
            ]);

            let stats_line = Line::from(vec![
                Span::styled(format!("{} ", progress_bar), Style::default().fg(status_color)),
                Span::styled(format!("{:3}% ", it.progress), Style::default().fg(TEXT)),
                Span::styled(format!("{:>10} ", size), Style::default().fg(ALT)),
                Span::styled(format!("{}{} ", ICON_DOWN, speed), Style::default().fg(GOOD)),
                if !eta.is_empty() {
                    Span::styled(format!("{} ", eta), Style::default().fg(ALT))
                } else {
                    Span::raw("")
                },
                Span::styled(format!("{}{}", crate::ui::theme::ICON_PEER, peers), Style::default().fg(TEXT)),
            ]);

            ListItem::new(vec![name_line, stats_line])
        })
        .collect();

    let list = List::new(list_items)
        .highlight_symbol("❯ ")
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_stateful_widget(list, area, list_state);
}

fn render_progress_bar(progress: u8) -> String {
    let width = 20;
    let filled = (progress as usize * width) / 100;
    let bar: String = std::iter::repeat(ICON_BAR).take(filled).collect();
    let empty: String = std::iter::repeat(' ').take(width - filled.min(width)).collect();
    format!("[{}{}]", bar, empty)
}
