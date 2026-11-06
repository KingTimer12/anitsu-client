//! PKCE (RFC 7636): o `code` que volta pelo navegador só vira sessão
//! com o `verifier`, que nunca sai deste processo.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

impl Pkce {
    pub fn new() -> Self {
        let mut bytes = [0u8; 48];
        getrandom::fill(&mut bytes).expect("gerador de números aleatórios do sistema indisponível");
        Self::from_verifier(URL_SAFE_NO_PAD.encode(bytes))
    }

    fn from_verifier(verifier: String) -> Self {
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        Self { verifier, challenge }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_rfc7636_example() {
        // Apêndice B da RFC 7636.
        let p = Pkce::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk".into());
        assert_eq!(p.challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn verifier_has_valid_length() {
        let p = Pkce::new();
        assert!((43..=128).contains(&p.verifier.len()));
        assert_ne!(Pkce::new().verifier, p.verifier);
    }
}
