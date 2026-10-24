use chordpad::{
    controller::Calibration,
    inspector::{Header, Record, Session},
    model::*,
};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_chordpad"))
        .args(args)
        .output()
        .unwrap()
}
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/recordings")
        .join(format!("{name}.jsonl"))
}
fn views(output: Output) -> Vec<Value> {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
fn events(views: &[Value]) -> Vec<&Value> {
    views
        .iter()
        .flat_map(|v| v["events"].as_array().unwrap())
        .collect()
}

#[test]
fn example_recordings_replay_deterministically_with_expected_chords() {
    for (name, count, cancelled) in [("chords", 5, 0), ("cancellation", 1, 3), ("triggers", 3, 0)] {
        let path = fixture(name);
        let path = path.to_str().unwrap();
        let a = run(&["replay", "--input", path, "--json"]);
        let b = run(&["replay", "--input", path, "--json"]);
        assert_eq!(a.stdout, b.stdout);
        let views = views(a);
        let events = events(&views);
        assert_eq!(
            events.iter().filter(|v| v["type"] == "chord").count(),
            count,
            "{name}"
        );
        assert_eq!(
            events.iter().filter(|v| v["type"] == "cancelled").count(),
            cancelled,
            "{name}"
        );
        assert_eq!(views.last().unwrap()["phase"], "waiting");
        if name == "triggers" {
            let chords: Vec<_> = events.iter().filter(|v| v["type"] == "chord").collect();
            assert_eq!(chords[0]["held"], serde_json::json!(["L2"]));
            assert_eq!(chords[1]["held"], serde_json::json!(["L2"]));
            assert_eq!(chords[2]["held"], serde_json::json!(["R2"]));
        }
        if name == "cancellation" {
            assert!(
                events
                    .iter()
                    .any(|v| v["type"] == "reset" && v["discarded"] == true)
            );
        }
    }
}

#[test]
fn malformed_truncated_and_out_of_order_recordings_fail() {
    let source = fs::read_to_string(fixture("chords")).unwrap();
    let records: Vec<Value> = source
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.jsonl");
    let mut bad_version = records.clone();
    bad_version[0]["header"]["schema_version"] = 99.into();
    let mut backward = records.clone();
    backward[3]["at_us"] = 1.into();
    let mut after_end = records.clone();
    after_end.push(records[1].clone());
    let mut invalid_calibration = records.clone();
    invalid_calibration[0]["header"]["calibration"]["triggers"]["L2"]["pressed"] = 0.into();
    for bad in [
        bad_version,
        backward,
        after_end,
        invalid_calibration,
        records[1..].to_vec(),
        records[..records.len() - 1].to_vec(),
    ] {
        fs::write(
            &path,
            bad.iter()
                .map(|record| serde_json::to_string(record).unwrap() + "\n")
                .collect::<String>(),
        )
        .unwrap();
        assert!(
            !run(&["replay", "--input", path.to_str().unwrap(), "--json"])
                .status
                .success()
        );
    }
}

#[test]
fn recording_and_calibration_exports_never_replace_existing_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("existing.json");
    fs::write(&path, "keep this file").unwrap();
    let header = Header::new(Calibration::default(), None);
    assert!(Session::new(header, Some(&path), true).is_err());
    assert!(chordpad::inspector::write_new_json(&path, &Calibration::default()).is_err());
    assert_eq!(fs::read_to_string(path).unwrap(), "keep this file");
}

#[test]
fn generated_layout_is_used_to_resolve_recorded_chords() {
    let dir = tempfile::tempdir().unwrap();
    let corpus = dir.path().join("train.txt");
    let output = dir.path().join("layout.json");
    let recording = dir.path().join("input.jsonl");
    fs::write(&corpus, "aA bB").unwrap();
    let generated = run(&[
        "generate",
        "--corpus",
        corpus.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--alphabet",
        "corpus",
        "--strategy",
        "l2-shift-r2-layer",
        "--iterations",
        "5",
        "--restarts",
        "1",
    ]);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let layout: Layout = serde_json::from_str(&fs::read_to_string(output).unwrap()).unwrap();
    let binding = layout
        .bindings
        .iter()
        .find(|b| b.action == Action::Character('A'))
        .unwrap();
    let down: Vec<_> = binding
        .chord
        .buttons
        .iter()
        .chain(binding.held.iter())
        .copied()
        .collect();
    let (x, y) = binding.chord.dpad.position();
    let calibration = Calibration::default();
    let mut press = vec![];
    for (button, code) in &calibration.buttons {
        press.push(chordpad::controller::RawEvent {
            event_type: 1,
            code: *code,
            value: i32::from(down.contains(button)),
        });
    }
    for (code, value) in [
        (16, i32::from(x)),
        (17, i32::from(y)),
        (2, if down.contains(&Button::L2) { 255 } else { 0 }),
        (5, if down.contains(&Button::R2) { 255 } else { 0 }),
    ] {
        press.push(chordpad::controller::RawEvent {
            event_type: 3,
            code,
            value,
        });
    }
    let release = press
        .iter()
        .map(|e| chordpad::controller::RawEvent { value: 0, ..*e })
        .collect();
    let records = [
        Record::Header {
            header: Box::new(Header::new(calibration, Some(layout))),
        },
        Record::Frame {
            at_us: 0,
            events: vec![],
        },
        Record::Frame {
            at_us: 10,
            events: press,
        },
        Record::Frame {
            at_us: 20,
            events: release,
        },
        Record::End {
            at_us: 30,
            reason: "test complete".into(),
        },
    ];
    fs::write(
        &recording,
        records
            .iter()
            .map(|r| serde_json::to_string(r).unwrap() + "\n")
            .collect::<String>(),
    )
    .unwrap();
    let views = views(run(&[
        "replay",
        "--input",
        recording.to_str().unwrap(),
        "--json",
    ]));
    let events = events(&views);
    assert!(events.iter().any(|v| v["type"] == "chord"
        && v["action"] == serde_json::json!({"type":"character","value":"A"})));
}
