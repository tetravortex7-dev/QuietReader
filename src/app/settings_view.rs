use super::dialogs::SettingsTab;
use super::fonts::apply_font_family;
use super::icons::{Icon, IconButton, IconLabel, selectable_icon_button};
use super::theme::ReaderTheme;
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, RichText, ScrollArea};

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
        updater: &crate::updater::UpdateChecker,
    ) {
        let text_color = theme.text_color();
        let accent_color = theme.accent_color();
        let card_bg = theme.panel_bg_color();
        let card_stroke = egui::Stroke::new(1.0, accent_color.gamma_multiply(0.35));

        let t = settings.ui_language.t();
        ui.vertical_centered(|ui| {
            ui.set_max_width(760.0);
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.add(
                    IconLabel::new(Icon::Settings, t.settings_title)
                        .size(22.0)
                        .strong()
                        .color(text_color),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(IconButton::new(Icon::Close, t.back_to_reading).font_size(14.0))
                        .clicked()
                    {
                        *show_settings = false;
                    }
                });
            });

            ui.add_space(4.0);
            if let Some(err) = AppSettings::last_save_error() {
                ui.add(
                    IconLabel::new(Icon::Warning, &t.save_settings_error.replace("{}", &err))
                        .color(Color32::RED)
                        .strong(),
                );
                ui.add_space(4.0);
            }
            ui.separator();
            ui.add_space(8.0);

            ui.horizontal_wrapped(|ui| {
                let tabs = [
                    (SettingsTab::Fonts, Icon::Fonts, t.tab_fonts),
                    (SettingsTab::Appearance, Icon::Appearance, t.tab_appearance),
                    (SettingsTab::AutoScroll, Icon::AutoScroll, t.tab_autoscroll),
                    (SettingsTab::ComicsAndManga, Icon::Book, t.tab_comics),
                    (SettingsTab::Hotkeys, Icon::Keyboard, t.tab_hotkeys),
                    (SettingsTab::System, Icon::Settings, t.tab_system),
                ];

                for (tab, icon, label) in tabs {
                    let is_active = *settings_tab == tab;
                    let btn = selectable_icon_button(ui, is_active, icon, label);
                    if btn.clicked() {
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
                            SettingsTab::Hotkeys => {
                                Self::render_hotkeys_tab(ui, theme, settings.ui_language)
                            }
                            SettingsTab::System => Self::render_system_tab(
                                ui,
                                settings,
                                on_register_association,
                                on_save_settings,
                                theme,
                                updater,
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
        let t = settings.ui_language.t();
        let text_color = theme.text_color();

        ui.label(
            RichText::new(t.font_choice)
                .size(17.0)
                .strong()
                .color(text_color),
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
                                RichText::new(font_info.badge_for_lang(settings.ui_language))
                                    .size(11.5)
                                    .color(theme.accent_color()),
                            );
                        });
                    });
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(font_info.desc_for_lang(settings.ui_language))
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
                RichText::new(t.font_size)
                    .size(15.0)
                    .strong()
                    .color(text_color),
            );
            if ui
                .add(IconButton::new(Icon::Minus, "").min_size(egui::Vec2::new(26.0, 22.0)))
                .on_hover_text(t.font_decrease_tip)
                .clicked()
                && *font_size > 12.0
            {
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
            if ui
                .add(IconButton::new(Icon::Plus, "").min_size(egui::Vec2::new(26.0, 22.0)))
                .on_hover_text(t.font_increase_tip)
                .clicked()
                && *font_size < 36.0
            {
                *font_size += 1.0;
                *on_save_settings = true;
            }
        });

        ui.add_space(8.0);
        egui::Frame::new()
            .fill(if settings.bionic_reading {
                theme.panel_bg_color()
            } else {
                theme.bg_color()
            })
            .stroke(egui::Stroke::new(
                if settings.bionic_reading { 1.5 } else { 1.0 },
                if settings.bionic_reading {
                    theme.accent_color()
                } else {
                    Color32::from_gray(120).gamma_multiply(0.3)
                },
            ))
            .corner_radius(6.0)
            .inner_margin(egui::Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .checkbox(
                            &mut settings.bionic_reading,
                            RichText::new(t.bionic_reading_title)
                                .strong()
                                .size(15.0)
                                .color(text_color),
                        )
                        .changed()
                    {
                        *on_save_settings = true;
                    }
                });
                ui.add_space(2.0);
                ui.label(
                    RichText::new(t.bionic_reading_desc)
                        .size(12.0)
                        .color(text_color.gamma_multiply(0.75)),
                );
            });

        ui.add_space(10.0);
        egui::Frame::new()
            .fill(theme.bg_color())
            .stroke(egui::Stroke::new(1.0, theme.accent_color().gamma_multiply(0.25)))
            .corner_radius(6.0)
            .inner_margin(egui::Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.label(RichText::new(t.live_preview).italics().size(12.0).color(text_color.gamma_multiply(0.65)));
                ui.add_space(6.0);
                let preview_job = super::text_view::create_sample_preview_job(
                    t.sample_text,
                    *font_size,
                    text_color,
                    settings.bionic_reading,
                    settings.ui_language.is_rtl(),
                );
                ui.label(preview_job);
            });
    }

    fn render_appearance_tab(
        ui: &mut egui::Ui,
        theme: &mut ReaderTheme,
        keep_screen_on: &mut bool,
        settings: &mut AppSettings,
        on_save_settings: &mut bool,
    ) {
        let t = settings.ui_language.t();
        let text_color = theme.text_color();

        ui.label(
            RichText::new(t.interface_language)
                .size(17.0)
                .strong()
                .color(text_color),
        );
        ui.label(
            RichText::new(t.interface_language_desc)
                .size(13.0)
                .color(text_color.gamma_multiply(0.75)),
        );
        ui.add_space(6.0);

        ui.horizontal_wrapped(|ui| {
            for &lang in crate::i18n::UiLanguage::ALL.iter() {
                let is_sel = settings.ui_language == lang;
                let text = lang.label(t);
                if ui.selectable_label(is_sel, text).clicked() && !is_sel {
                    settings.ui_language = lang;
                    *on_save_settings = true;
                }
            }
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        ui.label(
            RichText::new(t.theme_title)
                .size(17.0)
                .strong()
                .color(text_color),
        );
        ui.add_space(6.0);

        let themes = [
            (
                ReaderTheme::Sepia,
                t.theme_sepia,
                t.theme_sepia_desc,
            ),
            (
                ReaderTheme::Twilight,
                t.theme_twilight,
                t.theme_twilight_desc,
            ),
            (
                ReaderTheme::Warm,
                t.theme_warm,
                t.theme_warm_desc,
            ),
            (
                ReaderTheme::Mint,
                t.theme_mint,
                t.theme_mint_desc,
            ),
            (
                ReaderTheme::Light,
                t.theme_light,
                t.theme_light_desc,
            ),
            (
                ReaderTheme::Dark,
                t.theme_dark,
                t.theme_dark_desc,
            ),
            (
                ReaderTheme::Oled,
                t.theme_oled,
                t.theme_oled_desc,
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

        let label = RichText::new(t.keep_screen_on)
            .size(14.0)
            .strong()
            .color(text_color);
        if ui.checkbox(keep_screen_on, label).changed() {
            crate::platform::set_keep_screen_on(*keep_screen_on);
            settings.keep_screen_on = *keep_screen_on;
            *on_save_settings = true;
        }
        ui.label(
            RichText::new(t.keep_screen_on_desc)
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
        let t = settings.ui_language.t();
        let text_color = theme.text_color();

        ui.label(
            RichText::new(t.autoscroll_title)
                .size(17.0)
                .strong()
                .color(text_color),
        );
        ui.label(
            RichText::new(t.autoscroll_desc)
                .size(13.0)
                .color(text_color.gamma_multiply(0.75)),
        );
        ui.add_space(8.0);

        let (scroll_icon, scroll_text) = if *is_autoscrolling {
            (Icon::Pause, t.autoscroll_stop)
        } else {
            (Icon::Play, t.autoscroll_start)
        };
        if ui
            .add(IconButton::new(scroll_icon, scroll_text).font_size(14.0))
            .clicked()
        {
            *on_toggle_autoscroll = true;
        }

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(t.motion_type).strong().color(text_color));
            if ui
                .radio_value(
                    &mut settings.auto_scroll.pause_mode,
                    true,
                    t.mode_pause,
                )
                .clicked()
            {
                *on_save_settings = true;
            }
            if ui
                .radio_value(
                    &mut settings.auto_scroll.pause_mode,
                    false,
                    t.mode_continuous,
                )
                .clicked()
            {
                *on_save_settings = true;
            }
        });

        ui.add_space(8.0);
        if settings.auto_scroll.pause_mode {
            ui.horizontal(|ui| {
                ui.label(t.pause_duration);
                let resp = ui.add(
                    egui::Slider::new(&mut settings.auto_scroll.pause_duration, 0.5..=15.0)
                        .suffix(" сек"),
                );
                if resp.drag_stopped() || resp.lost_focus() {
                    *on_save_settings = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(t.step_distance);
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
                ui.label(t.scroll_speed);
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
        let t = settings.ui_language.t();
        let text_color = theme.text_color();
        ui.label(
            RichText::new(t.comics_title)
                .size(17.0)
                .strong()
                .color(text_color),
        );
        ui.add_space(6.0);

        let modes = [
            (
                "Auto",
                t.mode_auto,
                t.mode_auto_desc,
            ),
            (
                "Manga",
                t.mode_manga,
                t.mode_manga_desc,
            ),
            (
                "Comic",
                t.mode_comic,
                t.mode_comic_desc,
            ),
            (
                "Webtoon",
                t.mode_webtoon,
                t.mode_webtoon_desc,
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
            RichText::new(t.layout_title)
                .strong()
                .color(text_color),
        );
        ui.horizontal(|ui| {
            let s1 = ui.radio_value(
                &mut settings.comic.page_layout,
                "Single".to_string(),
                t.layout_single,
            );
            let s2 = ui.radio_value(
                &mut settings.comic.page_layout,
                "Double".to_string(),
                t.layout_double,
            );
            if s1.clicked() || s2.clicked() {
                *on_save_settings = true;
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (val, label) in [
                ("FitWindow", t.fit_window),
                ("FitWidth", t.fit_width),
                ("FitHeight", t.fit_height),
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

    fn render_hotkeys_tab(ui: &mut egui::Ui, theme: &ReaderTheme, lang: crate::i18n::UiLanguage) {
        let t = lang.t();
        let text_color = theme.text_color();
        ui.label(
            RichText::new(t.tab_hotkeys)
                .size(17.0)
                .strong()
                .color(text_color),
        );
        ui.add_space(6.0);

        let hotkeys = [
            ("O", "Открыть книгу или документ (файл)"),
            ("P", "Открыть папку с мангой или комиксом"),
            ("F / Esc", "Полноэкранный режим без панелей (вход и выход)"),
            ("B", "Показать / скрыть оглавление книги"),
            ("V", "Включить / приостановить чтение книги вслух (TTS)"),
            ("J", "Открыть / закрыть окно настроек читалки"),
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
                "Ctrl + F",
                "Поиск по тексту книги с быстрым переходом к главе",
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
        updater: &crate::updater::UpdateChecker,
    ) {
        let t = settings.ui_language.t();
        let text_color = theme.text_color();
        let card_stroke = egui::Stroke::new(1.0, theme.accent_color().gamma_multiply(0.25));

        ui.label(
            RichText::new(t.system_title)
                .size(17.0)
                .strong()
                .color(text_color),
        );
        ui.add_space(6.0);

        egui::Frame::new()
            .fill(theme.bg_color())
            .stroke(card_stroke)
            .corner_radius(6.0)
            .inner_margin(egui::Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(t.update_title)
                        .strong()
                        .color(text_color),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("{}: v{}", t.update_current_version, crate::updater::CURRENT_VERSION))
                        .size(12.0)
                        .color(text_color.gamma_multiply(0.8)),
                );
                ui.add_space(6.0);
                if ui
                    .checkbox(&mut settings.check_updates_on_startup, t.update_auto_check)
                    .changed()
                {
                    *on_save_settings = true;
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button(t.update_check_btn).clicked() {
                        updater.check_now();
                    }
                    match updater.status() {
                        crate::updater::UpdateStatus::Idle => {}
                        crate::updater::UpdateStatus::Checking => {
                            ui.label(RichText::new(t.update_checking).color(theme.accent_color()));
                        }
                        crate::updater::UpdateStatus::UpToDate { version } => {
                            ui.label(
                                RichText::new(format!("{} (v{})", t.update_up_to_date, version))
                                    .color(Color32::from_rgb(46, 139, 87))
                                    .strong(),
                            );
                        }
                        crate::updater::UpdateStatus::UpdateAvailable(ref info) => {
                            let msg = format!("{} v{}!", t.update_available, info.latest_version);
                            ui.label(
                                RichText::new(msg)
                                    .color(Color32::from_rgb(220, 140, 20))
                                    .strong(),
                            );
                            if ui.button(t.update_download).clicked() {
                                ui.ctx().open_url(egui::OpenUrl::new_tab(&info.release_url));
                            }
                        }
                        crate::updater::UpdateStatus::Error(ref err) => {
                            ui.label(
                                RichText::new(format!("{}: {}", t.update_error, err))
                                    .color(Color32::RED)
                                    .size(11.0),
                            );
                        }
                    }
                });
            });
        ui.add_space(8.0);

        if crate::platform::supports_file_association() {
            egui::Frame::new()
                .fill(theme.bg_color())
                .stroke(card_stroke)
                .corner_radius(6.0)
                .inner_margin(egui::Margin::symmetric(12, 10))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(t.assoc_title)
                            .strong()
                            .color(text_color),
                    );
                    ui.add_space(4.0);
                    ui.label(t.assoc_hint);
                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        if ui.small_button(t.select_all).clicked() {
                            settings.associated_extensions =
                                crate::platform::SUPPORTED_EXTENSIONS_INFO
                                    .iter()
                                    .map(|(ext, _)| ext.to_string())
                                    .collect();
                            *on_save_settings = true;
                        }
                        if ui.small_button(t.deselect_all).clicked() {
                            settings.associated_extensions.clear();
                            *on_save_settings = true;
                        }
                        if ui.small_button(t.only_books).clicked() {
                            settings.associated_extensions =
                                vec!["fb2".to_string(), "epub".to_string()];
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
                                        ui.add(
                                            IconLabel::new(Icon::Check, t.status_default)
                                                .size(11.0)
                                                .color(Color32::from_rgb(46, 139, 87))
                                                .strong(),
                                        );
                                    }
                                    crate::platform::AssociationStatus::Registered => {
                                        ui.label(
                                            RichText::new(t.status_registered)
                                                .size(11.0)
                                                .color(Color32::GRAY),
                                        );
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
                    if ui
                        .add_enabled(
                            has_selected,
                            egui::Button::new(
                                RichText::new(t.register_btn)
                                    .strong(),
                            ),
                        )
                        .clicked()
                    {
                        *on_register_association = true;
                    }

                    ui.add_space(8.0);
                    if ui
                        .checkbox(
                            &mut settings.dismiss_association_prompt,
                            t.dont_show_assoc,
                        )
                        .changed()
                    {
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
                    RichText::new(t.settings_storage)
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
                    .add(IconButton::new(Icon::Check, t.save_settings_now))
                    .clicked()
                {
                    *on_save_settings = true;
                }
            });
    }
}
