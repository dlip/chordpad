use padtype::{
    chord::{Event, Recognizer},
    controller::Snapshot,
    model::{Dpad, Layout, Modifier},
};
use serde_json::{Value, json};

#[test]
fn android_recognition_vectors_match_the_rust_engine() {
    let layout: Layout = serde_json::from_str(include_str!(
        "../android/app/src/main/assets/default-layout.json"
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
            for event in engine.update(&snapshot) {
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
