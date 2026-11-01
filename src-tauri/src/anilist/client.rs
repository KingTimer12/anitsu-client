use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tauri_plugin_http::reqwest::{Client, StatusCode, header};
use tokio::sync::Mutex;

use super::error::{AniListError, AniListResult};

const ENDPOINT: &str = "https://graphql.anilist.co";
const MAX_ATTEMPTS: usize = 3;
const DEFAULT_RETRY_AFTER: Duration = Duration::from_secs(60);

/// Cliente GraphQL do AniList (público, sem auth).
///
/// Respeita o rate limit: em `429` espera o `Retry-After` antes de tentar de novo,
/// e chamadas concorrentes esperam o mesmo bloqueio.
pub struct AniListClient {
    http: Client,
    blocked_until: Mutex<Option<Instant>>,
}

impl AniListClient {
    pub fn new() -> Self {
        Self {
            http: Client::builder()
                .user_agent(concat!("anitsu-client/", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("falha ao criar cliente HTTP"),
            blocked_until: Mutex::new(None),
        }
    }

    /// Executa query e devolve `data`. Erros parciais (ex.: um alias 404) são ignorados
    /// quando há `data`; sem `data`, o primeiro erro vira [`AniListError::Api`].
    pub async fn query(&self, query: &str, variables: Value) -> AniListResult<Value> {
        let body = json!({ "query": query, "variables": variables });

        for _ in 0..MAX_ATTEMPTS {
            self.wait_if_blocked().await;

            let resp = self
                .http
                .post(ENDPOINT)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::ACCEPT, "application/json")
                .body(body.to_string())
                .send()
                .await?;

            if resp.status() == StatusCode::TOO_MANY_REQUESTS {
                let retry_after = resp
                    .headers()
                    .get(header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .map(Duration::from_secs)
                    .unwrap_or(DEFAULT_RETRY_AFTER);
                *self.blocked_until.lock().await = Some(Instant::now() + retry_after);
                continue;
            }

            let status = resp.status();
            let text = resp.text().await?;
            let parsed: Value = serde_json::from_str(&text)
                .map_err(|e| AniListError::Decode(format!("{e} (HTTP {status})")))?;
            return extract_data(parsed);
        }
        Err(AniListError::RateLimited)
    }

    async fn wait_if_blocked(&self) {
        let until = *self.blocked_until.lock().await;
        if let Some(until) = until
            && until > Instant::now()
        {
            tokio::time::sleep_until(until.into()).await;
        }
    }
}

fn extract_data(mut body: Value) -> AniListResult<Value> {
    let data = body.get_mut("data").map(Value::take).unwrap_or(Value::Null);
    let has_data = data.as_object().is_some_and(|o| o.values().any(|v| !v.is_null()));
    if has_data {
        return Ok(data);
    }
    let first_error = body
        .get("errors")
        .and_then(Value::as_array)
        .and_then(|errs| errs.first());
    match first_error {
        Some(err) => {
            let status = err.get("status").and_then(Value::as_u64).unwrap_or(500) as u16;
            if status == 404 {
                return Err(AniListError::NotFound);
            }
            let message = err
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("erro desconhecido")
                .to_owned();
            Err(AniListError::Api { status, message })
        }
        None => Err(AniListError::NotFound),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_not_found() {
        let body = json!({"errors":[{"message":"Not Found.","status":404}],"data":{"Media":null}});
        assert!(matches!(extract_data(body), Err(AniListError::NotFound)));
    }

    #[test]
    fn keeps_partial_data() {
        let body = json!({"errors":[{"message":"x","status":404}],"data":{"a":null,"b":{"id":1}}});
        assert_eq!(extract_data(body).unwrap()["b"]["id"], 1);
    }

    #[test]
    fn maps_api_error() {
        let body = json!({"errors":[{"message":"Max query complexity","status":400}],"data":null});
        assert!(matches!(extract_data(body), Err(AniListError::Api { status: 400, .. })));
    }
}
