//! Onde parou em cada álbum e preferências do player (`<pasta de dados>/music.json`).

use std::{collections::HashMap, path::PathBuf, sync::Mutex};

use serde::{Deserialize, Serialize};

use super::queue::Repeat;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumProgress {
    /// Índice da faixa no álbum.
    pub index: usize,
    pub position: f64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MusicPrefs {
    /// 0..1 (a curva perceptiva é aplicada no motor).
    pub volume: f32,
    pub shuffle: bool,
    pub repeat: Repeat,
    /// Volume igualado entre álbuns.
    pub normalize: bool,
}

impl Default for MusicPrefs {
    fn default() -> Self {
        Self { volume: 0.8, shuffle: false, repeat: Repeat::All, normalize: true }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MusicData {
    pub progress: HashMap<String, AlbumProgress>,
    pub prefs: MusicPrefs,
}

pub struct MusicStore {
    path: PathBuf,
    data: Mutex<MusicData>,
}

impl MusicStore {
    pub fn open(path: PathBuf) -> Self {
        let data = std::fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Self { path, data: Mutex::new(data) }
    }

    pub fn data(&self) -> MusicData {
        self.data.lock().unwrap().clone()
    }

    fn update(&self, f: impl FnOnce(&mut MusicData)) {
        let bytes = {
            let mut d = self.data.lock().unwrap();
            f(&mut d);
            serde_json::to_vec(&*d)
        };
        if let Ok(b) = bytes {
            let tmp = self.path.with_extension("json.tmp");
            if std::fs::write(&tmp, b).is_ok() {
                let _ = std::fs::rename(&tmp, &self.path);
            }
        }
    }

    pub fn set_progress(&self, album: &str, index: usize, position: f64) {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
        self.update(|d| {
            d.progress.insert(album.to_owned(), AlbumProgress { index, position, updated_at: now });
        });
    }

    pub fn set_prefs(&self, prefs: MusicPrefs) {
        self.update(|d| d.prefs = prefs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_defaults() {
        let path = std::env::temp_dir().join(format!("higanbana-music-test-{}.json", std::process::id()));
        let s = MusicStore::open(path.clone());
        assert_eq!(s.data().prefs, MusicPrefs::default());
        s.set_progress("OSTs/X", 3, 61.5);
        s.set_prefs(MusicPrefs { volume: 0.5, shuffle: true, repeat: Repeat::One, normalize: false });
        let again = MusicStore::open(path.clone());
        assert_eq!(again.data().progress["OSTs/X"].index, 3);
        assert_eq!(again.data().prefs.repeat, Repeat::One);
        let _ = std::fs::remove_file(path);
    }
}
