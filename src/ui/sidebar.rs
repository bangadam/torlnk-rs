use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;

use super::state::{Category, Region, Section};
use super::theme::{ACCENT, ALT, DIM, TEXT};

/// Width including the border (2 cols). Inner list gets width-2.
pub const SIDEBAR_WIDTH: u16 = 18;

pub fn render_sidebar(
    frame: &mut Frame,
    area: Rect,
    section: Section,
    region: Region,
    list_state: &mut ListState,
    dl_count: usize,
    seed_count: usize,
) {
    let focused = region == Region::Sidebar;
    let border_color = if focused { ACCENT } else { DIM };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            " Browse ",
            Style::default().fg(if focused { ACCENT } else { ALT }),
        ));

    // Build list items
    let mut items: Vec<ListItem> = Category::all()
        .iter()
        .map(|cat| {
            let is_active = section == Section::Category(*cat);
            let style = if is_active {
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TEXT)
            };
            ListItem::new(Line::from(Span::styled(cat.label(), style)))
        })
        .collect();

    // Separator
    items.push(ListItem::new(Line::from("")));

    // Downloads with count
    let dl_label = if dl_count > 0 {
        format!("Downloads ({})", dl_count)
    } else {
        "Downloads".to_string()
    };
    let dl_active = section == Section::Downloads;
    let dl_style = if dl_active {
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(TEXT)
    };
    items.push(ListItem::new(Line::from(Span::styled(dl_label, dl_style))));

    // Seeding with count
    let seed_label = if seed_count > 0 {
        format!("Seeding ({})", seed_count)
    } else {
        "Seeding".to_string()
    };
    let seed_active = section == Section::Seeding;
    let seed_style = if seed_active {
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(TEXT)
    };
    items.push(ListItem::new(Line::from(Span::styled(seed_label, seed_style))));

    let list = List::new(items)
        .block(block)
        .highlight_symbol("❯ ")
        .highlight_style(Style::default().fg(ACCENT));

    frame.render_stateful_widget(list, area, list_state);
}
