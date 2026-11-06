use serde::{Serialize, Serializer, ser::SerializeStruct};
use reqwest;

use crate::auth::AuthError;

#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error("erro de rede: {0}")]
    Network(#[from] reqwest::Error),
    #[error("HTTP {status}: {body}")]
    Status { status: u16, body: String },
    #[error("resposta inválida: {0}")]
    Decode(String),
    #[error("caminho inválido: {0}")]
    InvalidPath(String),
    #[error("erro de arquivo: {0}")]
    Io(#[from] std::io::Error),
}

impl HttpError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Auth(e) => e.kind(),
            Self::Status { status: 401, .. } => "not_authenticated",
            Self::Status { .. } => "http",
            Self::Network(_) => "network",
            Self::Decode(_) => "decode",
            Self::InvalidPath(_) => "invalid_path",
            Self::Io(_) => "io",
        }
    }
}

/// Chega no frontend como `{ kind, message, status }`.
impl Serialize for HttpError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("HttpError", 3)?;
        s.serialize_field("kind", self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        s.serialize_field(
            "status",
            &match self {
                Self::Status { status, .. } => Some(*status),
                _ => None,
            },
        )?;
        s.end()
    }
}

pub type HttpResult<T> = Result<T, HttpError>;
