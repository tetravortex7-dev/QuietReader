use crate::core::Command;
use eframe::egui;

pub fn process_keyboard_events(
    wants_keyboard_input: bool,
    key_pressed: impl Fn(egui::Key) -> bool,
    modifiers: egui::Modifiers,
    is_manga: bool,
) -> Vec<Command> {
    if wants_keyboard_input {
        return Vec::new();
    }

    let mut cmds = Vec::new();
    let ctrl = modifiers.ctrl || modifiers.command || modifiers.alt;
    let cmd_mod = modifiers.ctrl || modifiers.command;
    let shift = modifiers.shift;

    if cmd_mod && key_pressed(egui::Key::A) {
        cmds.push(Command::CopyChapterText);
    } else if !ctrl && key_pressed(egui::Key::A) {
        cmds.push(Command::ToggleAutoscroll);
    }
    if cmd_mod && (key_pressed(egui::Key::Plus) || key_pressed(egui::Key::Equals)) {
        cmds.push(Command::ZoomIn);
    } else if cmd_mod && key_pressed(egui::Key::Minus) {
        cmds.push(Command::ZoomOut);
    } else if cmd_mod && key_pressed(egui::Key::Num0) {
        cmds.push(Command::ResetZoom);
    }
    if cmd_mod && key_pressed(egui::Key::F) {
        cmds.push(Command::ToggleSearch);
    } else if !ctrl && key_pressed(egui::Key::F) {
        cmds.push(Command::ToggleFullscreen);
    } else if (!ctrl && key_pressed(egui::Key::V))
        || key_pressed(egui::Key::F4)
        || (cmd_mod && shift && key_pressed(egui::Key::S))
    {
        cmds.push(Command::ToggleTTS);
    } else if !ctrl && key_pressed(egui::Key::B) {
        cmds.push(Command::ToggleToc);
    } else if !ctrl && key_pressed(egui::Key::J) {
        cmds.push(Command::ToggleSettings);
    } else if !ctrl && key_pressed(egui::Key::L) {
        cmds.push(Command::ToggleReadingRuler);
    } else if cmd_mod && key_pressed(egui::Key::D) {
        cmds.push(Command::ToggleBookmark);
    } else if !ctrl && key_pressed(egui::Key::M) {
        cmds.push(Command::ToggleAmbient);
    } else if key_pressed(egui::Key::Escape) {
        cmds.push(Command::ExitFullscreen);
    }
    if ctrl && (key_pressed(egui::Key::ArrowRight) || key_pressed(egui::Key::ArrowDown)) {
        cmds.push(Command::NextChapter);
    } else if ctrl && (key_pressed(egui::Key::ArrowLeft) || key_pressed(egui::Key::ArrowUp)) {
        cmds.push(Command::PrevChapter);
    }

    if key_pressed(egui::Key::PageDown) || (!shift && key_pressed(egui::Key::Space)) {
        cmds.push(Command::PageDown);
    } else if key_pressed(egui::Key::PageUp) || (shift && key_pressed(egui::Key::Space)) {
        cmds.push(Command::PageUp);
    } else if key_pressed(egui::Key::ArrowDown) {
        cmds.push(Command::LineDown);
    } else if key_pressed(egui::Key::ArrowUp) {
        cmds.push(Command::LineUp);
    } else if key_pressed(egui::Key::Home) {
        cmds.push(Command::Home);
    } else if key_pressed(egui::Key::End) {
        cmds.push(Command::End);
    } else if !ctrl && key_pressed(egui::Key::ArrowLeft) {
        cmds.push(if is_manga {
            Command::NextPage
        } else {
            Command::PrevPage
        });
    } else if !ctrl && key_pressed(egui::Key::ArrowRight) {
        cmds.push(if is_manga {
            Command::PrevPage
        } else {
            Command::NextPage
        });
    }
    cmds
}

pub fn poll_keyboard_commands(ctx: &egui::Context, is_manga: bool) -> Vec<Command> {
    if ctx.egui_wants_keyboard_input() {
        return Vec::new();
    }
    ctx.input(|i| process_keyboard_events(false, |k| i.key_pressed(k), i.modifiers, is_manga))
}
