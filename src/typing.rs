use crate::{
    Result,
    chord::{Event, Phase, Recognizer},
    controller::Snapshot,
    model::{Action, Button, Layout, Modifier, button_mask},
};
use std::path::PathBuf;

pub enum TypingLayout {
    Standard(Box<Layout>),
    Alpha(crate::alpha_banks::Layout),
}

impl TypingLayout {
    pub fn parse(value: serde_json::Value) -> Result<Self> {
        let layout = if value.get("format").is_some() {
            Self::Alpha(serde_json::from_value(value).map_err(|e| e.to_string())?)
        } else {
            Self::Standard(serde_json::from_value(value).map_err(|e| e.to_string())?)
        };
        match &layout {
            Self::Standard(layout) => layout.validate()?,
            Self::Alpha(layout) => layout.validate()?,
        }
        Ok(layout)
    }

    pub fn actions(&self) -> Result<Vec<Action>> {
        match self {
            Self::Standard(layout) => {
                Ok(layout.bindings.iter().map(|b| b.action.clone()).collect())
            }
            Self::Alpha(layout) => layout.actions(),
        }
    }

    pub fn validate_pause(&self, buttons: &[Button]) -> Result<u8> {
        match self {
            Self::Standard(layout) => validate_pause(layout, buttons),
            Self::Alpha(layout) => layout.validate_pause(buttons),
        }
    }
}

pub(crate) enum Engine {
    Standard(Recognizer),
    Alpha(Box<crate::alpha_banks::Engine>),
}

impl Engine {
    pub(crate) fn phase(&self) -> Phase {
        match self {
            Self::Standard(e) => e.phase(),
            Self::Alpha(e) => e.phase(),
        }
    }
    pub(crate) fn update_at(&mut self, snapshot: &Snapshot, now: u64) -> Vec<Event> {
        match self {
            Self::Standard(e) => e.update_at(snapshot, now),
            Self::Alpha(e) => e.update_at(snapshot, now),
        }
    }
    fn ready(&self) -> bool {
        match self {
            Self::Standard(engine) => engine.phase() != Phase::Waiting,
            Self::Alpha(engine) => engine.phase() != Phase::Waiting,
        }
    }
    fn update(&mut self, snapshot: &Snapshot) -> Vec<Event> {
        match self {
            Self::Standard(engine) => engine.update(snapshot),
            Self::Alpha(engine) => engine.update(snapshot),
        }
    }
    fn repeat(&mut self) -> Vec<Event> {
        match self {
            Self::Standard(engine) => engine.repeat(),
            Self::Alpha(engine) => engine.repeat(),
        }
    }
    pub(crate) fn reset(&mut self, reason: &str) -> Event {
        match self {
            Self::Standard(engine) => engine.reset(reason),
            Self::Alpha(engine) => engine.reset(reason),
        }
    }
}

#[derive(clap::Args)]
pub struct TypeArgs {
    #[arg(long)]
    pub device: PathBuf,
    #[arg(long)]
    pub layout: PathBuf,
    #[arg(long)]
    pub calibration: Option<PathBuf>,
    /// Display actions without opening uinput or sending keyboard events.
    #[arg(long)]
    pub dry_run: bool,
    /// Start typing after all controls return to neutral. Otherwise start paused.
    #[arg(long)]
    pub start_active: bool,
    /// Reserved toggle gesture, including any held modifiers. All supersets are reserved.
    #[arg(long, value_delimiter = ',', default_value = "A,B,X,Y", value_parser = parse_button)]
    pub pause_buttons: Vec<Button>,
    /// Give Chordpad exclusive access to the controller until exit.
    #[arg(long)]
    pub grab: bool,
    #[arg(long)]
    pub duration: Option<u64>,
}

fn parse_button(value: &str) -> Result<Button> {
    Button::ALL
        .into_iter()
        .find(|b| format!("{b:?}").eq_ignore_ascii_case(value))
        .ok_or_else(|| format!("unknown button {value:?}; use A,B,X,Y,L1,R1,L2,R2"))
}

pub trait Output {
    fn send(&mut self, action: &Action) -> Result<()>;
    fn send_modified(&mut self, action: &Action, modifiers: &[Modifier]) -> Result<()> {
        if modifiers.is_empty() {
            self.send(action)
        } else {
            Err("output does not support keyboard modifiers".into())
        }
    }
    fn set_modifiers(&mut self, _modifiers: &[Modifier]) -> Result<()> {
        Ok(())
    }
    fn release_all(&mut self) -> Result<()>;
}

pub struct DryRun;

impl Output for DryRun {
    fn send(&mut self, action: &Action) -> Result<()> {
        use std::io::Write;
        writeln!(std::io::stdout().lock(), "WOULD TYPE {}", action.label())
            .map_err(|e| e.to_string())
    }

    fn send_modified(&mut self, action: &Action, modifiers: &[Modifier]) -> Result<()> {
        if modifiers.is_empty() {
            return self.send(action);
        }
        use std::io::Write;
        writeln!(
            std::io::stdout().lock(),
            "WOULD TYPE {:?} + {}",
            modifiers,
            action.label()
        )
        .map_err(|e| e.to_string())
    }
    fn set_modifiers(&mut self, modifiers: &[Modifier]) -> Result<()> {
        use std::io::Write;
        writeln!(std::io::stdout().lock(), "WOULD HOLD {modifiers:?}").map_err(|e| e.to_string())
    }
    fn release_all(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Output for Box<dyn Output> {
    fn send(&mut self, action: &Action) -> Result<()> {
        (**self).send(action)
    }
    fn send_modified(&mut self, action: &Action, modifiers: &[Modifier]) -> Result<()> {
        (**self).send_modified(action, modifiers)
    }
    fn set_modifiers(&mut self, modifiers: &[Modifier]) -> Result<()> {
        (**self).set_modifiers(modifiers)
    }
    fn release_all(&mut self) -> Result<()> {
        (**self).release_all()
    }
}

pub fn validate_pause(layout: &Layout, buttons: &[Button]) -> Result<u8> {
    let mask = button_mask(buttons)?;
    if mask == 0 {
        return Err("pause gesture must contain at least one button".into());
    }
    if layout.profile.navigation_layer
        && (Button::L1.bit() | Button::L2.bit() | Button::R2.bit()) & mask == mask
    {
        return Err(
            "pause gesture conflicts with R2 + L1 navigation; choose --pause-buttons".into(),
        );
    }
    if layout.profile.function_layer
        && (Button::R1.bit() | Button::L2.bit() | Button::R2.bit()) & mask == mask
    {
        return Err(
            "pause gesture conflicts with R2 + R1 function layer; choose --pause-buttons".into(),
        );
    }
    if buttons
        .iter()
        .any(|b| layout.profile.disabled_buttons.contains(b))
    {
        return Err("pause gesture uses a disabled button; choose --pause-buttons".into());
    }
    for binding in &layout.bindings {
        let physical = binding.chord.input()?.mask | button_mask(&binding.held)?;
        if physical & mask == mask {
            return Err(format!(
                "pause gesture conflicts with {}; reserve it with profile.forbidden_combinations or choose --pause-buttons",
                binding.action.label()
            ));
        }
    }
    Ok(mask)
}

pub struct Runtime<O: Output> {
    engine: Engine,
    output: O,
    pause_mask: u8,
    gesture_latched: bool,
    paused: bool,
    shift_button: Option<Button>,
    held_modifiers: Vec<Modifier>,
}

impl<O: Output> Runtime<O> {
    pub fn new(layout: Layout, buttons: &[Button], paused: bool, output: O) -> Result<Self> {
        layout.validate()?;
        Self::with_layout(
            TypingLayout::Standard(Box::new(layout)),
            buttons,
            paused,
            output,
        )
    }

    pub fn with_layout(
        layout: TypingLayout,
        buttons: &[Button],
        paused: bool,
        output: O,
    ) -> Result<Self> {
        let pause_mask = layout.validate_pause(buttons)?;
        let shift_button = match &layout {
            TypingLayout::Standard(layout) => layout.strategy.shift,
            TypingLayout::Alpha(_) => Some(Button::L2),
        };
        let engine = match layout {
            TypingLayout::Standard(layout) => Engine::Standard(Recognizer::new(
                layout.strategy.clone(),
                layout.profile.clone(),
                Some(*layout),
            )?),
            TypingLayout::Alpha(layout) => {
                Engine::Alpha(Box::new(crate::alpha_banks::Engine::new(layout)?))
            }
        };
        Ok(Self {
            engine,
            output,
            pause_mask,
            gesture_latched: true,
            paused,
            shift_button,
            held_modifiers: vec![],
        })
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn set_paused(&mut self, paused: bool) -> Result<Vec<Event>> {
        if self.paused == paused {
            return Ok(vec![]);
        }
        self.paused = paused;
        let events = vec![self.engine.reset(if paused { "paused" } else { "resumed" })];
        self.held_modifiers.clear();
        self.output.release_all()?;
        Ok(events)
    }

    pub fn reset(&mut self, reason: &str) -> Result<Vec<Event>> {
        // A resync cannot safely tell whether an interrupted gesture already toggled.
        self.gesture_latched = true;
        let event = self.engine.reset(reason);
        self.held_modifiers.clear();
        self.output.release_all()?;
        Ok(vec![event])
    }

    pub fn update(&mut self, snapshot: &Snapshot) -> Result<Vec<Event>> {
        let input = snapshot.input();
        if self.gesture_latched {
            if snapshot.is_neutral() {
                self.gesture_latched = false;
                if !self.paused {
                    return Ok(self.engine.update(snapshot));
                }
            }
            return Ok(vec![]);
        }
        if input.mask & self.pause_mask == self.pause_mask {
            self.gesture_latched = true;
            return self.set_paused(!self.paused);
        }
        if self.paused {
            return Ok(vec![]);
        }
        let events = self.engine.update(snapshot);
        let mut modifiers = if self.engine.ready() {
            snapshot.modifiers()
        } else {
            vec![]
        };
        if self.engine.ready()
            && self
                .shift_button
                .is_some_and(|b| snapshot.buttons.contains(&b))
        {
            modifiers.push(Modifier::Shift);
        }
        modifiers.sort();
        modifiers.dedup();
        if modifiers != self.held_modifiers {
            self.output.set_modifiers(&modifiers)?;
            self.held_modifiers = modifiers;
        }
        self.emit(&events)?;
        Ok(events)
    }

    pub fn repeat(&mut self) -> Result<Vec<Event>> {
        if self.paused || self.gesture_latched {
            return Ok(vec![]);
        }
        let events = self.engine.repeat();
        self.emit(&events)?;
        Ok(events)
    }

    fn emit(&mut self, events: &[Event]) -> Result<()> {
        for event in events {
            if let Event::Chord {
                action: Some(action),
                keyboard_modifiers,
                ..
            }
            | Event::Menu {
                action,
                keyboard_modifiers,
                ..
            } = event
            {
                self.output.send_modified(action, keyboard_modifiers)?;
            }
        }
        Ok(())
    }
}

impl<O: Output> Drop for Runtime<O> {
    fn drop(&mut self) {
        let _ = self.output.release_all();
    }
}
