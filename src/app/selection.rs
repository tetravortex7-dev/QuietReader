use eframe::egui;

const SELECTION_ID: &str = "latest_text_selection";
const INTERNAL_FLAG: &str = "internal_selection_copy_flag";
const SAVED_STATE_ID: &str = "saved_label_selection_state";

pub struct SelectionCapturePlugin;

impl egui::plugin::Plugin for SelectionCapturePlugin {
    fn debug_name(&self) -> &'static str {
        "SelectionCapturePlugin"
    }

    fn on_end_pass(&mut self, ui: &mut egui::Ui) {
        let is_secondary_or_menu = ui.input(|i| {
            i.pointer.button_down(egui::PointerButton::Secondary)
                || i.pointer.button_pressed(egui::PointerButton::Secondary)
                || i.pointer.button_released(egui::PointerButton::Secondary)
        }) || egui::Popup::is_any_open(ui.ctx());

        let plugin_handle = ui
            .ctx()
            .plugin::<egui::text_selection::LabelSelectionState>();
        let has_sel = plugin_handle.lock().has_selection();
        let primary_click_empty = ui.input(|i| {
            i.pointer.button_released(egui::PointerButton::Primary) && !i.modifiers.shift
        }) && !has_sel;
        let escape_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));

        ui.ctx().data_mut(|d| {
            let state_id = egui::Id::unique(SAVED_STATE_ID);
            let sel_id = egui::Id::unique(SELECTION_ID);
            if has_sel && !is_secondary_or_menu {
                d.insert_temp(state_id, plugin_handle.lock().clone());
            } else if is_secondary_or_menu {
                if let Some(saved) =
                    d.get_temp::<egui::text_selection::LabelSelectionState>(state_id)
                {
                    *plugin_handle.lock() = saved;
                }
            } else if primary_click_empty || escape_pressed {
                d.remove_temp::<String>(sel_id);
                d.remove_temp::<egui::text_selection::LabelSelectionState>(state_id);
            }
        });
    }

    fn output_hook(&mut self, ctx: &egui::Context, output: &mut egui::FullOutput) {
        let is_internal: bool = ctx.data_mut(|d| {
            d.remove_temp(egui::Id::unique(INTERNAL_FLAG))
                .unwrap_or(false)
        });

        for cmd in &output.platform_output.commands {
            match cmd {
                egui::OutputCommand::TextSelectionSettled(text)
                | egui::OutputCommand::CopyText(text) => {
                    if !text.trim().is_empty() {
                        let sel_id = egui::Id::unique(SELECTION_ID);
                        ctx.data_mut(|d| d.insert_temp(sel_id, text.clone()));
                    }
                }
                _ => {}
            }
        }

        if is_internal {
            output
                .platform_output
                .commands
                .retain(|cmd| !matches!(cmd, egui::OutputCommand::CopyText(_)));
        }
    }
}

pub fn get_selected_text(ctx: &egui::Context) -> Option<String> {
    ctx.data(|d| d.get_temp(egui::Id::unique(SELECTION_ID)))
}

pub fn has_active_selection(ctx: &egui::Context) -> bool {
    let has_plugin = ctx
        .plugin::<egui::text_selection::LabelSelectionState>()
        .lock()
        .has_selection();
    has_plugin || get_selected_text(ctx).map_or(false, |s| !s.trim().is_empty())
}

pub fn request_selection_capture(ctx: &egui::Context) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::unique(INTERNAL_FLAG), true));
    ctx.input_mut(|i| {
        if !i.events.iter().any(|e| matches!(e, egui::Event::Copy)) {
            i.events.push(egui::Event::Copy);
        }
    });
    ctx.request_repaint();
}

#[allow(dead_code)]
pub fn clear_selection(ctx: &egui::Context) {
    ctx.data_mut(|d| {
        d.remove_temp::<String>(egui::Id::unique(SELECTION_ID));
        d.remove_temp::<egui::text_selection::LabelSelectionState>(egui::Id::unique(
            SAVED_STATE_ID,
        ));
    });
}
