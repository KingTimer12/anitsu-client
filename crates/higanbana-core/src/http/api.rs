use serde::Deserialize;
use serde_json::{Map, Value};
use reqwest::Method;

use super::{
    client::{Service, read_value},
    error::{HttpError, HttpResult},
};
use crate::cloud::Cloud;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestArgs {
    pub service: Service,
    #[serde(default)]
    pub method: Option<String>,
    pub path: String,
    /// Valores não-string viram string; `null` é ignorado.
    #[serde(default)]
    pub query: Option<Map<String, Value>>,
    #[serde(default)]
    pub body: Option<Value>,
}

/// Requisição genérica para um serviço do Anitsu (JSON/texto).
/// Para arquivos grandes use os serviços dedicados (ex.: [`Cloud::download`]).
pub async fn request(cloud: &Cloud, request: RequestArgs) -> HttpResult<Value> {
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

    let resp = cloud
        .apis
        .get(request.service)
        .send(&cloud.auth, method, &request.path, |r| {
            let r = r.query(&query);
            match &request.body {
                Some(body) => r.json(body),
                None => r,
            }
        })
        .await?;
    read_value(resp).await
}
