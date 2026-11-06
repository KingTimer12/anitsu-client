//! Autenticação no anitsu.moe (Supabase Auth em `auth.anitsu.moe`, login via Discord).
//!
//! - [`manager::AuthManager`]: dono da sessão (memória + disco + refresh).
//! - [`browser_login`]: OAuth Discord no navegador padrão + servidor loopback + PKCE.
//! - [`gotrue`]: refresh/logout no GoTrue.

mod browser_login;
pub mod config;
mod cookie;
mod error;
mod gotrue;
mod manager;
pub mod session;
mod pkce;
mod store;

pub use error::{AuthError, AuthResult};
pub use manager::AuthManager;
pub use session::Session;
