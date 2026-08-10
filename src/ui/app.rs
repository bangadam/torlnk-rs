use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::CrosstermBackend};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::style::{Color, Style, Modifier};
use ratatui::text::{Line, Span};
use std::io::stdout;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::config::config::Config;
use crate::download::queue::{DownloadQueue, QueueEvent};
use crate::download::types::DownloadStatus;
use crate::sources::{all_sources, sources_by_group, TorrentResult};

use super::state::{
    CaptureMode, Category, Notice, NoticeLevel, Region, SearchResult, Section, View,
};
use super::theme::{ACCENT, DIM, TEXT};
use super::{
    downloads, footer, help, results, seeding, sidebar, splash,
};

pub struct App {
    pub config: Config,
    pub queue: Arc<DownloadQueue>,
    pub view: View,
    pub section: Section,
    pub region: Region,
    pub category: Category,
    pub results: Vec<SearchResult>,
    pub search_query: String,
    pub search_input: String,
    pub filter_input: String,
    pub searching: bool,
    pub hide_dead: bool,
    pub show_help: bool,
    pub capture: CaptureMode,
    pub sidebar_state: ratatui::widgets::ListState,
    pub results_state: ratatui::widgets::ListState,
    pub downloads_state: ratatui::widgets::ListState,
    pub seeds_state: ratatui::widgets::ListState,
    pub notice: Option<Notice>,
    pub search_rx: Option<mpsc::UnboundedReceiver<Vec<SearchResult>>>,
    pub queue_rx: Option<mpsc::UnboundedReceiver<QueueEvent>>,
    pub exit: bool,
}

impl App {
    pub fn new(config: Config, queue: Arc<DownloadQueue>) -> Self {
        let mut sidebar_state = ratatui::widgets::ListState::default();
        sidebar_state.select(Some(0));

        Self {
            config,
            queue,
            view: View::Browser,
            section: Section::Category(Category::All),
            region: Region::Content,
            category: Category::All,
            results: vec![],
            search_query: String::new(),
            search_input: String::new(),
            filter_input: String::new(),
            searching: false,
            hide_dead: false,
            show_help: false,
            capture: CaptureMode::None,
            sidebar_state,
            results_state: ratatui::widgets::ListState::default(),
            downloads_state: ratatui::widgets::ListState::default(),
            seeds_state: ratatui::widgets::ListState::default(),
            notice: None,
            search_rx: None,
            queue_rx: None,
            exit: false,
        }
    }

    fn sidebar_items_count(&self) -> usize {
        Category::all().len() + 2
    }

    fn selected_sidebar_index(&self) -> usize {
        self.sidebar_state.selected().unwrap_or(0)
    }

    fn sync_section_from_sidebar(&mut self) {
        let idx = self.selected_sidebar_index();
        let cats = Category::all();
        if idx < cats.len() {
            self.section = Section::Category(cats[idx]);
            self.category = cats[idx];
        } else if idx == cats.len() + 1 {
            self.section = Section::Downloads;
        } else if idx == cats.len() + 2 {
            self.section = Section::Seeding;
        }
    }

    fn move_sidebar(&mut self, delta: i32) {
        let count = self.sidebar_items_count() as i32;
        let mut idx = self.selected_sidebar_index() as i32 + delta;
        if idx < 0 { idx = count - 1; }
        if idx >= count { idx = 0; }
        self.sidebar_state.select(Some(idx as usize));
        self.sync_section_from_sidebar();
    }

    fn tab_next(&mut self) {
        self.region = match (self.region, self.section) {
            (Region::Sidebar, _) => Region::Content,
            (Region::Content, _) => Region::Sidebar,
            (Region::Help, _) => Region::Sidebar,
        };
    }

    fn move_list(&mut self, delta: i32) {
        let state = match self.section {
            Section::Category(_) => &mut self.results_state,
            Section::Downloads => &mut self.downloads_state,
            Section::Seeding => &mut self.seeds_state,
        };
        let len = match self.section {
            Section::Category(_) => self.results.len(),
            Section::Downloads => self.queue.get_items_sync().len(),
            Section::Seeding => self.queue.get_seeds_sync().len(),
        } as i32;
        if len == 0 { return; }
        let mut idx = state.selected().unwrap_or(0) as i32 + delta;
        if idx < 0 { idx = len - 1; }
        if idx >= len { idx = 0; }
        state.select(Some(idx as usize));
    }

    fn set_notice(&mut self, msg: &str, level: NoticeLevel) {
        self.notice = Some(Notice {
            message: msg.to_string(),
            level,
            at: chrono::Utc::now().timestamp_millis(),
        });
    }

    fn notice_success(&mut self, msg: &str) { self.set_notice(msg, NoticeLevel::Success); }
    fn notice_error(&mut self, msg: &str) { self.set_notice(msg, NoticeLevel::Error); }
    fn notice_warn(&mut self, msg: &str) { self.set_notice(msg, NoticeLevel::Warn); }
    fn notice_info(&mut self, msg: &str) { self.set_notice(msg, NoticeLevel::Info); }

    fn start_search(&mut self) {
        if self.search_input.is_empty() { return; }
        self.searching = true;
        self.view = View::Browser;
        self.region = Region::Content;
        self.search_query = self.search_input.clone();
        self.results.clear();
        self.results_state.select(Some(0));
    }

    async fn handle_key(&mut self, key: KeyEvent) {
        if self.show_help {
            if key.code == KeyCode::Esc || key.code == KeyCode::Char('?') || key.code == KeyCode::Char('q') {
                self.show_help = false;
            }
            return;
        }

        match self.capture {
            CaptureMode::Text => {
                match key.code {
                    KeyCode::Char(c) => {
                        if key.modifiers.contains(KeyModifiers::CONTROL) {
                            if c == 'c' { self.capture = CaptureMode::None; }
                            return;
                        }
                        self.search_input.push(c);
                    }
                    KeyCode::Backspace => { self.search_input.pop(); }
                    KeyCode::Enter => {
                        self.capture = CaptureMode::None;
                        self.start_search();
                        self.spawn_search().await;
                    }
                    KeyCode::Esc => { self.capture = CaptureMode::None; self.search_input.clear(); }
                    _ => {}
                }
                return;
            }
            CaptureMode::Esc => {
                if key.code == KeyCode::Esc {
                    self.capture = CaptureMode::None;
                }
                return;
            }
            CaptureMode::None => {}
        }

        match key.code {
            KeyCode::Char('q') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.exit = true;
                } else {
                    self.exit = true;
                }
            }
            KeyCode::Char('?') => { self.show_help = true; }
            KeyCode::Tab => { self.tab_next(); }
            KeyCode::Esc => {
                // Exit search capture or clear search
                if self.capture == CaptureMode::Text {
                    self.capture = CaptureMode::None;
                }
            }
            _ => {}
        }

        if self.region == Region::Sidebar {
            self.handle_sidebar_key(key).await;
        } else {
            self.handle_content_key(key).await;
        }
    }

    async fn handle_sidebar_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.move_sidebar(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_sidebar(1),
            KeyCode::Enter => {
                self.region = Region::Content;
            }
            KeyCode::Char('/') => {
                self.capture = CaptureMode::Text;
                self.search_input.clear();
            }
            _ => {}
        }
    }

    async fn handle_content_key(&mut self, key: KeyEvent) {
        match self.section {
            Section::Category(_) => self.handle_results_key(key).await,
            Section::Downloads => self.handle_downloads_key(key).await,
            Section::Seeding => self.handle_seeds_key(key).await,
        }
    }

    async fn handle_results_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.move_list(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_list(1),
            KeyCode::Char('/') => {
                self.capture = CaptureMode::Text;
                self.search_input = self.search_query.clone();
            }
            KeyCode::Char('d') => {
                if let Some(r) = self.selected_result() {
                    self.queue.add(
                        crate::download::queue::AddInput {
                            id: r.result.id(),
                            name: r.result.name.clone(),
                            magnet: r.result.magnet(),
                            source: Some(r.result.source),
                            size_bytes: Some(r.result.size_bytes),
                        },
                        &self.config.download_dir,
                    ).await;
                    self.notice_success(&format!("Download added: {}", r.result.name));
                } else {
                    self.notice_warn("No result selected");
                }
            }
            KeyCode::Char('y') => {
                if let Some(r) = self.selected_result() {
                    let magnet = r.result.magnet();
                    copy_to_clipboard(&magnet);
                    self.notice_success("Magnet link copied to clipboard");
                } else {
                    self.notice_warn("No result selected");
                }
            }
            KeyCode::Char('z') => {
                self.hide_dead = !self.hide_dead;
                if self.hide_dead {
                    self.notice_info("Hidden dead torrents (0 seeders)");
                } else {
                    self.notice_info("Showing all torrents");
                }
            }
            KeyCode::Char('m') => {
                self.capture = CaptureMode::Text;
                self.search_input.clear();
            }
            _ => {}
        }
    }

    async fn handle_downloads_key(&mut self, key: KeyEvent) {
        let items = self.queue.get_items_sync();
        let idx = self.downloads_state.selected().unwrap_or(0);
        let id = items.get(idx).map(|it| it.id.clone());
        let name = items.get(idx).map(|it| it.name.clone());

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.move_list(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_list(1),
            KeyCode::Char('p') => {
                if let (Some(id), Some(name)) = (&id, &name) {
                    let was_paused = items.iter().find(|it| &it.id == id)
                        .map(|it| it.status == DownloadStatus::Paused)
                        .unwrap_or(false);
                    self.queue.toggle_pause(id).await;
                    if was_paused {
                        self.notice_success(&format!("Resumed: {}", name));
                    } else {
                        self.notice_info(&format!("Paused: {}", name));
                    }
                }
            }
            KeyCode::Char('c') => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.queue.retry_failed().await;
                    self.notice_info("Retrying all failed downloads");
                } else if let (Some(id), Some(name)) = (&id, &name) {
                    self.queue.cancel(id).await;
                    self.notice_warn(&format!("Cancelled: {}", name));
                }
            }
            KeyCode::Char('e') => {
                if let Some(id) = &id {
                    let items = self.queue.get_items_sync();
                    if let Some(it) = items.iter().find(|x| &x.id == id) {
                        open_folder(&it.dir);
                        self.notice_info("Opened download folder");
                    }
                }
            }
            _ => {}
        }
    }

    async fn handle_seeds_key(&mut self, key: KeyEvent) {
        let seeds = self.queue.get_seeds_sync();
        let idx = self.seeds_state.selected().unwrap_or(0);
        let id = seeds.get(idx).map(|s| s.id.clone());
        let name = seeds.get(idx).map(|s| s.name.clone());

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.move_list(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_list(1),
            KeyCode::Char('p') => {
                if let (Some(id), Some(name)) = (&id, &name) {
                    self.queue.stop_seeding(id).await;
                    self.notice_info(&format!("Stopped seeding: {}", name));
                }
            }
            KeyCode::Char('c') => {
                if let (Some(id), Some(name)) = (&id, &name) {
                    self.queue.remove(id, key.modifiers.contains(KeyModifiers::SHIFT)).await;
                    if key.modifiers.contains(KeyModifiers::SHIFT) {
                        self.notice_warn(&format!("Removed + deleted files: {}", name));
                    } else {
                        self.notice_info(&format!("Removed: {}", name));
                    }
                }
            }
            KeyCode::Char('e') => {
                if let Some(id) = &id {
                    let seeds = self.queue.get_seeds_sync();
                    if let Some(s) = seeds.iter().find(|x| &x.id == id) {
                        open_folder(&s.dir);
                        self.notice_info("Opened seed folder");
                    }
                }
            }
            _ => {}
        }
    }

    fn selected_result(&self) -> Option<&SearchResult> {
        let idx = self.results_state.selected().unwrap_or(0);
        self.results.get(idx)
    }

    async fn spawn_search(&mut self) {
        let query = self.search_query.clone();
        let category = self.category;
        let source_count;

        let (tx, rx) = mpsc::unbounded_channel::<Vec<SearchResult>>();
        self.search_rx = Some(rx);

        let sources = if let Some(group) = category.group() {
            all_sources().into_iter().filter(|s| s.groups().contains(&group)).collect::<Vec<_>>()
        } else {
            all_sources()
        };
        source_count = sources.len();

        for source in sources {
            let tx = tx.clone();
            let query = query.clone();
            let source = source.clone();
            tokio::spawn(async move {
                let client = crate::util::net::build_client();
                match source.search(&query, &client, None).await {
                    Ok(results) => {
                        let sr: Vec<SearchResult> = results
                            .into_iter()
                            .map(|r| SearchResult {
                                source_label: source.id().tag().to_string(),
                                source_color: super::theme::source_color(&source.id()),
                                result: r,
                            })
                            .collect();
                        let _ = tx.send(sr);
                    }
                    Err(_) => { let _ = tx.send(vec![]); }
                }
            });
        }

        let _ = source_count;
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        let _ = self.config.clone();

        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(stdout(), crossterm::terminal::EnterAlternateScreen)?;
        crossterm::execute!(stdout(), crossterm::cursor::Hide)?;

        let backend = CrosstermBackend::new(stdout());
        let mut terminal = Terminal::new(backend)?;

        // Start queue tick loop
        let queue = self.queue.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(crate::download::queue::POLL_MS)).await;
                queue.tick().await;
            }
        });

        while !self.exit {
            terminal.draw(|f| self.render(f))?;

            // Drain search results into self.results
            let mut new_results = vec![];
            if let Some(rx) = self.search_rx.as_mut() {
                while let Ok(batch) = rx.try_recv() {
                    if !batch.is_empty() {
                        new_results.extend(batch);
                    }
                }
            }
            if !new_results.is_empty() {
                self.results.extend(new_results);
                self.searching = false;
            }

            // Drain queue events for auto-alerts
            let mut pending_events = vec![];
            if let Some(rx) = self.queue_rx.as_mut() {
                while let Ok(ev) = rx.try_recv() {
                    pending_events.push(ev);
                }
            }
            for ev in pending_events {
                match ev {
                    QueueEvent::Completed(name) => {
                        self.notice_success(&format!("Download complete: {}", name));
                    }
                    QueueEvent::Failed(name) => {
                        self.notice_error(&format!("Download failed: {}", name));
                    }
                    QueueEvent::Update => {}
                }
            }

            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key(key).await;
                }
            }
        }

        // Cleanup
        crossterm::execute!(stdout(), crossterm::cursor::Show)?;
        crossterm::execute!(stdout(), crossterm::terminal::LeaveAlternateScreen)?;
        crossterm::terminal::disable_raw_mode()?;

        self.queue.suspend().await;
        Ok(())
    }

    fn section_title(&self) -> String {
        match self.section {
            Section::Category(cat) => match cat {
                Category::All => "All Torrents",
                Category::Games => "Games",
                Category::Movies => "Movies",
                Category::Tv => "TV Shows",
                Category::Anime => "Anime",
            }.to_string(),
            Section::Downloads => "Downloads".to_string(),
            Section::Seeding => "Seeding".to_string(),
        }
    }

    fn render_title_bar(&self, frame: &mut ratatui::Frame, area: Rect) {
        let title = format!(" torlnk │ {} ", self.section_title());
        let line = Line::from(Span::styled(
            title,
            Style::default()
                .fg(TEXT)
                .add_modifier(Modifier::BOLD)
                .bg(Color::Rgb(50, 45, 65)),
        ));
        frame.render_widget(Paragraph::new(line), area);
    }

    fn render(&mut self, frame: &mut ratatui::Frame) {
        let area = frame.area();

        // Auto-expire notice after 3s
        if let Some(n) = &self.notice {
            let now = chrono::Utc::now().timestamp_millis();
            if now - n.at > 3000 {
                self.notice = None;
            }
        }

        if self.view == View::Splash {
            splash::render_splash(frame, area, None, false);
            if self.show_help {
                help::render_help_overlay(frame, area);
            }
            footer::render_footer(frame, footer_area(area), self.region, self.section);
            return;
        }

        // Layout: title bar (1) + body (min) + footer (1)
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(area);

        self.render_title_bar(frame, outer[0]);

        // Body: sidebar | content
        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(sidebar::SIDEBAR_WIDTH), Constraint::Min(1)])
            .split(outer[1]);

        // Sidebar with counts
        let dl_count = self.queue.get_items_sync().len();
        let seed_count = self.queue.get_seeds_sync().len();
        sidebar::render_sidebar(
            frame,
            body[0],
            self.section,
            self.region,
            &mut self.sidebar_state,
            dl_count,
            seed_count,
        );

        // Content pane with border + focus indication
        let content_focused = self.region == Region::Content;
        let content_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if content_focused { ACCENT } else { DIM }))
            .title(Span::styled(
                format!(" {} ", self.section_title()),
                Style::default().fg(if content_focused { ACCENT } else { DIM }),
            ));
        frame.render_widget(content_block, body[1]);

        let inner = Block::default().borders(Borders::ALL).inner(body[1]);

        let content = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(inner);

        self.render_search_bar(frame, content[0]);

        match self.section {
            Section::Category(_) => {
                results::render_results(
                    frame,
                    content[1],
                    &self.results,
                    &mut self.results_state,
                    &self.search_query,
                    self.searching,
                    self.hide_dead,
                );
            }
            Section::Downloads => {
                let items = self.queue.get_items_sync();
                downloads::render_downloads(frame, content[1], &items, &mut self.downloads_state);
            }
            Section::Seeding => {
                let seeds = self.queue.get_seeds_sync();
                seeding::render_seeding(frame, content[1], &seeds, &mut self.seeds_state);
            }
        }

        // Footer or transient notice
        if let Some(notice) = &self.notice {
            footer::render_notice(frame, outer[2], &notice.message, notice.level);
        } else {
            footer::render_footer(frame, outer[2], self.region, self.section);
        }

        if self.show_help {
            help::render_help_overlay(frame, area);
        }
    }

    fn render_search_bar(&self, frame: &mut ratatui::Frame, area: Rect) {
        let (text, style) = if self.capture == CaptureMode::Text {
            (
                format!(" {} {}", super::theme::ICON_POINTER, self.search_input),
                Style::default().fg(ACCENT),
            )
        } else if !self.search_query.is_empty() {
            (
                format!(" {} {}", super::theme::ICON_POINTER, self.search_query),
                Style::default().fg(TEXT),
            )
        } else {
            (
                " Search torrents... (press /)".to_string(),
                Style::default().fg(DIM),
            )
        };

        frame.render_widget(Paragraph::new(text).style(style), area);
    }
}

fn footer_area(area: Rect) -> Rect {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area)
        [1]
}

fn copy_to_clipboard(text: &str) {
    // Try pbcopy (macOS), xclip (Linux), clip (Windows)
    #[cfg(target_os = "macos")]
    let cmd = "pbcopy";
    #[cfg(target_os = "linux")]
    let cmd = "xclip";
    #[cfg(target_os = "windows")]
    let cmd = "clip";

    let _ = std::process::Command::new(cmd)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(text.as_bytes())?;
            }
            child.wait()
        });
}

fn open_folder(path: &str) {
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(target_os = "linux")]
    let cmd = "xdg-open";
    #[cfg(target_os = "windows")]
    let cmd = "explorer";

    let _ = std::process::Command::new(cmd).arg(path).spawn();
}
