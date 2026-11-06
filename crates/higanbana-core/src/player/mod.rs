//! Player de vídeo: proxy de stream local, AniSkip e estado persistido.
//! Quem toca é a interface (mpv); daqui sai só a URL e o que lembrar.

mod api;
pub mod aniskip;
pub mod logic;
pub mod proxy;
pub mod store;

use std::sync::Arc;

use futures_util::{FutureExt, future::BoxFuture, stream};
use reqwest::{Client, Method, header::RANGE};

use crate::{anilist::cache::DiskCache, cloud::Cloud};
use aniskip::SkipTimes;
use proxy::{StreamProxy, Upstream, UpstreamResponse};
use store::PlayerStore;

pub struct Player {
    cloud: Arc<Cloud>,
    pub proxy: StreamProxy,
    pub store: PlayerStore,
    aniskip: DiskCache<SkipTimes>,
    http: Client,
}

/// Busca os bytes em `/api/preview` da nuvem, com a sessão do núcleo.
pub struct NuvemUpstream {
    cloud: Arc<Cloud>,
}

impl Upstream for NuvemUpstream {
    fn fetch(&self, path: &str, range: Option<&str>) -> BoxFuture<'static, Result<UpstreamResponse, String>> {
        let cloud = self.cloud.clone();
        let path = path.to_owned();
        let range = range.map(str::to_owned);
        async move {
            let resp = cloud
                .apis
                .nuvem
                .send_raw(&cloud.auth, Method::GET, "preview", |r| {
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

impl Player {
    /// Sobe o proxy local; precisa rodar dentro do runtime do núcleo.
    pub async fn start(cloud: Arc<Cloud>) -> Result<Self, Box<dyn std::error::Error>> {
        let proxy = StreamProxy::start(NuvemUpstream { cloud: cloud.clone() }).await?;
        let dir = &cloud.paths.data;
        Ok(Self {
            store: PlayerStore::open(dir.join("player.json")),
            aniskip: DiskCache::open(dir.join("aniskip-v2.json"), aniskip::TTL_SECS),
            http: Client::builder().user_agent(concat!("higanbana-client/", env!("CARGO_PKG_VERSION"))).build()?,
            proxy,
            cloud,
        })
    }
}
