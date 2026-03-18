mod app_state;
mod events;
mod spotify;
mod storage;
mod worker;

use crate::events::message::{Action, AuthState, StateUpdateEnum};
use crate::worker::{spotify_worker, ui};
use app_state::{AppState, Focus};
use arboard::Clipboard;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
// use std::time::Duration;
use tokio::time::{Duration, timeout};

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let log_dir = format!("{}/.config/sporc", home);
    std::fs::create_dir_all(&log_dir).ok();
    let file_appender = tracing_appender::rolling::never(&log_dir, "sporc.log");
    tracing_subscriber::fmt()
        .with_writer(file_appender)
        .with_ansi(false)
        .with_target(true)
        .with_level(true)
        .with_env_filter(tracing_subscriber::EnvFilter::new("debug"))
        .init();

    tracing::info!("starting sporcli");

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

    tracing::info!("sporcli shutdown complete");

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
                    tracing::info!("[main] AuthStatus received: {:?}", auth_state);
                    if AuthState::Authenticated == auth_state.clone() {
                        tracing::info!("[main] Authenticated! Firing post-auth API calls");

                        // start polling
                        // poll_signal_tx.send(true).ok();

                        // initialize call to the apis
                        action_tx.try_send(Action::GetProfile).ok();
                        action_tx.try_send(Action::GetPlaylists).ok();
                        action_tx.try_send(Action::GetLikedSongs).ok();
                        action_tx.try_send(Action::GetCurrentTrack).ok();
                        action_tx.try_send(Action::GetDevices).ok();
                    }

                    app.auth_state = auth_state;
                }
                // CURRENT TRACK
                StateUpdateEnum::TrackInfo(track) => {
                    app.current_track_info = Some(track);
                    app.is_playing = true;

                    match timeout(Duration::from_secs(10), async {
                        action_tx.try_send(Action::GetCurrentTrack).ok();
                    })
                    .await
                    {
                        Ok(_) => tracing::info!("Fetching the current track"),
                        Err(_) => tracing::info!("Error while fetching"),
                    }
                }
                StateUpdateEnum::PlaybackStatus(is_playing) => {
                    app.is_playing = is_playing;
                }
                StateUpdateEnum::Volume(volume) => {
                    app.volume = Some(volume);
                }
                StateUpdateEnum::Devices(devices) => {
                    app.error_message = None;
                    app.available_devices = Some(devices);
                    let len = app.available_devices.as_ref().map_or(0, |d| d.len());
                    if len == 0 {
                        app.selected_device_index = 0;
                    } else if app.selected_device_index >= len {
                        app.selected_device_index = len - 1;
                    }

                    match timeout(Duration::from_secs(15), async {
                        action_tx.try_send(Action::GetDevices).ok();
                    })
                    .await
                    {
                        Ok(_) => tracing::info!("Fetching the current devices"),
                        Err(_) => tracing::info!("Error while fetching"),
                    }
                }

                // ERROR
                StateUpdateEnum::Error(msg) => {
                    tracing::error!("[main] Error received: {}", msg);
                    app.error_message = Some(msg);

                    // ERROR -> CLEANUP

                    match timeout(Duration::from_secs(5), async {
                        // // call the apis
                        app.error_message = None;

                        // action_tx.try_send(Action::GetProfile).ok();
                        // action_tx.try_send(Action::GetPlaylists).ok();
                        // action_tx.try_send(Action::GetLikedSongs).ok();
                        // action_tx.try_send(Action::GetCurrentTrack).ok();
                        // action_tx.try_send(Action::GetDevices).ok();
                    })
                    .await
                    {
                        Ok(_) => {
                            tracing::info!(
                                "called the api's after few seconds for refreshing the state"
                            )
                        }
                        Err(_) => tracing::info!("Error while fetching with url"),
                    };
                }
                StateUpdateEnum::CopyUrl(url) => {
                    app.auth_url = Some(url);
                }
                StateUpdateEnum::Playlists(playlists) => {
                    tracing::info!("[main] Playlists received: {} items", playlists.len());
                    app.error_message = None;
                    app.playlist = Some(playlists);

                    // stop polling once we have data
                    // poll_signal_tx.send(false).ok();
                }
                StateUpdateEnum::TrackList(playlist_id, music_list) => {
                    tracing::info!(
                        "[main] TrackList received for {}: {} items",
                        playlist_id,
                        music_list.items.len()
                    );
                    app.error_message = None;
                    app.music_list.insert(playlist_id, music_list);
                }
                StateUpdateEnum::UserProfile(profile) => {
                    tracing::info!("[main] UserProfile received: {:?}", profile.display_name);
                    app.error_message = None;
                    app.user_profile = Some(profile);
                }
            }
        }

        terminal.draw(|f| ui::render(f, app))?;

        // EVENT HANDLING
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        tracing::info!("[main] Quit key pressed");
                        return Ok(());
                    }
                    KeyCode::Char('a') => {
                        tracing::info!("[main] Authenticate key pressed");
                        action_tx.try_send(Action::Authenticate).ok();
                    }
                    KeyCode::Char('c') => {
                        let mut clipboard = Clipboard::new().unwrap();

                        if let Some(url) = app.auth_url.clone() {
                            clipboard.set_text(url).ok();
                            tracing::info!("[main] Auth URL copied to clipboard");
                            // clipboard.set().text(url).ok();
                            std::thread::sleep(Duration::from_millis(200));
                        }

                        // action_tx.try_send(Action::CC).ok();
                    }
                    KeyCode::Up => match app.focus {
                        Focus::Playlist => {
                            if app.selected_playlist_index > 0 {
                                app.selected_playlist_index -= 1;

                                // calculaing the scroll and updating
                                if app.selected_playlist_index < app.playlist_scroll_offset {
                                    app.playlist_scroll_offset = app.selected_playlist_index;
                                }

                                // getting the musics list from the playlists
                                if let Some(playlists) = app.playlist.as_ref() {
                                    if let Some(playlist) =
                                        playlists.get(app.selected_playlist_index)
                                    {
                                        let id = playlist.id.clone();
                                        app.active_playlist_id = Some(id.clone());
                                        app.selected_music_index = 0;
                                        app.musiclist_scroll_offset = 0;
                                        if !app.music_list.contains_key(&id) {
                                            tracing::info!(
                                                "[main] GetPlaylistTracks requested: {}",
                                                id
                                            );
                                            action_tx.try_send(Action::GetPlaylistTracks(id)).ok();
                                        }
                                    }
                                }
                            }
                        }
                        Focus::MusicList => {
                            if app.selected_music_index > 0 {
                                app.selected_music_index -= 1;
                            }

                            // calculaing the scroll and updating
                            if app.selected_music_index < app.musiclist_scroll_offset {
                                app.musiclist_scroll_offset = app.selected_music_index;
                            }
                        }
                        Focus::Search => {}
                        Focus::Devices => {}
                    },
                    KeyCode::Down => match app.focus {
                        Focus::Playlist => {
                            let len = app.playlist.as_ref().map_or(0, |p| p.len());
                            if len > 0 && app.selected_playlist_index < len - 1 {
                                app.selected_playlist_index += 1;

                                // calculating the scroll and then updating
                                let visible_rows = app.visible_rows_playlist;
                                if app.selected_playlist_index
                                    >= app.playlist_scroll_offset + visible_rows
                                {
                                    app.playlist_scroll_offset =
                                        app.selected_playlist_index - visible_rows + 1;
                                }

                                // getting the music list on down
                                if let Some(playlists) = app.playlist.as_ref() {
                                    if let Some(playlist) =
                                        playlists.get(app.selected_playlist_index)
                                    {
                                        let id = playlist.id.clone();
                                        app.active_playlist_id = Some(id.clone());
                                        app.selected_music_index = 0;
                                        app.musiclist_scroll_offset = 0;
                                        if !app.music_list.contains_key(&id) {
                                            tracing::info!(
                                                "[main] GetPlaylistTracks requested: {}",
                                                id
                                            );
                                            action_tx.try_send(Action::GetPlaylistTracks(id)).ok();
                                        }
                                    }
                                }
                            }
                        }
                        Focus::MusicList => {
                            let len = app
                                .active_playlist_id
                                .as_ref()
                                .and_then(|id| app.music_list.get(id))
                                .map_or(0, |m| m.items.len());
                            if len > 0 && app.selected_music_index < len - 1 {
                                app.selected_music_index += 1;
                            }

                            if len > 0 && app.selected_music_index < len - 1 {
                                // calculating the scroll and then updating
                                let visible_rows = app.visible_rows_musiclist;
                                if app.selected_music_index
                                    >= app.musiclist_scroll_offset + visible_rows
                                {
                                    app.musiclist_scroll_offset =
                                        app.selected_music_index - visible_rows + 1;
                                }
                            }
                        }
                        Focus::Search => {}
                        Focus::Devices => {}
                    },
                    KeyCode::Tab => {
                        // Cycle focus: Playlist -> MusicList -> Search -> Devices -> Playlist
                        app.focus = match app.focus {
                            Focus::Playlist => Focus::MusicList,
                            Focus::MusicList => Focus::Search,
                            Focus::Search => Focus::Devices,
                            Focus::Devices => Focus::Playlist,
                        };
                    }
                    KeyCode::Left => {
                        if let Focus::Devices = app.focus {
                            if app.selected_device_index > 0 {
                                app.selected_device_index -= 1;
                            }
                        } else {
                            tracing::info!("[main] PreviousTrack requested");
                            action_tx.try_send(Action::PreviousTrack).ok();
                        }
                    }
                    KeyCode::Right => {
                        if let Focus::Devices = app.focus {
                            let len = app.available_devices.as_ref().map_or(0, |d| d.len());
                            if len > 0 && app.selected_device_index < len - 1 {
                                app.selected_device_index += 1;
                            }
                        } else {
                            tracing::info!("[main] NextTrack requested");
                            action_tx.try_send(Action::NextTrack).ok();
                        }
                    }
                    KeyCode::Enter => match app.focus {
                        Focus::Playlist => {
                            // NOT REQUIRED WHILE PRESSING ENTERING
                            // if let Some(playlists) = app.playlist.as_ref() {
                            //     if let Some(playlist) = playlists.get(app.selected_playlist_index) {
                            //         let id = playlist.id.clone();
                            //         app.active_playlist_id = Some(id.clone());
                            //         app.selected_music_index = 0;
                            //         app.musiclist_scroll_offset = 0;
                            //         if !app.music_list.contains_key(&id) {
                            //             tracing::info!(
                            //                 "[main] GetPlaylistTracks requested: {}",
                            //                 id
                            //             );
                            //             action_tx.try_send(Action::GetPlaylistTracks(id)).ok();
                            //         }
                            //     }
                            // }
                        }
                        Focus::Devices => {
                            if let Some(devices) = app.available_devices.as_ref() {
                                if let Some(device) = devices.get(app.selected_device_index) {
                                    tracing::info!("[main] ChangeDevice requested: {}", device.id);
                                    action_tx
                                        .try_send(Action::ChangeDevice(device.id.clone()))
                                        .ok();
                                }
                            }
                        }
                        _ => {}
                    },
                    KeyCode::Char(' ') => {
                        if app.focus != Focus::MusicList {
                            if app.is_playing {
                                action_tx.try_send(Action::Pause).ok();
                            } else {
                                action_tx.try_send(Action::Play(None)).ok();
                            }
                        } else {
                            // made an action to play of selected music

                            action_tx.try_send(Action::Play(None)).ok();
                        }
                    }
                    _ => {}
                }
            }
        }
        app.on_tick();
    }
}
