use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};

use super::{Listing, SearchResponse, download};
use crate::{
    auth::AuthManager,
    http::{AnitsuApis, HttpResult},
};

/// Lista um diretório. `path` vazio = raiz.
#[tauri::command]
pub async fn nuvem_list(
    apis: State<'_, AnitsuApis>,
    auth: State<'_, AuthManager>,
    path: String,
) -> HttpResult<Listing> {
    apis.nuvem.get_json(&auth, "files", &[("path", path)]).await
}

/// Busca pastas por nome (animes, mangás, doramas, OSTs...).
#[tauri::command]
pub async fn nuvem_search(
    apis: State<'_, AnitsuApis>,
    auth: State<'_, AuthManager>,
    query: String,
) -> HttpResult<SearchResponse> {
    apis.nuvem.get_json(&auth, "search", &[("q", query)]).await
}

/// Baixa um arquivo. Sem `dest`, salva na pasta Downloads do usuário.
/// Progresso em `nuvem://download-progress` (filtrar por `id`).
#[tauri::command]
pub async fn nuvem_download(
    app: AppHandle,
    apis: State<'_, AnitsuApis>,
    auth: State<'_, AuthManager>,
    id: String,
    path: String,
    dest: Option<PathBuf>,
) -> HttpResult<PathBuf> {
    let dest = match dest {
        Some(d) => d,
        None => {
            let name = path.rsplit('/').next().unwrap_or(&path);
            app.path()
                .download_dir()?
                .join(download::sanitize_file_name(name))
        }
    };
    download::download(&app, &apis.nuvem, &auth, &id, &path, &dest).await
}
