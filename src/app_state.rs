use crate::events::message::{AuthState, Track, UserProfile};

pub struct AppState {
    pub auth_url: Option<String>,
    pub status: Option<String>,
    pub volume: Option<u8>,
    pub playlist: Option<Vec<String>>,
    pub search: Option<String>,
    pub music_list: Option<Vec<String>>,

    pub auth_state: AuthState,             // From events::AuthState
    pub current_track_info: Option<Track>, // From events::Track
    pub user_profile: Option<UserProfile>, // From events::UserProfile
    pub is_playing: bool,
    pub focus: Focus,
    pub selected_playlist_index: usize,
    pub selected_music_index: usize,

    pub tick: usize,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Focus {
    Playlist,
    Search,
    MusicList,
}

impl AppState {
    pub fn new() -> AppState {
        AppState {
            auth_state: AuthState::NotAuthenticated,
            current_track_info: None,
            user_profile: None,
            error_message: None,
            is_playing: false,
            music_list: None,
            playlist: None,
            search: None,
            status: None,
            volume: None,
            auth_url: None,
            focus: Focus::Playlist,
            selected_playlist_index: 0,
            selected_music_index: 0,
            tick: 0,
        }
    }

    pub fn on_tick(&mut self) {
        // progress bar
        self.tick += 1;
    }

    pub fn reset_tick(&mut self) {
        self.tick = 0;
    }
}
