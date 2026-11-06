use serde::Serialize;

use super::{Reader, zip::Entry};

const IMAGE_EXT: [&str; 7] = ["jpg", "jpeg", "png", "webp", "gif", "avif", "bmp"];
const ARCHIVE_EXT: [&str; 4] = ["cbz", "zip", "cbr", "rar"];

/// Uma imagem do arquivo. `index` é a posição no índice do zip (usada na URL da página).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PageInfo {
    pub index: usize,
    pub name: String,
}

/// O que há num zip: imagens (capítulo/volume, talvez em pastas) ou
/// `.cbz` guardados dentro dele (coletânea de capítulos).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OpenResult {
    pub pages: Vec<PageInfo>,
    pub archives: Vec<String>,
}

fn has_ext(e: &Entry, exts: &[&str]) -> bool {
    let name = e.name.to_ascii_lowercase();
    let base = name.rsplit('/').next().unwrap_or(&name);
    !e.is_dir()
        && !name.starts_with("__macosx/")
        && !base.starts_with("._")
        && base.rsplit_once('.').is_some_and(|(_, ext)| exts.contains(&ext))
}

/// Imagens (sem pastas, miniaturas do macOS nem outros arquivos) e zips internos.
pub fn contents_of(entries: &[Entry]) -> OpenResult {
    let pages = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| has_ext(e, &IMAGE_EXT))
        .map(|(index, e)| PageInfo { index, name: e.name.clone() })
        .collect();
    let archives = entries.iter().filter(|e| has_ext(e, &ARCHIVE_EXT)).map(|e| e.name.clone()).collect();
    OpenResult { pages, archives }
}

impl Reader {
    /// Abre um `.cbz` da nuvem (ou o `inner` guardado dentro dele): lê só o índice.
    pub async fn open(&self, path: &str, size: u64, inner: Option<&str>) -> Result<OpenResult, String> {
        let arc = self.archive(path, size, inner).await?;
        let out = contents_of(&arc.entries);
        if out.pages.is_empty() && out.archives.is_empty() {
            return Err("Nenhuma imagem neste arquivo.".into());
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(name: &str) -> Entry {
        Entry { name: name.into(), method: 0, compressed_size: 1, size: 1, local_offset: 0, data_offset: None }
    }

    #[test]
    fn keeps_only_images() {
        let entries = [e("Cap 01/"), e("Cap 01/001.JPG"), e("ComicInfo.xml"), e("__MACOSX/Cap 01/._001.jpg"), e("Cap 01/._002.png"), e("002.webp")];
        let out = contents_of(&entries);
        assert_eq!(
            out.pages,
            vec![PageInfo { index: 1, name: "Cap 01/001.JPG".into() }, PageInfo { index: 5, name: "002.webp".into() }]
        );
        assert!(out.archives.is_empty());
    }

    #[test]
    fn lists_inner_archives_of_a_collection() {
        let entries = [e("Capitulo 1.cbz"), e("Capitulo 10.CBZ"), e("leia-me.txt"), e("extras/Capitulo 2.zip")];
        let out = contents_of(&entries);
        assert!(out.pages.is_empty());
        assert_eq!(out.archives, vec!["Capitulo 1.cbz", "Capitulo 10.CBZ", "extras/Capitulo 2.zip"]);
    }
}
