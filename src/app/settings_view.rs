use super::dialogs::SettingsTab;
use super::fonts::apply_font_family;
use super::theme::ReaderTheme;
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, RichText, ScrollArea, Vec2};

pub struct SettingsView;

impl SettingsView {
    pub fn render(
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        show_settings: &mut bool,
        settings_tab: &mut SettingsTab,
        settings: &mut AppSettings,
        font_size: &mut f32,
        theme: &mut ReaderTheme,
        keep_screen_on: &mut bool,
        is_autoscrolling: &mut bool,
        on_toggle_autoscroll: &mut bool,
        on_save_settings: &mut bool,
        on_register_association: &mut bool,
    ) {
        let text_color = theme.text_color();
        let accent_color = theme.accent_color();
        let card_bg = theme.panel_bg_color();
        let card_stroke = egui::Stroke::new(1.0, accent_color.gamma_multiply(0.35));

        ui.vertical_centered(|ui| {
            ui.set_max_width(760.0);
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("⚙ Настройки QuietReader")
                        .size(22.0)
                        .strong()
                        .color(text_color),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(RichText::new("✕ Вернуться к чтению").size(14.0).strong())
                        .clicked()
                    {
                        *show_settings = false;
                    }
                });
            });

            ui.add_space(4.0);
            if let Some(err) = AppSettings::last_save_error() {
                ui.label(
                    RichText::new(format!("⚠ Ошибка сохранения настроек: {err}"))
                        .color(Color32::RED)
                        .strong(),
                );
                ui.add_space(4.0);
            }
            ui.separator();
            ui.add_space(8.0);

            ui.horizontal_wrapped(|ui| {
                let tabs = [
                    (SettingsTab::Fonts, "🔤 Шрифты и текст"),
                    (SettingsTab::Appearance, "🎨 Внешний вид"),
                    (SettingsTab::AutoScroll, "📜 Автопрокрутка"),
                    (SettingsTab::ComicsAndManga, "📖 Комиксы и манга"),
                    (SettingsTab::Hotkeys, "⌨ Горячие клавиши"),
                    (SettingsTab::System, "⚙ Система и файлы"),
                ];

                for (tab, label) in tabs {
                    let is_active = *settings_tab == tab;
                    let btn = egui::Button::selectable(
                        is_active,
                        RichText::new(label).size(13.5).strong(),
                    )
                    .min_size(Vec2::new(115.0, 32.0));
                    if ui.add(btn).clicked() {
                        *settings_tab = tab;
                    }
                }
            });

            ui.add_space(10.0);

            egui::Frame::new()
                .fill(card_bg)
                .stroke(card_stroke)
                .corner_radius(8.0)
                .inner_margin(egui::Margin::symmetric(20, 16))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());

                    ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| match *settings_tab {
                            SettingsTab::Fonts => Self::render_fonts_tab(
                                ui,
                                ctx,
                                settings,
                                font_size,
                                theme,
                                on_save_settings,
                            ),
                            SettingsTab::Appearance => Self::render_appearance_tab(
                                ui,
                                theme,
                                keep_screen_on,
                                settings,
                                on_save_settings,
                            ),
                            SettingsTab::AutoScroll => Self::render_autoscroll_tab(
                                ui,
                                settings,
                                is_autoscrolling,
                                on_toggle_autoscroll,
                                theme,
                                on_save_settings,
                            ),
                            SettingsTab::ComicsAndManga => {
                                Self::render_comics_tab(ui, settings, theme, on_save_settings)
                            }
                            SettingsTab::Hotkeys => Self::render_hotkeys_tab(ui, theme),
                            SettingsTab::System => Self::render_system_tab(
                                ui,
                                settings,
                                on_register_association,
                                on_save_settings,
                                theme,
                            ),
                        });
                });
        });
    }

    fn render_fonts_tab(
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        settings: &mut AppSettings,
        font_size: &mut f32,
        theme: &ReaderTheme,
        on_save_settings: &mut bool,
    ) {
        let text_color = theme.text_color();

        ui.label(
            RichText::new("Гарнитура текста книги")
                .size(17.0)
                .strong()
                .color(text_color),
        );
        ui.label(
            RichText::new("Выберите типографику для максимального комфорта ваших глаз:")
                .size(13.0)
                .color(text_color.gamma_multiply(0.75)),
        );
        ui.add_space(8.0);

        for font_info in super::fonts::AVAILABLE_FONTS {
            let is_selected = settings.font_family == font_info.id;
            egui::Frame::new()
                .fill(if is_selected {
                    theme.panel_bg_color()
                } else {
                    theme.bg_color()
                })
                .stroke(egui::Stroke::new(
                    if is_selected { 1.5 } else { 1.0 },
                    if is_selected {
                        theme.accent_color()
                    } else {
                        Color32::from_gray(120).gamma_multiply(0.3)
                    },
                ))
                .corner_radius(6.0)
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .radio(
                                is_selected,
                                RichText::new(font_info.name)
                                    .size(15.0)
                                    .strong()
                                    .color(text_color),
                            )
                            .clicked()
                            && !is_selected
                        {
                            settings.font_family = font_info.id.to_string();
                            apply_font_family(ctx, font_info.id);
                            *on_save_settings = true;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(font_info.badge)
                                    .size(11.5)
                                    .color(theme.accent_color()),
                            );
                        });
                    });
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(font_info.description)
                            .size(12.0)
                            .color(text_color.gamma_multiply(0.75)),
                    );
                });
            ui.add_space(4.0);
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Размер шрифта:")
                    .size(15.0)
                    .strong()
                    .color(text_color),
            );
            if ui.button(RichText::new("  ➖  ").size(13.0)).clicked() && *font_size > 12.0 {
                *font_size -= 1.0;
                *on_save_settings = true;
            }
            let mut size_val = *font_size;
            let resp = ui.add(egui::Slider::new(&mut size_val, 12.0..=36.0).suffix(" pt"));
            if resp.changed() {
                *font_size = size_val;
            }
            if resp.drag_stopped() || resp.lost_focus() {
                *on_save_settings = true;
            }
            if ui.button(RichText::new("  ➕  ").size(13.0)).clicked() && *font_size < 36.0 {
                *font_size += 1.0;
                *on_save_settings = true;
            }
        });

        ui.add_space(10.0);
        egui::Frame::new()
            .fill(theme.bg_color())
            .stroke(egui::Stroke::new(1.0, theme.accent_color().gamma_multiply(0.25)))
            .corner_radius(6.0)
            .inner_margin(egui::Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.label(RichText::new("Живой предпросмотр текста:").italics().size(12.0).color(text_color.gamma_multiply(0.65)));
                ui.add_space(6.0);
                let sample = "— В тишине страниц оживают миры. Чтение — это не просто буквы на экране, а живой разговор с мудростью прошлых эпох.\n\nБыть спокойным, видеть ясно, читать с удовольствием.";
                ui.label(RichText::new(sample).size(*font_size).color(text_color));
            });
    }

    fn render_appearance_tab(
        ui: &mut egui::Ui,
        theme: &mut ReaderTheme,
        keep_screen_on: &mut bool,
        settings: &mut AppSettings,
        on_save_settings: &mut bool,
    ) {
        let text_color = theme.text_color();

        ui.label(
            RichText::new("Цветовая тема читалки")
                .size(17.0)
                .strong()
                .color(text_color),
        );
        ui.add_space(6.0);

        let themes = [
            (
                ReaderTheme::Sepia,
                "Сепия (рекомендуется)",
                "Теплый мягкий фон цвета пергамента и коричневатый текст. Снижает нагрузку на зрение при длительном чтении.",
            ),
            (
                ReaderTheme::Warm,
                "Теплая (Закатная бумага)",
                "Мягкий янтарный оттенок и шоколадный шрифт (стиль Moon+ Reader). Идеален для вечернего чтения без синего спектра.",
            ),
            (
                ReaderTheme::Mint,
                "Мятная бумага (PocketBook)",
                "Освежающий пастельно-зеленый фон и контрастный темно-зеленый текст. Снимает спазм глазных мышц.",
            ),
            (
                ReaderTheme::Light,
                "Светлая",
                "Классическая дневная страница с естественной бумажной контрастностью.",
            ),
            (
                ReaderTheme::Dark,
                "Тёмная",
                "Глубокий ночной фон со светло-серым текстом для чтения в полумраке или темноте.",
            ),
            (
                ReaderTheme::Oled,
                "OLED (Абсолютный чёрный)",
                "100% черный фон (RGB #000000) с полным отключением пикселей на OLED/AMOLED дисплеях. Максимальная экономия батареи.",
            ),
        ];

        for (theme_variant, title, desc) in themes {
            let is_sel = *theme == theme_variant;
            let fill = if is_sel {
                theme.panel_bg_color()
            } else {
                theme.bg_color()
            };
            let stroke = if is_sel {
                egui::Stroke::new(1.5, theme.accent_color())
            } else {
                egui::Stroke::new(1.0, Color32::from_gray(120).gamma_multiply(0.3))
            };
            egui::Frame::new()
                .fill(fill)
                .stroke(stroke)
                .corner_radius(6.0)
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    if ui
                        .radio(
                            is_sel,
                            RichText::new(title).size(15.0).strong().color(text_color),
                        )
                        .clicked()
                    {
                        *theme = theme_variant;
                        *on_save_settings = true;
                    }
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(desc)
                            .size(12.0)
                            .color(text_color.gamma_multiply(0.75)),
                    );
                });
            ui.add_space(4.0);
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        let label = RichText::new("🔆 Не выключать экран во время чтения")
            .size(14.0)
            .strong()
            .color(text_color);
        if ui.checkbox(keep_screen_on, label).changed() {
            crate::platform::set_keep_screen_on(*keep_screen_on);
            settings.keep_screen_on = *keep_screen_on;
            *on_save_settings = true;
        }
        ui.label(
            RichText::new("Пока окно QuietReader активно, дисплей не гаснет автоматически.")
                .size(12.0)
                .color(text_color.gamma_multiply(0.7)),
        );
    }

    fn render_autoscroll_tab(
        ui: &mut egui::Ui,
        settings: &mut AppSettings,
        is_autoscrolling: &mut bool,
        on_toggle_autoscroll: &mut bool,
        theme: &ReaderTheme,
        on_save_settings: &mut bool,
    ) {
        let text_color = theme.text_color();

        ui.label(
            RichText::new("Автоматическая промотка страниц")
                .size(17.0)
                .strong()
                .color(text_color),
        );
        ui.label(
            RichText::new("Чтение без постоянных касаний колесика мыши:")
                .size(13.0)
                .color(text_color.gamma_multiply(0.75)),
        );
        ui.add_space(8.0);

        let scroll_btn_text = if *is_autoscrolling {
            "⏸ Остановить автопрокрутку (клавиша A)"
        } else {
            "▶ Запустить автопрокрутку (клавиша A)"
        };
        if ui
            .button(RichText::new(scroll_btn_text).size(14.0).strong())
            .clicked()
        {
            *on_toggle_autoscroll = true;
        }

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Тип движения:").strong().color(text_color));
            if ui
                .radio_value(
                    &mut settings.auto_scroll.pause_mode,
                    true,
                    "С паузами (шаг + остановка)",
                )
                .clicked()
            {
                *on_save_settings = true;
            }
            if ui
                .radio_value(
                    &mut settings.auto_scroll.pause_mode,
                    false,
                    "Непрерывный (плавный скролл)",
                )
                .clicked()
            {
                *on_save_settings = true;
            }
        });

        ui.add_space(8.0);
        if settings.auto_scroll.pause_mode {
            ui.horizontal(|ui| {
                ui.label("Время паузы для чтения:");
                let resp = ui.add(
                    egui::Slider::new(&mut settings.auto_scroll.pause_duration, 0.5..=15.0)
                        .suffix(" сек"),
                );
                if resp.drag_stopped() || resp.lost_focus() {
                    *on_save_settings = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Шаг сдвига:");
                let resp = ui.add(
                    egui::Slider::new(&mut settings.auto_scroll.step_distance, 30.0..=350.0)
                        .suffix(" px"),
                );
                if resp.drag_stopped() || resp.lost_focus() {
                    *on_save_settings = true;
                }
            });
        } else {
            ui.horizontal(|ui| {
                ui.label("Скорость прокрутки:");
                let resp = ui.add(
                    egui::Slider::new(&mut settings.auto_scroll.continuous_speed, 10.0..=250.0)
                        .suffix(" px/сек"),
                );
                if resp.drag_stopped() || resp.lost_focus() {
                    *on_save_settings = true;
                }
            });
        }
    }

    fn render_comics_tab(
        ui: &mut egui::Ui,
        settings: &mut AppSettings,
        theme: &ReaderTheme,
        on_save_settings: &mut bool,
    ) {
        let text_color = theme.text_color();
        ui.label(
            RichText::new("Параметры комиксов, манги и PDF")
                .size(17.0)
                .strong()
                .color(text_color),
        );
        ui.add_space(6.0);

        let modes = [
            (
                "Auto",
                "Авто (японский порядок R2L для манги, обычный для остальных)",
                "Определяет ориентацию по метаданным архива",
            ),
            (
                "Manga",
                "Только манга (справа-налево)",
                "Принудительный японский порядок страниц R2L",
            ),
            (
                "Comic",
                "Обычный комикс (слева-направо)",
                "Европейский порядок страниц L2R",
            ),
            (
                "Webtoon",
                "Вебтун (вертикальная лента)",
                "Непрерывная вертикальная прокрутка",
            ),
        ];

        for (mode_key, title, desc) in modes {
            let is_sel = settings.comic.reading_mode == mode_key;
            let fill = if is_sel {
                theme.panel_bg_color()
            } else {
                theme.bg_color()
            };
            let stroke = if is_sel {
                egui::Stroke::new(1.5, theme.accent_color())
            } else {
                egui::Stroke::new(1.0, Color32::from_gray(120).gamma_multiply(0.3))
            };
            egui::Frame::new()
                .fill(fill)
                .stroke(stroke)
                .corner_radius(6.0)
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    if ui
                        .radio(is_sel, RichText::new(title).strong().color(text_color))
                        .clicked()
                    {
                        settings.comic.reading_mode = mode_key.to_string();
                        *on_save_settings = true;
                    }
                    ui.label(
                        RichText::new(desc)
                            .size(11.5)
                            .color(text_color.gamma_multiply(0.75)),
                    );
                });
            ui.add_space(3.0);
        }

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);
        ui.label(
            RichText::new("Разворот и масштаб страниц:")
                .strong()
                .color(text_color),
        );
        ui.horizontal(|ui| {
            let s1 = ui.radio_value(
                &mut settings.comic.page_layout,
                "Single".to_string(),
                "Одиночная (1)",
            );
            let s2 = ui.radio_value(
                &mut settings.comic.page_layout,
                "Double".to_string(),
                "Разворот (2)",
            );
            if s1.clicked() || s2.clicked() {
                *on_save_settings = true;
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (val, label) in [
                ("FitWindow", "По окну"),
                ("FitWidth", "По ширине"),
                ("FitHeight", "По высоте"),
                ("Original", "100%"),
            ] {
                if ui
                    .radio_value(&mut settings.comic.fit_mode, val.to_string(), label)
                    .clicked()
                {
                    *on_save_settings = true;
                }
            }
        });
    }

    fn render_hotkeys_tab(ui: &mut egui::Ui, theme: &ReaderTheme) {
        let text_color = theme.text_color();
        ui.label(
            RichText::new("Горячие клавиши читалки")
                .size(17.0)
                .strong()
                .color(text_color),
        );
        ui.add_space(6.0);

        let hotkeys = [
            ("F / Esc", "Полноэкранный режим без панелей (вход и выход)"),
            (
                "Зажатое колёсико мыши",
                "Захват и перетаскивание страницы вверх / вниз",
            ),
            ("A", "Запуск / пауза автоматической промотки текста"),
            (
                "Пробел / Shift+Пробел",
                "Быстрая прокрутка на экран вниз / вверх",
            ),
            ("PageDown / PageUp", "Листание на страницу вниз / вверх"),
            ("Стрелки Вниз / Вверх", "Плавный сдвиг на строку"),
            (
                "Стрелки Влево / Вправо",
                "Листание страниц комикса/манги (R2L / L2R)",
            ),
            (
                "Ctrl + Вправо / Влево",
                "Переход к следующему / предыдущему разделу",
            ),
            (
                "Ctrl + Плюс / Минус / 0",
                "Увеличить / уменьшить / сбросить размер шрифта",
            ),
            (
                "Ctrl + A / Ctrl + C",
                "Скопировать весь раздел / выделенный текст",
            ),
            ("Home / End", "Переход в начало / конец раздела или комикса"),
        ];

        let stroke = egui::Stroke::new(1.0, theme.accent_color().gamma_multiply(0.2));
        for (key, desc) in hotkeys {
            egui::Frame::new()
                .fill(theme.bg_color())
                .stroke(stroke)
                .corner_radius(4.0)
                .inner_margin(egui::Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(key).strong().color(theme.accent_color()));
                        ui.label("—");
                        ui.label(RichText::new(desc).color(text_color));
                    });
                });
            ui.add_space(2.0);
        }
    }

    fn render_system_tab(
        ui: &mut egui::Ui,
        settings: &mut AppSettings,
        on_register_association: &mut bool,
        on_save_settings: &mut bool,
        theme: &ReaderTheme,
    ) {
        let text_color = theme.text_color();
        let card_stroke = egui::Stroke::new(1.0, theme.accent_color().gamma_multiply(0.25));

        ui.label(
            RichText::new("Интеграция с системой и хранилище")
                .size(17.0)
                .strong()
                .color(text_color),
        );
        ui.add_space(6.0);

        if crate::platform::supports_file_association() {
            egui::Frame::new()
                .fill(theme.bg_color())
                .stroke(card_stroke)
                .corner_radius(6.0)
                .inner_margin(egui::Margin::symmetric(12, 10))
                .show(ui, |ui| {
                    ui.label(RichText::new("Ассоциация форматов с QuietReader:").strong().color(text_color));
                    ui.add_space(4.0);
                    ui.label("Отметьте форматы, которые вы хотите открывать в QuietReader:");
                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        if ui.small_button("Выбрать все").clicked() {
                            settings.associated_extensions = crate::platform::SUPPORTED_EXTENSIONS_INFO
                                .iter()
                                .map(|(ext, _)| ext.to_string())
                                .collect();
                            *on_save_settings = true;
                        }
                        if ui.small_button("Снять все").clicked() {
                            settings.associated_extensions.clear();
                            *on_save_settings = true;
                        }
                        if ui.small_button("Только книги (FB2, EPUB)").clicked() {
                            settings.associated_extensions = vec!["fb2".to_string(), "epub".to_string()];
                            *on_save_settings = true;
                        }
                    });
                    ui.add_space(6.0);

                    egui::Grid::new("settings_associations_grid")
                        .num_columns(2)
                        .spacing([16.0, 4.0])
                        .show(ui, |ui| {
                            for &(ext, label) in crate::platform::SUPPORTED_EXTENSIONS_INFO {
                                let mut is_checked = settings
                                    .associated_extensions
                                    .iter()
                                    .any(|e| e.eq_ignore_ascii_case(ext));
                                if ui.checkbox(&mut is_checked, label).changed() {
                                    if is_checked {
                                        if !settings
                                            .associated_extensions
                                            .iter()
                                            .any(|e| e.eq_ignore_ascii_case(ext))
                                        {
                                            settings.associated_extensions.push(ext.to_string());
                                        }
                                    } else {
                                        settings
                                            .associated_extensions
                                            .retain(|e| !e.eq_ignore_ascii_case(ext));
                                    }
                                    *on_save_settings = true;
                                }

                                match crate::platform::check_extension_association_status(ext) {
                                    crate::platform::AssociationStatus::Default => {
                                        ui.label(RichText::new("✔ по умолчанию").size(11.0).color(Color32::from_rgb(46, 139, 87)));
                                    }
                                    crate::platform::AssociationStatus::Registered => {
                                        ui.label(RichText::new("зарегистрирован").size(11.0).color(Color32::GRAY));
                                    }
                                    _ => {
                                        ui.label("");
                                    }
                                }
                                ui.end_row();
                            }
                        });

                    ui.add_space(8.0);
                    let has_selected = !settings.associated_extensions.is_empty();
                    if ui.add_enabled(
                        has_selected,
                        egui::Button::new(RichText::new("📌 Зарегистрировать выбранные и открыть параметры Windows").strong()),
                    ).clicked() {
                        *on_register_association = true;
                    }

                    ui.add_space(8.0);
                    if ui.checkbox(&mut settings.dismiss_association_prompt, "Не показывать диалог ассоциации при старте программы").changed() {
                        *on_save_settings = true;
                    }
                });
            ui.add_space(8.0);
        }

        egui::Frame::new()
            .fill(theme.bg_color())
            .stroke(card_stroke)
            .corner_radius(6.0)
            .inner_margin(egui::Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Хранилище настроек:")
                        .strong()
                        .color(text_color),
                );
                ui.add_space(4.0);
                let path_str = AppSettings::get_settings_path().display().to_string();
                ui.label(
                    RichText::new(path_str)
                        .size(12.0)
                        .color(text_color.gamma_multiply(0.7)),
                );
                ui.add_space(6.0);
                if ui
                    .button(RichText::new("💾 Сохранить настройки сейчас").strong())
                    .clicked()
                {
                    *on_save_settings = true;
                }
            });
    }
}
