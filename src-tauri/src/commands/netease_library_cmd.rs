use serde_json::Value;
use std::collections::HashMap;
use tauri::State;

use crate::error::AppResult;
use crate::state::AppState;

fn current_netease_user_id(state: &AppState) -> AppResult<u64> {
    state
        .auth
        .lock()
        .netease
        .as_ref()
        .and_then(|auth| auth.user_id)
        .ok_or_else(|| crate::error::AppError::Api("Netease not logged in".into()))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn netease_get_recent_play_songs(
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> AppResult<Value> {
    state
        .netease()
        .get_recent_play_songs(limit.unwrap_or(100))
        .await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn netease_get_user_record(
    period: String,
    state: State<'_, AppState>,
) -> AppResult<Value> {
    let uid = current_netease_user_id(state.inner())?;
    let record_type = match period.as_str() {
        "week" => 1,
        "all" => 0,
        _ => return Err(crate::error::AppError::Api("Invalid ranking period".into())),
    };
    state.netease().get_user_record(uid, record_type).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn netease_report_playback(
    song_id: u64,
    total_time_ms: u64,
    played_ms: u64,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let cookies: HashMap<String, String> = state
        .auth
        .lock()
        .netease
        .as_ref()
        .filter(|auth| auth.has_login())
        .ok_or_else(|| crate::error::AppError::Api("Netease not logged in".into()))?
        .cookies
        .iter()
        .map(|cookie| (cookie.name.clone(), cookie.value.clone()))
        .collect();
    let song = crate::api::netease::scrobble::ScrobbleSong {
        id: song_id,
        total_time_ms,
        played_ms,
    };
    crate::api::netease::scrobble::report(&state.http(), &cookies, &song).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn netease_subscribe_playlist(
    playlist_id: u64,
    state: State<'_, AppState>,
) -> AppResult<Value> {
    state.netease().subscribe_playlist(playlist_id).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn netease_unsubscribe_playlist(
    playlist_id: u64,
    state: State<'_, AppState>,
) -> AppResult<Value> {
    state.netease().unsubscribe_playlist(playlist_id).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn netease_follow_artist(artist_id: u64, state: State<'_, AppState>) -> AppResult<Value> {
    state.netease().follow_artist(artist_id).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn netease_unfollow_artist(
    artist_id: u64,
    state: State<'_, AppState>,
) -> AppResult<Value> {
    state.netease().unfollow_artist(artist_id).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn netease_get_followed_artists(
    offset: Option<u32>,
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> AppResult<Value> {
    state
        .netease()
        .get_followed_artists(offset.unwrap_or(0), limit.unwrap_or(1000))
        .await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn netease_add_songs_to_playlist(
    playlist_id: u64,
    song_ids: Vec<u64>,
    state: State<'_, AppState>,
) -> AppResult<Value> {
    state
        .netease()
        .add_songs_to_playlist(playlist_id, &song_ids)
        .await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn netease_remove_songs_from_playlist(
    playlist_id: u64,
    song_ids: Vec<u64>,
    state: State<'_, AppState>,
) -> AppResult<Value> {
    state
        .netease()
        .remove_songs_from_playlist(playlist_id, &song_ids)
        .await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn netease_create_playlist(name: String, state: State<'_, AppState>) -> AppResult<Value> {
    state.netease().create_cloud_playlist(&name).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn netease_delete_playlist(
    playlist_id: u64,
    state: State<'_, AppState>,
) -> AppResult<Value> {
    state.netease().delete_cloud_playlist(playlist_id).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn netease_rename_playlist(
    playlist_id: u64,
    name: String,
    state: State<'_, AppState>,
) -> AppResult<Value> {
    state
        .netease()
        .rename_cloud_playlist(playlist_id, &name)
        .await
}
