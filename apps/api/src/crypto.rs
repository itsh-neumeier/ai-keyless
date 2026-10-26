//! Zufallstoken, Token-Hashes und Passwort-Hashing (Argon2id).

use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// 256 Bit aus dem Betriebssystem-Zufall, base64url ohne Padding.
pub fn random_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("Betriebssystem-Zufall nicht verfügbar");
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Tokens werden nur als SHA-256 gespeichert; ein DB-Leck verrät keine gültigen Tokens.
pub fn token_hash(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn constant_time_eq(a: &str, b: &str) -> bool {
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

/// Argon2id mit den Standardparametern der Bibliothek (PHC-String inkl. Salt).
/// CPU-intensiv: nur über `spawn_blocking` aufrufen.
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    Ok(Argon2::default()
        .hash_password(password.as_bytes())?
        .to_string())
}

pub fn verify_password(password: &str, phc: &str) -> bool {
    Argon2::default()
        .verify_password(password.as_bytes(), phc)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_unique_and_urlsafe() {
        let a = random_token();
        let b = random_token();
        assert_ne!(a, b);
        assert_eq!(a.len(), 43);
        assert!(
            a.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
    }

    #[test]
    fn token_hash_is_stable_hex() {
        let h = token_hash("abc");
        assert_eq!(
            h,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn password_roundtrip() {
        let phc = hash_password("ein-langes-passwort").unwrap();
        assert!(phc.starts_with("$argon2id$"));
        assert!(verify_password("ein-langes-passwort", &phc));
        assert!(!verify_password("falsch-falsch-falsch", &phc));
        assert!(!verify_password("x", "kein-phc"));
    }

    #[test]
    fn constant_time_compare() {
        assert!(constant_time_eq("abc", "abc"));
        assert!(!constant_time_eq("abc", "abd"));
        assert!(!constant_time_eq("abc", "abcd"));
    }
}
