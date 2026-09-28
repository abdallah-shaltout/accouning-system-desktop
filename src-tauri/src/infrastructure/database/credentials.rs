//! Root/app secrets for the managed server (phase-a2 A2-8, P2-49). Production uses the OS keyring
//! (same service as `core::device`); tests use a per-test keyring service so parallel test runs
//! never collide; `db_dev_server` uses a fixed, well-known password so a developer can `mariadb
//! -uroot -pequal-dev` into it without hunting through a keyring.

use crate::utils::id::Id;

/// Same service name `core::device` uses for the app-user DB password — the account naming scheme
/// (below) keeps the two uses from colliding inside one service namespace.
pub const KEYRING_SERVICE: &str = crate::core::device::KEYRING_SERVICE;

pub enum CredentialStore {
    /// Production: a real OS keyring entry, keyed by `dataDirId` so a reprovisioned data directory
    /// never reads another one's stale secret.
    Keyring { service: String },
    /// Tests only: a real keyring entry under a per-test service name, so tests never collide with
    /// each other or with the production service — removed when the test drops its `ServerPaths`.
    #[allow(dead_code)]
    TestKeyring { service: String },
    /// `db_dev_server` only: a fixed, well-known password, never read from or written to any
    /// keyring.
    Fixed { root: String },
}

impl CredentialStore {
    pub fn production() -> Self {
        Self::Keyring { service: KEYRING_SERVICE.to_string() }
    }

    /// A fresh, unique service name per test run — real keyring entries, but scoped so nothing
    /// production reads can ever see them.
    #[allow(dead_code)]
    pub fn for_test() -> Self {
        Self::TestKeyring { service: format!("{KEYRING_SERVICE}.test-{}", Id::new()) }
    }

    pub fn fixed(root: impl Into<String>) -> Self {
        Self::Fixed { root: root.into() }
    }

    /// The root secret a fresh provisioning installs: a new random 32-char secret — except for the
    /// fixed dev/test store, whose configured password must become the real root password so
    /// `EQUAL_TEST_DATABASE_URL=mysql://root:equal-dev@…` and later adoption both work.
    pub fn new_root_secret(&self) -> String {
        match self {
            CredentialStore::Fixed { root } => root.clone(),
            _ => generate_secret(32),
        }
    }

    fn account_root(data_dir_id: Id) -> String {
        format!("dbserver-root:{data_dir_id}")
    }

    fn account_lan(data_dir_id: Id) -> String {
        format!("dbserver-lan:{data_dir_id}")
    }

    fn service_name(&self) -> Option<&str> {
        match self {
            CredentialStore::Keyring { service } | CredentialStore::TestKeyring { service } => Some(service),
            CredentialStore::Fixed { .. } => None,
        }
    }

    pub fn get_root_secret(&self, data_dir_id: Id) -> Result<String, keyring::Error> {
        match self {
            CredentialStore::Fixed { root } => Ok(root.clone()),
            _ => {
                let service = self.service_name().expect("keyring variant always has a service");
                let entry = keyring::Entry::new(service, &Self::account_root(data_dir_id))?;
                entry.get_password()
            }
        }
    }

    pub fn set_root_secret(&self, data_dir_id: Id, secret: &str) -> Result<(), keyring::Error> {
        match self {
            CredentialStore::Fixed { .. } => Ok(()), // fixed password, nothing to persist
            _ => {
                let service = self.service_name().expect("keyring variant always has a service");
                let entry = keyring::Entry::new(service, &Self::account_root(data_dir_id))?;
                entry.set_password(secret)
            }
        }
    }

    #[allow(dead_code)]
    pub fn get_lan_secret(&self, data_dir_id: Id) -> Result<String, keyring::Error> {
        let service = self.service_name().ok_or(keyring::Error::NoEntry)?;
        let entry = keyring::Entry::new(service, &Self::account_lan(data_dir_id))?;
        entry.get_password()
    }

    #[allow(dead_code)]
    pub fn set_lan_secret(&self, data_dir_id: Id, secret: &str) -> Result<(), keyring::Error> {
        let service = self.service_name().ok_or(keyring::Error::NoEntry)?;
        let entry = keyring::Entry::new(service, &Self::account_lan(data_dir_id))?;
        entry.set_password(secret)
    }

    /// A3-1 `disable_lan_sharing`: removes just the LAN secret (the root secret is untouched).
    /// A missing entry is not an error — disabling is idempotent.
    #[allow(dead_code)]
    pub fn delete_lan_secret(&self, data_dir_id: Id) {
        if let Some(service) = self.service_name() {
            if let Ok(entry) = keyring::Entry::new(service, &Self::account_lan(data_dir_id)) {
                let _ = entry.delete_credential();
            }
        }
    }

    /// Removes every secret this store may have written for a data directory — used by tests
    /// (`TestKeyring`, dropped after the test) to leave no residue.
    #[allow(dead_code)]
    pub fn delete_all(&self, data_dir_id: Id) {
        if let Some(service) = self.service_name() {
            if let Ok(entry) = keyring::Entry::new(service, &Self::account_root(data_dir_id)) {
                let _ = entry.delete_credential();
            }
            if let Ok(entry) = keyring::Entry::new(service, &Self::account_lan(data_dir_id)) {
                let _ = entry.delete_credential();
            }
        }
    }
}

/// Crockford base32 alphabet (`0-9A-HJKMNP-TV-Z`) — no `I`, `L`, `O`, `U`, so a generated secret
/// never needs URL escaping and is unambiguous to read aloud (P2-49).
const CROCKFORD_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Generates a random secret of `len` characters from the Crockford base32 alphabet using
/// `getrandom` (OS CSPRNG) — never `rand`'s thread-local RNG, so this compiles and behaves
/// identically whether or not that crate is in the dependency tree.
pub fn generate_secret(len: usize) -> String {
    let mut bytes = vec![0u8; len];
    getrandom::fill(&mut bytes).expect("OS CSPRNG must be available");
    bytes.iter().map(|b| CROCKFORD_ALPHABET[(*b as usize) % CROCKFORD_ALPHABET.len()] as char).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_secret_has_expected_length_and_alphabet() {
        let secret = generate_secret(32);
        assert_eq!(secret.len(), 32);
        assert!(secret.chars().all(|c| CROCKFORD_ALPHABET.contains(&(c as u8))));
        // No character needs URL escaping.
        assert!(secret.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn generated_secrets_are_not_all_identical() {
        let a = generate_secret(24);
        let b = generate_secret(24);
        assert_ne!(a, b);
    }

    #[test]
    fn fixed_store_always_returns_the_configured_password() {
        let store = CredentialStore::fixed("equal-dev");
        let id = Id::new();
        assert_eq!(store.get_root_secret(id).unwrap(), "equal-dev");
    }

    #[test]
    fn test_keyring_round_trips_and_cleans_up() {
        let store = CredentialStore::for_test();
        let id = Id::new();
        store.set_root_secret(id, "s3cr3t-test-value").unwrap();
        assert_eq!(store.get_root_secret(id).unwrap(), "s3cr3t-test-value");
        store.delete_all(id);
        assert!(store.get_root_secret(id).is_err());
    }
}
