use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Token storage with encryption and expiration
///
/// Tokens are encrypted using XOR with a machine-specific key for basic obfuscation.
/// For production use, consider using proper encryption libraries like ring or sodiumoxide.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStore {
    tokens: HashMap<String, StoredToken>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredToken {
    /// Encrypted token value
    encrypted_value: Vec<u8>,
    /// When this token was stored (Unix timestamp)
    stored_at: u64,
    /// Token validity duration in seconds (default: 30 days)
    valid_for_seconds: u64,
}

impl TokenStore {
    /// Create a new empty token store
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
        }
    }

    /// Load token store from disk
    pub fn load() -> crate::Result<Self> {
        let store_path = Self::store_path()?;

        if store_path.exists() {
            let contents = std::fs::read_to_string(&store_path)?;
            let store: TokenStore = serde_json::from_str(&contents)
                .map_err(|e| crate::Error::ConfigError(format!("Failed to parse token store: {}", e)))?;
            Ok(store)
        } else {
            Ok(Self::new())
        }
    }

    /// Save token store to disk
    pub fn save(&self) -> crate::Result<()> {
        let store_path = Self::store_path()?;

        // Create directory if it doesn't exist
        if let Some(parent) = store_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let contents = serde_json::to_string_pretty(self)
            .map_err(|e| crate::Error::ConfigError(format!("Failed to serialize token store: {}", e)))?;

        std::fs::write(&store_path, contents)?;
        Ok(())
    }

    /// Store a token with expiration
    pub fn set_token(&mut self, platform: &str, token: &str, valid_for_days: u64) {
        let encrypted = self.encrypt(token);
        let stored_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.tokens.insert(
            platform.to_string(),
            StoredToken {
                encrypted_value: encrypted,
                stored_at,
                valid_for_seconds: valid_for_days * 24 * 60 * 60,
            },
        );
    }

    /// Get a token if it exists and hasn't expired
    pub fn get_token(&self, platform: &str) -> Option<String> {
        let stored = self.tokens.get(platform)?;

        // Check if token has expired
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if now - stored.stored_at > stored.valid_for_seconds {
            return None; // Token expired
        }

        Some(self.decrypt(&stored.encrypted_value))
    }

    /// Check if a token exists and is valid
    pub fn has_valid_token(&self, platform: &str) -> bool {
        self.get_token(platform).is_some()
    }

    /// Remove a token
    pub fn remove_token(&mut self, platform: &str) {
        self.tokens.remove(platform);
    }

    /// Get token expiration info (days remaining)
    pub fn get_token_days_remaining(&self, platform: &str) -> Option<u64> {
        let stored = self.tokens.get(platform)?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let elapsed = now - stored.stored_at;
        if elapsed > stored.valid_for_seconds {
            return Some(0); // Expired
        }

        let remaining = stored.valid_for_seconds - elapsed;
        Some(remaining / (24 * 60 * 60))
    }

    /// Clear all tokens
    pub fn clear(&mut self) {
        self.tokens.clear();
    }

    /// Get the token store file path
    fn store_path() -> crate::Result<PathBuf> {
        let data_dir = if cfg!(target_os = "windows") {
            dirs::data_dir()
                .ok_or_else(|| crate::Error::ConfigError("Could not find data directory".into()))?
                .join("reposcout")
        } else {
            // XDG data dir on Unix-like systems
            dirs::data_dir()
                .ok_or_else(|| crate::Error::ConfigError("Could not find data directory".into()))?
                .join("reposcout")
        };

        Ok(data_dir.join("tokens.json"))
    }

    /// Simple XOR encryption with machine-specific key
    /// For basic obfuscation - not cryptographically secure
    fn encrypt(&self, data: &str) -> Vec<u8> {
        let key = self.get_machine_key();
        data.bytes()
            .enumerate()
            .map(|(i, b)| b ^ key[i % key.len()])
            .collect()
    }

    /// Decrypt XOR-encrypted data
    fn decrypt(&self, data: &[u8]) -> String {
        let key = self.get_machine_key();
        let decrypted: Vec<u8> = data
            .iter()
            .enumerate()
            .map(|(i, &b)| b ^ key[i % key.len()])
            .collect();
        String::from_utf8_lossy(&decrypted).to_string()
    }

    /// Generate a machine-specific key for encryption
    /// Uses hostname + username as seed
    fn get_machine_key(&self) -> Vec<u8> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let hostname = hostname::get()
            .unwrap_or_else(|_| std::ffi::OsString::from("unknown"))
            .to_string_lossy()
            .to_string();

        let username = whoami::username();
        let seed = format!("reposcout-{}-{}", hostname, username);

        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        let hash = hasher.finish();

        // Generate 32-byte key from hash
        let mut key = Vec::with_capacity(32);
        let mut val = hash;
        for _ in 0..4 {
            key.extend_from_slice(&val.to_le_bytes());
            val = val.wrapping_mul(1103515245).wrapping_add(12345);
        }
        key
    }
}

impl Default for TokenStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_encryption() {
        let store = TokenStore::new();
        let original = "ghp_test_token_12345";

        let encrypted = store.encrypt(original);
        let decrypted = store.decrypt(&encrypted);

        assert_eq!(original, decrypted);
        assert_ne!(encrypted, original.as_bytes());
    }

    #[test]
    fn test_token_storage() {
        let mut store = TokenStore::new();

        store.set_token("github", "ghp_test_token", 30);
        assert!(store.has_valid_token("github"));

        let token = store.get_token("github");
        assert_eq!(token, Some("ghp_test_token".to_string()));
    }

    #[test]
    fn test_token_expiration() {
        let mut store = TokenStore::new();

        // Set token with 0 days validity (should expire immediately)
        store.set_token("github", "test", 0);

        // Sleep briefly to ensure time has passed
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Token should be expired
        assert!(!store.has_valid_token("github"));
    }

    #[test]
    fn test_token_removal() {
        let mut store = TokenStore::new();

        store.set_token("github", "test", 30);
        assert!(store.has_valid_token("github"));

        store.remove_token("github");
        assert!(!store.has_valid_token("github"));
    }
}
