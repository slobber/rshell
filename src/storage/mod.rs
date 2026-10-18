use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{params, Connection, OptionalExtension};

#[derive(Clone, Debug, Default)]
pub struct SecurityRecord {
    pub password_salt: Option<Vec<u8>>,
    pub password_wrap: Option<Vec<u8>>,
    pub keyring_wrap: Option<Vec<u8>>,
    pub remember_until: Option<DateTime<Utc>>,
}

impl SecurityRecord {
    pub fn has_password(&self) -> bool {
        self.password_salt.is_some() && self.password_wrap.is_some()
    }
}

mod hosts;
mod keys;
mod known_hosts;

#[derive(Clone, Debug)]
pub struct SavedKeySummary {
    pub id: u64,
    pub name: String,
    pub fingerprint: String,
    pub created_at: DateTime<Utc>,
}

pub struct Persistence {
    pub path: PathBuf,
    conn: Connection,
}

impl Persistence {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("Could not create {}: {error}", parent.display()))?;
        }
        let conn = Connection::open(&path)
            .map_err(|error| format!("Could not open {}: {error}", path.display()))?;
        conn.busy_timeout(std::time::Duration::from_secs(3))
            .map_err(|error| error.to_string())?;
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;",
        )
        .map_err(|error| error.to_string())?;

        let mut persistence = Self { path, conn };
        persistence.migrate()?;
        persistence.ensure_security_row()?;
        persistence.secure_file_permissions()?;
        Ok(persistence)
    }

    fn migrate(&mut self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS app_security (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    password_salt BLOB,
                    password_wrap BLOB,
                    keyring_wrap BLOB,
                    remember_until INTEGER,
                    updated_at INTEGER NOT NULL
                 );
                 CREATE TABLE IF NOT EXISTS host_folders (
                    id INTEGER PRIMARY KEY,
                    parent_id INTEGER REFERENCES host_folders(id) ON DELETE CASCADE,
                    name TEXT NOT NULL,
                    subtitle TEXT NOT NULL DEFAULT '',
                    expanded INTEGER NOT NULL DEFAULT 1,
                    sort_order INTEGER NOT NULL DEFAULT 0
                 );
                 CREATE TABLE IF NOT EXISTS hosts (
                    id INTEGER PRIMARY KEY,
                    folder_id INTEGER NOT NULL REFERENCES host_folders(id) ON DELETE CASCADE,
                    name TEXT NOT NULL,
                    ip TEXT NOT NULL,
                    port INTEGER NOT NULL,
                    auth_type TEXT NOT NULL,
                    username_enc BLOB,
                    secret_enc BLOB,
                    gateway TEXT NOT NULL DEFAULT '',
                    environment TEXT NOT NULL DEFAULT '',
                    keep_alive INTEGER NOT NULL DEFAULT 30,
                    backend_type TEXT NOT NULL DEFAULT 'ssh',
                    wsl_distro TEXT,
                    key_source TEXT NOT NULL DEFAULT 'filesystem',
                    key_id INTEGER,
                    os_kind TEXT NOT NULL DEFAULT 'unknown',
                    sort_order INTEGER NOT NULL DEFAULT 0
                 );
                 CREATE INDEX IF NOT EXISTS idx_hosts_folder ON hosts(folder_id);
                 CREATE TABLE IF NOT EXISTS saved_keys (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    fingerprint TEXT NOT NULL DEFAULT '',
                    private_key_enc BLOB NOT NULL,
                    created_at INTEGER NOT NULL
                 );
                 CREATE TABLE IF NOT EXISTS known_hosts (
                    host_key TEXT PRIMARY KEY,
                    fingerprint TEXT NOT NULL,
                    updated_at INTEGER NOT NULL
                 );
                 CREATE TABLE IF NOT EXISTS app_settings (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                 );
                 PRAGMA user_version = 1;",
            )
            .map_err(|error| error.to_string())?;
        self.ensure_column("hosts", "backend_type", "TEXT NOT NULL DEFAULT 'ssh'")?;
        self.ensure_column("hosts", "wsl_distro", "TEXT")?;
        self.ensure_column("hosts", "key_source", "TEXT NOT NULL DEFAULT 'filesystem'")?;
        self.ensure_column("hosts", "key_id", "INTEGER")?;
        self.ensure_column("hosts", "os_kind", "TEXT NOT NULL DEFAULT 'unknown'")?;
        self.conn
            .execute_batch("PRAGMA user_version = 5;")
            .map_err(|error| error.to_string())
    }

    fn ensure_column(&self, table: &str, column: &str, definition: &str) -> Result<(), String> {
        let mut statement = self
            .conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .map_err(|error| error.to_string())?;
        let columns = statement
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|error| error.to_string())?;
        let mut exists = false;
        for name in columns {
            if name.map_err(|error| error.to_string())? == column {
                exists = true;
                break;
            }
        }
        drop(statement);
        if !exists {
            self.conn
                .execute(
                    &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
                    [],
                )
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn ensure_security_row(&self) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO app_security
                    (id, password_salt, password_wrap, keyring_wrap, remember_until, updated_at)
                 VALUES (1, NULL, NULL, NULL, NULL, ?1)",
                params![Utc::now().timestamp()],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    #[cfg(unix)]
    fn secure_file_permissions(&self) -> Result<(), String> {
        use std::os::unix::fs::PermissionsExt;
        let permissions = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&self.path, permissions)
            .map_err(|error| format!("Could not secure database permissions: {error}"))
    }

    #[cfg(not(unix))]
    fn secure_file_permissions(&self) -> Result<(), String> {
        Ok(())
    }

    pub fn load_security(&self) -> Result<SecurityRecord, String> {
        self.conn
            .query_row(
                "SELECT password_salt, password_wrap, keyring_wrap, remember_until
                 FROM app_security WHERE id = 1",
                [],
                |row| {
                    let remember_until: Option<i64> = row.get(3)?;
                    Ok(SecurityRecord {
                        password_salt: row.get(0)?,
                        password_wrap: row.get(1)?,
                        keyring_wrap: row.get(2)?,
                        remember_until: remember_until
                            .and_then(|timestamp| Utc.timestamp_opt(timestamp, 0).single()),
                    })
                },
            )
            .optional()
            .map_err(|error| error.to_string())
            .map(Option::unwrap_or_default)
    }

    pub fn save_security(&mut self, record: &SecurityRecord) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE app_security SET
                    password_salt = ?1,
                    password_wrap = ?2,
                    keyring_wrap = ?3,
                    remember_until = ?4,
                    updated_at = ?5
                 WHERE id = 1",
                params![
                    record.password_salt,
                    record.password_wrap,
                    record.keyring_wrap,
                    record.remember_until.map(|value| value.timestamp()),
                    Utc::now().timestamp()
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn load_settings(&self) -> Result<HashMap<String, String>, String> {
        let mut statement = self
            .conn
            .prepare("SELECT key, value FROM app_settings")
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| error.to_string())?;
        let mut settings = HashMap::new();
        for row in rows {
            let (key, value) = row.map_err(|error| error.to_string())?;
            settings.insert(key, value);
        }
        Ok(settings)
    }

    pub fn save_settings(&mut self, settings: &HashMap<String, String>) -> Result<(), String> {
        let transaction = self.conn.transaction().map_err(|error| error.to_string())?;
        for (key, value) in settings {
            transaction
                .execute(
                    "INSERT INTO app_settings(key, value) VALUES(?1, ?2)
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                    params![key, value],
                )
                .map_err(|error| error.to_string())?;
        }
        transaction.commit().map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AuthKind, ConnectionBackend, Host, HostFolder, HostStatus, KeySource};
    use crate::security::DataCipher;

    #[test]
    fn sensitive_host_fields_are_not_stored_as_plaintext_utf8() {
        let unique = format!(
            "rshell-test-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );
        let path = std::env::temp_dir().join(unique).join("test.db");
        let cipher = DataCipher::new([3u8; 32]);
        let mut folder = HostFolder::new(1, "folder");
        folder.hosts.push(Host {
            id: 2,
            folder_id: 1,
            name: "visible-host-name".to_owned(),
            ip: "10.0.0.2".to_owned(),
            port: 22,
            user: "secret-user-value".to_owned(),
            auth: AuthKind::Password,
            key_source: KeySource::FileSystem,
            key_id: None,
            secret: "secret-password-value".to_owned(),
            gateway: "direct".to_owned(),
            environment: "TEST".to_owned(),
            keep_alive: 30,
            backend: ConnectionBackend::Ssh,
            os: crate::domain::HostOs::Ubuntu,
            wsl_distro: None,
            status: HostStatus::Offline,
            latency_ms: None,
            credentials_unavailable: false,
        });
        {
            let mut persistence = Persistence::open(&path).unwrap();
            persistence.save_host_tree(&[folder], &cipher).unwrap();
        }
        let bytes = std::fs::read(&path).unwrap();
        let as_text = String::from_utf8_lossy(&bytes);
        assert!(as_text.contains("visible-host-name"));
        assert!(as_text.contains("10.0.0.2"));
        assert!(!as_text.contains("secret-user-value"));
        assert!(!as_text.contains("secret-password-value"));
        let persistence = Persistence::open(&path).unwrap();
        let loaded = persistence.load_host_tree(&cipher).unwrap();
        assert_eq!(loaded[0].hosts[0].user, "secret-user-value");
        assert_eq!(loaded[0].hosts[0].secret, "secret-password-value");
        assert_eq!(loaded[0].hosts[0].os, crate::domain::HostOs::Ubuntu);
        let _ = std::fs::remove_dir_all(path.parent().unwrap().to_path_buf());
    }

    #[test]
    fn managed_private_keys_are_encrypted_and_listed() {
        let unique = format!(
            "rshell-key-test-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );
        let path = std::env::temp_dir().join(unique).join("test.db");
        let cipher = DataCipher::new([5u8; 32]);
        let pem = "-----BEGIN OPENSSH PRIVATE KEY-----\nsecret-key-material\n-----END OPENSSH PRIVATE KEY-----";
        let mut persistence = Persistence::open(&path).unwrap();
        let id = persistence
            .add_saved_key(&cipher, "deploy key", pem)
            .unwrap();
        let keys = persistence.list_saved_keys().unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].id, id);
        assert_eq!(keys[0].name, "deploy key");
        let bytes = std::fs::read(&path).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("secret-key-material"));
        persistence.delete_saved_key(id).unwrap();
        assert!(persistence.list_saved_keys().unwrap().is_empty());
        drop(persistence);
        let _ = std::fs::remove_dir_all(path.parent().unwrap().to_path_buf());
    }
}
