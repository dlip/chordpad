use crate::{Result, controller::Snapshot, model::*};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Waiting,
    Idle,
    Collecting,
    Releasing,
    Cancelled,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Ready,
    Chord {
        chord: Chord,
        held: Vec<Button>,
        action: Option<Action>,
    },
    Unmapped {
        chord: Chord,
        held: Vec<Button>,
    },
    Cancelled {
        reason: String,
    },
    Reset {
        reason: String,
        discarded: bool,
    },
}

pub struct Engine {
    pub phase: Phase,
    strategy: Strategy,
    profile: Profile,
    layout: Option<Layout>,
    previous: Input,
    previous_buttons: u8,
    held: u8,
    frozen: Option<Input>,
}

impl Engine {
    pub fn new(strategy: Strategy, profile: Profile, layout: Option<Layout>) -> Result<Self> {
        Config {
            profile: profile.clone(),
            strategies: vec![strategy.clone()],
            ..Config::default()
        }
        .validate()?;
        if let Some(layout) = &layout {
            layout.validate()?;
            if layout.strategy != strategy || layout.profile != profile {
                return Err("layout and recognizer configuration differ".into());
            }
        }
        Ok(Self {
            phase: Phase::Waiting,
            strategy,
            profile,
            layout,
            previous: Input {
                dpad: Dpad::Neutral,
                mask: 0,
            },
            previous_buttons: 0,
            held: 0,
            frozen: None,
        })
    }

    pub fn reset(&mut self, reason: &str) -> Event {
        let discarded = matches!(self.phase, Phase::Collecting | Phase::Releasing);
        self.phase = Phase::Waiting;
        self.previous = Input {
            dpad: Dpad::Neutral,
            mask: 0,
        };
        self.previous_buttons = 0;
        self.held = 0;
        self.frozen = None;
        Event::Reset {
            reason: reason.into(),
            discarded,
        }
    }

    pub fn update(&mut self, snapshot: &Snapshot) -> Vec<Event> {
        let physical = snapshot.input();
        if self.phase == Phase::Waiting {
            self.previous_buttons = physical.mask;
            if snapshot.dpad.is_some() && physical.controls() == 0 {
                self.phase = Phase::Idle;
                return vec![Event::Ready];
            }
            return vec![];
        }
        if snapshot.dpad.is_none() {
            return self.cancel("invalid D-pad position");
        }
        let excluded = self.strategy.modifier_mask()
            | self
                .profile
                .modifier_only_buttons
                .iter()
                .fold(0, |m, b| m | b.bit());
        let current = Input {
            mask: physical.mask & !excluded,
            ..physical
        };
        let new_modifier =
            (physical.mask & !self.previous_buttons) & self.strategy.modifier_mask() != 0;
        self.previous_buttons = physical.mask;
        let result = match self.phase {
            Phase::Idle => {
                if current.controls() != 0 {
                    self.held = physical.mask & self.strategy.modifier_mask();
                    self.phase = Phase::Collecting;
                }
                vec![]
            }
            Phase::Cancelled => {
                if current.controls() == 0 {
                    self.phase = Phase::Idle;
                }
                vec![]
            }
            Phase::Collecting | Phase::Releasing => {
                let (px, py) = self.previous.dpad.position();
                let (cx, cy) = current.dpad.position();
                if new_modifier {
                    self.cancel("modifier pressed during a stroke")
                } else if px * cx < 0 || py * cy < 0 {
                    self.cancel("D-pad moved directly between opposite directions")
                } else {
                    let added = current.mask & !self.previous.mask != 0
                        || direction_bits(current.dpad) & !direction_bits(self.previous.dpad) != 0;
                    let released = self.previous.mask & !current.mask != 0
                        || direction_bits(self.previous.dpad) & !direction_bits(current.dpad) != 0;
                    if self.phase == Phase::Releasing && added
                        || self.phase == Phase::Collecting && released && added
                    {
                        self.cancel("new control pressed during release")
                    } else {
                        if self.phase == Phase::Collecting && released {
                            self.frozen = Some(self.previous);
                            self.phase = Phase::Releasing;
                        }
                        if self.phase == Phase::Releasing && current.controls() == 0 {
                            vec![self.commit()]
                        } else {
                            vec![]
                        }
                    }
                }
            }
            Phase::Waiting => unreachable!(),
        };
        if self.phase == Phase::Cancelled && current.controls() == 0 {
            self.phase = Phase::Idle;
        }
        self.previous = current;
        result
    }

    fn cancel(&mut self, reason: &str) -> Vec<Event> {
        if self.phase == Phase::Cancelled {
            return vec![];
        }
        self.phase = Phase::Cancelled;
        self.frozen = None;
        vec![Event::Cancelled {
            reason: reason.into(),
        }]
    }

    fn commit(&mut self) -> Event {
        let input = self.frozen.take().unwrap();
        self.phase = Phase::Idle;
        if !self.profile.allows(Input {
            mask: input.mask | self.held,
            ..input
        }) {
            return Event::Cancelled {
                reason: "chord is forbidden by the layout profile".into(),
            };
        }
        let held: Vec<_> = Button::ALL
            .into_iter()
            .filter(|b| self.held & b.bit() != 0)
            .collect();
        let chord = input.chord();
        let action = self
            .layout
            .as_ref()
            .and_then(|layout| {
                layout.bindings.iter().find(|b| {
                    b.chord.input().ok() == Some(input)
                        && button_mask(&b.held).ok() == Some(self.held)
                })
            })
            .map(|b| b.action.clone());
        if self.layout.is_some() && action.is_none() {
            Event::Unmapped { chord, held }
        } else {
            Event::Chord {
                chord,
                held,
                action,
            }
        }
    }
}

fn direction_bits(dpad: Dpad) -> u8 {
    let (x, y) = dpad.position();
    u8::from(x < 0) | (u8::from(x > 0) << 1) | (u8::from(y < 0) << 2) | (u8::from(y > 0) << 3)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn state(buttons: &[Button], dpad: Dpad) -> Snapshot {
        Snapshot {
            buttons: buttons.into(),
            dpad: Some(dpad),
            l2: 0.0,
            r2: 0.0,
        }
    }
    fn engine() -> Engine {
        Engine::new(
            Strategy {
                name: "test".into(),
                shift: Some(Button::L2),
                layer: Some(Button::R2),
            },
            Profile::default(),
            None,
        )
        .unwrap()
    }
    fn update(engine: &mut Engine, buttons: &[Button]) -> Vec<Event> {
        engine.update(&state(buttons, Dpad::Neutral))
    }
    fn arm(engine: &mut Engine) {
        assert_eq!(update(engine, &[]), [Event::Ready]);
    }
    fn permutations() -> [[Button; 3]; 6] {
        use Button::*;
        [
            [A, L1, R1],
            [A, R1, L1],
            [L1, A, R1],
            [L1, R1, A],
            [R1, A, L1],
            [R1, L1, A],
        ]
    }

    #[test]
    fn every_press_and_release_order_emits_the_full_chord_once() {
        for presses in permutations() {
            for releases in permutations() {
                let mut e = engine();
                arm(&mut e);
                let mut down = vec![];
                for button in presses {
                    down.push(button);
                    assert!(update(&mut e, &down).is_empty());
                }
                for (index, button) in releases.into_iter().enumerate() {
                    down.retain(|&b| b != button);
                    let events = update(&mut e, &down);
                    if index < 2 {
                        assert!(events.is_empty());
                    } else {
                        assert_eq!(
                            events,
                            [Event::Chord {
                                chord: Chord {
                                    dpad: Dpad::Neutral,
                                    buttons: vec![Button::A, Button::L1, Button::R1]
                                },
                                held: vec![],
                                action: None
                            }]
                        );
                    }
                }
                assert!(update(&mut e, &[]).is_empty());
            }
        }
    }

    #[test]
    fn startup_and_reset_require_full_neutral_and_discard_pending_strokes() {
        let mut e = engine();
        assert!(update(&mut e, &[Button::A]).is_empty());
        assert_eq!(e.phase, Phase::Waiting);
        arm(&mut e);
        for reason in ["disconnect", "resync", "pause"] {
            update(&mut e, &[Button::A]);
            assert_eq!(
                e.reset(reason),
                Event::Reset {
                    reason: reason.into(),
                    discarded: true
                }
            );
            assert!(update(&mut e, &[Button::L2]).is_empty());
            arm(&mut e);
            update(&mut e, &[Button::B]);
            assert!(matches!(
                update(&mut e, &[]).as_slice(),
                [Event::Chord { .. }]
            ));
        }
    }

    #[test]
    fn held_modifier_context_is_latched_and_can_persist_across_strokes() {
        let mut e = engine();
        arm(&mut e);
        update(&mut e, &[Button::L2]);
        for button in [Button::A, Button::B] {
            update(&mut e, &[Button::L2, button]);
            let events = update(&mut e, &[Button::L2]);
            assert!(matches!(events.as_slice(),[Event::Chord{held,..}] if held==&[Button::L2]));
        }
        update(&mut e, &[Button::L2, Button::A]);
        update(&mut e, &[Button::A]);
        assert!(
            matches!(update(&mut e,&[]).as_slice(),[Event::Chord{held,..}] if held==&[Button::L2])
        );
    }

    #[test]
    fn late_or_repressed_modifiers_cancel_the_stroke() {
        let mut e = engine();
        arm(&mut e);
        update(&mut e, &[Button::A]);
        assert!(matches!(
            update(&mut e, &[Button::A, Button::L2]).as_slice(),
            [Event::Cancelled { .. }]
        ));
        update(&mut e, &[]);
        update(&mut e, &[Button::L2]);
        update(&mut e, &[Button::L2, Button::A]);
        update(&mut e, &[Button::A]);
        assert!(matches!(
            update(&mut e, &[Button::A, Button::L2]).as_slice(),
            [Event::Cancelled { .. }]
        ));
        assert!(update(&mut e, &[]).is_empty());
    }

    #[test]
    fn cancellation_on_final_release_rearms_without_swallowing_the_next_stroke() {
        let mut e = engine();
        arm(&mut e);
        update(&mut e, &[Button::A]);
        assert!(matches!(
            update(&mut e, &[Button::L2]).as_slice(),
            [Event::Cancelled { .. }]
        ));
        assert_eq!(e.phase, Phase::Idle);
        update(&mut e, &[Button::L2, Button::B]);
        assert!(
            matches!(update(&mut e,&[Button::L2]).as_slice(),[Event::Chord{chord,held,..}]
            if chord.buttons==[Button::B] && held==&[Button::L2])
        );
    }

    #[test]
    fn new_buttons_during_release_and_mixed_reports_cancel() {
        let mut e = engine();
        arm(&mut e);
        update(&mut e, &[Button::A, Button::L1]);
        update(&mut e, &[Button::A]);
        assert_eq!(e.phase, Phase::Releasing);
        assert!(matches!(
            update(&mut e, &[Button::A, Button::B]).as_slice(),
            [Event::Cancelled { .. }]
        ));
        update(&mut e, &[]);
        update(&mut e, &[Button::A]);
        assert!(matches!(
            update(&mut e, &[Button::L1]).as_slice(),
            [Event::Cancelled { .. }]
        ));
        assert!(update(&mut e, &[]).is_empty());
    }

    #[test]
    fn dpad_rolls_into_diagonals_but_opposites_and_invalid_positions_cancel() {
        let mut e = engine();
        arm(&mut e);
        e.update(&state(&[], Dpad::Up));
        e.update(&state(&[], Dpad::UpLeft));
        assert!(
            matches!(update(&mut e,&[]).as_slice(),[Event::Chord{chord,..}] if chord.dpad==Dpad::UpLeft)
        );
        e.update(&state(&[], Dpad::Up));
        assert!(matches!(
            e.update(&state(&[], Dpad::Down)).as_slice(),
            [Event::Cancelled { .. }]
        ));
        update(&mut e, &[]);
        update(&mut e, &[Button::A]);
        let mut invalid = state(&[Button::A], Dpad::Neutral);
        invalid.dpad = None;
        assert!(matches!(
            e.update(&invalid).as_slice(),
            [Event::Cancelled { .. }]
        ));
        assert!(update(&mut e, &[]).is_empty());
    }

    #[test]
    fn diagonal_chords_survive_either_direction_release_order() {
        use Dpad::*;
        for (diagonal, directions) in [
            (UpLeft, [Up, Left]),
            (UpRight, [Up, Right]),
            (DownLeft, [Down, Left]),
            (DownRight, [Down, Right]),
        ] {
            for first in directions {
                for remaining in directions {
                    let mut e = engine();
                    arm(&mut e);
                    assert!(e.update(&state(&[], first)).is_empty());
                    assert!(e.update(&state(&[], diagonal)).is_empty());
                    assert!(e.update(&state(&[], remaining)).is_empty());
                    assert_eq!(e.phase, Phase::Releasing);
                    assert_eq!(
                        update(&mut e, &[]),
                        [Event::Chord {
                            chord: Chord {
                                dpad: diagonal,
                                buttons: vec![]
                            },
                            held: vec![],
                            action: None,
                        }]
                    );
                    assert!(update(&mut e, &[]).is_empty());
                }
            }
        }
    }

    #[test]
    fn partial_dpad_release_freezes_buttons_and_rejects_new_presses() {
        for release_button_first in [false, true] {
            let mut e = engine();
            arm(&mut e);
            e.update(&state(&[Button::A], Dpad::UpLeft));
            e.update(&state(&[Button::A], Dpad::Up));
            let middle = if release_button_first {
                state(&[], Dpad::Up)
            } else {
                state(&[Button::A], Dpad::Neutral)
            };
            assert!(e.update(&middle).is_empty());
            assert_eq!(
                update(&mut e, &[]),
                [Event::Chord {
                    chord: Chord {
                        dpad: Dpad::UpLeft,
                        buttons: vec![Button::A]
                    },
                    held: vec![],
                    action: None,
                }]
            );
        }
        for same_report in [false, true] {
            let mut e = engine();
            arm(&mut e);
            e.update(&state(&[], Dpad::UpLeft));
            if !same_report {
                e.update(&state(&[], Dpad::Up));
            }
            assert!(matches!(
                e.update(&state(&[Button::A], Dpad::Up)).as_slice(),
                [Event::Cancelled { .. }]
            ));
            assert!(update(&mut e, &[]).is_empty());
        }
    }

    #[test]
    fn a_rolled_release_cannot_turn_a_forbidden_diagonal_into_a_cardinal_chord() {
        let mut e = engine();
        e.profile.diagonals = false;
        arm(&mut e);
        e.update(&state(&[], Dpad::UpLeft));
        e.update(&state(&[], Dpad::Up));
        assert!(matches!(
            update(&mut e, &[]).as_slice(),
            [Event::Cancelled { .. }]
        ));
    }

    #[test]
    fn profile_checks_apply_to_the_final_chord_not_its_prefix() {
        let mut e = engine();
        e.profile.forbidden_chords.push(Chord {
            dpad: Dpad::Neutral,
            buttons: vec![Button::A],
        });
        arm(&mut e);
        update(&mut e, &[Button::A]);
        update(&mut e, &[Button::A, Button::L1]);
        assert!(matches!(
            update(&mut e, &[]).as_slice(),
            [Event::Chord { .. }]
        ));
        update(&mut e, &[Button::A]);
        assert!(matches!(
            update(&mut e, &[]).as_slice(),
            [Event::Cancelled { .. }]
        ));
    }

    #[test]
    fn lookup_uses_the_generated_layout_and_reports_unmapped_chords() {
        let strategy = Strategy {
            name: "test".into(),
            shift: Some(Button::L2),
            layer: Some(Button::R2),
        };
        let profile = Profile::default();
        let chord = Chord {
            dpad: Dpad::Neutral,
            buttons: vec![Button::A],
        };
        let mut layout = Layout {
            schema_version: SCHEMA_VERSION,
            cost_model_version: COST_MODEL_VERSION,
            strategy: strategy.clone(),
            profile: profile.clone(),
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
                Binding {
                    action: Action::Character('a'),
                    chord: chord.clone(),
                    held: vec![],
                },
                Binding {
                    action: Action::Character('A'),
                    chord,
                    held: vec![Button::L2],
                },
            ],
        };
        let mut e = Engine::new(strategy, profile, Some(layout.clone())).unwrap();
        arm(&mut e);
        update(&mut e, &[Button::A, Button::L2]);
        assert!(matches!(
            update(&mut e, &[]).as_slice(),
            [Event::Chord {
                action: Some(Action::Character('A')),
                ..
            }]
        ));
        update(&mut e, &[Button::L1]);
        assert!(matches!(
            update(&mut e, &[]).as_slice(),
            [Event::Unmapped { .. }]
        ));
        layout.bindings[0].chord.buttons = vec![Button::L2];
        assert!(layout.validate().is_err());
    }
}
