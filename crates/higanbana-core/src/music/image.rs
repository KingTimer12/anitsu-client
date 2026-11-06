//! Imagens soltas da nuvem (capa do álbum, scans), buscadas com a sessão do núcleo.

use crate::{cloud::Cloud, reader::mime_for};

/// Não servir arquivos maiores que isso como imagem.
pub const MAX_IMAGE: u64 = 25 * 1024 * 1024;

/// `"/<tamanho>/<caminho>"`, tudo codificado (formato das URLs `nuvemimg://` do frontend web).
pub fn parse_image_uri(uri_path: &str) -> Option<(u64, String)> {
    let decoded = urlencoding::decode(uri_path.trim_start_matches('/')).ok()?;
    let (size, path) = decoded.split_once('/')?;
    let size = size.parse().ok()?;
    (!path.is_empty()).then(|| (size, path.to_owned()))
}

pub fn allowed(path: &str, size: u64) -> bool {
    size > 0 && size <= MAX_IMAGE && mime_for(path).starts_with("image/")
}

impl Cloud {
    /// Bytes de uma imagem da nuvem (até [`MAX_IMAGE`]).
    pub async fn image(&self, path: &str, size: u64) -> Result<Vec<u8>, String> {
        if !allowed(path, size) {
            return Err("não é uma imagem da nuvem".into());
        }
        crate::reader::fetch_range(self, path, 0, size - 1).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_size_and_path() {
        let raw = format!("/{}", urlencoding::encode("1234/OSTs/A b/Scans/Front.JPG"));
        assert_eq!(parse_image_uri(&raw), Some((1234, "OSTs/A b/Scans/Front.JPG".to_string())));
        assert_eq!(parse_image_uri("/x/y.jpg"), None);
        assert_eq!(parse_image_uri("/12/"), None);
    }

    #[test]
    fn refuses_huge_or_non_image_files() {
        assert!(allowed("a/b.jpg", 1000));
        assert!(!allowed("a/b.jpg", MAX_IMAGE + 1));
        assert!(!allowed("a/b.flac", 1000));
    }
}
