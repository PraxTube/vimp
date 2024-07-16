use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders},
    Frame, Terminal,
};
use std::{
    error::Error,
    io,
    path::PathBuf,
    sync::mpsc::Sender,
    time::{Duration, Instant},
};

use crate::load::load_music_files;
use crate::song;
use crate::song::SongInfo;

use crate::ui::active_song_info::render_active_song_info;
use crate::ui::debug;
use crate::ui::song::render_song_list;

pub struct App {
    pub progress: u32,
    pub volume: i32,
    pub songs: Vec<SongInfo>,
    pub song_info: Option<song::SongInfo>,
    pub debugger: debug::Debug,

    current_song_index: usize,
    pause: bool,
    quit: bool,
    tx: Sender<song::ActionData>,
}

impl App {
    pub fn new(tx: Sender<song::ActionData>, path: &PathBuf) -> App {
        App {
            progress: 0,
            volume: 50,
            song_info: None,
            songs: load_music_files(path)
                .iter()
                .map(|f| SongInfo::new(f.to_path_buf()))
                .collect(),
            debugger: debug::Debug::new(),

            current_song_index: 0,
            pause: false,
            quit: false,
            tx,
        }
    }

    pub fn on_tick(&mut self) {
        if self.pause {
            return;
        }
        if self.current_song_index >= self.songs.len() {
            return;
        }

        self.progress += 1;
        if self.progress > self.songs[self.current_song_index].duration {
            self.progress = 0;
            self.try_play_next_song();
        }
    }

    fn try_play_current_song(&mut self) {
        self.debugger.print(&format!("{}", self.current_song_index));
        if self.current_song_index >= self.songs.len() {
            return;
        }
        if self.pause {
            self.toggle_pause_song();
        }
        self.progress = 0;

        self.debugger.print("plaing");

        let new_song_info = self.songs[self.current_song_index].clone();
        self.song_info = Some(new_song_info.clone());
        self.tx
            .send(song::ActionData {
                action: song::Action::AddSong,
                data: song::DataType::SongInfo(new_song_info),
            })
            .unwrap();
    }

    fn change_volume(&mut self, amount: i32) {
        self.volume = (self.volume + amount).max(0).min(100);
        self.tx
            .send(song::ActionData {
                action: song::Action::Volume,
                data: song::DataType::Int(self.volume),
            })
            .unwrap();
    }

    fn try_play_next_song(&mut self) {
        self.current_song_index = (self.current_song_index + 1).min(self.songs.len() - 1);
        self.try_play_current_song();
    }

    fn try_play_previous_song(&mut self) {
        if self.current_song_index != 0 {
            self.current_song_index -= 1;
        }
        self.try_play_current_song();
    }

    pub fn toggle_pause_song(&mut self) {
        if self.song_info.is_none() {
            return;
        }

        self.pause = !self.pause;

        self.tx
            .send(song::ActionData {
                action: song::Action::TogglePause,
                data: song::DataType::Null,
            })
            .unwrap();
    }
}

pub fn setup(tx: Sender<song::ActionData>, path: PathBuf) -> Result<(), Box<dyn Error>> {
    let app = App::new(tx, &path);
    if app.songs.is_empty() {
        println!("There are no songs in the given dir, {:?}. Exiting.", path);
        return Ok(());
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick_rate = Duration::from_secs(1);
    run_app(&mut terminal, app, tick_rate).unwrap();

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        if app.quit {
            return Ok(());
        }

        terminal.draw(|f| ui(f, &mut app))?;
        main_controller(&mut app, tick_rate, &mut last_tick).unwrap();
    }
}

fn main_controller(app: &mut App, tick_rate: Duration, last_tick: &mut Instant) -> io::Result<()> {
    let timeout = tick_rate
        .checked_sub(last_tick.elapsed())
        .unwrap_or_else(|| Duration::from_secs(0));
    if crossterm::event::poll(timeout)? {
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => app.quit = true,
                KeyCode::Char('j') => app.try_play_next_song(),
                KeyCode::Char('k') => app.try_play_previous_song(),
                KeyCode::Char('w') => app.change_volume(5),
                KeyCode::Char('b') => app.change_volume(-5),
                KeyCode::Char(' ') => app.toggle_pause_song(),
                _ => {}
            }
        }
    }
    if last_tick.elapsed() >= tick_rate {
        app.on_tick();
        *last_tick = Instant::now();
    }
    Ok(())
}

fn ui(f: &mut Frame, app: &mut App) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(f.size());

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(main_chunks[0]);
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)].as_ref())
        .split(chunks[1]);

    render_song_list(f, app, chunks[0]);

    let block = Block::default().title("Playing Song").borders(Borders::ALL);
    f.render_widget(block, right_chunks[0]);

    debug::render_active_song_info(f, app, right_chunks[1]);

    match &app.song_info {
        Some(info) => render_active_song_info(f, app, chunks[1], info.clone()),
        None => {}
    }
}
