use super::QuietReaderApp;
use crate::core::library::unix_day_index;
use eframe::egui;
use std::time::{Duration, Instant};

pub const READING_ACTIVITY_GRACE: Duration = Duration::from_secs(30);
const LIBRARY_SAVE_INTERVAL: Duration = Duration::from_secs(60);
const MAX_ACCRUAL_DT: f32 = 0.25;

pub fn is_reading_active(
    has_book: bool,
    library_visible: bool,
    blocking_dialog: bool,
    tts_active: bool,
    autoscroll_active: bool,
    recently_active: bool,
) -> bool {
    has_book
        && !library_visible
        && !blocking_dialog
        && (tts_active || autoscroll_active || recently_active)
}

impl QuietReaderApp {
    pub(crate) fn accrue_reading_time(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        let user_active = ctx.input(|input| {
            input.pointer.delta().length_sq() > 0.0
                || input.pointer.any_down()
                || input.pointer.any_click()
                || input.smooth_scroll_delta != egui::Vec2::ZERO
                || input.events.iter().any(|event| {
                    matches!(
                        event,
                        egui::Event::Key { pressed: true, .. }
                            | egui::Event::Text(_)
                            | egui::Event::Paste(_)
                            | egui::Event::Copy
                            | egui::Event::Cut
                    )
                })
        });
        if user_active {
            self.last_reading_activity = Some(now);
        }

        let tts_active = {
            let status = self.tts.status();
            status.is_playing && !status.is_paused
        };
        let recently_active = self
            .last_reading_activity
            .map(|time| now.duration_since(time) <= READING_ACTIVITY_GRACE)
            .unwrap_or(false);
        let blocking_dialog = self.show_settings_dialog
            || self.show_recent_dialog
            || self.show_association_dialog
            || self.show_opds_dialog;
        let active = is_reading_active(
            self.document.is_some() && self.current_file_path.is_some(),
            self.show_library,
            blocking_dialog,
            tts_active,
            self.autoscroll.is_active(),
            recently_active,
        );
        if !active {
            self.reading_seconds_accumulator = 0.0;
        } else {
            let dt = ctx
                .input(|input| input.stable_dt)
                .clamp(0.0, MAX_ACCRUAL_DT);
            self.reading_seconds_accumulator += dt;
            if self.reading_seconds_accumulator >= 1.0 {
                let whole_seconds = self.reading_seconds_accumulator.floor() as u64;
                self.reading_seconds_accumulator -= whole_seconds as f32;
                if whole_seconds > 0 {
                    let day = unix_day_index(crate::core::license::unix_now());
                    if self.library.record_reading_seconds(day, whole_seconds) {
                        self.library_dirty = true;
                    }
                }
            }
        }
        if self.library_dirty && self.last_library_save.elapsed() >= LIBRARY_SAVE_INTERVAL {
            self.save_library_now();
        }
    }
}
