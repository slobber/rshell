use crate::app::ShellCoreApp;
use crate::domain::{Host, KeySource};
use crate::ssh::SshAuthMaterial;

impl ShellCoreApp {
    pub fn resolve_ssh_auth(&self, host: &Host) -> Result<SshAuthMaterial, String> {
        if host.credentials_unavailable {
            return Err("Stored credentials are unavailable; edit the host first".to_owned());
        }
        if host.user.trim().is_empty() {
            return Err("Username is required".to_owned());
        }
        match host.auth {
            crate::domain::AuthKind::Password => {
                if host.secret.is_empty() {
                    Err("Password is required".to_owned())
                } else {
                    Ok(SshAuthMaterial::Password(host.secret.clone()))
                }
            }
            crate::domain::AuthKind::Agent => Ok(SshAuthMaterial::Agent),
            crate::domain::AuthKind::Key => {
                if host.key_source == KeySource::Managed {
                    let key_id = host
                        .key_id
                        .ok_or_else(|| "No managed private key is selected".to_owned())?;
                    let storage = self
                        .storage
                        .as_ref()
                        .ok_or_else(|| "Local database is unavailable".to_owned())?;
                    let cipher = self
                        .cipher
                        .as_ref()
                        .ok_or_else(|| "RShell must be unlocked to use managed keys".to_owned())?;
                    let pem = storage.load_saved_key(cipher, key_id)?;
                    Ok(SshAuthMaterial::PrivateKey(pem))
                } else {
                    let path = crate::local_fs::resolve(&host.secret);
                    let pem = std::fs::read_to_string(&path).map_err(|error| {
                        format!("Could not read private key {}: {error}", path.display())
                    })?;
                    Ok(SshAuthMaterial::PrivateKey(pem))
                }
            }
        }
    }
}
