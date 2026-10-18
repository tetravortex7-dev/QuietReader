use eframe::egui;

pub const FONT_FAMILY_BOLD: &str = "reader-bold";
pub const FONT_FAMILY_ITALIC: &str = "reader-italic";
pub const FONT_FAMILY_BOLD_ITALIC: &str = "reader-bold-italic";

pub fn apply_font_family(ctx: &egui::Context, font_name: &str) {
    let mut fonts = egui::FontDefinitions::default();

    if let Some(bytes) = crate::platform::load_system_font_variant_bytes(font_name, false, false) {
        fonts.font_data.insert(
            font_name.to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, font_name.to_owned());
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
                font_name.to_owned(),
                "Ubuntu-Light".to_owned(),
            ],
        );
    } else {
        fonts.families.insert(
            egui::FontFamily::Name(FONT_FAMILY_BOLD.into()),
            vec![font_name.to_owned(), "Ubuntu-Light".to_owned()],
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
                font_name.to_owned(),
                "Ubuntu-Light".to_owned(),
            ],
        );
    } else {
        fonts.families.insert(
            egui::FontFamily::Name(FONT_FAMILY_ITALIC.into()),
            vec![font_name.to_owned(), "Ubuntu-Light".to_owned()],
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
                font_name.to_owned(),
                "Ubuntu-Light".to_owned(),
            ],
        );
    } else {
        fonts.families.insert(
            egui::FontFamily::Name(FONT_FAMILY_BOLD_ITALIC.into()),
            vec![
                bold_key,
                italic_key,
                font_name.to_owned(),
                "Ubuntu-Light".to_owned(),
            ],
        );
    }

    if let Some(bytes) = crate::platform::load_system_cjk_font_bytes() {
        fonts.font_data.insert(
            "system_cjk".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
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

    if let Some(bytes) = crate::platform::load_system_arabic_font_bytes() {
        fonts.font_data.insert(
            "system_arabic".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
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
    pub badge: &'static str,
    pub description: &'static str,
}

pub const AVAILABLE_FONTS: [FontOptionInfo; 8] = [
    FontOptionInfo {
        id: "Georgia",
        name: "Georgia",
        badge: "Антиква • По умолчанию",
        description: "Классический книжный шрифт с мягкими засечками. Разработан специально для комфортного чтения больших объемов текста на экранах.",
    },
    FontOptionInfo {
        id: "Palatino Linotype",
        name: "Palatino Linotype",
        badge: "Книжная классика",
        description: "Эталонная книжная антиква с открытыми гармоничными формами и безупречной удобочитаемостью. Признанный стандарт качественной литературы.",
    },
    FontOptionInfo {
        id: "Calibri",
        name: "Calibri",
        badge: "Гуманистический гротеск",
        description: "Современный мягкий шрифт без засечек со скругленными линиями. Легкий и приятный для художественной литературы.",
    },
    FontOptionInfo {
        id: "Segoe UI",
        name: "Segoe UI",
        badge: "Интерфейсный гротеск",
        description: "Четкий, строгий системный шрифт с высокой резкостью контуров для экранов высокого разрешения.",
    },
    FontOptionInfo {
        id: "Verdana",
        name: "Verdana",
        badge: "Экранный гротеск",
        description: "Шрифт с увеличенным межбуквенным интервалом и открытыми знаками. Создан специально для идеального чтения без усталости глаз.",
    },
    FontOptionInfo {
        id: "Comic Sans MS",
        name: "Comic Sans MS",
        badge: "Доступность • Дислексия",
        description: "Рекомендован ассоциациями дислексии. Несимметричные, уникальные формы букв исключают зеркальную путаницу. Полная поддержка кириллицы и латиницы.",
    },
    FontOptionInfo {
        id: "Sitka",
        name: "Sitka",
        badge: "Оптическая разборчивость",
        description: "Шрифт Microsoft, созданный совместно с исследователями чтения для улучшения распознавания слов и снижения утомления глаз.",
    },
    FontOptionInfo {
        id: "OpenDyslexic",
        name: "OpenDyslexic",
        badge: "Специализированный",
        description: "Специальный шрифт с утяжелённым основанием букв против дислексии. Используется системная или пользовательская версия при наличии.",
    },
];

impl FontOptionInfo {
    pub fn badge_for_lang(&self, lang: crate::i18n::UiLanguage) -> &'static str {
        if matches!(lang, crate::i18n::UiLanguage::Ru | crate::i18n::UiLanguage::Uk) {
            return self.badge;
        }
        match self.id {
            "Georgia" => "Serif • Default",
            "Palatino Linotype" => "Book Classic",
            "Calibri" => "Humanist Sans",
            "Segoe UI" => "UI Sans",
            "Verdana" => "Screen Sans",
            "Comic Sans MS" => "Accessibility • Dyslexia",
            "Sitka" => "Optical Legibility",
            "OpenDyslexic" => "Specialized Dyslexia",
            _ => self.badge,
        }
    }

    pub fn desc_for_lang(&self, lang: crate::i18n::UiLanguage) -> &'static str {
        if matches!(lang, crate::i18n::UiLanguage::Ru | crate::i18n::UiLanguage::Uk) {
            return self.description;
        }
        match self.id {
            "Georgia" => "Classic book serif with gentle serifs designed for comfortable long-form screen reading.",
            "Palatino Linotype" => "Literary benchmark serif with harmonious open forms and superb readability.",
            "Calibri" => "Modern warm sans-serif with rounded lines, easy on the eyes.",
            "Segoe UI" => "Crisp, clean system sans-serif with sharp outlines for high-DPI displays.",
            "Verdana" => "Wide letter spacing and open apertures designed for fatigue-free screen reading.",
            "Comic Sans MS" => "Recommended by dyslexia associations. Distinct, non-mirrored letterforms prevent visual confusion. Full Latin and Cyrillic support.",
            "Sitka" => "Microsoft font designed with reading researchers to enhance word recognition and reduce eye fatigue.",
            "OpenDyslexic" => "Specialized typeface with weighted bottoms to prevent letter inversion. Uses system or user installation if available.",
            _ => self.description,
        }
    }
}
