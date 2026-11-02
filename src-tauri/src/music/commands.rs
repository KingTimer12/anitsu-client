//! Comandos do player chamados pelo front. O JS só manda caminho e tamanho;
//! quem fala com a nuvem é o Rust.

use std::{collections::HashMap, path::PathBuf, sync::Mutex, time::Duration};

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use super::{
    decode::{needed_prefix, probe_prefix},
    engine::{Command, Engine},
    queue::{QueueTrack, Repeat},
};

/// Bytes lidos do começo de cada faixa para tags e duração.
const META_PREFIX: u64 = 256 * 1024;
const META_RETRIES: u32 = 5;
/// Teto para tags com capa embutida.
const MAX_META_PREFIX: u64 = 16 * 1024 * 1024;

pub struct MusicState {
    pub engine: Engine,
    meta: Mutex<HashMap<String, TrackMeta>>,
    meta_path: Option<PathBuf>,
}

impl MusicState {
    pub fn new(engine: Engine, meta_path: Option<PathBuf>) -> Self {
        let meta = meta_path
            .as_deref()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Self { engine, meta: Mutex::new(meta), meta_path }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackMeta {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub duration: Option<f64>,
    pub codec: String,
    pub bits: Option<u32>,
    pub sample_rate: u32,
    pub lossless: bool,
    /// Capa embutida como data URL (só quando pedida; não vai para o cache).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
}

pub fn parse_cmd(s: &str) -> Option<Command> {
    Some(match s {
        "pause" => Command::Pause,
        "resume" => Command::Resume,
        "toggle" => Command::Toggle,
        "next" => Command::Next,
        "prev" => Command::Prev,
        "stop" => Command::Stop,
        _ => return None,
    })
}

/// Último byte do prefixo lido (inclusivo).
pub fn prefix_end(size: u64) -> u64 {
    META_PREFIX.min(size).saturating_sub(1)
}

/// Trecho da nuvem, esperando e tentando de novo em 429.
async fn fetch_retry(app: &AppHandle, path: &str, start: u64, end: u64) -> Result<Vec<u8>, String> {
    let mut tries = 0;
    loop {
        match crate::reader::fetch_range(app, path, start, end).await {
            Ok(b) => return Ok(b),
            Err(e) if e.contains("429") && tries < META_RETRIES => {
                tries += 1;
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            Err(e) => return Err(e),
        }
    }
}

#[tauri::command]
pub fn music_play(state: State<'_, MusicState>, album: String, tracks: Vec<QueueTrack>, index: usize, at: f64) {
    state.engine.send(Command::Play { album, tracks, index, at });
}

#[tauri::command]
pub fn music_command(state: State<'_, MusicState>, cmd: String) -> Result<(), String> {
    state.engine.send(parse_cmd(&cmd).ok_or_else(|| format!("comando desconhecido: {cmd}"))?);
    Ok(())
}

#[tauri::command]
pub fn music_seek(state: State<'_, MusicState>, secs: f64) {
    state.engine.send(Command::Seek(secs));
}

#[tauri::command]
pub fn music_jump(state: State<'_, MusicState>, index: usize) {
    state.engine.send(Command::Jump(index));
}

#[tauri::command]
pub fn music_set(state: State<'_, MusicState>, volume: Option<f32>, shuffle: Option<bool>, repeat: Option<Repeat>, normalize: Option<bool>) {
    let e = &state.engine;
    if let Some(v) = volume {
        e.send(Command::Volume(v));
    }
    if let Some(s) = shuffle {
        e.send(Command::Shuffle(s));
    }
    if let Some(r) = repeat {
        e.send(Command::Repeat(r));
    }
    if let Some(n) = normalize {
        e.send(Command::Normalize(n));
    }
}

/// Tags e duração de uma faixa, lendo só o começo dela (com cache em disco).
#[tauri::command]
pub async fn music_meta(app: AppHandle, path: String, size: u64, ext: String, want_cover: bool) -> Result<TrackMeta, String> {
    let key = super::gain::cache_key(&path, size);
    let state = app.state::<MusicState>();
    if let Some(hit) = state.meta.lock().unwrap().get(&key).filter(|_| !want_cover) {
        return Ok(hit.clone());
    }
    let mut prefix = fetch_retry(&app, &path, 0, prefix_end(size)).await?;
    // Capa grande embutida: busca até o fim das tags (uma vez só, com teto).
    if let Some(need) = needed_prefix(&prefix).map(|n| n.min(size).min(MAX_META_PREFIX))
        && need > prefix.len() as u64
    {
        let more = fetch_retry(&app, &path, prefix.len() as u64, need - 1).await?;
        prefix.extend(more);
    }
    let info = tauri::async_runtime::spawn_blocking(move || probe_prefix(prefix, size, &ext)).await.map_err(|e| e.to_string())??;
    let mut meta = TrackMeta {
        title: info.tags.title.clone(),
        artist: info.tags.artist.clone(),
        duration: info.duration,
        codec: info.codec.clone(),
        bits: info.bits,
        sample_rate: info.sample_rate,
        lossless: info.lossless,
        cover: None,
    };
    {
        let mut cache = state.meta.lock().unwrap();
        cache.insert(key, meta.clone());
        if let (Some(p), Ok(json)) = (&state.meta_path, serde_json::to_vec(&*cache)) {
            let _ = std::fs::write(p, json);
        }
    }
    if want_cover {
        meta.cover = info
            .tags
            .cover
            .map(|(mime, data)| format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(data)));
    }
    Ok(meta)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_command_names() {
        assert!(matches!(parse_cmd("toggle"), Some(Command::Toggle)));
        assert!(matches!(parse_cmd("pause"), Some(Command::Pause)));
        assert!(matches!(parse_cmd("resume"), Some(Command::Resume)));
        assert!(matches!(parse_cmd("next"), Some(Command::Next)));
        assert!(matches!(parse_cmd("prev"), Some(Command::Prev)));
        assert!(matches!(parse_cmd("stop"), Some(Command::Stop)));
        assert!(parse_cmd("x").is_none());
    }

    #[test]
    fn meta_prefix_end_is_capped_by_size() {
        assert_eq!(prefix_end(10_000_000), 262_143);
        assert_eq!(prefix_end(1000), 999);
    }
}
