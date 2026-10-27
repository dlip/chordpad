use crate::layers::HeldLayers;
use crate::{
    Result,
    chord::{Engine as HalfEngine, Event, MenuKeys, Phase},
    controller::Snapshot,
    model::*,
};
use std::collections::VecDeque;
use std::time::Instant;

const MAX_PENDING: usize = 64;

struct Half {
    engine: HalfEngine,
    pending: Option<u64>,
    function: bool,
}

impl Half {
    fn new(strategy: &Strategy) -> Result<Self> {
        let mut strategy = strategy.clone();
        strategy.independent = false;
        Ok(Self {
            engine: HalfEngine::new(strategy, Profile::default(), None)?,
            pending: None,
            function: false,
        })
    }
}

struct Pending {
    id: u64,
    event: Option<Event>,
}

pub struct Engine {
    layout: Layout,
    halves: [Half; 2],
    queue: VecDeque<Pending>,
    waiting: bool,
    next_id: u64,
    menu: MenuKeys,
    menu_pending: [Option<u64>; MenuButton::ALL.len()],
    layers: HeldLayers,
    started: Instant,
}

impl Engine {
    pub fn new(layout: Layout) -> Result<Self> {
        layout.validate()?;
        if !layout.strategy.independent {
            return Err("independent recognizer requires an independent layout".into());
        }
        Ok(Self {
            halves: [Half::new(&layout.strategy)?, Half::new(&layout.strategy)?],
            layout,
            queue: VecDeque::new(),
            waiting: true,
            next_id: 0,
            menu: MenuKeys::default(),
            menu_pending: [None; MenuButton::ALL.len()],
            layers: HeldLayers::default(),
            started: Instant::now(),
        })
    }

    pub fn phase(&self) -> Phase {
        if self.waiting {
            return Phase::Waiting;
        }
        for phase in [Phase::Collecting, Phase::Releasing, Phase::Cancelled] {
            if self.halves.iter().any(|h| h.engine.phase == phase)
                || phase == Phase::Collecting && self.menu.collecting()
            {
                return phase;
            }
        }
        Phase::Idle
    }

    pub fn reset(&mut self, reason: &str) -> Event {
        let discarded = !self.queue.is_empty();
        self.queue.clear();
        self.menu = MenuKeys::default();
        self.menu_pending = [None; MenuButton::ALL.len()];
        self.layers = HeldLayers::default();
        self.waiting = true;
        for half in &mut self.halves {
            half.engine.reset(reason);
            half.pending = None;
        }
        Event::Reset {
            reason: reason.into(),
            discarded,
        }
    }

    fn complete(&mut self, side: Side, event: Event) -> Option<Event> {
        if let Some(id) = self.halves[side.index()].pending.take() {
            self.queue.iter_mut().find(|p| p.id == id).unwrap().event = Some(event);
            None
        } else {
            Some(event)
        }
    }

    fn resolve(
        &self,
        input: Input,
        mut held: Vec<Button>,
        mut keyboard_modifiers: Vec<Modifier>,
        function: bool,
    ) -> Event {
        let context = Input {
            mask: button_mask(&held).unwrap(),
            dpad: Dpad::Neutral,
        };
        if let Err(reason) = validate_independent_input(&self.layout.profile, input, context) {
            return Event::Cancelled { reason };
        }
        if function {
            held.push(Button::R1);
            held.sort();
            if !self.layout.profile.allows(Input {
                mask: input.mask | button_mask(&held).unwrap(),
                ..input
            }) {
                return Event::Cancelled {
                    reason: "function chord or held context is forbidden by the profile".into(),
                };
            }
            if held.contains(&Button::L2) {
                keyboard_modifiers.push(Modifier::Shift);
                keyboard_modifiers.sort();
                keyboard_modifiers.dedup();
            }
            let action = self
                .layout
                .function_bindings()
                .into_iter()
                .find(|b| b.chord.input().ok() == Some(input))
                .map(|b| b.action);
            if matches!(action, Some(Action::Key(key)) if key.is_consumer()) {
                keyboard_modifiers.clear();
            }
            return match action {
                Some(action) => Event::Chord {
                    chord: input.chord(),
                    held,
                    held_dpad: Dpad::Neutral,
                    keyboard_modifiers,
                    action: Some(action),
                },
                None => Event::Unmapped {
                    chord: input.chord(),
                    held,
                    held_dpad: Dpad::Neutral,
                    keyboard_modifiers,
                },
            };
        }
        let fixed = independent_fixed_action(input);
        if fixed.is_some() && held.contains(&Button::L2) {
            keyboard_modifiers.push(Modifier::Shift);
            keyboard_modifiers.sort();
            keyboard_modifiers.dedup();
        }
        let binding = self.layout.bindings.iter().find(|b| {
            b.chord.input().ok() == Some(input)
                && if let Some(action) = &fixed {
                    b.action == *action && b.held.is_empty()
                } else {
                    b.context().ok() == Some(context)
                }
        });
        let chord = input.chord();
        match binding {
            Some(binding) => Event::Chord {
                keyboard_modifiers: keyboard_modifiers.clone(),
                chord,
                held,
                held_dpad: Dpad::Neutral,
                action: Some(binding.action.clone()),
            },
            None => Event::Unmapped {
                keyboard_modifiers,
                chord,
                held,
                held_dpad: Dpad::Neutral,
            },
        }
    }

    pub fn update(&mut self, snapshot: &Snapshot) -> Vec<Event> {
        self.update_at(snapshot, self.started.elapsed().as_millis() as u64)
    }

    pub fn repeat(&mut self) -> Vec<Event> {
        self.repeat_at(self.started.elapsed().as_millis() as u64)
    }

    pub fn repeat_at(&mut self, now: u64) -> Vec<Event> {
        if self.waiting || !self.queue.is_empty() {
            return vec![];
        }
        self.layers.repeat(now)
    }

    pub fn update_at(&mut self, snapshot: &Snapshot, now: u64) -> Vec<Event> {
        let input = snapshot.input();
        if snapshot.dpad.is_none() {
            return vec![self.reset("invalid D-pad position")];
        }
        if self.waiting {
            if !snapshot.is_neutral() {
                return vec![];
            }
            self.waiting = false;
            for half in &mut self.halves {
                half.engine.update(snapshot);
            }
            return vec![Event::Ready];
        }
        let change = if self.layout.profile.navigation_layer || self.layout.profile.function_layer {
            self.layers.update(snapshot, &self.layout.profile, now)
        } else {
            crate::layers::Change {
                changed: false,
                function: false,
                input,
                events: vec![],
            }
        };
        if change.changed {
            for side in Side::ALL {
                let _ = self.complete(
                    side,
                    Event::Cancelled {
                        reason: "layer changed during a stroke".into(),
                    },
                );
                self.halves[side.index()].engine.reset("layer changed");
                self.halves[side.index()].engine.update(&Snapshot {
                    buttons: vec![],
                    menu_buttons: vec![],
                    dpad: Some(Dpad::Neutral),
                    l2: 0.0,
                    r2: 0.0,
                    left_stick: Dpad::Neutral,
                    right_stick: Dpad::Neutral,
                });
            }
        }
        let input = change.input;
        let modifiers = input.mask & self.layout.strategy.modifier_mask();
        let mut events = vec![];
        // Same-report starts are ordered left, then right.
        for side in Side::ALL {
            let i = side.index();
            let local = Input {
                mask: input.mask & !self.layout.strategy.modifier_mask(),
                dpad: input.dpad,
            }
            .half(side);
            if self.halves[i].engine.phase == Phase::Idle && local.controls() != 0 {
                if self.queue.len() == MAX_PENDING {
                    return vec![self.reset("too many pending strokes")];
                }
                let id = self.next_id;
                self.next_id = self.next_id.wrapping_add(1);
                self.queue.push_back(Pending { id, event: None });
                self.halves[i].pending = Some(id);
                self.halves[i].function = change.function;
            }
            let local_snapshot = Snapshot {
                menu_buttons: vec![],
                buttons: Input {
                    mask: local.mask | modifiers,
                    ..local
                }
                .chord()
                .buttons,
                dpad: Some(local.dpad),
                l2: snapshot.l2,
                r2: snapshot.r2,
                left_stick: snapshot.left_stick,
                right_stick: snapshot.right_stick,
            };
            for event in self.halves[i].engine.update(&local_snapshot) {
                let event = match event {
                    Event::Chord {
                        chord,
                        held,
                        keyboard_modifiers,
                        ..
                    } => self.resolve(
                        chord.input().unwrap(),
                        held,
                        keyboard_modifiers,
                        self.halves[i].function,
                    ),
                    Event::Ready => continue,
                    other => other,
                };
                if let Some(event) = self.complete(side, event) {
                    events.push(event);
                }
            }
            if side == Side::Left {
                for event in &change.events {
                    if self.queue.len() == MAX_PENDING {
                        return vec![self.reset("too many pending strokes")];
                    }
                    let id = self.next_id;
                    self.next_id = self.next_id.wrapping_add(1);
                    self.queue.push_back(Pending {
                        id,
                        event: Some(event.clone()),
                    });
                }
            }
        }
        for change in self.menu.update(snapshot, self.layout.strategy.shift) {
            let pending = &mut self.menu_pending[change.button.index()];
            if change.started {
                if self.queue.len() == MAX_PENDING {
                    return vec![self.reset("too many pending strokes")];
                }
                let id = self.next_id;
                self.next_id = self.next_id.wrapping_add(1);
                self.queue.push_back(Pending { id, event: None });
                *pending = Some(id);
            }
            if let Some(event) = change.completed {
                let id = pending.take().unwrap();
                self.queue.iter_mut().find(|p| p.id == id).unwrap().event = Some(event);
            }
        }
        while self.queue.front().is_some_and(|p| p.event.is_some()) {
            events.push(self.queue.pop_front().unwrap().event.unwrap());
        }
        events
    }
}
