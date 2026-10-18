use super::{EditorDialog, ShellCoreApp};

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
}
