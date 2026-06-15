//! Django-compatible PBKDF2-SHA256 password hashing.
//!
//! Stored format: `pbkdf2_sha256$<iterations>$<salt>$<base64hash>`
//! Matches `django.contrib.auth.hashers.PBKDF2PasswordHasher` so rows
//! are interchangeable with the Python backend during the migration window.

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use hmac::Hmac;
use pbkdf2::pbkdf2;
use rand::{distributions::Alphanumeric, Rng};
use sha2::{Digest, Sha256};

const DEFAULT_ITERATIONS: u32 = 600_000;
const HASH_LEN: usize = 32;
const SALT_LEN: usize = 22;

pub enum VerifyOutcome {
    Plain,    // candidate matched directly
    Legacy,   // candidate matched only after sha256-hex transform — caller should rehash
    Failed,
}

pub fn make(plain: &str) -> String {
    let salt: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(SALT_LEN)
        .map(char::from)
        .collect();
    let hash = pbkdf2_sha256(plain.as_bytes(), salt.as_bytes(), DEFAULT_ITERATIONS);
    format!(
        "pbkdf2_sha256${}${}${}",
        DEFAULT_ITERATIONS,
        salt,
        B64.encode(hash)
    )
}

pub fn verify(stored: &str, candidate: &str) -> VerifyOutcome {
    if check(stored, candidate) {
        return VerifyOutcome::Plain;
    }
    let legacy = sha256_hex(candidate);
    if check(stored, &legacy) {
        return VerifyOutcome::Legacy;
    }
    VerifyOutcome::Failed
}

fn check(stored: &str, candidate: &str) -> bool {
    let parts: Vec<&str> = stored.splitn(4, '$').collect();
    if parts.len() != 4 || parts[0] != "pbkdf2_sha256" {
        return false;
    }
    let iterations: u32 = match parts[1].parse() {
        Ok(n) => n,
        Err(_) => return false,
    };
    let salt = parts[2];
    let expected = match B64.decode(parts[3]) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let actual = pbkdf2_sha256(candidate.as_bytes(), salt.as_bytes(), iterations);
    constant_time_eq(&expected, &actual)
}

fn pbkdf2_sha256(password: &[u8], salt: &[u8], iterations: u32) -> Vec<u8> {
    let mut out = vec![0u8; HASH_LEN];
    pbkdf2::<Hmac<Sha256>>(password, salt, iterations, &mut out).expect("pbkdf2");
    out
}

fn sha256_hex(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    let bytes = h.finalize();
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let h = make("hunter2");
        assert!(matches!(verify(&h, "hunter2"), VerifyOutcome::Plain));
        assert!(matches!(verify(&h, "wrong"), VerifyOutcome::Failed));
    }

    #[test]
    fn detects_legacy_double_hash() {
        // Produce a row in the old shape: pbkdf2(sha256(plain)).
        let plain = "secret";
        let derived = sha256_hex(plain);
        let stored = make(&derived);
        match verify(&stored, plain) {
            VerifyOutcome::Legacy => {}
            other => panic!("expected Legacy, got {:?}", match other {
                VerifyOutcome::Plain => "Plain",
                VerifyOutcome::Legacy => "Legacy",
                VerifyOutcome::Failed => "Failed",
            }),
        }
    }
}
