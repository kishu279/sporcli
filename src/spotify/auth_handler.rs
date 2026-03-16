use crate::events::message::StateUpdateEnum;
use crate::storage;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

pub const REQUIRED_SCOPES: &[&str] = &[
    "playlist-read-private",
    "user-library-read",
    "user-read-currently-playing",
    "user-modify-playback-state",
    "user-read-playback-state",
    "user-read-recently-played",
];

pub fn has_required_scopes(scope: Option<&str>) -> bool {
    let granted_scopes = scope
        .unwrap_or_default()
        .split_whitespace()
        .collect::<std::collections::HashSet<_>>();

    REQUIRED_SCOPES
        .iter()
        .all(|required_scope| granted_scopes.contains(required_scope))
}

// generate the pkce pair which to be verirified by the server
pub fn generate_pkce_pair() -> (String, String) {
    use rand::distributions::{Alphanumeric, DistString};

    // 1. Generate a verifier using ONLY safe characters
    // Alphanumeric is safe, then we manually add the other PKCE-safe chars if desired,
    // but alphanumeric alone is perfectly valid and high-entropy.
    let verifier = Alphanumeric.sample_string(&mut rand::thread_rng(), 64);

    // 2. Hash it
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();

    // 3. Encode the challenge as URL-SAFE and NO-PADDING
    let challenge = URL_SAFE_NO_PAD.encode(hash);

    (verifier, challenge)
}

pub async fn authorize(
    client_id: &str,
    redirect_uri: &str,
    state_tx: &tokio::sync::mpsc::Sender<StateUpdateEnum>,
) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("[authorize] Generating PKCE pair and auth URL");
    let (code_verifier, code_challenge) = generate_pkce_pair();

    let scopes = REQUIRED_SCOPES.join("%20");

    let auth_url = format!(
        "https://accounts.spotify.com/authorize?client_id={}&response_type=code&redirect_uri={}&code_challenge_method=S256&code_challenge={}&scope={}",
        client_id, redirect_uri, code_challenge, scopes
    );

    // println!("Open: {}", auth_url);
    state_tx
        .send(StateUpdateEnum::AuthStatus(
            crate::events::message::AuthState::Authenticating {
                url: auth_url.clone(),
            },
        ))
        .await?;

    // update the app state
    state_tx.send(StateUpdateEnum::CopyUrl(auth_url)).await?;

    tracing::info!("[authorize] Binding to 127.0.0.1:8888, waiting for OAuth callback...");
    let listener = TcpListener::bind("127.0.0.1:8888")?;
    let (mut stream, _) = listener.accept()?;
    tracing::info!("[authorize] Callback received");

    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    let code = request_line
        .split_whitespace()
        .nth(1)
        .and_then(|path| path.split("code=").nth(1))
        .and_then(|code| code.split('&').next())
        .ok_or_else(|| format!("No authorization code found in request: {}", request_line))?;

    stream.write_all(b"HTTP/1.1 200 OK\r\n\r\nAuthenticated!")?;

    Ok((code.to_string(), code_verifier))
}

// GETTING TOKEN FROM THE SPOTIFY
use serde::{Deserialize, Serialize};

use reqwest::{
    Client,
    header::{CONTENT_TYPE, HeaderMap, HeaderValue},
};

#[derive(Serialize)]
struct PayloadGetToken {
    client_id: String,
    code: String,
    redirect_uri: String,
    grant_type: String,
    code_verifier: String,
}

#[derive(Serialize)]
struct PayloadGetRefreshToken {
    client_id: String,
    grant_type: String,
    refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: Option<String>,
    pub expires_in: Option<i64>,
    pub scope: Option<String>,
    pub refresh_token: Option<String>,
    pub token_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StoredToken {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>, // unix timestamp
    pub token_type: Option<String>,
    pub scope: Option<String>,
}

impl StoredToken {
    pub fn is_valid(&self) -> bool {
        self.access_token.is_some() && self.refresh_token.is_some() && self.expires_at.is_some()
    }

    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(exp) => exp < chrono::Utc::now().timestamp(),
            None => true,
        }
    }
}

// GET THE SPOTIFY AUTH TOKEN FROM THE MOBILE
pub async fn get_token(
    code: &str,
    code_verifier: &str,
    redirect_uri: &str,
) -> Result<TokenResponse, Box<dyn std::error::Error + Send + Sync>> {
    let mut headers = HeaderMap::new();

    let url = "https://accounts.spotify.com/api/token";

    // 'Content-Type': 'application/x-www-form-urlencoded',
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_str("application/x-www-form-urlencoded")?,
    );

    let payload = PayloadGetToken {
        client_id: "970f2de2fa8141108ea3fbd3c9498985".into(), // always same for this app
        grant_type: "authorization_code".into(),
        code: code.into(),
        redirect_uri: redirect_uri.into(),
        code_verifier: code_verifier.into(),
    };

    let client = Client::new();

    tracing::info!("[get_token] Requesting token from Spotify");
    let res = client
        .post(url)
        .headers(headers)
        .form(&payload)
        .send()
        .await?;

    tracing::info!("[get_token] Response status: {}", res.status());
    if res.status().is_success() {
        let body = res.json::<TokenResponse>().await?;
        tracing::info!("[get_token] Token received, scope: {:?}", body.scope);

        // Convert expires_in (seconds) → absolute timestamp
        let stored_token = StoredToken {
            access_token: body.access_token.clone(),
            refresh_token: body.refresh_token.clone(),
            expires_at: Some(chrono::Utc::now().timestamp() + body.expires_in.unwrap()),
            token_type: body.token_type.clone(),
            scope: body.scope.clone(),
        };

        // save the token
        storage::save_credentials(&stored_token)?;
        tracing::info!("[get_token] Token saved to disk");

        return Ok(body);
    }

    // Request failed - get error details
    let status = res.status();
    let error_body = res
        .text()
        .await
        .unwrap_or_else(|_| "Unable to read error".to_string());
    tracing::error!(
        "[get_token] Token request failed: {} - {}",
        status,
        error_body
    );
    Err(format!(
        "Token request failed with status {}: {}",
        status, error_body
    )
    .into())
}

// GET THE REFRESHED TOKEN FROM THE SPOTIFY CLI
pub async fn refresh_token(
    refresh_token: &str,
    old_refresh_token: &str,
    old_scope: Option<&str>,
) -> Result<TokenResponse, Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("[refresh_token] Requesting refreshed token from Spotify");
    let mut headers = HeaderMap::new();
    let url = "https://accounts.spotify.com/api/token";

    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_str("application/x-www-form-urlencoded")?,
    );

    let payload = PayloadGetRefreshToken {
        client_id: "970f2de2fa8141108ea3fbd3c9498985".into(),
        grant_type: "refresh_token".into(),
        refresh_token: refresh_token.into(),
    };

    let client = Client::new();
    let res = client.post(url).form(&payload).send().await?;
    tracing::info!("[refresh_token] Response status: {}", res.status());

    if res.status().is_success() {
        let body = res.json::<TokenResponse>().await?;

        let stored_token = StoredToken {
            access_token: body.access_token.clone(),
            refresh_token: body
                .refresh_token
                .clone()
                .or(Some(old_refresh_token.to_string())),
            expires_at: Some(chrono::Utc::now().timestamp() + body.expires_in.unwrap()),
            token_type: body.token_type.clone(),
            scope: body.scope.clone().or_else(|| old_scope.map(str::to_string)),
        };

        storage::save_credentials(&stored_token)?;
        tracing::info!("[refresh_token] Refreshed token saved to disk");

        return Ok(body);
    }

    // on failure
    tracing::error!("[refresh_token] Refresh token request failed");
    Err("Refresh token failed".into())
}
