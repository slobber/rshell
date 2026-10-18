use crate::domain::{LogLevel, Workspace};
use crate::security::{
    apply_remember, remove_password, reset_credentials, set_password, unlock_with_password,
    validate_password,
};

use super::{AppLock, SecurityAction, ShellCoreApp};

impl ShellCoreApp {
    pub fn unlock(&mut self) {
        let Some(storage) = &self.storage else {
            return;
        };
        match unlock_with_password(&self.security_record, &self.unlock_password) {
            Ok(cipher) => {
                let mut record = self.security_record.clone();
                if apply_remember(
                    &mut record,
                    storage.path.as_path(),
                    &cipher,
                    self.unlock_remember,
                )
                .is_err()
                {
                    self.unlock_error = Some(self.t("keyring_unavailable").to_owned());
                }
                if let Some(storage) = &mut self.storage {
                    let _ = storage.save_security(&record);
                    match storage.load_host_tree(&cipher) {
                        Ok(hosts) => {
                            self.hosts = hosts;
                            self.saved_keys = storage.list_saved_keys().unwrap_or_default();
                        }
                        Err(error) => self.storage_error = Some(error),
                    }
                }
                self.security_record = record;
                self.cipher = Some(cipher);
                self.lock_state = AppLock::Unlocked;
                self.unlock_password.clear();
                self.unlock_error = None;
                self.log(LogLevel::Success, "RShell unlocked.");
            }
            Err(error) => self.unlock_error = Some(error),
        }
    }

    pub fn reset_lost_credentials(&mut self) {
        let Some(storage) = &mut self.storage else {
            return;
        };
        if let Err(error) = storage.clear_host_credentials() {
            self.storage_error = Some(error);
            return;
        }
        let mut record = self.security_record.clone();
        match reset_credentials(&mut record, storage.path.as_path()) {
            Ok(cipher) => {
                let _ = storage.save_security(&record);
                self.security_record = record;
                self.cipher = Some(cipher.clone());
                match storage.load_host_tree(&cipher) {
                    Ok(hosts) => {
                        self.hosts = hosts;
                        self.saved_keys = storage.list_saved_keys().unwrap_or_default();
                    }
                    Err(error) => self.storage_error = Some(error),
                }
                self.lock_state = AppLock::Unlocked;
                self.reset_confirmation = false;
                self.log(LogLevel::Warning, "Local credentials were cleared.");
                self.toast(self.t("credentials_cleared"));
            }
            Err(error) => self.storage_error = Some(error),
        }
    }

    pub fn update_password(&mut self, action: SecurityAction) {
        let Some(cipher) = self.cipher.clone() else {
            return;
        };
        let Some(storage) = &mut self.storage else {
            return;
        };
        if self.security_record.has_password()
            && unlock_with_password(&self.security_record, &self.security_current).is_err()
        {
            self.security_notice = Some((false, self.t("wrong_password").to_owned()));
            return;
        }
        match action {
            SecurityAction::Set | SecurityAction::Change => {
                if let Err(error) = validate_password(&self.security_new, &self.security_confirm) {
                    self.security_notice = Some((false, error));
                    return;
                }
                let mut record = self.security_record.clone();
                match set_password(
                    &mut record,
                    storage.path.as_path(),
                    &cipher,
                    &self.security_new,
                    self.security_remember,
                ) {
                    Ok(()) => {
                        if let Err(error) = storage.save_security(&record) {
                            self.security_notice = Some((false, error));
                            return;
                        }
                        self.security_record = record;
                        self.security_new.clear();
                        self.security_confirm.clear();
                        self.security_current.clear();
                        self.security_notice = Some((true, self.t("password_set").to_owned()));
                        self.toast(self.t("password_set"));
                    }
                    Err(error) => self.security_notice = Some((false, error)),
                }
            }
            SecurityAction::Remove => {
                let mut record = self.security_record.clone();
                match remove_password(&mut record, storage.path.as_path(), &cipher) {
                    Ok(()) => {
                        if let Err(error) = storage.save_security(&record) {
                            self.security_notice = Some((false, error));
                            return;
                        }
                        self.security_record = record;
                        self.security_current.clear();
                        self.security_notice = Some((true, self.t("password_removed").to_owned()));
                        self.toast(self.t("password_removed"));
                    }
                    Err(error) => self.security_notice = Some((false, error)),
                }
            }
        }
    }

    pub fn lock_now(&mut self) {
        if self.security_record.has_password() {
            self.ssh.shutdown();
            self.rdp.shutdown();
            self.cipher = None;
            self.connected_hosts.clear();
            self.workspace = Workspace::new();
            self.hosts.clear();
            self.lock_state = AppLock::Locked;
            self.unlock_password.clear();
            self.unlock_error = None;
        }
    }
}
