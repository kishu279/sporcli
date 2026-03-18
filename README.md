# Sporcli

A terminal-based Spotify client built with Rust.

## Features

- Browse and play your Spotify playlists
- View currently playing track information
- Control playback (play, pause, next, previous)
- Search functionality
- Device management
- Clean terminal UI with keyboard navigation

## Prerequisites

- Rust (latest stable version)
- Spotify Premium account
- Spotify Developer Application credentials

## Setup

1. **Create a Spotify Application**
   - Go to [Spotify Developer Dashboard](https://developer.spotify.com/dashboard)
   - Create a new application
   - Add `http://localhost:8888/callback` to the Redirect URIs
   - Note your Client ID and Client Secret

2. **Configure Environment Variables**
   
   Create a `.env` file in the project root:
   ```
   SPOTIFY_CLIENT_ID=your_client_id_here
   SPOTIFY_CLIENT_SECRET=your_client_secret_here
   SPOTIFY_REDIRECT_URI=http://localhost:8888/callback
   ```

3. **Build and Run**
   ```bash
   cargo build --release
   cargo run
   ```

## Controls

### Authentication
- `a` - Start authentication process
- `c` - Copy authentication URL to clipboard
- `q` - Quit application

### Navigation
- `Tab` - Cycle focus between panels (Playlist → Music List → Search → Devices)
- `↑` - Move selection up (in Playlist or Music List panel)
- `↓` - Move selection down (in Playlist or Music List panel)
- `←` - Navigate devices left (when Devices panel is focused)
- `→` - Navigate devices right (when Devices panel is focused)

### Playlist Panel (when focused)
- `↑` / `↓` - Navigate through playlists
- Automatically loads tracks when navigating to a playlist

### Music List Panel (when focused)
- `↑` / `↓` - Navigate through tracks
- `Space` - Play selected track

### Devices Panel (when focused)
- `←` / `→` - Navigate through available devices
- `Enter` - Switch playback to selected device

### Playback Controls (global)
- `Space` - Play selected track (in Music List) OR Play/Pause current playback (in other panels)
- `←` - Previous track (when not in Devices panel)
- `→` - Next track (when not in Devices panel)

### General
- `q` - Quit application

## UI Layout

```
┌─────────────────────────────────────────────────────────────┐
│                         Spotify                             │
├──────┬──────────────────────────────────┬───────────────────┤
│ Logo │  Playlist  │   Music List       │   Track Info      │
│      │            │                    ├───────────────────┤
│      │            │                    │   Devices         │
│      ├────────────┴────────────────────┴───────────────────┤
│      │  Search    │   Player Controls                      │
└──────┴────────────────────────────────────────────────────┘
│ [←/→] Prev/Next  [space] Play/Pause  [q] Quit             │
└─────────────────────────────────────────────────────────────┘
```

### Panels

1. **Playlist Panel** (Left)
   - Shows your Spotify playlists
   - Navigate with ↑/↓ when focused
   - Tracks load automatically when you navigate to a playlist
   - Yellow border when focused

2. **Music List Panel** (Center)
   - Shows tracks from selected playlist
   - Format: `Track Name - Artist`
   - Navigate with ↑/↓ when focused
   - Press Space to play selected track
   - Yellow border when focused
   - Tracks load automatically when navigating playlists

3. **Track Info Panel** (Top Right)
   - Displays currently playing track
   - Shows track name, artist, album
   - Progress bar visualization

4. **Devices Panel** (Bottom Right)
   - Lists available Spotify devices
   - Active device marked with ●
   - Inactive devices marked with ○
   - Navigate with ←/→ when focused
   - Press Enter to switch to selected device
   - Yellow border when focused

5. **Search Box** (Bottom Left)
   - Currently displays "Search..." placeholder
   - Press Tab to focus
   - Yellow border when focused
   - (Search functionality in development)

6. **Player Controls** (Bottom Center)
   - Visual playback controls
   - ⏪ Previous | ▶/⏸ Play/Pause | ⏩ Next

## Features in Detail

### Playlist Management
- Automatically loads your Spotify playlists on startup
- Caches playlist tracks to avoid redundant API calls
- Supports both private and collaborative playlists

### Playback Control
- Play any track from your playlists
- Control playback state (play/pause)
- Skip to next/previous tracks
- Works with any active Spotify device

### Device Selection
- View all available Spotify Connect devices
- See which device is currently active
- Navigate and select devices

## Troubleshooting

### Authentication Issues
- Ensure your Spotify app credentials are correct
- Check that the redirect URI matches exactly
- Make sure you have a Spotify Premium account

### Playback Issues
- Ensure you have an active Spotify device
- Check that the device is not in private session mode
- Verify your Spotify Premium subscription is active

### No Playlists Showing
- Wait a moment for playlists to load (spinner will show)
- Check your internet connection
- Verify authentication was successful

## Required Spotify Scopes

The application requests the following permissions:
- `playlist-read-private` - Read your private playlists
- `playlist-read-collaborative` - Read collaborative playlists
- `user-library-read` - Access your saved tracks
- `user-read-currently-playing` - Read currently playing track
- `user-modify-playback-state` - Control playback
- `user-read-playback-state` - Read playback state
- `user-read-recently-played` - Access recently played tracks

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
