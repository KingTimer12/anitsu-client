use std::path::{Path, PathBuf};

use super::{error::AuthResult, session::Session};

const SESSION_FILE: &str = "session.json";

/// Persistência da sessão em `<pasta de dados>/session.json`.
pub struct SessionStore {
    path: PathBuf,
}

impl SessionStore {
    pub fn new(dir: &Path) -> AuthResult<Self> {
        std::fs::create_dir_all(dir)?;
        Ok(Self {
            path: dir.join(SESSION_FILE),
        })
    }

    /// Arquivo ausente ou corrompido = sem sessão.
    pub fn load(&self) -> Option<Session> {
        let text = std::fs::read_to_string(&self.path).ok()?;
        serde_json::from_str(&text).ok()
    }

    /// Escrita atômica: grava em arquivo temporário e renomeia.
    pub fn save(&self, session: &Session) -> AuthResult<()> {
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(session)?)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    pub fn clear(&self) -> AuthResult<()> {
        match std::fs::remove_file(&self.path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
            _ => Ok(()),
        }
    }
}
