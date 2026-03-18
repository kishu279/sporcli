use crate::spotify::auth_handler;
use crate::storage;
use indexmap::IndexMap;

const BASE_URL: &str = "https://api.spotify.com";

#[derive(Debug)]
pub struct SpotifyClient {
    pub client_id: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
    http: reqwest::Client,
}

impl SpotifyClient {
    pub fn new() -> Self {
        Self {
            access_token: None,
            refresh_token: None,
            client_id: "970f2de2fa8141108ea3fbd3c9498985".into(),
            expires_at: None,
            http: reqwest::Client::new(),
        }
    }

    pub fn set_token(&mut self, access_token: String, refresh_token: String, expires_at: i64) {
        self.access_token = Some(access_token);
        self.refresh_token = Some(refresh_token);
        self.expires_at = Some(expires_at);
    }

    pub fn token(&self) -> Result<&str, Box<dyn std::error::Error + Send + Sync>> {
        self.access_token
            .as_deref()
            .ok_or_else(|| "Not authenticated".into())
    }

    // authentication flows
    pub async fn authenticate_flow(
        &mut self,
        state_tx: &tokio::sync::mpsc::Sender<crate::events::message::StateUpdateEnum>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // load credentials
        match storage::load_credentials() {
            Ok(token)
                if token.is_valid()
                    && !token.is_expired()
                    && auth_handler::has_required_scopes(token.scope.as_deref()) =>
            {
                tracing::info!(
                    "[auth_flow] Found valid cached token, expires_at: {:?}",
                    token.expires_at
                );
                // Token is valid and not expired, use it
                self.set_token(
                    token.access_token.unwrap(),
                    token.refresh_token.clone().unwrap(),
                    token.expires_at.unwrap(),
                );
                // Send authenticated state
                state_tx
                    .send(crate::events::message::StateUpdateEnum::AuthStatus(
                        crate::events::message::AuthState::Authenticated,
                    ))
                    .await?;
            }
            Ok(token)
                if token.is_valid()
                    && auth_handler::has_required_scopes(token.scope.as_deref()) =>
            {
                tracing::warn!(
                    "[auth_flow] Token expired (expires_at: {:?}), attempting refresh",
                    token.expires_at
                );
                // Token expired but valid, try refresh
                let refresh_token_str = token.refresh_token.clone().unwrap();
                match auth_handler::refresh_token(
                    &refresh_token_str,
                    &refresh_token_str,
                    token.scope.as_deref(),
                )
                .await
                {
                    Ok(new_token) => {
                        tracing::info!("[auth_flow] Token refreshed successfully");
                        let new_refresh =
                            new_token.refresh_token.clone().unwrap_or(refresh_token_str);
                        self.set_token(
                            new_token.access_token.unwrap(),
                            new_refresh,
                            chrono::Utc::now().timestamp() + new_token.expires_in.unwrap(),
                        );
                        // Send authenticated state
                        state_tx
                            .send(crate::events::message::StateUpdateEnum::AuthStatus(
                                crate::events::message::AuthState::Authenticated,
                            ))
                            .await?;
                    }
                    Err(e) => {
                        tracing::error!("[auth_flow] Token refresh failed: {}", e);
                        // Notify UI that refresh failed, falling back to full auth
                        state_tx
                            .send(crate::events::message::StateUpdateEnum::Error(format!(
                                "Token refresh failed ({}), retrying full auth...",
                                e
                            )))
                            .await
                            .ok();
                        // Refresh failed, do full auth
                        self.authenticate(&state_tx).await?;
                    }
                }
            }
            Ok(token) if token.is_valid() => {
                tracing::warn!(
                    "[auth_flow] Cached token missing required scopes: {:?}. Starting full OAuth flow",
                    token.scope
                );
                state_tx
                    .send(crate::events::message::StateUpdateEnum::Error(
                        "Spotify permissions changed. Please authenticate again to grant the required scopes."
                            .to_string(),
                    ))
                    .await
                    .ok();
                self.authenticate(state_tx).await?;
            }
            _ => {
                tracing::info!("[auth_flow] No valid token found, starting full OAuth flow");
                // No token or invalid, do full auth
                self.authenticate(state_tx).await?;
            }
        }

        Ok(())
    }

    pub async fn authenticate(
        &mut self,
        state_tx: &tokio::sync::mpsc::Sender<crate::events::message::StateUpdateEnum>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("[auth] Starting full OAuth authentication");
        let redirect_uri = "http://127.0.0.1:8888/callback";
        let (code, code_verifier) =
            auth_handler::authorize(&self.client_id, &redirect_uri, state_tx).await?;
        let token =
            auth_handler::get_token(code.as_str(), code_verifier.as_str(), redirect_uri).await?;

        self.set_token(
            token.access_token.ok_or("No access token received")?,
            token.refresh_token.ok_or("No refresh token received")?,
            chrono::Utc::now().timestamp() + token.expires_in.ok_or("No expiry time received")?,
        );

        state_tx
            .send(crate::events::message::StateUpdateEnum::AuthStatus(
                crate::events::message::AuthState::Authenticated,
            ))
            .await?;

        tracing::info!("[auth] OAuth authentication completed successfully");

        Ok(())
    }

    // Helper to handle response status and errors
    fn handle_response_status(
        status: reqwest::StatusCode,
        endpoint: &str,
        body: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !status.is_success() {
            tracing::error!("[api] Request failed: {} - {}", status, body);
            return Err(format!("API error {} for {}: {}", status, endpoint, body).into());
        }
        Ok(())
    }

    // function calls
    // ── Playback Controls ──────────────────────────────────────

    pub async fn play(&self, uri: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let endpoint = "/v1/me/player/play";
        tracing::trace!("[api] → PUT {}{}  body: (empty)", BASE_URL, endpoint);
        let res = self
            .http
            .put(format!("{}{}", BASE_URL, endpoint))
            .bearer_auth(self.token()?)
            .header("Content-Length", "0")
            .body("")
            .send()
            .await?;
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        Self::handle_response_status(status, endpoint, body)?;
        Ok(())
    }

    pub async fn pause(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let endpoint = "/v1/me/player/pause";
        tracing::trace!("[api] → PUT {}{}  body: (empty)", BASE_URL, endpoint);
        let res = self
            .http
            .put(format!("{}{}", BASE_URL, endpoint))
            .bearer_auth(self.token()?)
            .header("Content-Length", "0")
            .body("")
            .send()
            .await?;
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        Self::handle_response_status(status, endpoint, body)?;
        Ok(())
    }

    pub async fn skip_next(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let endpoint = "/v1/me/player/next";
        tracing::trace!("[api] → POST {}{}  body: (empty)", BASE_URL, endpoint);
        let res = self
            .http
            .post(format!("{}{}", BASE_URL, endpoint))
            .bearer_auth(self.token()?)
            .header("Content-Length", "0")
            .body("")
            .send()
            .await?;
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        Self::handle_response_status(status, endpoint, body)?;
        Ok(())
    }

    pub async fn skip_previous(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let endpoint = "/v1/me/player/previous";
        tracing::trace!("[api] → POST {}{}  body: (empty)", BASE_URL, endpoint);
        let res = self
            .http
            .post(format!("{}{}", BASE_URL, endpoint))
            .bearer_auth(self.token()?)
            .header("Content-Length", "0")
            .body("")
            .send()
            .await?;
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        Self::handle_response_status(status, endpoint, body)?;
        Ok(())
    }

    // ── Track Info ─────────────────────────────────────────────

    pub async fn get_current_track(
        &self,
    ) -> Result<Option<crate::events::message::Track>, Box<dyn std::error::Error + Send + Sync>>
    {
        let endpoint = "/v1/me/player/currently-playing";
        tracing::trace!("[api] → GET {}{}", BASE_URL, endpoint);
        let res = self
            .http
            .get(format!("{}{}", BASE_URL, endpoint))
            .bearer_auth(self.token()?)
            .send()
            .await?;
        let status = res.status();

        if status == 204 {
            // get the recently music played
            match self.get_recently_played().await.ok() {
                Some(data) => {
                    tracing::info!("[api] Recently played tracks recieved");
                    if let Some(track) = data.first() {
                        return Ok(Some(track.clone()));
                    }
                }
                None => {
                    tracing::info!("[api] No recently played tracks found");
                    return Ok(None);
                }
            }
        }

        let body = res.text().await.unwrap_or_default();
        Self::handle_response_status(status, endpoint, body.clone())?;

        let data: serde_json::Value = serde_json::from_str(&body)?;

        if let Some(item) = data.get("item") {
            let track = crate::events::message::Track {
                name: item["name"].as_str().unwrap_or("Unknown").to_string(),
                artist: item["artists"][0]["name"]
                    .as_str()
                    .unwrap_or("Unknown")
                    .to_string(),
                album: item["album"]["name"]
                    .as_str()
                    .unwrap_or("Unknown")
                    .to_string(),
                duration_ms: item["duration_ms"].as_u64().unwrap_or(0),
                progress_ms: data["progress_ms"].as_u64().unwrap_or(0),
            };

            Ok(Some(track))
        } else {
            Ok(None)
        }
    }

    // ── Playlists ──────────────────────────────────────────────

    pub async fn get_playlists(
        &self,
    ) -> Result<Vec<crate::events::message::Playlist>, Box<dyn std::error::Error + Send + Sync>>
    {
        let endpoint = "/v1/me/playlists";
        tracing::trace!("[api] → GET {}{}", BASE_URL, endpoint);
        let res = self
            .http
            .get(format!("{}{}", BASE_URL, endpoint))
            .bearer_auth(self.token()?)
            .send()
            .await?;
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        Self::handle_response_status(status, endpoint, body.clone())?;

        let data: serde_json::Value = serde_json::from_str(&body)?;

        let mut playlists = vec![];
        if let Some(items) = data["items"].as_array() {
            for item in items {
                playlists.push(crate::events::message::Playlist {
                    id: item["id"].as_str().unwrap_or("").to_string(),
                    name: item["name"].as_str().unwrap_or("Unknown").to_string(),
                    // Feb 2026: "tracks" renamed to "items", fallback to both
                    track_count: item["items"]["total"]
                        .as_u64()
                        .or_else(|| item["tracks"]["total"].as_u64())
                        .unwrap_or(0),
                });
            }
        }
        Ok(playlists)
    }

    pub async fn get_tracks(
        &self,
        playlist_id: &str,
    ) -> Result<crate::events::message::MusicList, Box<dyn std::error::Error + Send + Sync>> {
        let endpoint = format!("/v1/playlists/{}", playlist_id);
        tracing::trace!("[api] → GET {}{}", BASE_URL, endpoint);

        let res = self
            .http
            .get(format!("{}{}", BASE_URL, endpoint))
            .bearer_auth(self.token()?)
            .send()
            .await?;

        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        Self::handle_response_status(status, &endpoint, body.clone())?;

        tracing::info!("[api] get_tracks response length: {} bytes", body.len());

        let data: serde_json::Value = serde_json::from_str(&body)?;

        let mut items: IndexMap<String, crate::events::message::TrackItem> = IndexMap::new();

        if let Some(items_obj) = data.get("items") {
            if let Some(arr) = items_obj.get("items").and_then(|v| v.as_array()) {
                tracing::info!("[api] Found {} track entries", arr.len());
                for entry in arr {
                    if let Some(track) = entry.get("item") {
                        let id = track["id"].as_str().unwrap_or("").to_string();
                        items.insert(
                            id.clone(),
                            crate::events::message::TrackItem {
                                id,
                                name: track["name"].as_str().unwrap_or("Unknown").to_string(),
                                artist: track["artists"][0]["name"]
                                    .as_str()
                                    .unwrap_or("Unknown")
                                    .to_string(),
                                album: track["album"]["name"]
                                    .as_str()
                                    .unwrap_or("Unknown")
                                    .to_string(),
                                duration_ms: track["duration_ms"].as_u64().unwrap_or(0),
                                uri: track["uri"].as_str().unwrap_or("").to_string(),
                            },
                        );
                    }
                }
            } else {
                tracing::warn!("[api] items.items is not an array");
            }
        } else {
            tracing::warn!("[api] No 'items' field in response");
        }

        tracing::info!("[api] Parsed {} tracks", items.len());

        Ok(crate::events::message::MusicList {
            total: data["items"]["total"].as_u64().unwrap_or(0) as usize,
            next: data["items"]["next"].as_str().map(|s| s.to_string()),
            items,
        })
    }

    // ── Devices ────────────────────────────────────────────────

    pub async fn get_available_devices(
        &self,
    ) -> Result<Vec<crate::events::message::Device>, Box<dyn std::error::Error + Send + Sync>> {
        let endpoint = "/v1/me/player/devices";
        tracing::trace!("[api] → GET {}{}", BASE_URL, endpoint);
        let res = self
            .http
            .get(format!("{}{}", BASE_URL, endpoint))
            .bearer_auth(self.token()?)
            .send()
            .await?;
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        Self::handle_response_status(status, endpoint, body.clone())?;

        let data: serde_json::Value = serde_json::from_str(&body)?;

        let mut devices = vec![];
        if let Some(items) = data["devices"].as_array() {
            for item in items {
                devices.push(crate::events::message::Device {
                    id: item["id"].as_str().unwrap_or("").to_string(),
                    name: item["name"]
                        .as_str()
                        .unwrap_or("Unknown Device")
                        .to_string(),
                    is_active: item["is_active"].as_bool().unwrap_or(false),
                    device_type: item["type"].as_str().unwrap_or("Unknown").to_string(),
                    volume_percent: item["volume_percent"]
                        .as_u64()
                        .and_then(|v| u8::try_from(v).ok()),
                });
            }
        }

        Ok(devices)
    }

    pub async fn change_devices(
        &self,
        device_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        #[derive(serde::Serialize)]
        struct TransferPlaybackPayload {
            device_ids: Vec<String>,
            play: bool,
        }

        let endpoint = "/v1/me/player";
        let payload = TransferPlaybackPayload {
            device_ids: vec![device_id.to_string()],
            play: true,
        };

        let body_preview = format!("{{\"device_ids\":[\"{}\"]}}", device_id);
        tracing::trace!(
            "[api] → PUT {}{}  body: {}",
            BASE_URL,
            endpoint,
            body_preview
        );
        let res = self
            .http
            .put(format!("{}{}", BASE_URL, endpoint))
            .bearer_auth(self.token()?)
            .json(&payload)
            .send()
            .await?;
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        Self::handle_response_status(status, endpoint, body)?;
        Ok(())
    }

    // ── Search ─────────────────────────────────────────────────

    pub async fn search(
        &self,
        query: &str,
    ) -> Result<Vec<crate::events::message::Track>, Box<dyn std::error::Error + Send + Sync>> {
        let encoded = urlencoding::encode(query);
        // Feb 2026: dev mode max limit is 10, paginate with offset for more
        let endpoint = format!("/v1/search?q={}&type=track&limit=10", encoded);
        tracing::trace!("[api] → GET {}{}", BASE_URL, endpoint);
        let res = self
            .http
            .get(format!("{}{}", BASE_URL, &endpoint))
            .bearer_auth(self.token()?)
            .send()
            .await?;
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        Self::handle_response_status(status, &endpoint, body.clone())?;

        let data: serde_json::Value = serde_json::from_str(&body)?;

        let mut tracks = vec![];
        if let Some(items) = data["tracks"]["items"].as_array() {
            for item in items {
                tracks.push(crate::events::message::Track {
                    name: item["name"].as_str().unwrap_or("Unknown").to_string(),
                    artist: item["artists"][0]["name"]
                        .as_str()
                        .unwrap_or("Unknown")
                        .to_string(),
                    album: item["album"]["name"]
                        .as_str()
                        .unwrap_or("Unknown")
                        .to_string(),
                    duration_ms: item["duration_ms"].as_u64().unwrap_or(0),
                    progress_ms: 0,
                });
            }
        }
        Ok(tracks)
    }

    pub async fn me(
        &self,
    ) -> Result<crate::events::message::UserProfile, Box<dyn std::error::Error + Send + Sync>> {
        let endpoint = "/v1/me";
        tracing::trace!("[api] → GET {}{}", BASE_URL, endpoint);
        let res = self
            .http
            .get(format!("{}{}", BASE_URL, endpoint))
            .bearer_auth(self.token()?)
            .send()
            .await?;
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        Self::handle_response_status(status, endpoint, body.clone())?;

        let data: serde_json::Value = serde_json::from_str(&body)?;

        // Feb 2026: email, country, product, followers removed from dev mode
        let profile = crate::events::message::UserProfile {
            id: data["id"].as_str().unwrap_or("").to_string(),
            display_name: data["display_name"].as_str().map(|s| s.to_string()),
            email: None,   // removed in Feb 2026 dev mode
            country: None, // removed in Feb 2026 dev mode
            product: None, // removed in Feb 2026 dev mode
            followers: 0,  // removed in Feb 2026 dev mode
            profile_image_url: data["images"]
                .as_array()
                .and_then(|imgs| imgs.first())
                .and_then(|img| img["url"].as_str())
                .map(|s| s.to_string()),
            uri: data["uri"].as_str().unwrap_or("").to_string(),
        };

        Ok(profile)
    }

    // ── Recently Played ────────────────────────────────────────

    pub async fn get_recently_played(
        &self,
    ) -> Result<Vec<crate::events::message::Track>, Box<dyn std::error::Error + Send + Sync>> {
        let endpoint = "/v1/me/player/recently-played";
        tracing::trace!("[api] → GET {}{}", BASE_URL, endpoint);
        let res = self
            .http
            .get(format!("{}{}", BASE_URL, endpoint))
            .bearer_auth(self.token()?)
            .send()
            .await?;
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        if status == reqwest::StatusCode::FORBIDDEN {
            tracing::warn!(
                "[api] GET {} -> 403 Forbidden (missing scope: user-read-recently-played). Re-authenticate to grant access.",
                endpoint
            );
            return Ok(vec![]);
        }
        Self::handle_response_status(status, endpoint, body.clone())?;

        let data: serde_json::Value = serde_json::from_str(&body)?;

        let mut tracks = vec![];
        if let Some(items) = data["items"].as_array() {
            for item in items {
                if let Some(track) = item.get("track") {
                    tracks.push(crate::events::message::Track {
                        name: track["name"].as_str().unwrap_or("Unknown").to_string(),
                        artist: track["artists"][0]["name"]
                            .as_str()
                            .unwrap_or("Unknown")
                            .to_string(),
                        album: track["album"]["name"]
                            .as_str()
                            .unwrap_or("Unknown")
                            .to_string(),
                        duration_ms: track["duration_ms"].as_u64().unwrap_or(0),
                        progress_ms: 0,
                    });
                }
            }
        }

        Ok(tracks)
    }
}
