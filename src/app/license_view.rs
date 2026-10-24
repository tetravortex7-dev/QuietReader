use super::icons::{Icon, IconButton, IconLabel};
use super::theme::ReaderTheme;
use crate::core::license::{LicenseError, LicenseStatus, ProFeature, TrialError, unix_now};
use crate::i18n::{Translations, fill};
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, RichText};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LicenseNotice {
    TrialStarted,
    Activated,
}

#[derive(Debug, Default)]
pub struct ProUpgradeState {
    pub is_open: bool,
    pub feature: Option<ProFeature>,
    pub key_input: String,
    pub error: Option<LicenseError>,
    pub trial_error: bool,
    pub notice: Option<LicenseNotice>,
}

impl ProUpgradeState {
    pub fn request(&mut self, feature: ProFeature) {
        self.open(Some(feature));
    }

    pub fn open(&mut self, feature: Option<ProFeature>) {
        self.is_open = true;
        self.feature = feature;
        self.error = None;
        self.trial_error = false;
        self.notice = None;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.feature = None;
        self.error = None;
        self.trial_error = false;
        self.notice = None;
    }

    fn start_trial(&mut self, settings: &mut AppSettings, on_save: &mut bool) {
        match settings.start_trial(unix_now()) {
            Ok(()) => {
                *on_save = true;
                self.trial_error = false;
                self.notice = Some(LicenseNotice::TrialStarted);
            }
            Err(TrialError::AlreadyStarted) => {
                self.trial_error = true;
            }
        }
    }

    fn activate(&mut self, settings: &mut AppSettings, on_save: &mut bool) {
        match settings.activate_license(&self.key_input) {
            Ok(_) => {
                *on_save = true;
                self.error = None;
                self.notice = Some(LicenseNotice::Activated);
                self.key_input.clear();
            }
            Err(error) => {
                self.error = Some(error);
            }
        }
    }
}

pub struct ProUpgradeDialog;

impl ProUpgradeDialog {
    pub fn render(
        ctx: &egui::Context,
        state: &mut ProUpgradeState,
        settings: &mut AppSettings,
        theme: &ReaderTheme,
        on_save: &mut bool,
    ) {
        if !state.is_open {
            return;
        }

        let t = settings.ui_language.t();
        let text_color = theme.text_color();
        let secondary = theme.secondary_text_color();
        let accent = theme.accent_color();
        let mut window_open = true;
        let mut close_requested = false;

        egui::Window::new(t.license_upgrade_title)
            .id(egui::Id::unique("pro_upgrade_dialog"))
            .open(&mut window_open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .default_width(520.0)
            .show(ctx, |ui| {
                ui.set_max_width(500.0);
                ui.horizontal(|ui| {
                    ui.add(
                        IconLabel::new(Icon::Pro, t.license_title)
                            .size(21.0)
                            .strong()
                            .color(accent),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(t.license_pro_badge)
                                .size(11.5)
                                .strong()
                                .color(accent),
                        );
                    });
                });
                ui.add_space(4.0);
                ui.label(
                    RichText::new(status_text(&t, &settings.license_status()))
                        .size(13.0)
                        .strong()
                        .color(text_color),
                );
                ui.add_space(6.0);

                if let Some(feature) = state.feature {
                    ui.label(
                        RichText::new(fill(
                            t.license_feature_locked,
                            &[("{feature}", feature_name(&t, feature))],
                        ))
                        .size(14.0)
                        .color(text_color),
                    );
                    ui.add_space(4.0);
                }

                ui.label(
                    RichText::new(t.license_upgrade_subtitle)
                        .size(12.5)
                        .color(secondary),
                );
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                ui.label(
                    RichText::new(t.license_features_title)
                        .size(13.0)
                        .strong()
                        .color(text_color),
                );
                ui.add_space(5.0);
                for feature in ProFeature::ALL {
                    ui.horizontal(|ui| {
                        ui.add(
                            IconLabel::new(Icon::Check, feature_name(&t, feature))
                                .size(12.5)
                                .color(text_color),
                        );
                    });
                }
                ui.add_space(8.0);
                ui.label(
                    RichText::new(t.license_free_forever)
                        .size(11.5)
                        .color(secondary),
                );
                ui.add_space(10.0);

                let status = settings.license_status();
                if !status.is_pro_active() {
                    if settings.trial_started_at_unix.is_none() {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(t.license_try_trial)
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .fill(accent)
                                .corner_radius(6.0)
                                .min_size(egui::Vec2::new(220.0, 30.0)),
                            )
                            .clicked()
                        {
                            state.start_trial(settings, on_save);
                        }
                        ui.add_space(4.0);
                    } else {
                        ui.label(
                            RichText::new(t.license_trial_one_time)
                                .size(11.5)
                                .color(secondary),
                        );
                        ui.add_space(4.0);
                    }
                }

                if state.trial_error {
                    ui.label(
                        RichText::new(t.license_trial_error)
                            .size(11.5)
                            .color(Color32::RED),
                    );
                    ui.add_space(4.0);
                }

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(t.license_key_label)
                            .size(12.5)
                            .color(text_color),
                    );
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut state.key_input)
                            .hint_text(t.license_key_hint)
                            .desired_width(300.0),
                    );
                    let submitted = response.lost_focus()
                        && ui.input(|input| input.key_pressed(egui::Key::Enter));
                    if (ui
                        .add(
                            IconButton::new(Icon::Key, t.license_activate)
                                .min_size(egui::Vec2::new(110.0, 26.0)),
                        )
                        .clicked()
                        || submitted)
                        && !state.key_input.trim().is_empty()
                    {
                        state.activate(settings, on_save);
                    }
                });
                ui.add_space(3.0);
                ui.label(
                    RichText::new(t.license_enter_key_hint)
                        .size(11.5)
                        .color(secondary),
                );

                if let Some(error) = state.error {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(error_text(&t, error))
                            .size(11.5)
                            .color(Color32::RED),
                    );
                }

                if let Some(notice) = state.notice {
                    ui.add_space(6.0);
                    let message = match notice {
                        LicenseNotice::TrialStarted => t.license_trial_started,
                        LicenseNotice::Activated => t.license_activated_thanks,
                    };
                    ui.add(
                        IconLabel::new(Icon::Check, message)
                            .size(12.0)
                            .color(Color32::from_rgb(46, 139, 87))
                            .strong(),
                    );
                }

                ui.add_space(8.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(IconButton::new(Icon::Close, t.license_close))
                        .clicked()
                    {
                        close_requested = true;
                    }
                });
            });

        if !window_open || close_requested {
            state.close();
        }
    }
}

pub fn render_license_section(
    ui: &mut egui::Ui,
    settings: &mut AppSettings,
    theme: &ReaderTheme,
    state: &mut ProUpgradeState,
    on_save: &mut bool,
) {
    let t = settings.ui_language.t();
    let text_color = theme.text_color();
    let secondary = theme.secondary_text_color();
    let accent = theme.accent_color();
    let status = settings.license_status();

    egui::Frame::new()
        .fill(theme.bg_color())
        .stroke(egui::Stroke::new(1.0, accent.gamma_multiply(0.3)))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    IconLabel::new(Icon::Pro, t.license_manage)
                        .strong()
                        .color(text_color),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(t.license_pro_badge)
                            .size(11.0)
                            .strong()
                            .color(accent),
                    );
                });
            });
            ui.add_space(5.0);
            ui.label(
                RichText::new(status_text(&t, &status))
                    .size(13.0)
                    .strong()
                    .color(text_color),
            );
            if let LicenseStatus::ProActivated { email, .. } = &status {
                ui.add_space(2.0);
                ui.label(
                    RichText::new(fill(t.license_licensed_to, &[("{email}", email)]))
                        .size(11.5)
                        .color(secondary),
                );
            }
            ui.add_space(6.0);

            ui.horizontal_wrapped(|ui| {
                if !status.is_pro_active() {
                    if settings.trial_started_at_unix.is_none()
                        && ui.button(t.license_try_trial).clicked()
                    {
                        state.start_trial(settings, on_save);
                    }
                    if ui
                        .add(IconButton::new(Icon::Key, t.license_key_label))
                        .clicked()
                    {
                        state.open(None);
                    }
                }
            });

            if state.trial_error {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(t.license_trial_error)
                        .size(11.0)
                        .color(Color32::RED),
                );
            }
            if status.is_trial_expired() {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(t.license_trial_one_time)
                        .size(11.0)
                        .color(secondary),
                );
            }
        });
}

pub(crate) fn status_text(t: &Translations, status: &LicenseStatus) -> String {
    match status {
        LicenseStatus::Free => t.license_free_status.to_string(),
        LicenseStatus::TrialActive { days_left, .. } => fill(
            t.license_trial_status,
            &[("{days}", &days_left.to_string())],
        ),
        LicenseStatus::TrialExpired => t.license_trial_expired.to_string(),
        LicenseStatus::ProActivated { .. } => t.license_pro_activated.to_string(),
    }
}

fn error_text(t: &Translations, error: LicenseError) -> &'static str {
    match error {
        LicenseError::InvalidFormat => t.license_invalid_format,
        LicenseError::InvalidSignature => t.license_invalid_signature,
        LicenseError::InvalidPayload => t.license_invalid_payload,
        LicenseError::InvalidPublicKey => t.license_internal_key_error,
    }
}

fn feature_name(t: &Translations, feature: ProFeature) -> &'static str {
    match feature {
        ProFeature::NeuralTts => t.license_feature_neural_tts,
        ProFeature::Translation => t.license_feature_translation,
        ProFeature::OpdsPro => t.license_feature_opds,
        ProFeature::FocusTools => t.license_feature_focus,
        ProFeature::AmbientAudio => t.license_feature_ambient,
        ProFeature::SmartScanProcessing => t.license_feature_smart_scan,
    }
}
