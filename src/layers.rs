use crate::{chord::Event, controller::Snapshot, model::*};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Mode {
    #[default]
    Normal,
    Navigation,
    Function,
    Ambiguous,
}

struct Repeating {
    key: Key,
    event: Event,
    at: u64,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum FacePhase {
    #[default]
    Idle,
    Collecting,
    Releasing,
    Cancelled,
}

#[derive(Default)]
struct NavigationFace {
    phase: FacePhase,
    previous: u8,
    frozen: u8,
    held: u8,
    modifiers: Vec<Modifier>,
    previous_held: u8,
    previous_modifiers: Vec<Modifier>,
    repeated: bool,
    repeating: Option<Repeating>,
}

impl NavigationFace {
    fn event(
        mask: u8,
        held: u8,
        modifiers: &[Modifier],
        profile: &Profile,
    ) -> Option<(Key, Event)> {
        if !profile.allows(Input {
            mask: mask | held,
            dpad: Dpad::Neutral,
        }) {
            return None;
        }
        let key = profile
            .navigation_face_keys()
            .into_iter()
            .find(|(buttons, _)| *buttons == mask)?
            .1;
        let mut modifiers = if key.is_consumer() {
            vec![]
        } else {
            modifiers.to_vec()
        };
        if !key.is_consumer() && held & Button::L2.bit() != 0 {
            modifiers.push(Modifier::Shift);
            modifiers.sort();
            modifiers.dedup();
        }
        Some((
            key,
            Event::Chord {
                chord: Input {
                    mask,
                    dpad: Dpad::Neutral,
                }
                .chord(),
                held: Input {
                    mask: held,
                    dpad: Dpad::Neutral,
                }
                .chord()
                .buttons,
                held_dpad: Dpad::Neutral,
                keyboard_modifiers: modifiers,
                action: Some(Action::Key(key)),
            },
        ))
    }

    fn update(
        &mut self,
        mask: u8,
        held: u8,
        modifiers: Vec<Modifier>,
        profile: &Profile,
        now: u64,
    ) -> Vec<Event> {
        let new_modifier = held & !self.previous_held != 0
            || modifiers
                .iter()
                .any(|m| !self.previous_modifiers.contains(m));
        self.previous_held = held;
        self.previous_modifiers = modifiers.clone();
        let mut events = vec![];
        match self.phase {
            FacePhase::Idle if mask != 0 => {
                self.phase = FacePhase::Collecting;
                self.held = held;
                self.modifiers = modifiers;
                self.repeated = false;
            }
            FacePhase::Collecting | FacePhase::Releasing => {
                let added = mask & !self.previous != 0;
                let released = self.previous & !mask != 0;
                if new_modifier
                    || (added && (released || self.phase == FacePhase::Releasing || self.repeated))
                {
                    self.phase = FacePhase::Cancelled;
                } else {
                    if self.phase == FacePhase::Collecting && released {
                        self.frozen = self.previous;
                        self.phase = FacePhase::Releasing;
                    }
                    if self.phase == FacePhase::Releasing && mask == 0 {
                        if !self.repeated
                            && let Some((_, event)) =
                                Self::event(self.frozen, self.held, &self.modifiers, profile)
                        {
                            events.push(event);
                        }
                        self.phase = FacePhase::Idle;
                    }
                }
            }
            _ => {}
        }
        if self.phase == FacePhase::Cancelled && mask == 0 {
            self.phase = FacePhase::Idle;
        }
        if self.phase != FacePhase::Collecting || !mask.is_power_of_two() {
            self.repeating = None;
        } else if self.repeating.is_none()
            && let Some((key, event)) = Self::event(mask, self.held, &self.modifiers, profile)
        {
            self.repeating = Some(Repeating {
                key,
                event,
                at: now.saturating_add(400),
            });
        }
        self.previous = mask;
        events
    }

    fn repeat(&mut self, now: u64) -> Vec<Event> {
        if let Some(repeating) = &mut self.repeating
            && now >= repeating.at
        {
            self.repeated = true;
            repeating.at = now.saturating_add(80);
            vec![repeating.event.clone()]
        } else {
            vec![]
        }
    }
}

#[derive(Default)]
pub(crate) struct HeldLayers {
    mode: Mode,
    previous: Input,
    blocked_buttons: u8,
    blocked_dpad: bool,
    active: Vec<Repeating>,
    face: NavigationFace,
}

pub(crate) struct Change {
    pub changed: bool,
    pub function: bool,
    pub input: Input,
    pub events: Vec<Event>,
}

impl HeldLayers {
    pub fn update(&mut self, snapshot: &Snapshot, profile: &Profile, now: u64) -> Change {
        let input = snapshot.input();
        let r2 = input.mask & Button::R2.bit() != 0;
        let navigation = profile.navigation_layer && r2 && input.mask & Button::L1.bit() != 0;
        let function = profile.function_layer && r2 && input.mask & Button::R1.bit() != 0;
        let mode = match (navigation, function) {
            (true, true) => Mode::Ambiguous,
            (true, false) => Mode::Navigation,
            (false, true) => Mode::Function,
            _ => Mode::Normal,
        };
        let changed = mode != self.mode;
        let function_exit = self.mode == Mode::Function && mode == Mode::Normal;
        if changed {
            self.face = NavigationFace::default();
        }
        self.blocked_buttons &= input.mask;
        if input.dpad == Dpad::Neutral {
            self.blocked_dpad = false;
        }
        if changed && !function_exit {
            self.blocked_buttons |= self.previous.mask & input.mask & 0x3f;
            self.blocked_dpad = self.previous.dpad != Dpad::Neutral && input.dpad != Dpad::Neutral;
            if self.mode != Mode::Normal {
                self.blocked_buttons |= input.mask & 0x0f;
                self.blocked_dpad = input.dpad != Dpad::Neutral;
            }
        }
        match mode {
            Mode::Navigation => self.blocked_buttons |= Button::L1.bit(),
            Mode::Function => self.blocked_buttons |= Button::R1.bit(),
            Mode::Ambiguous => self.blocked_buttons |= Button::L1.bit() | Button::R1.bit(),
            Mode::Normal => {}
        }
        self.mode = mode;
        self.previous = input;
        let mut events = vec![];
        let mut active = vec![];
        if mode == Mode::Navigation {
            let held_mask = (Button::L1.bit() | Button::R2.bit()) | (input.mask & Button::L2.bit());
            let mut modifiers = snapshot.modifiers();
            if input.mask & Button::L2.bit() != 0 {
                modifiers.push(Modifier::Shift);
                modifiers.sort();
            }
            let direction = if self.blocked_dpad {
                Dpad::Neutral
            } else {
                input.dpad
            };
            let arrow = match direction {
                Dpad::Up => Some(Key::Up),
                Dpad::Down => Some(Key::Down),
                Dpad::Left => Some(Key::Left),
                Dpad::Right => Some(Key::Right),
                _ => None,
            };
            let mut controls = vec![];
            if let Some(key) = arrow {
                controls.push((
                    Input {
                        mask: 0,
                        dpad: direction,
                    },
                    key,
                ));
            }
            if profile.navigation_media {
                events.extend(self.face.update(
                    input.mask & !self.blocked_buttons & 0x0f,
                    held_mask,
                    snapshot.modifiers(),
                    profile,
                    now,
                ));
            }
            for (button, key) in [
                (Button::A, Key::PageDown),
                (Button::B, Key::End),
                (Button::X, Key::Home),
                (Button::Y, Key::PageUp),
            ] {
                if !profile.navigation_media
                    && input.mask & !self.blocked_buttons & button.bit() != 0
                {
                    controls.push((
                        Input {
                            mask: button.bit(),
                            dpad: Dpad::Neutral,
                        },
                        key,
                    ));
                }
            }
            for (control, key) in controls {
                if !profile.allows(Input {
                    mask: control.mask | held_mask,
                    ..control
                }) {
                    continue;
                }
                let event = Event::Chord {
                    chord: control.chord(),
                    held: Input {
                        mask: held_mask,
                        dpad: Dpad::Neutral,
                    }
                    .chord()
                    .buttons,
                    held_dpad: Dpad::Neutral,
                    keyboard_modifiers: modifiers.clone(),
                    action: Some(Action::Key(key)),
                };
                let at = match self.active.iter().find(|r| r.key == key) {
                    Some(previous) => previous.at,
                    None => {
                        events.push(event.clone());
                        now.saturating_add(400)
                    }
                };
                active.push(Repeating { key, event, at });
            }
        }
        self.active = active;
        Change {
            changed: changed && !function_exit,
            function: mode == Mode::Function,
            events,
            input: Input {
                mask: input.mask
                    & !self.blocked_buttons
                    & match mode {
                        Mode::Navigation => !0x0f,
                        Mode::Ambiguous => !0x3f,
                        _ => u8::MAX,
                    },
                dpad: if matches!(mode, Mode::Navigation | Mode::Ambiguous) || self.blocked_dpad {
                    Dpad::Neutral
                } else {
                    input.dpad
                },
            },
        }
    }

    pub fn repeat(&mut self, now: u64) -> Vec<Event> {
        let mut events = vec![];
        for active in &mut self.active {
            if now >= active.at {
                events.push(active.event.clone());
                active.at = now.saturating_add(80);
            }
        }
        events.extend(self.face.repeat(now));
        events
    }
}
