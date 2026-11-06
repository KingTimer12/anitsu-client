use serde::{Serialize, Serializer, ser::SerializeStruct};
use reqwest;

#[derive(Debug, thiserror::Error)]
pub enum AniListError {
    #[error("erro de rede ao falar com o AniList: {0}")]
    Network(#[from] reqwest::Error),
    #[error("limite de requisições do AniList atingido; tente de novo em instantes")]
    RateLimited,
    #[error("não encontrado no AniList")]
    NotFound,
    #[error("AniList respondeu {status}: {message}")]
    Api { status: u16, message: String },
    #[error("resposta inválida do AniList: {0}")]
    Decode(String),
}

impl AniListError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Network(_) => "network",
            Self::RateLimited => "rate_limited",
            Self::NotFound => "not_found",
            Self::Api { .. } => "anilist",
            Self::Decode(_) => "decode",
        }
    }
}

/// Mesmo formato dos outros erros: `{ kind, message, status }`.
impl Serialize for AniListError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("AniListError", 3)?;
        s.serialize_field("kind", self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        s.serialize_field(
            "status",
            &match self {
                Self::Api { status, .. } => Some(*status),
                _ => None,
            },
        )?;
        s.end()
    }
}

pub type AniListResult<T> = Result<T, AniListError>;
