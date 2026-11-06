//! O que a interface pede ao player de música. Ela só manda caminho e tamanho;
//! quem fala com a nuvem é o núcleo.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use tokio::runtime::Handle;

use super::{
    decode::{needed_prefix, probe_prefix},
    engine::{Command, Engine, StateFn},
    media_keys::MediaKeys,
    queue::{QueueTrack, Repeat},
};
use crate::{
    cloud::Cloud,
    events::{Event, EventFn},
};

/// Bytes lidos do começo de cada faixa para tags e duração.
const META_PREFIX: u64 = 256 * 1024;
const META_RETRIES: u32 = 5;
/// Teto para tags com capa embutida.
const MAX_META_PREFIX: u64 = 16 * 1024 * 1024;

pub struct Music {
    pub engine: Engine,
    pub store: super::store::MusicStore,
    cloud: Arc<Cloud>,
    meta: Mutex<HashMap<String, TrackMeta>>,
    meta_path: Option<PathBuf>,
    keys: Arc<OnceLock<MediaKeys>>,
}

impl Music {
    /// Motor tocando na saída padrão; estados vão para `events` (e para as teclas de mídia, se ligadas).
    pub fn start(rt: Handle, cloud: Arc<Cloud>, events: EventFn) -> Self {
        let keys: Arc<OnceLock<MediaKeys>> = Arc::default();
        let k = keys.clone();
        let on_state: StateFn = Box::new(move |s| {
            if let Some(k) = k.get() {
                k.update(&s);
            }
            events(Event::MusicState(s));
        });
        let data = cloud.paths.data.clone();
        let engine = Engine::spawn(rt, cloud.clone(), Some(data.join("music-cache.json")), on_state);
        let meta_path = Some(data.join("music-meta.json"));
        let meta = meta_path
            .as_deref()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        // Preferências salvas valem desde o início.
        let store = super::store::MusicStore::open(data.join("music.json"));
        let p = store.data().prefs;
        let this = Self { engine, store, cloud, meta: Mutex::new(meta), meta_path, keys };
        this.set(Some(p.volume), Some(p.shuffle), Some(p.repeat), Some(p.normalize));
        this
    }

    /// Liga as teclas de mídia e o controle de mídia do sistema à janela (uma vez só).
    pub fn attach_media_keys(&self, hwnd: Option<usize>) {
        let _ = self.keys.get_or_init(|| MediaKeys::start(hwnd, self.engine.clone()));
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
async fn fetch_retry(cloud: &Cloud, path: &str, start: u64, end: u64) -> Result<Vec<u8>, String> {
    let mut tries = 0;
    loop {
        match crate::reader::fetch_range(cloud, path, start, end).await {
            Ok(b) => return Ok(b),
            Err(e) if e.contains("429") && tries < META_RETRIES => {
                tries += 1;
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            Err(e) => return Err(e),
        }
    }
}

impl Music {
    pub fn play(&self, album: String, tracks: Vec<QueueTrack>, index: usize, at: f64) {
        self.engine.send(Command::Play { album, tracks, index, at });
    }

    /// `pause`, `resume`, `toggle`, `next`, `prev` ou `stop`.
    pub fn command(&self, cmd: &str) -> Result<(), String> {
        self.engine.send(parse_cmd(cmd).ok_or_else(|| format!("comando desconhecido: {cmd}"))?);
        Ok(())
    }

    pub fn seek(&self, secs: f64) {
        self.engine.send(Command::Seek(secs));
    }

    pub fn jump(&self, index: usize) {
        self.engine.send(Command::Jump(index));
    }

    pub fn set(&self, volume: Option<f32>, shuffle: Option<bool>, repeat: Option<Repeat>, normalize: Option<bool>) {
        let e = &self.engine;
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
    pub async fn meta(&self, path: &str, size: u64, ext: &str, want_cover: bool) -> Result<TrackMeta, String> {
        let key = super::gain::cache_key(path, size);
        if let Some(hit) = self.meta.lock().unwrap().get(&key).filter(|_| !want_cover) {
            return Ok(hit.clone());
        }
        let mut prefix = fetch_retry(&self.cloud, path, 0, prefix_end(size)).await?;
        // Capa grande embutida: busca até o fim das tags (uma vez só, com teto).
        if let Some(need) = needed_prefix(&prefix).map(|n| n.min(size).min(MAX_META_PREFIX))
            && need > prefix.len() as u64
        {
            let more = fetch_retry(&self.cloud, path, prefix.len() as u64, need - 1).await?;
            prefix.extend(more);
        }
        let ext = ext.to_owned();
        let info = tokio::task::spawn_blocking(move || probe_prefix(prefix, size, &ext)).await.map_err(|e| e.to_string())??;
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
            let mut cache = self.meta.lock().unwrap();
            cache.insert(key, meta.clone());
            if let (Some(p), Ok(json)) = (&self.meta_path, serde_json::to_vec(&*cache)) {
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
