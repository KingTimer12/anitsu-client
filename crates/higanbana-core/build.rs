use std::time::{SystemTime, UNIX_EPOCH};

const KEY_VAR: &str = "ANITSU_SUPABASE_ANON_KEY";

fn main() {
    embed_anon_key();
}

/// Lê a chave anon da Anitsu de `ANITSU_SUPABASE_ANON_KEY` (variável de ambiente
/// ou `.env` na raiz do projeto, fora do git) e a embute no binário mascarada
/// com XOR, para não aparecer em texto puro no repositório nem no executável.
fn embed_anon_key() {
    println!("cargo:rerun-if-env-changed={KEY_VAR}");
    println!("cargo:rerun-if-changed=../../.env");

    let key = std::env::var(KEY_VAR)
        .ok()
        .or_else(|| read_dotenv("../../.env"))
        .map(|k| k.trim().to_owned())
        .filter(|k| !k.is_empty())
        .unwrap_or_else(|| {
            panic!(
                "\n\n{KEY_VAR} não definida.\n\
                 Copie `.env.example` para `.env` na raiz do projeto e preencha a chave \
                 (veja a seção \"Chave do Supabase\" do README).\n\n"
            )
        });

    let mask = random_bytes(key.len());
    let masked: Vec<u8> = key.bytes().zip(&mask).map(|(b, m)| b ^ m).collect();
    println!("cargo:rustc-env=ANITSU_ANON_KEY_MASKED={}", hex(&masked));
    println!("cargo:rustc-env=ANITSU_ANON_KEY_MASK={}", hex(&mask));
}

fn read_dotenv(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok()?.lines().find_map(|line| {
        let (k, v) = line.trim().split_once('=')?;
        (k.trim() == KEY_VAR).then(|| v.trim().trim_matches('"').to_owned())
    })
}

/// Bytes pseudo-aleatórios (xorshift semeado pelo relógio); só para mascarar.
fn random_bytes(n: usize) -> Vec<u8> {
    let mut s = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15)
        | 1;
    (0..n)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            (s >> 24) as u8
        })
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
