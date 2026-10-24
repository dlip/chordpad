use chordpad::{
    chord::{Event, Phase, Recognizer},
    controller::Snapshot,
    model::*,
};

fn input(dpad: Dpad, buttons: &[Button]) -> Input {
    Input {
        dpad,
        mask: button_mask(buttons).unwrap(),
    }
}
fn binding(c: char, ordinary: Input, held: &[Button]) -> Binding {
    Binding {
        action: Action::Character(c),
        chord: ordinary.chord(),
        held: held.into(),
        held_dpad: Dpad::Neutral,
    }
}
fn layout() -> Layout {
    use Button::*;
    use Dpad::*;
    let mut bindings = vec![];
    for (lower, upper, ordinary) in [
        ('a', 'A', input(Up, &[])),
        ('b', 'B', input(Neutral, &[A])),
        ('c', 'C', input(UpLeft, &[L1])),
        ('d', 'D', input(Neutral, &[B, R1])),
    ] {
        bindings.push(binding(lower, ordinary, &[]));
        bindings.push(binding(upper, ordinary, &[L2]));
    }
    for (plain, shifted, ordinary) in [('1', '!', input(Up, &[])), ('2', '@', input(Neutral, &[A]))]
    {
        bindings.push(binding(plain, ordinary, &[R2]));
        bindings.push(binding(shifted, ordinary, &[R2, L2]));
    }
    bindings.push(binding(' ', input(Neutral, &[R1]), &[]));
    bindings.push(Binding {
        action: Action::Key(Key::Backspace),
        chord: input(Neutral, &[L1]).chord(),
        held: vec![],
        held_dpad: Neutral,
    });
    Layout {
        schema_version: SCHEMA_VERSION,
        cost_model_version: COST_MODEL_VERSION,
        strategy: Strategy {
            independent: true,
            name: "independent".into(),
            shift: Some(L2),
            layer: Some(R2),
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
        recognition: "independent-release".into(),
        bindings,
    }
}
fn engine() -> Recognizer {
    let layout = layout();
    Recognizer::new(
        layout.strategy.clone(),
        layout.profile.clone(),
        Some(layout),
    )
    .unwrap()
}
fn snapshot(state: Input) -> Snapshot {
    Snapshot {
        buttons: state.chord().buttons,
        menu_buttons: vec![],
        dpad: Some(state.dpad),
        l2: 0.0,
        r2: 0.0,
        left_stick: Dpad::Neutral,
        right_stick: Dpad::Neutral,
    }
}

fn menu_snapshot(dpad: Dpad, buttons: &[Button], menu: &[MenuButton]) -> Snapshot {
    Snapshot {
        menu_buttons: menu.into(),
        ..snapshot(input(dpad, buttons))
    }
}

#[test]
fn menu_keys_share_stroke_start_order_with_both_typing_halves() {
    use Dpad::*;
    use MenuButton::*;
    let mut e = engine();
    arm(&mut e);
    // Enter released before an earlier letter still follows that letter.
    e.update(&menu_snapshot(Up, &[], &[]));
    e.update(&menu_snapshot(Up, &[], &[Start]));
    assert!(e.update(&menu_snapshot(Up, &[], &[])).is_empty());
    assert_eq!(
        actions(&e.update(&menu_snapshot(Neutral, &[], &[]))),
        [Action::Character('a'), Action::Key(Key::Enter),]
    );
    // Both menu buttons and a later right-hand stroke release independently.
    e.update(&menu_snapshot(Neutral, &[], &[Select]));
    e.update(&menu_snapshot(Neutral, &[], &[Start, Select]));
    e.update(&menu_snapshot(Neutral, &[Button::A], &[Start, Select]));
    assert!(e.update(&menu_snapshot(Neutral, &[], &[Select])).is_empty());
    assert_eq!(
        actions(&e.update(&menu_snapshot(Neutral, &[], &[]))),
        [
            Action::Key(Key::Escape),
            Action::Key(Key::Enter),
            Action::Character('b'),
        ]
    );
    // Within one report: left, right, Start, Select.
    e.update(&menu_snapshot(Up, &[Button::A], &[Start, Select]));
    assert_eq!(
        actions(&e.update(&menu_snapshot(Neutral, &[], &[]))),
        [
            Action::Character('a'),
            Action::Character('b'),
            Action::Key(Key::Enter),
            Action::Key(Key::Escape),
        ]
    );
}

#[test]
fn menu_keys_keep_their_actions_on_every_layer_and_latch_keyboard_modifiers() {
    use Dpad::*;
    use MenuButton::*;
    use Modifier::*;
    for held in [
        vec![],
        vec![Button::L2],
        vec![Button::R2],
        vec![Button::L2, Button::R2],
    ] {
        let mut e = engine();
        arm(&mut e);
        let mut state = menu_snapshot(Neutral, &held, &[]);
        state.left_stick = Right;
        e.update(&state);
        for button in [Start, Select] {
            state.menu_buttons = vec![button];
            assert!(e.update(&state).is_empty());
            // Releasing held modifiers early preserves the initial context.
            let release = menu_snapshot(Neutral, &[], &[]);
            let events = e.update(&release);
            let expected = if held.contains(&Button::L2) {
                vec![Ctrl, Alt, Shift, Meta]
            } else {
                vec![Ctrl, Alt, Meta]
            };
            assert!(
                matches!(events.as_slice(), [Event::Menu { action, keyboard_modifiers, .. }]
                if *action == button.action() && *keyboard_modifiers == expected),
                "{events:?}"
            );
            state.menu_buttons.clear();
            e.update(&state);
        }
    }
}

#[test]
fn menu_keys_cancel_on_late_modifiers_and_reset_requires_full_neutral() {
    use Dpad::*;
    use MenuButton::*;
    let mut e = engine();
    let held = menu_snapshot(Neutral, &[], &[Start]);
    assert!(e.update(&held).is_empty());
    assert_eq!(e.phase(), Phase::Waiting);
    arm(&mut e);
    e.update(&held);
    assert_eq!(e.phase(), Phase::Collecting);
    assert!(matches!(
        e.reset("pause"),
        Event::Reset {
            discarded: true,
            ..
        }
    ));
    assert!(e.update(&held).is_empty());
    arm(&mut e);
    for late in [
        menu_snapshot(Neutral, &[Button::L2], &[Start]),
        Snapshot {
            right_stick: Right,
            ..held.clone()
        },
    ] {
        e.update(&held);
        e.update(&late);
        let events = e.update(&menu_snapshot(Neutral, &[], &[]));
        assert!(matches!(events.as_slice(), [Event::Cancelled { .. }]));
    }
    // Completed menu output queued behind a pending letter is also discarded.
    e.update(&menu_snapshot(Up, &[], &[]));
    e.update(&menu_snapshot(Up, &[], &[Select]));
    assert!(e.update(&menu_snapshot(Up, &[], &[])).is_empty());
    e.reset("disconnect");
    arm(&mut e);
    assert!(e.update(&menu_snapshot(Neutral, &[], &[])).is_empty());
}
fn update(e: &mut Recognizer, dpad: Dpad, buttons: &[Button]) -> Vec<Event> {
    e.update(&snapshot(input(dpad, buttons)))
}
fn actions(events: &[Event]) -> Vec<Action> {
    events
        .iter()
        .filter_map(|e| {
            if let Event::Chord {
                action: Some(action),
                ..
            } = e
            {
                Some(action.clone())
            } else if let Event::Menu { action, .. } = e {
                Some(action.clone())
            } else {
                None
            }
        })
        .collect()
}
fn chars(events: &[Event]) -> String {
    actions(events)
        .into_iter()
        .filter_map(|a| {
            if let Action::Character(c) = a {
                Some(c)
            } else {
                None
            }
        })
        .collect()
}
fn arm(e: &mut Recognizer) {
    assert_eq!(update(e, Dpad::Neutral, &[]), [Event::Ready]);
}

#[test]
fn overlapping_halves_type_in_start_order_even_when_release_order_is_reversed() {
    use Button::*;
    use Dpad::*;
    let mut e = engine();
    arm(&mut e);
    update(&mut e, Up, &[]);
    update(&mut e, Up, &[A]);
    assert!(update(&mut e, Up, &[]).is_empty());
    assert_eq!(chars(&update(&mut e, Neutral, &[])), "ab");
    update(&mut e, Neutral, &[A]);
    update(&mut e, Up, &[A]);
    assert!(update(&mut e, Neutral, &[A]).is_empty());
    assert_eq!(chars(&update(&mut e, Neutral, &[])), "ba");
}

#[test]
fn partial_release_and_diagonals_on_one_half_do_not_cancel_the_other() {
    use Button::*;
    use Dpad::*;
    let mut e = engine();
    arm(&mut e);
    update(&mut e, UpLeft, &[L1]);
    update(&mut e, Up, &[B, R1]);
    assert!(update(&mut e, Up, &[]).is_empty());
    assert_eq!(chars(&update(&mut e, Neutral, &[])), "cd");
}

#[test]
fn shift_uses_the_same_letter_chords_on_both_halves_and_stays_held() {
    use Button::*;
    use Dpad::*;
    let mut e = engine();
    arm(&mut e);
    update(&mut e, Neutral, &[L2]);
    update(&mut e, Up, &[L2]);
    assert_eq!(chars(&update(&mut e, Neutral, &[L2])), "A");
    update(&mut e, Neutral, &[L2, A]);
    assert_eq!(chars(&update(&mut e, Neutral, &[L2])), "B");
    update(&mut e, Up, &[L2, A]);
    assert_eq!(chars(&update(&mut e, Neutral, &[L2])), "AB");
    assert!(update(&mut e, Neutral, &[]).is_empty());
    update(&mut e, Up, &[]);
    assert_eq!(chars(&update(&mut e, Neutral, &[])), "a");
}

#[test]
fn number_layer_stays_held_across_both_halves_space_and_backspace() {
    use Button::*;
    use Dpad::*;
    let mut e = engine();
    arm(&mut e);
    update(&mut e, Neutral, &[R2]);
    let mut typed = vec![];
    for (dpad, button) in [
        (Up, None),
        (Neutral, Some(A)),
        (Neutral, Some(R1)),
        (Neutral, Some(L1)),
        (Up, None),
    ] {
        let mut down = vec![R2];
        down.extend(button);
        update(&mut e, dpad, &down);
        typed.extend(actions(&update(&mut e, Neutral, &[R2])));
    }
    assert_eq!(
        typed,
        [
            Action::Character('1'),
            Action::Character('2'),
            Action::Character(' '),
            Action::Key(Key::Backspace),
            Action::Character('1')
        ]
    );
    assert!(update(&mut e, Neutral, &[]).is_empty());
    update(&mut e, Neutral, &[A]);
    assert_eq!(chars(&update(&mut e, Neutral, &[])), "b");
}

#[test]
fn combined_shift_and_number_layer_type_shifted_symbols_sequentially() {
    use Button::*;
    use Dpad::*;
    let mut e = engine();
    arm(&mut e);
    update(&mut e, Neutral, &[R2, L2]);
    update(&mut e, Up, &[R2, L2]);
    assert_eq!(chars(&update(&mut e, Neutral, &[R2, L2])), "!");
    update(&mut e, Neutral, &[R2, L2, A]);
    assert_eq!(chars(&update(&mut e, Neutral, &[R2, L2])), "@");
    for (button, expected) in [
        (R1, Action::Character(' ')),
        (L1, Action::Key(Key::Backspace)),
    ] {
        update(&mut e, Neutral, &[R2, L2, button]);
        assert_eq!(actions(&update(&mut e, Neutral, &[R2, L2])), [expected]);
    }
}

#[test]
fn modifier_release_preserves_the_context_latched_at_stroke_start() {
    use Button::*;
    use Dpad::*;
    for (held, expected) in [(&[L2][..], "A"), (&[R2][..], "1"), (&[R2, L2][..], "!")] {
        let mut e = engine();
        arm(&mut e);
        update(&mut e, Up, held);
        update(&mut e, Up, &[]);
        assert_eq!(chars(&update(&mut e, Neutral, &[])), expected);
    }
}

#[test]
fn late_modifiers_cancel_active_strokes_but_idle_halves_can_start() {
    use Button::*;
    use Dpad::*;
    let mut e = engine();
    arm(&mut e);
    update(&mut e, Up, &[]);
    let cancelled = update(&mut e, Up, &[L2, A]);
    assert!(
        cancelled
            .iter()
            .any(|e| matches!(e, Event::Cancelled { .. }))
    );
    assert_eq!(chars(&update(&mut e, Neutral, &[L2])), "B");
    update(&mut e, Up, &[L2]);
    assert_eq!(chars(&update(&mut e, Neutral, &[L2])), "A");
}

#[test]
fn cancellation_on_final_release_rearms_without_swallowing_the_next_stroke() {
    use Button::*;
    use Dpad::*;
    let mut e = engine();
    arm(&mut e);
    update(&mut e, Up, &[]);
    assert!(
        update(&mut e, Neutral, &[L2])
            .iter()
            .any(|e| matches!(e, Event::Cancelled { .. }))
    );
    assert_eq!(e.phase(), Phase::Idle);
    update(&mut e, Up, &[L2]);
    assert_eq!(chars(&update(&mut e, Neutral, &[L2])), "A");
}

#[test]
fn cancelled_earlier_strokes_unblock_later_completed_output() {
    use Button::*;
    use Dpad::*;
    let mut e = engine();
    arm(&mut e);
    update(&mut e, Up, &[]);
    update(&mut e, Up, &[A]);
    assert!(update(&mut e, Up, &[]).is_empty());
    let events = update(&mut e, Down, &[]);
    assert!(matches!(events.first(), Some(Event::Cancelled { .. })));
    assert_eq!(chars(&events), "b");
}

#[test]
fn reset_discards_queued_output_and_requires_full_neutral_including_triggers() {
    use Button::*;
    use Dpad::*;
    let mut e = engine();
    arm(&mut e);
    update(&mut e, Up, &[]);
    update(&mut e, Up, &[A]);
    update(&mut e, Up, &[]);
    assert!(matches!(
        e.reset("dropped input"),
        Event::Reset {
            discarded: true,
            ..
        }
    ));
    assert!(update(&mut e, Neutral, &[R2]).is_empty());
    assert_eq!(update(&mut e, Neutral, &[]), [Event::Ready]);
    update(&mut e, Neutral, &[A]);
    assert_eq!(chars(&update(&mut e, Neutral, &[])), "b");
}

#[test]
fn pending_queue_is_bounded_and_overflow_discards_all_strokes() {
    use Button::*;
    use Dpad::*;
    let mut e = engine();
    arm(&mut e);
    update(&mut e, Up, &[]);
    for _ in 0..63 {
        update(&mut e, Up, &[A]);
        assert!(update(&mut e, Up, &[]).is_empty());
    }
    assert!(matches!(
        update(&mut e, Up, &[A]).as_slice(),
        [Event::Reset {
            discarded: true,
            ..
        }]
    ));
    assert_eq!(update(&mut e, Neutral, &[]), [Event::Ready]);
}

#[test]
fn opposite_pair_option_keeps_adjacent_pairs_and_excludes_opposite_supersets() {
    use Button::*;
    let p = Profile {
        exclude_opposite_face_pairs: true,
        ..Profile::default()
    };
    for pair in [[A, B], [A, X], [B, Y], [X, Y]] {
        assert!(p.allows(input(Dpad::Neutral, &pair)));
    }
    for pair in [[A, Y], [B, X]] {
        assert!(!p.allows(input(Dpad::Neutral, &pair)));
        assert!(!p.allows(input(Dpad::Up, &[pair[0], pair[1], L2])));
    }
}

#[test]
fn layout_validation_enforces_fixed_keys_base_letters_symbol_layer_and_shift_pairs() {
    use Button::*;
    use Dpad::*;
    let base = layout();
    base.validate().unwrap();
    for invalid in [
        binding('a', input(Up, &[A]), &[]),
        binding('a', input(Up, &[]), &[R2]),
        binding('a', input(Neutral, &[L1]), &[]),
        binding('1', input(Up, &[]), &[]),
        binding('A', input(Neutral, &[X]), &[L2]),
        binding('a', input(Neutral, &[L2]), &[]),
        binding('a', input(Up, &[]), &[L1, R2]),
    ] {
        let mut l = base.clone();
        l.bindings[0] = invalid;
        assert!(l.validate().is_err());
    }
    let mut l = base.clone();
    l.bindings[0].held_dpad = Up;
    assert!(l.validate().is_err());
    let mut l = base.clone();
    l.schema_version = 1;
    assert!(l.validate().is_err());
    let mut l = base.clone();
    l.strategy.shift = Some(R2);
    assert!(l.validate().is_err());
    let mut l = base;
    l.bindings.push(l.bindings[0].clone());
    assert!(l.validate().is_err());
}

#[test]
fn raw_report_replay_and_typing_runtime_agree_on_overlaps_held_layers_and_pause() {
    use Button::*;
    use Dpad::*;
    use chordpad::{
        controller::{Calibration, RawEvent},
        inspector::{Header, Record},
        typing::{Output, Runtime},
    };
    use std::{cell::RefCell, fs, process::Command, rc::Rc};
    struct Sink(Rc<RefCell<String>>);
    impl Output for Sink {
        fn send(&mut self, action: &Action) -> chordpad::Result<()> {
            if let Action::Character(c) = action {
                self.0.borrow_mut().push(*c);
            }
            Ok(())
        }
        fn release_all(&mut self) -> chordpad::Result<()> {
            Ok(())
        }
    }
    let sent = Rc::new(RefCell::new(String::new()));
    let mut runtime = Runtime::new(layout(), &[L1, R1, R2, L2], false, Sink(sent.clone())).unwrap();
    let states = [
        input(Neutral, &[]),
        input(Up, &[]),
        input(Up, &[A]),
        input(Up, &[]),
        input(Neutral, &[]),
        input(Up, &[L2, A]),
        input(Neutral, &[L2]),
        input(Neutral, &[]),
        input(Up, &[R2]),
        input(Neutral, &[R2]),
        input(Neutral, &[R2, A]),
        input(Neutral, &[R2]),
        input(Up, &[R2, L2]),
        input(Neutral, &[R2, L2]),
        input(Neutral, &[R2, L2, A]),
        input(Neutral, &[R2, L2]),
        input(Neutral, &[]),
    ];
    let calibration = Calibration::default();
    let mut records = vec![Record::Header {
        header: Box::new(Header::new(calibration.clone(), Some(layout()))),
    }];
    for (i, state) in states.into_iter().enumerate() {
        runtime.update(&snapshot(state)).unwrap();
        let mut events: Vec<_> = calibration
            .buttons
            .iter()
            .map(|(button, code)| RawEvent {
                event_type: 1,
                code: *code,
                value: i32::from(state.mask & button.bit() != 0),
            })
            .collect();
        let (x, y) = state.dpad.position();
        events.extend(
            [
                (16, i32::from(x)),
                (17, i32::from(y)),
                (2, if state.mask & L2.bit() != 0 { 255 } else { 0 }),
                (5, if state.mask & R2.bit() != 0 { 255 } else { 0 }),
            ]
            .into_iter()
            .map(|(code, value)| RawEvent {
                event_type: 3,
                code,
                value,
            }),
        );
        if i % 2 == 0 {
            events.reverse();
        }
        records.push(Record::Frame {
            at_us: i as u64 * 10,
            events,
        });
    }
    assert_eq!(*sent.borrow(), "abAB12!@");
    records.push(Record::End {
        at_us: 1000,
        reason: "complete".into(),
    });
    let dir = tempfile::tempdir().unwrap();
    let recording = dir.path().join("independent.jsonl");
    fs::write(
        &recording,
        records
            .iter()
            .map(|r| serde_json::to_string(r).unwrap() + "\n")
            .collect::<String>(),
    )
    .unwrap();
    let replay = || {
        Command::new(env!("CARGO_BIN_EXE_chordpad"))
            .args(["replay", "--input"])
            .arg(&recording)
            .arg("--json")
            .output()
            .unwrap()
    };
    let result = replay();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(result.stdout, replay().stdout);
    let views: Vec<serde_json::Value> = String::from_utf8(result.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let typed: String = views
        .iter()
        .flat_map(|v| v["events"].as_array().unwrap())
        .filter_map(|e| e["action"]["value"].as_str())
        .collect();
    assert_eq!(typed, *sent.borrow());
    for state in [input(Up, &[]), input(Up, &[A]), input(Up, &[])] {
        runtime.update(&snapshot(state)).unwrap();
    }
    runtime.set_paused(true).unwrap();
    runtime.set_paused(false).unwrap();
    runtime.update(&snapshot(input(Neutral, &[]))).unwrap();
    assert_eq!(*sent.borrow(), "abAB12!@");
}

#[test]
fn stick_modifiers_are_latched_and_later_modifier_presses_cancel_only_active_strokes() {
    use Button::*;
    use Dpad::*;
    use Modifier::*;
    let mut e = engine();
    arm(&mut e);
    let mut state = snapshot(input(Up, &[L2]));
    state.right_stick = Up;
    e.update(&state);
    state.buttons.push(A);
    e.update(&state);
    state.buttons.retain(|b| *b != A);
    assert!(e.update(&state).is_empty());
    // Releasing the stick first preserves Alt on both queued uppercase strokes.
    state.right_stick = Neutral;
    state.dpad = Some(Neutral);
    let events = e.update(&state);
    assert_eq!(chars(&events), "AB");
    assert!(events.iter().all(
        |event| matches!(event,Event::Chord{keyboard_modifiers,..} if keyboard_modifiers==&[Alt])
    ));
    state.left_stick = Up;
    state.dpad = Some(Up);
    e.update(&state);
    state.left_stick = UpLeft;
    state.buttons.push(A);
    assert!(
        e.update(&state)
            .iter()
            .any(|e| matches!(e, Event::Cancelled { .. }))
    );
    state.dpad = Some(Neutral);
    state.buttons.retain(|b| *b != A);
    let events = e.update(&state);
    assert_eq!(chars(&events), "B");
    assert!(
        matches!(events.as_slice(),[Event::Chord{keyboard_modifiers,..}] if keyboard_modifiers==&[Ctrl,Alt])
    );
}

#[test]
fn held_sticks_must_center_before_startup_or_reset_can_rearm() {
    let mut e = engine();
    let mut state = snapshot(Input::default());
    state.left_stick = Dpad::Right;
    assert!(e.update(&state).is_empty());
    assert_eq!(e.phase(), Phase::Waiting);
    state.left_stick = Dpad::Neutral;
    assert_eq!(e.update(&state), [Event::Ready]);
    state.left_stick = Dpad::Up;
    state.dpad = Some(Dpad::Up);
    e.update(&state);
    assert!(matches!(
        e.reset("pause"),
        Event::Reset {
            discarded: true,
            ..
        }
    ));
    state.dpad = Some(Dpad::Neutral);
    assert!(e.update(&state).is_empty());
    state.left_stick = Dpad::Neutral;
    assert_eq!(e.update(&state), [Event::Ready]);
}

#[cfg(target_os = "linux")]
#[test]
fn thumbsticks_shift_layers_and_editing_keys_reach_keyboard_with_balanced_modifiers() {
    use Button::*;
    use Dpad::*;
    use Modifier::*;
    use chordpad::{
        output::{KeySink, Keyboard, stroke_with_modifiers},
        typing::Runtime,
    };
    use evdev::{InputEvent, KeyCode};
    use std::{cell::RefCell, io, rc::Rc};
    type Frames = Rc<RefCell<Vec<Vec<(u16, i32)>>>>;
    #[derive(Clone)]
    struct Sink(Frames);
    impl KeySink for Sink {
        fn emit(&mut self, events: &[InputEvent]) -> io::Result<()> {
            self.0
                .borrow_mut()
                .push(events.iter().map(|e| (e.code(), e.value())).collect());
            Ok(())
        }
    }
    let frames = Rc::new(RefCell::new(vec![]));
    let mut runtime = Runtime::new(
        layout(),
        &[L1, R1, L2, R2],
        false,
        Keyboard::new(Sink(frames.clone())),
    )
    .unwrap();
    runtime.update(&snapshot(Input::default())).unwrap();
    let mut state = snapshot(input(Neutral, &[L2]));
    state.left_stick = Right;
    runtime.update(&state).unwrap();
    for (dpad, button, expected) in [
        (Up, None, 30),
        (Neutral, Some(R1), 57),
        (Neutral, Some(L1), 14),
    ] {
        state.dpad = Some(dpad);
        state.buttons.extend(button);
        runtime.update(&state).unwrap();
        state.dpad = Some(Neutral);
        state.buttons = vec![L2];
        runtime.update(&state).unwrap();
        let records = frames.borrow();
        assert_eq!(
            records[records.len() - 2],
            [(29, 1), (56, 1), (42, 1), (125, 1), (expected, 1)]
        );
        assert_eq!(
            records[records.len() - 1],
            [(expected, 0), (125, 0), (42, 0), (56, 0), (29, 0)]
        );
    }
    state = snapshot(input(Neutral, &[R2]));
    state.right_stick = Right;
    runtime.update(&state).unwrap();
    state.dpad = Some(Up);
    runtime.update(&state).unwrap();
    state.dpad = Some(Neutral);
    runtime.update(&state).unwrap();
    assert_eq!(frames.borrow()[6], [(29, 1), (2, 1)]);
    state.buttons.push(L2);
    runtime.update(&state).unwrap();
    state.dpad = Some(Up);
    runtime.update(&state).unwrap();
    state.dpad = Some(Neutral);
    runtime.update(&state).unwrap();
    assert_eq!(frames.borrow()[8], [(29, 1), (42, 1), (2, 1)]);
    assert_eq!(frames.borrow()[9], [(2, 0), (42, 0), (29, 0)]);
    for (button, code) in [(MenuButton::Start, 28), (MenuButton::Select, 1)] {
        state.menu_buttons = vec![button];
        runtime.update(&state).unwrap();
        state.menu_buttons.clear();
        runtime.update(&state).unwrap();
        let frames = frames.borrow();
        assert_eq!(frames[frames.len() - 2], [(29, 1), (42, 1), (code, 1)]);
        assert_eq!(frames[frames.len() - 1], [(code, 0), (42, 0), (29, 0)]);
    }
    let shortcut = Action::Shortcut(Shortcut {
        modifiers: vec![Ctrl, Shift],
        key: Key::A,
    });
    assert_eq!(
        stroke_with_modifiers(&shortcut, &[Ctrl, Alt, Meta]).unwrap(),
        [
            KeyCode(29),
            KeyCode(56),
            KeyCode(42),
            KeyCode(125),
            KeyCode(30)
        ]
    );
}

#[test]
fn raw_stick_reports_replay_latched_modifiers_with_shift_and_number_layer() {
    use chordpad::{
        controller::{Calibration, RawEvent, StickAxis, Thumbstick},
        inspector::{Header, Record},
    };
    use std::{fs, process::Command};
    let axis = |code| StickAxis {
        code,
        minimum: -1000,
        center: 0,
        maximum: 1000,
        inverted: false,
    };
    let calibration = Calibration {
        schema_version: 2,
        thumbsticks: [
            (Side::Left, Thumbstick::new(axis(0), axis(1))),
            (Side::Right, Thumbstick::new(axis(3), axis(4))),
        ]
        .into(),
        ..Calibration::default()
    };
    let mut records = vec![Record::Header {
        header: Box::new(Header::new(calibration, Some(layout()))),
    }];
    for (i, events) in [
        vec![],
        vec![(3, 2, 255), (3, 0, 1000), (3, 17, -1)],
        vec![(3, 17, 0), (3, 0, 0)],
        vec![(3, 2, 0), (3, 5, 255), (3, 3, 1000), (1, 304, 1)],
        vec![(1, 304, 0)],
        vec![(3, 2, 255), (3, 17, -1)],
        vec![(3, 17, 0), (3, 2, 0), (3, 5, 0), (3, 3, 0)],
    ]
    .into_iter()
    .enumerate()
    {
        records.push(Record::Frame {
            at_us: i as u64 * 10,
            events: events
                .into_iter()
                .map(|(event_type, code, value)| RawEvent {
                    event_type,
                    code,
                    value,
                })
                .collect(),
        });
    }
    records.push(Record::End {
        at_us: 100,
        reason: "complete".into(),
    });
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sticks.jsonl");
    fs::write(
        &path,
        records
            .iter()
            .map(|r| serde_json::to_string(r).unwrap() + "\n")
            .collect::<String>(),
    )
    .unwrap();
    let replay = || {
        Command::new(env!("CARGO_BIN_EXE_chordpad"))
            .args(["replay", "--input"])
            .arg(&path)
            .arg("--json")
            .output()
            .unwrap()
    };
    let result = replay();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(result.stdout, replay().stdout);
    let views: Vec<serde_json::Value> = String::from_utf8(result.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let chords: Vec<_> = views
        .iter()
        .flat_map(|v| v["events"].as_array().unwrap())
        .filter(|e| e["type"] == "chord")
        .collect();
    assert_eq!(
        chords
            .iter()
            .map(|e| e["action"]["value"].as_str().unwrap())
            .collect::<String>(),
        "A2!"
    );
    assert_eq!(
        chords[0]["keyboard_modifiers"],
        serde_json::json!(["ctrl", "alt", "meta"])
    );
    for chord in &chords[1..] {
        assert_eq!(chord["keyboard_modifiers"], serde_json::json!(["ctrl"]));
    }
}

#[test]
fn base_punctuation_resolves_rolled_chords_and_shift_pairs() {
    use Button::*;
    let mut layout = layout();
    for binding in &mut layout.bindings {
        if matches!(binding.action, Action::Character('c' | 'C')) {
            binding.chord.dpad = Dpad::DownLeft;
        }
    }
    layout.profile.base_punctuation = vec!['\'', ',', '.', ';', '/'];
    let mappings = [
        ('\'', Some('"'), vec![A, B, R1]),
        (',', Some('<'), vec![A, X, R1]),
        ('.', Some('>'), vec![B, Y, R1]),
        ('/', Some('?'), vec![X, Y, R1]),
    ];
    for (plain, shifted, buttons) in &mappings {
        let chord = input(Dpad::Neutral, buttons);
        layout.bindings.push(binding(*plain, chord, &[]));
        if let Some(shifted) = shifted {
            layout.bindings.push(binding(*shifted, chord, &[L2]));
        }
    }
    for (c, held) in [(';', vec![]), (':', vec![L2])] {
        layout
            .bindings
            .push(binding(c, input(Dpad::UpLeft, &[L1]), &held));
    }
    layout.validate().unwrap();
    let mut forbidden = layout.clone();
    forbidden.profile.base_punctuation.clear();
    assert!(forbidden.validate().is_err());
    let mut e = Recognizer::new(
        layout.strategy.clone(),
        layout.profile.clone(),
        Some(layout),
    )
    .unwrap();
    arm(&mut e);
    for (held, expected) in [(vec![], ';'), (vec![L2], ':')] {
        update(&mut e, Dpad::Neutral, &held);
        update(&mut e, Dpad::Up, &held);
        let mut down = held.clone();
        down.push(L1);
        update(&mut e, Dpad::UpLeft, &down);
        assert!(update(&mut e, Dpad::Left, &down).is_empty());
        assert!(update(&mut e, Dpad::Left, &held).is_empty());
        assert_eq!(
            chars(&update(&mut e, Dpad::Neutral, &held)),
            expected.to_string()
        );
        update(&mut e, Dpad::Neutral, &[]);
    }
    for (plain, shifted, buttons) in mappings {
        for (held, expected) in
            std::iter::once((vec![], plain)).chain(shifted.map(|s| (vec![L2], s)))
        {
            update(&mut e, Dpad::Neutral, &held);
            let mut down = held.clone();
            down.push(R1);
            update(&mut e, Dpad::Neutral, &down);
            down.push(buttons[0]);
            update(&mut e, Dpad::Neutral, &down);
            down.push(buttons[1]);
            update(&mut e, Dpad::Neutral, &down);
            down.retain(|b| *b != R1);
            assert!(update(&mut e, Dpad::Neutral, &down).is_empty());
            down.retain(|b| *b != buttons[0]);
            assert!(update(&mut e, Dpad::Neutral, &down).is_empty());
            assert_eq!(
                chars(&update(&mut e, Dpad::Neutral, &held)),
                expected.to_string()
            );
            update(&mut e, Dpad::Neutral, &[]);
        }
    }
    for punctuation in [vec!['a'], vec!['1'], vec!['"'], vec!['é'], vec![',', ',']] {
        let profile = Profile {
            base_punctuation: punctuation,
            ..Profile::default()
        };
        assert!(profile.validate().is_err());
    }
}
