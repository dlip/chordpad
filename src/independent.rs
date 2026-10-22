use crate::{
    Result,
    chord::{Engine as HalfEngine, Event, Phase},
    controller::Snapshot,
    model::*,
};
use std::collections::VecDeque;

const MAX_PENDING: usize = 64;

struct Half {
    engine: HalfEngine,
    pending: Option<u64>,
}

impl Half {
    fn new(strategy: &Strategy) -> Result<Self> {
        let mut strategy = strategy.clone();
        strategy.independent = false;
        Ok(Self {
            engine: HalfEngine::new(strategy, Profile::default(), None)?,
            pending: None,
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
        })
    }

    pub fn phase(&self) -> Phase {
        if self.waiting {
            return Phase::Waiting;
        }
        for phase in [Phase::Collecting, Phase::Releasing, Phase::Cancelled] {
            if self.halves.iter().any(|h| h.engine.phase == phase) {
                return phase;
            }
        }
        Phase::Idle
    }

    pub fn reset(&mut self, reason: &str) -> Event {
        let discarded = !self.queue.is_empty();
        self.queue.clear();
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
        held: Vec<Button>,
        mut keyboard_modifiers: Vec<Modifier>,
    ) -> Event {
        let context = Input {
            mask: button_mask(&held).unwrap(),
            dpad: Dpad::Neutral,
        };
        if let Err(reason) = validate_independent_input(&self.layout.profile, input, context) {
            return Event::Cancelled { reason };
        }
        let fixed = independent_fixed_action(input);
        if fixed.is_some() && held.contains(&Button::L2) {
            keyboard_modifiers.push(Modifier::Shift);
            keyboard_modifiers.sort();
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
        let modifiers = input.mask & self.layout.strategy.modifier_mask();
        let mut events = vec![];
        // Same-report starts are ordered left, then right.
        for side in Side::ALL {
            let i = side.index();
            let local = Input {
                mask: input.mask & !self.layout.strategy.modifier_mask(),
                ..input
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
            }
            let local_snapshot = Snapshot {
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
                    } => self.resolve(chord.input().unwrap(), held, keyboard_modifiers),
                    Event::Ready => continue,
                    other => other,
                };
                if let Some(event) = self.complete(side, event) {
                    events.push(event);
                }
            }
        }
        while self.queue.front().is_some_and(|p| p.event.is_some()) {
            events.push(self.queue.pop_front().unwrap().event.unwrap());
        }
        events
    }
}
