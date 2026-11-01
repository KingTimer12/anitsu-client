use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_http::reqwest::Method;
use tokio::{fs::File, io::AsyncWriteExt};

use crate::{
    auth::AuthManager,
    http::{AnitsuHttp, HttpResult},
};

pub const PROGRESS_EVENT: &str = "nuvem://download-progress";
const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress<'a> {
    pub id: &'a str,
    pub path: &'a str,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub done: bool,
}

/// Nome de arquivo seguro para o sistema local.
pub fn sanitize_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let trimmed = cleaned.trim_matches([' ', '.']);
    if trimmed.is_empty() { "download".into() } else { trimmed.into() }
}

/// Baixa `remote_path` em stream para `dest` (via `<dest>.part`), emitindo progresso.
pub async fn download(
    app: &AppHandle,
    http: &AnitsuHttp,
    auth: &AuthManager,
    id: &str,
    remote_path: &str,
    dest: &Path,
) -> HttpResult<PathBuf> {
    let mut resp = http
        .send(auth, Method::GET, "download", |r| r.query(&[("path", remote_path)]))
        .await?;
    let total = resp.content_length();

    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let part = PathBuf::from(format!("{}.part", dest.display()));

    let result: HttpResult<u64> = async {
        let mut file = File::create(&part).await?;
        let mut downloaded = 0u64;
        let mut last_emit = Instant::now();
        while let Some(chunk) = resp.chunk().await? {
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            if last_emit.elapsed() >= PROGRESS_INTERVAL {
                last_emit = Instant::now();
                let _ = app.emit(
                    PROGRESS_EVENT,
                    DownloadProgress { id, path: remote_path, downloaded, total, done: false },
                );
            }
        }
        file.flush().await?;
        Ok(downloaded)
    }
    .await;

    let downloaded = match result {
        Ok(n) => n,
        Err(e) => {
            let _ = tokio::fs::remove_file(&part).await;
            return Err(e);
        }
    };

    tokio::fs::rename(&part, dest).await?;
    let _ = app.emit(
        PROGRESS_EVENT,
        DownloadProgress { id, path: remote_path, downloaded, total, done: true },
    );
    Ok(dest.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_names() {
        assert_eq!(
            sanitize_file_name("[Troidex] B-The Beginning - Ep. 01 [BD-Rip_1080p_H264_Dual].mkv"),
            "[Troidex] B-The Beginning - Ep. 01 [BD-Rip_1080p_H264_Dual].mkv"
        );
        assert_eq!(sanitize_file_name("a/b\\c:d?.mkv"), "a_b_c_d_.mkv");
        assert_eq!(sanitize_file_name(".."), "download");
    }
}
