use crate::events::message::{Action, StateUpdateEnum};
use crate::spotify::SpotifyClient;

pub async fn spotify_worker(
    mut action_rx: tokio::sync::mpsc::Receiver<Action>,
    state_tx: tokio::sync::mpsc::Sender<StateUpdateEnum>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut spotify_client = SpotifyClient::new();

    loop {
        match action_rx.recv().await {
            Some(Action::Authenticate) => {
                // tracing::info!("[worker] Action: Authenticate");
                if let Err(e) = spotify_client.authenticate_flow(&state_tx).await {
                    state_tx
                        .send(StateUpdateEnum::AuthStatus(
                            crate::events::message::AuthState::Error(format!(
                                "Authentication failed: {}",
                                e
                            )),
                        ))
                        .await
                        .ok();
                }
            }
            Some(Action::Play) => {
                if let Err(e) = spotify_client.play().await {
                    state_tx
                        .send(StateUpdateEnum::Error(format!("Play failed: {}", e)))
                        .await
                        .ok();
                } else {
                    state_tx
                        .send(StateUpdateEnum::PlaybackStatus(true))
                        .await
                        .ok();
                }
            }
            Some(Action::Pause) => {
                if let Err(e) = spotify_client.pause().await {
                    state_tx
                        .send(StateUpdateEnum::Error(format!("Pause failed: {}", e)))
                        .await
                        .ok();
                } else {
                    state_tx
                        .send(StateUpdateEnum::PlaybackStatus(false))
                        .await
                        .ok();
                }
            }
            Some(Action::Quit) => break Ok(()),
            Some(Action::NextTrack) => {
                if let Err(e) = spotify_client.skip_next().await {
                    state_tx
                        .send(StateUpdateEnum::Error(format!("Skip next failed: {}", e)))
                        .await
                        .ok();
                }
            }
            Some(Action::PreviousTrack) => {
                if let Err(e) = spotify_client.skip_previous().await {
                    state_tx
                        .send(StateUpdateEnum::Error(format!(
                            "Skip previous failed: {}",
                            e
                        )))
                        .await
                        .ok();
                }
            }
            Some(Action::GetCurrentTrack) => {
                // tracing::info!("[worker] Action: GetCurrentTrack");
                match spotify_client.get_current_track().await {
                    Ok(Some(track)) => {
                        state_tx.send(StateUpdateEnum::TrackInfo(track)).await.ok();
                    }
                    Ok(None) => {
                        state_tx
                            .send(StateUpdateEnum::Error(
                                "No track currently playing".to_string(),
                            ))
                            .await
                            .ok();
                    }
                    Err(e) => {
                        // tracing::error!("[worker] GetCurrentTrack failed: {}", e);
                        state_tx
                            .send(StateUpdateEnum::Error(format!(
                                "Failed to get track: {}",
                                e
                            )))
                            .await
                            .ok();
                    }
                }
            }
            Some(Action::GetPlaylists) => {
                // tracing::info!("[worker] Action: GetPlaylists");
                match spotify_client.get_playlists().await {
                    Ok(playlists) => {
                        state_tx
                            .send(StateUpdateEnum::Playlists(playlists))
                            .await
                            .ok();
                    }
                    Err(e) => {
                        // tracing::error!("[worker] GetPlaylists failed: {}", e);
                        state_tx
                            .send(StateUpdateEnum::Error(format!(
                                "Failed to get playlists: {}",
                                e
                            )))
                            .await
                            .ok();
                    }
                }
            }
            Some(Action::GetLikedSongs) => {
                // Feb 2026: GET /v1/me/tracks removed in dev mode
                // tracing::warn!("[worker] GetLikedSongs skipped - endpoint removed in Feb 2026");
            }
            Some(Action::Search(query)) => match spotify_client.search(&query).await {
                Ok(tracks) => {
                    state_tx.send(StateUpdateEnum::TrackList(tracks)).await.ok();
                }
                Err(e) => {
                    state_tx
                        .send(StateUpdateEnum::Error(format!("Search failed: {}", e)))
                        .await
                        .ok();
                }
            },
            Some(Action::GetProfile) => match spotify_client.me().await {
                Ok(profile) => {
                    state_tx
                        .send(StateUpdateEnum::UserProfile(profile))
                        .await
                        .ok();
                }
                Err(e) => {
                    state_tx
                        .send(StateUpdateEnum::Error(format!(
                            "Failed to get profile: {}",
                            e
                        )))
                        .await
                        .ok();
                }
            },
            Some(_) => {}
            None => {}
        }
    }
}
