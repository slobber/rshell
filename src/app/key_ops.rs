use super::ShellCoreApp;

impl ShellCoreApp {
    pub fn import_saved_key_from_path(&mut self) -> Result<(), String> {
        let name = self.key_name.trim().to_owned();
        let path = self.key_file_path.trim().to_owned();
        if name.is_empty() {
            return Err("Key name is required".to_owned());
        }
        if path.is_empty() {
            return Err("Private key file path is required".to_owned());
        }
        let resolved = if path == "~" {
            crate::config::home_dir()
        } else if let Some(rest) = path.strip_prefix("~/").or_else(|| path.strip_prefix("~\\")) {
            crate::config::home_dir().join(rest)
        } else {
            std::path::PathBuf::from(&path)
        };
        let pem = std::fs::read_to_string(&resolved)
            .map_err(|error| format!("Could not read {}: {error}", resolved.display()))?;
        let cipher = self
            .cipher
            .as_ref()
            .ok_or_else(|| "RShell must be unlocked to import a key".to_owned())?;
        let storage = self
            .storage
            .as_mut()
            .ok_or_else(|| "Local database is unavailable".to_owned())?;
        storage.add_saved_key(cipher, &name, &pem)?;
        self.saved_keys = storage.list_saved_keys()?;
        self.key_name.clear();
        self.key_file_path.clear();
        self.keyring_error = None;
        self.log(
            crate::domain::LogLevel::Success,
            format!("Imported managed private key '{name}'."),
        );
        Ok(())
    }

    pub fn delete_saved_key(&mut self, id: u64) {
        let Some(storage) = &mut self.storage else {
            return;
        };
        match storage.delete_saved_key(id) {
            Ok(()) => {
                self.saved_keys = storage.list_saved_keys().unwrap_or_default();
                self.keyring_error = None;
                self.toast("Managed private key deleted");
            }
            Err(error) => {
                self.keyring_error = Some(error.clone());
                self.log(crate::domain::LogLevel::Error, error);
            }
        }
    }
}
