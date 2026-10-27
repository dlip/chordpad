use chordpad::{
    chord::{Event, Recognizer},
    controller::Snapshot,
    model::{Dpad, Layout, Modifier},
};
use serde_json::{Value, json};

#[test]
fn android_recognition_vectors_match_the_rust_engine() {
    let layout: Layout = serde_json::from_str(include_str!(
        "../android/app/src/main/assets/legacy-layout.json"
    ))
    .unwrap();
    layout.validate().unwrap();
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../android/app/src/test/resources/chord-vectors.json"
    ))
    .unwrap();
    for case in cases {
        let mut engine = Recognizer::new(
            layout.strategy.clone(),
            layout.profile.clone(),
            Some(layout.clone()),
        )
        .unwrap();
        let mut actual = vec![];
        for frame in case["frames"].as_array().unwrap() {
            if frame["reset"] == true {
                engine.reset("test reset");
                continue;
            }
            let now = frame.get("at_ms").and_then(Value::as_u64).unwrap_or(0);
            let events = if frame["tick"] == true {
                engine.repeat_at(now)
            } else {
                let snapshot = Snapshot {
                    buttons: serde_json::from_value(frame["buttons"].clone()).unwrap(),
                    dpad: serde_json::from_value(frame["dpad"].clone()).unwrap(),
                    menu_buttons: serde_json::from_value(
                        frame.get("menu_buttons").cloned().unwrap_or(json!([])),
                    )
                    .unwrap(),
                    l2: 0.0,
                    r2: 0.0,
                    left_stick: frame
                        .get("left_stick")
                        .map(|value| serde_json::from_value(value.clone()).unwrap())
                        .unwrap_or(Dpad::Neutral),
                    right_stick: frame
                        .get("right_stick")
                        .map(|value| serde_json::from_value(value.clone()).unwrap())
                        .unwrap_or(Dpad::Neutral),
                };
                engine.update_at(&snapshot, now)
            };
            for event in events {
                let (action, modifiers) = match event {
                    Event::Chord {
                        action: Some(action),
                        keyboard_modifiers,
                        ..
                    }
                    | Event::Menu {
                        action,
                        keyboard_modifiers,
                        ..
                    } => (action, keyboard_modifiers),
                    _ => continue,
                };
                let mut action = serde_json::to_value(action).unwrap();
                assert!(
                    modifiers.windows(2).all(|pair| pair[0] < pair[1]),
                    "{}",
                    case["name"]
                );
                let modifiers = modifiers.iter().fold(0, |mask, modifier| {
                    mask | match modifier {
                        Modifier::Ctrl => 1,
                        Modifier::Alt => 2,
                        Modifier::Shift => 4,
                        Modifier::Meta => 8,
                    }
                });
                action["modifiers"] = json!(modifiers);
                actual.push(action);
            }
        }
        assert_eq!(json!(actual), case["actions"], "{}", case["name"]);
    }
}

#[test]
fn android_alpha_bank_vectors_match_linux() {
    use chordpad::{alpha_banks, model::Action};
    let layout = include_str!("../layout.json");
    assert_eq!(
        layout,
        include_str!("../android/app/src/main/assets/default-layout.json")
    );
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../android/app/src/test/resources/alpha-bank-vectors.json"
    ))
    .unwrap();
    for case in cases {
        let mut engine = alpha_banks::Engine::new(serde_json::from_str(layout).unwrap()).unwrap();
        let mut actual = vec![];
        for frame in case["frames"].as_array().unwrap() {
            if frame["reset"] == true {
                engine.reset("test reset");
                continue;
            }
            let now = frame.get("at_ms").and_then(Value::as_u64).unwrap_or(0);
            let events = if frame["tick"] == true {
                engine.repeat_at(now)
            } else {
                engine.update_at(
                    &Snapshot {
                        buttons: serde_json::from_value(frame["buttons"].clone()).unwrap(),
                        dpad: serde_json::from_value(frame["dpad"].clone()).unwrap(),
                        menu_buttons: serde_json::from_value(
                            frame.get("menu_buttons").cloned().unwrap_or(json!([])),
                        )
                        .unwrap(),
                        l2: 0.0,
                        r2: 0.0,
                        left_stick: serde_json::from_value(
                            frame.get("left_stick").cloned().unwrap_or(json!("neutral")),
                        )
                        .unwrap(),
                        right_stick: serde_json::from_value(
                            frame
                                .get("right_stick")
                                .cloned()
                                .unwrap_or(json!("neutral")),
                        )
                        .unwrap(),
                    },
                    now,
                )
            };
            for event in events {
                let (action, mut modifiers) = match event {
                    Event::Chord {
                        action: Some(action),
                        keyboard_modifiers,
                        ..
                    }
                    | Event::Menu {
                        action,
                        keyboard_modifiers,
                        ..
                    } => (action, keyboard_modifiers),
                    _ => continue,
                };
                let action = match action {
                    Action::Shortcut(shortcut) => {
                        modifiers.extend(shortcut.modifiers);
                        Action::Key(shortcut.key)
                    }
                    other => other,
                };
                let mut action = serde_json::to_value(action).unwrap();
                let mask = modifiers.iter().fold(0, |m, v| {
                    m | match v {
                        Modifier::Ctrl => 1,
                        Modifier::Alt => 2,
                        Modifier::Shift => 4,
                        Modifier::Meta => 8,
                    }
                });
                action["modifiers"] = json!(mask);
                actual.push(action);
            }
        }
        assert_eq!(json!(actual), case["actions"], "{}", case["name"]);
    }
}
