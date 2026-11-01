//! Tempos de abertura/encerramento do AniSkip (reserva quando o arquivo não tem capítulos).
//!
//! `GET https://api.aniskip.com/v2/skip-times/{malId}/{ep}?types[]=op&types[]=ed&episodeLength={s}`

use serde::{Deserialize, Serialize};

pub const API: &str = "https://api.aniskip.com/v2";
/// "Não encontrado" também fica em cache, por 30 dias.
pub const TTL_SECS: i64 = 30 * 24 * 3600;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Interval {
    pub start: f64,
    pub end: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SkipTimes {
    pub op: Option<Interval>,
    pub ed: Option<Interval>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    #[serde(default)]
    results: Vec<Result>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Result {
    interval: RawInterval,
    skip_type: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawInterval {
    start_time: f64,
    end_time: f64,
}

/// `None` = falha transitória (não guardar em cache). 404 = sem tempos conhecidos.
pub fn parse_response(status: u16, body: &str) -> Option<SkipTimes> {
    match status {
        404 => Some(SkipTimes::default()),
        200 => {
            let resp: Response = serde_json::from_str(body).ok()?;
            let pick = |kind: &str| {
                resp.results
                    .iter()
                    .find(|r| r.skip_type == kind)
                    .map(|r| Interval { start: r.interval.start_time, end: r.interval.end_time })
            };
            // "mixed-*": abertura/encerramento sobre cenas do episódio; valem quando não há o puro.
            Some(SkipTimes {
                op: pick("op").or_else(|| pick("mixed-op")),
                ed: pick("ed").or_else(|| pick("mixed-ed")),
            })
        }
        _ => None,
    }
}

pub fn url(mal_id: u32, episode: u32, duration: f64) -> String {
    format!(
        "{API}/skip-times/{mal_id}/{episode}?types[]=op&types[]=ed&types[]=mixed-op&types[]=mixed-ed&episodeLength={}",
        duration.round() as i64
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_found() {
        let body = r#"{"found":true,"results":[{"interval":{"startTime":90.5,"endTime":180.0},"skipType":"op","skipId":"x","episodeLength":1420.0},{"interval":{"startTime":1290.0,"endTime":1380.0},"skipType":"ed","skipId":"y","episodeLength":1420.0}],"message":"Successfully found skip times","statusCode":200}"#;
        assert_eq!(
            parse_response(200, body),
            Some(SkipTimes {
                op: Some(Interval { start: 90.5, end: 180.0 }),
                ed: Some(Interval { start: 1290.0, end: 1380.0 }),
            })
        );
    }

    #[test]
    fn parse_mixed_types() {
        let body = r#"{"found":true,"results":[{"interval":{"startTime":10.0,"endTime":100.0},"skipType":"mixed-op","skipId":"a","episodeLength":1420.0},{"interval":{"startTime":1300.0,"endTime":1390.0},"skipType":"mixed-ed","skipId":"b","episodeLength":1420.0}],"message":"","statusCode":200}"#;
        assert_eq!(
            parse_response(200, body),
            Some(SkipTimes {
                op: Some(Interval { start: 10.0, end: 100.0 }),
                ed: Some(Interval { start: 1300.0, end: 1390.0 }),
            })
        );
    }

    #[test]
    fn url_asks_mixed_types() {
        let u = url(1, 2, 1420.4);
        assert!(u.contains("types[]=mixed-op") && u.contains("types[]=mixed-ed"), "{u}");
    }

    #[test]
    fn parse_not_found() {
        let body = r#"{"found":false,"results":[],"message":"No skip times found","statusCode":404}"#;
        assert_eq!(parse_response(404, body), Some(SkipTimes::default()));
    }

    #[test]
    fn parse_server_error() {
        assert_eq!(parse_response(500, "oops"), None);
    }
}
