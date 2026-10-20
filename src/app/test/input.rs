#[test]
fn test_hotkeys_b_v_j() {
    use crate::app::input::process_keyboard_events;
    use crate::core::Command;
    use eframe::egui::{Key, Modifiers};

    let cmds_b = process_keyboard_events(false, |k| k == Key::B, Modifiers::default(), false);
    assert_eq!(cmds_b, vec![Command::ToggleToc]);

    let cmds_v = process_keyboard_events(false, |k| k == Key::V, Modifiers::default(), false);
    assert_eq!(cmds_v, vec![Command::ToggleTTS]);

    let cmds_j = process_keyboard_events(false, |k| k == Key::J, Modifiers::default(), false);
    assert_eq!(cmds_j, vec![Command::ToggleSettings]);

    let mut ctrl_mod = Modifiers::default();
    ctrl_mod.ctrl = true;
    let cmds_ctrl_v = process_keyboard_events(false, |k| k == Key::V, ctrl_mod, false);
    assert!(cmds_ctrl_v.is_empty());

    let cmds_typing = process_keyboard_events(
        true,
        |k| k == Key::B || k == Key::V || k == Key::J,
        Modifiers::default(),
        false,
    );
    assert!(cmds_typing.is_empty());
}

#[test]
fn test_hotkey_l_reading_ruler() {
    use crate::app::input::process_keyboard_events;
    use crate::core::Command;
    use eframe::egui::{Key, Modifiers};

    let cmds_l = process_keyboard_events(false, |k| k == Key::L, Modifiers::default(), false);
    assert_eq!(cmds_l, vec![Command::ToggleReadingRuler]);

    let mut ctrl_mod = Modifiers::default();
    ctrl_mod.ctrl = true;
    let cmds_ctrl_l = process_keyboard_events(false, |k| k == Key::L, ctrl_mod, false);
    assert!(cmds_ctrl_l.is_empty());

    let cmds_typing = process_keyboard_events(true, |k| k == Key::L, Modifiers::default(), false);
    assert!(cmds_typing.is_empty());
}

#[test]
fn test_reading_ruler_settings_and_translations() {
    use crate::i18n::UiLanguage;
    use crate::settings::AppSettings;

    let settings = AppSettings::default();
    assert!(!settings.reading_ruler);
    assert_eq!(settings.reading_ruler_lines, 3);
    assert!((settings.reading_ruler_opacity - 0.45).abs() < f32::EPSILON);

    for lang in UiLanguage::CONCRETE {
        let t = lang.t();
        assert!(!t.reading_ruler.is_empty());
        assert!(!t.hide_reading_ruler.is_empty());
        assert!(!t.reading_ruler_tooltip.is_empty());
        assert!(!t.reading_ruler_title.is_empty());
        assert!(!t.reading_ruler_desc.is_empty());
        assert!(!t.reading_ruler_height_label.is_empty());
        assert!(!t.reading_ruler_dim_label.is_empty());
    }
}

#[test]
fn test_keyboard_input_suppressed_when_wants_keyboard_input() {
    use crate::app::input::process_keyboard_events;
    use eframe::egui::{Key, Modifiers};

    let dummy_modifiers = Modifiers::default();
    let cmds = process_keyboard_events(
        true,
        |k| {
            matches!(
                k,
                Key::A | Key::Plus | Key::Space | Key::ArrowRight | Key::F
            )
        },
        dummy_modifiers,
        false,
    );
    assert!(cmds.is_empty());

    let cmds_not_focused =
        process_keyboard_events(false, |k| matches!(k, Key::F), dummy_modifiers, false);
    assert_eq!(
        cmds_not_focused,
        vec![crate::core::Command::ToggleFullscreen]
    );
}

#[test]
fn test_keyboard_manga_vs_comic_navigation_direction() {
    use crate::app::input::process_keyboard_events;
    use eframe::egui::{Key, Modifiers};

    let dummy_modifiers = Modifiers::default();

    let manga_left = process_keyboard_events(false, |k| k == Key::ArrowLeft, dummy_modifiers, true);
    assert_eq!(manga_left, vec![crate::core::Command::NextPage]);

    let manga_right =
        process_keyboard_events(false, |k| k == Key::ArrowRight, dummy_modifiers, true);
    assert_eq!(manga_right, vec![crate::core::Command::PrevPage]);

    let comic_left =
        process_keyboard_events(false, |k| k == Key::ArrowLeft, dummy_modifiers, false);
    assert_eq!(comic_left, vec![crate::core::Command::PrevPage]);

    let comic_right =
        process_keyboard_events(false, |k| k == Key::ArrowRight, dummy_modifiers, false);
    assert_eq!(comic_right, vec![crate::core::Command::NextPage]);
}

#[test]
fn test_autoscroll_controller_state_and_paged_advance() {
    use crate::app::autoscroll::AutoscrollController;

    let mut controller = AutoscrollController::default();
    assert!(!controller.is_active());

    controller.toggle(100.0, 500.0, 60.0);
    assert!(controller.is_active());
    assert_eq!(controller.step_start_scroll_y, 100.0);
    assert_eq!(controller.target_step_scroll_y, 160.0);

    controller.stop();
    assert!(!controller.is_active());
}
