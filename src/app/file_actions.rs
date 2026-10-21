use super::{EditorDialog, SftpFileDialog, SftpFileOperation, ShellCoreApp};

impl ShellCoreApp {
    pub fn save_editor(&mut self, dialog: &EditorDialog) -> Result<(), String> {
        if dialog.remote {
            let tab_id = dialog
                .tab_id
                .ok_or_else(|| "Remote editor is missing its SFTP tab".to_owned())?;
            if self
                .ssh
                .save_remote_file(tab_id, dialog.path.clone(), dialog.content.clone())
            {
                Ok(())
            } else {
                Err("SFTP channel is not available".to_owned())
            }
        } else {
            crate::local_fs::write_text(&dialog.path, &dialog.content)
        }
    }

    pub fn submit_sftp_file_operation(&mut self, dialog: &SftpFileDialog) -> Result<(), String> {
        let new_name = dialog.name_input.trim();
        if dialog.operation != SftpFileOperation::Delete {
            validate_item_name(new_name)?;
        }

        let old_path = join_path(&dialog.directory, &dialog.item_name, dialog.remote);
        let new_path = join_path(&dialog.directory, new_name, dialog.remote);
        if dialog.remote {
            let session_id = dialog
                .session_id
                .ok_or_else(|| "The remote SFTP session is no longer available".to_owned())?;
            let accepted = match dialog.operation {
                SftpFileOperation::NewFolder => self
                    .ssh
                    .create_remote_directory(session_id, new_path.clone()),
                SftpFileOperation::Rename => {
                    self.ssh
                        .rename_remote_entry(session_id, old_path.clone(), new_path.clone())
                }
                SftpFileOperation::Delete => {
                    self.ssh
                        .remove_remote_entry(session_id, old_path.clone(), dialog.is_dir)
                }
            };
            if !accepted {
                return Err("SFTP channel is not available".to_owned());
            }
            return Ok(());
        }

        match dialog.operation {
            SftpFileOperation::NewFolder => {
                let path = crate::local_fs::resolve(&new_path);
                if path.exists() {
                    return Err(format!("{} already exists", path.display()));
                }
                std::fs::create_dir(&path)
                    .map_err(|error| format!("Could not create {}: {error}", path.display()))?;
            }
            SftpFileOperation::Rename => {
                let old_path = crate::local_fs::resolve(&old_path);
                let new_path = crate::local_fs::resolve(&new_path);
                if new_path.exists() {
                    return Err(format!("{} already exists", new_path.display()));
                }
                std::fs::rename(&old_path, &new_path).map_err(|error| {
                    format!(
                        "Could not rename {} to {}: {error}",
                        old_path.display(),
                        new_path.display()
                    )
                })?;
            }
            SftpFileOperation::Delete => {
                let path = crate::local_fs::resolve(&old_path);
                let metadata = std::fs::symlink_metadata(&path)
                    .map_err(|error| format!("Could not inspect {}: {error}", path.display()))?;
                let result = if metadata.is_dir() && !metadata.file_type().is_symlink() {
                    std::fs::remove_dir_all(&path)
                } else {
                    std::fs::remove_file(&path)
                };
                result.map_err(|error| format!("Could not delete {}: {error}", path.display()))?;
            }
        }

        if let Some(tab) = self.workspace.find_tab_mut(dialog.tab_id) {
            crate::app::sessions::refresh_local_directory(tab);
        }
        let action = match dialog.operation {
            SftpFileOperation::NewFolder => "Created folder",
            SftpFileOperation::Rename => "Renamed",
            SftpFileOperation::Delete => "Deleted",
        };
        self.log(
            crate::domain::LogLevel::Success,
            format!("{action} {}", dialog.name_input),
        );
        self.toast(format!("{action} {}", dialog.name_input));
        Ok(())
    }
}

fn validate_item_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name == "." || name == ".." {
        return Err("Enter a valid name".to_owned());
    }
    if name
        .chars()
        .any(|character| matches!(character, '/' | '\\' | '\0'))
    {
        return Err("Names cannot contain path separators".to_owned());
    }
    Ok(())
}

fn join_path(parent: &str, name: &str, remote: bool) -> String {
    if name.is_empty() {
        return parent.to_owned();
    }
    if remote {
        if parent.ends_with('/') {
            format!("{parent}{name}")
        } else {
            format!("{parent}/{name}")
        }
    } else {
        crate::local_fs::join(parent, name)
    }
}

#[cfg(test)]
mod tests {
    use super::{join_path, validate_item_name};

    #[test]
    fn item_names_reject_path_traversal_and_separators() {
        assert!(validate_item_name("release-notes.txt").is_ok());
        assert!(validate_item_name("").is_err());
        assert!(validate_item_name(".").is_err());
        assert!(validate_item_name("..").is_err());
        assert!(validate_item_name("nested/file").is_err());
        assert!(validate_item_name(r"nested\file").is_err());
    }

    #[test]
    fn remote_paths_join_with_one_separator() {
        assert_eq!(
            join_path("/var/www/", "index.html", true),
            "/var/www/index.html"
        );
        assert_eq!(
            join_path("/var/www", "index.html", true),
            "/var/www/index.html"
        );
    }
}
