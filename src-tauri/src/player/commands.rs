use tauri::{AppHandle, Manager, State};

use super::{
    PlayerState,
    aniskip::{self, SkipTimes},
    store::{PlayerData, Progress, Settings, TrackPref},
};
use crate::nuvem::download::sanitize_file_name;

/// URL que o mpv deve abrir: o arquivo já baixado (mesmo tamanho) ou o proxy de stream.
#[tauri::command]
pub fn player_stream_url(app: AppHandle, state: State<'_, PlayerState>, path: String, size: u64) -> String {
    let name = path.rsplit('/').next().unwrap_or(&path);
    if let Ok(dir) = app.path().download_dir() {
        let local = dir.join(sanitize_file_name(name));
        if std::fs::metadata(&local).is_ok_and(|m| m.is_file() && m.len() == size) {
            return local.to_string_lossy().into_owned();
        }
    }
    state.proxy.url_for(&path)
}

#[tauri::command]
pub fn player_data(state: State<'_, PlayerState>) -> PlayerData {
    state.store.data()
}

#[tauri::command]
pub fn player_save_progress(state: State<'_, PlayerState>, path: String, progress: Progress) {
    state.store.update(|d| {
        d.progress.insert(path, progress);
    });
}

/// Faixas escolhidas, lembradas por pasta de release.
#[tauri::command]
pub fn player_save_tracks(state: State<'_, PlayerState>, release: String, pref: TrackPref) {
    state.store.update(|d| {
        d.tracks.insert(release, pref);
    });
}

#[tauri::command]
pub fn player_save_settings(state: State<'_, PlayerState>, settings: Settings) {
    state.store.update(|d| d.settings = settings);
}

/// Tempos de abertura/encerramento no AniSkip, com cache (inclusive "não encontrado").
/// Falha de rede devolve vazio e não é guardada.
#[tauri::command]
pub async fn player_aniskip(state: State<'_, PlayerState>, mal_id: u32, episode: u32, duration: f64) -> Result<SkipTimes, ()> {
    let key = format!("{mal_id}:{episode}");
    if let Some(hit) = state.aniskip.get(&key) {
        return Ok(hit);
    }
    let Ok(resp) = state.http.get(aniskip::url(mal_id, episode, duration)).send().await else {
        return Ok(SkipTimes::default());
    };
    let status = resp.status().as_u16();
    let body = resp.text().await.unwrap_or_default();
    match aniskip::parse_response(status, &body) {
        Some(times) => {
            state.aniskip.insert(key, times.clone());
            state.aniskip.flush();
            Ok(times)
        }
        None => Ok(SkipTimes::default()),
    }
}
