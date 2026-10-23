use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_padtype"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}

fn success(output: Output) -> Output {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[test]
fn generate_export_and_score_are_reproducible_and_respect_constraints() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("train.txt"), "aab ba AAB!\n").unwrap();
    fs::write(dir.path().join("held.txt"), "BA ab!\n").unwrap();
    fs::write(
        dir.path().join("config.json"),
        r#"{
        "profile": {"diagonals": false, "forbidden_combinations": [["A", "Y"]]},
        "strategies": [
            {"name":"direct"},
            {"name":"shift-layer", "shift":"L2", "layer":"R2"}
        ],
        "pinned_bindings": [{"action":{"type":"character","value":" "},"chord":{"buttons":["A"]}}]
    }"#,
    )
    .unwrap();
    let args = [
        "generate",
        "--corpus",
        "train.txt",
        "--held-out",
        "held.txt",
        "--config",
        "config.json",
        "--alphabet",
        "corpus",
        "--iterations",
        "300",
        "--restarts",
        "2",
        "--output",
        "layout.json",
    ];
    success(run(dir.path(), &args));
    let layout_bytes = fs::read(dir.path().join("layout.json")).unwrap();
    let report_bytes = fs::read(dir.path().join("layout.report.json")).unwrap();
    let report: Value = serde_json::from_slice(&report_bytes).unwrap();
    assert_eq!(report["candidates"].as_array().unwrap().len(), 2);
    assert!(report["skipped"].as_array().unwrap().is_empty());
    for candidate in report["candidates"].as_array().unwrap() {
        assert!(
            candidate["training"]["objective"].as_f64().unwrap()
                <= candidate["baseline"]["objective"].as_f64().unwrap()
        );
        let mut space_found = false;
        for binding in candidate["layout"]["bindings"].as_array().unwrap() {
            assert!(!binding["chord"]["dpad"].as_str().unwrap().contains('-'));
            let buttons = binding["chord"]["buttons"].as_array().unwrap();
            assert!(!buttons.contains(&Value::from("L2")));
            assert!(!buttons.contains(&Value::from("R2")));
            assert!(!(buttons.contains(&Value::from("A")) && buttons.contains(&Value::from("Y"))));
            if binding["action"]["value"] == " " {
                assert_eq!(buttons, &[Value::from("A")]);
                assert_eq!(binding["held"], serde_json::json!([]));
                space_found = true;
            }
        }
        assert!(space_found);
    }
    let scored = success(run(
        dir.path(),
        &["score", "--layout", "layout.json", "--corpus", "train.txt"],
    ));
    let score: Value = serde_json::from_slice(&scored.stdout).unwrap();
    assert!(
        (score["objective"].as_f64().unwrap()
            - report["candidates"][0]["training"]["objective"]
                .as_f64()
                .unwrap())
        .abs()
            < 1e-10
    );
    success(run(dir.path(), &args));
    assert_eq!(
        layout_bytes,
        fs::read(dir.path().join("layout.json")).unwrap()
    );
    assert_eq!(
        report_bytes,
        fs::read(dir.path().join("layout.report.json")).unwrap()
    );
}

#[test]
fn validation_errors_do_not_create_output_files() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("train.txt"), "ab").unwrap();
    fs::write(dir.path().join("held.txt"), "Ω").unwrap();
    fs::write(
        dir.path().join("bad.json"),
        r#"{"weights":{"activations":-1}}"#,
    )
    .unwrap();
    for args in [
        vec!["generate", "--corpus", "train.txt", "--config", "bad.json"],
        vec![
            "generate",
            "--corpus",
            "train.txt",
            "--alphabet",
            "corpus",
            "--held-out",
            "held.txt",
        ],
        vec!["generate", "--iterations", "0"],
        vec!["generate", "--strategy", "unknown"],
        vec![
            "generate",
            "--output",
            "layout.json",
            "--report",
            "./layout.json",
        ],
    ] {
        let output = run(dir.path(), &args);
        assert!(!output.status.success());
        assert!(!dir.path().join("layout.json").exists());
        assert!(!dir.path().join("layout.report.json").exists());
    }
}

#[test]
fn default_configuration_round_trips_and_unknown_fields_fail() {
    let dir = tempfile::tempdir().unwrap();
    let defaults = success(run(dir.path(), &["defaults"]));
    let config: padtype::model::Config = serde_json::from_slice(&defaults.stdout).unwrap();
    config.validate().unwrap();
    assert_eq!(config.strategies.len(), 7);
    assert_eq!(
        config.profile.modifier_only_buttons,
        [padtype::model::Button::L2, padtype::model::Button::R2]
    );
    let partial: padtype::model::Config =
        serde_json::from_str(r#"{"profile":{"diagonals":false}}"#).unwrap();
    assert_eq!(
        partial.profile.modifier_only_buttons,
        config.profile.modifier_only_buttons
    );
    assert!(
        serde_json::from_str::<padtype::model::Config>(r#"{"profile":{"diagonal":false}}"#)
            .is_err()
    );
}

#[test]
fn failed_export_preserves_an_existing_layout() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("train.txt"), "ab").unwrap();
    fs::write(dir.path().join("layout.json"), "existing layout").unwrap();
    let output = run(
        dir.path(),
        &[
            "generate",
            "--corpus",
            "train.txt",
            "--alphabet",
            "corpus",
            "--strategy",
            "direct",
            "--iterations",
            "1",
            "--restarts",
            "1",
            "--report",
            "missing/report.json",
        ],
    );
    assert!(!output.status.success());
    assert_eq!(
        fs::read_to_string(dir.path().join("layout.json")).unwrap(),
        "existing layout"
    );
    assert!(!dir.path().join("layout.report.json").exists());
}

#[test]
fn output_paths_cannot_overwrite_any_input_file() {
    let dir = tempfile::tempdir().unwrap();
    let inputs = [
        ("train.txt", "ab"),
        ("held.txt", "ba"),
        ("config.json", "{}"),
    ];
    for (path, contents) in inputs {
        fs::write(dir.path().join(path), contents).unwrap();
    }
    fs::write(dir.path().join("layout.json"), "existing layout").unwrap();
    fs::write(dir.path().join("layout.report.json"), "existing report").unwrap();
    for (input, _) in inputs {
        let absolute = dir.path().join(input).to_str().unwrap().to_owned();
        for target in [input, absolute.as_str()] {
            for flag in ["--output", "--report"] {
                let mut args = vec![
                    "generate",
                    "--corpus",
                    "train.txt",
                    "--held-out",
                    "held.txt",
                    "--config",
                    "config.json",
                    "--alphabet",
                    "corpus",
                    "--strategy",
                    "direct",
                    "--iterations",
                    "1",
                    "--restarts",
                    "1",
                    "--output",
                    "layout.json",
                    "--report",
                    "layout.report.json",
                ];
                let index = args.iter().position(|&arg| arg == flag).unwrap() + 1;
                args[index] = target;
                let output = run(dir.path(), &args);
                assert!(!output.status.success(), "{flag} {target} should fail");
                assert!(
                    String::from_utf8_lossy(&output.stderr)
                        .contains("must not overwrite input file")
                );
                for (path, contents) in inputs {
                    assert_eq!(fs::read_to_string(dir.path().join(path)).unwrap(), contents);
                }
                assert_eq!(
                    fs::read_to_string(dir.path().join("layout.json")).unwrap(),
                    "existing layout"
                );
                assert_eq!(
                    fs::read_to_string(dir.path().join("layout.report.json")).unwrap(),
                    "existing report"
                );
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn symlink_aliases_cannot_bypass_input_protection() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("train.txt"), "ab").unwrap();
    std::os::unix::fs::symlink(dir.path(), dir.path().join("alias")).unwrap();
    std::os::unix::fs::symlink("train.txt", dir.path().join("linked.txt")).unwrap();
    for target in ["alias/train.txt", "linked.txt"] {
        let output = run(
            dir.path(),
            &[
                "generate",
                "--corpus",
                "train.txt",
                "--output",
                target,
                "--strategy",
                "direct",
                "--alphabet",
                "corpus",
                "--iterations",
                "1",
                "--restarts",
                "1",
            ],
        );
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("must not overwrite input file"));
        assert_eq!(
            fs::read_to_string(dir.path().join("train.txt")).unwrap(),
            "ab"
        );
        assert!(dir.path().join("linked.txt").is_symlink());
    }
}

#[cfg(unix)]
#[test]
fn output_paths_must_differ_even_through_symlinked_directories() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("out")).unwrap();
    std::os::unix::fs::symlink("out", dir.path().join("alias")).unwrap();
    let output = run(
        dir.path(),
        &[
            "generate",
            "--output",
            "out/result.json",
            "--report",
            "alias/result.json",
            "--strategy",
            "direct",
            "--iterations",
            "1",
            "--restarts",
            "1",
        ],
    );
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("layout and report paths must be different")
    );
    assert!(!dir.path().join("out/result.json").exists());
}

#[test]
fn independent_generation_scores_and_renders_every_binding_reproducibly() {
    let dir = tempfile::tempdir().unwrap();
    let args = [
        "generate",
        "--independent",
        "--exclude-opposite-face-pairs",
        "--same-hand-penalty",
        "1",
        "--iterations",
        "500",
        "--restarts",
        "1",
    ];
    success(run(dir.path(), &args));
    let bytes = fs::read(dir.path().join("layout.json")).unwrap();
    let layout: padtype::model::Layout = serde_json::from_slice(&bytes).unwrap();
    layout.validate().unwrap();
    assert!(layout.strategy.independent);
    assert!(layout.profile.exclude_opposite_face_pairs);
    assert_eq!(layout.profile.costs.same_hand_transition, 1.0);
    assert!(layout.bindings.iter().any(|b| !b.held.is_empty()));
    use padtype::model::{Action, Button, Dpad, Key};
    let find = |action| layout.bindings.iter().find(|b| b.action == action).unwrap();
    assert_eq!(layout.strategy.shift, Some(Button::L2));
    assert_eq!(layout.strategy.layer, Some(Button::R2));
    for lower in 'a'..='z' {
        let base = find(Action::Character(lower));
        let upper = find(Action::Character(lower.to_ascii_uppercase()));
        assert!(base.held.is_empty());
        assert_eq!(base.chord, upper.chord);
        assert_eq!(upper.held, [Button::L2]);
    }
    for b in &layout.bindings {
        if matches!(b.action, Action::Character(c) if c.is_ascii_digit() || c.is_ascii_punctuation())
        {
            assert!(b.held.contains(&Button::R2));
        }
    }
    for (action, button) in [
        (Action::Character(' '), Button::R1),
        (Action::Key(Key::Backspace), Button::L1),
    ] {
        let b = find(action);
        assert_eq!(b.chord.buttons, [button]);
        assert_eq!(b.chord.dpad, Dpad::Neutral);
        assert!(b.held.is_empty());
    }

    for b in &layout.bindings {
        padtype::model::validate_independent_input(
            &layout.profile,
            b.chord.input().unwrap(),
            b.context().unwrap(),
        )
        .unwrap();
    }
    success(run(dir.path(), &["render", "--layout", "layout.json"]));
    let html = fs::read_to_string(dir.path().join("layout.html")).unwrap();
    assert!(!html.contains("<svg"));
    assert!(html.contains("data-labels="));
    assert!(
        html.contains("<span>a / A</span>")
            || html.contains("<strong class=\"action-label\">a / A</strong>")
    );
    fs::write(dir.path().join("task.txt"), "the quick brown fox\n").unwrap();
    let scored = success(run(
        dir.path(),
        &["score", "--layout", "layout.json", "--corpus", "task.txt"],
    ));
    let score: Value = serde_json::from_slice(&scored.stdout).unwrap();
    assert!(score["same_hand_transition_rate"].as_f64().unwrap() <= 1.0);
    success(run(dir.path(), &args));
    assert_eq!(bytes, fs::read(dir.path().join("layout.json")).unwrap());
    assert!(
        !run(
            dir.path(),
            &["generate", "--independent", "--same-hand-penalty=-1"]
        )
        .status
        .success()
    );
}

#[test]
fn independent_example_pins_base_punctuation_and_uses_dedicated_menu_keys() {
    use padtype::model::{Action, Button, Dpad, Key, Layout};
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("config.json"),
        include_str!("../examples/independent.json"),
    )
    .unwrap();
    success(run(
        dir.path(),
        &[
            "generate",
            "--config",
            "config.json",
            "--iterations",
            "10",
            "--restarts",
            "1",
        ],
    ));
    let layout: Layout =
        serde_json::from_slice(&fs::read(dir.path().join("layout.json")).unwrap()).unwrap();
    layout.validate().unwrap();
    assert_eq!(layout.profile.base_punctuation, ['\'', ',', '.', ';', '/']);
    assert!(layout.profile.navigation_layer);
    assert!(layout.profile.function_layer);
    assert_eq!(layout.function_bindings().len(), 15);
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
        assert_eq!(binding.chord.dpad, dpad);
        assert!(binding.chord.buttons.is_empty());
    }
    for (digit, button) in [('9', Button::Y), ('0', Button::B)] {
        let binding = layout
            .bindings
            .iter()
            .find(|b| b.action == Action::Character(digit))
            .unwrap();
        assert_eq!(binding.chord.buttons, [button]);
        assert_eq!(binding.chord.dpad, Dpad::Neutral);
    }
    assert!(layout.bindings.iter().all(|b| !b.held.contains(&Button::R2)
        || (!b.chord.buttons.contains(&Button::L1) && !b.chord.buttons.contains(&Button::R1))));
    for (plain, shifted, buttons) in [
        ('\'', Some('"'), vec![Button::A, Button::B, Button::R1]),
        (',', Some('<'), vec![Button::A, Button::X, Button::R1]),
        ('.', Some('>'), vec![Button::B, Button::Y, Button::R1]),
        ('/', Some('?'), vec![Button::X, Button::Y, Button::R1]),
        (';', Some(':'), vec![Button::L1]),
    ] {
        let base = layout
            .bindings
            .iter()
            .find(|b| b.action == Action::Character(plain))
            .unwrap();
        assert_eq!(base.chord.buttons, buttons);
        assert_eq!(
            base.chord.dpad,
            if plain == ';' {
                Dpad::UpLeft
            } else {
                Dpad::Neutral
            }
        );
        assert!(base.held.is_empty());
        if let Some(shifted) = shifted {
            let upper = layout
                .bindings
                .iter()
                .find(|b| b.action == Action::Character(shifted))
                .unwrap();
            assert_eq!(upper.chord, base.chord);
            assert_eq!(upper.held, [Button::L2]);
        }
    }
    for action in [
        Action::Character('\n'),
        Action::Key(Key::Enter),
        Action::Key(Key::Escape),
    ] {
        assert!(!layout.bindings.iter().any(|b| b.action == action));
    }
    success(run(dir.path(), &["render", "--layout", "layout.json"]));
    fs::write(dir.path().join("task.txt"), "'\",.<>;:/?\n").unwrap();
    success(run(
        dir.path(),
        &["score", "--layout", "layout.json", "--corpus", "task.txt"],
    ));
    fs::write(dir.path().join("task.txt"), "\n\n").unwrap();
    let scored = success(run(
        dir.path(),
        &["score", "--layout", "layout.json", "--corpus", "task.txt"],
    ));
    let score: Value = serde_json::from_slice(&scored.stdout).unwrap();
    assert_eq!(score["metrics"]["activations"], 1.0);
    assert_eq!(score["metrics"]["effort"], 1.5);
}
