use chordpad::{model::*, render};
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

fn exported_labels(html: &str) -> Vec<String> {
    html.split("data-labels=\"")
        .skip(1)
        .flat_map(|part| {
            let attribute = part
                .split('"')
                .next()
                .unwrap()
                .replace("&quot;", "\"")
                .replace("&#39;", "'")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&amp;", "&");
            serde_json::from_str::<Vec<String>>(&attribute).unwrap()
        })
        .collect()
}

#[test]
fn export_pairs_shift_and_keeps_every_binding_without_controller_svgs() {
    let layout = layout();
    let html = render::html(&layout).unwrap();
    assert_eq!(exported_labels(&html).len(), layout.bindings.len());
    assert!(!html.contains("<svg"));
    assert!(!html.contains("preview-diagram"));
    assert_eq!(html.matches("<section class=\"layer\"").count(), 2);
    assert!(html.contains("<span>a / A</span>"));
    assert!(html.contains("<span>b / B</span>"));
    assert!(html.contains("Hold L2 + R2; press D-pad Up Left + L1"));
    assert!(html.contains("Ctrl + Shift + C"));
    let labels = exported_labels(&html);
    for label in ["é", "Space", "Tab", "Enter", "<", "'", "\""] {
        assert!(labels.iter().any(|value| value == label));
    }
    assert_eq!(render::html(&layout).unwrap(), html);
}

#[test]
fn combinations_outside_the_maps_keep_paired_searchable_text_instructions() {
    let mut layout = layout();
    for (character, held) in [('g', vec![]), ('G', vec![Button::R2])] {
        layout.bindings.push(Binding {
            action: Action::Character(character),
            chord: Chord {
                dpad: Dpad::Neutral,
                buttons: if character == 'G' {
                    vec![Button::Y, Button::B, Button::A]
                } else {
                    vec![Button::A, Button::B, Button::Y]
                },
            },
            held,
            held_dpad: Dpad::Neutral,
        });
    }
    let html = render::html(&layout).unwrap();
    assert!(html.contains("<strong class=\"action-label\">g / G</strong>"));
    assert!(html.contains("G: Hold R2; press Y + B + A"));
    assert!(!html.contains("<span>g / G</span>"));
    assert_eq!(exported_labels(&html).len(), layout.bindings.len());
}

#[test]
fn long_actions_keep_full_names_in_maps_and_text_reference() {
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
    assert!(html.contains("<span>Ctrl + Alt + Shift + Meta + Page Down</span>"));
    assert!(
        html.contains(
            "<strong class=\"action-label\">Ctrl + Alt + Shift + Meta + Page Down</strong>"
        )
    );
    assert!(html.contains("<span>Page Down</span>"));
    assert!(!html.contains("<svg"));
}

#[test]
fn export_escapes_layout_text_and_handles_direct_and_custom_modifier_roles() {
    let mut layout = layout();
    layout.strategy.name = "</title><script>alert(1)</script>&\"".into();
    let html = render::html(&layout).unwrap();
    assert!(!html.contains("<script>alert(1)</script>"));
    assert!(html.contains("&lt;/title&gt;&lt;script&gt;alert(1)&lt;/script&gt;&amp;&quot;"));
    let labels = exported_labels(&html);
    for label in ["<", "'", "\""] {
        assert!(labels.iter().any(|value| value == label));
    }
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
    assert!(html.contains("with L1 held"));
    assert!(!html.contains("with R2 held"));
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
        Command::new(env!("CARGO_BIN_EXE_chordpad"))
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
        let result = Command::new(env!("CARGO_BIN_EXE_chordpad"))
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
fn number_overview_includes_face_symbols_and_shifted_outputs() {
    let mut layout: Layout = serde_json::from_str(include_str!(
        "../android/app/src/main/assets/default-layout.json"
    ))
    .unwrap();
    let overview = |layout: &Layout| {
        render::html(layout)
            .unwrap()
            .split("aria-label=\"Number and symbol positions\"")
            .nth(1)
            .unwrap()
            .split("</section>")
            .next()
            .unwrap()
            .to_string()
    };
    let html = overview(&layout);
    for (output, label) in [
        ("- / _", "X / □"),
        ("= / +", "Y / △"),
        ("] / }", "A + X"),
        ("[ / {", "A + B"),
        ("\\ / |", "Y + B"),
        ("` / ~", "X + Y"),
        ("9 / (", "A / ×"),
        ("0 / )", "B / ○"),
        ("7 / &amp;", "←"),
    ] {
        assert!(html.contains(&format!("<span>{output}</span><span>{label}</span>")));
    }
    // Symbol-only layouts should still get a layer overview.
    layout.profile.function_layer = false;
    layout.profile.function_volume = false;
    layout.profile.function_print_screen = false;
    layout.bindings.retain(|b| {
        !matches!(b.action, Action::Character(c) if c.is_ascii_digit() || "!@#$%^&*()".contains(c))
    });
    assert!(overview(&layout).contains("<span>= / +</span>"));
}

#[test]
fn eight_way_maps_cover_base_shift_shoulders_and_derived_layers() {
    let mut layout: Layout = serde_json::from_str(include_str!(
        "../android/app/src/main/assets/default-layout.json"
    ))
    .unwrap();
    let section = |html: &str, label: &str| {
        html.split(&format!("aria-label=\"{label}\""))
            .nth(1)
            .unwrap()
            .split("</section>")
            .next()
            .unwrap()
            .to_string()
    };
    let html = render::html(&layout).unwrap();
    let base = section(&html, "Base and Shift positions");
    assert_eq!(base.matches("class=\"binding-map\"").count(), 4);
    for cell in [
        "<span>t / T</span><span>↑</span>",
        "<span>w / W</span><span>Y + B</span>",
        "<span>y / Y</span><span>A + X</span>",
        "<span>; / :</span><span>↖ + L1</span>",
        "<span>&#39; / &quot;</span><span>A + B + R1</span>",
        "<span>Backspace</span><span>L1 alone</span>",
        "<span>Space</span><span>R1 alone</span>",
    ] {
        assert!(base.contains(cell), "Missing base map cell: {cell}");
    }
    let navigation = section(&html, "Navigation layer");
    assert_eq!(navigation.matches("class=\"binding-map\"").count(), 2);
    assert!(navigation.contains("<span>↑</span><span>↑</span>"));
    assert!(navigation.contains("<span>Page Down</span><span>A / ×</span>"));
    assert!(navigation.contains("<span>·</span><span>↗</span>"));
    assert!(navigation.contains("<span>Delete</span><span>A + X</span>"));
    assert!(navigation.contains("<span>Play/Pause</span><span>A + B</span>"));
    let functions = section(&html, "Function layer");
    assert!(functions.contains("<span>Volume Up</span><span>X + Y</span>"));
    assert!(functions.contains("<span>F2</span><span>↗</span>"));
    assert!(functions.contains("<span>F11</span><span>X / □</span>"));
    // Imported function positions and profile restrictions must reach the maps.
    layout.profile.function_volume = false;
    layout.profile.extra_function_chords[0].buttons = vec![Button::A, Button::X];
    layout.profile.forbidden_chords.push(Chord {
        dpad: Dpad::Right,
        buttons: vec![Button::L1, Button::R2],
    });
    let html = render::html(&layout).unwrap();
    assert!(section(&html, "Function layer").contains("<span>F11</span><span>A + X</span>"));
    assert!(section(&html, "Navigation layer").contains("<span>·</span><span>→</span>"));
    assert_eq!(
        exported_labels(&html).len(),
        layout.bindings.len() + 12 + 12
    );
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
    assert_eq!(html.matches("class=\"chord-card\"").count(), 2);
    assert_eq!(html.matches("<section class=\"layer\"").count(), 2);
    assert!(html.contains("with L2 held"));
    assert!(html.contains("Hold R2"));
    assert!(html.contains("R1 is Space and L1 is Backspace in every layer"));
    assert!(html.contains("uppercase letters use the same chords as lowercase"));
    assert!(html.contains("Hold L2 + R2; press A"));
    assert!(html.contains("Thumbstick modifiers"));
    assert_eq!(html.matches("class=\"stick-cell\"").count(), 18);
    assert!(html.contains("Alt + Command"));
    assert!(html.contains("Ctrl + Command"));
    for (side, inward, inward_down) in [
        ("Left", "Right", "Down Right"),
        ("Right", "Left", "Down Left"),
    ] {
        assert!(html.contains(&format!(
            "aria-label=\"{side} stick {inward}: Ctrl + Alt + Command\""
        )));
        assert!(html.contains(&format!(
            "aria-label=\"{side} stick {inward_down}: Alt + Command\""
        )));
    }
    for (index, output) in ["a / A", "1 / !"].into_iter().enumerate() {
        let layer = html
            .split(&format!("<section class=\"layer\" data-layer=\"{index}\""))
            .nth(1)
            .unwrap()
            .split("</section>")
            .next()
            .unwrap();
        let maps = layer.split("<div class=\"chord-grid\">").next().unwrap();
        assert_eq!(maps.matches("class=\"binding-map\"").count(), 2);
        assert!(maps.contains(&format!("<span>{output}</span><span>A / ×</span>")));
    }
}
