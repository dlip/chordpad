use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::Result;

pub const SCHEMA_VERSION: u32 = 2;
pub const COST_MODEL_VERSION: u32 = 2;

fn is_false(value: &bool) -> bool {
    !value
}
fn is_zero(value: &f64) -> bool {
    *value == 0.0
}

fn no_skipgram_weights(value: &[f64; 3]) -> bool {
    *value == [0.0; 3]
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub const ALL: [Self; 2] = [Self::Left, Self::Right];
    pub fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
        }
    }
    pub fn mask(self) -> u8 {
        match self {
            Self::Left => Button::L1.bit() | Button::L2.bit(),
            Self::Right => !(Button::L1.bit() | Button::L2.bit()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finger {
    LeftThumb,
    LeftIndex,
    LeftMiddle,
    RightThumb,
    RightIndex,
    RightMiddle,
}

impl Finger {
    pub fn bit(self) -> u8 {
        1 << self as u8
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Button {
    A,
    B,
    X,
    Y,
    L1,
    R1,
    L2,
    R2,
}

impl Button {
    pub const ALL: [Self; 8] = [
        Self::A,
        Self::B,
        Self::X,
        Self::Y,
        Self::L1,
        Self::R1,
        Self::L2,
        Self::R2,
    ];

    pub fn bit(self) -> u8 {
        1 << self as u8
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum MenuButton {
    Start,
    Select,
}

impl MenuButton {
    pub const ALL: [Self; 2] = [Self::Start, Self::Select];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn action(self) -> Action {
        Action::Key(match self {
            Self::Start => Key::Enter,
            Self::Select => Key::Escape,
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum Dpad {
    #[default]
    Neutral,
    Up,
    Down,
    Left,
    Right,
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}

impl Dpad {
    pub fn is_neutral(&self) -> bool {
        *self == Self::Neutral
    }
    pub fn bits(self) -> u8 {
        let (x, y) = self.position();
        u8::from(x < 0) | (u8::from(x > 0) << 1) | (u8::from(y < 0) << 2) | (u8::from(y > 0) << 3)
    }
    pub const ALL: [Self; 9] = [
        Self::Neutral,
        Self::Up,
        Self::Down,
        Self::Left,
        Self::Right,
        Self::UpLeft,
        Self::UpRight,
        Self::DownLeft,
        Self::DownRight,
    ];

    pub fn position(self) -> (i8, i8) {
        match self {
            Self::Neutral => (0, 0),
            Self::Up => (0, -1),
            Self::Down => (0, 1),
            Self::Left => (-1, 0),
            Self::Right => (1, 0),
            Self::UpLeft => (-1, -1),
            Self::UpRight => (1, -1),
            Self::DownLeft => (-1, 1),
            Self::DownRight => (1, 1),
        }
    }

    pub fn controls(self) -> u32 {
        let (x, y) = self.position();
        u32::from(x != 0) + u32::from(y != 0)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Chord {
    pub dpad: Dpad,
    pub buttons: Vec<Button>,
}

impl Chord {
    pub fn input(&self) -> Result<Input> {
        Ok(Input {
            dpad: self.dpad,
            mask: button_mask(&self.buttons)?,
        })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Input {
    pub dpad: Dpad,
    pub mask: u8,
}

impl Input {
    pub fn fingers(self, context: Self) -> u8 {
        let physical = self.with_context(context);
        let mut fingers = 0;
        if self.dpad != Dpad::Neutral {
            fingers |= Finger::LeftThumb.bit();
        }
        if self.mask & 0x0f != 0 {
            fingers |= Finger::RightThumb.bit();
        }
        for (bumper, trigger, index, middle) in [
            (
                Button::L1,
                Button::L2,
                Finger::LeftIndex,
                Finger::LeftMiddle,
            ),
            (
                Button::R1,
                Button::R2,
                Finger::RightIndex,
                Finger::RightMiddle,
            ),
        ] {
            let both =
                physical.mask & (bumper.bit() | trigger.bit()) == bumper.bit() | trigger.bit();
            if self.mask & bumper.bit() != 0 || (!both && self.mask & trigger.bit() != 0) {
                fingers |= index.bit();
            }
            if both && self.mask & trigger.bit() != 0 {
                fingers |= middle.bit();
            }
        }
        fingers
    }

    pub fn middle_fingers(self) -> u8 {
        self.fingers(Self::default()) & (Finger::LeftMiddle.bit() | Finger::RightMiddle.bit())
    }

    pub fn half(self, side: Side) -> Self {
        Self {
            mask: self.mask & side.mask(),
            dpad: if side == Side::Left {
                self.dpad
            } else {
                Dpad::Neutral
            },
        }
    }
    pub fn typing_side(self) -> Option<Side> {
        if self.controls() == 0 || self.mask & (Button::L2.bit() | Button::R2.bit()) != 0 {
            return None;
        }
        Side::ALL.into_iter().find(|side| self.half(*side) == self)
    }
    pub fn with_context(self, context: Self) -> Self {
        Self {
            mask: self.mask | context.mask,
            dpad: if self.dpad == Dpad::Neutral {
                context.dpad
            } else {
                self.dpad
            },
        }
    }
    pub fn hands(self) -> u8 {
        u8::from(self.half(Side::Left).controls() > 0)
            | (u8::from(self.half(Side::Right).controls() > 0) << 1)
    }
    pub fn controls(self) -> u32 {
        self.mask.count_ones() + self.dpad.controls()
    }

    pub fn chord(self) -> Chord {
        Chord {
            dpad: self.dpad,
            buttons: Button::ALL
                .into_iter()
                .filter(|b| self.mask & b.bit() != 0)
                .collect(),
        }
    }
}

pub fn button_mask(buttons: &[Button]) -> Result<u8> {
    let mut mask = 0;
    for &button in buttons {
        if mask & button.bit() != 0 {
            return Err(format!("duplicate button {button:?}"));
        }
        mask |= button.bit();
    }
    Ok(mask)
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Action {
    Character(char),
    Key(Key),
    Shortcut(Shortcut),
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct Shortcut {
    pub modifiers: Vec<Modifier>,
    pub key: Key,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Modifier {
    Ctrl,
    Alt,
    Shift,
    Meta,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Key {
    Backspace,
    Escape,
    Enter,
    Tab,
    Space,
    Delete,
    Insert,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    PrintScreen,
    MediaPlayPause,
    MediaNext,
    MediaPrevious,
    MediaStop,
    VolumeUp,
    VolumeDown,
    Mute,
}

impl Key {
    pub fn is_consumer(self) -> bool {
        matches!(
            self,
            Self::MediaPlayPause
                | Self::MediaNext
                | Self::MediaPrevious
                | Self::MediaStop
                | Self::VolumeUp
                | Self::VolumeDown
                | Self::Mute
        )
    }
}

impl Action {
    pub fn label(&self) -> String {
        match self {
            Self::Character(' ') => "Space".into(),
            Self::Character('\n') => "Enter".into(),
            Self::Character('\t') => "Tab".into(),
            Self::Character(c) => format!("{c:?}"),
            Self::Key(key) => format!("{key:?}"),
            Self::Shortcut(shortcut) => shortcut
                .modifiers
                .iter()
                .map(|m| format!("{m:?}"))
                .chain(std::iter::once(format!("{:?}", shortcut.key)))
                .collect::<Vec<_>>()
                .join("+"),
        }
    }

    pub fn validate(&self) -> Result<()> {
        if let Self::Shortcut(shortcut) = self {
            let unique: BTreeSet<_> = shortcut.modifiers.iter().collect();
            if unique.is_empty() || unique.len() != shortcut.modifiers.len() {
                return Err("shortcut modifiers must be nonempty and unique".into());
            }
            if !shortcut.modifiers.windows(2).all(|w| w[0] < w[1]) {
                return Err("shortcut modifiers must be ordered: ctrl, alt, shift, meta".into());
            }
        }
        Ok(())
    }

    pub fn group(&self, shift: bool) -> (Self, bool) {
        if shift && let Self::Character(c) = self {
            let base = "abcdefghijklmnopqrstuvwxyz`1234567890-=[]\\;',./";
            let shifted = "ABCDEFGHIJKLMNOPQRSTUVWXYZ~!@#$%^&*()_+{}|:\"<>?";
            if let Some(index) = shifted.chars().position(|s| s == *c) {
                return (Self::Character(base.chars().nth(index).unwrap()), true);
            }
        }
        (self.clone(), false)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Strategy {
    #[serde(default, skip_serializing_if = "is_false")]
    pub independent: bool,
    pub name: String,
    #[serde(default)]
    pub shift: Option<Button>,
    #[serde(default)]
    pub layer: Option<Button>,
}

impl Strategy {
    pub fn recognition(&self) -> &'static str {
        if self.independent {
            "independent-release"
        } else {
            "release-all"
        }
    }
    pub fn modifier_mask(&self) -> u8 {
        self.shift.map_or(0, Button::bit) | self.layer.map_or(0, Button::bit)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Costs {
    #[serde(default, skip_serializing_if = "is_zero")]
    pub same_hand_transition: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub same_finger_transition: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub grip_change: f64,
    #[serde(default, skip_serializing_if = "no_skipgram_weights")]
    pub skipgram_weights: [f64; 3],
    #[serde(default, skip_serializing_if = "is_zero")]
    pub awkward_diagonal_extra: f64,
    pub buttons: BTreeMap<Button, f64>,
    pub dpad: f64,
    pub diagonal_extra: f64,
    pub chord_complexity: f64,
    pub multiple_face_buttons: f64,
    pub same_side_shoulders: f64,
    pub held_load: f64,
    pub modifier_change: f64,
}

impl Default for Costs {
    fn default() -> Self {
        Self {
            same_hand_transition: 0.0,
            same_finger_transition: 0.0,
            grip_change: 0.0,
            skipgram_weights: [0.0; 3],
            awkward_diagonal_extra: 0.0,
            buttons: Button::ALL
                .into_iter()
                .map(|b| {
                    let cost = match b {
                        Button::A | Button::B | Button::X | Button::Y => 1.0,
                        Button::L1 | Button::R1 => 1.1,
                        Button::L2 | Button::R2 => 1.4,
                    };
                    (b, cost)
                })
                .collect(),
            dpad: 1.0,
            diagonal_extra: 0.6,
            chord_complexity: 0.18,
            multiple_face_buttons: 1.5,
            same_side_shoulders: 0.7,
            held_load: 0.15,
            modifier_change: 0.35,
        }
    }
}

impl Costs {
    fn diagonal_effort(&self, dpad: Dpad) -> f64 {
        if dpad.controls() != 2 {
            return 0.0;
        }
        self.diagonal_extra
            + if matches!(dpad, Dpad::UpRight | Dpad::DownLeft) {
                self.awkward_diagonal_extra
            } else {
                0.0
            }
    }

    pub fn effort_with_context(&self, input: Input, context: Input) -> f64 {
        let base = self.effort(input, context.mask);
        if context.dpad == Dpad::Neutral {
            return base;
        }
        let before = (input.mask | context.mask).count_ones() + input.dpad.controls();
        let after = before + context.dpad.controls();
        base + (self.dpad + self.diagonal_effort(context.dpad)) * self.held_load
            + self.chord_complexity
                * (f64::from(after.saturating_sub(1)).powi(2)
                    - f64::from(before.saturating_sub(1)).powi(2))
    }
    pub fn effort(&self, input: Input, held: u8) -> f64 {
        let physical = input.mask | held;
        let count = physical.count_ones() + input.dpad.controls();
        let face_extra = (physical & 0x0f).count_ones().saturating_sub(1);
        let same_side = [(Button::L1, Button::L2), (Button::R1, Button::R2)]
            .into_iter()
            .filter(|(a, b)| physical & (a.bit() | b.bit()) == a.bit() | b.bit())
            .count();
        let buttons = self
            .buttons
            .iter()
            .filter(|(b, _)| input.mask & b.bit() != 0)
            .map(|(_, c)| *c)
            .sum::<f64>();
        let load = self
            .buttons
            .iter()
            .filter(|(b, _)| held & b.bit() != 0)
            .map(|(_, c)| *c)
            .sum::<f64>()
            * self.held_load;
        buttons
            + load
            + if input.dpad == Dpad::Neutral {
                0.0
            } else {
                self.dpad
            }
            + self.diagonal_effort(input.dpad)
            + self.awkward_diagonal_extra
                * [
                    Button::X.bit() | Button::Y.bit(),
                    Button::A.bit() | Button::B.bit(),
                ]
                .into_iter()
                .filter(|pair| physical & pair == *pair)
                .count() as f64
            + self.chord_complexity * f64::from(count.saturating_sub(1)).powi(2)
            + self.multiple_face_buttons * f64::from(face_extra).powi(2)
            + self.same_side_shoulders * same_side as f64
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Weights {
    pub effort: f64,
    pub activations: f64,
    pub movement: f64,
    pub coverage: f64,
    pub learning: f64,
}

impl Default for Weights {
    fn default() -> Self {
        Self {
            effort: 1.0,
            activations: 1.5,
            movement: 0.4,
            coverage: 0.1,
            learning: 0.1,
        }
    }
}

fn default_extra_function_chords() -> [Chord; 2] {
    [
        Chord {
            dpad: Dpad::Left,
            buttons: vec![],
        },
        Chord {
            dpad: Dpad::Down,
            buttons: vec![],
        },
    ]
}

fn default_function_chords(chords: &[Chord; 2]) -> bool {
    *chords == default_extra_function_chords()
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Profile {
    #[serde(default, skip_serializing_if = "is_false")]
    pub navigation_media: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub function_volume: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub function_print_screen: bool,
    #[serde(skip_serializing_if = "default_function_chords")]
    pub extra_function_chords: [Chord; 2],
    #[serde(default, skip_serializing_if = "is_false")]
    pub function_layer: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub navigation_layer: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub exclude_opposite_face_pairs: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub base_punctuation: Vec<char>,
    pub diagonals: bool,
    pub disabled_buttons: Vec<Button>,
    pub modifier_only_buttons: Vec<Button>,
    pub max_chord_size: Option<u32>,
    pub forbidden_combinations: Vec<Vec<Button>>,
    pub forbidden_chords: Vec<Chord>,
    pub costs: Costs,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            navigation_media: false,
            function_volume: false,
            function_print_screen: false,
            exclude_opposite_face_pairs: false,
            function_layer: false,
            extra_function_chords: default_extra_function_chords(),
            navigation_layer: false,
            base_punctuation: vec![],
            diagonals: true,
            disabled_buttons: vec![],
            modifier_only_buttons: vec![Button::L2, Button::R2],
            max_chord_size: None,
            forbidden_combinations: vec![],
            forbidden_chords: vec![],
            costs: Costs::default(),
        }
    }
}

impl Profile {
    pub fn navigation_face_keys(&self) -> Vec<(u8, Key)> {
        let mut keys = vec![
            (Button::A.bit(), Key::PageDown),
            (Button::B.bit(), Key::End),
            (Button::X.bit(), Key::Home),
            (Button::Y.bit(), Key::PageUp),
        ];
        if self.navigation_media {
            keys.extend([
                (Button::X.bit() | Button::Y.bit(), Key::MediaPrevious),
                (Button::Y.bit() | Button::B.bit(), Key::MediaNext),
                (Button::A.bit() | Button::X.bit(), Key::Delete),
                (Button::A.bit() | Button::B.bit(), Key::MediaPlayPause),
            ]);
        }
        keys
    }

    pub fn function_extras(&self) -> Vec<(Chord, Key)> {
        let mut extras: Vec<_> = self
            .extra_function_chords
            .iter()
            .cloned()
            .zip([Key::F11, Key::F12])
            .collect();
        if self.function_volume {
            for (buttons, key) in [
                (vec![Button::X, Button::Y], Key::VolumeUp),
                (vec![Button::A, Button::X], Key::VolumeDown),
                (vec![Button::A, Button::B], Key::Mute),
            ] {
                extras.push((
                    Chord {
                        dpad: Dpad::Neutral,
                        buttons,
                    },
                    key,
                ));
            }
        }
        if self.function_print_screen {
            extras.push((
                Chord {
                    dpad: Dpad::Neutral,
                    buttons: vec![Button::B, Button::Y],
                },
                Key::PrintScreen,
            ));
        }
        extras
    }

    pub fn validate(&self) -> Result<()> {
        button_mask(&self.disabled_buttons)?;
        button_mask(&self.modifier_only_buttons)?;
        if self.navigation_media && !self.navigation_layer {
            return Err("navigation_media requires the navigation layer".into());
        }
        if self.function_volume && !self.function_layer {
            return Err("function_volume requires the function layer".into());
        }
        if self.function_print_screen && !self.function_layer {
            return Err("function_print_screen requires the function layer".into());
        }
        let mut function_chords = BTreeSet::new();
        for (chord, _) in self.function_extras() {
            let input = chord.input()?;
            if input.typing_side().is_none()
                || input.mask & !0x0f != 0
                || !function_chords.insert(input)
            {
                return Err(
                    "Function extras require distinct D-pad or face-button combos without shoulders".into(),
                );
            }
        }
        let mut punctuation = BTreeSet::new();
        for &c in &self.base_punctuation {
            if !c.is_ascii_punctuation()
                || Action::Character(c).group(true).1
                || !punctuation.insert(c)
            {
                return Err(
                    "base_punctuation must list distinct unshifted ASCII punctuation characters"
                        .into(),
                );
            }
        }

        if self.max_chord_size.is_some_and(|n| n == 0 || n > 10) {
            return Err("max_chord_size must be between 1 and 10".into());
        }
        for buttons in &self.forbidden_combinations {
            if button_mask(buttons)? == 0 {
                return Err("a forbidden combination cannot be empty".into());
            }
        }
        for chord in &self.forbidden_chords {
            chord.input()?;
        }
        for button in Button::ALL {
            let value = self
                .costs
                .buttons
                .get(&button)
                .ok_or_else(|| format!("missing effort cost for {button:?}"))?;
            nonnegative(&format!("cost for {button:?}"), *value)?;
        }
        for (name, value) in [
            ("same_hand_transition", self.costs.same_hand_transition),
            ("same_finger_transition", self.costs.same_finger_transition),
            ("grip_change", self.costs.grip_change),
            ("awkward_diagonal_extra", self.costs.awkward_diagonal_extra),
            ("dpad", self.costs.dpad),
            ("diagonal_extra", self.costs.diagonal_extra),
            ("chord_complexity", self.costs.chord_complexity),
            ("multiple_face_buttons", self.costs.multiple_face_buttons),
            ("same_side_shoulders", self.costs.same_side_shoulders),
            ("held_load", self.costs.held_load),
            ("modifier_change", self.costs.modifier_change),
        ] {
            nonnegative(name, value)?;
        }
        for value in self.costs.skipgram_weights {
            nonnegative("skipgram_weights", value)?;
        }
        Ok(())
    }

    pub fn allows(&self, input: Input) -> bool {
        if input.controls() == 0
            || self.exclude_opposite_face_pairs
                && [
                    Button::A.bit() | Button::Y.bit(),
                    Button::B.bit() | Button::X.bit(),
                ]
                .into_iter()
                .any(|pair| input.mask & pair == pair)
            || (!self.diagonals && input.dpad.controls() == 2)
            || self
                .disabled_buttons
                .iter()
                .any(|b| input.mask & b.bit() != 0)
            || self
                .max_chord_size
                .is_some_and(|max| input.controls() > max)
        {
            return false;
        }
        !self.forbidden_combinations.iter().any(|buttons| {
            let mask = buttons.iter().fold(0, |mask, b| mask | b.bit());
            input.mask & mask == mask
        }) && !self
            .forbidden_chords
            .iter()
            .any(|chord| chord.input().is_ok_and(|c| c == input))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    #[serde(default, skip_serializing_if = "Dpad::is_neutral")]
    pub held_dpad: Dpad,
    pub action: Action,
    pub chord: Chord,
    #[serde(default)]
    pub held: Vec<Button>,
}

impl Binding {
    pub fn context(&self) -> Result<Input> {
        Ok(Input {
            mask: button_mask(&self.held)?,
            dpad: self.held_dpad,
        })
    }
}

pub fn independent_fixed_action(input: Input) -> Option<Action> {
    if input.dpad != Dpad::Neutral {
        return None;
    }
    if input.mask == Button::L1.bit() {
        Some(Action::Key(Key::Backspace))
    } else if input.mask == Button::R1.bit() {
        Some(Action::Character(' '))
    } else {
        None
    }
}

pub fn independent_binding_allowed(
    profile: &Profile,
    action: &Action,
    input: Input,
    context: Input,
) -> bool {
    if profile.function_layer && matches!(action, Action::Character(c) if c.is_ascii_digit()) {
        return context.mask & Button::R2.bit() != 0
            && input.mask & (Button::L1.bit() | Button::R1.bit()) == 0
            && !profile
                .function_extras()
                .iter()
                .any(|(chord, _)| chord.input().ok() == Some(input))
            && profile.allows(input.with_context(Input {
                mask: Button::R1.bit() | Button::R2.bit(),
                dpad: Dpad::Neutral,
            }));
    }
    if let Some(fixed) = independent_fixed_action(input) {
        return *action == fixed && context.controls() == 0;
    }
    match action {
        Action::Character(' ') | Action::Key(Key::Space | Key::Backspace) => false,
        Action::Character(c) if c.is_alphabetic() => context.mask & Button::R2.bit() == 0,
        Action::Character(c) if !c.is_whitespace() && !c.is_control() => {
            context.mask & Button::R2.bit() != 0
                || matches!(action.group(true).0, Action::Character(base) if profile.base_punctuation.contains(&base))
        }
        _ => true,
    }
}

pub fn validate_independent_input(profile: &Profile, input: Input, context: Input) -> Result<()> {
    if profile.function_layer
        && context.mask & Button::R2.bit() != 0
        && input.mask & Button::R1.bit() != 0
    {
        return Err("R2 + R1 is reserved for the function layer".into());
    }
    if profile.navigation_layer
        && context.mask & Button::R2.bit() != 0
        && input.mask & Button::L1.bit() != 0
    {
        return Err("R2 + L1 is reserved for the navigation layer".into());
    }
    input
        .typing_side()
        .ok_or("independent typing chords must use one half and cannot contain triggers")?;
    if context.dpad != Dpad::Neutral || context.mask & !(Button::L2.bit() | Button::R2.bit()) != 0 {
        return Err("independent held controls must be L2 (Shift) and R2 (numbers/symbols)".into());
    }
    if input.mask & button_mask(&profile.modifier_only_buttons)? != 0
        || !profile.allows(input.with_context(context))
    {
        return Err("independent chord or held context is forbidden by the profile".into());
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub profile: Profile,
    pub weights: Weights,
    pub strategies: Vec<Strategy>,
    pub pinned_bindings: Vec<Binding>,
}

impl Default for Config {
    fn default() -> Self {
        let strategy = |name: &str, shift, layer| Strategy {
            independent: false,
            name: name.into(),
            shift,
            layer,
        };
        Self {
            profile: Profile::default(),
            weights: Weights::default(),
            pinned_bindings: vec![],
            strategies: vec![
                strategy("direct", None, None),
                strategy("l2-shift", Some(Button::L2), None),
                strategy("r2-shift", Some(Button::R2), None),
                strategy("l2-layer", None, Some(Button::L2)),
                strategy("r2-layer", None, Some(Button::R2)),
                strategy("l2-shift-r2-layer", Some(Button::L2), Some(Button::R2)),
                strategy("r2-shift-l2-layer", Some(Button::R2), Some(Button::L2)),
            ],
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        self.profile.validate()?;
        let weights = [
            self.weights.effort,
            self.weights.activations,
            self.weights.movement,
            self.weights.coverage,
            self.weights.learning,
        ];
        for weight in weights {
            nonnegative("weight", weight)?;
        }
        if weights.iter().all(|&w| w == 0.0) {
            return Err("at least one weight must be positive".into());
        }
        if self.strategies.is_empty() {
            return Err("at least one strategy is required".into());
        }
        let mut names = BTreeSet::new();
        for strategy in &self.strategies {
            if strategy.independent
                && (strategy.shift != Some(Button::L2) || strategy.layer != Some(Button::R2))
            {
                return Err("independent strategies require L2 as Shift and R2 as the numbers/symbols layer".into());
            }
            if self.profile.navigation_layer && !strategy.independent {
                return Err("the navigation layer requires an independent strategy".into());
            }
            if self.profile.function_layer && !strategy.independent {
                return Err("the function layer requires an independent strategy".into());
            }
            if strategy.name.is_empty() || !names.insert(&strategy.name) {
                return Err("strategy names must be nonempty and unique".into());
            }
            if strategy.shift.is_some() && strategy.shift == strategy.layer {
                return Err(format!(
                    "{} uses one button for both Shift and layer",
                    strategy.name
                ));
            }
        }
        let mut actions = BTreeSet::new();
        for binding in &self.pinned_bindings {
            binding.action.validate()?;
            binding.chord.input()?;
            button_mask(&binding.held)?;
            if !actions.insert(&binding.action) {
                return Err(format!("duplicate pin for {}", binding.action.label()));
            }
        }
        Ok(())
    }
}

fn nonnegative(name: &str, value: f64) -> Result<()> {
    if !value.is_finite() || value < 0.0 {
        Err(format!("{name} must be finite and nonnegative"))
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Metrics {
    pub effort: f64,
    pub activations: f64,
    pub movement: f64,
    pub coverage: f64,
    pub learning: f64,
}

impl Metrics {
    pub fn normalization(self) -> Self {
        let scale = |v: f64| if v > 1e-9 { v } else { 1.0 };
        Self {
            effort: scale(self.effort),
            activations: scale(self.activations),
            movement: scale(self.movement),
            coverage: scale(self.coverage),
            learning: scale(self.learning),
        }
    }

    pub fn objective(self, reference: Self, w: &Weights) -> f64 {
        self.effort / reference.effort * w.effort
            + self.activations / reference.activations * w.activations
            + self.movement / reference.movement * w.movement
            + self.coverage / reference.coverage * w.coverage
            + self.learning / reference.learning * w.learning
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub schema_version: u32,
    pub cost_model_version: u32,
    pub strategy: Strategy,
    pub profile: Profile,
    pub weights: Weights,
    pub normalization: Metrics,
    pub recognition: String,
    pub bindings: Vec<Binding>,
}

impl Layout {
    pub fn navigation_bindings(&self) -> Vec<Binding> {
        if !self.profile.navigation_layer {
            return vec![];
        }
        let mut controls = vec![
            (
                Input {
                    mask: 0,
                    dpad: Dpad::Up,
                },
                Key::Up,
            ),
            (
                Input {
                    mask: 0,
                    dpad: Dpad::Down,
                },
                Key::Down,
            ),
            (
                Input {
                    mask: 0,
                    dpad: Dpad::Left,
                },
                Key::Left,
            ),
            (
                Input {
                    mask: 0,
                    dpad: Dpad::Right,
                },
                Key::Right,
            ),
        ];
        controls.extend(
            self.profile
                .navigation_face_keys()
                .into_iter()
                .map(|(mask, key)| {
                    (
                        Input {
                            mask,
                            dpad: Dpad::Neutral,
                        },
                        key,
                    )
                }),
        );
        controls
            .into_iter()
            .map(|(input, key)| Binding {
                chord: input.chord(),
                held: vec![Button::L1, Button::R2],
                held_dpad: Dpad::Neutral,
                action: Action::Key(key),
            })
            .filter(|binding| {
                self.profile.allows(
                    binding
                        .chord
                        .input()
                        .unwrap()
                        .with_context(binding.context().unwrap()),
                )
            })
            .collect()
    }

    pub fn function_bindings(&self) -> Vec<Binding> {
        if !self.profile.function_layer {
            return vec![];
        }
        let keys = [
            Key::F10,
            Key::F1,
            Key::F2,
            Key::F3,
            Key::F4,
            Key::F5,
            Key::F6,
            Key::F7,
            Key::F8,
            Key::F9,
        ];
        let mut bindings = vec![];
        for binding in &self.bindings {
            if let Action::Character(c) = binding.action
                && c.is_ascii_digit()
                && binding.held == [Button::R2]
            {
                bindings.push(Binding {
                    action: Action::Key(keys[c as usize - '0' as usize]),
                    chord: binding.chord.clone(),
                    held: vec![Button::R1, Button::R2],
                    held_dpad: Dpad::Neutral,
                });
            }
        }
        for (chord, key) in self.profile.function_extras() {
            bindings.push(Binding {
                action: Action::Key(key),
                chord,
                held: vec![Button::R1, Button::R2],
                held_dpad: Dpad::Neutral,
            });
        }
        bindings
    }

    pub fn scoring_actions(&self) -> Vec<Action> {
        let mut actions: Vec<_> = self.bindings.iter().map(|b| b.action.clone()).collect();
        if self.strategy.independent {
            for action in [Action::Character('\n'), Action::Key(Key::Escape)] {
                if !actions.contains(&action) {
                    actions.push(action);
                }
            }
        }
        actions
    }

    pub fn validate(&self) -> Result<()> {
        if ![1, SCHEMA_VERSION].contains(&self.schema_version)
            || ![1, COST_MODEL_VERSION].contains(&self.cost_model_version)
        {
            return Err("unsupported layout schema or cost model version".into());
        }
        if self.recognition != self.strategy.recognition() {
            return Err("unsupported chord recognition policy".into());
        }
        if (self.strategy.independent
            || self.profile.costs.same_hand_transition != 0.0
            || self.profile.costs.same_finger_transition != 0.0
            || self.profile.costs.grip_change != 0.0
            || self.profile.costs.skipgram_weights != [0.0; 3]
            || self.profile.costs.awkward_diagonal_extra != 0.0)
            && (self.schema_version < 2 || self.cost_model_version < 2)
        {
            return Err(
                "independent recognition and extended ergonomic scoring require layout/cost versions 2"
                    .into(),
            );
        }
        let r = self.normalization;
        if [r.effort, r.activations, r.movement, r.coverage, r.learning]
            .iter()
            .any(|v| !v.is_finite() || *v <= 0.0)
        {
            return Err("normalization values must be finite and positive".into());
        }
        Config {
            profile: self.profile.clone(),
            weights: self.weights.clone(),
            strategies: vec![self.strategy.clone()],
            pinned_bindings: self.bindings.clone(),
        }
        .validate()?;
        if self.bindings.is_empty() {
            return Err("layout must contain bindings".into());
        }
        let excluded =
            self.strategy.modifier_mask() | button_mask(&self.profile.modifier_only_buttons)?;
        let mut groups = BTreeMap::new();
        let mut occupied = BTreeMap::new();
        for binding in &self.bindings {
            let input = binding.chord.input()?;
            let held = button_mask(&binding.held)?;
            let context = binding.context()?;
            if self.strategy.independent {
                validate_independent_input(&self.profile, input, context)?;
                if !independent_binding_allowed(&self.profile, &binding.action, input, context) {
                    return Err("independent layouts require base letters, R2 numbers/symbols, R1 Space and L1 Backspace".into());
                }
            }
            let (group, shifted) = binding.action.group(self.strategy.shift.is_some());
            let shift = self.strategy.shift.map_or(0, Button::bit);
            if input.controls() == 0
                || binding.held_dpad != Dpad::Neutral
                || input.mask & excluded != 0
                || held & !self.strategy.modifier_mask() != 0
                || (held & shift != 0) != shifted
                || !self.profile.allows(Input {
                    mask: input.mask | held,
                    ..input
                })
            {
                return Err(format!("invalid binding for {}", binding.action.label()));
            }
            let slot = (
                input,
                Input {
                    dpad: Dpad::Neutral,
                    mask: held & self.strategy.layer.map_or(0, Button::bit),
                },
            );
            if groups
                .insert(group.clone(), slot)
                .is_some_and(|previous| previous != slot)
            {
                return Err("Shift-related bindings must share an ordinary chord and layer".into());
            }
            if occupied
                .insert(slot, group.clone())
                .is_some_and(|previous| previous != group)
            {
                return Err("two groups occupy the same chord and layer".into());
            }
        }
        if self.profile.function_layer {
            let bindings = self.function_bindings();
            if bindings.len() != 10 + self.profile.function_extras().len()
                || bindings.iter().any(|b| {
                    !self
                        .profile
                        .allows(b.chord.input().unwrap().with_context(b.context().unwrap()))
                })
            {
                return Err(
                    "the function layer requires all ten number bindings and allowed F1–F12 combos"
                        .into(),
                );
            }
        }
        Ok(())
    }
}
