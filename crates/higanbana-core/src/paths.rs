//! Pastas do app no sistema.

use std::{io, path::PathBuf};

/// Mesmo identificador do app Tauri: quem já usava mantém sessão, progresso e caches.
pub const IDENTIFIER: &str = "dev.kingtimer12.higanbana";

#[derive(Debug, Clone)]
pub struct Paths {
    /// Sessão, progresso do player, caches de música (`%APPDATA%/<id>` no Windows).
    pub data: PathBuf,
    /// Caches do AniList (`%LOCALAPPDATA%/<id>` no Windows).
    pub cache: PathBuf,
    /// Pasta Downloads do usuário, destino padrão dos downloads.
    pub downloads: Option<PathBuf>,
}

impl Paths {
    /// Pastas padrão do sistema, criadas se faltarem.
    pub fn system() -> io::Result<Self> {
        let missing = || io::Error::new(io::ErrorKind::NotFound, "pasta de dados do sistema não encontrada");
        let paths = Self {
            data: dirs::data_dir().ok_or_else(missing)?.join(IDENTIFIER),
            cache: dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join(IDENTIFIER),
            downloads: dirs::download_dir(),
        };
        paths.create()?;
        Ok(paths)
    }

    /// Tudo dentro de `root` (testes, modo portátil).
    pub fn under(root: impl Into<PathBuf>) -> io::Result<Self> {
        let root = root.into();
        let paths = Self { data: root.join("data"), cache: root.join("cache"), downloads: Some(root.join("downloads")) };
        paths.create()?;
        Ok(paths)
    }

    fn create(&self) -> io::Result<()> {
        std::fs::create_dir_all(&self.data)?;
        std::fs::create_dir_all(&self.cache)
    }
}
