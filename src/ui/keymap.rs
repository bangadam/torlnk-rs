pub struct Hint {
    pub keys: &'static str,
    pub label: &'static str,
}

pub struct HelpGroup {
    pub title: &'static str,
    pub hints: &'static [Hint],
}

pub const HELP_GROUPS: &[HelpGroup] = &[
    HelpGroup {
        title: "Navigate",
        hints: &[
            Hint { keys: "↑↓←→ / hjkl", label: "Navigate panes and lists" },
            Hint { keys: "↵", label: "Open" },
            Hint { keys: "tab", label: "Switch pane" },
            Hint { keys: "esc", label: "Back" },
            Hint { keys: "o", label: "Default download folder" },
            Hint { keys: "t", label: "Extra trackers" },
            Hint { keys: "q", label: "Quit" },
        ],
    },
    HelpGroup {
        title: "Search",
        hints: &[
            Hint { keys: "/", label: "Edit search" },
            Hint { keys: "f", label: "Filter list" },
            Hint { keys: "d", label: "Download (shift+d: folder)" },
            Hint { keys: "s", label: "Sort results" },
            Hint { keys: "z", label: "Hide dead torrents" },
            Hint { keys: "y", label: "Copy magnet" },
            Hint { keys: "↵", label: "Open details" },
            Hint { keys: "e", label: "Export as .torrent" },
            Hint { keys: "m", label: "Paste magnet" },
        ],
    },
    HelpGroup {
        title: "Downloads",
        hints: &[
            Hint { keys: "p", label: "Pause/resume" },
            Hint { keys: "c", label: "Cancel or remove (shift+c: all)" },
            Hint { keys: "f", label: "Retry failed" },
            Hint { keys: "d", label: "Download again" },
            Hint { keys: "e", label: "Open folder" },
            Hint { keys: "s", label: "Export torrent file" },
        ],
    },
    HelpGroup {
        title: "Seeding",
        hints: &[
            Hint { keys: "p", label: "Pause/resume" },
            Hint { keys: "c", label: "Remove (shift+c: all)" },
            Hint { keys: "e", label: "Open folder" },
        ],
    },
];

pub fn footer_hints(region: crate::ui::state::Region, section: crate::ui::state::Section) -> Vec<Hint> {
    use crate::ui::state::{Region, Section};

    if region == Region::Sidebar {
        return vec![
            Hint { keys: "↑↓", label: "Move" },
            Hint { keys: "↵", label: "Open" },
            Hint { keys: "tab", label: "Switch" },
            Hint { keys: "?", label: "Keys" },
            Hint { keys: "q", label: "Quit" },
        ];
    }

    match section {
        Section::Seeding => vec![
            Hint { keys: "p", label: "Pause" },
            Hint { keys: "c", label: "Remove" },
            Hint { keys: "e", label: "Folder" },
            Hint { keys: "tab", label: "Switch" },
            Hint { keys: "?", label: "Keys" },
        ],
        Section::Downloads => vec![
            Hint { keys: "p", label: "Pause" },
            Hint { keys: "c", label: "Cancel" },
            Hint { keys: "e", label: "Folder" },
            Hint { keys: "s", label: "Export" },
            Hint { keys: "tab", label: "Switch" },
            Hint { keys: "?", label: "Keys" },
        ],
        Section::Category(_) => vec![
            Hint { keys: "↑↓", label: "Move" },
            Hint { keys: "d", label: "Download" },
            Hint { keys: "y", label: "Copy" },
            Hint { keys: "s", label: "Sort" },
            Hint { keys: "/", label: "Search" },
            Hint { keys: "tab", label: "Switch" },
            Hint { keys: "?", label: "Keys" },
        ],
    }
}
