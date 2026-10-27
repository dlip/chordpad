use chordpad::{
    chord::{Event, Recognizer},
    controller::Snapshot,
    model::*,
    render,
    typing::validate_pause,
};
use std::{fs, process::Command};

fn layout() -> Layout {
    serde_json::from_str(include_str!(
        "../android/app/src/main/assets/legacy-layout.json"
    ))
    .unwrap()
}

#[test]
fn function_keys_share_number_chords_and_have_twelve_reference_outputs() {
    let layout = layout();
    let functions = layout.function_bindings();
    assert_eq!(functions.len(), 16);
    for (digit, key) in [
        ('1', Key::F1),
        ('2', Key::F2),
        ('3', Key::F3),
        ('4', Key::F4),
        ('5', Key::F5),
        ('6', Key::F6),
        ('7', Key::F7),
        ('8', Key::F8),
        ('9', Key::F9),
        ('0', Key::F10),
    ] {
        let number = layout
            .bindings
            .iter()
            .find(|b| b.action == Action::Character(digit) && b.held == [Button::R2])
            .unwrap();
        let function = functions
            .iter()
            .find(|b| b.action == Action::Key(key))
            .unwrap();
        assert_eq!(function.chord, number.chord);
        assert_eq!(function.held, [Button::R1, Button::R2]);
    }
    let html = render::html(&layout).unwrap();
    for key in 1..=12 {
        assert!(html.contains(&format!("<span>F{key}</span>")));
    }
    assert!(!html.contains("<svg"));
    assert!(html.contains("R2 + R1: function keys"));
    assert!(html.contains("R2: numbers"));
    for (key, button) in [(Key::F11, Button::X), (Key::F12, Button::Y)] {
        assert_eq!(
            functions
                .iter()
                .find(|b| b.action == Action::Key(key))
                .unwrap()
                .chord,
            Chord {
                dpad: Dpad::Neutral,
                buttons: vec![button]
            }
        );
    }
}

#[test]
fn bundled_numbers_follow_clockwise_dpad_and_face_positions() {
    let layout = layout();
    for (digit, dpad) in ('1'..='8').zip([
        Dpad::Up,
        Dpad::UpRight,
        Dpad::Right,
        Dpad::DownRight,
        Dpad::Down,
        Dpad::DownLeft,
        Dpad::Left,
        Dpad::UpLeft,
    ]) {
        let binding = layout
            .bindings
            .iter()
            .find(|b| b.action == Action::Character(digit))
            .unwrap();
        assert_eq!(
            binding.chord,
            Chord {
                dpad,
                buttons: vec![]
            }
        );
        assert_eq!(binding.held, [Button::R2]);
    }
    for (digit, button) in [('9', Button::A), ('0', Button::B), ('=', Button::Y)] {
        assert_eq!(
            layout
                .bindings
                .iter()
                .find(|b| b.action == Action::Character(digit))
                .unwrap()
                .chord,
            Chord {
                dpad: Dpad::Neutral,
                buttons: vec![button]
            }
        );
    }
}

#[test]
fn older_layouts_keep_default_f11_f12_and_custom_pairs_are_supported() {
    let mut legacy = layout();
    legacy.profile.function_volume = false;
    legacy.profile.function_print_screen = false;
    legacy.profile.extra_function_chords = Profile::default().extra_function_chords;
    legacy
        .bindings
        .retain(|b| matches!(b.action,Action::Character(c) if c.is_ascii_digit()));
    for (digit, button) in [('5', Button::A), ('7', Button::X), ('9', Button::Y)] {
        legacy
            .bindings
            .iter_mut()
            .find(|b| b.action == Action::Character(digit))
            .unwrap()
            .chord = Chord {
            dpad: Dpad::Neutral,
            buttons: vec![button],
        };
    }
    let json = serde_json::to_value(legacy).unwrap();
    assert!(json["profile"].get("extra_function_chords").is_none());
    let legacy: Layout = serde_json::from_value(json).unwrap();
    legacy.validate().unwrap();
    for (key, dpad) in [(Key::F11, Dpad::Left), (Key::F12, Dpad::Down)] {
        assert_eq!(
            legacy
                .function_bindings()
                .iter()
                .find(|b| b.action == Action::Key(key))
                .unwrap()
                .chord
                .dpad,
            dpad
        );
    }
    let mut custom = layout();
    custom.profile.function_volume = false;
    custom.profile.extra_function_chords[0].buttons = vec![Button::A, Button::X];
    custom.validate().unwrap();
    let mut engine = Recognizer::new(
        custom.strategy.clone(),
        custom.profile.clone(),
        Some(custom.clone()),
    )
    .unwrap();
    let state = |buttons| Snapshot {
        buttons,
        menu_buttons: vec![],
        dpad: Some(Dpad::Neutral),
        l2: 0.0,
        r2: 0.0,
        left_stick: Dpad::Neutral,
        right_stick: Dpad::Neutral,
    };
    engine.update(&state(vec![]));
    engine.update(&state(vec![Button::R1, Button::R2]));
    engine.update(&state(vec![Button::R1, Button::R2, Button::A]));
    engine.update(&state(vec![Button::R1, Button::R2, Button::A, Button::X]));
    assert!(
        engine
            .update(&state(vec![Button::R1, Button::R2, Button::A]))
            .is_empty()
    );
    assert!(matches!(
        engine
            .update(&state(vec![Button::R1, Button::R2]))
            .as_slice(),
        [Event::Chord {
            action: Some(Action::Key(Key::F11)),
            ..
        }]
    ));
    custom.profile.extra_function_chords[1] = custom.profile.extra_function_chords[0].clone();
    assert!(custom.validate().is_err());
}

#[test]
fn volume_controls_are_optional_and_reserve_their_function_chords() {
    let mut layout = layout();
    assert_eq!(layout.function_bindings().len(), 16);
    layout.profile.extra_function_chords[0].buttons = vec![Button::A, Button::X];
    assert!(layout.validate().is_err());
    layout.profile.function_volume = false;
    layout.validate().unwrap();
    assert_eq!(layout.function_bindings().len(), 13);
    assert!(
        serde_json::to_value(&layout).unwrap()["profile"]
            .get("function_volume")
            .is_none()
    );
    layout.profile.function_volume = true;
    layout.profile.function_layer = false;
    assert!(layout.validate().unwrap_err().contains("function_volume"));
}

#[test]
fn print_screen_is_optional_and_reserves_its_function_chord() {
    let mut layout = layout();
    let binding = layout
        .function_bindings()
        .into_iter()
        .find(|b| b.action == Action::Key(Key::PrintScreen))
        .unwrap();
    assert_eq!(binding.chord.buttons, [Button::B, Button::Y]);
    assert_eq!(binding.chord.dpad, Dpad::Neutral);
    assert_eq!(binding.held, [Button::R1, Button::R2]);
    #[cfg(target_os = "linux")]
    assert_eq!(
        chordpad::output::stroke(&binding.action).unwrap(),
        [evdev::KeyCode::KEY_SYSRQ]
    );
    layout.profile.extra_function_chords[0] = binding.chord.clone();
    assert!(layout.validate().is_err());
    layout.profile.function_print_screen = false;
    layout.validate().unwrap();
    assert!(
        !layout
            .function_bindings()
            .iter()
            .any(|b| b.action == binding.action)
    );
    assert!(
        serde_json::to_value(&layout).unwrap()["profile"]
            .get("function_print_screen")
            .is_none()
    );
    layout.profile.function_print_screen = true;
    layout.profile.function_volume = false;
    layout.profile.function_layer = false;
    assert!(
        layout
            .validate()
            .unwrap_err()
            .contains("function_print_screen")
    );
}

#[test]
fn invalid_function_layouts_and_pause_conflicts_are_rejected() {
    let original = layout();
    let mut missing = original.clone();
    missing
        .bindings
        .retain(|b| b.action != Action::Character('0'));
    assert!(
        missing
            .validate()
            .unwrap_err()
            .contains("ten number bindings")
    );
    let mut collision = original.clone();
    for binding in &mut collision.bindings {
        if binding.action.group(true).0 == Action::Character('1') {
            binding.chord = Chord {
                dpad: Dpad::Neutral,
                buttons: vec![Button::A],
            };
        }
    }
    assert!(collision.validate().is_err());
    let mut forbidden = original.clone();
    forbidden
        .profile
        .forbidden_combinations
        .push(vec![Button::R1, Button::R2]);
    assert!(forbidden.validate().is_err());
    assert!(
        validate_pause(&original, &[Button::R1, Button::R2])
            .unwrap_err()
            .contains("function")
    );
    assert!(validate_pause(&original, &[Button::L1, Button::R1, Button::L2, Button::R2]).is_ok());
}

#[test]
fn layouts_without_the_option_keep_r2_r1_symbol_chords() {
    let mut layout = layout();
    layout.profile.function_layer = false;
    layout.profile.function_volume = false;
    layout.profile.function_print_screen = false;
    layout.profile.navigation_layer = false;
    layout.profile.navigation_media = false;
    layout.bindings = vec![Binding {
        action: Action::Character('1'),
        chord: Chord {
            dpad: Dpad::Neutral,
            buttons: vec![Button::A, Button::R1],
        },
        held: vec![Button::R2],
        held_dpad: Dpad::Neutral,
    }];
    layout.validate().unwrap();
    assert!(layout.function_bindings().is_empty());
    let mut engine = Recognizer::new(
        layout.strategy.clone(),
        layout.profile.clone(),
        Some(layout),
    )
    .unwrap();
    let snapshot = |buttons: Vec<Button>| Snapshot {
        buttons,
        menu_buttons: vec![],
        dpad: Some(Dpad::Neutral),
        l2: 0.0,
        r2: 0.0,
        left_stick: Dpad::Neutral,
        right_stick: Dpad::Neutral,
    };
    engine.update(&snapshot(vec![]));
    assert!(
        engine
            .update(&snapshot(vec![Button::A, Button::R1, Button::R2]))
            .is_empty()
    );
    assert!(matches!(
        engine.update(&snapshot(vec![Button::R2])).as_slice(),
        [Event::Chord {
            action: Some(Action::Character('1')),
            ..
        }]
    ));
}

#[test]
fn generator_includes_all_digits_for_function_layer_with_a_small_corpus() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("corpus.txt"), "aA").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_chordpad"))
        .current_dir(dir.path())
        .args([
            "generate",
            "--independent",
            "--function-layer",
            "--alphabet",
            "corpus",
            "--corpus",
            "corpus.txt",
            "--iterations",
            "10",
            "--restarts",
            "1",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let layout: Layout =
        serde_json::from_slice(&fs::read(dir.path().join("layout.json")).unwrap()).unwrap();
    layout.validate().unwrap();
    assert!(layout.profile.function_layer);
    assert_eq!(layout.function_bindings().len(), 12);
}
