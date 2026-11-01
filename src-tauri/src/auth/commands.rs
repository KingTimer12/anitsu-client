//! Comandos expostos ao frontend.

use tauri::State;

use super::{error::AuthResult, manager::AuthManager, session::Session};

#[tauri::command]
pub async fn auth_login(auth: State<'_, AuthManager>) -> AuthResult<Session> {
    auth.login().await
}

#[tauri::command]
pub async fn auth_logout(auth: State<'_, AuthManager>) -> AuthResult<()> {
    auth.logout().await
}

#[tauri::command]
pub async fn auth_session(auth: State<'_, AuthManager>) -> AuthResult<Option<Session>> {
    auth.session().await
}

#[tauri::command]
pub async fn auth_access_token(auth: State<'_, AuthManager>) -> AuthResult<Option<String>> {
    auth.access_token().await
}

#[tauri::command]
pub async fn auth_refresh(auth: State<'_, AuthManager>) -> AuthResult<Session> {
    auth.refresh().await
}

/// Reabre o navegador na página de login em andamento.
#[tauri::command]
pub async fn auth_reopen_login(auth: State<'_, AuthManager>) -> AuthResult<()> {
    auth.reopen_login().await
}

/// Cancela o login em andamento (`auth_login` rejeita com `cancelled`).
#[tauri::command]
pub async fn auth_cancel_login(auth: State<'_, AuthManager>) -> AuthResult<()> {
    auth.cancel_login().await;
    Ok(())
}
