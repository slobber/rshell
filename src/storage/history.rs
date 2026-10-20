use chrono::{DateTime, TimeZone, Utc};
use rusqlite::params;

use super::Persistence;
use crate::security::{open_secret, seal_secret, DataCipher};

#[derive(Clone, Debug)]
pub struct CommandHistoryEntry {
    pub id: i64,
    pub session_uid: String,
    pub host_ip: String,
    pub user: String,
    pub cwd: String,
    pub command: String,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_history_round_trips_encrypted_and_prunes_per_session() {
        let unique = format!(
            "rshell-history-test-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );
        let path = std::env::temp_dir().join(unique).join("test.db");
        let cipher = DataCipher::new([6_u8; 32]);
        let mut persistence = Persistence::open(&path).unwrap();
        for index in 0..3 {
            persistence
                .append_command_history(
                    &cipher,
                    &CommandHistoryRecord {
                        session_uid: "session-a".to_owned(),
                        host_ip: "10.0.0.8".to_owned(),
                        user: "root".to_owned(),
                        cwd: "/srv".to_owned(),
                        command: format!("secret-command-{index}"),
                    },
                    2,
                )
                .unwrap();
        }
        let bytes = std::fs::read(&path).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("secret-command"));
        let entries = persistence.load_command_history(&cipher).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].command, "secret-command-1");
        assert_eq!(entries[1].command, "secret-command-2");
        assert_eq!(entries[1].session_uid, "session-a");
        assert_eq!(entries[1].cwd, "/srv");
        drop(persistence);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn deletes_only_the_selected_session() {
        let unique = format!(
            "rshell-history-delete-test-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );
        let path = std::env::temp_dir().join(unique).join("test.db");
        let cipher = DataCipher::new([9_u8; 32]);
        let mut persistence = Persistence::open(&path).unwrap();
        for (session_uid, command) in [
            ("session-a", "df -h"),
            ("session-a", "free -h"),
            ("session-b", "uptime"),
        ] {
            persistence
                .append_command_history(
                    &cipher,
                    &CommandHistoryRecord {
                        session_uid: session_uid.to_owned(),
                        host_ip: "10.0.0.8".to_owned(),
                        user: "root".to_owned(),
                        cwd: "/".to_owned(),
                        command: command.to_owned(),
                    },
                    100,
                )
                .unwrap();
        }

        persistence
            .delete_command_history_session("session-a")
            .unwrap();
        let entries = persistence.load_command_history(&cipher).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].session_uid, "session-b");
        assert_eq!(entries[0].command, "uptime");

        drop(persistence);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}

#[derive(Clone, Debug)]
pub struct CommandHistoryRecord {
    pub session_uid: String,
    pub host_ip: String,
    pub user: String,
    pub cwd: String,
    pub command: String,
}

impl Persistence {
    pub fn append_command_history(
        &mut self,
        cipher: &DataCipher,
        record: &CommandHistoryRecord,
        per_session_limit: usize,
    ) -> Result<(), String> {
        let command = seal_secret(cipher, record.command.as_bytes());
        let now = Utc::now().timestamp();
        self.conn
            .execute(
                "INSERT INTO command_history
                    (session_uid, host_ip, username, cwd, command_enc, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    record.session_uid,
                    record.host_ip,
                    record.user,
                    record.cwd,
                    command,
                    now
                ],
            )
            .map_err(|error| error.to_string())?;
        self.conn
            .execute(
                "DELETE FROM command_history
                 WHERE session_uid = ?1
                   AND id NOT IN (
                       SELECT id FROM command_history
                       WHERE session_uid = ?1
                       ORDER BY id DESC
                       LIMIT ?2
                   )",
                params![record.session_uid, per_session_limit.max(1) as i64],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn load_command_history(
        &self,
        cipher: &DataCipher,
    ) -> Result<Vec<CommandHistoryEntry>, String> {
        let mut statement = self
            .conn
            .prepare(
                "SELECT id, session_uid, host_ip, username, cwd, command_enc, created_at
                 FROM command_history ORDER BY id",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Vec<u8>>(5)?,
                    row.get::<_, i64>(6)?,
                ))
            })
            .map_err(|error| error.to_string())?;
        let mut entries = Vec::new();
        for row in rows {
            let (id, session_uid, host_ip, user, cwd, command_enc, created_at) =
                row.map_err(|error| error.to_string())?;
            let Some(command) = open_secret(cipher, &command_enc)
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())
            else {
                continue;
            };
            let Some(created_at) = Utc.timestamp_opt(created_at, 0).single() else {
                continue;
            };
            entries.push(CommandHistoryEntry {
                id,
                session_uid,
                host_ip,
                user,
                cwd,
                command,
                created_at,
            });
        }
        Ok(entries)
    }

    pub fn delete_command_history_session(&mut self, session_uid: &str) -> Result<(), String> {
        self.conn
            .execute(
                "DELETE FROM command_history WHERE session_uid = ?1",
                params![session_uid],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}
