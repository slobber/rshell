use std::collections::HashMap;

use chrono::Utc;
use rusqlite::params;

use super::Persistence;

impl Persistence {
    pub fn load_known_host_keys(&self) -> Result<HashMap<String, String>, String> {
        let mut statement = self
            .conn
            .prepare("SELECT host_key, fingerprint FROM known_hosts")
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| error.to_string())?;
        let mut keys = HashMap::new();
        for row in rows {
            let (host_key, fingerprint) = row.map_err(|error| error.to_string())?;
            keys.insert(host_key, fingerprint);
        }
        Ok(keys)
    }

    pub fn save_known_host_key(&mut self, host_key: &str, fingerprint: &str) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO known_hosts(host_key, fingerprint, updated_at)
                 VALUES(?1, ?2, ?3)
                 ON CONFLICT(host_key) DO UPDATE SET
                    fingerprint = excluded.fingerprint,
                    updated_at = excluded.updated_at",
                params![host_key, fingerprint, Utc::now().timestamp()],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}
