//! Protocolo `nuvemimg://`: imagens soltas da nuvem (capa do álbum, scans).
//! O webview pede `<tamanho>/<caminho>`; o Rust busca com a sessão.

use tauri::{
    UriSchemeContext, UriSchemeResponder,
    http::{Request, Response, StatusCode, header},
};

use crate::reader::mime_for;

/// Não servir arquivos maiores que isso como imagem.
pub const MAX_IMAGE: u64 = 25 * 1024 * 1024;

/// `"/<tamanho>/<caminho>"` (o `convertFileSrc` codifica tudo).
pub fn parse_image_uri(uri_path: &str) -> Option<(u64, String)> {
    let decoded = urlencoding::decode(uri_path.trim_start_matches('/')).ok()?;
    let (size, path) = decoded.split_once('/')?;
    let size = size.parse().ok()?;
    (!path.is_empty()).then(|| (size, path.to_owned()))
}

pub fn allowed(path: &str, size: u64) -> bool {
    size > 0 && size <= MAX_IMAGE && mime_for(path).starts_with("image/")
}

fn text(status: StatusCode, msg: &str) -> Response<Vec<u8>> {
    Response::builder().status(status).header(header::CONTENT_TYPE, "text/plain; charset=utf-8").body(msg.as_bytes().to_vec()).unwrap()
}

pub fn handle_protocol(ctx: UriSchemeContext<'_, tauri::Wry>, request: Request<Vec<u8>>, responder: UriSchemeResponder) {
    let app = ctx.app_handle().clone();
    let Some((size, path)) = parse_image_uri(request.uri().path()).filter(|(s, p)| allowed(p, *s)) else {
        return responder.respond(text(StatusCode::BAD_REQUEST, "endereço inválido"));
    };
    tauri::async_runtime::spawn(async move {
        let resp = match crate::reader::fetch_range(&app, &path, 0, size - 1).await {
            Ok(bytes) => Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime_for(&path))
                .header(header::CACHE_CONTROL, "max-age=31536000, immutable")
                .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .body(bytes)
                .unwrap(),
            Err(e) => text(StatusCode::BAD_GATEWAY, &e),
        };
        responder.respond(resp);
    });
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
