use serde_json::Value;
use tauri::State;

use super::{
    error::AniListResult,
    service::{AniList, LookupRequest},
};

/// Casa nomes de pasta com o AniList. Mesma ordem da entrada; `null` = sem match.
#[tauri::command]
pub async fn anilist_lookup(
    anilist: State<'_, AniList>,
    items: Vec<LookupRequest>,
) -> AniListResult<Vec<Option<Value>>> {
    anilist.lookup(&items).await
}

/// Animes da temporada atual no AniList.
#[tauri::command]
pub async fn anilist_season(anilist: State<'_, AniList>) -> AniListResult<Value> {
    anilist.season().await
}

/// Detalhe completo de uma mídia.
#[tauri::command]
pub async fn anilist_media(anilist: State<'_, AniList>, id: i64) -> AniListResult<Value> {
    anilist.media(id).await
}
