use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub root_dir: PathBuf,
    pub database_path: PathBuf,
    pub config_path: Option<PathBuf>,
    pub warning: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ConfigFile {
    storage: Option<StorageConfig>,
}

#[derive(Debug, Deserialize, Default)]
struct StorageConfig {
    database: Option<String>,
}

impl AppConfig {
    pub fn load() -> Self {
        let user_root = default_root();
        let config_path = find_config_file(&user_root);
        let Some(path) = config_path else {
            return Self {
                root_dir: user_root.clone(),
                database_path: user_root.join("rshell.db"),
                config_path: None,
                warning: None,
            };
        };

        let root_dir = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| user_root.clone());
        let raw = match std::fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(error) => {
                return Self {
                    root_dir: root_dir.clone(),
                    database_path: root_dir.join("rshell.db"),
                    config_path: Some(path),
                    warning: Some(format!("Could not read config.toml: {error}")),
                };
            }
        };

        let parsed = toml::from_str::<ConfigFile>(&raw);
        let (database, warning) = match parsed {
            Ok(file) => {
                let database = file
                    .storage
                    .and_then(|storage| storage.database)
                    .unwrap_or_else(|| "rshell.db".to_owned());
                (database, None)
            }
            Err(error) => (
                "rshell.db".to_owned(),
                Some(format!("Invalid config.toml, using defaults: {error}")),
            ),
        };

        let database_path = resolve_path(&database, &root_dir);
        Self {
            root_dir,
            database_path,
            config_path: Some(path),
            warning,
        }
    }

    pub fn ensure_directories(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.root_dir)?;
        if let Some(parent) = self.database_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(())
    }
}

fn find_config_file(user_root: &Path) -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("RSHELL_CONFIG") {
        let path = PathBuf::from(path);
        let path = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(path)
        };
        if path.is_file() {
            return Some(path);
        }
    }

    if let Ok(executable) = std::env::current_exe() {
        if let Some(directory) = executable.parent() {
            let path = directory.join("config.toml");
            if path.is_file() {
                return Some(path);
            }
        }
    }

    let path = user_root.join("config.toml");
    path.is_file().then_some(path)
}

fn default_root() -> PathBuf {
    home_dir().join(".config").join("rshell")
}

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        })
        .unwrap_or_else(|| PathBuf::from("."))
}

fn resolve_path(value: &str, base: &Path) -> PathBuf {
    let expanded = if value == "~" {
        home_dir()
    } else if let Some(rest) = value
        .strip_prefix("~/")
        .or_else(|| value.strip_prefix("~\\"))
    {
        home_dir().join(rest)
    } else {
        PathBuf::from(value)
    };
    if expanded.is_absolute() {
        expanded
    } else {
        base.join(expanded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_database_is_resolved_against_config_directory() {
        let base = PathBuf::from("C:\\config");
        assert_eq!(
            resolve_path("data\\rshell.db", &base),
            base.join("data\\rshell.db")
        );
    }
}
