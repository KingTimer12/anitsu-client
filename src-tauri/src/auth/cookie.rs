//! Cookie de sessão do `@supabase/ssr`, usado para autenticar na nuvem.
//!
//! Formato: `<nome>=base64-<base64url(JSON)>`, possivelmente dividido em
//! `<nome>.0`, `<nome>.1`, ... quando passa do limite de tamanho de cookie.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

use super::session::Session;

/// `true` se `name` é o cookie de sessão ou um pedaço dele.
#[cfg(test)]
pub fn is_session_cookie(cookie_name: &str, name: &str) -> bool {
    match name.strip_prefix(cookie_name) {
        Some("") => true,
        Some(rest) => rest
            .strip_prefix('.')
            .is_some_and(|i| i.parse::<usize>().is_ok()),
        None => false,
    }
}

/// Inverso de [`encode_session`]; usado nos testes de ida e volta.
#[cfg(test)]
pub fn decode_session(cookie_name: &str, cookies: &[(String, String)]) -> Option<Session> {
    let raw = join_chunks(cookie_name, cookies)?;
    let json = match raw.strip_prefix("base64-") {
        Some(b64) => String::from_utf8(URL_SAFE_NO_PAD.decode(b64.trim_end_matches('=')).ok()?).ok()?,
        None => raw,
    };
    serde_json::from_str::<Session>(&json)
        .ok()
        .map(Session::normalized)
}

/// Tamanho máximo de cada pedaço (mesmo valor do `@supabase/ssr`).
const MAX_CHUNK_SIZE: usize = 3180;

/// Monta o header `Cookie` igual ao que o navegador manda para `*.anitsu.moe`:
/// `nome=base64-...` ou `nome.0=...; nome.1=...` quando passa de [`MAX_CHUNK_SIZE`].
pub fn encode_session(cookie_name: &str, session: &Session) -> Result<String, serde_json::Error> {
    let value = format!("base64-{}", URL_SAFE_NO_PAD.encode(serde_json::to_vec(session)?));
    if value.len() <= MAX_CHUNK_SIZE {
        return Ok(format!("{cookie_name}={value}"));
    }
    // base64url é ASCII, então cortar por bytes é seguro.
    Ok(value
        .as_bytes()
        .chunks(MAX_CHUNK_SIZE)
        .enumerate()
        .map(|(i, chunk)| format!("{cookie_name}.{i}={}", std::str::from_utf8(chunk).unwrap()))
        .collect::<Vec<_>>()
        .join("; "))
}

#[cfg(test)]
fn join_chunks(cookie_name: &str, cookies: &[(String, String)]) -> Option<String> {
    if let Some((_, v)) = cookies.iter().find(|(n, _)| n == cookie_name) {
        return Some(v.clone());
    }

    let mut chunks: Vec<(usize, &str)> = cookies
        .iter()
        .filter_map(|(n, v)| {
            let idx = n.strip_prefix(cookie_name)?.strip_prefix('.')?.parse().ok()?;
            Some((idx, v.as_str()))
        })
        .collect();
    if chunks.is_empty() {
        return None;
    }
    chunks.sort_by_key(|(i, _)| *i);
    // Pedaços precisam ser contíguos a partir de 0.
    if chunks.iter().enumerate().any(|(pos, (i, _))| pos != *i) {
        return None;
    }
    Some(chunks.into_iter().map(|(_, v)| v).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::session::now;

    const NAME: &str = "sb-auth-auth-token";
    const JSON: &str = r#"{"access_token":"at","refresh_token":"rt","token_type":"bearer","expires_in":3600,"expires_at":1999999999,"user":{"id":"u1"}}"#;

    fn encode(json: &str) -> String {
        format!("base64-{}", URL_SAFE_NO_PAD.encode(json))
    }

    #[test]
    fn decodes_single_cookie() {
        let s = decode_session(NAME, &[(NAME.into(), encode(JSON))]).unwrap();
        assert_eq!(s.access_token, "at");
        assert_eq!(s.refresh_token, "rt");
        assert_eq!(s.expires_at, Some(1999999999));
    }

    #[test]
    fn decodes_chunked_cookie_out_of_order() {
        let full = encode(JSON);
        let (a, b) = full.split_at(20);
        let s = decode_session(
            NAME,
            &[
                (format!("{NAME}.1"), b.into()),
                ("other".into(), "x".into()),
                (format!("{NAME}.0"), a.into()),
            ],
        )
        .unwrap();
        assert_eq!(s.access_token, "at");
    }

    #[test]
    fn ignores_missing_or_broken() {
        assert!(decode_session(NAME, &[]).is_none());
        assert!(decode_session(NAME, &[(format!("{NAME}.1"), "x".into())]).is_none());
        assert!(decode_session(NAME, &[(NAME.into(), "base64-!!!".into())]).is_none());
    }

    #[test]
    fn computes_expires_at_from_expires_in() {
        let s = decode_session(
            NAME,
            &[(NAME.into(), encode(r#"{"access_token":"a","refresh_token":"r","expires_in":3600}"#))],
        )
        .unwrap();
        assert!(s.expires_at.unwrap() > now());
        assert!(!s.needs_refresh());
    }

    fn parse_header(header: &str) -> Vec<(String, String)> {
        header
            .split("; ")
            .map(|kv| {
                let (k, v) = kv.split_once('=').unwrap();
                (k.to_owned(), v.to_owned())
            })
            .collect()
    }

    #[test]
    fn encode_roundtrip_small() {
        let s = decode_session(NAME, &[(NAME.into(), encode(JSON))]).unwrap();
        let header = encode_session(NAME, &s).unwrap();
        assert!(header.starts_with(&format!("{NAME}=base64-")));
        let back = decode_session(NAME, &parse_header(&header)).unwrap();
        assert_eq!(back.access_token, "at");
        assert_eq!(back.user, s.user);
    }

    #[test]
    fn encode_roundtrip_chunked_preserves_extra_fields() {
        let big = format!(
            r#"{{"access_token":"{}","refresh_token":"rt","expires_at":1999999999,"provider_token":"pt"}}"#,
            "a".repeat(5000)
        );
        let s = decode_session(NAME, &[(NAME.into(), encode(&big))]).unwrap();
        let header = encode_session(NAME, &s).unwrap();
        assert!(header.contains(&format!("{NAME}.0=")));
        assert!(header.contains(&format!("{NAME}.1=")));
        let back = decode_session(NAME, &parse_header(&header)).unwrap();
        assert_eq!(back.access_token.len(), 5000);
        assert_eq!(back.extra.get("provider_token").unwrap(), "pt");
    }

    #[test]
    fn matches_session_cookie_names() {
        assert!(is_session_cookie(NAME, NAME));
        assert!(is_session_cookie(NAME, &format!("{NAME}.3")));
        assert!(!is_session_cookie(NAME, &format!("{NAME}-code-verifier")));
        assert!(!is_session_cookie(NAME, "other"));
    }
}
