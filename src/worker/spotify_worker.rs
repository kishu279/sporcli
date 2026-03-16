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
                tracing::info!("[worker] Action: Authenticate");
                if let Err(e) = spotify_client.authenticate_flow(&state_tx).await {
                    tracing::error!("[worker] Authenticate failed: {}", e);
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
                tracing::info!("[worker] Action: Play");
                if let Err(e) = spotify_client.play().await {
                    tracing::error!("[worker] Play failed: {}", e);
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
                tracing::info!("[worker] Action: Pause");
                if let Err(e) = spotify_client.pause().await {
                    tracing::error!("[worker] Pause failed: {}", e);
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
            Some(Action::Quit) => {
                tracing::info!("[worker] Action: Quit");
                break Ok(());
            }
            Some(Action::NextTrack) => {
                tracing::info!("[worker] Action: NextTrack");
                if let Err(e) = spotify_client.skip_next().await {
                    tracing::error!("[worker] NextTrack failed: {}", e);
                    state_tx
                        .send(StateUpdateEnum::Error(format!("Skip next failed: {}", e)))
                        .await
                        .ok();
                } else {
                    if let Ok(Some(track)) = spotify_client.get_current_track().await {
                        state_tx.send(StateUpdateEnum::TrackInfo(track)).await.ok();
                    }
                    state_tx
                        .send(StateUpdateEnum::PlaybackStatus(true))
                        .await
                        .ok();
                }
            }
            Some(Action::PreviousTrack) => {
                tracing::info!("[worker] Action: PreviousTrack");
                if let Err(e) = spotify_client.skip_previous().await {
                    tracing::error!("[worker] PreviousTrack failed: {}", e);
                    state_tx
                        .send(StateUpdateEnum::Error(format!(
                            "Skip previous failed: {}",
                            e
                        )))
                        .await
                        .ok();
                } else {
                    if let Ok(Some(track)) = spotify_client.get_current_track().await {
                        state_tx.send(StateUpdateEnum::TrackInfo(track)).await.ok();
                    }
                    state_tx
                        .send(StateUpdateEnum::PlaybackStatus(true))
                        .await
                        .ok();
                }
            }
            Some(Action::GetCurrentTrack) => {
                tracing::info!("[worker] Action: GetCurrentTrack");
                match spotify_client.get_current_track().await {
                    Ok(Some(track)) => {
                        state_tx.send(StateUpdateEnum::TrackInfo(track)).await.ok();
                    }
                    Ok(None) => {
                        // Not sending this error / we can send not playing enum update
                        state_tx
                            .send(StateUpdateEnum::Error(
                                "No track currently playing".to_string(),
                            ))
                            .await
                            .ok();

                        state_tx
                            .send(StateUpdateEnum::PlaybackStatus(false))
                            .await
                            .ok();
                    }
                    Err(e) => {
                        tracing::error!("[worker] GetCurrentTrack failed: {}", e);
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
            Some(Action::GetDevices) => {
                tracing::info!("[worker] Action: GetDevices");
                match spotify_client.get_available_devices().await {
                    Ok(devices) => {
                        tracing::info!("[worker] Devices received: {}", devices.len());
                        state_tx.send(StateUpdateEnum::Devices(devices)).await.ok();
                    }
                    Err(e) => {
                        tracing::error!("[worker] GetDevices failed: {}", e);
                        state_tx
                            .send(StateUpdateEnum::Error(format!(
                                "Failed to get devices: {}",
                                e
                            )))
                            .await
                            .ok();
                    }
                }
            }
            Some(Action::ChangeDevice(device_id)) => {
                tracing::info!("[worker] Action: ChangeDevice -> {}", device_id);
                match spotify_client.change_devices(&device_id).await {
                    Ok(()) => {
                        state_tx
                            .send(StateUpdateEnum::PlaybackStatus(true))
                            .await
                            .ok();
                        if let Ok(devices) = spotify_client.get_available_devices().await {
                            state_tx.send(StateUpdateEnum::Devices(devices)).await.ok();
                        }
                    }
                    Err(e) => {
                        tracing::error!("[worker] ChangeDevice failed: {}", e);
                        state_tx
                            .send(StateUpdateEnum::Error(format!(
                                "Failed to change device: {}",
                                e
                            )))
                            .await
                            .ok();
                    }
                }
            }
            Some(Action::GetPlaylists) => {
                tracing::info!("[worker] Action: GetPlaylists");
                match spotify_client.get_playlists().await {
                    Ok(playlists) => {
                        state_tx
                            .send(StateUpdateEnum::Playlists(playlists))
                            .await
                            .ok();
                    }
                    Err(e) => {
                        tracing::error!("[worker] GetPlaylists failed: {}", e);
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
                tracing::warn!("[worker] GetLikedSongs skipped - endpoint removed in Feb 2026");
            }
            Some(Action::Search(query)) => {
                tracing::info!("[worker] Action: Search query='{}'", query);
                match spotify_client.search(&query).await {
                    Ok(tracks) => {
                        tracing::info!("[worker] Search returned {} tracks", tracks.len());
                        state_tx.send(StateUpdateEnum::TrackList(tracks)).await.ok();
                    }
                    Err(e) => {
                        tracing::error!("[worker] Search failed: {}", e);
                        state_tx
                            .send(StateUpdateEnum::Error(format!("Search failed: {}", e)))
                            .await
                            .ok();
                    }
                }
            }
            Some(Action::GetProfile) => {
                tracing::info!("[worker] Action: GetProfile");
                match spotify_client.me().await {
                    Ok(profile) => {
                        tracing::info!(
                            "[worker] Profile loaded: {:?}",
                            profile.display_name.as_deref().unwrap_or("unknown")
                        );
                        state_tx
                            .send(StateUpdateEnum::UserProfile(profile))
                            .await
                            .ok();
                    }
                    Err(e) => {
                        tracing::error!("[worker] GetProfile failed: {}", e);
                        state_tx
                            .send(StateUpdateEnum::Error(format!(
                                "Failed to get profile: {}",
                                e
                            )))
                            .await
                            .ok();
                    }
                }
            }
            Some(other) => {
                tracing::debug!("[worker] Unhandled action: {:?}", other);
            }
            None => {
                tracing::warn!("[worker] action channel closed");
            }
        }
    }
}
