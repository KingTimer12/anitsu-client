//! Metadados do AniList (https://graphql.anilist.co), só leitura.
//!
//! - [`service::AniList`]: busca em lote + casamento com nomes de pasta + cache em disco.
//! - [`client::AniListClient`]: GraphQL com respeito ao rate limit (30–90 req/min).
//! - [`matcher`]: limpa nomes de pasta da nuvem e escolhe o melhor resultado.

pub(crate) mod cache;
mod client;
pub mod commands;
mod error;
mod matcher;
mod queries;
mod service;

use tauri::Manager;

pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    app.manage(service::AniList::new(app.handle()));
    Ok(())
}
