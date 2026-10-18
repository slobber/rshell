use chrono::{TimeZone, Utc};
use rusqlite::params;

use crate::security::{seal_secret, DataCipher};

use super::{Persistence, SavedKeySummary};

impl Persistence {
    pub fn list_saved_keys(&self) -> Result<Vec<SavedKeySummary>, String> {
        let mut statement = self
            .conn
            .prepare(
                "SELECT id, name, fingerprint, created_at
                 FROM saved_keys ORDER BY created_at DESC, id DESC",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                let created = row.get::<_, i64>(3)?;
                Ok(SavedKeySummary {
                    id: row.get::<_, i64>(0)? as u64,
                    name: row.get(1)?,
                    fingerprint: row.get(2)?,
                    created_at: Utc
                        .timestamp_opt(created, 0)
                        .single()
                        .unwrap_or_else(Utc::now),
                })
            })
            .map_err(|error| error.to_string())?;
        rows.map(|row| row.map_err(|error| error.to_string()))
            .collect()
    }

    pub fn add_saved_key(
        &mut self,
        cipher: &DataCipher,
        name: &str,
        private_key_pem: &str,
    ) -> Result<u64, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Key name is required".to_owned());
        }
        if !private_key_pem.contains("PRIVATE KEY") {
            return Err("The selected file does not look like a private key".to_owned());
        }
        let encrypted = seal_secret(cipher, private_key_pem.as_bytes());
        let fingerprint = local_fingerprint(private_key_pem);
        self.conn
            .execute(
                "INSERT INTO saved_keys(name, fingerprint, private_key_enc, created_at)
                 VALUES(?1, ?2, ?3, ?4)",
                params![name, fingerprint, encrypted, Utc::now().timestamp()],
            )
            .map_err(|error| error.to_string())?;
        Ok(self.conn.last_insert_rowid() as u64)
    }

    pub fn delete_saved_key(&mut self, id: u64) -> Result<(), String> {
        let transaction = self.conn.transaction().map_err(|error| error.to_string())?;
        transaction
            .execute(
                "UPDATE hosts SET key_id = NULL WHERE key_id = ?1",
                params![id as i64],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute("DELETE FROM saved_keys WHERE id = ?1", params![id as i64])
            .map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())
    }

    pub fn load_saved_key(&self, cipher: &DataCipher, id: u64) -> Result<String, String> {
        let encrypted = self
            .conn
            .query_row(
                "SELECT private_key_enc FROM saved_keys WHERE id = ?1",
                params![id as i64],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .map_err(|error| format!("Could not load managed key: {error}"))?;
        let bytes = crate::security::open_secret(cipher, &encrypted)
            .map_err(|error| format!("Could not decrypt managed key: {error}"))?;
        String::from_utf8(bytes).map_err(|error| format!("Managed key is not UTF-8: {error}"))
    }
}

fn local_fingerprint(private_key_pem: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in private_key_pem.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("LOCAL:{hash:016X}")
}
