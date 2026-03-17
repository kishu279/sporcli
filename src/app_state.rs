use std::collections::HashMap;
use crate::events::message::{AuthState, Device, MusicList, Playlist, Track, UserProfile};

pub struct AppState {
    pub auth_url: Option<String>,
    pub status: Option<String>,
    pub volume: Option<u8>,
    pub playlist: Option<Vec<Playlist>>,
    pub search: Option<String>,
    pub music_list: HashMap<String, MusicList>,
    pub active_playlist_id: Option<String>,
    pub available_devices: Option<Vec<Device>>,

    pub auth_state: AuthState,             // From events::AuthState
    pub current_track_info: Option<Track>, // From events::Track
    pub user_profile: Option<UserProfile>, // From events::UserProfile
    pub is_playing: bool,
    pub focus: Focus,
    
    pub tick: usize,
    pub error_message: Option<String>,
    
    // SELECTED INDEX ON LIST
    pub selected_playlist_index: usize,
    pub selected_music_index: usize,
    pub selected_device_index: usize,
    // SCROLL
    pub playlist_scroll_offset: usize,
    pub musiclist_scroll_offset: usize,
    pub visible_rows_playlist: usize,
    pub visible_rows_musiclist: usize,
}

#[derive(Debug, Clone)]
pub enum Focus {
    Playlist,
    Search,
    MusicList,
    Devices,
}


impl AppState {
    pub fn new() -> AppState {
        AppState {
            auth_state: AuthState::NotAuthenticated,
            current_track_info: None,
            user_profile: None,
            error_message: None,
            is_playing: false,
            music_list: HashMap::new(),
            active_playlist_id: None,
            available_devices: None,
            playlist: None,
            search: None,
            status: None,
            volume: None,
            auth_url: None,
            focus: Focus::Playlist,
            selected_playlist_index: 0,
            selected_music_index: 0,
            selected_device_index: 0,
            tick: 0,
            playlist_scroll_offset: 0,
            visible_rows_playlist: 0,
            musiclist_scroll_offset: 0,
            visible_rows_musiclist: 0,
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
