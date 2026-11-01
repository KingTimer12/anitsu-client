//! Autenticação no anitsu.moe (Supabase Auth em `auth.anitsu.moe`, login via Discord).
//!
//! - [`manager::AuthManager`]: dono da sessão (memória + disco + refresh).
//! - [`browser_login`]: OAuth Discord no navegador padrão + servidor loopback + PKCE.
//! - [`gotrue`]: refresh/logout no GoTrue.
//! - [`commands`]: ponte para o frontend.

mod browser_login;
pub mod commands;
pub mod config;
mod cookie;
mod error;
mod gotrue;
mod manager;
mod session;
mod pkce;
mod store;

use tauri::Manager;

pub use error::AuthError;
pub use manager::AuthManager;

/// Registra o [`AuthManager`] como state na inicialização do app.
pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    app.manage(AuthManager::new(app.handle().clone())?);
    Ok(())
}
