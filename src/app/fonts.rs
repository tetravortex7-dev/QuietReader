use eframe::egui;

pub fn apply_font_family(ctx: &egui::Context, font_name: &str) {
    let mut fonts = egui::FontDefinitions::default();

    if let Some(bytes) = crate::platform::load_system_font_bytes(font_name) {
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

    ctx.set_fonts(fonts);
}

pub struct FontOptionInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub badge: &'static str,
    pub description: &'static str,
}

pub const AVAILABLE_FONTS: [FontOptionInfo; 5] = [
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
];
