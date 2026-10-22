use eframe::egui;

pub const FONT_FAMILY_BOLD: &str = "reader-bold";
pub const FONT_FAMILY_ITALIC: &str = "reader-italic";
pub const FONT_FAMILY_BOLD_ITALIC: &str = "reader-bold-italic";

pub fn apply_font_family(ctx: &egui::Context, font_name: &str) {
    let mut fonts = egui::FontDefinitions::default();

    if let Some(bytes) = crate::platform::load_system_cjk_font_bytes() {
        fonts.font_data.insert(
            "system_cjk".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
    }
    if let Some(bytes) = crate::platform::load_system_arabic_font_bytes() {
        fonts.font_data.insert(
            "system_arabic".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
    }

    for opt in AVAILABLE_FONTS {
        let opt_key = format!("font-opt-{}", opt.id);
        if let Some(bytes) = crate::platform::load_system_font_variant_bytes(opt.id, false, false) {
            fonts.font_data.insert(
                opt_key.clone(),
                std::sync::Arc::new(egui::FontData::from_owned(bytes)),
            );
            let mut fam = vec![opt_key, "Ubuntu-Light".to_owned()];
            if fonts.font_data.contains_key("system_cjk") {
                fam.push("system_cjk".to_owned());
            }
            if fonts.font_data.contains_key("system_arabic") {
                fam.push("system_arabic".to_owned());
            }
            fonts
                .families
                .insert(egui::FontFamily::Name(opt.id.into()), fam);
        }
    }

    let active_key = format!("active-{}", font_name);
    if let Some(bytes) = crate::platform::load_system_font_variant_bytes(font_name, false, false) {
        fonts.font_data.insert(
            active_key.clone(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, active_key.clone());
    }

    let bold_key = format!("{font_name}-Bold");
    if let Some(bytes) = crate::platform::load_system_font_variant_bytes(font_name, true, false) {
        fonts.font_data.insert(
            bold_key.clone(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
        fonts.families.insert(
            egui::FontFamily::Name(FONT_FAMILY_BOLD.into()),
            vec![
                bold_key.clone(),
                active_key.clone(),
                "Ubuntu-Light".to_owned(),
            ],
        );
    } else {
        fonts.families.insert(
            egui::FontFamily::Name(FONT_FAMILY_BOLD.into()),
            vec![active_key.clone(), "Ubuntu-Light".to_owned()],
        );
    }

    let italic_key = format!("{font_name}-Italic");
    if let Some(bytes) = crate::platform::load_system_font_variant_bytes(font_name, false, true) {
        fonts.font_data.insert(
            italic_key.clone(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
        fonts.families.insert(
            egui::FontFamily::Name(FONT_FAMILY_ITALIC.into()),
            vec![
                italic_key.clone(),
                active_key.clone(),
                "Ubuntu-Light".to_owned(),
            ],
        );
    } else {
        fonts.families.insert(
            egui::FontFamily::Name(FONT_FAMILY_ITALIC.into()),
            vec![active_key.clone(), "Ubuntu-Light".to_owned()],
        );
    }

    let bold_italic_key = format!("{font_name}-BoldItalic");
    if let Some(bytes) = crate::platform::load_system_font_variant_bytes(font_name, true, true) {
        fonts.font_data.insert(
            bold_italic_key.clone(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
        fonts.families.insert(
            egui::FontFamily::Name(FONT_FAMILY_BOLD_ITALIC.into()),
            vec![
                bold_italic_key,
                bold_key,
                italic_key,
                active_key,
                "Ubuntu-Light".to_owned(),
            ],
        );
    } else {
        fonts.families.insert(
            egui::FontFamily::Name(FONT_FAMILY_BOLD_ITALIC.into()),
            vec![bold_key, italic_key, active_key, "Ubuntu-Light".to_owned()],
        );
    }

    if fonts.font_data.contains_key("system_cjk") {
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .push("system_cjk".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Name(FONT_FAMILY_BOLD.into()))
            .or_default()
            .push("system_cjk".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Name(FONT_FAMILY_ITALIC.into()))
            .or_default()
            .push("system_cjk".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Name(FONT_FAMILY_BOLD_ITALIC.into()))
            .or_default()
            .push("system_cjk".to_owned());
    }

    if fonts.font_data.contains_key("system_arabic") {
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .push("system_arabic".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Name(FONT_FAMILY_BOLD.into()))
            .or_default()
            .push("system_arabic".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Name(FONT_FAMILY_ITALIC.into()))
            .or_default()
            .push("system_arabic".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Name(FONT_FAMILY_BOLD_ITALIC.into()))
            .or_default()
            .push("system_arabic".to_owned());
    }

    ctx.set_fonts(fonts);
}

pub struct FontOptionInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub index: usize,
}

pub const AVAILABLE_FONTS: [FontOptionInfo; 9] = [
    FontOptionInfo {
        id: "Georgia",
        name: "Georgia",
        index: 0,
    },
    FontOptionInfo {
        id: "Palatino Linotype",
        name: "Palatino Linotype",
        index: 1,
    },
    FontOptionInfo {
        id: "Calibri",
        name: "Calibri",
        index: 2,
    },
    FontOptionInfo {
        id: "Segoe UI",
        name: "Segoe UI",
        index: 3,
    },
    FontOptionInfo {
        id: "Verdana",
        name: "Verdana",
        index: 4,
    },
    FontOptionInfo {
        id: "Comic Sans MS",
        name: "Comic Sans MS",
        index: 5,
    },
    FontOptionInfo {
        id: "Sitka",
        name: "Sitka",
        index: 6,
    },
    FontOptionInfo {
        id: "OpenDyslexic",
        name: "OpenDyslexic",
        index: 7,
    },
    FontOptionInfo {
        id: "Lexend",
        name: "Lexend",
        index: 8,
    },
];

impl FontOptionInfo {
    pub fn badge_for_lang(&self, lang: crate::i18n::UiLanguage) -> &'static str {
        lang.t().fonts[self.index].badge
    }

    pub fn desc_for_lang(&self, lang: crate::i18n::UiLanguage) -> &'static str {
        lang.t().fonts[self.index].description
    }
}
