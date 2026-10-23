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

fn compact_action(action: &Action) -> String {
    let label = match action {
        Action::Character(' ') | Action::Key(Key::Space) => "␣".into(),
        Action::Character('\n') | Action::Key(Key::Enter) => "↵".into(),
        Action::Character('\t') | Action::Key(Key::Tab) => "⇥".into(),
        Action::Key(Key::Backspace) => "⌫".into(),
        Action::Key(Key::Escape) => "Esc".into(),
        Action::Key(Key::Up) => "↑".into(),
        Action::Key(Key::Down) => "↓".into(),
        Action::Key(Key::Left) => "←".into(),
        Action::Key(Key::Right) => "→".into(),
        Action::Key(Key::Home) => "Hm".into(),
        Action::Key(Key::PageUp) => "Pg↑".into(),
        Action::Key(Key::PageDown) => "Pg↓".into(),
        Action::Key(Key::Insert) => "Ins".into(),
        Action::Key(Key::Delete) => "Del".into(),
        _ => action_name(action),
    };
    if label.chars().count() <= 3 {
        label
    } else {
        "…".into()
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

fn diagram(layout: &Layout, binding: Option<&Binding>) -> String {
    let (mask, held, dpad, held_dpad) = binding
        .map(|b| {
            (
                b.chord.input().unwrap().mask,
                button_mask(&b.held).unwrap(),
                b.chord.dpad,
                b.held_dpad,
            )
        })
        .unwrap_or((0, 0, Dpad::Neutral, Dpad::Neutral));
    let title = binding
        .map(|b| format!("{}: {}", action_name(&b.action), notation(b)))
        .unwrap_or_else(|| {
            "Base controls: A south, B east, X west, Y north; shoulders and triggers above".into()
        });
    let mut svg = format!(
        r#"<svg class="controller" viewBox="0 0 480 270" role="img" aria-label="{}" xmlns="http://www.w3.org/2000/svg"><title>{}</title><path class="shell" d="M92 75 Q79 52 54 66 L17 203 Q9 238 41 243 Q64 249 104 203 L133 179 H347 L376 203 Q416 249 439 243 Q471 238 463 203 L426 66 Q401 52 388 75 Z"/><path class="seam" d="M197 109 H283 M211 122 H269"/>"#,
        escape(&title),
        escape(&title)
    );
    let quick: BTreeMap<_, _> = layout
        .bindings
        .iter()
        .filter(|b| {
            b.context().unwrap().controls() == 0
                && b.chord.dpad == Dpad::Neutral
                && b.chord.buttons.len() == 1
        })
        .map(|b| (b.chord.buttons[0], &b.action))
        .collect();
    for (button, x, y) in [
        (Button::L2, 52, 6),
        (Button::R2, 316, 6),
        (Button::L1, 52, 41),
        (Button::R1, 316, 41),
    ] {
        let state = if held & button.bit() != 0 {
            "held"
        } else if mask & button.bit() != 0 {
            "pressed"
        } else {
            "rest"
        };
        let role = if layout.strategy.shift == Some(button) {
            " · Shift"
        } else if layout.strategy.layer == Some(button) {
            " · Layer"
        } else {
            ""
        };
        let label = if binding.is_none()
            && let Some(action) = quick.get(&button)
        {
            format!("{button:?}  {}", compact_action(action))
        } else if binding.is_none() {
            format!("{button:?}{role}")
        } else {
            format!("{button:?}")
        };
        let tooltip = if binding.is_none() {
            quick
                .get(&button)
                .map(|a| format!("<title>{}</title>", escape(&action_name(a))))
                .unwrap_or_default()
        } else {
            String::new()
        };
        write!(svg, r#"<g class="control {state}" data-button="{button:?}">{tooltip}<rect x="{x}" y="{y}" width="112" height="27" rx="9"/><text x="{}" y="{}" class="shoulder-label">{}</text></g>"#,x+56,y+18,escape(&label)).unwrap();
    }
    for (button, x, y) in [
        (Button::A, 372, 183),
        (Button::B, 418, 137),
        (Button::X, 326, 137),
        (Button::Y, 372, 91),
    ] {
        let state = if held & button.bit() != 0 {
            "held"
        } else if mask & button.bit() != 0 {
            "pressed"
        } else {
            "rest"
        };
        write!(svg, r#"<g class="control {state}" data-button="{button:?}"><circle cx="{x}" cy="{y}" r="23"/>"#).unwrap();
        if binding.is_none()
            && let Some(action) = quick.get(&button)
        {
            write!(svg,r#"<title>{}</title><text x="{x}" y="{}" class="input-label">{button:?}</text><text x="{x}" y="{}" class="output-label">{}</text>"#,escape(&action_name(action)),y-7,y+13,escape(&compact_action(action))).unwrap();
        } else {
            write!(
                svg,
                r#"<text x="{x}" y="{}" class="face-label">{button:?}</text>"#,
                y + 7
            )
            .unwrap();
        }
        svg.push_str("</g>");
    }
    for (button, x) in [(MenuButton::Select, 182), (MenuButton::Start, 248)] {
        let action = action_name(&button.action());
        write!(svg, r#"<g class="control rest" data-menu-button="{button:?}"><title>{button:?}: {action}</title><rect x="{x}" y="138" width="50" height="24" rx="8"/><text x="{}" y="154" class="shoulder-label">{button:?}</text><text x="{}" y="178" class="shoulder-label">{action}</text></g>"#, x+25, x+25).unwrap();
    }
    let (dx, dy) = dpad.position();
    for (dir, x, y, active) in [
        (Dpad::Up, 91, 85, dy < 0),
        (Dpad::Down, 91, 153, dy > 0),
        (Dpad::Left, 57, 119, dx < 0),
        (Dpad::Right, 125, 119, dx > 0),
    ] {
        let state = if held_dpad.bits() & dir.bits() != 0 {
            "held"
        } else if active {
            "pressed"
        } else {
            "rest"
        };
        write!(svg,r#"<g class="control {state}" data-direction="{}"><rect x="{x}" y="{y}" width="34" height="34" rx="6"/>"#,direction(dir).1).unwrap();
        let output = if binding.is_none() {
            layout
                .bindings
                .iter()
                .find(|b| {
                    b.context().unwrap().controls() == 0
                        && b.chord.buttons.is_empty()
                        && b.chord.dpad == dir
                })
                .map(|b| &b.action)
        } else {
            None
        };
        let label = output
            .map(compact_action)
            .unwrap_or_else(|| direction(dir).0.into());
        let compact = if label.chars().count() > 2 {
            " compact"
        } else {
            ""
        };
        if let Some(action) = output {
            write!(svg, "<title>{}</title>", escape(&action_name(action))).unwrap();
        }
        write!(
            svg,
            r#"<text x="{}" y="{}" class="dpad-label{compact}">{}</text></g>"#,
            x + 17,
            y + 24,
            escape(&label)
        )
        .unwrap();
    }
    svg.push_str(r#"<rect class="dpad-center" x="94" y="122" width="28" height="28" rx="4"/><circle class="center-mark" cx="240" cy="154" r="4"/></svg>"#);
    svg
}

fn number_reference(layout: &Layout) -> String {
    let mut out = String::from(
        "<section class=\"stick-guide\" aria-label=\"Number positions\"><h2>R2: numbers</h2><p>Hold R2. Add L2 for shifted symbols. The diagrams show the number bindings in this layout.</p><div class=\"stick-maps\">",
    );
    for face in [false, true] {
        write!(
            out,
            "<div><h3>{}</h3><div class=\"stick-map\">",
            if face { "Face buttons" } else { "D-pad" }
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
                mask,
                dpad: if face { Dpad::Neutral } else { dpad },
            };
            let number = layout
                .bindings
                .iter()
                .find_map(|b| match b.action {
                    Action::Character(c)
                        if c.is_ascii_digit()
                            && b.held == [Button::R2]
                            && b.chord.input().ok() == Some(input) =>
                    {
                        Some(c.to_string())
                    }
                    _ => None,
                })
                .unwrap_or_else(|| "·".into());
            write!(
                out,
                "<div class=\"stick-cell number-cell\"><span>{number}</span><span>{}</span></div>",
                escape(&label)
            )
            .unwrap();
        }
        out.push_str("</div></div>");
    }
    out.push_str("</div></section>");
    out
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

pub fn html(layout: &Layout) -> Result<String> {
    layout.validate()?;
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
    out.push_str("</style></head><body><header class=\"topbar wrap\"><a class=\"brand\" href=\"#top\"><span class=\"brand-icon\" aria-hidden=\"true\">✚</span>padtype<span class=\"brand-note\">layout reference</span></a><button class=\"print-button js-only\" id=\"print\" type=\"button\">Print / Save PDF <span aria-hidden=\"true\">↗</span></button></header><main id=\"top\" class=\"wrap\"><div class=\"intro\"><div><p class=\"eyebrow\">CONTROLLER → KEYBOARD</p><h1>Your typing layout<span>.</span></h1><p class=\"lede\">Look up any character and see how to press it.</p></div><div class=\"layout-meta\">");
    write!(
        out,
        "<strong>{} bindings</strong><span>{}</span></div></div>",
        layout.bindings.len(),
        escape(&layout.strategy.name)
    )
    .unwrap();
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
        write!(out,"<p class=\"mode-note\">Each half releases independently. {editing}. Hold L2 for Shift: uppercase letters use the same chords as lowercase. Hold R2 for numbers and symbols across consecutive strokes; add L2 for shifted symbols. Both triggers work with either half. Hold modifiers before starting a stroke. Overlapping strokes type in start order; simultaneous starts type left first.</p>").unwrap();
    }
    out.push_str("<p class=\"mode-note\">Start sends Enter and Select sends Escape on release in every layer. These fixed controls also accept held Shift and thumbstick modifiers.</p>");
    out.push_str("<section class=\"overview\" aria-label=\"Controller overview\"><div class=\"preview\"><div class=\"preview-heading\"><p class=\"eyebrow\" id=\"preview-title\">BASE CONTROLS</p><button class=\"text-button js-only\" id=\"reset-preview\" type=\"button\">Reset overview</button></div><div id=\"preview-diagram\">");
    out.push_str(&diagram(layout, None));
    out.push_str("</div><p id=\"preview-instruction\" class=\"preview-caption\">Single-control bindings are shown here. … marks a longer action name; see its card below.</p></div><div class=\"guide\"><h2>Read the controller</h2><div class=\"legend\"><span><i class=\"press-swatch\"></i>Press for this chord</span><span><i class=\"hold-swatch\"></i>Hold as a modifier</span></div><ol><li><strong>Hold</strong> the modifier controls shown, if any.</li><li><strong>Press</strong> the highlighted buttons and D-pad directions together.</li><li><strong>Release</strong> ordinary controls to type. Held modifiers can stay down.</li></ol><p class=\"guide-note\">D-pad diagonals highlight two directions. A + B means press both buttons.</p></div></section>");
    if layout.strategy.independent {
        out.push_str(&thumbstick_reference());
        if layout
            .bindings
            .iter()
            .any(|b| matches!(b.action,Action::Character(c) if c.is_ascii_digit()))
        {
            out.push_str(&number_reference(layout));
        }
    }
    if layout.profile.navigation_layer {
        out.push_str("<section class=\"stick-guide\" aria-label=\"Navigation layer\"><h2>R2 + L1: navigation</h2><p>Hold R2 and L1. Use the D-pad for arrows, Y / Triangle for Page Up, A / Cross for Page Down, X / Square for Home, and B / Circle for End. Navigation keys fire on press and repeat after 400 ms, every 80 ms. Add L2 to select text; thumbstick modifiers also apply. D-pad diagonals produce no arrows. Release either layer key to stop. Held inputs must release before typing again. R2 alone keeps the numbers/symbols layer; L1 is consumed as the navigation modifier.</p><div class=\"chord-grid navigation-grid\">");
        for (dpad, buttons, key) in [
            (Dpad::Up, vec![], Key::Up),
            (Dpad::Down, vec![], Key::Down),
            (Dpad::Left, vec![], Key::Left),
            (Dpad::Right, vec![], Key::Right),
            (Dpad::Neutral, vec![Button::Y], Key::PageUp),
            (Dpad::Neutral, vec![Button::A], Key::PageDown),
            (Dpad::Neutral, vec![Button::X], Key::Home),
            (Dpad::Neutral, vec![Button::B], Key::End),
        ] {
            let binding = Binding {
                chord: Chord { dpad, buttons },
                held: vec![Button::L1, Button::R2],
                held_dpad: Dpad::Neutral,
                action: Action::Key(key),
            };
            write!(
                out,
                "<article class=\"navigation-card\"><strong>{}</strong>{}<p>{}</p></article>",
                if matches!(key, Key::Up | Key::Down | Key::Left | Key::Right) {
                    compact_action(&binding.action)
                } else {
                    action_name(&binding.action)
                },
                diagram(layout, Some(&binding)),
                escape(&notation(&binding))
            )
            .unwrap();
        }
        out.push_str("</div></section>");
    }
    if layout.profile.function_layer {
        out.push_str("<section class=\"stick-guide\" aria-label=\"Function layer\"><h2>R2 + R1: function keys</h2><p>Hold R2 and R1. Use the number combos: 1–9 send F1–F9, and 0 sends F10. F11 and F12 use the separate combos shown below. Release the entire ordinary chord to send one function key. Keep the prefix held for sequential inputs. Add L2 for Shift or use thumbstick modifiers. R1 is consumed as the layer key.</p><div class=\"chord-grid navigation-grid\">");
        let mut bindings = layout.function_bindings();
        bindings.sort_by(|a, b| a.action.cmp(&b.action));
        for binding in bindings {
            write!(
                out,
                "<article class=\"function-card\"><strong>{}</strong>{}<p>{}</p></article>",
                action_name(&binding.action),
                diagram(layout, Some(&binding)),
                escape(&notation(&binding))
            )
            .unwrap();
        }
        out.push_str("</div></section>");
    }
    out.push_str("<div class=\"browser-tools js-only\"><div class=\"search-field\"><label for=\"search\">Find a character or action</label><div class=\"search-row\"><span aria-hidden=\"true\">⌕</span><input id=\"search\" type=\"search\" placeholder=\"Try a, A, ?, Space, or Backspace\" autocomplete=\"off\" spellcheck=\"false\"><button id=\"clear-search\" class=\"text-button\" type=\"button\">Clear</button></div></div><p id=\"result-count\" role=\"status\" aria-live=\"polite\"></p></div><nav class=\"layer-nav js-only\" aria-label=\"Filter by held controls\"><button type=\"button\" data-filter=\"all\" aria-pressed=\"true\">All bindings</button>");
    let mut contexts = vec![Input::default()];
    if layout.strategy.independent {
        contexts.extend(
            layout
                .bindings
                .iter()
                .map(|b| b.context().unwrap())
                .filter(|c| c.controls() != 0)
                .collect::<BTreeSet<_>>(),
        );
    } else {
        for held in [
            layout.strategy.shift.map(Button::bit),
            layout.strategy.layer.map(Button::bit),
            Some(layout.strategy.modifier_mask()),
        ]
        .into_iter()
        .flatten()
        {
            let context = Input {
                mask: held,
                dpad: Dpad::Neutral,
            };
            if !contexts.contains(&context)
                && layout
                    .bindings
                    .iter()
                    .any(|b| b.context().unwrap() == context)
            {
                contexts.push(context);
            }
        }
    }
    for (index, held) in contexts.iter().enumerate() {
        write!(
            out,
            r#"<button type="button" data-filter="{index}" aria-pressed="false">{}</button>"#,
            escape(&context_name(layout, *held))
        )
        .unwrap();
    }
    out.push_str("</nav><p id=\"no-results\" class=\"no-results\" hidden>No matching bindings. Try another character or choose All bindings.</p>");
    for (index, held) in contexts.into_iter().enumerate() {
        let mut bindings: Vec<_> = layout
            .bindings
            .iter()
            .filter(|b| b.context().unwrap() == held)
            .collect();
        bindings.sort_by(|a, b| {
            category(&a.action)
                .cmp(&category(&b.action))
                .then(a.action.cmp(&b.action))
        });
        write!(out,r#"<section class="layer" data-layer="{index}" aria-labelledby="layer-{index}"><div class="layer-heading"><div><p class="eyebrow">{}</p><h2 id="layer-{index}">{}</h2></div><span class="binding-count">{} bindings</span><p class="print-legend">Solid controls: press · Dashed controls: hold · Release ordinary controls to type</p></div><div class="chord-grid">"#,if held.controls() == 0 { "NO MODIFIERS".into() } else { escape(&format!("HOLD {}",controls(held))) },escape(&context_name(layout,held)),bindings.len()).unwrap();
        let empty = bindings.is_empty();
        for binding in bindings {
            let label = action_name(&binding.action);
            let notation = notation(binding);
            let character = match binding.action {
                Action::Character(c) if !c.is_control() => c.to_string(),
                _ => String::new(),
            };
            let word = !matches!(binding.action, Action::Character(c) if !c.is_whitespace() && !c.is_control());
            write!(out,r#"<button type="button" class="chord-card" data-character="{}" data-label="{}" data-search="{}" aria-pressed="false" aria-label="{}"><span class="action-label{}" dir="auto">{}</span>{}<span class="chord-notation">{}</span></button>"#,
                escape(&character),escape(&label),escape(&format!("{label} {notation}").to_lowercase()),escape(&format!("{label}: {notation}")),if word { " word" } else { "" },escape(&label),diagram(layout,Some(binding)),escape(&notation)).unwrap();
        }
        if empty {
            out.push_str("<p class=\"empty-layer\">No bindings in this state.</p>");
        }
        out.push_str("</div></section>");
    }
    out.push_str("<noscript><p>All bindings are shown. Enable JavaScript for character search and chord previews.</p></noscript></main><footer class=\"wrap\"><span>padtype / visual layout reference</span><span>Press together · release to type</span></footer><script>");
    out.push_str(include_str!("render/script.js"));
    out.push_str("</script></body></html>\n");
    Ok(out)
}
