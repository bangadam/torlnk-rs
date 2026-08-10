use ratatui::style::Color;

pub const ACCENT: Color = Color::Rgb(167, 139, 250);
pub const TEXT: Color = Color::Rgb(233, 228, 245);
pub const ALT: Color = Color::Rgb(185, 167, 230);
pub const GOOD: Color = Color::Rgb(134, 214, 162);
pub const WARN: Color = Color::Rgb(240, 197, 96);
pub const BAD: Color = Color::Rgb(238, 125, 146);
pub const BRIGHT: Color = Color::Rgb(216, 180, 254);
pub const DIM: Color = Color::Rgb(82, 76, 96);

pub const ICON_DONE: &str = "✓";
pub const ICON_ERROR: &str = "✗";
pub const ICON_PENDING: &str = "·";
pub const ICON_POINTER: &str = "❯";
pub const ICON_DOT: &str = "·";
pub const ICON_WARN: &str = "⚠";
pub const ICON_BAR: &str = "▌";
pub const ICON_DOWN: &str = "↓";
pub const ICON_UP: &str = "↑";
pub const ICON_PEER: &str = "•";
pub const ICON_PAUSE: &str = "⏸";

pub const RAIL_WIDTH: u16 = 16;
pub const GUTTER: u16 = 2;

pub fn source_color(id: &crate::sources::SourceId) -> Color {
    use crate::sources::SourceId;
    match id {
        SourceId::Fitgirl => ACCENT,
        SourceId::Yts => GOOD,
        SourceId::Eztv => WARN,
        SourceId::Nyaa => BRIGHT,
        SourceId::Subsplease => ALT,
        SourceId::TpbMovies | SourceId::TpbTv => Color::Rgb(95, 208, 197),
        SourceId::X1337Movies | SourceId::X1337Tv => Color::Rgb(246, 165, 92),
        SourceId::Bittorrented => Color::Rgb(125, 184, 240),
    }
}
