use std::sync::LazyLock;

use serde::Deserialize;

/// Config compartilhada com o frontend (`anitsu.config.json` na raiz do projeto).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnitsuConfig {
    /// Site Next.js (`https://anitsu.moe`).
    pub site_url: String,
    /// Servidor de arquivos (`https://nuvem.anitsu.moe`), autenticado pelo cookie de sessão.
    pub nuvem_url: String,
    /// Supabase (GoTrue/PostgREST) self-hosted (`https://auth.anitsu.moe`).
    pub supabase_url: String,
    /// Chave `anon` pública, a mesma embutida no bundle JS do site.
    pub supabase_anon_key: String,
    /// `sb-${hostname.split(".")[0]}-auth-token` do `@supabase/ssr`.
    pub session_cookie: String,
}

pub static CONFIG: LazyLock<AnitsuConfig> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../anitsu.config.json"))
        .expect("anitsu.config.json inválido")
});
