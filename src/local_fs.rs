use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Local, Utc};

use crate::domain::FileEntry;

pub fn home_dir() -> PathBuf {
    crate::config::home_dir()
}

pub fn resolve(path: &str) -> PathBuf {
    if path == "~" || path.is_empty() {
        home_dir()
    } else if let Some(rest) = path.strip_prefix("~/").or_else(|| path.strip_prefix("~\\")) {
        home_dir().join(rest)
    } else {
        PathBuf::from(path)
    }
}

pub fn list(path: &str) -> Result<(String, Vec<FileEntry>), String> {
    let resolved = resolve(path);
    let canonical = fs::canonicalize(&resolved).unwrap_or(resolved);
    let mut entries = Vec::new();

    let read_dir = fs::read_dir(&canonical)
        .map_err(|error| format!("Could not read {}: {error}", canonical.display()))?;
    for item in read_dir {
        let item = item.map_err(|error| error.to_string())?;
        let metadata = match item.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        let is_dir = metadata.is_dir();
        let name = item.file_name().to_string_lossy().into_owned();
        let size = if is_dir { 0 } else { metadata.len() };
        let modified = metadata
            .modified()
            .ok()
            .and_then(format_time)
            .unwrap_or_else(|| "--".to_owned());
        let readonly = metadata.permissions().readonly();
        let perm = format!(
            "{}{}",
            if is_dir { "d" } else { "-" },
            if readonly { "r-xr-xr-x" } else { "rwxrwxrwx" }
        );
        entries.push(FileEntry {
            archive: is_archive(&name),
            name,
            is_dir,
            size,
            modified,
            perm,
            owner: local_owner(),
            symlink: item
                .file_type()
                .map(|kind| kind.is_symlink())
                .unwrap_or(false),
        });
    }

    entries.sort_by(|left, right| {
        right
            .is_dir
            .cmp(&left.is_dir)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    Ok((display_path(&canonical), entries))
}

pub fn join(parent: &str, name: &str) -> String {
    let path = resolve(parent).join(name);
    display_path(&path)
}

pub fn parent(path: &str) -> String {
    let resolved = resolve(path);
    resolved
        .parent()
        .map(display_path)
        .unwrap_or_else(|| display_path(&resolved))
}

pub fn read_text(path: &str, max_bytes: usize) -> Result<(String, bool), String> {
    let resolved = resolve(path);
    let metadata = fs::metadata(&resolved)
        .map_err(|error| format!("Could not read {}: {error}", resolved.display()))?;
    if metadata.len() > max_bytes as u64 * 8 {
        return Err(format!(
            "File is too large to open in the built-in editor ({} bytes)",
            metadata.len()
        ));
    }
    let bytes = fs::read(&resolved)
        .map_err(|error| format!("Could not read {}: {error}", resolved.display()))?;
    let truncated = bytes.len() > max_bytes;
    let bytes = &bytes[..bytes.len().min(max_bytes)];
    Ok((String::from_utf8_lossy(bytes).into_owned(), truncated))
}

pub fn write_text(path: &str, content: &str) -> Result<(), String> {
    let resolved = resolve(path);
    fs::write(&resolved, content)
        .map_err(|error| format!("Could not write {}: {error}", resolved.display()))
}

pub fn modified_label(time: SystemTime) -> String {
    format_time(time).unwrap_or_else(|| "--".to_owned())
}

fn format_time(time: SystemTime) -> Option<String> {
    let datetime: DateTime<Utc> = time.into();
    Some(
        datetime
            .with_timezone(&Local)
            .format("%Y-%m-%d %H:%M")
            .to_string(),
    )
}

fn local_owner() -> String {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "local".to_owned())
}

fn is_archive(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [".zip", ".tar", ".gz", ".tgz", ".bz2", ".xz", ".7z", ".rar"]
        .iter()
        .any(|extension| lower.ends_with(extension))
}

pub fn display_path(path: &Path) -> String {
    let text = path.display().to_string();
    if let Some(rest) = text.strip_prefix("\\\\?\\UNC\\") {
        format!("\\\\{rest}")
    } else if let Some(rest) = text.strip_prefix("\\\\?\\") {
        rest.to_owned()
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::display_path;
    use std::path::Path;

    #[test]
    fn display_path_strips_windows_verbatim_prefixes() {
        assert_eq!(
            display_path(Path::new(r"\\?\C:\Users\slobber")),
            r"C:\Users\slobber"
        );
        assert_eq!(
            display_path(Path::new(r"\\?\UNC\server\share\folder")),
            r"\\server\share\folder"
        );
        assert_eq!(
            display_path(Path::new(r"C:\Users\slobber")),
            r"C:\Users\slobber"
        );
    }
}
