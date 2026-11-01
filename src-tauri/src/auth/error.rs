use serde::{Serialize, Serializer, ser::SerializeStruct};

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("não autenticado")]
    NotAuthenticated,
    #[error("login cancelado")]
    Cancelled,
    #[error("tempo de login esgotado")]
    Timeout,
    #[error("login já está em andamento")]
    InProgress,
    /// Servidor recusou o refresh token (revogado/expirado). Sessão local é descartada.
    #[error("sessão inválida: {0}")]
    InvalidSession(String),
    #[error("erro de rede: {0}")]
    Network(String),
    /// Discord/GoTrue recusou ou o fluxo OAuth falhou.
    #[error("{0}")]
    Provider(String),
    #[error("erro de armazenamento: {0}")]
    Storage(String),
    #[error(transparent)]
    Tauri(#[from] tauri::Error),
}

impl AuthError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::NotAuthenticated => "not_authenticated",
            Self::Cancelled => "cancelled",
            Self::Timeout => "timeout",
            Self::InProgress => "in_progress",
            Self::InvalidSession(_) => "invalid_session",
            Self::Network(_) => "network",
            Self::Provider(_) => "provider",
            Self::Storage(_) => "storage",
            Self::Tauri(_) => "internal",
        }
    }
}

impl From<std::io::Error> for AuthError {
    fn from(e: std::io::Error) -> Self {
        Self::Storage(e.to_string())
    }
}

impl From<serde_json::Error> for AuthError {
    fn from(e: serde_json::Error) -> Self {
        Self::Storage(e.to_string())
    }
}

impl From<tauri_plugin_http::reqwest::Error> for AuthError {
    fn from(e: tauri_plugin_http::reqwest::Error) -> Self {
        Self::Network(e.to_string())
    }
}

/// Chega no frontend como `{ kind, message }`.
impl Serialize for AuthError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("AuthError", 2)?;
        s.serialize_field("kind", self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

pub type AuthResult<T> = Result<T, AuthError>;
