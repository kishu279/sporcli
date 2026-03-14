// ACTIONS to SPOTIFY
#[derive(Debug, Clone)]
pub enum Action {
    Authenticate,
    Play,
    Pause,
    GetCurrentTrack,
    PreviousTrack,
    NextTrack,
    VolumeUp,
    VolumeDown,
    GetPlaylists,
    GetLikedSongs,
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
    Playlists(Vec<Playlist>),
    TrackInfo(Track),
    TrackList(Vec<Track>),
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
