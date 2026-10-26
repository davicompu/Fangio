//! Pure helpers for AI-decision provenance.

use sha2::{Digest, Sha256};

/// First 12 lowercase hex chars of the SHA-256 of a system prompt.
pub fn prompt_fingerprint(system_prompt: &str) -> String {
    let digest = Sha256::digest(system_prompt.as_bytes());
    digest.iter().take(6).map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_12_hex_and_stable() {
        let a = prompt_fingerprint("You are X");
        assert_eq!(a.len(), 12);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(a, prompt_fingerprint("You are X"));
        assert_ne!(a, prompt_fingerprint("You are Y"));
    }
}
