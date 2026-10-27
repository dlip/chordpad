use Button::*;
use chordpad::{
    alpha_banks::{Engine, Layout},
    chord::{Event, Phase},
    controller::Snapshot,
    model::{Action, Button, Dpad, Key, MenuButton, Modifier},
    typing::{Output, Runtime, TypingLayout},
};
use std::{cell::RefCell, rc::Rc};

fn layout() -> Layout {
    serde_json::from_str(include_str!("../experiments/four-bank-alpha/layout.json")).unwrap()
}
fn state(buttons: &[Button], dpad: Dpad) -> Snapshot {
    Snapshot {
        buttons: buttons.to_vec(),
        menu_buttons: vec![],
        dpad: Some(dpad),
        l2: 0.0,
        r2: 0.0,
        left_stick: Dpad::Neutral,
        right_stick: Dpad::Neutral,
    }
}
fn update(engine: &mut Engine, buttons: &[Button], dpad: Dpad, at: u64) -> Vec<Event> {
    engine.update_at(&state(buttons, dpad), at)
}
fn engine() -> Engine {
    let mut engine = Engine::new(layout()).unwrap();
    assert_eq!(update(&mut engine, &[], Dpad::Neutral, 0), [Event::Ready]);
    engine
}
fn outputs(events: Vec<Event>) -> Vec<(Action, Vec<Modifier>)> {
    events
        .into_iter()
        .filter_map(|e| match e {
            Event::Chord {
                action: Some(action),
                keyboard_modifiers,
                ..
            }
            | Event::Menu {
                action,
                keyboard_modifiers,
                ..
            } => Some((action, keyboard_modifiers)),
            _ => None,
        })
        .collect()
}
fn chars(text: &str) -> Vec<(Action, Vec<Modifier>)> {
    text.chars()
        .map(|c| (Action::Character(c), vec![]))
        .collect()
}

#[test]
fn exported_layout_validates_and_rejects_invalid_controls_selectors_and_timing() {
    layout().validate().unwrap();
    let original: serde_json::Value =
        serde_json::from_str(include_str!("../experiments/four-bank-alpha/layout.json")).unwrap();
    for (pointer, replacement) in [
        ("/banks/0/bindings/0/control", serde_json::json!("bad")),
        ("/banks/0/held", serde_json::json!(["L1"])),
        ("/recognition/late_bumper_window_ms", serde_json::json!(0)),
        ("/recognition/trigger_tap_max_ms", serde_json::json!(1001)),
        ("/held_shift_buttons", serde_json::json!(["R1"])),
    ] {
        let mut value = original.clone();
        *value.pointer_mut(pointer).unwrap() = replacement;
        assert!(TypingLayout::parse(value).is_err(), "{pointer}");
    }
    assert!(layout().validate_pause(&[L1, R1, L2, R2]).is_err());
    layout().validate_pause(&[A, B, X, Y]).unwrap();
}

#[test]
fn mnemonic_pins_and_number_banks_emit_the_exported_characters() {
    for (buttons, dpad, expected) in [
        (vec![L1, Y], Dpad::Neutral, 'u'),
        (vec![L1], Dpad::Down, 'd'),
        (vec![L1], Dpad::Left, 'l'),
        (vec![], Dpad::Right, 'r'),
        (vec![B], Dpad::Neutral, 'o'),
        (vec![R2], Dpad::Up, '1'),
        (vec![R2], Dpad::Right, '2'),
        (vec![R2], Dpad::Down, '3'),
        (vec![R2], Dpad::Left, '4'),
        (vec![R2, L1], Dpad::Up, '5'),
        (vec![R2, L1], Dpad::Right, '6'),
        (vec![R2, L1], Dpad::Down, '7'),
        (vec![R2, L1], Dpad::Left, '8'),
    ] {
        let mut e = engine();
        update(&mut e, &buttons, dpad, 10);
        assert_eq!(
            outputs(update(&mut e, &[], Dpad::Neutral, 20)),
            chars(&expected.to_string())
        );
    }
}

#[test]
fn newest_overlap_owns_late_bumper_and_reversed_releases_preserve_press_order() {
    let mut e = engine();
    update(&mut e, &[], Dpad::Up, 10);
    update(&mut e, &[Y], Dpad::Up, 20);
    update(&mut e, &[Y, L1], Dpad::Up, 30);
    assert!(outputs(update(&mut e, &[L1], Dpad::Up, 40)).is_empty());
    assert_eq!(outputs(update(&mut e, &[], Dpad::Neutral, 50)), chars("su"));
}

#[test]
fn adjacent_face_buttons_and_dpad_components_roll_as_separate_letters() {
    let mut e = engine();
    update(&mut e, &[Y], Dpad::Neutral, 10);
    update(&mut e, &[Y, B], Dpad::Neutral, 20);
    assert!(update(&mut e, &[Y], Dpad::Neutral, 30).is_empty());
    assert_eq!(outputs(update(&mut e, &[], Dpad::Neutral, 40)), chars("io"));
    update(&mut e, &[], Dpad::Up, 50);
    update(&mut e, &[], Dpad::UpRight, 60);
    assert!(update(&mut e, &[], Dpad::Up, 70).is_empty());
    assert_eq!(outputs(update(&mut e, &[], Dpad::Neutral, 80)), chars("sr"));
}

#[test]
fn late_bumper_deadline_does_not_create_a_stray_backspace() {
    for (delay, expected) in [(99, "b"), (100, "s"), (101, "s")] {
        let mut e = engine();
        update(&mut e, &[], Dpad::Up, 10);
        update(&mut e, &[L1], Dpad::Up, 10 + delay);
        assert_eq!(
            outputs(update(&mut e, &[], Dpad::Neutral, 120)),
            chars(expected)
        );
    }
}

#[test]
fn held_bumper_release_preserves_bank_and_used_taps_do_not_leak() {
    let mut e = engine();
    update(&mut e, &[L1], Dpad::Neutral, 10);
    update(&mut e, &[L1, Y], Dpad::Neutral, 20);
    update(&mut e, &[Y], Dpad::Neutral, 30);
    assert_eq!(outputs(update(&mut e, &[], Dpad::Neutral, 40)), chars("u"));
    update(&mut e, &[R1], Dpad::Neutral, 50);
    update(&mut e, &[R1, A], Dpad::Neutral, 60);
    assert_eq!(
        outputs(update(&mut e, &[R1], Dpad::Neutral, 70)),
        chars("k")
    );
    assert!(update(&mut e, &[], Dpad::Neutral, 80).is_empty());
    update(&mut e, &[R1], Dpad::Neutral, 90);
    assert_eq!(
        outputs(update(&mut e, &[], Dpad::Neutral, 100)),
        [(Action::Key(Key::Space), vec![])]
    );
    update(&mut e, &[L1], Dpad::Neutral, 110);
    assert_eq!(
        outputs(update(&mut e, &[], Dpad::Neutral, 120)),
        [(Action::Key(Key::Backspace), vec![])]
    );
}

#[test]
fn one_shot_shift_clears_after_one_key_and_held_shift_does_not_latch() {
    let mut e = engine();
    update(&mut e, &[L2], Dpad::Neutral, 10);
    update(&mut e, &[], Dpad::Neutral, 20);
    update(&mut e, &[A], Dpad::Neutral, 30);
    assert_eq!(
        outputs(update(&mut e, &[], Dpad::Neutral, 40)),
        [(Action::Character('a'), vec![Modifier::Shift])]
    );
    update(&mut e, &[A], Dpad::Neutral, 50);
    assert_eq!(outputs(update(&mut e, &[], Dpad::Neutral, 60)), chars("a"));
    let mut modified = state(&[L2, R1, R2], Dpad::Right);
    modified.left_stick = Dpad::Left;
    e.update_at(&modified, 70);
    assert_eq!(
        outputs(update(&mut e, &[], Dpad::Neutral, 80)),
        [(
            Action::Key(Key::Right),
            vec![Modifier::Ctrl, Modifier::Shift]
        )]
    );
    update(&mut e, &[A], Dpad::Neutral, 90);
    assert_eq!(outputs(update(&mut e, &[], Dpad::Neutral, 100)), chars("a"));
}

#[test]
fn shift_selects_navigation_and_secondary_face_controls_are_distinct() {
    for (prefix, buttons, key) in [
        (vec![R1, R2], vec![Y], Key::PageUp),
        (vec![R1, R2, L1], vec![A], Key::MediaPlayPause),
        (vec![R1, R2, L1], vec![X], Key::Delete),
    ] {
        let mut e = engine();
        let mut down = prefix.clone();
        down.extend(buttons);
        update(&mut e, &prefix, Dpad::Neutral, 10);
        update(&mut e, &down, Dpad::Neutral, 20);
        assert_eq!(
            outputs(update(&mut e, &[], Dpad::Neutral, 30)),
            [(Action::Key(key), vec![])]
        );
    }
}

#[test]
fn r2_taps_cycle_numbers_functions_off_without_a_gap_deadline() {
    for (tap_count, expected) in [
        (1, Action::Character('1')),
        (2, Action::Key(Key::F1)),
        (3, Action::Character('s')),
    ] {
        let mut e = engine();
        for tap in 0..tap_count {
            let at = 10 + tap * 1000;
            update(&mut e, &[R2], Dpad::Neutral, at);
            update(&mut e, &[], Dpad::Neutral, at + 20);
        }
        update(&mut e, &[], Dpad::Up, 4000);
        assert_eq!(
            outputs(update(&mut e, &[], Dpad::Neutral, 4010)),
            [(expected, vec![])]
        );
        update(&mut e, &[], Dpad::Up, 4020);
        assert_eq!(
            outputs(update(&mut e, &[], Dpad::Neutral, 4030)),
            chars("s")
        );
    }
    for (duration, expected) in [(250, Action::Key(Key::F1)), (251, Action::Character('1'))] {
        let mut e = engine();
        update(&mut e, &[R2], Dpad::Neutral, 10);
        update(&mut e, &[], Dpad::Neutral, 10 + duration);
        update(&mut e, &[R2], Dpad::Neutral, 1000);
        update(&mut e, &[], Dpad::Neutral, 1020);
        update(&mut e, &[], Dpad::Up, 1030);
        assert_eq!(
            outputs(update(&mut e, &[], Dpad::Neutral, 1040)),
            [(expected, vec![])]
        );
    }
    let mut e = engine();
    for at in [10, 30] {
        update(&mut e, &[R2], Dpad::Up, at);
        assert_eq!(
            outputs(update(&mut e, &[], Dpad::Neutral, at + 10)),
            chars("1")
        );
    }
    update(&mut e, &[A], Dpad::Neutral, 50);
    assert_eq!(outputs(update(&mut e, &[], Dpad::Neutral, 60)), chars("a"));
}

#[test]
fn navigation_prefix_without_a_key_consumes_space_and_cannot_arm_functions() {
    for r1_first in [false, true] {
        let mut e = engine();
        update(&mut e, &[if r1_first { R1 } else { R2 }], Dpad::Neutral, 10);
        update(&mut e, &[R1, R2], Dpad::Neutral, 20);
        assert!(update(&mut e, &[], Dpad::Neutral, 30).is_empty());
        update(&mut e, &[R2], Dpad::Up, 40);
        assert_eq!(outputs(update(&mut e, &[], Dpad::Neutral, 50)), chars("1"));
    }
}

#[test]
fn editing_media_bank_uses_directions_and_clears_modifiers_for_consumer_keys() {
    for (button, dpad, key) in [
        (None, Dpad::Up, Key::VolumeUp),
        (None, Dpad::Down, Key::VolumeDown),
        (None, Dpad::Left, Key::MediaPrevious),
        (None, Dpad::Right, Key::MediaNext),
        (Some(A), Dpad::Neutral, Key::MediaPlayPause),
        (Some(B), Dpad::Neutral, Key::MediaStop),
        (Some(Y), Dpad::Neutral, Key::PrintScreen),
        (Some(X), Dpad::Neutral, Key::Delete),
    ] {
        let mut e = engine();
        let mut buttons = vec![R1, R2, L1, L2];
        update(&mut e, &buttons, Dpad::Neutral, 10);
        buttons.extend(button);
        update(&mut e, &buttons, dpad, 20);
        let modifiers = if key.is_consumer() {
            vec![]
        } else {
            vec![Modifier::Shift]
        };
        assert_eq!(
            outputs(update(&mut e, &[], Dpad::Neutral, 30)),
            [(Action::Key(key), modifiers)]
        );
    }
}

#[test]
fn function_l1_face_buttons_are_unbound_and_do_not_type() {
    for button in [Y, B, A, X] {
        let mut e = engine();
        update(&mut e, &[R2], Dpad::Neutral, 10);
        update(&mut e, &[], Dpad::Neutral, 20);
        update(&mut e, &[R2], Dpad::Neutral, 30);
        update(&mut e, &[], Dpad::Neutral, 40);
        update(&mut e, &[L1, button], Dpad::Neutral, 50);
        let events = update(&mut e, &[], Dpad::Neutral, 60);
        assert!(matches!(events.as_slice(), [Event::Unmapped { .. }]));
        assert!(outputs(events).is_empty());
    }
}

#[test]
fn media_keys_repeat_while_held_without_a_release_duplicate() {
    for (dpad, key) in [
        (Dpad::Up, Key::VolumeUp),
        (Dpad::Down, Key::VolumeDown),
        (Dpad::Left, Key::MediaPrevious),
        (Dpad::Right, Key::MediaNext),
    ] {
        let mut e = engine();
        update(&mut e, &[R1, R2, L1], dpad, 10);
        assert_eq!(outputs(e.repeat_at(310)), [(Action::Key(key), vec![])]);
        assert!(update(&mut e, &[], Dpad::Neutral, 320).is_empty());
    }
}

#[test]
fn late_r1_preserves_number_stroke_and_navigation_overrides_and_consumes_one_shot_functions() {
    let mut e = engine();
    update(&mut e, &[R2], Dpad::Up, 10);
    update(&mut e, &[R2, R1], Dpad::Up, 20);
    assert_eq!(outputs(update(&mut e, &[], Dpad::Neutral, 30)), chars("1"));
    for at in [40, 60] {
        update(&mut e, &[R2], Dpad::Neutral, at);
        update(&mut e, &[], Dpad::Neutral, at + 10);
    }
    update(&mut e, &[R2, R1], Dpad::Up, 80);
    assert_eq!(
        outputs(update(&mut e, &[], Dpad::Neutral, 90)),
        [(Action::Key(Key::Up), vec![])]
    );
    update(&mut e, &[], Dpad::Up, 100);
    assert_eq!(outputs(update(&mut e, &[], Dpad::Neutral, 110)), chars("s"));
    update(&mut e, &[R1], Dpad::Neutral, 120);
    assert_eq!(
        outputs(update(&mut e, &[], Dpad::Neutral, 130)),
        [(Action::Key(Key::Space), vec![])]
    );
}

#[test]
fn standalone_bumpers_repeat_after_300_ms_then_80_ms_and_stop_on_release() {
    for (button, key) in [(L1, Key::Backspace), (R1, Key::Space)] {
        let mut e = engine();
        update(&mut e, &[button], Dpad::Neutral, 10);
        let expected = vec![(Action::Key(key), vec![])];
        assert!(e.repeat_at(309).is_empty());
        assert_eq!(outputs(e.repeat_at(310)), expected);
        assert!(e.repeat_at(389).is_empty());
        assert_eq!(outputs(e.repeat_at(390)), expected);
        assert!(update(&mut e, &[], Dpad::Neutral, 400).is_empty());
        assert!(e.repeat_at(1000).is_empty());
        assert_eq!(e.phase(), Phase::Idle);
    }
}

#[test]
fn letters_repeat_after_300_ms_then_80_ms_and_do_not_block_overlapping_output() {
    let mut e = engine();
    update(&mut e, &[A], Dpad::Neutral, 10);
    update(&mut e, &[A, B], Dpad::Neutral, 20);
    assert!(update(&mut e, &[A], Dpad::Neutral, 30).is_empty());
    assert!(e.repeat_at(309).is_empty());
    assert_eq!(outputs(e.repeat_at(310)), chars("ao"));
    assert!(e.repeat_at(389).is_empty());
    assert_eq!(outputs(e.repeat_at(390)), chars("a"));
    assert!(update(&mut e, &[], Dpad::Neutral, 400).is_empty());
    assert!(e.repeat_at(1000).is_empty());
}

#[test]
fn navigation_repeats_after_300_ms_then_80_ms_without_a_release_duplicate() {
    let mut e = engine();
    update(&mut e, &[R1, R2, L2], Dpad::Right, 10);
    assert!(e.repeat_at(309).is_empty());
    let expected = vec![(Action::Key(Key::Right), vec![Modifier::Shift])];
    assert_eq!(outputs(e.repeat_at(310)), expected);
    assert!(e.repeat_at(389).is_empty());
    assert_eq!(outputs(e.repeat_at(390)), expected);
    assert!(update(&mut e, &[], Dpad::Neutral, 400).is_empty());
    assert!(e.repeat_at(500).is_empty());
}

#[test]
fn menu_keys_capture_shift_and_reset_discards_every_pending_mode() {
    let mut e = engine();
    let mut s = state(&[L2], Dpad::Neutral);
    s.menu_buttons.push(MenuButton::Start);
    e.update_at(&s, 10);
    assert_eq!(
        outputs(update(&mut e, &[], Dpad::Neutral, 20)),
        [(Action::Key(Key::Enter), vec![Modifier::Shift])]
    );
    update(&mut e, &[R2], Dpad::Neutral, 30);
    update(&mut e, &[], Dpad::Neutral, 40);
    e.reset("resync");
    assert_eq!(e.phase(), Phase::Waiting);
    assert!(update(&mut e, &[A], Dpad::Neutral, 50).is_empty());
    assert_eq!(update(&mut e, &[], Dpad::Neutral, 60), [Event::Ready]);
    update(&mut e, &[R2], Dpad::Up, 70);
    assert_eq!(outputs(update(&mut e, &[], Dpad::Neutral, 80)), chars("1"));
    update(&mut e, &[L2, A], Dpad::Neutral, 90);
    e.reset("paused");
    update(&mut e, &[], Dpad::Neutral, 100);
    update(&mut e, &[A], Dpad::Neutral, 110);
    assert_eq!(outputs(update(&mut e, &[], Dpad::Neutral, 120)), chars("a"));
}

#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Captured>>);
type Captured = Vec<(Action, Vec<Modifier>)>;
impl Output for Capture {
    fn send(&mut self, action: &Action) -> chordpad::Result<()> {
        self.send_modified(action, &[])
    }
    fn send_modified(&mut self, action: &Action, modifiers: &[Modifier]) -> chordpad::Result<()> {
        self.0
            .borrow_mut()
            .push((action.clone(), modifiers.to_vec()));
        Ok(())
    }
    fn release_all(&mut self) -> chordpad::Result<()> {
        Ok(())
    }
}

#[test]
fn linux_runtime_accepts_candidate_and_pause_gesture_discards_partial_typing() {
    let capture = Capture::default();
    let log = capture.0.clone();
    let source = TypingLayout::parse(
        serde_json::from_str(include_str!("../experiments/four-bank-alpha/layout.json")).unwrap(),
    )
    .unwrap();
    let mut runtime = Runtime::with_layout(source, &[A, B, X, Y], true, capture).unwrap();
    runtime.update(&state(&[], Dpad::Neutral)).unwrap();
    runtime
        .update(&state(&[A, B, X, Y], Dpad::Neutral))
        .unwrap();
    assert!(!runtime.paused());
    runtime.update(&state(&[], Dpad::Neutral)).unwrap();
    runtime.update(&state(&[A], Dpad::Neutral)).unwrap();
    runtime.update(&state(&[], Dpad::Neutral)).unwrap();
    assert_eq!(*log.borrow(), chars("a"));
    runtime.update(&state(&[A], Dpad::Neutral)).unwrap();
    runtime
        .update(&state(&[A, B, X, Y], Dpad::Neutral))
        .unwrap();
    assert!(runtime.paused());
    runtime.update(&state(&[], Dpad::Neutral)).unwrap();
    assert_eq!(*log.borrow(), chars("a"));
}

#[test]
fn stick_clicks_repeat_fixed_command_shortcuts_and_preserve_one_shot_shift() {
    use chordpad::model::Shortcut;
    let mut e = engine();
    update(&mut e, &[L2], Dpad::Neutral, 10);
    update(&mut e, &[], Dpad::Neutral, 20);
    let mut s = state(&[R1, R2, L1], Dpad::Neutral);
    s.menu_buttons = vec![MenuButton::L3, MenuButton::R3];
    s.left_stick = Dpad::Left;
    e.update_at(&s, 30);
    let repeated = outputs(e.repeat_at(1000));
    s.menu_buttons = vec![MenuButton::L3];
    s.left_stick = Dpad::UpRight;
    assert!(e.update_at(&s, 1010).is_empty());
    assert_eq!(
        repeated,
        vec![
            (
                Action::Shortcut(Shortcut {
                    modifiers: vec![Modifier::Meta],
                    key: Key::D
                }),
                vec![]
            ),
            (
                Action::Shortcut(Shortcut {
                    modifiers: vec![Modifier::Shift, Modifier::Meta],
                    key: Key::D
                }),
                vec![]
            ),
        ]
    );
    assert!(update(&mut e, &[], Dpad::Neutral, 1020).is_empty());
    update(&mut e, &[A], Dpad::Neutral, 1030);
    assert_eq!(
        outputs(update(&mut e, &[], Dpad::Neutral, 1040)),
        [(Action::Character('a'), vec![Modifier::Shift])]
    );
    let mut s = state(&[], Dpad::Neutral);
    s.menu_buttons = vec![MenuButton::L3];
    e.update_at(&s, 1050);
    e.reset("paused");
    assert!(e.update_at(&s, 1060).is_empty());
    assert_eq!(update(&mut e, &[], Dpad::Neutral, 1070), [Event::Ready]);
}

#[test]
fn normalizer_detects_stick_clicks_and_requires_release_before_rearming() {
    use chordpad::controller::{Calibration, Normalizer, RawEvent};
    let mut n = Normalizer::new(Calibration::default()).unwrap();
    let raw = |code, value| RawEvent {
        event_type: 1,
        code,
        value,
    };
    let s = n.apply(&[raw(317, 1), raw(318, 1)]).unwrap();
    assert_eq!(s.menu_buttons, [MenuButton::L3, MenuButton::R3]);
    assert!(!s.is_neutral());
    let mut e = Engine::new(layout()).unwrap();
    assert!(e.update_at(&s, 0).is_empty());
    let s = n.apply(&[raw(317, 0), raw(318, 0)]).unwrap();
    assert_eq!(e.update_at(&s, 10), [Event::Ready]);
    let s = n.apply(&[raw(317, 1)]).unwrap();
    e.update_at(&s, 20);
    let s = n.apply(&[raw(317, 2)]).unwrap();
    assert!(e.update_at(&s, 30).is_empty());
    let s = n.apply(&[raw(317, 0)]).unwrap();
    assert_eq!(outputs(e.update_at(&s, 40)).len(), 1);
}

#[test]
fn production_recognizer_keeps_stick_shortcuts_fixed_across_modifier_changes() {
    use chordpad::{chord::Recognizer, model::Layout as StandardLayout};
    let layout: StandardLayout =
        serde_json::from_str(include_str!("../examples/legacy-layout.json")).unwrap();
    let mut e = Recognizer::new(
        layout.strategy.clone(),
        layout.profile.clone(),
        Some(layout),
    )
    .unwrap();
    e.update(&state(&[], Dpad::Neutral));
    let mut s = state(&[L2], Dpad::Neutral);
    s.menu_buttons = vec![MenuButton::L3, MenuButton::R3];
    e.update(&s);
    s.left_stick = Dpad::UpRight;
    assert!(e.update(&s).is_empty());
    s.menu_buttons.clear();
    let result = outputs(e.update(&s));
    assert_eq!(
        result,
        vec![
            (MenuButton::L3.action(), vec![]),
            (MenuButton::R3.action(), vec![])
        ]
    );
}
