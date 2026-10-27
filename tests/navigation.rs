use chordpad::{
    Result,
    chord::{Event, Recognizer},
    controller::Snapshot,
    model::*,
    render,
    typing::{Output, Runtime},
};
use std::{cell::RefCell, rc::Rc};

fn layout() -> Layout {
    let mut layout: Layout = serde_json::from_str(include_str!(
        "../android/app/src/main/assets/legacy-layout.json"
    ))
    .unwrap();
    layout.profile.function_layer = false;
    layout.profile.function_volume = false;
    layout.profile.function_print_screen = false;
    layout
}

fn state(dpad: Dpad, buttons: &[Button]) -> Snapshot {
    Snapshot {
        dpad: Some(dpad),
        buttons: buttons.into(),
        menu_buttons: vec![],
        l2: 0.0,
        r2: 0.0,
        left_stick: Dpad::Neutral,
        right_stick: Dpad::Neutral,
    }
}

#[test]
fn navigation_reservations_validate_and_older_layouts_keep_their_digit() {
    let mut layout = layout();
    layout.bindings = vec![Binding {
        action: Action::Character('0'),
        chord: Chord {
            dpad: Dpad::Up,
            buttons: vec![Button::L1],
        },
        held: vec![Button::R2],
        held_dpad: Dpad::Neutral,
    }];
    assert!(layout.validate().unwrap_err().contains("reserved"));
    layout.profile.navigation_layer = false;
    layout.profile.navigation_media = false;
    layout.validate().unwrap();
    assert!(
        !serde_json::to_string(&layout)
            .unwrap()
            .contains("navigation_layer")
    );
    let mut engine = Recognizer::new(
        layout.strategy.clone(),
        layout.profile.clone(),
        Some(layout),
    )
    .unwrap();
    engine.update_at(&state(Dpad::Neutral, &[]), 0);
    assert!(
        engine
            .update_at(&state(Dpad::Up, &[Button::L1, Button::R2]), 1)
            .is_empty()
    );
    assert!(engine.repeat_at(1000).is_empty());
    assert!(matches!(
        engine
            .update_at(&state(Dpad::Neutral, &[Button::R2]), 1001)
            .as_slice(),
        [Event::Chord {
            action: Some(Action::Character('0')),
            ..
        }]
    ));
}

#[test]
fn navigation_respects_forbidden_modifier_contexts() {
    let mut layout = layout();
    layout.bindings.retain(|b| b.held.is_empty());
    layout
        .profile
        .forbidden_combinations
        .push(vec![Button::L2, Button::R2]);
    let mut engine = Recognizer::new(
        layout.strategy.clone(),
        layout.profile.clone(),
        Some(layout),
    )
    .unwrap();
    engine.update_at(&state(Dpad::Neutral, &[]), 0);
    assert!(
        engine
            .update_at(&state(Dpad::Up, &[Button::L1, Button::L2, Button::R2]), 10)
            .is_empty()
    );
    assert!(engine.repeat_at(500).is_empty());
    engine.update_at(&state(Dpad::Neutral, &[Button::L1, Button::R2]), 600);
    assert!(matches!(
        engine
            .update_at(&state(Dpad::Up, &[Button::L1, Button::R2]), 700)
            .as_slice(),
        [Event::Chord {
            action: Some(Action::Key(Key::Up)),
            ..
        }]
    ));
}

#[test]
fn pause_cannot_consume_the_navigation_trigger_in_a_sparse_layout() {
    let mut layout = layout();
    layout.bindings.retain(|b| b.held.is_empty());
    assert!(
        chordpad::typing::validate_pause(&layout, &[Button::R2])
            .unwrap_err()
            .contains("navigation")
    );
    assert!(chordpad::typing::validate_pause(&layout, &[Button::L2, Button::R2]).is_err());
}

#[derive(Clone, Default)]
struct Sink(Rc<RefCell<Sent>>);
type Sent = Vec<(Action, Vec<Modifier>)>;
impl Output for Sink {
    fn send(&mut self, action: &Action) -> Result<()> {
        self.send_modified(action, &[])
    }
    fn send_modified(&mut self, action: &Action, modifiers: &[Modifier]) -> Result<()> {
        self.0.borrow_mut().push((action.clone(), modifiers.into()));
        Ok(())
    }
    fn release_all(&mut self) -> Result<()> {
        Ok(())
    }
}

#[test]
fn typing_sends_modified_navigation_and_pause_or_disconnect_stops_it() {
    let sink = Sink::default();
    let mut runtime = Runtime::new(
        layout(),
        &[Button::L1, Button::R1, Button::L2, Button::R2],
        false,
        sink.clone(),
    )
    .unwrap();
    runtime.update(&state(Dpad::Neutral, &[])).unwrap();
    runtime
        .update(&state(Dpad::Left, &[Button::L1, Button::L2, Button::R2]))
        .unwrap();
    assert_eq!(
        *sink.0.borrow(),
        vec![(Action::Key(Key::Left), vec![Modifier::Shift])]
    );
    runtime.set_paused(true).unwrap();
    assert!(runtime.repeat().unwrap().is_empty());
    runtime.set_paused(false).unwrap();
    runtime.update(&state(Dpad::Left, &[Button::R2])).unwrap();
    runtime.reset("disconnected").unwrap();
    assert!(runtime.repeat().unwrap().is_empty());
    assert_eq!(sink.0.borrow().len(), 1);
}

#[test]
fn older_navigation_profiles_keep_immediate_independent_face_keys() {
    let mut layout = layout();
    layout.profile.navigation_media = false;
    let json = serde_json::to_value(&layout).unwrap();
    assert!(json["profile"].get("navigation_media").is_none());
    let mut engine = Recognizer::new(
        layout.strategy.clone(),
        layout.profile.clone(),
        Some(layout),
    )
    .unwrap();
    engine.update_at(&state(Dpad::Neutral, &[]), 0);
    engine.update_at(&state(Dpad::Neutral, &[Button::R2, Button::L1]), 1);
    let events = engine.update_at(
        &state(
            Dpad::Neutral,
            &[Button::R2, Button::L1, Button::A, Button::X],
        ),
        2,
    );
    let keys: Vec<_> = events
        .into_iter()
        .filter_map(|event| {
            if let Event::Chord {
                action: Some(Action::Key(key)),
                ..
            } = event
            {
                Some(key)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(keys, [Key::PageDown, Key::Home]);
}

#[test]
fn visual_reference_includes_eight_navigation_outputs() {
    let html = render::html(&layout()).unwrap();
    let navigation = html
        .split("aria-label=\"Navigation layer\"")
        .nth(1)
        .unwrap()
        .split("</section>")
        .next()
        .unwrap();
    assert_eq!(navigation.matches("class=\"chord-card\"").count(), 12);
    assert_eq!(navigation.matches("class=\"binding-map\"").count(), 2);
    assert!(!html.contains("<svg"));
    assert!(html.contains("R2 + L1: navigation"));
    assert!(html.contains("Add L2 to select text"));
}
