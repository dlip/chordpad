use chordpad::{
    controller::{Calibration, RawEvent},
    inspector::{Header, Record, Session},
    typing::TypingLayout,
};
use serde_json::Value;
use std::{fs, process::Command};

#[test]
fn default_layout_is_shared_with_android_and_legacy_layouts_remain_available() {
    assert_eq!(
        include_str!("../layout.json"),
        include_str!("../android/app/src/main/assets/default-layout.json")
    );
    assert_eq!(
        include_str!("../examples/legacy-layout.json"),
        include_str!("../android/app/src/main/assets/legacy-layout.json")
    );
    assert!(matches!(
        TypingLayout::parse(serde_json::from_str(include_str!("../layout.json")).unwrap()).unwrap(),
        TypingLayout::Alpha(_)
    ));
    assert!(matches!(
        TypingLayout::parse(
            serde_json::from_str(include_str!("../examples/legacy-layout.json")).unwrap()
        )
        .unwrap(),
        TypingLayout::Standard(_)
    ));
}

#[test]
fn default_cli_reference_has_all_banks_and_escapes_custom_characters() {
    let dir = tempfile::tempdir().unwrap();
    let mut layout: Value = serde_json::from_str(include_str!("../layout.json")).unwrap();
    layout["banks"][0]["bindings"][0]["character"] = Value::String("<".into());
    let input = dir.path().join("layout.json");
    let output = dir.path().join("layout.html");
    fs::write(&input, serde_json::to_vec(&layout).unwrap()).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_chordpad"))
        .args(["render", "--layout"])
        .arg(input)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let html = fs::read_to_string(output).unwrap();
    assert_eq!(html.matches("<section>").count(), 12);
    assert!(html.contains(">&lt;</strong>"));
    assert!(html.contains("Command+Shift+D"));
    assert!(html.contains("<em>!</em>"));
    assert!(!html.contains("@@"));
    assert!(
        html.find("R1 + R2: navigation").unwrap() < html.find("R2 tap twice: function").unwrap()
    );
}

#[test]
fn default_recordings_replay_alpha_and_one_shot_numbers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.jsonl");
    let layout =
        TypingLayout::parse(serde_json::from_str(include_str!("../layout.json")).unwrap()).unwrap();
    let mut session = Session::new(
        Header::with_layout(Calibration::default(), Some(layout)),
        Some(&path),
        true,
    )
    .unwrap();
    session
        .update(Record::Frame {
            at_us: 0,
            events: vec![],
        })
        .unwrap();
    for (at_us, event_type, code, value) in [
        (20_000, 1, 304, 1),
        (40_000, 1, 304, 0),
        (60_000, 3, 5, 255),
        (80_000, 3, 5, 0),
        (100_000, 3, 17, -1),
        (120_000, 3, 17, 0),
    ] {
        session
            .update(Record::Frame {
                at_us,
                events: vec![RawEvent {
                    event_type,
                    code,
                    value,
                }],
            })
            .unwrap();
    }
    session.finish(140_000, "test").unwrap();
    drop(session);
    let result = Command::new(env!("CARGO_BIN_EXE_chordpad"))
        .args(["replay", "--json", "--input"])
        .arg(path)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let actions: Vec<Value> = String::from_utf8(result.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .flat_map(|view| view["events"].as_array().unwrap().clone())
        .filter_map(|event| event.get("action").cloned())
        .map(|action| action["value"].clone())
        .collect();
    assert_eq!(
        actions,
        [Value::String("a".into()), Value::String("1".into())]
    );
}
