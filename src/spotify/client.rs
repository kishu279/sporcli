use crate::spotify::auth_handler;
use crate::storage;

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
            Ok(token) if token.is_valid() && !token.is_expired() => {
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
            Ok(token) if token.is_valid() => {
                tracing::warn!(
                    "[auth_flow] Token expired (expires_at: {:?}), attempting refresh",
                    token.expires_at
                );
                // Token expired but valid, try refresh
                let refresh_token_str = token.refresh_token.clone().unwrap();
                match auth_handler::refresh_token(&refresh_token_str, &refresh_token_str).await {
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

    // methods
    async fn api_get(
        &self,
        endpoint: &str,
    ) -> Result<reqwest::Response, Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!("[api] GET {}", endpoint);
        let res = self
            .http
            .get(format!("{}{}", BASE_URL, endpoint))
            .bearer_auth(self.token()?)
            .send()
            .await?;
        let status = res.status();
        tracing::debug!("[api] GET {} -> status: {}", endpoint, status);
        if !status.is_success() {
            let body = res.text().await.unwrap_or_default();
            tracing::error!("[api] GET {} failed: {} - {}", endpoint, status, body);
            return Err(format!("API error {} for {}: {}", status, endpoint, body).into());
        }
        Ok(res)
    }

    async fn api_put(
        &self,
        endpoint: &str,
    ) -> Result<reqwest::Response, Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!("[api] PUT {}", endpoint);
        let res = self
            .http
            .put(format!("{}{}", BASE_URL, endpoint))
            .bearer_auth(self.token()?)
            .header("Content-Length", "0")
            .body("")
            .send()
            .await?;
        let status = res.status();
        tracing::debug!("[api] PUT {} -> status: {}", endpoint, status);
        if !status.is_success() {
            let body = res.text().await.unwrap_or_default();
            tracing::error!("[api] PUT {} failed: {} - {}", endpoint, status, body);
            return Err(format!("API error {} for {}: {}", status, endpoint, body).into());
        }
        Ok(res)
    }

    async fn api_post(
        &self,
        endpoint: &str,
    ) -> Result<reqwest::Response, Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!("[api] POST {}", endpoint);
        let res = self
            .http
            .post(format!("{}{}", BASE_URL, endpoint))
            .bearer_auth(self.token()?)
            .header("Content-Length", "0")
            .send()
            .await?;
        let status = res.status();
        tracing::debug!("[api] POST {} -> status: {}", endpoint, status);
        if !status.is_success() {
            let body = res.text().await.unwrap_or_default();
            tracing::error!("[api] POST {} failed: {} - {}", endpoint, status, body);
            return Err(format!("API error {} for {}: {}", status, endpoint, body).into());
        }
        Ok(res)
    }

    // function calls
    // ── Playback Controls ──────────────────────────────────────

    pub async fn play(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::trace!("[api] playback play");
        self.api_put("/v1/me/player/play").await?;
        Ok(())
    }

    pub async fn pause(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::trace!("[api] playback pause");
        self.api_put("/v1/me/player/pause").await?;
        Ok(())
    }

    pub async fn skip_next(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::trace!("[api] playback next");
        self.api_post("/v1/me/player/next").await?;
        Ok(())
    }

    pub async fn skip_previous(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::trace!("[api] playback previous");
        self.api_post("/v1/me/player/previous").await?;
        Ok(())
    }

    // ── Track Info ─────────────────────────────────────────────

    pub async fn get_current_track(
        &self,
    ) -> Result<Option<crate::events::message::Track>, Box<dyn std::error::Error + Send + Sync>>
    {
        tracing::trace!("[api] get_current_track");
        let response = self.api_get("/v1/me/player/currently-playing").await?;

        if response.status() == 204 {
            return Ok(None);
        }

        let data: serde_json::Value = response.json().await?;

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
        tracing::trace!("[api] get_playlists");
        let response = self.api_get("/v1/me/playlists").await?;
        let data: serde_json::Value = response.json().await?;

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

    // ── Liked Songs ────────────────────────────────────────────
    // NOTE: GET /v1/me/tracks removed in Feb 2026 dev mode migration.
    // Use GET /me/library/contains or extended quota mode to restore this.
    pub async fn get_liked_songs(
        &self,
    ) -> Result<Vec<crate::events::message::Track>, Box<dyn std::error::Error + Send + Sync>> {
        tracing::warn!("[api] get_liked_songs: /v1/me/tracks removed in Feb 2026 dev mode");
        Err("Liked songs endpoint removed in dev mode (Feb 2026 migration)".into())
    }

    // ── Search ─────────────────────────────────────────────────

    pub async fn search(
        &self,
        query: &str,
    ) -> Result<Vec<crate::events::message::Track>, Box<dyn std::error::Error + Send + Sync>> {
        tracing::trace!("[api] search query='{}'", query);
        let encoded = urlencoding::encode(query);
        // Feb 2026: dev mode max limit is 10, paginate with offset for more
        let endpoint = format!("/v1/search?q={}&type=track&limit=10", encoded);
        let response = self.api_get(&endpoint).await?;
        let data: serde_json::Value = response.json().await?;

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
        tracing::trace!("[api] get profile /v1/me");
        let response = self.api_get("/v1/me").await?;
        let data: serde_json::Value = response.json().await?;

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
}
