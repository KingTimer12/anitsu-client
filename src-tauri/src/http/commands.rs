use serde::Deserialize;
use serde_json::{Map, Value};
use tauri::State;
use tauri_plugin_http::reqwest::Method;

use super::{
    client::{AnitsuApis, Service, read_value},
    error::{HttpError, HttpResult},
};
use crate::auth::AuthManager;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestArgs {
    service: Service,
    #[serde(default)]
    method: Option<String>,
    path: String,
    /// Valores não-string viram string; `null` é ignorado.
    #[serde(default)]
    query: Option<Map<String, Value>>,
    #[serde(default)]
    body: Option<Value>,
}

/// Requisição genérica para um serviço do Anitsu (JSON/texto).
/// Para arquivos grandes use comandos dedicados (ex.: `nuvem_download`).
#[tauri::command]
pub async fn anitsu_request(
    apis: State<'_, AnitsuApis>,
    auth: State<'_, AuthManager>,
    request: RequestArgs,
) -> HttpResult<Value> {
    let method = request
        .method
        .as_deref()
        .unwrap_or("GET")
        .to_uppercase()
        .parse::<Method>()
        .map_err(|e| HttpError::InvalidPath(e.to_string()))?;

    let query: Vec<(String, String)> = request
        .query
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(k, v)| match v {
            Value::Null => None,
            Value::String(s) => Some((k, s)),
            other => Some((k, other.to_string())),
        })
        .collect();

    let resp = apis
        .get(request.service)
        .send(&auth, method, &request.path, |r| {
            let r = r.query(&query);
            match &request.body {
                Some(body) => r.json(body),
                None => r,
            }
        })
        .await?;
    read_value(resp).await
}
