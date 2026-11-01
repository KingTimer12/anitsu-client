//! Cliente mínimo do GoTrue (Supabase Auth) usando o reqwest do `tauri-plugin-http`.

use tauri_plugin_http::reqwest::{Client, header::CONTENT_TYPE};

use super::{
    config::CONFIG,
    error::{AuthError, AuthResult},
    session::Session,
};

pub struct GoTrueClient {
    http: Client,
}

impl GoTrueClient {
    pub fn new() -> Self {
        Self { http: Client::new() }
    }

    fn url(path: &str) -> String {
        format!("{}/auth/v1{path}", CONFIG.supabase_url)
    }

    /// `POST /auth/v1/token?grant_type=refresh_token`.
    /// Resposta 4xx = refresh token inválido → [`AuthError::InvalidSession`].
    pub async fn refresh(&self, refresh_token: &str) -> AuthResult<Session> {
        let body = serde_json::to_vec(&serde_json::json!({ "refresh_token": refresh_token }))?;
        let resp = self
            .http
            .post(Self::url("/token?grant_type=refresh_token"))
            .header("apikey", &CONFIG.supabase_anon_key)
            .header(CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await?;

        let status = resp.status();
        let bytes = resp.bytes().await?;
        if status.is_client_error() {
            return Err(AuthError::InvalidSession(String::from_utf8_lossy(&bytes).into()));
        }
        if !status.is_success() {
            return Err(AuthError::Network(format!(
                "{status}: {}",
                String::from_utf8_lossy(&bytes)
            )));
        }
        let session: Session = serde_json::from_slice(&bytes)
            .map_err(|e| AuthError::Network(format!("resposta inválida do GoTrue: {e}")))?;
        Ok(session.normalized())
    }

    /// URL de início do OAuth (PKCE). O GoTrue redireciona para o provedor e,
    /// no fim, para `redirect_to?code=...`.
    pub fn authorize_url(&self, provider: &str, redirect_to: &str, code_challenge: &str) -> String {
        tauri::Url::parse_with_params(
            &Self::url("/authorize"),
            [
                ("provider", provider),
                ("redirect_to", redirect_to),
                ("code_challenge", code_challenge),
                ("code_challenge_method", "s256"),
            ],
        )
        .expect("URL do GoTrue inválida")
        .into()
    }

    /// `POST /auth/v1/token?grant_type=pkce`: troca o `code` do callback pela sessão.
    pub async fn exchange_code(&self, auth_code: &str, code_verifier: &str) -> AuthResult<Session> {
        let body = serde_json::to_vec(&serde_json::json!({
            "auth_code": auth_code,
            "code_verifier": code_verifier,
        }))?;
        let resp = self
            .http
            .post(Self::url("/token?grant_type=pkce"))
            .header("apikey", &CONFIG.supabase_anon_key)
            .header(CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await?;

        let status = resp.status();
        let bytes = resp.bytes().await?;
        if !status.is_success() {
            let msg = serde_json::from_slice::<serde_json::Value>(&bytes)
                .ok()
                .and_then(|v| {
                    v.get("msg")
                        .or_else(|| v.get("error_description"))
                        .and_then(|m| m.as_str().map(str::to_owned))
                })
                .unwrap_or_else(|| String::from_utf8_lossy(&bytes).into_owned());
            return Err(AuthError::Provider(format!("troca do código falhou ({status}): {msg}")));
        }
        let session: Session = serde_json::from_slice(&bytes)
            .map_err(|e| AuthError::Network(format!("resposta inválida do GoTrue: {e}")))?;
        Ok(session.normalized())
    }

    /// `POST /auth/v1/logout` (revoga o refresh token no servidor).
    pub async fn logout(&self, access_token: &str) -> AuthResult<()> {
        self.http
            .post(Self::url("/logout"))
            .header("apikey", &CONFIG.supabase_anon_key)
            .bearer_auth(access_token)
            .send()
            .await?;
        Ok(())
    }
}
