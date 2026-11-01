use tauri::{AppHandle, Manager, State};

use super::PlayerState;
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
