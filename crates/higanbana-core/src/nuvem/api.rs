use std::path::PathBuf;

use super::{Listing, SearchResponse, download};
use crate::{cloud::Cloud, http::HttpResult};

/// Pastas do acervo que o app não mostra: `AniGUI (Downloader Oficial)` é de outro aplicativo.
pub fn is_hidden(name: &str) -> bool {
    let n = name.trim().to_lowercase();
    n.starts_with("anigui") && !n[6..].starts_with(|c: char| c.is_alphanumeric() || c == '_')
}

impl Cloud {
    /// Lista um diretório. `path` vazio = raiz.
    pub async fn list(&self, path: &str) -> HttpResult<Listing> {
        let mut listing: Listing = self.apis.nuvem.get_json(&self.auth, "files", &[("path", path)]).await?;
        listing.files.retain(|f| !is_hidden(&f.name));
        Ok(listing)
    }

    /// Busca pastas por nome (animes, mangás, doramas, OSTs...).
    pub async fn search(&self, query: &str) -> HttpResult<SearchResponse> {
        let mut res: SearchResponse = self.apis.nuvem.get_json(&self.auth, "search", &[("q", query)]).await?;
        res.results.retain(|r| !r.path.split('/').any(is_hidden));
        Ok(res)
    }

    /// Onde um arquivo da nuvem fica ao ser baixado sem destino escolhido.
    pub fn default_download_path(&self, remote_path: &str) -> Option<PathBuf> {
        let name = remote_path.rsplit('/').next().unwrap_or(remote_path);
        Some(self.paths.downloads.as_ref()?.join(download::sanitize_file_name(name)))
    }

    /// Baixa um arquivo. Sem `dest`, salva na pasta Downloads do usuário.
    /// Progresso em [`crate::Event::DownloadProgress`] (filtrar por `id`).
    pub async fn download(&self, id: &str, path: &str, dest: Option<PathBuf>) -> HttpResult<PathBuf> {
        let dest = match dest.or_else(|| self.default_download_path(path)) {
            Some(d) => d,
            None => {
                return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "pasta Downloads não encontrada").into());
            }
        };
        download::download(&self.events, &self.apis.nuvem, &self.auth, id, path, &dest).await
    }
}
