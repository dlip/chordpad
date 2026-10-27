//! Runtime for the default single-button, four-bank layout.
use crate::{
    Result,
    chord::{Event, Phase},
    controller::Snapshot,
    model::{Action, Button, Chord, Dpad, Key, MenuButton, Modifier, button_mask},
};
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, time::Instant};

const CONTROLS: [&str; 8] = ["↑", "→", "↓", "←", "Y / △", "B / ○", "A / ×", "X / □"];
const SELECTORS: [&[Button]; 6] = [
    &[],
    &[Button::L1],
    &[Button::R1],
    &[Button::L1, Button::R1],
    &[Button::R2],
    &[Button::R2, Button::L1],
];

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Layout {
    format: String,
    recognition: Recognition,
    banks: Vec<Bank>,
    function_banks: Vec<Bank>,
    navigation_banks: Vec<Bank>,
    held_shift_buttons: Vec<Button>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Recognition {
    late_bumper_window_ms: u64,
    #[serde(alias = "function_double_tap_ms")]
    trigger_tap_max_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Bank {
    held: Vec<Button>,
    bindings: Vec<Binding>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Binding {
    control: String,
    character: Option<char>,
    key: Option<Key>,
}

impl Binding {
    fn action(&self) -> Result<Action> {
        match (self.character, self.key) {
            (Some(c), None) => Ok(Action::Character(c)),
            (None, Some(key)) => Ok(Action::Key(key)),
            _ => Err("each alpha-bank binding needs exactly one character or key".into()),
        }
    }
}

impl Layout {
    pub fn validate(&self) -> Result<()> {
        if self.format != "chordpad-alpha-bank-candidate-v2" {
            return Err("unsupported alpha-bank layout format".into());
        }
        if !(1..=1000).contains(&self.recognition.late_bumper_window_ms)
            || !(1..=1000).contains(&self.recognition.trigger_tap_max_ms)
        {
            return Err("alpha-bank timing windows must be between 1 and 1000 ms".into());
        }
        if self.held_shift_buttons != [Button::L2] {
            return Err("alpha-bank held Shift must use L2".into());
        }
        for (banks, selectors, function) in [
            (&self.banks, SELECTORS.to_vec(), false),
            (&self.function_banks, vec![SELECTORS[4], SELECTORS[5]], true),
            (
                &self.navigation_banks,
                vec![
                    &[Button::R1, Button::R2][..],
                    &[Button::R1, Button::R2, Button::L1][..],
                ],
                false,
            ),
        ] {
            if banks.len() != selectors.len() {
                return Err("incorrect number of alpha-bank layers".into());
            }
            for (index, (bank, selector)) in banks.iter().zip(selectors).enumerate() {
                if button_mask(&bank.held)? != button_mask(selector)? {
                    return Err("unexpected alpha-bank selector or layer order".into());
                }
                let controls = if function && index == 1 {
                    &CONTROLS[..4]
                } else {
                    &CONTROLS[..]
                };
                if bank.bindings.len() != controls.len()
                    || controls
                        .iter()
                        .any(|c| bank.bindings.iter().filter(|b| b.control == *c).count() != 1)
                {
                    return Err(
                        "each bank must bind its thumb controls exactly once; function + L1 uses only the D-pad".into(),
                    );
                }
                for binding in &bank.bindings {
                    binding.action()?.validate()?;
                }
            }
        }
        Ok(())
    }

    pub fn actions(&self) -> Result<Vec<Action>> {
        self.banks
            .iter()
            .chain(&self.function_banks)
            .chain(&self.navigation_banks)
            .flat_map(|b| &b.bindings)
            .map(Binding::action)
            .collect()
    }

    pub fn validate_pause(&self, buttons: &[Button]) -> Result<u8> {
        let mask = button_mask(buttons)?;
        // A stroke can use every shoulder and one thumb control. Reserve a multi-face gesture.
        let faces = mask & (Button::A.bit() | Button::B.bit() | Button::X.bit() | Button::Y.bit());
        if faces.count_ones() < 2 {
            return Err(
                "alpha-bank pause gesture conflicts with typing; use --pause-buttons A,B,X,Y"
                    .into(),
            );
        }
        Ok(mask)
    }

    fn action(&self, mode: Mode, bank: usize, control: usize) -> Option<Action> {
        let bank = match mode {
            Mode::Alpha => &self.banks[bank],
            Mode::Number => &self.banks[4 + bank],
            Mode::Navigation => &self.navigation_banks[bank],
            Mode::Function => &self.function_banks[bank],
        };
        bank.bindings
            .iter()
            .find(|b| b.control == CONTROLS[control])
            .map(|binding| binding.action().unwrap())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Alpha,
    Number,
    Navigation,
    Function,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Control {
    Thumb(usize),
    Bumper(Button),
    Menu(MenuButton),
}

struct Stroke {
    control: Control,
    mode: Mode,
    bank: usize,
    started: u64,
    modifiers: Vec<Modifier>,
    released: bool,
    frozen: bool,
    consumed: bool,
    repeat_at: u64,
    repeat_eligible: bool,
}

pub struct Engine {
    layout: Layout,
    waiting: bool,
    previous: Snapshot,
    queue: VecDeque<Stroke>,
    repeating: Vec<Stroke>,
    one_shot_shift: bool,
    shift_used: bool,
    shift_started: u64,
    r2_started: u64,
    r2_used: bool,
    one_shot_r2: Option<Mode>,
    started: Instant,
}

fn neutral() -> Snapshot {
    Snapshot {
        buttons: vec![],
        menu_buttons: vec![],
        dpad: Some(Dpad::Neutral),
        l2: 0.0,
        r2: 0.0,
        left_stick: Dpad::Neutral,
        right_stick: Dpad::Neutral,
    }
}

fn thumb_mask(snapshot: &Snapshot) -> u8 {
    let bits = snapshot.dpad.unwrap_or_default().bits();
    let mut result = 0;
    for (i, bit) in [4, 2, 8, 1].into_iter().enumerate() {
        if bits & bit != 0 {
            result |= 1 << i;
        }
    }
    for (i, button) in [Button::Y, Button::B, Button::A, Button::X]
        .into_iter()
        .enumerate()
    {
        if snapshot.buttons.contains(&button) {
            result |= 1 << (i + 4);
        }
    }
    result
}

impl Engine {
    pub fn new(layout: Layout) -> Result<Self> {
        layout.validate()?;
        Ok(Self {
            layout,
            waiting: true,
            previous: neutral(),
            queue: VecDeque::new(),
            repeating: vec![],
            one_shot_shift: false,
            shift_used: false,
            shift_started: 0,
            r2_started: 0,
            r2_used: false,
            one_shot_r2: None,
            started: Instant::now(),
        })
    }

    pub fn phase(&self) -> Phase {
        if self.waiting {
            Phase::Waiting
        } else if self.queue.is_empty() && self.repeating.is_empty() {
            Phase::Idle
        } else {
            Phase::Collecting
        }
    }

    pub fn reset(&mut self, reason: &str) -> Event {
        let discarded = !self.queue.is_empty()
            || !self.repeating.is_empty()
            || self.one_shot_shift
            || self.one_shot_r2.is_some();
        self.queue.clear();
        self.repeating.clear();
        self.waiting = true;
        self.previous = neutral();
        self.one_shot_shift = false;
        self.shift_used = false;
        self.one_shot_r2 = None;
        self.r2_used = false;
        Event::Reset {
            reason: reason.into(),
            discarded,
        }
    }

    fn consume(&mut self, button: Button) {
        for stroke in &mut self.queue {
            if stroke.control == Control::Bumper(button) && !stroke.released {
                stroke.consumed = true;
                stroke.released = true;
            }
        }
    }

    fn modifiers(&mut self, snapshot: &Snapshot, one_shot: bool) -> Vec<Modifier> {
        let mut modifiers = snapshot.modifiers();
        if snapshot.buttons.contains(&Button::L2) || (one_shot && self.one_shot_shift) {
            modifiers.push(Modifier::Shift);
        }
        self.shift_used |= snapshot.buttons.contains(&Button::L2);
        modifiers.sort();
        modifiers.dedup();
        modifiers
    }

    fn clear_one_shots(&mut self) {
        self.one_shot_shift = false;
        self.one_shot_r2 = None;
    }

    fn selector_down(&self, snapshot: &Snapshot, button: Button) -> bool {
        snapshot.buttons.contains(&button)
            && !self
                .repeating
                .iter()
                .any(|s| s.control == Control::Bumper(button))
    }

    fn control_down(control: Control, snapshot: &Snapshot) -> bool {
        match control {
            Control::Thumb(i) => thumb_mask(snapshot) & (1 << i) != 0,
            Control::Bumper(button) => snapshot.buttons.contains(&button),
            Control::Menu(button) => snapshot.menu_buttons.contains(&button),
        }
    }

    fn prepare_bumper(&mut self, stroke: &mut Stroke) {
        if self.one_shot_shift && !stroke.modifiers.contains(&Modifier::Shift) {
            stroke.modifiers.push(Modifier::Shift);
            stroke.modifiers.sort();
        }
        self.clear_one_shots();
    }

    fn repeat_strokes_at(&mut self, snapshot: &Snapshot, now: u64) -> Vec<Event> {
        let mut events = vec![];
        self.repeating
            .retain(|s| Self::control_down(s.control, snapshot));
        for i in 0..self.repeating.len() {
            if now >= self.repeating[i].repeat_at {
                events.push(self.event(&self.repeating[i]));
                self.repeating[i].repeat_at = now.saturating_add(80);
            }
        }
        while let Some(stroke) = self.queue.front() {
            if !stroke.repeat_eligible
                || stroke.released
                || now < stroke.repeat_at
                || !Self::control_down(stroke.control, snapshot)
            {
                break;
            }
            if let Control::Thumb(i) = stroke.control
                && self.layout.action(stroke.mode, stroke.bank, i).is_none()
            {
                break;
            }
            let mut stroke = self.queue.pop_front().unwrap();
            if matches!(stroke.control, Control::Bumper(_)) {
                self.prepare_bumper(&mut stroke);
            }
            stroke.frozen = true;
            events.push(self.event(&stroke));
            stroke.repeat_at = now.saturating_add(80);
            self.repeating.push(stroke);
            events.extend(self.flush());
        }
        events
    }

    fn press(&mut self, control: Control, snapshot: &Snapshot, now: u64) {
        for stroke in &mut self.queue {
            if matches!(stroke.control, Control::Thumb(_)) {
                stroke.frozen = true;
            }
        }
        let mut bank_buttons = snapshot.input().mask;
        for stroke in &self.repeating {
            if let Control::Bumper(button) = stroke.control {
                bank_buttons &= !button.bit();
            }
        }
        let down = |b: Button| bank_buttons & b.bit() != 0;
        let mode = if down(Button::R2) && down(Button::R1) {
            Mode::Navigation
        } else if let Some(mode) = self.one_shot_r2 {
            mode
        } else if down(Button::R2) {
            Mode::Number
        } else {
            Mode::Alpha
        };
        let bank = usize::from(down(Button::L1))
            + if mode == Mode::Alpha {
                2 * usize::from(down(Button::R1))
            } else {
                0
            };
        if matches!(control, Control::Thumb(_)) {
            if down(Button::L1) {
                self.consume(Button::L1);
            }
            if down(Button::R1) {
                self.consume(Button::R1);
            }
        }
        self.r2_used |= down(Button::R2);
        let uses_one_shots = !matches!(control, Control::Bumper(_))
            && !matches!(control, Control::Menu(button) if !button.uses_keyboard_modifiers());
        let modifiers = if matches!(control, Control::Menu(button) if !button.uses_keyboard_modifiers())
        {
            vec![]
        } else {
            self.modifiers(snapshot, uses_one_shots)
        };
        if uses_one_shots {
            self.clear_one_shots();
        }
        self.queue.push_back(Stroke {
            control,
            mode,
            bank,
            started: now,
            modifiers,
            released: false,
            frozen: false,
            consumed: false,
            repeat_at: now.saturating_add(300),
            repeat_eligible: !matches!(control, Control::Bumper(_)) || matches!(control, Control::Bumper(button)
                if snapshot.menu_buttons.is_empty()
                && [Button::L1, Button::R1, Button::R2].into_iter().all(|b| b == button || !snapshot.buttons.contains(&b))),
        });
    }

    fn release(&mut self, control: Control) {
        if let Some(stroke) = self
            .queue
            .iter_mut()
            .find(|s| s.control == control && !s.released)
        {
            stroke.released = true;
            stroke.frozen = true;
        }
    }

    fn event(&self, stroke: &Stroke) -> Event {
        let (chord, action, held) = match stroke.control {
            Control::Thumb(i) => {
                let chord = if i < 4 {
                    Chord {
                        dpad: [Dpad::Up, Dpad::Right, Dpad::Down, Dpad::Left][i],
                        buttons: vec![],
                    }
                } else {
                    Chord {
                        dpad: Dpad::Neutral,
                        buttons: vec![[Button::Y, Button::B, Button::A, Button::X][i - 4]],
                    }
                };
                let mut held = if stroke.mode == Mode::Alpha {
                    let mut held = vec![];
                    if stroke.bank & 1 != 0 {
                        held.push(Button::L1);
                    }
                    if stroke.bank & 2 != 0 {
                        held.push(Button::R1);
                    }
                    held
                } else {
                    let mut held = vec![Button::R2];
                    if stroke.bank != 0 {
                        held.push(Button::L1);
                    }
                    if stroke.mode == Mode::Navigation {
                        held.push(Button::R1);
                    }
                    held
                };
                held.sort();
                (chord, self.layout.action(stroke.mode, stroke.bank, i), held)
            }
            Control::Bumper(button) => (
                Chord {
                    dpad: Dpad::Neutral,
                    buttons: vec![button],
                },
                Some(Action::Key(if button == Button::R1 {
                    Key::Space
                } else {
                    Key::Backspace
                })),
                vec![],
            ),
            Control::Menu(button) => {
                return Event::Menu {
                    button,
                    action: button.action(),
                    keyboard_modifiers: stroke.modifiers.clone(),
                };
            }
        };
        let keyboard_modifiers = if matches!(action, Some(Action::Key(k)) if k.is_consumer()) {
            vec![]
        } else {
            stroke.modifiers.clone()
        };
        match action {
            Some(action) => Event::Chord {
                chord,
                held,
                held_dpad: Dpad::Neutral,
                action: Some(action),
                keyboard_modifiers,
            },
            None => Event::Unmapped {
                chord,
                held,
                held_dpad: Dpad::Neutral,
                keyboard_modifiers,
            },
        }
    }

    fn flush(&mut self) -> Vec<Event> {
        let mut events = vec![];
        while self.queue.front().is_some_and(|s| s.released) {
            let mut stroke = self.queue.pop_front().unwrap();
            if !stroke.consumed {
                // Bumpers become keys only when their standalone tap commits.
                if matches!(stroke.control, Control::Bumper(_)) {
                    self.prepare_bumper(&mut stroke);
                }
                events.push(self.event(&stroke));
            }
        }
        events
    }

    pub fn update(&mut self, snapshot: &Snapshot) -> Vec<Event> {
        self.update_at(snapshot, self.started.elapsed().as_millis() as u64)
    }

    pub fn update_at(&mut self, snapshot: &Snapshot, now: u64) -> Vec<Event> {
        if snapshot.dpad.is_none() {
            return vec![self.reset("invalid D-pad position")];
        }
        if self.waiting {
            if !snapshot.is_neutral() {
                return vec![];
            }
            self.waiting = false;
            self.previous = snapshot.clone();
            return vec![Event::Ready];
        }
        let mut events = self.repeat_strokes_at(snapshot, now);
        let prev = self.previous.input().mask;
        let held = snapshot.input().mask;
        let pressed = held & !prev;
        let released = prev & !held;
        let current_thumbs = thumb_mask(snapshot);
        let previous_thumbs = thumb_mask(&self.previous);
        let other_press = current_thumbs & !previous_thumbs != 0
            || snapshot
                .menu_buttons
                .iter()
                .any(|b| !self.previous.menu_buttons.contains(b))
            || (snapshot.left_stick != Dpad::Neutral
                && snapshot.left_stick != self.previous.left_stick)
            || (snapshot.right_stick != Dpad::Neutral
                && snapshot.right_stick != self.previous.right_stick);
        for stroke in &mut self.queue {
            if let Control::Bumper(button) = stroke.control
                && (other_press || pressed & !button.bit() != 0)
            {
                stroke.repeat_eligible = false;
            }
        }
        for i in 0..8 {
            if previous_thumbs & !current_thumbs & (1 << i) != 0 {
                self.release(Control::Thumb(i));
            }
        }
        if pressed & Button::L2.bit() != 0 {
            self.shift_used = false;
            self.shift_started = now;
        }
        if released & Button::L2.bit() != 0
            && !self.shift_used
            && now.saturating_sub(self.shift_started) <= self.layout.recognition.trigger_tap_max_ms
        {
            self.one_shot_shift = !self.one_shot_shift;
        }
        if pressed & Button::R2.bit() != 0 {
            self.r2_started = now;
            self.r2_used = self.selector_down(snapshot, Button::R1)
                || self
                    .queue
                    .iter()
                    .any(|s| !s.released && matches!(s.control, Control::Thumb(_)));
            for stroke in &mut self.queue {
                stroke.frozen = true;
            }
        }
        if released & Button::R2.bit() != 0
            && !self.r2_used
            && now.saturating_sub(self.r2_started) <= self.layout.recognition.trigger_tap_max_ms
        {
            self.one_shot_r2 = match self.one_shot_r2 {
                None => Some(Mode::Number),
                Some(Mode::Number) => Some(Mode::Function),
                _ => None,
            };
        }
        if held & Button::R2.bit() != 0 && self.selector_down(snapshot, Button::R1) {
            self.consume(Button::R1);
            self.r2_used = true;
        }
        for button in [Button::L1, Button::R1] {
            if released & button.bit() != 0 {
                if held & Button::L2.bit() != 0 {
                    self.shift_used = true;
                }
                self.release(Control::Bumper(button));
            }
        }
        // New thumb strokes in this report own any simultaneously pressed bumper.
        for i in 0..8 {
            if current_thumbs & !previous_thumbs & (1 << i) != 0 {
                self.press(Control::Thumb(i), snapshot, now);
            }
        }
        for button in [Button::L1, Button::R1] {
            if pressed & button.bit() == 0 {
                continue;
            }
            self.r2_used |= held & Button::R2.bit() != 0;
            let bit = if button == Button::L1 { 1 } else { 2 };
            let late = self
                .queue
                .iter_mut()
                .rev()
                .find(|s| matches!(s.control, Control::Thumb(_)));
            let joined = if let Some(stroke) = late {
                if !stroke.released
                    && !stroke.frozen
                    && (stroke.mode == Mode::Alpha || button == Button::L1)
                    && now.saturating_sub(stroke.started)
                        < self.layout.recognition.late_bumper_window_ms
                {
                    stroke.bank |= bit;
                    true
                } else {
                    false
                }
            } else {
                false
            };
            if !joined
                && held & Button::R2.bit() == 0
                && current_thumbs == 0
                && previous_thumbs == 0
            {
                self.press(Control::Bumper(button), snapshot, now);
            }
        }
        for button in MenuButton::ALL {
            let down = snapshot.menu_buttons.contains(&button);
            let was_down = self.previous.menu_buttons.contains(&button);
            if down && !was_down {
                self.press(Control::Menu(button), snapshot, now);
            }
            if !down && was_down {
                self.release(Control::Menu(button));
            }
        }
        self.previous = snapshot.clone();
        if self.queue.len() > 64 {
            return vec![self.reset("too many overlapping alpha-bank strokes")];
        }
        events.extend(self.flush());
        events
    }

    pub fn repeat(&mut self) -> Vec<Event> {
        self.repeat_at(self.started.elapsed().as_millis() as u64)
    }

    pub fn repeat_at(&mut self, now: u64) -> Vec<Event> {
        if self.waiting {
            return vec![];
        }
        self.repeat_strokes_at(&self.previous.clone(), now)
    }
}

impl Layout {
    pub fn reference_html(&self) -> Result<String> {
        use std::fmt::Write;
        self.validate()?;
        fn escape(value: &str) -> String {
            value
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
        }
        fn shifted_character(c: char) -> Option<char> {
            "`1234567890-=[]\\;',./"
                .chars()
                .zip("~!@#$%^&*()_+{}|:\"<>?".chars())
                .find_map(|(base, shifted)| (base == c).then_some(shifted))
        }
        fn label(action: &Action) -> String {
            match action {
                Action::Character('\t') => "Tab".into(),
                Action::Character(c) => c.to_string(),
                Action::Key(Key::Up) => "↑".into(),
                Action::Key(Key::Right) => "→".into(),
                Action::Key(Key::Down) => "↓".into(),
                Action::Key(Key::Left) => "←".into(),
                Action::Key(Key::MediaPlayPause) => "Play".into(),
                Action::Key(Key::MediaPrevious) => "Prev".into(),
                Action::Key(Key::MediaNext) => "Next".into(),
                Action::Key(Key::MediaStop) => "Stop".into(),
                Action::Key(Key::VolumeUp) => "Vol+".into(),
                Action::Key(Key::VolumeDown) => "Vol−".into(),
                Action::Key(Key::PrintScreen) => "PrtSc".into(),
                Action::Key(Key::Delete) => "Del".into(),
                Action::Key(Key::PageUp) => "PgUp".into(),
                Action::Key(Key::PageDown) => "PgDn".into(),
                _ => action.label(),
            }
        }
        let titles = [
            "Base: letters and punctuation",
            "L1",
            "R1",
            "L1 + R1",
            "R2: hold or tap once",
            "Add L1: numbers 5–8",
            "R1 + R2: navigation",
            "Add L1: editing / media",
            "R2 tap twice: function",
            "Add L1: function",
        ];
        let mut sections = String::new();
        for (index, (bank, title)) in self
            .banks
            .iter()
            .chain(&self.navigation_banks)
            .chain(&self.function_banks)
            .zip(titles)
            .enumerate()
        {
            write!(sections, "<section><h2>{title}</h2><div class=\"hands\">").unwrap();
            for side in 0..2 {
                write!(
                    sections,
                    "<div class=\"hand\"><p>{}</p><div class=\"pad\">",
                    if side == 0 { "D-pad" } else { "Face buttons" }
                )
                .unwrap();
                for (position, area) in ["north", "east", "south", "west"].iter().enumerate() {
                    let control = CONTROLS[side * 4 + position];
                    let action = bank
                        .bindings
                        .iter()
                        .find(|b| b.control == control)
                        .map(Binding::action)
                        .transpose()?;
                    let text = action.as_ref().map(label).unwrap_or_else(|| "·".into());
                    let class = if text.chars().count() > 3 {
                        "longer"
                    } else if text.chars().count() > 2 {
                        "long"
                    } else {
                        ""
                    };
                    let shifted = if (4..6).contains(&index) {
                        match action {
                            Some(Action::Character(c)) => shifted_character(c)
                                .map(|c| format!("<em>{}</em>", escape(&c.to_string())))
                                .unwrap_or_default(),
                            _ => String::new(),
                        }
                    } else {
                        String::new()
                    };
                    write!(sections, "<div class=\"key {area}\"><small>{control}</small><strong class=\"{class}\">{}{shifted}</strong></div>", escape(&text)).unwrap();
                }
                sections.push_str("</div></div>");
            }
            sections.push_str("</div></section>");
        }
        let mut sticks = String::new();
        for side in crate::model::Side::ALL {
            let (click, shortcut, purpose) = if side == crate::model::Side::Left {
                ("L3", "Command+D", "Dictate")
            } else {
                ("R3", "Command+Shift+D", "Cancel Dictation")
            };
            write!(
                sticks,
                "<section><h2>{side:?} stick: hold for modifiers</h2><div class=\"stick-map\">"
            )
            .unwrap();
            for (direction, arrow) in [
                (Dpad::UpLeft, "↖"),
                (Dpad::Up, "↑"),
                (Dpad::UpRight, "↗"),
                (Dpad::Left, "←"),
                (Dpad::Neutral, "·"),
                (Dpad::Right, "→"),
                (Dpad::DownLeft, "↙"),
                (Dpad::Down, "↓"),
                (Dpad::DownRight, "↘"),
            ] {
                if direction == Dpad::Neutral {
                    write!(sticks,"<div class=\"key\"><small>Click {click}</small><strong>{shortcut}</strong></div>").unwrap();
                } else {
                    let modifiers = crate::controller::thumbstick_modifiers(side, direction)
                        .iter()
                        .map(|m| format!("{m:?}"))
                        .collect::<Vec<_>>()
                        .chunks(2)
                        .map(|pair| pair.join(" + "))
                        .collect::<Vec<_>>()
                        .join("<br>+ ");
                    write!(sticks,"<div class=\"key\"><small>{arrow}</small><strong>{modifiers}</strong></div>").unwrap();
                }
            }
            write!(
                sticks,
                "</div><p>You can map {click} to {purpose} on your computer.</p></section>"
            )
            .unwrap();
        }
        Ok(include_str!("render/four-bank.html")
            .replace("@@BANKS@@", &sections)
            .replace("@@STICKS@@", &sticks)
            .replace(
                "@@WINDOW_MS@@",
                &self.recognition.late_bumper_window_ms.to_string(),
            ))
    }
}
