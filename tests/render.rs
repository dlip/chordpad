use padtype::{model::*, render};
use std::{fs, process::Command};

fn layout() -> Layout {
    let binding = |action, dpad, buttons, held| Binding {
        held_dpad: Dpad::Neutral,
        action,
        chord: Chord { dpad, buttons },
        held,
    };
    Layout {
        schema_version: SCHEMA_VERSION,
        cost_model_version: COST_MODEL_VERSION,
        strategy: Strategy {
            independent: false,
            name: "reference".into(),
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
            binding(
                Action::Character('a'),
                Dpad::Neutral,
                vec![Button::A],
                vec![],
            ),
            binding(
                Action::Character('A'),
                Dpad::Neutral,
                vec![Button::A],
                vec![Button::R2],
            ),
            binding(
                Action::Character('b'),
                Dpad::UpLeft,
                vec![Button::L1],
                vec![Button::L2],
            ),
            binding(
                Action::Character('B'),
                Dpad::UpLeft,
                vec![Button::L1],
                vec![Button::L2, Button::R2],
            ),
            binding(
                Action::Character(' '),
                Dpad::Neutral,
                vec![Button::R1],
                vec![],
            ),
            binding(
                Action::Character('\n'),
                Dpad::Neutral,
                vec![Button::B],
                vec![],
            ),
            binding(
                Action::Character('\t'),
                Dpad::Down,
                vec![],
                vec![Button::L2],
            ),
            binding(Action::Character('é'), Dpad::Up, vec![], vec![]),
            binding(
                Action::Character('<'),
                Dpad::Down,
                vec![Button::X],
                vec![Button::R2],
            ),
            binding(
                Action::Character('\''),
                Dpad::Neutral,
                vec![Button::X],
                vec![],
            ),
            binding(
                Action::Character('"'),
                Dpad::Neutral,
                vec![Button::X],
                vec![Button::R2],
            ),
            binding(Action::Key(Key::Backspace), Dpad::Right, vec![], vec![]),
            binding(
                Action::Shortcut(Shortcut {
                    modifiers: vec![Modifier::Ctrl, Modifier::Shift],
                    key: Key::C,
                }),
                Dpad::DownRight,
                vec![],
                vec![],
            ),
        ],
    }
}

#[test]
fn export_covers_every_binding_and_highlights_ordinary_and_held_controls() {
    let layout = layout();
    let html = render::html(&layout).unwrap();
    assert_eq!(
        html.matches("class=\"chord-card\"").count(),
        layout.bindings.len()
    );
    assert_eq!(html.matches("<section class=\"layer\"").count(), 4);
    assert!(html.contains("R2 · Shift"));
    assert!(html.contains("L2 · Layer"));
    assert!(html.contains("Ctrl + Shift + C"));
    assert!(html.contains("data-character=\"é\""));
    assert!(html.contains("data-character=\" \""));
    let tab = html
        .split("data-label=\"Tab\"")
        .nth(1)
        .unwrap()
        .split("</button>")
        .next()
        .unwrap();
    assert!(tab.contains("class=\"action-label word\""));
    let upper = html
        .split("class=\"chord-card\" data-character=\"B\"")
        .nth(1)
        .unwrap()
        .split("</button>")
        .next()
        .unwrap();
    assert!(upper.contains("Hold L2 + R2; press D-pad Up Left + L1"));
    for button in ["L2", "R2"] {
        assert!(upper.contains(&format!("class=\"control held\" data-button=\"{button}\"")));
    }
    assert!(upper.contains("class=\"control pressed\" data-button=\"L1\""));
    for direction in ["Up", "Left"] {
        assert!(upper.contains(&format!(
            "class=\"control pressed\" data-direction=\"{direction}\""
        )));
    }
    assert_eq!(render::html(&layout).unwrap(), html);
}

#[test]
fn long_single_control_actions_keep_full_names_without_overflowing_overview_labels() {
    let mut layout = layout();
    layout.strategy.shift = None;
    layout.strategy.layer = None;
    layout.bindings = vec![
        Binding {
            held_dpad: Dpad::Neutral,
            action: Action::Shortcut(Shortcut {
                modifiers: vec![
                    Modifier::Ctrl,
                    Modifier::Alt,
                    Modifier::Shift,
                    Modifier::Meta,
                ],
                key: Key::PageDown,
            }),
            chord: Chord {
                dpad: Dpad::Neutral,
                buttons: vec![Button::A],
            },
            held: vec![],
        },
        Binding {
            held_dpad: Dpad::Neutral,
            action: Action::Shortcut(Shortcut {
                modifiers: vec![Modifier::Ctrl],
                key: Key::C,
            }),
            chord: Chord {
                dpad: Dpad::Neutral,
                buttons: vec![Button::L1],
            },
            held: vec![],
        },
        Binding {
            held_dpad: Dpad::Neutral,
            action: Action::Key(Key::PageDown),
            chord: Chord {
                dpad: Dpad::Up,
                buttons: vec![],
            },
            held: vec![],
        },
        Binding {
            held_dpad: Dpad::Neutral,
            action: Action::Key(Key::Escape),
            chord: Chord {
                dpad: Dpad::Down,
                buttons: vec![],
            },
            held: vec![],
        },
    ];
    let html = render::html(&layout).unwrap();
    let overview = html
        .split("id=\"preview-diagram\">")
        .nth(1)
        .unwrap()
        .split("</svg>")
        .next()
        .unwrap();
    assert!(overview.contains("class=\"output-label\">…</text>"));
    assert!(overview.contains("class=\"shoulder-label\">L1  …</text>"));
    assert!(overview.contains("class=\"dpad-label compact\">Pg↓</text>"));
    assert!(overview.contains("class=\"dpad-label compact\">Esc</text>"));
    assert!(overview.contains("<title>Ctrl + Alt + Shift + Meta + Page Down</title>"));
    assert!(!overview.contains("class=\"output-label\">Ctrl"));
    assert!(html.contains("data-label=\"Ctrl + Alt + Shift + Meta + Page Down\""));
    assert!(html.contains(
        "class=\"action-label word\" dir=\"auto\">Ctrl + Alt + Shift + Meta + Page Down</span>"
    ));
}

#[test]
fn export_escapes_layout_text_and_handles_direct_and_custom_modifier_roles() {
    let mut layout = layout();
    layout.strategy.name = "</title><script>alert(1)</script>&\"".into();
    let html = render::html(&layout).unwrap();
    assert!(!html.contains("<script>alert(1)</script>"));
    assert!(html.contains("&lt;/title&gt;&lt;script&gt;alert(1)&lt;/script&gt;&amp;&quot;"));
    assert!(html.contains("data-character=\"&lt;\""));
    assert!(html.contains("data-character=\"&#39;\""));
    assert!(html.contains("data-character=\"&quot;\""));
    layout.bindings.retain(|b| b.held.is_empty());
    layout.strategy.shift = None;
    layout.strategy.layer = None;
    assert_eq!(
        render::html(&layout)
            .unwrap()
            .matches("<section class=\"layer\"")
            .count(),
        1
    );
    layout.strategy.shift = Some(Button::L1);
    layout.bindings.push(Binding {
        held_dpad: Dpad::Neutral,
        action: Action::Character('A'),
        chord: Chord {
            buttons: vec![Button::A],
            dpad: Dpad::Neutral,
        },
        held: vec![Button::L1],
    });
    let html = render::html(&layout).unwrap();
    assert!(html.contains("L1 · Shift"));
    assert!(!html.contains("R2 · Shift"));
    layout.schema_version = 99;
    assert!(render::html(&layout).is_err());
}

#[test]
fn cli_renders_and_invalid_inputs_preserve_existing_output() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("layout.json");
    let output = dir.path().join("layout.html");
    let original = serde_json::to_vec(&layout()).unwrap();
    fs::write(&input, &original).unwrap();
    let run = |output: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_padtype"))
            .arg("render")
            .arg("--layout")
            .arg(&input)
            .arg("--output")
            .arg(output)
            .output()
            .unwrap()
    };
    let rendered = run(&output);
    assert!(
        rendered.status.success(),
        "{}",
        String::from_utf8_lossy(&rendered.stderr)
    );
    let document = fs::read_to_string(&output).unwrap();
    assert!(document.starts_with("<!doctype html>"));
    assert_eq!(fs::read(&input).unwrap(), original);
    assert!(!run(&input).status.success());
    assert_eq!(fs::read(&input).unwrap(), original);
    fs::write(&input, "{}").unwrap();
    assert!(!run(&output).status.success());
    assert_eq!(fs::read_to_string(&output).unwrap(), document);
}

#[cfg(unix)]
#[test]
fn output_aliases_cannot_overwrite_the_source_layout() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("layout.json");
    let original = serde_json::to_vec(&layout()).unwrap();
    fs::write(&input, &original).unwrap();
    std::os::unix::fs::symlink("layout.json", dir.path().join("linked.html")).unwrap();
    std::os::unix::fs::symlink(dir.path(), dir.path().join("alias")).unwrap();
    for path in [
        dir.path().join("linked.html"),
        dir.path().join("alias/layout.json"),
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_padtype"))
            .arg("render")
            .arg("--layout")
            .arg(&input)
            .arg("--output")
            .arg(path)
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("must not overwrite"));
        assert_eq!(fs::read(&input).unwrap(), original);
    }
}

#[test]
fn independent_reference_shows_fixed_keys_shift_pairs_and_number_layer() {
    let mut layout = layout();
    layout.strategy = Strategy {
        name: "independent".into(),
        independent: true,
        shift: Some(Button::L2),
        layer: Some(Button::R2),
    };
    layout.recognition = "independent-release".into();
    let make = |c, held| Binding {
        action: Action::Character(c),
        chord: Chord {
            dpad: Dpad::Neutral,
            buttons: vec![Button::A],
        },
        held,
        held_dpad: Dpad::Neutral,
    };
    layout.bindings = vec![
        make('a', vec![]),
        make('A', vec![Button::L2]),
        make('1', vec![Button::R2]),
        make('!', vec![Button::R2, Button::L2]),
    ];
    let html = render::html(&layout).unwrap();
    assert_eq!(html.matches("class=\"chord-card\"").count(), 4);
    assert_eq!(html.matches("<section class=\"layer\"").count(), 4);
    assert!(html.contains("L2 · Shift"));
    assert!(html.contains("R2 · Layer"));
    assert!(html.contains("R1 is Space and L1 is Backspace in every layer"));
    assert!(html.contains("uppercase letters use the same chords as lowercase"));
    assert!(html.contains("Hold L2 + R2; press A"));
    assert!(html.contains("Thumbstick modifiers"));
    assert_eq!(html.matches("class=\"stick-cell\"").count(), 18);
    assert!(html.contains("Alt + Command"));
    assert!(html.contains("Ctrl + Command"));
}
