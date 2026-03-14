mod app_state;
mod events;
mod spotify;
mod storage;
mod worker;

use crate::events::message::AuthState;
use crate::events::message::{Action, StateUpdateEnum};
use crate::worker::{spotify_worker, ui};
use app_state::AppState;
use arboard::Clipboard;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    // App State
    let mut app = AppState::new();

    // channels
    let (action_tx, action_rx) = tokio::sync::mpsc::channel::<Action>(32);
    let (state_tx, state_rx) = tokio::sync::mpsc::channel::<StateUpdateEnum>(32);

    let spotify_worker_handler = tokio::spawn(async move {
        spotify_worker(action_rx, state_tx).await.ok();
    });

    // default sending the authenticate request
    action_tx.send(Action::Authenticate).await.ok();

    // terminal ui logic
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    run_app(&mut terminal, &mut app, action_tx, state_rx).await?;

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    // worker_handle.await.ok();
    spotify_worker_handler.abort();

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut AppState,
    action_tx: tokio::sync::mpsc::Sender<Action>,
    mut state_rx: tokio::sync::mpsc::Receiver<StateUpdateEnum>,
) -> io::Result<()> {
    loop {
        while let Ok(update) = state_rx.try_recv() {
            match update {
                StateUpdateEnum::AuthStatus(auth_state) => {
                    // tracing::info!("[main] AuthStatus received: {:?}", auth_state);
                    if AuthState::Authenticated == auth_state.clone() {
                        // tracing::info!("[main] Authenticated! Firing post-auth API calls");

                        // start polling
                        // poll_signal_tx.send(true).ok();

                        // initialize call to the apis
                        action_tx.try_send(Action::GetProfile).ok();
                        action_tx.try_send(Action::GetPlaylists).ok();
                        action_tx.try_send(Action::GetLikedSongs).ok();
                        action_tx.try_send(Action::GetCurrentTrack).ok();
                    }

                    app.auth_state = auth_state;
                }
                StateUpdateEnum::TrackInfo(track) => {
                    app.current_track_info = Some(track);
                }
                StateUpdateEnum::PlaybackStatus(is_playing) => {
                    app.is_playing = is_playing;
                }
                StateUpdateEnum::Volume(volume) => {
                    app.volume = Some(volume);
                }
                StateUpdateEnum::Error(msg) => {
                    // tracing::error!("[main] Error received: {}", msg);
                    app.error_message = Some(msg);
                }
                StateUpdateEnum::CopyUrl(url) => {
                    app.auth_url = Some(url);
                }
                StateUpdateEnum::Playlists(playlists) => {
                    // tracing::info!("[main] Playlists received: {} items", playlists.len());
                    app.error_message = None;
                    app.playlist = Some(playlists.iter().map(|p| p.name.clone()).collect());

                    // stop polling once we have data
                    // poll_signal_tx.send(false).ok();
                }
                StateUpdateEnum::TrackList(tracks) => {
                    // tracing::info!("[main] TrackList received: {} items", tracks.len());
                    app.error_message = None;
                    app.music_list = Some(
                        tracks
                            .iter()
                            .map(|t| format!("{} - {}", t.name, t.artist))
                            .collect(),
                    );
                }
                StateUpdateEnum::UserProfile(profile) => {
                    // tracing::info!("[main] UserProfile received: {:?}", profile.display_name);
                    app.error_message = None;
                    app.user_profile = Some(profile);
                }
            }
        }

        terminal.draw(|f| ui::render(f, app))?;

        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('a') => {
                        action_tx.try_send(Action::Authenticate).ok();
                    }
                    KeyCode::Char('c') => {
                        let mut clipboard = Clipboard::new().unwrap();

                        if let Some(url) = app.auth_url.clone() {
                            clipboard.set_text(url).ok();
                            // clipboard.set().text(url).ok();
                            std::thread::sleep(Duration::from_millis(200));
                        }

                        // action_tx.try_send(Action::CC).ok();
                    }
                    // KeyCode::Up => match app.focus {
                    //     Focus::Playlist => {
                    //         if app.selected_playlist_index > 0 {
                    //             app.selected_playlist_index -= 1;
                    //         }
                    //     }
                    //     Focus::MusicList => {
                    //         if app.selected_music_index > 0 {
                    //             app.selected_music_index -= 1;
                    //         }
                    //     }
                    //     Focus::Search => {}
                    // },
                    // KeyCode::Down => match app.focus {
                    //     Focus::Playlist => {
                    //         let len = app.playlist.as_ref().map_or(0, |p| p.len());
                    //         if len > 0 && app.selected_playlist_index < len - 1 {
                    //             app.selected_playlist_index += 1;
                    //         }
                    //     }
                    //     Focus::MusicList => {
                    //         let len = app.music_list.as_ref().map_or(0, |m| m.len());
                    //         if len > 0 && app.selected_music_index < len - 1 {
                    //             app.selected_music_index += 1;
                    //         }
                    //     }
                    //     Focus::Search => {}
                    //                     },
                    // KeyCode::Tab => {
                    //     // Cycle focus: Playlist -> MusicList -> Search -> Playlist
                    //     app.focus = match app.focus {
                    //         Focus::Playlist => Focus::MusicList,
                    //         Focus::MusicList => Focus::Search,
                    //         Focus::Search => Focus::Playlist,
                    //     };
                    // }
                    _ => {}
                }
            }
        }
        app.on_tick();
    }
}
