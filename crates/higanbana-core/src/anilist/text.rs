//! Textos do AniList em português: rótulos, datas e descrição limpa.

use std::sync::LazyLock;

use regex::Regex;

use super::types::FuzzyDate;

fn pick(map: &[(&str, &str)], key: Option<&str>) -> String {
    let Some(key) = key.filter(|k| !k.is_empty()) else { return String::new() };
    map.iter().find(|(k, _)| *k == key).map_or_else(
        || {
            // Desconhecido: "SOME_KEY" → "Some key".
            let mut s = key.to_lowercase().replace('_', " ");
            if let Some(f) = s.get_mut(0..1) {
                f.make_ascii_uppercase();
            }
            s
        },
        |(_, v)| v.to_string(),
    )
}

const FORMAT: &[(&str, &str)] = &[
    ("TV", "Série de TV"),
    ("TV_SHORT", "TV curta"),
    ("MOVIE", "Filme"),
    ("SPECIAL", "Especial"),
    ("OVA", "OVA"),
    ("ONA", "ONA"),
    ("MUSIC", "Clipe musical"),
    ("MANGA", "Mangá"),
    ("NOVEL", "Light novel"),
    ("ONE_SHOT", "One-shot"),
];
const STATUS: &[(&str, &str)] = &[
    ("FINISHED", "Finalizado"),
    ("RELEASING", "Em lançamento"),
    ("NOT_YET_RELEASED", "Ainda não lançado"),
    ("CANCELLED", "Cancelado"),
    ("HIATUS", "Em hiato"),
];
const SEASON: &[(&str, &str)] = &[("WINTER", "Inverno"), ("SPRING", "Primavera"), ("SUMMER", "Verão"), ("FALL", "Outono")];
const GENRE: &[(&str, &str)] = &[
    ("Action", "Ação"),
    ("Adventure", "Aventura"),
    ("Comedy", "Comédia"),
    ("Fantasy", "Fantasia"),
    ("Horror", "Terror"),
    ("Music", "Música"),
    ("Mystery", "Mistério"),
    ("Psychological", "Psicológico"),
    ("Sci-Fi", "Ficção científica"),
    ("Slice of Life", "Cotidiano"),
    ("Sports", "Esportes"),
    ("Supernatural", "Sobrenatural"),
    ("Thriller", "Suspense"),
];
const SOURCE: &[(&str, &str)] = &[
    ("ORIGINAL", "Original"),
    ("MANGA", "Mangá"),
    ("LIGHT_NOVEL", "Light novel"),
    ("VISUAL_NOVEL", "Visual novel"),
    ("VIDEO_GAME", "Jogo"),
    ("NOVEL", "Romance"),
    ("WEB_NOVEL", "Web novel"),
    ("WEB_MANGA", "Web mangá"),
    ("DOUJINSHI", "Doujinshi"),
    ("ANIME", "Anime"),
    ("GAME", "Jogo"),
    ("COMIC", "Quadrinhos"),
    ("LIVE_ACTION", "Live action"),
    ("PICTURE_BOOK", "Livro ilustrado"),
    ("MULTIMEDIA_PROJECT", "Projeto multimídia"),
    ("OTHER", "Outro"),
];
const RELATION: &[(&str, &str)] = &[
    ("SEQUEL", "Sequência"),
    ("PREQUEL", "Prequela"),
    ("SIDE_STORY", "História paralela"),
    ("PARENT", "Obra principal"),
    ("ALTERNATIVE", "Versão alternativa"),
    ("ADAPTATION", "Adaptação"),
    ("SOURCE", "Obra original"),
    ("SUMMARY", "Resumo"),
    ("SPIN_OFF", "Spin-off"),
    ("CHARACTER", "Mesmos personagens"),
    ("COMPILATION", "Compilação"),
    ("CONTAINS", "Contém"),
    ("OTHER", "Relacionado"),
];
const ROLE: &[(&str, &str)] = &[
    ("Director", "Direção"),
    ("Original Creator", "Criação original"),
    ("Original Story", "História original"),
    ("Original Character Design", "Design original de personagens"),
    ("Character Design", "Design de personagens"),
    ("Series Composition", "Composição da série"),
    ("Script", "Roteiro"),
    ("Music", "Música"),
    ("Art Director", "Direção de arte"),
    ("Sound Director", "Direção de som"),
    ("Chief Animation Director", "Direção-chefe de animação"),
    ("Animation Director", "Direção de animação"),
    ("Assistant Director", "Assistente de direção"),
    ("Director of Photography", "Direção de fotografia"),
    ("Theme Song Performance", "Tema musical"),
    ("Story", "História"),
    ("Art", "Arte"),
    ("Story & Art", "História e arte"),
];
const CHARACTER_ROLE: &[(&str, &str)] = &[("MAIN", "Principal"), ("SUPPORTING", "Coadjuvante"), ("BACKGROUND", "Figurante")];

pub fn format(f: Option<&str>) -> String {
    pick(FORMAT, f)
}
pub fn status(s: Option<&str>) -> String {
    pick(STATUS, s)
}
pub fn season(s: Option<&str>) -> String {
    pick(SEASON, s)
}
pub fn source(s: Option<&str>) -> String {
    pick(SOURCE, s)
}
pub fn relation(r: Option<&str>) -> String {
    pick(RELATION, r)
}
pub fn character_role(r: Option<&str>) -> String {
    pick(CHARACTER_ROLE, r)
}
pub fn genre(g: &str) -> String {
    GENRE.iter().find(|(k, _)| *k == g).map_or(g, |(_, v)| v).to_owned()
}

/// "Key Animation (ep 3)" → mantém os detalhes entre parênteses.
pub fn staff_role(role: &str) -> String {
    let (base, rest) = match role.find('(') {
        Some(i) => (role[..i].trim(), Some(role[i..].trim())),
        None => (role.trim(), None),
    };
    let base = ROLE.iter().find(|(k, _)| *k == base).map_or(base, |(_, v)| v);
    match rest {
        Some(r) => format!("{base} {r}"),
        None => base.to_owned(),
    }
}

const MONTHS: [&str; 12] = ["jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"];

pub fn date(d: Option<FuzzyDate>) -> Option<String> {
    let d = d?;
    let year = d.year?;
    let Some(m) = d.month.filter(|m| (1..=12).contains(m)) else { return Some(year.to_string()) };
    Some(match d.day {
        Some(day) => format!("{day} {} {year}", MONTHS[m as usize - 1]),
        None => format!("{} {year}", MONTHS[m as usize - 1]),
    })
}

/// Número com separador de milhar (`12.345`).
pub fn number(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

static BR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)<br\s*/?>").unwrap());
static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());
static SOURCE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\n?\s*\((?:Source|Fonte):\s*([^)]*)\)\s*$").unwrap());
static PARAS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\n\s*\n+").unwrap());
static LINES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*\n\s*").unwrap());

fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#039;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
        .replace("&mdash;", "—")
        .replace("&ndash;", "–")
        .replace("&hellip;", "…")
}

/// Descrição do AniList (HTML simples) → parágrafos de texto e a fonte, se houver.
pub fn clean_description(raw: Option<&str>) -> (Vec<String>, Option<String>) {
    let Some(raw) = raw else { return (Vec::new(), None) };
    let text = BR.replace_all(raw, "\n");
    let text = decode_entities(&TAG.replace_all(&text, "")).replace('\r', "");
    let mut source = None;
    let body = SOURCE_RE
        .replace(&text, |c: &regex::Captures| {
            source = Some(c[1].trim().to_owned());
            ""
        })
        .into_owned();
    let paragraphs = PARAS
        .split(&body)
        .map(|p| LINES.replace_all(p, " ").trim().to_owned())
        .filter(|p| !p.is_empty())
        .collect();
    (paragraphs, source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels() {
        assert_eq!(format(Some("TV")), "Série de TV");
        assert_eq!(format(Some("NEW_THING")), "New thing");
        assert_eq!(format(None), "");
        assert_eq!(genre("Action"), "Ação");
        assert_eq!(genre("Mecha"), "Mecha");
        assert_eq!(staff_role("Key Animation (ep 3)"), "Key Animation (ep 3)");
        assert_eq!(staff_role("Director"), "Direção");
        assert_eq!(staff_role("Director (eps 1-3)"), "Direção (eps 1-3)");
    }

    #[test]
    fn dates_and_numbers() {
        assert_eq!(date(Some(FuzzyDate { year: Some(2018), month: Some(3), day: Some(2) })).as_deref(), Some("2 mar 2018"));
        assert_eq!(date(Some(FuzzyDate { year: Some(2018), month: None, day: None })).as_deref(), Some("2018"));
        assert_eq!(date(None), None);
        assert_eq!(number(1234567), "1.234.567");
        assert_eq!(number(12), "12");
    }

    #[test]
    fn description() {
        let (p, s) = clean_description(Some("Linha <i>um</i><br>continua.<br><br>Segundo &amp; fim.\n(Source: Crunchyroll)"));
        assert_eq!(p, vec!["Linha um continua.", "Segundo & fim."]);
        assert_eq!(s.as_deref(), Some("Crunchyroll"));
    }
}
