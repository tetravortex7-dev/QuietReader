use crate::settings::AutoScrollSettings;
use eframe::egui;

/// Состояние и контроллер автоматической промотки и перетаскивания страницы зажатым колёсиком
#[derive(Debug, Default, Clone)]
pub struct AutoscrollController {
    pub is_active: bool,
    pub phase_elapsed: f32,
    pub in_pause: bool,
    pub step_start_scroll_y: f32,
    pub target_step_scroll_y: f32,
    pub is_middle_dragging: bool,
}

impl AutoscrollController {
    /// Переключает состояние активности автопрокрутки
    pub fn toggle(&mut self, current_scroll_y: f32, max_scroll_y: f32, step_distance: f32) {
        self.is_active = !self.is_active;
        self.phase_elapsed = 0.0;
        self.in_pause = false;
        self.step_start_scroll_y = current_scroll_y;
        self.target_step_scroll_y = (current_scroll_y + step_distance).min(max_scroll_y);
    }

    /// Возвращает активность автопрокрутки
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// Останавливает автопрокрутку
    pub fn stop(&mut self) {
        self.is_active = false;
    }

    /// Выполняет обновление положения при автопрокрутке для текущего кадра
    pub fn update(
        &mut self,
        ctx: &egui::Context,
        settings: &AutoScrollSettings,
        current_scroll_y: f32,
        max_scroll_y: f32,
        _total_chapters: usize,
        current_chapter: &mut usize,
        pending_scroll_y: &mut Option<f32>,
        last_rendered_chapter: &mut usize,
        next_non_empty_chapter: Option<usize>,
    ) {
        if !self.is_active {
            return;
        }

        let dt = ctx.input(|i| i.stable_dt).clamp(0.001, 0.1);
        ctx.request_repaint();

        let at_bottom = current_scroll_y + 4.0 >= max_scroll_y;

        if at_bottom {
            if let Some(next_idx) = next_non_empty_chapter {
                *current_chapter = next_idx;
                *pending_scroll_y = Some(0.0);
                *last_rendered_chapter = *current_chapter;
                self.phase_elapsed = 0.0;
                self.in_pause = true; // небольшая пауза в начале нового раздела
            } else {
                // Конец книги
                self.is_active = false;
            }
        } else if !settings.pause_mode {
            // Плавная непрерывная прокрутка
            let speed = settings.continuous_speed.clamp(5.0, 400.0);
            let next_y = (current_scroll_y + speed * dt).min(max_scroll_y);
            *pending_scroll_y = Some(next_y);
        } else {
            // Режим с паузами (шаг + остановка для чтения)
            let pause_dur = settings.pause_duration.max(0.2);
            let move_dur = settings.step_duration.max(0.1);

            self.phase_elapsed += dt;

            if self.in_pause {
                if self.phase_elapsed >= pause_dur {
                    self.in_pause = false;
                    self.phase_elapsed = 0.0;
                    self.step_start_scroll_y = current_scroll_y;
                    self.target_step_scroll_y =
                        (current_scroll_y + settings.step_distance).min(max_scroll_y);
                }
            } else {
                let progress = (self.phase_elapsed / move_dur).clamp(0.0, 1.0);
                // Плавная интерполяция Hermite (ease-in-out)
                let smooth_t = progress * progress * (3.0 - 2.0 * progress);
                let current_target = self.step_start_scroll_y
                    + (self.target_step_scroll_y - self.step_start_scroll_y) * smooth_t;
                *pending_scroll_y = Some(current_target);

                if self.phase_elapsed >= move_dur {
                    self.in_pause = true;
                    self.phase_elapsed = 0.0;
                }
            }
        }
    }

    /// Перетаскивание страницы вверх/вниз при зажатом колёсике мыши (захват страницы рукой)
    pub fn handle_middle_click_scroll(
        &mut self,
        ctx: &egui::Context,
        show_toc: bool,
        current_scroll_y: f32,
        max_scroll_y: f32,
        pending_scroll_y: &mut Option<f32>,
    ) {
        let (pressed, down, delta_y, over_toc) = ctx.input(|i| {
            let p = &i.pointer;
            let over_toc = show_toc && p.latest_pos().map(|pos| pos.x < 280.0).unwrap_or(false);
            (
                p.button_pressed(egui::PointerButton::Middle),
                p.button_down(egui::PointerButton::Middle),
                p.delta().y,
                over_toc,
            )
        });

        if pressed && !over_toc {
            self.is_middle_dragging = true;
            self.is_active = false;
        }

        if self.is_middle_dragging {
            if down {
                ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
                if delta_y.abs() > 0.0 {
                    let base_y = pending_scroll_y.unwrap_or(current_scroll_y);
                    let new_scroll = (base_y - delta_y).clamp(0.0, max_scroll_y);
                    *pending_scroll_y = Some(new_scroll);
                }
            } else {
                self.is_middle_dragging = false;
            }
        }
    }
}
