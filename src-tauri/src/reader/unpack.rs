//! Descompactação de `.rar` cujas imagens estão compactadas.
//!
//! O formato de compressão do RAR é proprietário; quem descompacta é a `unrar`
//! (código da RARLAB). Ela só abre arquivo em disco, então o rar (já baixado
//! para a memória) vai para um arquivo temporário que é apagado logo depois.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const TEMP_PREFIX: &str = "higanbana-unrar-";

/// Apaga o arquivo temporário mesmo se a descompactação falhar.
struct TempFile(PathBuf);

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn temp_path() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("{TEMP_PREFIX}{}-{n}.rar", std::process::id()))
}

/// Nome no mesmo formato do índice (`rar::file_entry`): separador `/`.
fn entry_name(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// Todos os arquivos do rar (em memória), pelo nome.
pub fn unpack(data: &[u8]) -> Result<HashMap<String, Vec<u8>>, String> {
    let tmp = TempFile(temp_path());
    std::fs::write(&tmp.0, data).map_err(|e| format!("não foi possível preparar o rar: {e}"))?;
    let mut files = HashMap::new();
    let mut arc = unrar::Archive::new(&tmp.0).open_for_processing().map_err(|e| format!("rar inválido: {e}"))?;
    while let Some(header) = arc.read_header().map_err(|e| format!("rar corrompido: {e}"))? {
        arc = if header.entry().is_file() {
            let name = entry_name(&header.entry().filename);
            let (bytes, next) = header.read().map_err(|e| format!("falha ao descompactar {name}: {e}"))?;
            files.insert(name, bytes);
            next
        } else {
            header.skip().map_err(|e| format!("rar corrompido: {e}"))?
        };
    }
    Ok(files)
}

/// Sobras de uma sessão que fechou no meio de uma descompactação.
pub fn clean_leftovers() {
    let Ok(dir) = std::fs::read_dir(std::env::temp_dir()) else { return };
    for f in dir.flatten() {
        if f.file_name().to_string_lossy().starts_with(TEMP_PREFIX) {
            let _ = std::fs::remove_file(f.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_file_is_removed_even_on_error() {
        assert!(unpack(b"not a rar").is_err());
        let left = std::fs::read_dir(std::env::temp_dir())
            .unwrap()
            .flatten()
            .filter(|f| f.file_name().to_string_lossy().starts_with(&format!("{TEMP_PREFIX}{}-", std::process::id())))
            .count();
        assert_eq!(left, 0);
    }
}
