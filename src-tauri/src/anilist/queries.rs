//! Queries GraphQL do AniList.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum MediaType {
    Anime,
    Manga,
}

impl MediaType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Anime => "ANIME",
            Self::Manga => "MANGA",
        }
    }
}

/// Campos para cards/pôsteres e para o casamento de nomes.
pub const SUMMARY_FRAGMENT: &str = "fragment Summary on Media {
  id type format status
  title { romaji english native userPreferred }
  synonyms
  seasonYear startDate { year }
  coverImage { extraLarge large color }
  bannerImage
  averageScore popularity
  genres
  episodes chapters
}";

/// Página de detalhe: descrição, staff, elenco, estúdio, relações...
pub const DETAIL_QUERY: &str = "query ($id: Int) {
  Media(id: $id) {
    id idMal type format status source countryOfOrigin siteUrl
    title { romaji english native userPreferred }
    synonyms
    description(asHtml: false)
    season seasonYear
    startDate { year month day }
    endDate { year month day }
    episodes duration chapters volumes
    averageScore meanScore popularity favourites
    genres
    tags { name rank isMediaSpoiler }
    coverImage { extraLarge large color }
    bannerImage
    trailer { id site thumbnail }
    studios(isMain: true) { nodes { id name } }
    staff(perPage: 16, sort: [RELEVANCE, ID]) {
      edges { role node { id name { full native } image { large } } }
    }
    characters(perPage: 12, sort: [ROLE, RELEVANCE, ID]) {
      edges {
        role
        node { id name { full native } image { large } }
        voiceActors(language: JAPANESE, sort: [RELEVANCE, ID]) { id name { full native } image { large } }
      }
    }
    relations {
      edges {
        relationType(version: 2)
        node { id type format title { userPreferred } coverImage { large color } startDate { year } }
      }
    }
    externalLinks { site url }
  }
}";

/// Animes em alta (mosaico da tela de login).
pub fn trending_query() -> String {
    format!(
        "query {{ Page(perPage: 30) {{ media(type: ANIME, sort: TRENDING_DESC, isAdult: false) {{ ...Summary }} }} }} {SUMMARY_FRAGMENT}"
    )
}

/// Monta uma query com várias buscas via aliases (`m0`, `m1`, ...), uma requisição só.
pub fn batch_search_query(types: &[MediaType], per_page: u32) -> String {
    let defs: Vec<String> = (0..types.len()).map(|i| format!("$s{i}: String")).collect();
    let body: Vec<String> = types
        .iter()
        .enumerate()
        .map(|(i, t)| {
            format!(
                "m{i}: Page(perPage: {per_page}) {{ media(search: $s{i}, type: {}, sort: SEARCH_MATCH) {{ ...Summary }} }}",
                t.as_str()
            )
        })
        .collect();
    format!(
        "query ({}) {{ {} }} {SUMMARY_FRAGMENT}",
        defs.join(", "),
        body.join(" ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_batch_query() {
        let q = batch_search_query(&[MediaType::Anime, MediaType::Manga], 6);
        assert!(q.starts_with("query ($s0: String, $s1: String)"));
        assert!(q.contains("m0: Page(perPage: 6) { media(search: $s0, type: ANIME"));
        assert!(q.contains("m1: Page(perPage: 6) { media(search: $s1, type: MANGA"));
        assert!(q.contains("fragment Summary on Media"));
    }
}
