use indexmap::IndexMap;

// ACTIONS to SPOTIFY
#[derive(Debug, Clone)]
pub enum Action {
    Authenticate,
    Play,
    Pause,
    GetCurrentTrack,
    GetDevices,
    ChangeDevice(String),
    PreviousTrack,
    NextTrack,
    VolumeUp,
    VolumeDown,
    GetPlaylists,
    GetLikedSongs,
    GetPlaylistTracks(String),
    Search(String),
    GetProfile,
    CC,
    Quit,
}

// SPOTIFY -> ACTIONS
#[derive(Debug, Clone)]
pub enum StateUpdateEnum {
    AuthStatus(AuthState),
    PlaybackStatus(bool),
    Volume(u8),
    Devices(Vec<Device>),
    Playlists(Vec<Playlist>),
    TrackInfo(Track),
    TrackList(String, MusicList),
    CopyUrl(String),
    UserProfile(UserProfile),
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuthState {
    Authenticated,
    NotAuthenticated,
    Authenticating { url: String },
    Error(String),
}

#[derive(Debug, Clone)]
pub struct Track {
    pub name: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub progress_ms: u64,
}

#[derive(Debug, Clone)]
pub struct TrackItem {
    pub id: String,
    pub name: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub uri: String,
}

#[derive(Debug, Clone)]
pub struct MusicList {
    pub items: IndexMap<String, TrackItem>,
    pub total: usize,
    pub next: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub track_count: u64,
}

#[derive(Debug, Clone)]
pub struct UserProfile {
    pub id: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub country: Option<String>,
    pub product: Option<String>,
    pub followers: u64,
    pub profile_image_url: Option<String>,
    pub uri: String,
}

#[derive(Debug, Clone)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub is_active: bool,
    pub device_type: String,
    pub volume_percent: Option<u8>,
}
