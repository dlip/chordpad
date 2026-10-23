use crate::{
    Result,
    chord::{Event, Recognizer},
    controller::Snapshot,
    model::{Action, Button, Layout, Modifier, button_mask},
};
use std::path::PathBuf;

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
    #[arg(long, value_delimiter = ',', default_value = "L1,R1,L2,R2", value_parser = parse_button)]
    pub pause_buttons: Vec<Button>,
    /// Give padtype exclusive access to the controller until exit.
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
    engine: Recognizer,
    output: O,
    pause_mask: u8,
    gesture_latched: bool,
    paused: bool,
}

impl<O: Output> Runtime<O> {
    pub fn new(layout: Layout, buttons: &[Button], paused: bool, output: O) -> Result<Self> {
        layout.validate()?;
        let pause_mask = validate_pause(&layout, buttons)?;
        Ok(Self {
            engine: Recognizer::new(
                layout.strategy.clone(),
                layout.profile.clone(),
                Some(layout),
            )?,
            output,
            pause_mask,
            gesture_latched: true,
            paused,
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
        self.output.release_all()?;
        Ok(events)
    }

    pub fn reset(&mut self, reason: &str) -> Result<Vec<Event>> {
        // A resync cannot safely tell whether an interrupted gesture already toggled.
        self.gesture_latched = true;
        let event = self.engine.reset(reason);
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
