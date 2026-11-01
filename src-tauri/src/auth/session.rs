use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Renova o token quando faltar menos que isso para expirar.
const EXPIRY_MARGIN_SECS: i64 = 60;

/// Sessão do Supabase Auth (mesmo formato do cookie do site e da resposta do GoTrue).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    #[serde(default)]
    pub token_type: Option<String>,
    #[serde(default)]
    pub expires_in: Option<i64>,
    /// Unix timestamp em segundos.
    #[serde(default)]
    pub expires_at: Option<i64>,
    #[serde(default)]
    pub user: Option<Value>,
    /// Demais campos (ex.: `provider_token`), preservados para remontar o cookie igual ao site.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

impl Session {
    /// Garante `expires_at` (o GoTrue às vezes só manda `expires_in`).
    pub fn normalized(mut self) -> Self {
        if self.expires_at.is_none()
            && let Some(expires_in) = self.expires_in
        {
            self.expires_at = Some(now() + expires_in);
        }
        self
    }

    pub fn needs_refresh(&self) -> bool {
        self.expires_at
            .is_some_and(|exp| exp - EXPIRY_MARGIN_SECS <= now())
    }
}

pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
