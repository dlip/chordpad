use crate::{
    Result,
    model::{Button, Dpad, Input, MenuButton, Modifier, Side, button_mask},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const KEY: u16 = 1;
pub const ABS: u16 = 3;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RawEvent {
    pub event_type: u16,
    pub code: u16,
    pub value: i32,
}

#[derive(Default)]
pub struct ReportReader {
    pending: Vec<RawEvent>,
    dropping: bool,
}

pub enum Packet {
    Frame(Vec<RawEvent>),
    Dropped,
    Resync,
    StateReset,
}

impl ReportReader {
    pub fn push(&mut self, event: RawEvent) -> Option<Packet> {
        if event.event_type == 0 && event.code == 3 {
            self.pending.clear();
            self.dropping = true;
            return Some(Packet::Dropped);
        }
        if self.dropping {
            if event.event_type == 0 && event.code == 0 {
                self.dropping = false;
                return Some(Packet::Resync);
            }
            return None;
        }
        if event.event_type == 0 && event.code == 0 {
            // The input core uses nonzero SYN_REPORT for synthesized releases
            // and forced report splits. Neither is a trustworthy stroke boundary.
            if event.value != 0 {
                self.pending.clear();
                return Some(Packet::StateReset);
            }
            return Some(Packet::Frame(std::mem::take(&mut self.pending)));
        }
        if event.event_type == KEY || event.event_type == ABS {
            self.pending.push(event);
        }
        None
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeviceId {
    pub vendor: u16,
    pub product: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum DpadSource {
    Hat {
        x: u16,
        y: u16,
    },
    Buttons {
        up: u16,
        down: u16,
        left: u16,
        right: u16,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Trigger {
    Axis {
        code: u16,
        rest: i32,
        pressed: i32,
        #[serde(default = "press_threshold")]
        press_threshold: f64,
        #[serde(default = "release_threshold")]
        release_threshold: f64,
    },
    Key {
        code: u16,
    },
}

fn press_threshold() -> f64 {
    0.65
}
fn release_threshold() -> f64 {
    0.35
}

impl Trigger {
    pub fn axis(code: u16, rest: i32, pressed: i32) -> Self {
        Self::Axis {
            code,
            rest,
            pressed,
            press_threshold: press_threshold(),
            release_threshold: release_threshold(),
        }
    }

    pub fn source(&self) -> (u16, u16) {
        match *self {
            Self::Axis { code, .. } => (ABS, code),
            Self::Key { code } => (KEY, code),
        }
    }

    fn update(&self, value: i32, active: bool) -> (f64, bool) {
        match *self {
            Self::Axis {
                rest,
                pressed,
                press_threshold,
                release_threshold,
                ..
            } => {
                let travel = ((f64::from(value) - f64::from(rest))
                    / (f64::from(pressed) - f64::from(rest)))
                .clamp(0.0, 1.0);
                (
                    travel,
                    if active {
                        travel > release_threshold
                    } else {
                        travel >= press_threshold
                    },
                )
            }
            Self::Key { .. } => (f64::from(value != 0), value != 0),
        }
    }

    pub fn calibrated(
        &self,
        rest: i32,
        minimum: i32,
        maximum: i32,
        advertised_span: f64,
    ) -> Result<Self> {
        let Self::Axis {
            code,
            press_threshold,
            release_threshold,
            ..
        } = *self
        else {
            return Ok(self.clone());
        };
        let pressed = if (f64::from(minimum) - f64::from(rest)).abs()
            > (f64::from(maximum) - f64::from(rest)).abs()
        {
            minimum
        } else {
            maximum
        };
        if (f64::from(pressed) - f64::from(rest)).abs() < (advertised_span * 0.25).max(1.0) {
            return Err("insufficient trigger travel; fully press and release both triggers during the sweep".into());
        }
        Ok(Self::Axis {
            code,
            rest,
            pressed,
            press_threshold,
            release_threshold,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StickAxis {
    pub code: u16,
    pub minimum: i32,
    pub center: i32,
    pub maximum: i32,
    #[serde(default)]
    pub inverted: bool,
}

impl StickAxis {
    fn normalized(&self, value: i32) -> f64 {
        let delta = f64::from(value) - f64::from(self.center);
        let span = if delta < 0.0 {
            f64::from(self.center) - f64::from(self.minimum)
        } else {
            f64::from(self.maximum) - f64::from(self.center)
        };
        (delta / span).clamp(-1.0, 1.0) * if self.inverted { -1.0 } else { 1.0 }
    }
}

fn stick_press_threshold() -> f64 {
    0.55
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Thumbstick {
    pub x: StickAxis,
    pub y: StickAxis,
    #[serde(default = "stick_press_threshold")]
    pub press_threshold: f64,
    #[serde(default = "release_threshold")]
    pub release_threshold: f64,
}

impl Thumbstick {
    pub fn new(x: StickAxis, y: StickAxis) -> Self {
        Self {
            x,
            y,
            press_threshold: stick_press_threshold(),
            release_threshold: release_threshold(),
        }
    }
    fn update(&self, x: i32, y: i32, previous: Dpad) -> Dpad {
        let axis = |value: f64, old: i8| {
            if value >= self.press_threshold {
                1
            } else if value <= -self.press_threshold {
                -1
            } else if old > 0 && value > self.release_threshold {
                1
            } else if old < 0 && value < -self.release_threshold {
                -1
            } else {
                0
            }
        };
        let (px, py) = previous.position();
        direction(
            axis(self.x.normalized(x), px),
            axis(self.y.normalized(y), py),
        )
        .unwrap()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Calibration {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub thumbsticks: BTreeMap<Side, Thumbstick>,
    #[serde(default)]
    pub device: Option<DeviceId>,
    pub buttons: BTreeMap<Button, u16>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub menu_buttons: BTreeMap<MenuButton, u16>,
    pub dpad: DpadSource,
    pub triggers: BTreeMap<Button, Trigger>,
}

impl Default for Calibration {
    fn default() -> Self {
        Self {
            schema_version: 1,
            thumbsticks: BTreeMap::new(),
            device: None,
            buttons: [
                (Button::A, 304),
                (Button::B, 305),
                (Button::X, 308),
                (Button::Y, 307),
                (Button::L1, 310),
                (Button::R1, 311),
            ]
            .into(),
            menu_buttons: [(MenuButton::Start, 315), (MenuButton::Select, 314)].into(),
            dpad: DpadSource::Hat { x: 16, y: 17 },
            triggers: [
                (Button::L2, Trigger::axis(2, 0, 255)),
                (Button::R2, Trigger::axis(5, 0, 255)),
            ]
            .into(),
        }
    }
}

impl Calibration {
    pub fn validate(&self) -> Result<()> {
        if ![1, 2].contains(&self.schema_version) {
            return Err("unsupported calibration schema version".into());
        }
        let mut sources = BTreeSet::new();
        if self.buttons.len() != 6 || self.triggers.len() != 2 {
            return Err("map six digital buttons and both triggers".into());
        }
        for button in [
            Button::A,
            Button::B,
            Button::X,
            Button::Y,
            Button::L1,
            Button::R1,
        ] {
            let code = self
                .buttons
                .get(&button)
                .ok_or_else(|| format!("missing mapping for {button:?}"))?;
            if !sources.insert((KEY, *code)) {
                return Err("duplicate input source in controller mapping".into());
            }
        }
        for &code in self.menu_buttons.values() {
            if !sources.insert((KEY, code)) {
                return Err("duplicate menu button input source".into());
            }
        }
        let dpad_sources = match self.dpad {
            DpadSource::Hat { x, y } => vec![(ABS, x), (ABS, y)],
            DpadSource::Buttons {
                up,
                down,
                left,
                right,
            } => vec![(KEY, up), (KEY, down), (KEY, left), (KEY, right)],
        };
        for source in dpad_sources {
            if !sources.insert(source) {
                return Err("duplicate D-pad input source".into());
            }
        }
        for button in [Button::L2, Button::R2] {
            let trigger = self
                .triggers
                .get(&button)
                .ok_or_else(|| format!("missing mapping for {button:?}"))?;
            if !sources.insert(trigger.source()) {
                return Err("duplicate trigger input source".into());
            }
            if let Trigger::Axis {
                rest,
                pressed,
                press_threshold,
                release_threshold,
                ..
            } = *trigger
                && (rest == pressed
                    || !press_threshold.is_finite()
                    || !release_threshold.is_finite()
                    || release_threshold < 0.0
                    || press_threshold > 1.0
                    || release_threshold >= press_threshold)
            {
                return Err("trigger endpoints must differ and 0 <= release_threshold < press_threshold <= 1".into());
            }
        }
        for stick in self.thumbsticks.values() {
            if !stick.press_threshold.is_finite()
                || !stick.release_threshold.is_finite()
                || stick.release_threshold < 0.0
                || stick.press_threshold > 1.0
                || stick.release_threshold >= stick.press_threshold
            {
                return Err(
                    "stick thresholds require 0 <= release_threshold < press_threshold <= 1".into(),
                );
            }
            for axis in [&stick.x, &stick.y] {
                if axis.minimum >= axis.center || axis.center >= axis.maximum {
                    return Err("stick axes require minimum < center < maximum".into());
                }
                if !sources.insert((ABS, axis.code)) {
                    return Err("duplicate thumbstick input source".into());
                }
            }
        }
        Ok(())
    }

    pub fn sources(&self) -> Vec<(u16, u16)> {
        let mut sources: Vec<_> = self.buttons.values().map(|&code| (KEY, code)).collect();
        sources.extend(self.menu_buttons.values().map(|&code| (KEY, code)));
        match self.dpad {
            DpadSource::Hat { x, y } => sources.extend([(ABS, x), (ABS, y)]),
            DpadSource::Buttons {
                up,
                down,
                left,
                right,
            } => sources.extend([(KEY, up), (KEY, down), (KEY, left), (KEY, right)]),
        }
        sources.extend(self.triggers.values().map(Trigger::source));
        for stick in self.thumbsticks.values() {
            sources.extend([(ABS, stick.x.code), (ABS, stick.y.code)]);
        }
        sources
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Snapshot {
    pub buttons: Vec<Button>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub menu_buttons: Vec<MenuButton>,
    pub dpad: Option<Dpad>,
    pub l2: f64,
    pub r2: f64,
    #[serde(skip_serializing_if = "Dpad::is_neutral")]
    pub left_stick: Dpad,
    #[serde(skip_serializing_if = "Dpad::is_neutral")]
    pub right_stick: Dpad,
}

impl Snapshot {
    pub fn is_neutral(&self) -> bool {
        self.dpad.is_some()
            && self.menu_buttons.is_empty()
            && self.input().controls() == 0
            && self.left_stick == Dpad::Neutral
            && self.right_stick == Dpad::Neutral
    }
    pub fn modifiers(&self) -> Vec<Modifier> {
        thumbstick_modifiers(Side::Left, self.left_stick)
            .into_iter()
            .chain(thumbstick_modifiers(Side::Right, self.right_stick))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
    pub fn input(&self) -> Input {
        Input {
            dpad: self.dpad.unwrap_or_default(),
            mask: self.buttons.iter().fold(0, |mask, b| mask | b.bit()),
        }
    }
}

pub fn thumbstick_modifiers(side: Side, direction: Dpad) -> Vec<Modifier> {
    let (x, y) = direction.position();
    let inward = if side == Side::Left { x > 0 } else { x < 0 };
    if inward {
        return match y {
            -1 => vec![
                Modifier::Ctrl,
                Modifier::Alt,
                Modifier::Shift,
                Modifier::Meta,
            ],
            1 => vec![Modifier::Ctrl, Modifier::Alt, Modifier::Meta],
            _ => vec![Modifier::Alt, Modifier::Meta],
        };
    }
    let mut modifiers = vec![];
    if x != 0 {
        modifiers.push(Modifier::Ctrl);
    }
    if y < 0 {
        modifiers.push(Modifier::Alt);
    }
    if y > 0 {
        modifiers.push(Modifier::Meta);
    }
    modifiers
}

pub struct Normalizer {
    pub calibration: Calibration,
    values: BTreeMap<(u16, u16), i32>,
    active: u8,
    sticks: [Dpad; 2],
}

impl Normalizer {
    pub fn new(calibration: Calibration) -> Result<Self> {
        calibration.validate()?;
        let mut values: BTreeMap<_, _> =
            calibration.sources().into_iter().map(|s| (s, 0)).collect();
        for trigger in calibration.triggers.values() {
            if let Trigger::Axis { code, rest, .. } = *trigger {
                values.insert((ABS, code), rest);
            }
        }
        for stick in calibration.thumbsticks.values() {
            for axis in [&stick.x, &stick.y] {
                values.insert((ABS, axis.code), axis.center);
            }
        }
        Ok(Self {
            calibration,
            values,
            active: 0,
            sticks: [Dpad::Neutral; 2],
        })
    }

    pub fn apply(&mut self, events: &[RawEvent]) -> Result<Snapshot> {
        for event in events {
            let source = (event.event_type, event.code);
            if !self.values.contains_key(&source) {
                continue;
            }
            if event.event_type == KEY {
                if event.value == 2 {
                    continue;
                }
                if event.value != 0 && event.value != 1 {
                    return Err("button values must be 0, 1, or repeat (2)".into());
                }
            }
            self.values.insert(source, event.value);
        }
        let key = |code| self.values.get(&(KEY, code)).copied().unwrap_or(0) != 0;
        let dpad = match self.calibration.dpad {
            DpadSource::Hat { x, y } => direction(self.values[&(ABS, x)], self.values[&(ABS, y)]),
            DpadSource::Buttons {
                up,
                down,
                left,
                right,
            } => {
                if key(up) && key(down) || key(left) && key(right) {
                    None
                } else {
                    direction(
                        i32::from(key(right)) - i32::from(key(left)),
                        i32::from(key(down)) - i32::from(key(up)),
                    )
                }
            }
        };
        let mut buttons: Vec<_> = self
            .calibration
            .buttons
            .iter()
            .filter(|(_, code)| key(**code))
            .map(|(&b, _)| b)
            .collect();
        let mut travel = [0.0; 2];
        for (index, button) in [Button::L2, Button::R2].into_iter().enumerate() {
            let trigger = &self.calibration.triggers[&button];
            let (value, active) = trigger.update(
                self.values[&trigger.source()],
                self.active & button.bit() != 0,
            );
            if active {
                self.active |= button.bit();
                buttons.push(button);
            } else {
                self.active &= !button.bit();
            }
            travel[index] = value;
        }
        for (side, stick) in &self.calibration.thumbsticks {
            self.sticks[side.index()] = stick.update(
                self.values[&(ABS, stick.x.code)],
                self.values[&(ABS, stick.y.code)],
                self.sticks[side.index()],
            );
        }
        buttons.sort();
        button_mask(&buttons)?;
        Ok(Snapshot {
            buttons,
            menu_buttons: self
                .calibration
                .menu_buttons
                .iter()
                .filter(|(_, code)| key(**code))
                .map(|(&button, _)| button)
                .collect(),
            dpad,
            l2: travel[0],
            r2: travel[1],
            left_stick: self.sticks[0],
            right_stick: self.sticks[1],
        })
    }
}

fn direction(x: i32, y: i32) -> Option<Dpad> {
    Dpad::ALL.into_iter().find(|d| {
        let (dx, dy) = d.position();
        i32::from(dx) == x && i32::from(dy) == y
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event(kind: u16, code: u16, value: i32) -> RawEvent {
        RawEvent {
            event_type: kind,
            code,
            value,
        }
    }

    #[test]
    fn synthetic_release_and_forced_report_boundaries_discard_pending_events() {
        let mut reader = ReportReader::default();
        reader.push(event(KEY, 304, 0));
        assert!(matches!(
            reader.push(event(0, 0, 1)),
            Some(Packet::StateReset)
        ));
        reader.push(event(KEY, 305, 1));
        assert!(
            matches!(reader.push(event(0, 0, 0)), Some(Packet::Frame(events)) if events == [event(KEY,305,1)])
        );
    }

    #[test]
    fn analog_triggers_have_hysteresis_and_support_reversed_endpoints() {
        let mut n = Normalizer::new(Calibration::default()).unwrap();
        for (raw, active) in [
            (160, false),
            (170, true),
            (128, true),
            (90, true),
            (89, false),
        ] {
            let state = n.apply(&[event(ABS, 2, raw)]).unwrap();
            assert_eq!(state.buttons.contains(&Button::L2), active);
        }
        let mut c = Calibration::default();
        c.triggers.insert(Button::L2, Trigger::axis(2, 255, 0));
        let mut n = Normalizer::new(c).unwrap();
        for (raw, active) in [(250, false), (64, true), (128, true), (200, false)] {
            assert_eq!(
                n.apply(&[event(ABS, 2, raw)])
                    .unwrap()
                    .buttons
                    .contains(&Button::L2),
                active
            );
        }
    }

    #[test]
    fn trigger_calibration_requires_travel_and_valid_thresholds() {
        let trigger = Trigger::axis(2, 0, 255);
        assert!(trigger.calibrated(0, 0, 0, 255.0).is_err());
        assert!(trigger.calibrated(0, 0, 10, 255.0).is_err());
        let reversed = trigger.calibrated(255, 0, 255, 255.0).unwrap();
        assert_eq!(reversed.update(0, false), (1.0, true));
        let mut c = Calibration::default();
        c.triggers.insert(
            Button::L2,
            Trigger::Axis {
                code: 2,
                rest: 0,
                pressed: 255,
                press_threshold: 0.2,
                release_threshold: 0.8,
            },
        );
        assert!(c.validate().is_err());
    }

    #[test]
    fn frames_are_atomic_order_independent_and_repeats_do_not_press_again() {
        let events = [
            event(KEY, 304, 1),
            event(KEY, 310, 1),
            event(ABS, 16, -1),
            event(ABS, 2, 200),
        ];
        let mut a = Normalizer::new(Calibration::default()).unwrap();
        let mut b = Normalizer::new(Calibration::default()).unwrap();
        let mut reversed = events;
        reversed.reverse();
        let state = a.apply(&events).unwrap();
        assert_eq!(state, b.apply(&reversed).unwrap());
        assert_eq!(state, a.apply(&[event(KEY, 304, 2)]).unwrap());
        assert_eq!(
            state,
            a.apply(&[event(KEY, 304, 0), event(KEY, 304, 1)]).unwrap()
        );
        assert!(
            !a.apply(&[event(KEY, 304, 1), event(KEY, 304, 0)])
                .unwrap()
                .buttons
                .contains(&Button::A)
        );
    }

    #[test]
    fn digital_dpad_opposites_are_invalid_and_dual_reports_are_not_combined() {
        let c = Calibration {
            dpad: DpadSource::Buttons {
                up: 544,
                down: 545,
                left: 546,
                right: 547,
            },
            ..Calibration::default()
        };
        let mut n = Normalizer::new(c).unwrap();
        assert_eq!(n.apply(&[event(KEY, 544, 1)]).unwrap().dpad, Some(Dpad::Up));
        assert_eq!(n.apply(&[event(KEY, 545, 1)]).unwrap().dpad, None);
        let mut n = Normalizer::new(Calibration::default()).unwrap();
        assert_eq!(
            n.apply(&[event(ABS, 17, -1), event(KEY, 545, 1)])
                .unwrap()
                .dpad,
            Some(Dpad::Up)
        );
    }

    #[test]
    fn digital_triggers_and_duplicate_mapping_validation() {
        let mut c = Calibration::default();
        c.triggers.insert(Button::R2, Trigger::Key { code: 313 });
        let mut n = Normalizer::new(c.clone()).unwrap();
        assert_eq!(n.apply(&[event(KEY, 313, 1)]).unwrap().r2, 1.0);
        assert_eq!(n.apply(&[event(KEY, 313, 0)]).unwrap().r2, 0.0);
        c.buttons.insert(Button::B, 304);
        assert!(c.validate().is_err());
    }

    #[test]
    fn menu_buttons_normalize_repeats_custom_codes_and_legacy_calibrations() {
        let mut calibration = Calibration::default();
        calibration.menu_buttons.insert(MenuButton::Start, 316);
        let mut n = Normalizer::new(calibration.clone()).unwrap();
        let start = n.apply(&[event(KEY, 316, 1)]).unwrap();
        assert_eq!(start.menu_buttons, [MenuButton::Start]);
        assert!(!start.is_neutral());
        assert_eq!(start.input(), Input::default());
        assert_eq!(n.apply(&[event(KEY, 316, 2)]).unwrap(), start);
        assert_eq!(
            n.apply(&[event(KEY, 314, 1)]).unwrap().menu_buttons,
            [MenuButton::Start, MenuButton::Select]
        );
        assert!(
            n.apply(&[event(KEY, 316, 0), event(KEY, 314, 0)])
                .unwrap()
                .is_neutral()
        );
        calibration.menu_buttons.insert(MenuButton::Select, 316);
        assert!(calibration.validate().is_err());
        calibration.menu_buttons.insert(MenuButton::Select, 304);
        assert!(calibration.validate().is_err());
        let mut old = serde_json::to_value(Calibration::default()).unwrap();
        old.as_object_mut().unwrap().remove("menu_buttons");
        let old: Calibration = serde_json::from_value(old).unwrap();
        assert!(old.menu_buttons.is_empty());
        assert!(
            Normalizer::new(old)
                .unwrap()
                .apply(&[event(KEY, 315, 1)])
                .unwrap()
                .is_neutral()
        );
    }

    #[test]
    fn dropped_reports_discard_partial_input_until_the_sync_boundary() {
        let mut r = ReportReader::default();
        assert!(r.push(event(KEY, 304, 1)).is_none());
        assert!(matches!(r.push(event(0, 3, 0)), Some(Packet::Dropped)));
        assert!(r.push(event(KEY, 310, 1)).is_none());
        assert!(matches!(r.push(event(0, 0, 0)), Some(Packet::Resync)));
        r.push(event(KEY, 305, 1));
        let Some(Packet::Frame(events)) = r.push(event(0, 0, 0)) else {
            panic!("expected a complete report");
        };
        assert_eq!(events, [event(KEY, 305, 1)]);
    }
}

#[cfg(test)]
mod stick_tests {
    use super::*;

    fn axis(code: u16) -> StickAxis {
        StickAxis {
            code,
            minimum: -1000,
            center: 0,
            maximum: 1000,
            inverted: false,
        }
    }
    fn calibration() -> Calibration {
        Calibration {
            schema_version: 2,
            thumbsticks: [
                (Side::Left, Thumbstick::new(axis(0), axis(1))),
                (Side::Right, Thumbstick::new(axis(3), axis(4))),
            ]
            .into(),
            ..Calibration::default()
        }
    }
    #[test]
    fn every_direction_has_the_requested_mirrored_modifiers() {
        use Dpad::*;
        use Modifier::*;
        for side in Side::ALL {
            for direction in Dpad::ALL {
                let expected = match (side, direction) {
                    (_, Neutral) => vec![],
                    (_, Up) => vec![Alt],
                    (_, Down) => vec![Meta],
                    (Side::Left, UpRight) | (Side::Right, UpLeft) => vec![Ctrl, Alt, Shift, Meta],
                    (Side::Left, DownRight) | (Side::Right, DownLeft) => vec![Ctrl, Alt, Meta],
                    (Side::Left, Right) | (Side::Right, Left) => vec![Alt, Meta],
                    (_, Left | Right) => vec![Ctrl],
                    (_, UpLeft | UpRight) => vec![Ctrl, Alt],
                    (_, DownLeft | DownRight) => vec![Ctrl, Meta],
                };
                let mut n = Normalizer::new(calibration()).unwrap();
                let (x, y) = direction.position();
                let first = if side == Side::Left { 0 } else { 3 };
                let events = [
                    RawEvent {
                        event_type: ABS,
                        code: first,
                        value: i32::from(x) * 1000,
                    },
                    RawEvent {
                        event_type: ABS,
                        code: first + 1,
                        value: i32::from(y) * 1000,
                    },
                ];
                assert_eq!(
                    n.apply(&events).unwrap().modifiers(),
                    expected,
                    "{side:?} {direction:?}"
                );
            }
        }
    }
    #[test]
    fn stick_hysteresis_centers_inversion_and_two_stick_unions_work() {
        use Modifier::*;
        let mut n = Normalizer::new(calibration()).unwrap();
        assert!(n.apply(&[]).unwrap().is_neutral());
        for (raw, expected) in [
            (540, vec![]),
            (560, vec![Alt, Meta]),
            (400, vec![Alt, Meta]),
            (350, vec![]),
            (-600, vec![Ctrl]),
            (-400, vec![Ctrl]),
            (0, vec![]),
        ] {
            assert_eq!(
                n.apply(&[RawEvent {
                    event_type: ABS,
                    code: 0,
                    value: raw
                }])
                .unwrap()
                .modifiers(),
                expected
            );
        }
        let state = n
            .apply(&[
                RawEvent {
                    event_type: ABS,
                    code: 1,
                    value: -1000,
                },
                RawEvent {
                    event_type: ABS,
                    code: 4,
                    value: 1000,
                },
            ])
            .unwrap();
        assert_eq!(state.modifiers(), [Alt, Meta]);
        let mut c = calibration();
        let axis = &mut c.thumbsticks.get_mut(&Side::Left).unwrap().x;
        axis.minimum = 0;
        axis.center = 32768;
        axis.maximum = 65535;
        axis.inverted = true;
        let mut n = Normalizer::new(c).unwrap();
        assert!(n.apply(&[]).unwrap().is_neutral());
        assert_eq!(
            n.apply(&[RawEvent {
                event_type: ABS,
                code: 0,
                value: 0
            }])
            .unwrap()
            .modifiers(),
            [Alt, Meta]
        );
    }
    #[test]
    fn invalid_stick_calibrations_are_rejected_and_old_mappings_remain_valid() {
        for kind in 0..4 {
            let mut c = calibration();
            let stick = c.thumbsticks.get_mut(&Side::Left).unwrap();
            match kind {
                0 => stick.x.center = 1000,
                1 => stick.y.code = 0,
                2 => stick.press_threshold = 0.2,
                _ => stick.x.code = 2,
            }
            assert!(c.validate().is_err());
        }
        let c: Calibration =
            serde_json::from_str(&serde_json::to_string(&Calibration::default()).unwrap()).unwrap();
        assert!(c.thumbsticks.is_empty());
        assert!(Normalizer::new(c).unwrap().apply(&[]).unwrap().is_neutral());
    }
}
