use std::collections::BTreeSet;

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use fontdb::{Database, Family, Query, Stretch, Style, Weight};

pub struct FontCatalog {
    db: Database,
    pub ui_families: Vec<String>,
    pub mono_families: Vec<String>,
}

impl FontCatalog {
    pub fn load() -> Self {
        let mut db = Database::new();
        db.load_system_fonts();

        let mut ui_families = BTreeSet::new();
        let mut mono_families = BTreeSet::new();
        for face in db.faces() {
            let Some((family, _)) = face.families.first() else {
                continue;
            };
            if family.starts_with('@') {
                continue;
            }
            ui_families.insert(family.clone());
            if face.monospaced || looks_monospaced(family) {
                mono_families.insert(family.clone());
            }
        }
        if mono_families.is_empty() {
            mono_families.extend(ui_families.iter().cloned());
        }

        Self {
            db,
            ui_families: ui_families.into_iter().collect(),
            mono_families: mono_families.into_iter().collect(),
        }
    }

    pub fn default_ui_font(&self) -> String {
        preferred(
            &self.ui_families,
            &[
                "Segoe UI",
                "Segoe UI Variable Text",
                "SF Pro Text",
                "Helvetica Neue",
                "Noto Sans",
                "Noto Sans CJK SC",
                "Microsoft YaHei UI",
                "Microsoft YaHei",
                "PingFang SC",
                "DejaVu Sans",
                "Arial",
            ],
        )
    }

    pub fn default_mono_font(&self) -> String {
        preferred(
            &self.mono_families,
            &[
                "Cascadia Mono",
                "Cascadia Code",
                "Consolas",
                "SF Mono",
                "Menlo",
                "JetBrains Mono",
                "Noto Sans Mono",
                "Noto Sans Mono CJK SC",
                "DejaVu Sans Mono",
                "Ubuntu Mono",
                "Courier New",
            ],
        )
    }

    pub fn apply(
        &self,
        ctx: &egui::Context,
        ui_family: &str,
        mono_family: &str,
    ) -> Result<(), String> {
        let mut fonts = FontDefinitions::default();
        if let Some((bytes, index)) = self.face_bytes(ui_family) {
            let mut data = FontData::from_owned(bytes);
            data.index = index;
            fonts.font_data.insert("rshell_ui".to_owned(), data.into());
            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, "rshell_ui".to_owned());
        }
        if let Some((bytes, index)) = self.face_bytes(mono_family) {
            let mut data = FontData::from_owned(bytes);
            data.index = index;
            fonts
                .font_data
                .insert("rshell_terminal".to_owned(), data.into());
            let family = fonts.families.entry(FontFamily::Monospace).or_default();
            family.insert(0, "rshell_terminal".to_owned());
            if ui_family != mono_family {
                if let Some((bytes, index)) = self.face_bytes(ui_family) {
                    let mut data = FontData::from_owned(bytes);
                    data.index = index;
                    fonts
                        .font_data
                        .insert("rshell_mono_cjk_fallback".to_owned(), data.into());
                    family.push("rshell_mono_cjk_fallback".to_owned());
                }
            }
        }
        ctx.set_fonts(fonts);
        Ok(())
    }

    fn face_bytes(&self, family: &str) -> Option<(Vec<u8>, u32)> {
        let id = self.db.query(&Query {
            families: &[Family::Name(family)],
            weight: Weight::NORMAL,
            stretch: Stretch::Normal,
            style: Style::Normal,
        })?;
        self.db
            .with_face_data(id, |data, face_index| Some((data.to_vec(), face_index)))?
    }
}

fn preferred(available: &[String], candidates: &[&str]) -> String {
    for candidate in candidates {
        if let Some(found) = available
            .iter()
            .find(|family| family.eq_ignore_ascii_case(candidate))
        {
            return found.clone();
        }
    }
    available
        .first()
        .cloned()
        .unwrap_or_else(|| "sans-serif".to_owned())
}

fn looks_monospaced(family: &str) -> bool {
    let lower = family.to_ascii_lowercase();
    lower.contains("mono")
        || lower.contains("console")
        || lower.contains("code")
        || lower.contains("courier")
        || lower.contains("menlo")
        || lower.contains("consolas")
        || lower.contains("cascadia")
}
