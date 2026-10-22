use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::Result;

pub const SCHEMA_VERSION: u32 = 1;
pub const COST_MODEL_VERSION: u32 = 1;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Input {
    pub dpad: Dpad,
    pub mask: u8,
}

impl Input {
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
    pub name: String,
    #[serde(default)]
    pub shift: Option<Button>,
    #[serde(default)]
    pub layer: Option<Button>,
}

impl Strategy {
    pub fn modifier_mask(&self) -> u8 {
        self.shift.map_or(0, Button::bit) | self.layer.map_or(0, Button::bit)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Costs {
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
            + if input.dpad.controls() == 2 {
                self.diagonal_extra
            } else {
                0.0
            }
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Profile {
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
    pub fn validate(&self) -> Result<()> {
        button_mask(&self.disabled_buttons)?;
        button_mask(&self.modifier_only_buttons)?;
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
        Ok(())
    }

    pub fn allows(&self, input: Input) -> bool {
        if input.controls() == 0
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
    pub action: Action,
    pub chord: Chord,
    #[serde(default)]
    pub held: Vec<Button>,
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
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != SCHEMA_VERSION || self.cost_model_version != COST_MODEL_VERSION {
            return Err("unsupported layout schema or cost model version".into());
        }
        if self.recognition != "release-all" {
            return Err("unsupported chord recognition policy".into());
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
            let (group, shifted) = binding.action.group(self.strategy.shift.is_some());
            let shift = self.strategy.shift.map_or(0, Button::bit);
            if input.controls() == 0
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
            let slot = (input, held & self.strategy.layer.map_or(0, Button::bit));
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
        Ok(())
    }
}
