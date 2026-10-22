use padtype::{
    Result,
    controller::Snapshot,
    model::*,
    typing::{Output, Runtime, validate_pause},
};
use std::{cell::RefCell, rc::Rc};

#[cfg(target_os = "linux")]
#[path = "support/uinput.rs"]
mod hardware;

fn layout() -> Layout {
    let binding = |action, buttons, held| Binding {
        action,
        chord: Chord {
            buttons,
            dpad: Dpad::Neutral,
        },
        held,
    };
    Layout {
        schema_version: SCHEMA_VERSION,
        cost_model_version: COST_MODEL_VERSION,
        strategy: Strategy {
            name: "typing-test".into(),
            shift: Some(Button::R2),
            layer: Some(Button::L2),
        },
        profile: Profile::default(),
        weights: Weights::default(),
        normalization: Metrics {
            effort: 1.0,
            activations: 1.0,
            movement: 1.0,
            coverage: 1.0,
            learning: 1.0,
        },
        recognition: "release-all".into(),
        bindings: vec![
            binding(Action::Character('t'), vec![Button::A], vec![]),
            binding(Action::Character('T'), vec![Button::A], vec![Button::R2]),
            binding(Action::Character('.'), vec![Button::A], vec![Button::L2]),
            binding(Action::Key(Key::Backspace), vec![Button::B], vec![]),
            binding(
                Action::Shortcut(Shortcut {
                    modifiers: vec![Modifier::Ctrl],
                    key: Key::C,
                }),
                vec![Button::L1],
                vec![],
            ),
        ],
    }
}

fn state(buttons: &[Button]) -> Snapshot {
    Snapshot {
        buttons: buttons.into(),
        dpad: Some(Dpad::Neutral),
        l2: 0.0,
        r2: 0.0,
    }
}

#[derive(Default)]
struct Sent {
    actions: Vec<Action>,
    releases: usize,
}

struct Sink(Rc<RefCell<Sent>>);

impl Output for Sink {
    fn send(&mut self, action: &Action) -> Result<()> {
        self.0.borrow_mut().actions.push(action.clone());
        Ok(())
    }
    fn release_all(&mut self) -> Result<()> {
        self.0.borrow_mut().releases += 1;
        Ok(())
    }
}

const PAUSE: [Button; 4] = [Button::L1, Button::R1, Button::L2, Button::R2];

#[test]
fn characters_modifiers_corrections_and_shortcuts_emit_once_after_release() {
    use Button::*;
    let sent = Rc::new(RefCell::new(Sent::default()));
    let mut runtime = Runtime::new(layout(), &PAUSE, false, Sink(sent.clone())).unwrap();
    runtime.update(&state(&[])).unwrap();
    for (held, button) in [
        (vec![], A),
        (vec![R2], A),
        (vec![L2], A),
        (vec![], B),
        (vec![], L1),
    ] {
        runtime.update(&state(&held)).unwrap();
        let mut down = held.clone();
        down.push(button);
        runtime.update(&state(&down)).unwrap();
        runtime.update(&state(&down)).unwrap();
        let before = sent.borrow().actions.len();
        runtime.update(&state(&held)).unwrap();
        runtime.update(&state(&held)).unwrap();
        assert_eq!(sent.borrow().actions.len(), before + 1);
        runtime.update(&state(&[])).unwrap();
    }
    assert_eq!(
        sent.borrow().actions,
        layout()
            .bindings
            .iter()
            .map(|b| b.action.clone())
            .collect::<Vec<_>>()
    );
}

#[test]
fn pause_gesture_consumes_prefix_and_release_and_toggles_once() {
    use Button::*;
    let sent = Rc::new(RefCell::new(Sent::default()));
    let mut runtime = Runtime::new(layout(), &PAUSE, true, Sink(sent.clone())).unwrap();
    // Starting with a held gesture must not silently activate typing.
    runtime.update(&state(&PAUSE)).unwrap();
    assert!(runtime.paused());
    runtime.update(&state(&[])).unwrap();
    for paused in [false, true, false] {
        runtime.update(&state(&[L1, R1])).unwrap();
        runtime.update(&state(&PAUSE)).unwrap();
        assert_eq!(runtime.paused(), paused);
        runtime.update(&state(&PAUSE)).unwrap();
        runtime.update(&state(&[L1, R1, L2, R2, A])).unwrap();
        runtime.update(&state(&[L1])).unwrap();
        runtime.update(&state(&[])).unwrap();
        assert_eq!(runtime.paused(), paused);
        assert!(sent.borrow().actions.is_empty());
    }
    runtime.update(&state(&[A])).unwrap();
    runtime.update(&state(&[])).unwrap();
    assert_eq!(sent.borrow().actions, [Action::Character('t')]);
}

#[test]
fn pause_reset_and_disconnect_discard_pending_strokes_and_require_neutral() {
    let sent = Rc::new(RefCell::new(Sent::default()));
    let mut runtime = Runtime::new(layout(), &PAUSE, false, Sink(sent.clone())).unwrap();
    runtime.update(&state(&[])).unwrap();
    for reason in ["input events dropped", "disconnected"] {
        runtime.update(&state(&[Button::A])).unwrap();
        runtime.reset(reason).unwrap();
        runtime.update(&state(&[Button::A])).unwrap();
        runtime.update(&state(&[])).unwrap();
        assert!(sent.borrow().actions.is_empty());
    }
    runtime.update(&state(&[Button::A])).unwrap();
    runtime.set_paused(true).unwrap();
    runtime.update(&state(&[])).unwrap();
    runtime.set_paused(false).unwrap();
    runtime.update(&state(&[Button::R2])).unwrap();
    runtime.update(&state(&[Button::R2, Button::A])).unwrap();
    runtime.update(&state(&[])).unwrap();
    assert!(sent.borrow().actions.is_empty());
    runtime.update(&state(&[Button::A])).unwrap();
    runtime.update(&state(&[])).unwrap();
    assert_eq!(sent.borrow().actions, [Action::Character('t')]);
    let before = sent.borrow().releases;
    drop(runtime);
    assert!(sent.borrow().releases > before);
}

#[test]
fn reserved_gestures_check_held_modifiers_supersets_and_disabled_controls() {
    let mut l = layout();
    assert!(validate_pause(&l, &[Button::A]).is_err());
    assert!(validate_pause(&l, &[Button::A, Button::R2]).is_err());
    assert!(validate_pause(&l, &[Button::L1, Button::L1]).is_err());
    assert!(validate_pause(&l, &[]).is_err());
    assert!(validate_pause(&l, &PAUSE).is_ok());
    l.profile.disabled_buttons.push(Button::R1);
    assert!(validate_pause(&l, &PAUSE).is_err());
}

#[test]
fn shortcuts_reject_empty_duplicate_and_noncanonical_modifiers() {
    for modifiers in [
        vec![],
        vec![Modifier::Ctrl, Modifier::Ctrl],
        vec![Modifier::Shift, Modifier::Ctrl],
    ] {
        let mut l = layout();
        l.bindings[4].action = Action::Shortcut(Shortcut {
            modifiers,
            key: Key::C,
        });
        assert!(l.validate().is_err());
    }
    let mut l = layout();
    l.bindings[4].action = Action::Shortcut(Shortcut {
        modifiers: vec![Modifier::Ctrl, Modifier::Shift],
        key: Key::C,
    });
    l.validate().unwrap();
    let json = serde_json::to_string(&l).unwrap();
    serde_json::from_str::<Layout>(&json)
        .unwrap()
        .validate()
        .unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn typing_preflight_rejects_unsupported_outputs_and_gesture_conflicts_before_device_access() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("layout.json");
    let run = |extra: &[&str]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_padtype"))
            .args(["type", "--device", "/does-not-exist", "--layout"])
            .arg(&path)
            .arg("--dry-run")
            .args(extra)
            .output()
            .unwrap()
    };
    std::fs::write(&path, serde_json::to_string(&layout()).unwrap()).unwrap();
    let conflict = run(&["--pause-buttons", "A"]);
    assert!(!conflict.status.success());
    assert!(String::from_utf8_lossy(&conflict.stderr).contains("pause gesture conflicts"));
    let mut l = layout();
    l.bindings[0].action = Action::Character('é');
    l.bindings.remove(1);
    std::fs::write(&path, serde_json::to_string(&l).unwrap()).unwrap();
    let unsupported = run(&[]);
    assert!(!unsupported.status.success());
    assert!(String::from_utf8_lossy(&unsupported.stderr).contains("does not support character"));
    assert!(
        String::from_utf8_lossy(&run(&["--duration", "0"]).stderr)
            .contains("duration must be positive")
    );
}

#[test]
fn generator_accepts_shortcut_pins_and_preserves_the_reserved_gesture() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("layout.json");
    let config = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/typing.json");
    let generated = std::process::Command::new(env!("CARGO_BIN_EXE_padtype"))
        .args(["generate", "--config"])
        .arg(config)
        .args([
            "--strategy",
            "r2-shift-l2-layer",
            "--iterations",
            "5",
            "--restarts",
            "1",
            "--output",
        ])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let l: Layout = serde_json::from_str(&std::fs::read_to_string(output).unwrap()).unwrap();
    l.validate().unwrap();
    validate_pause(&l, &PAUSE).unwrap();
    assert_eq!(
        l.bindings
            .iter()
            .filter(|b| matches!(b.action, Action::Shortcut(_)))
            .count(),
        2
    );
    assert_eq!(l.profile.forbidden_combinations, [PAUSE.to_vec()]);
}

#[cfg(target_os = "linux")]
mod keyboard {
    use super::*;
    use evdev::{InputEvent, KeyCode};
    use padtype::output::{KeySink, Keyboard, stroke};
    use std::{collections::BTreeSet, io};

    #[derive(Default)]
    struct Frames {
        values: Vec<Vec<(u16, i32)>>,
        fail_on: BTreeSet<usize>,
        calls: usize,
    }
    struct Device(Rc<RefCell<Frames>>);
    impl KeySink for Device {
        fn emit(&mut self, events: &[InputEvent]) -> io::Result<()> {
            let mut f = self.0.borrow_mut();
            f.calls += 1;
            f.values
                .push(events.iter().map(|e| (e.code(), e.value())).collect());
            if f.fail_on.contains(&f.calls) {
                Err(io::Error::other("injected partial write"))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn ascii_output_matches_us_keyboard_rows_and_shift_pairs() {
        let rows = [
            (
                "`1234567890-=",
                "~!@#$%^&*()_+",
                vec![41, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
            ),
            (
                "qwertyuiop[]\\",
                "QWERTYUIOP{}|",
                (16..=27).chain([43]).collect(),
            ),
            ("asdfghjkl;'", "ASDFGHJKL:\"", (30..=40).collect()),
            ("zxcvbnm,./", "ZXCVBNM<>?", (44..=53).collect()),
        ];
        let mut covered = BTreeSet::new();
        for (base, shifted, codes) in rows {
            for ((a, b), code) in base.chars().zip(shifted.chars()).zip(codes) {
                assert_eq!(stroke(&Action::Character(a)).unwrap(), [KeyCode(code)]);
                assert_eq!(
                    stroke(&Action::Character(b)).unwrap(),
                    [KeyCode::KEY_LEFTSHIFT, KeyCode(code)]
                );
                covered.extend([a, b]);
            }
        }
        for (c, code) in [(' ', 57), ('\n', 28), ('\t', 15)] {
            assert_eq!(stroke(&Action::Character(c)).unwrap(), [KeyCode(code)]);
            covered.insert(c);
        }
        assert!(
            (' '..='~')
                .chain(['\n', '\t'])
                .all(|c| covered.contains(&c))
        );
        assert!(stroke(&Action::Character('é')).is_err());
        assert!(stroke(&Action::Character('\0')).is_err());
    }

    #[test]
    fn actions_balance_keys_and_release_modifiers_in_reverse_order() {
        let frames = Rc::new(RefCell::new(Frames::default()));
        let mut keyboard = Keyboard::new(Device(frames.clone()));
        keyboard.send(&Action::Character('A')).unwrap();
        keyboard
            .send(&Action::Shortcut(Shortcut {
                modifiers: vec![Modifier::Ctrl, Modifier::Shift],
                key: Key::C,
            }))
            .unwrap();
        keyboard.send(&Action::Key(Key::Backspace)).unwrap();
        assert_eq!(
            frames.borrow().values,
            [
                vec![(42, 1), (30, 1)],
                vec![(30, 0), (42, 0)],
                vec![(29, 1), (42, 1), (46, 1)],
                vec![(46, 0), (42, 0), (29, 0)],
                vec![(14, 1)],
                vec![(14, 0)],
            ]
        );
        let before = frames.borrow().calls;
        drop(keyboard);
        assert_eq!(frames.borrow().calls, before);
    }

    #[test]
    fn partial_write_failures_attempt_cleanup_and_retry_on_drop() {
        for failures in [vec![1], vec![2], vec![1, 2]] {
            let frames = Rc::new(RefCell::new(Frames {
                fail_on: failures.into_iter().collect(),
                ..Frames::default()
            }));
            let mut keyboard = Keyboard::new(Device(frames.clone()));
            assert!(keyboard.send(&Action::Character('A')).is_err());
            drop(keyboard);
            let f = frames.borrow();
            assert_eq!(f.values.last().unwrap(), &vec![(30, 0), (42, 0)]);
            assert!(!f.fail_on.contains(&f.calls));
        }
    }
}
