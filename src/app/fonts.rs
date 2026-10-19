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
    pub badge: &'static str,
    pub description: &'static str,
}

pub const AVAILABLE_FONTS: [FontOptionInfo; 9] = [
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
        badge: "Специализированный • Зашит в приложение",
        description: "Специальный шрифт с утяжелённым основанием букв против переворачивания знаков при дислексии. Всегда встроен в читалку.",
    },
    FontOptionInfo {
        id: "Lexend",
        name: "Lexend",
        badge: "Беглость чтения • Зашит в приложение",
        description: "Разработан исследователями чтения для снижения зрительного утомления, расширения интервала и мгновенного повышения беглости чтения. Встроен в читалку.",
    },
];

impl FontOptionInfo {
    pub fn badge_for_lang(&self, lang: crate::i18n::UiLanguage) -> &'static str {
        match lang.resolve() {
            crate::i18n::UiLanguage::Ru => self.badge,
            crate::i18n::UiLanguage::Uk => match self.id {
                "Georgia" => "Антиква • За замовчуванням",
                "Palatino Linotype" => "Книжкова класика",
                "Calibri" => "Гуманістичний гротеск",
                "Segoe UI" => "Інтерфейсний гротеск",
                "Verdana" => "Екранний гротеск",
                "Comic Sans MS" => "Доступність • Дислексія",
                "Sitka" => "Оптична розбірливість",
                "OpenDyslexic" => "Спеціалізований • Вбудований",
                "Lexend" => "Швидкість читання • Вбудований",
                _ => self.badge,
            },
            _ => match self.id {
                "Georgia" => "Serif • Default",
                "Palatino Linotype" => "Book Classic",
                "Calibri" => "Humanist Sans",
                "Segoe UI" => "UI Sans",
                "Verdana" => "Screen Sans",
                "Comic Sans MS" => "Accessibility • Dyslexia",
                "Sitka" => "Optical Legibility",
                "OpenDyslexic" => "Specialized • Embedded",
                "Lexend" => "Reading Fluency • Embedded",
                _ => self.badge,
            },
        }
    }

    pub fn desc_for_lang(&self, lang: crate::i18n::UiLanguage) -> &'static str {
        match lang.resolve() {
            crate::i18n::UiLanguage::Ru => self.description,
            crate::i18n::UiLanguage::Uk => match self.id {
                "Georgia" => {
                    "Класичний книжковий шрифт із м'якими засічками. Створений спеціально для комфортного читання великих обсягів тексту на екранах."
                }
                "Palatino Linotype" => {
                    "Еталонна книжкова антиква з відкритими гармонійними формами та бездоганною читабельністю."
                }
                "Calibri" => {
                    "Сучасний м'який шрифт без засічок із заокругленими лініями. Легкий та приємний для художньої літератури."
                }
                "Segoe UI" => {
                    "Чіткий, строгий системний шрифт із високою різкістю контурів для екранів високої роздільної здатності."
                }
                "Verdana" => {
                    "Шрифт зі збільшеним міжбуквеним інтервалом та відкритими знаками для читання без втоми очей."
                }
                "Comic Sans MS" => {
                    "Рекомендований асоціаціями дислексії. Несиметричні форми літер виключають дзеркальну плутанину. Підтримка кирилиці та латиниці."
                }
                "Sitka" => {
                    "Шрифт Microsoft, створений спільно з дослідниками читання для покращення розпізнавання слів."
                }
                "OpenDyslexic" => {
                    "Спеціальний шрифт з обтяженою основою літер проти дислексії. Вбудований безпосередньо у програму."
                }
                "Lexend" => {
                    "Розроблений дослідниками для покращення сприйняття тексту та швидкості читання. Вбудований безпосередньо у програму."
                }
                _ => self.description,
            },
            _ => match self.id {
                "Georgia" => {
                    "Classic book serif with gentle serifs designed for comfortable long-form screen reading."
                }
                "Palatino Linotype" => {
                    "Literary benchmark serif with harmonious open forms and superb readability."
                }
                "Calibri" => "Modern warm sans-serif with rounded lines, easy on the eyes.",
                "Segoe UI" => {
                    "Crisp, clean system sans-serif with sharp outlines for high-DPI displays."
                }
                "Verdana" => {
                    "Wide letter spacing and open apertures designed for fatigue-free screen reading."
                }
                "Comic Sans MS" => {
                    "Recommended by dyslexia associations. Distinct, non-mirrored letterforms prevent visual confusion. Full Latin and Cyrillic support."
                }
                "Sitka" => {
                    "Microsoft font designed with reading researchers to enhance word recognition and reduce eye fatigue."
                }
                "OpenDyslexic" => {
                    "Specialized typeface with weighted bottoms to prevent letter inversion. Built directly into the reader."
                }
                "Lexend" => {
                    "Scientifically designed to ease dyslexia, widen character spacing, and improve reading fluency. Built directly into the reader."
                }
                _ => self.description,
            },
        }
    }
}
