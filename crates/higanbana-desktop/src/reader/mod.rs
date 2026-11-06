//! Leitor de mangá.

mod view;

use std::collections::HashMap;

use higanbana_core::{nuvem::FileEntry, reader::store::ReadProgress};

use crate::state::AppState;

pub use view::{ReaderEvent, ReaderView};

/// O que está aberto no leitor. `dir` é a pasta dos arquivos; `title_path`, a chave das preferências.
#[derive(Debug, Clone)]
pub struct ReadTarget {
    pub dir: String,
    pub file: FileEntry,
    pub files: Vec<FileEntry>,
    pub title_path: String,
    pub title: String,
}

/// Progresso de leitura por caminho (lido do núcleo).
pub fn progress_map(s: &AppState) -> HashMap<String, ReadProgress> {
    s.core.reader().store.data().progress
}
