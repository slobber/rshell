use std::collections::HashMap;

use super::Persistence;
use crate::domain::{
    AuthKind, ConnectionBackend, Host, HostFolder, HostStatus, KeySource, RdpConnectionMode,
};
use crate::security::{open_secret, seal_secret, DataCipher};
use rusqlite::params;

impl Persistence {
    pub fn load_host_tree(&self, cipher: &DataCipher) -> Result<Vec<HostFolder>, String> {
        let mut folder_statement = self
            .conn
            .prepare(
                "SELECT id, parent_id, name, subtitle, expanded, sort_order
                 FROM host_folders ORDER BY sort_order, id",
            )
            .map_err(|error| error.to_string())?;
        let folder_rows = folder_statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)? as u64,
                    row.get::<_, Option<i64>>(1)?.map(|id| id as u64),
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)? != 0,
                    row.get::<_, i64>(5)?,
                ))
            })
            .map_err(|error| error.to_string())?;

        let mut folders: HashMap<u64, HostFolder> = HashMap::new();
        let mut order = Vec::new();
        for row in folder_rows {
            let (id, parent_id, title, subtitle, expanded, sort_order) =
                row.map_err(|error| error.to_string())?;
            folders.insert(
                id,
                HostFolder {
                    id,
                    parent_id,
                    title,
                    subtitle,
                    expanded,
                    hosts: Vec::new(),
                    children: Vec::new(),
                    sort_order,
                },
            );
            order.push(id);
        }

        let mut host_statement = self
            .conn
            .prepare(
                "SELECT id, folder_id, name, ip, port, auth_type,
                        username_enc, secret_enc, gateway, environment, keep_alive,
                        backend_type, wsl_distro, key_source, key_id, os_kind, rdp_mode
                 FROM hosts ORDER BY sort_order, id",
            )
            .map_err(|error| error.to_string())?;
        let host_rows = host_statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)? as u64,
                    row.get::<_, i64>(1)? as u64,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)? as u16,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<Vec<u8>>>(6)?,
                    row.get::<_, Option<Vec<u8>>>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, i64>(10)? as u32,
                    row.get::<_, String>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, String>(13)?,
                    row.get::<_, Option<i64>>(14)?,
                    row.get::<_, String>(15)?,
                    row.get::<_, Option<i64>>(16)?,
                ))
            })
            .map_err(|error| error.to_string())?;
        for row in host_rows {
            let (
                id,
                folder_id,
                name,
                ip,
                port,
                auth_type,
                username_enc,
                secret_enc,
                gateway,
                environment,
                keep_alive,
                backend_type,
                wsl_distro,
                key_source,
                key_id,
                os_kind,
                rdp_mode,
            ) = row.map_err(|error| error.to_string())?;
            let auth = AuthKind::parse(&auth_type);
            let key_source = KeySource::parse(&key_source);
            let backend = ConnectionBackend::parse(&backend_type);
            let port = if backend_type == "psrp" && matches!(port, 5985 | 5986) {
                3389
            } else {
                port
            };
            let user = username_enc
                .and_then(|blob| open_secret(cipher, &blob).ok())
                .and_then(|bytes| String::from_utf8(bytes).ok());
            let secret = secret_enc
                .and_then(|blob| open_secret(cipher, &blob).ok())
                .and_then(|bytes| String::from_utf8(bytes).ok());
            let credentials_available = match auth {
                AuthKind::Agent => user.is_some(),
                AuthKind::Key if key_source == KeySource::Managed => {
                    user.is_some() && key_id.is_some()
                }
                _ => user.is_some() && secret.is_some(),
            };
            if let Some(folder) = folders.get_mut(&folder_id) {
                folder.hosts.push(Host {
                    id,
                    folder_id,
                    name,
                    ip,
                    port,
                    user: user.unwrap_or_default(),
                    auth,
                    key_source,
                    key_id: key_id.map(|id| id as u64),
                    secret: secret.unwrap_or_default(),
                    gateway,
                    environment,
                    keep_alive,
                    backend,
                    os: crate::domain::HostOs::parse(&os_kind),
                    wsl_distro,
                    status: HostStatus::Offline,
                    latency_ms: None,
                    rdp_mode: RdpConnectionMode::from_db(rdp_mode),
                    credentials_unavailable: !credentials_available,
                });
            }
        }

        let parent_map: HashMap<u64, Option<u64>> = folders
            .iter()
            .map(|(id, folder)| (*id, folder.parent_id))
            .collect();
        let mut children_map: HashMap<Option<u64>, Vec<u64>> = HashMap::new();
        for id in order {
            children_map
                .entry(parent_map.get(&id).copied().flatten())
                .or_default()
                .push(id);
        }

        fn build(
            id: u64,
            folders: &mut HashMap<u64, HostFolder>,
            children_map: &HashMap<Option<u64>, Vec<u64>>,
        ) -> HostFolder {
            let mut folder = folders
                .remove(&id)
                .unwrap_or_else(|| HostFolder::new(id, "Folder"));
            if let Some(children) = children_map.get(&Some(id)) {
                folder.children = children
                    .iter()
                    .map(|child| build(*child, folders, children_map))
                    .collect();
            }
            folder
        }

        let mut roots = children_map
            .get(&None)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|id| build(id, &mut folders, &children_map))
            .collect::<Vec<_>>();
        roots.sort_by_key(|folder| folder.sort_order);
        Ok(roots)
    }

    pub fn save_host_tree(
        &mut self,
        folders: &[HostFolder],
        cipher: &DataCipher,
    ) -> Result<(), String> {
        let transaction = self.conn.transaction().map_err(|error| error.to_string())?;
        transaction
            .execute("DELETE FROM hosts", [])
            .map_err(|error| error.to_string())?;
        transaction
            .execute("DELETE FROM host_folders", [])
            .map_err(|error| error.to_string())?;

        fn insert_folder(
            transaction: &rusqlite::Transaction<'_>,
            folder: &HostFolder,
            parent_id: Option<u64>,
            cipher: &DataCipher,
        ) -> Result<(), String> {
            transaction
                .execute(
                    "INSERT INTO host_folders
                        (id, parent_id, name, subtitle, expanded, sort_order)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        folder.id as i64,
                        parent_id.map(|id| id as i64),
                        folder.title,
                        folder.subtitle,
                        i64::from(folder.expanded),
                        folder.sort_order
                    ],
                )
                .map_err(|error| error.to_string())?;
            for (index, host) in folder.hosts.iter().enumerate() {
                let username = if host.user.is_empty() && host.secret.is_empty() {
                    None
                } else {
                    Some(seal_secret(cipher, host.user.as_bytes()))
                };
                let secret = if host.secret.is_empty() {
                    None
                } else {
                    Some(seal_secret(cipher, host.secret.as_bytes()))
                };
                transaction
                    .execute(
                        "INSERT INTO hosts
                            (id, folder_id, name, ip, port, auth_type, username_enc,
                             secret_enc, gateway, environment, keep_alive,
                             backend_type, wsl_distro, key_source, key_id,
                             os_kind, rdp_mode, sort_order)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                                 ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
                        params![
                            host.id as i64,
                            folder.id as i64,
                            host.name,
                            host.ip,
                            i64::from(host.port),
                            host.auth.as_str(),
                            username,
                            secret,
                            host.gateway,
                            host.environment,
                            i64::from(host.keep_alive),
                            host.backend.as_str(),
                            host.wsl_distro,
                            host.key_source.as_str(),
                            host.key_id.map(|id| id as i64),
                            host.os.as_str(),
                            host.rdp_mode.to_db(),
                            index as i64
                        ],
                    )
                    .map_err(|error| error.to_string())?;
            }
            for child in &folder.children {
                insert_folder(transaction, child, Some(folder.id), cipher)?;
            }
            Ok(())
        }

        for (index, folder) in folders.iter().enumerate() {
            let mut root = folder.clone();
            root.sort_order = index as i64;
            insert_folder(&transaction, &root, None, cipher)?;
        }
        transaction.commit().map_err(|error| error.to_string())
    }

    pub fn clear_host_credentials(&mut self) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE hosts SET username_enc = NULL, secret_enc = NULL, key_id = NULL",
                [],
            )
            .map_err(|error| error.to_string())?;
        self.conn
            .execute("DELETE FROM saved_keys", [])
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}
