use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

use argon2::Argon2;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use chrono::{DateTime, Utc};

use crate::domain::RememberDuration;
use crate::storage::SecurityRecord;

const KEY_LEN: usize = 32;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEYRING_SERVICE: &str = "RShell Local Vault";

#[derive(Clone, Debug)]
pub struct DataCipher {
    key: [u8; KEY_LEN],
}

impl DataCipher {
    pub fn new(key: [u8; KEY_LEN]) -> Self {
        Self { key }
    }

    pub fn key(&self) -> [u8; KEY_LEN] {
        self.key
    }
}

#[derive(Debug)]
pub enum VaultOpen {
    Unlocked(DataCipher),
    Locked,
    CredentialsLost,
}

pub fn initialize_or_unlock(
    record: &mut SecurityRecord,
    db_path: &Path,
) -> Result<VaultOpen, String> {
    if record.keyring_wrap.is_none() && record.password_wrap.is_none() {
        let key = random_key()?;
        wrap_with_keyring(record, &key, db_path)?;
        record.remember_until = Some(Utc::now() + chrono::Duration::days(365 * 100));
        return Ok(VaultOpen::Unlocked(DataCipher::new(key)));
    }

    if record.has_password() {
        let remember_valid = record
            .remember_until
            .is_some_and(|expiry| expiry > Utc::now());
        if remember_valid {
            if let Some(key) = unwrap_with_keyring(record, db_path) {
                return Ok(VaultOpen::Unlocked(DataCipher::new(key)));
            }
        }
        return Ok(VaultOpen::Locked);
    }

    match unwrap_with_keyring(record, db_path) {
        Some(key) => Ok(VaultOpen::Unlocked(DataCipher::new(key))),
        None => Ok(VaultOpen::CredentialsLost),
    }
}

pub fn unlock_with_password(record: &SecurityRecord, password: &str) -> Result<DataCipher, String> {
    let salt = record
        .password_salt
        .as_deref()
        .ok_or_else(|| "No password is configured".to_owned())?;
    let wrapped = record
        .password_wrap
        .as_deref()
        .ok_or_else(|| "No password wrapper is configured".to_owned())?;
    let key = derive_password_key(password, salt)?;
    let data_key =
        open_secret_bytes(&key, wrapped).ok_or_else(|| "Incorrect password".to_owned())?;
    let data_key: [u8; KEY_LEN] = data_key
        .try_into()
        .map_err(|_| "Invalid wrapped data key".to_owned())?;
    Ok(DataCipher::new(data_key))
}

pub fn set_password(
    record: &mut SecurityRecord,
    db_path: &Path,
    cipher: &DataCipher,
    password: &str,
    remember: RememberDuration,
) -> Result<(), String> {
    if password.chars().count() < 6 {
        return Err("Password must contain at least 6 characters".to_owned());
    }
    let salt = random_bytes::<SALT_LEN>()?;
    let password_key = derive_password_key(password, &salt)?;
    record.password_salt = Some(salt.to_vec());
    record.password_wrap = Some(seal_secret_bytes(&password_key, &cipher.key()));

    if remember == RememberDuration::None {
        delete_keyring_secret(db_path);
        record.keyring_wrap = None;
        record.remember_until = None;
    } else {
        apply_remember(record, db_path, cipher, remember)?;
    }
    Ok(())
}

pub fn remove_password(
    record: &mut SecurityRecord,
    db_path: &Path,
    cipher: &DataCipher,
) -> Result<(), String> {
    wrap_with_keyring(record, &cipher.key(), db_path)?;
    record.password_salt = None;
    record.password_wrap = None;
    record.remember_until = Some(Utc::now() + chrono::Duration::days(365 * 100));
    Ok(())
}

pub fn apply_remember(
    record: &mut SecurityRecord,
    db_path: &Path,
    cipher: &DataCipher,
    remember: RememberDuration,
) -> Result<(), String> {
    if remember == RememberDuration::None {
        delete_keyring_secret(db_path);
        record.keyring_wrap = None;
        record.remember_until = None;
        return Ok(());
    }
    wrap_with_keyring(record, &cipher.key(), db_path)?;
    record.remember_until = remember.expiry(Utc::now());
    Ok(())
}

pub fn reset_credentials(
    record: &mut SecurityRecord,
    db_path: &Path,
) -> Result<DataCipher, String> {
    let key = random_key()?;
    record.password_salt = None;
    record.password_wrap = None;
    wrap_with_keyring(record, &key, db_path)?;
    record.remember_until = Some(Utc::now() + chrono::Duration::days(365 * 100));
    Ok(DataCipher::new(key))
}

fn derive_password_key(password: &str, salt: &[u8]) -> Result<[u8; KEY_LEN], String> {
    let mut key = [0u8; KEY_LEN];
    Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|error| format!("Could not derive password key: {error}"))?;
    Ok(key)
}

pub fn seal_secret(cipher: &DataCipher, plaintext: &[u8]) -> Vec<u8> {
    seal_secret_bytes(&cipher.key, plaintext)
}

pub fn open_secret(cipher: &DataCipher, blob: &[u8]) -> Result<Vec<u8>, String> {
    open_secret_bytes(&cipher.key, blob).ok_or_else(|| "Could not decrypt local secret".to_owned())
}

fn seal_secret_bytes(key: &[u8; KEY_LEN], plaintext: &[u8]) -> Vec<u8> {
    let cipher = ChaCha20Poly1305::new(
        &Key::try_from(key.as_slice()).expect("ChaCha20-Poly1305 key must be 32 bytes"),
    );
    let nonce_bytes = random_bytes::<NONCE_LEN>().unwrap_or([7u8; NONCE_LEN]);
    let nonce =
        Nonce::try_from(nonce_bytes.as_slice()).expect("ChaCha20-Poly1305 nonce must be 12 bytes");
    let encrypted = cipher
        .encrypt(&nonce, plaintext)
        .expect("ChaCha20-Poly1305 encryption cannot fail for valid buffers");
    let mut blob = Vec::with_capacity(NONCE_LEN + encrypted.len());
    blob.extend_from_slice(&nonce_bytes);
    blob.extend_from_slice(&encrypted);
    blob
}

fn open_secret_bytes(key: &[u8; KEY_LEN], blob: &[u8]) -> Option<Vec<u8>> {
    if blob.len() <= NONCE_LEN {
        return None;
    }
    let (nonce_bytes, ciphertext) = blob.split_at(NONCE_LEN);
    let cipher = ChaCha20Poly1305::new(
        &Key::try_from(key.as_slice()).expect("ChaCha20-Poly1305 key must be 32 bytes"),
    );
    let nonce = Nonce::try_from(nonce_bytes).ok()?;
    cipher.decrypt(&nonce, ciphertext).ok()
}

fn random_key() -> Result<[u8; KEY_LEN], String> {
    random_bytes::<KEY_LEN>()
}

fn random_bytes<const N: usize>() -> Result<[u8; N], String> {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes)
        .map_err(|error| format!("Could not obtain secure randomness: {error}"))?;
    Ok(bytes)
}

fn wrap_with_keyring(
    record: &mut SecurityRecord,
    data_key: &[u8; KEY_LEN],
    db_path: &Path,
) -> Result<(), String> {
    let secret = random_key()?;
    let wrapped = seal_secret_bytes(&secret, data_key);
    let entry = keyring_entry(db_path)?;
    entry
        .set_password(&hex_encode(&secret))
        .map_err(|error| format!("Could not store auto-unlock secret: {error}"))?;
    record.keyring_wrap = Some(wrapped);
    Ok(())
}

fn unwrap_with_keyring(record: &SecurityRecord, db_path: &Path) -> Option<[u8; KEY_LEN]> {
    let wrapped = record.keyring_wrap.as_deref()?;
    let entry = keyring_entry(db_path).ok()?;
    let encoded = entry.get_password().ok()?;
    let secret = hex_decode(&encoded)?;
    if secret.len() != KEY_LEN {
        return None;
    }
    let secret: [u8; KEY_LEN] = secret.try_into().ok()?;
    let key = open_secret_bytes(&secret, wrapped)?;
    key.try_into().ok()
}

fn delete_keyring_secret(db_path: &Path) {
    if let Ok(entry) = keyring_entry(db_path) {
        let _ = entry.delete_credential();
    }
}

fn keyring_entry(db_path: &Path) -> Result<keyring::Entry, String> {
    let mut hasher = DefaultHasher::new();
    db_path.to_string_lossy().hash(&mut hasher);
    let account = format!("vault-{:016x}", hasher.finish());
    keyring::Entry::new(KEYRING_SERVICE, &account)
        .map_err(|error| format!("Could not open OS credential store: {error}"))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn hex_decode(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    let mut bytes = Vec::with_capacity(text.len() / 2);
    for pair in text.as_bytes().as_chunks::<2>().0 {
        let high = (pair[0] as char).to_digit(16)?;
        let low = (pair[1] as char).to_digit(16)?;
        bytes.push(((high << 4) | low) as u8);
    }
    Some(bytes)
}

pub fn remember_expiry_label(until: Option<DateTime<Utc>>) -> String {
    match until {
        Some(until) if until > Utc::now() => until.format("%Y-%m-%d %H:%M").to_string(),
        _ => "expired".to_owned(),
    }
}

pub fn validate_password(password: &str, confirmation: &str) -> Result<(), String> {
    if password.chars().count() < 6 {
        return Err("Password must contain at least 6 characters".to_owned());
    }
    if password != confirmation {
        return Err("Passwords do not match".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypted_secret_round_trips_and_rejects_wrong_key() {
        let cipher = DataCipher::new([7u8; KEY_LEN]);
        let blob = seal_secret(&cipher, b"root credential");
        assert_eq!(open_secret(&cipher, &blob).unwrap(), b"root credential");
        let other = DataCipher::new([8u8; KEY_LEN]);
        assert!(open_secret(&other, &blob).is_err());
    }

    #[test]
    fn password_wrap_accepts_only_the_correct_password() {
        let cipher = DataCipher::new([9u8; KEY_LEN]);
        let mut record = SecurityRecord::default();
        set_password(
            &mut record,
            &std::path::PathBuf::from("test.db"),
            &cipher,
            "correct-horse",
            RememberDuration::None,
        )
        .unwrap();
        assert!(unlock_with_password(&record, "correct-horse").is_ok());
        assert!(unlock_with_password(&record, "wrong-password").is_err());
        assert!(record.keyring_wrap.is_none());
    }
}
