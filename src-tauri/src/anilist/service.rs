use std::collections::HashMap;

use serde::Deserialize;
use serde_json::{Map, Value, json};
use tauri::{AppHandle, Manager};

use super::{
    cache::DiskCache,
    client::AniListClient,
    error::{AniListError, AniListResult},
    matcher::{self, ParsedName},
    queries::{self, MediaType},
};

/// Buscas por requisição (aliases). Mantém a complexidade da query baixa.
const BATCH_SIZE: usize = 10;
const CANDIDATES_PER_SEARCH: u32 = 6;
const MATCH_TTL: i64 = 7 * 24 * 3600;
const DETAIL_TTL: i64 = 24 * 3600;

#[derive(Debug, Clone, Deserialize)]
pub struct LookupRequest {
    /// Nome da pasta na nuvem, ex.: `[2018] B - The Beginning`.
    pub name: String,
    #[serde(rename = "type")]
    pub media_type: MediaType,
}

/// Metadados do AniList com cache em disco.
pub struct AniList {
    client: AniListClient,
    /// chave de busca → resumo da mídia casada (ou `null` se nada casou).
    matches: DiskCache<Option<Value>>,
    /// id → detalhe.
    details: DiskCache<Value>,
}

impl AniList {
    pub fn new(app: &AppHandle) -> Self {
        let dir = app
            .path()
            .app_cache_dir()
            .unwrap_or_else(|_| std::env::temp_dir().join("anitsu-client"));
        Self {
            client: AniListClient::new(),
            matches: DiskCache::open(dir.join("anilist-matches.json"), MATCH_TTL),
            details: DiskCache::open(dir.join("anilist-details.json"), DETAIL_TTL),
        }
    }

    /// Casa nomes de pasta com mídias do AniList. Resultado na mesma ordem da entrada.
    /// Itens que falharam por rede/limite voltam `None` e não entram no cache.
    pub async fn lookup(&self, items: &[LookupRequest]) -> AniListResult<Vec<Option<Value>>> {
        let keyed: Vec<(String, ParsedName, MediaType)> = items
            .iter()
            .map(|it| {
                let parsed = matcher::parse_folder_name(&it.name);
                let key = format!(
                    "{}|{}|{}",
                    it.media_type.as_str(),
                    matcher::normalize(&parsed.title),
                    parsed.year.map(|y| y.to_string()).unwrap_or_default()
                );
                (key, parsed, it.media_type)
            })
            .collect();

        // Buscas que faltam no cache, sem repetição.
        let mut pending: Vec<&(String, ParsedName, MediaType)> = Vec::new();
        for item in &keyed {
            if self.matches.get(&item.0).is_none() && !pending.iter().any(|p| p.0 == item.0) {
                pending.push(item);
            }
        }

        let mut fresh: HashMap<String, Option<Value>> = HashMap::new();
        let mut last_error = None;
        for chunk in pending.chunks(BATCH_SIZE) {
            match self.search_batch(chunk).await {
                Ok(found) => {
                    for (key, value) in found {
                        self.matches.insert(key.clone(), value.clone());
                        fresh.insert(key, value);
                    }
                }
                Err(e) => last_error = Some(e),
            }
        }
        if !pending.is_empty() {
            self.matches.flush();
        }

        // Tudo falhou: propaga o erro em vez de fingir que nada casou.
        if let Some(e) = last_error
            && fresh.is_empty()
            && !pending.is_empty()
        {
            return Err(e);
        }

        Ok(keyed
            .iter()
            .map(|(key, _, _)| fresh.get(key).cloned().flatten().or_else(|| self.matches.get(key).flatten()))
            .collect())
    }

    async fn search_batch(
        &self,
        chunk: &[&(String, ParsedName, MediaType)],
    ) -> AniListResult<Vec<(String, Option<Value>)>> {
        let types: Vec<MediaType> = chunk.iter().map(|c| c.2).collect();
        let query = queries::batch_search_query(&types, CANDIDATES_PER_SEARCH);
        let mut variables = Map::new();
        for (i, c) in chunk.iter().enumerate() {
            variables.insert(format!("s{i}"), Value::String(c.1.title.clone()));
        }

        let data = self.client.query(&query, Value::Object(variables)).await?;

        Ok(chunk
            .iter()
            .enumerate()
            .map(|(i, (key, parsed, _))| {
                let candidates = data
                    .get(format!("m{i}"))
                    .and_then(|p| p.get("media"))
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let best = matcher::pick_best(&candidates, parsed).map(|idx| candidates[idx].clone());
                (key.clone(), best)
            })
            .collect())
    }

    /// Animes em alta. Usa o cache de detalhes com chave própria.
    pub async fn trending(&self) -> AniListResult<Value> {
        const KEY: &str = "trending";
        if let Some(v) = self.details.get(KEY) {
            return Ok(v);
        }
        let data = self.client.query(&queries::trending_query(), json!({})).await?;
        let list = data
            .pointer("/Page/media")
            .cloned()
            .ok_or_else(|| AniListError::Decode("Page.media ausente".into()))?;
        self.details.insert(KEY.into(), list.clone());
        self.details.flush();
        Ok(list)
    }

    /// Detalhe completo (descrição, staff, elenco...).
    pub async fn media(&self, id: i64) -> AniListResult<Value> {
        let key = id.to_string();
        if let Some(v) = self.details.get(&key) {
            return Ok(v);
        }
        let data = self.client.query(queries::DETAIL_QUERY, json!({ "id": id })).await?;
        let media = data.get("Media").cloned().filter(|m| !m.is_null()).ok_or(AniListError::NotFound)?;
        self.details.insert(key, media.clone());
        self.details.flush();
        Ok(media)
    }
}
