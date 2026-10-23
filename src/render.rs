use crate::{Result, model::*};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
};

fn escape(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '&' => "&amp;".into(),
            '<' => "&lt;".into(),
            '>' => "&gt;".into(),
            '"' => "&quot;".into(),
            '\'' => "&#39;".into(),
            '\0' => "�".into(),
            _ => c.to_string(),
        })
        .collect()
}

fn key_name(key: Key) -> String {
    match key {
        Key::PageUp => "Page Up".into(),
        Key::PageDown => "Page Down".into(),
        Key::MediaPlayPause => "Play/Pause".into(),
        Key::MediaNext => "Next".into(),
        Key::MediaPrevious => "Previous".into(),
        Key::MediaStop => "Stop".into(),
        Key::VolumeUp => "Volume Up".into(),
        Key::VolumeDown => "Volume Down".into(),
        Key::Mute => "Mute".into(),
        _ => format!("{key:?}"),
    }
}

fn action_name(action: &Action) -> String {
    match action {
        Action::Character(' ') => "Space".into(),
        Action::Character('\n') => "Enter".into(),
        Action::Character('\t') => "Tab".into(),
        Action::Character(c) if c.is_control() => format!("U+{:04X}", *c as u32),
        Action::Character(c) => c.to_string(),
        Action::Key(key) => key_name(*key),
        Action::Shortcut(s) => s
            .modifiers
            .iter()
            .map(|m| format!("{m:?}"))
            .chain(std::iter::once(key_name(s.key)))
            .collect::<Vec<_>>()
            .join(" + "),
    }
}

fn map_action(action: &Action) -> String {
    match action {
        Action::Key(Key::Up) => "↑".into(),
        Action::Key(Key::Down) => "↓".into(),
        Action::Key(Key::Left) => "←".into(),
        Action::Key(Key::Right) => "→".into(),
        _ => action_name(action),
    }
}

fn direction(dpad: Dpad) -> (&'static str, &'static str) {
    match dpad {
        Dpad::Neutral => ("", "neutral"),
        Dpad::Up => ("↑", "Up"),
        Dpad::Down => ("↓", "Down"),
        Dpad::Left => ("←", "Left"),
        Dpad::Right => ("→", "Right"),
        Dpad::UpLeft => ("↖", "Up Left"),
        Dpad::UpRight => ("↗", "Up Right"),
        Dpad::DownLeft => ("↙", "Down Left"),
        Dpad::DownRight => ("↘", "Down Right"),
    }
}

fn buttons(mask: u8) -> String {
    Button::ALL
        .into_iter()
        .filter(|b| mask & b.bit() != 0)
        .map(|b| format!("{b:?}"))
        .collect::<Vec<_>>()
        .join(" + ")
}

fn controls(input: Input) -> String {
    let mut names = vec![];
    if input.mask != 0 {
        names.push(buttons(input.mask));
    }
    if input.dpad != Dpad::Neutral {
        names.push(format!("D-pad {}", direction(input.dpad).1));
    }
    names.join(" + ")
}

fn notation(binding: &Binding) -> String {
    let mut parts = vec![];
    if binding.chord.dpad != Dpad::Neutral {
        parts.push(format!("D-pad {}", direction(binding.chord.dpad).1));
    }
    parts.extend(binding.chord.buttons.iter().map(|b| format!("{b:?}")));
    let press = format!("Press {}", parts.join(" + "));
    if binding.context().unwrap().controls() == 0 {
        press
    } else {
        format!(
            "Hold {}; press {}",
            controls(binding.context().unwrap()),
            parts.join(" + ")
        )
    }
}

fn context_name(layout: &Layout, context: Input) -> String {
    let held = context.mask;
    if context.controls() == 0 {
        return "Base".into();
    }
    let mut names = vec![];
    if let Some(b) = layout.strategy.shift
        && held & b.bit() != 0
    {
        names.push(format!("{b:?} · Shift"));
    }
    if let Some(b) = layout.strategy.layer
        && held & b.bit() != 0
    {
        names.push(format!("{b:?} · Layer"));
    }
    names.join(" + ")
}

fn eight_way_maps(bindings: &[Binding], contexts: &[u8]) -> String {
    let mut out = String::from("<div class=\"stick-maps binding-maps\">");
    for (face, shoulder) in [
        (false, 0),
        (true, 0),
        (false, Button::L1.bit()),
        (true, Button::R1.bit()),
    ] {
        if shoulder != 0
            && !bindings.iter().any(|b| {
                contexts.contains(&b.context().unwrap().mask)
                    && b.chord.input().unwrap().mask & shoulder != 0
            })
        {
            continue;
        }
        let label = format!(
            "{}{}",
            if face { "Face buttons" } else { "D-pad" },
            if shoulder == 0 {
                String::new()
            } else {
                format!(" + {}", buttons(shoulder))
            }
        );
        write!(
            out,
            "<div class=\"binding-map\" aria-label=\"{}\"><h3>{}</h3><div class=\"stick-map\">",
            escape(&label),
            escape(&label)
        )
        .unwrap();
        for dpad in [
            Dpad::UpLeft,
            Dpad::Up,
            Dpad::UpRight,
            Dpad::Left,
            Dpad::Neutral,
            Dpad::Right,
            Dpad::DownLeft,
            Dpad::Down,
            Dpad::DownRight,
        ] {
            let (mask, label) = if face {
                match dpad {
                    Dpad::UpLeft => (Button::X.bit() | Button::Y.bit(), "X + Y".into()),
                    Dpad::Up => (Button::Y.bit(), "Y / △".into()),
                    Dpad::UpRight => (Button::Y.bit() | Button::B.bit(), "Y + B".into()),
                    Dpad::Left => (Button::X.bit(), "X / □".into()),
                    Dpad::Right => (Button::B.bit(), "B / ○".into()),
                    Dpad::DownLeft => (Button::A.bit() | Button::X.bit(), "A + X".into()),
                    Dpad::Down => (Button::A.bit(), "A / ×".into()),
                    Dpad::DownRight => (Button::A.bit() | Button::B.bit(), "A + B".into()),
                    Dpad::Neutral => (0, "".into()),
                }
            } else {
                (0, direction(dpad).0.to_string())
            };
            let input = Input {
                mask: mask | shoulder,
                dpad: if face { Dpad::Neutral } else { dpad },
            };
            let label = if shoulder == 0 {
                label
            } else if dpad == Dpad::Neutral {
                format!("{} alone", buttons(shoulder))
            } else {
                format!("{label} + {}", buttons(shoulder))
            };
            let outputs: Vec<_> = contexts
                .iter()
                .map(|held| {
                    bindings
                        .iter()
                        .find(|b| {
                            b.context().unwrap().mask == *held
                                && b.chord.input().ok() == Some(input)
                        })
                        .map(|b| map_action(&b.action))
                        .unwrap_or_else(|| "·".into())
                })
                .collect();
            let output = if outputs.iter().all(|value| value == &outputs[0]) {
                outputs[0].clone()
            } else {
                outputs.join(" / ")
            };
            write!(
                out,
                "<div class=\"stick-cell binding-cell\"><span>{}</span><span>{}</span></div>",
                escape(&output),
                escape(&label)
            )
            .unwrap();
        }
        out.push_str("</div></div>");
    }
    out.push_str("</div>");
    out
}

fn typing_map_bindings(source: &[Binding], profile: &Profile) -> Vec<Binding> {
    let mut bindings = source.to_vec();
    for binding in source {
        if binding.held.is_empty()
            && binding.chord.dpad == Dpad::Neutral
            && (binding.chord.buttons == [Button::L1] || binding.chord.buttons == [Button::R1])
        {
            let mut shifted = binding.clone();
            shifted.held = vec![Button::L2];
            if profile.allows(
                shifted
                    .chord
                    .input()
                    .unwrap()
                    .with_context(shifted.context().unwrap()),
            ) {
                bindings.push(shifted);
            }
        }
    }
    bindings
}

fn thumbstick_reference() -> String {
    let mut out = String::from(
        "<section class=\"stick-guide\" aria-labelledby=\"stick-title\"><h2 id=\"stick-title\">Thumbstick modifiers</h2><p>Hold either stick while typing. Command sends Meta / Super on Linux. Add L2 for Shift or R2 for numbers and symbols.</p><div class=\"stick-maps\">",
    );
    for side in Side::ALL {
        write!(out, "<div><h3>{side:?} stick</h3><div class=\"stick-map\">").unwrap();
        for dpad in [
            Dpad::UpLeft,
            Dpad::Up,
            Dpad::UpRight,
            Dpad::Left,
            Dpad::Neutral,
            Dpad::Right,
            Dpad::DownLeft,
            Dpad::Down,
            Dpad::DownRight,
        ] {
            let modifiers = crate::controller::thumbstick_modifiers(side, dpad);
            let label = if modifiers.is_empty() {
                "Neutral".into()
            } else {
                modifiers
                    .iter()
                    .map(|m| {
                        if *m == Modifier::Meta {
                            "Command".into()
                        } else {
                            format!("{m:?}")
                        }
                    })
                    .collect::<Vec<String>>()
                    .join(" + ")
            };
            write!(out, "<div class=\"stick-cell\" role=\"img\" aria-label=\"{side:?} stick {}: {}\"><span aria-hidden=\"true\">{}</span><span>{}</span></div>", direction(dpad).1, escape(&label), if dpad == Dpad::Neutral { "●" } else { direction(dpad).0 }, escape(&label)).unwrap();
        }
        out.push_str("</div></div>");
    }
    out.push_str("</div></section>");
    out
}

fn category(action: &Action) -> u8 {
    match action {
        Action::Character(c) if c.is_alphabetic() => 0,
        Action::Character(c) if c.is_ascii_digit() => 1,
        Action::Character(c) if !c.is_whitespace() && !c.is_control() => 2,
        Action::Character(_) | Action::Key(_) => 3,
        Action::Shortcut(_) => 4,
    }
}

struct ReferenceLayer {
    title: String,
    label: String,
    note: String,
    contexts: Vec<u8>,
    bindings: Vec<Binding>,
}

fn reference_layers(layout: &Layout) -> Vec<ReferenceLayer> {
    let shift = layout.strategy.shift.map_or(0, Button::bit);
    let mut contexts = BTreeSet::from([0]);
    contexts.extend(
        layout
            .bindings
            .iter()
            .map(|b| b.context().unwrap().mask & !shift),
    );
    let mut layers = vec![];
    for held in contexts {
        let context = Input {
            mask: held,
            dpad: Dpad::Neutral,
        };
        let (title, label) = if layout.strategy.independent {
            if held == 0 {
                ("Base + L2: Shift".into(), "Base and Shift positions".into())
            } else {
                (
                    "R2: numbers and symbols".into(),
                    "Number and symbol positions".into(),
                )
            }
        } else {
            let name = context_name(layout, context);
            (name.clone(), name)
        };
        let contexts = if shift == 0 {
            vec![held]
        } else {
            vec![held, held | shift]
        };
        let prefix = if held == 0 {
            String::new()
        } else {
            format!("Hold {}. ", controls(context))
        };
        let paired = layout.strategy.shift.map_or(String::new(), |button| format!("Pairs show the normal output, then the output with {button:?} held. A single label applies to both. "));
        layers.push(ReferenceLayer {
            title, label,
            note: format!("{prefix}{paired}Press the controls shown together. In shoulder maps, the center is the shoulder alone. · marks an unassigned position."),
            bindings: layout.bindings.iter().filter(|b| contexts.contains(&b.context().unwrap().mask)).cloned().collect(),
            contexts,
        });
    }
    if layout.profile.navigation_layer {
        let bindings = layout.navigation_bindings();
        let face_note = if layout.profile.navigation_media {
            " Face-button taps send on full chord release; holding a single face button starts repeating at 400 ms. Adjacent face pairs send media controls once on release and never repeat."
        } else {
            ""
        };
        layers.push(ReferenceLayer {
            title: "R2 + L1: navigation".into(), label: "Navigation layer".into(),
            note: format!("Hold R2 and L1. D-pad arrows fire immediately and repeat after 400 ms, every 80 ms.{face_note} D-pad diagonals produce no arrows. Add L2 to select text; thumbstick modifiers also apply. Release either layer key to stop. Held inputs must release before typing again."),
            contexts: vec![Button::L1.bit() | Button::R2.bit()], bindings,
        });
    }
    if layout.profile.function_layer {
        let volume_note = if layout.profile.function_volume {
            " Adjacent face pairs add Volume Up, Volume Down, and Mute, once per chord. Media and volume controls ignore keyboard modifiers."
        } else {
            ""
        };
        layers.push(ReferenceLayer {
            title: "R2 + R1: function keys".into(), label: "Function layer".into(),
            note: format!("Hold R2 and R1. Number combos 1–9 send F1–F9; 0 sends F10. F11 and F12 use the separate combos shown here. Release the entire ordinary chord to send one key. Keep the prefix held for sequential inputs. Add L2 for Shift or use thumbstick modifiers.{volume_note}"),
            contexts: vec![Button::R1.bit() | Button::R2.bit()], bindings: layout.function_bindings(),
        });
    }
    layers
}

fn text_reference(bindings: &[Binding], map_bindings: &[Binding], contexts: &[u8]) -> String {
    let mut groups = BTreeMap::<Input, Vec<&Binding>>::new();
    for binding in bindings {
        groups
            .entry(binding.chord.input().unwrap())
            .or_default()
            .push(binding);
    }
    let mut groups: Vec<_> = groups.into_values().collect();
    for bindings in &mut groups {
        bindings.sort_by_key(|binding| binding.context().unwrap().mask);
    }
    groups.sort_by_key(|bindings| (category(&bindings[0].action), bindings[0].action.clone()));
    let mut out = String::from(
        "<details class=\"binding-details\"><summary>Text chord reference</summary><div class=\"chord-grid\">",
    );
    for bindings in groups {
        let outputs: Vec<_> = contexts
            .iter()
            .map(|held| {
                map_bindings
                    .iter()
                    .find(|b| {
                        b.context().unwrap().mask == *held
                            && b.chord.input().unwrap() == bindings[0].chord.input().unwrap()
                    })
                    .map(|b| action_name(&b.action))
                    .unwrap_or_else(|| "·".into())
            })
            .collect();
        let output = if outputs.iter().all(|value| value == &outputs[0]) {
            outputs[0].clone()
        } else {
            outputs.join(" / ")
        };
        let characters: Vec<_> = bindings
            .iter()
            .filter_map(|b| {
                if let Action::Character(c) = b.action {
                    Some(c.to_string())
                } else {
                    None
                }
            })
            .collect();
        let labels: Vec<_> = bindings.iter().map(|b| action_name(&b.action)).collect();
        let search = bindings
            .iter()
            .map(|b| format!("{} {}", action_name(&b.action), notation(b)))
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        let instructions = bindings
            .iter()
            .map(|b| format!("{}: {}", action_name(&b.action), notation(b)))
            .collect::<Vec<_>>()
            .join("; ");
        write!(out, "<article class=\"chord-card\" data-characters=\"{}\" data-labels=\"{}\" data-search=\"{}\"><strong class=\"action-label\">{}</strong><span class=\"chord-notation\">{}</span></article>", escape(&serde_json::to_string(&characters).unwrap()), escape(&serde_json::to_string(&labels).unwrap()), escape(&search), escape(&output), escape(&instructions)).unwrap();
    }
    out.push_str("</div></details>");
    out
}

pub fn html(layout: &Layout) -> Result<String> {
    layout.validate()?;
    let layers = reference_layers(layout);
    let mut out = String::from(
        "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">",
    );
    write!(
        out,
        "<title>padtype · {}</title><style>",
        escape(&layout.strategy.name)
    )
    .unwrap();
    out.push_str(include_str!("render/style.css"));
    out.push_str("</style></head><body><header class=\"topbar wrap\"><a class=\"brand\" href=\"#top\"><span class=\"brand-icon\" aria-hidden=\"true\">✚</span>padtype<span class=\"brand-note\">layout reference</span></a><button class=\"print-button js-only\" id=\"print\" type=\"button\">Print / Save PDF</button></header><main id=\"top\" class=\"wrap\"><div class=\"intro\"><div><p class=\"eyebrow\">CONTROLLER → KEYBOARD</p><h1>Your typing layout<span>.</span></h1><p class=\"lede\">Find a character and read its chord on the map.</p></div><div class=\"layout-meta\">");
    write!(
        out,
        "<strong>{} bindings</strong><span>{}</span></div></div>",
        layout.bindings.len(),
        escape(&layout.strategy.name)
    )
    .unwrap();
    out.push_str("<p class=\"mode-note\">A / Cross is south, B / Circle east, X / Square west, and Y / Triangle north. Diagonal cells mean press both adjacent controls. Hold modifiers before starting a chord; release ordinary controls to type. Start sends Enter and Select sends Escape in every layer.</p>");
    if layout.strategy.independent {
        let editing = match (
            layout.profile.navigation_layer,
            layout.profile.function_layer,
        ) {
            (false, false) => "R1 is Space and L1 is Backspace in every layer",
            (true, false) => "R1 is Space; L1 is Backspace outside navigation",
            (false, true) => "R1 is Space outside functions; L1 is Backspace",
            (true, true) => "R1 is Space outside functions; L1 is Backspace outside navigation",
        };
        write!(out, "<p class=\"mode-note\">{editing}. Each half releases independently; overlapping strokes type in start order. Hold L2 for Shift: uppercase letters use the same chords as lowercase. Hold R2 for numbers and symbols across consecutive strokes; add L2 for shifted symbols.</p>").unwrap();
    }
    out.push_str("<div class=\"browser-tools js-only\"><div class=\"search-field\"><label for=\"search\">Find a character or action</label><div class=\"search-row\"><input id=\"search\" type=\"search\" placeholder=\"Try m, M, &lt;, Space, or F11\" autocomplete=\"off\" spellcheck=\"false\"><button id=\"clear-search\" class=\"text-button\" type=\"button\">Clear</button></div></div><p id=\"result-count\" role=\"status\" aria-live=\"polite\"></p></div><nav class=\"layer-nav js-only\" aria-label=\"Filter by layer\"><button type=\"button\" data-filter=\"all\" aria-pressed=\"true\">All layers</button>");
    for (index, layer) in layers.iter().enumerate() {
        write!(
            out,
            "<button type=\"button\" data-filter=\"{index}\" aria-pressed=\"false\">{}</button>",
            escape(&layer.title)
        )
        .unwrap();
    }
    out.push_str("</nav><p id=\"no-results\" class=\"no-results\" hidden>No matching bindings. Try another character or choose All layers.</p>");
    for (index, layer) in layers.iter().enumerate() {
        write!(out, "<section class=\"layer\" data-layer=\"{index}\" aria-label=\"{}\" aria-labelledby=\"layer-{index}\"><div class=\"layer-heading\"><h2 id=\"layer-{index}\">{}</h2><span class=\"binding-count\">{} bindings</span></div><p class=\"map-note\">{}</p>", escape(&layer.label), escape(&layer.title), layer.bindings.len(), escape(&layer.note)).unwrap();
        let bindings = if layout.strategy.independent {
            typing_map_bindings(&layer.bindings, &layout.profile)
        } else {
            layer.bindings.clone()
        };
        out.push_str(&eight_way_maps(&bindings, &layer.contexts));
        out.push_str(&text_reference(&layer.bindings, &bindings, &layer.contexts));
        if layer.bindings.is_empty() {
            out.push_str("<p class=\"empty-layer\">No bindings in this state.</p>");
        }
        out.push_str("</section>");
    }
    if layout.strategy.independent {
        out.push_str(&thumbstick_reference());
    }
    out.push_str("<noscript><p>All layers and maps are shown. Open a text chord reference for combinations outside the maps. Enable JavaScript to search.</p></noscript></main><footer class=\"wrap\"><span>padtype / visual layout reference</span><span>Press together · release to type</span></footer><script>");
    out.push_str(include_str!("render/script.js"));
    out.push_str("</script></body></html>\n");
    Ok(out)
}
