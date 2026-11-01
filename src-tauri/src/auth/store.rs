use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use super::{error::AuthResult, session::Session};

const SESSION_FILE: &str = "session.json";

/// Persistência da sessão em `<app_data_dir>/session.json`.
pub struct SessionStore {
    path: PathBuf,
}

impl SessionStore {
    pub fn new(app: &AppHandle) -> AuthResult<Self> {
        let dir = app.path().app_data_dir()?;
        std::fs::create_dir_all(&dir)?;
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
