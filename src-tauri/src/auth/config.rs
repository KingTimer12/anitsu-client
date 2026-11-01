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
    /// Chave `anon` pública. Não fica no repositório: vem de `ANITSU_SUPABASE_ANON_KEY`
    /// na hora do build (ver `build.rs`) e é embutida mascarada.
    #[serde(skip)]
    pub supabase_anon_key: String,
    /// `sb-${hostname.split(".")[0]}-auth-token` do `@supabase/ssr`.
    pub session_cookie: String,
}

pub static CONFIG: LazyLock<AnitsuConfig> = LazyLock::new(|| {
    let mut config: AnitsuConfig = serde_json::from_str(include_str!("../../../anitsu.config.json"))
        .expect("anitsu.config.json inválido");
    config.supabase_anon_key = anon_key();
    config
});

/// Desfaz a máscara XOR aplicada pelo `build.rs`.
fn anon_key() -> String {
    let masked = unhex(env!("ANITSU_ANON_KEY_MASKED"));
    let mask = unhex(env!("ANITSU_ANON_KEY_MASK"));
    let bytes: Vec<u8> = masked.iter().zip(&mask).map(|(b, m)| b ^ m).collect();
    String::from_utf8(bytes).expect("chave anon inválida")
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex inválido"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anon_key_is_unmasked() {
        let key = &CONFIG.supabase_anon_key;
        // JWT: três partes em base64url.
        assert_eq!(key.split('.').count(), 3, "chave anon não parece um JWT");
    }
}
