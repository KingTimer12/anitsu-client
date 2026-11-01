//! Player de vídeo: proxy de stream local, AniSkip e estado persistido.

pub mod commands;
pub mod proxy;

use futures_util::{FutureExt, future::BoxFuture, stream};
use tauri::{AppHandle, Manager};
use tauri_plugin_http::reqwest::{Method, header::RANGE};

use crate::{auth::AuthManager, http::AnitsuApis};
use proxy::{StreamProxy, Upstream, UpstreamResponse};

pub struct PlayerState {
    pub proxy: StreamProxy,
}

/// Busca os bytes em `/api/preview` da nuvem, com a sessão do [`AuthManager`].
pub struct NuvemUpstream {
    app: AppHandle,
}

impl Upstream for NuvemUpstream {
    fn fetch(&self, path: &str, range: Option<&str>) -> BoxFuture<'static, Result<UpstreamResponse, String>> {
        let app = self.app.clone();
        let path = path.to_owned();
        let range = range.map(str::to_owned);
        async move {
            let apis = app.state::<AnitsuApis>();
            let auth = app.state::<AuthManager>();
            let resp = apis
                .nuvem
                .send(&auth, Method::GET, "preview", |r| {
                    let r = r.query(&[("path", &path)]);
                    match &range {
                        Some(range) => r.header(RANGE, range),
                        None => r,
                    }
                })
                .await
                .map_err(|e| e.to_string())?;

            let status = resp.status().as_u16();
            let headers = resp
                .headers()
                .iter()
                .filter_map(|(k, v)| Some((k.to_string(), v.to_str().ok()?.to_owned())))
                .collect();
            let body = stream::unfold(resp, |mut resp| async move {
                match resp.chunk().await {
                    Ok(Some(chunk)) => Some((Ok(chunk), resp)),
                    Ok(None) => None,
                    Err(e) => Some((Err(std::io::Error::other(e.to_string())), resp)),
                }
            });
            Ok(UpstreamResponse { status, headers, body: Box::pin(body) })
        }
        .boxed()
    }
}

pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let upstream = NuvemUpstream { app: app.handle().clone() };
    let proxy = tauri::async_runtime::block_on(StreamProxy::start(upstream))?;
    app.manage(PlayerState { proxy });
    Ok(())
}
