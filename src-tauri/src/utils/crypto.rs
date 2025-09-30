use sha1::{Digest, Sha1};
use sha2::Sha256;

pub struct CryptoUtils;

impl CryptoUtils {
    /// Calculate SHA-1 fingerprint and return as uppercase hex string
    pub fn calculate_sha1_fingerprint(data: &[u8]) -> String {
        let mut hasher = Sha1::new();
        hasher.update(data);
        let result = hasher.finalize();
        hex::encode(result).to_uppercase()
    }

    /// Calculate SHA-256 fingerprint and return as uppercase hex string
    pub fn calculate_sha256_fingerprint(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        hex::encode(result).to_uppercase()
    }
}
