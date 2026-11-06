//! Página onde parou, preferências por título e configurações do leitor
//! (`<pasta de dados>/reader.json`).

use std::{collections::HashMap, path::PathBuf, sync::Mutex};

use serde::{Deserialize, Serialize};

use super::logic::{Direction, ProgressStyle, ReadMode};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadProgress {
    pub page: usize,
    pub total: usize,
    pub read: bool,
    pub updated_at: i64,
    /// Só no resumo de uma coletânea: chave e nome do capítulo interno onde parou.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct TitlePrefs {
    pub direction: Direction,
    pub mode: ReadMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReaderSettings {
    /// Emendar capítulos (scroll infinito / virar direto para o próximo).
    pub seam: bool,
    pub progress: ProgressStyle,
}

impl Default for ReaderSettings {
    fn default() -> Self {
        Self { seam: true, progress: ProgressStyle::Thread }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReaderData {
    pub progress: HashMap<String, ReadProgress>,
    pub prefs: HashMap<String, TitlePrefs>,
    pub settings: ReaderSettings,
}

pub struct ReaderStore {
    path: PathBuf,
    data: Mutex<ReaderData>,
}

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

impl ReaderStore {
    pub fn open(path: PathBuf) -> Self {
        let data = std::fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Self { path, data: Mutex::new(data) }
    }

    pub fn data(&self) -> ReaderData {
        self.data.lock().unwrap().clone()
    }

    fn update(&self, f: impl FnOnce(&mut ReaderData)) {
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

    /// `key` = caminho do arquivo ou chave do capítulo interno.
    pub fn set_progress(&self, key: &str, page: usize, total: usize) {
        self.update(|d| {
            let prev_read = d.progress.get(key).is_some_and(|p| p.read);
            let read = prev_read || page + 1 >= total;
            d.progress.insert(key.to_owned(), ReadProgress { page, total, read, updated_at: now(), unit: None, unit_name: None });
        });
    }

    /// Resumo de uma coletânea: em qual capítulo interno parou.
    pub fn set_summary(&self, path: &str, index: usize, count: usize, read: bool, unit: &str, unit_name: &str) {
        self.update(|d| {
            let prev_read = d.progress.get(path).is_some_and(|p| p.read);
            d.progress.insert(
                path.to_owned(),
                ReadProgress {
                    page: index,
                    total: count,
                    read: prev_read || (read && index + 1 >= count),
                    updated_at: now(),
                    unit: Some(unit.to_owned()),
                    unit_name: Some(unit_name.to_owned()),
                },
            );
        });
    }

    pub fn set_prefs(&self, title_path: &str, prefs: TitlePrefs) {
        self.update(|d| {
            d.prefs.insert(title_path.to_owned(), prefs);
        });
    }

    pub fn set_settings(&self, s: ReaderSettings) {
        self.update(|d| d.settings = s);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_marks_read_on_last_page_and_keeps_it() {
        let path = std::env::temp_dir().join(format!("higanbana-reader-test-{}.json", std::process::id()));
        let s = ReaderStore::open(path.clone());
        s.set_progress("a.cbz", 3, 10);
        assert!(!s.data().progress["a.cbz"].read);
        s.set_progress("a.cbz", 9, 10);
        assert!(s.data().progress["a.cbz"].read);
        s.set_progress("a.cbz", 0, 10);
        assert!(s.data().progress["a.cbz"].read);
        let again = ReaderStore::open(path.clone());
        assert_eq!(again.data().progress["a.cbz"].page, 0);
        let _ = std::fs::remove_file(path);
        assert_eq!(serde_json::to_value(TitlePrefs::default()).unwrap()["direction"], "rtl");
    }
}
