use crate::{
    Result,
    chord::{Event, Phase, Recognizer},
    controller::{Calibration, Normalizer, RawEvent, Snapshot},
    model::*,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

#[derive(clap::Args)]
pub struct InspectArgs {
    #[arg(long)]
    pub device: PathBuf,
    #[arg(long)]
    pub layout: Option<PathBuf>,
    #[arg(long)]
    pub calibration: Option<PathBuf>,
    /// Save complete reports to a new JSONL recording. Existing files are preserved.
    #[arg(long)]
    pub record: Option<PathBuf>,
    /// Stop automatically after this many seconds; otherwise use Ctrl-C.
    #[arg(long)]
    pub duration: Option<u64>,
    #[arg(long)]
    pub json: bool,
}

#[derive(clap::Args)]
pub struct CalibrateArgs {
    #[arg(long)]
    pub device: PathBuf,
    #[arg(short, long, default_value = "controller.json")]
    pub output: PathBuf,
    /// Use an existing mapping, including custom raw event codes.
    #[arg(long)]
    pub calibration: Option<PathBuf>,
    #[arg(long, default_value_t = 2)]
    pub rest_seconds: u64,
    #[arg(long, default_value_t = 8)]
    pub sweep_seconds: u64,
}

#[derive(clap::Args)]
pub struct ReplayArgs {
    #[arg(long)]
    pub input: PathBuf,
    #[arg(long)]
    pub json: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Header {
    pub schema_version: u32,
    pub calibration: Calibration,
    pub strategy: Strategy,
    pub profile: Profile,
    pub layout: Option<Layout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alpha_layout: Option<crate::alpha_banks::Layout>,
}

impl Header {
    pub fn new(calibration: Calibration, layout: Option<Layout>) -> Self {
        let (strategy, profile) = layout
            .as_ref()
            .map(|l| (l.strategy.clone(), l.profile.clone()))
            .unwrap_or_else(|| {
                (
                    Strategy {
                        independent: false,
                        name: "inspector".into(),
                        shift: Some(Button::L2),
                        layer: Some(Button::R2),
                    },
                    Profile::default(),
                )
            });
        Self {
            schema_version: 1,
            calibration,
            strategy,
            profile,
            layout,
            alpha_layout: None,
        }
    }
    pub fn with_layout(
        calibration: Calibration,
        layout: Option<crate::typing::TypingLayout>,
    ) -> Self {
        match layout {
            Some(crate::typing::TypingLayout::Standard(layout)) => {
                Self::new(calibration, Some(*layout))
            }
            Some(crate::typing::TypingLayout::Alpha(layout)) => {
                let mut header = Self::new(calibration, None);
                header.alpha_layout = Some(layout);
                header
            }
            None => Self::new(calibration, None),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Record {
    Header { header: Box<Header> },
    Frame { at_us: u64, events: Vec<RawEvent> },
    Reset { at_us: u64, reason: String },
    End { at_us: u64, reason: String },
}

#[derive(Debug, Serialize)]
pub struct View {
    pub at_us: u64,
    pub snapshot: Option<Snapshot>,
    pub phase: Phase,
    pub events: Vec<Event>,
}

struct Run {
    normalizer: Normalizer,
    engine: crate::typing::Engine,
    last: Option<Snapshot>,
    at_us: u64,
    ended: bool,
}

impl Run {
    fn new(header: Header) -> Result<Self> {
        if header.schema_version != 1 {
            return Err("unsupported recording schema version".into());
        }
        Ok(Self {
            normalizer: Normalizer::new(header.calibration)?,
            engine: match header.alpha_layout {
                Some(layout) => {
                    if header.layout.is_some() {
                        return Err("recording has two layouts".into());
                    }
                    crate::typing::Engine::Alpha(Box::new(crate::alpha_banks::Engine::new(layout)?))
                }
                None => crate::typing::Engine::Standard(Recognizer::new(
                    header.strategy,
                    header.profile,
                    header.layout,
                )?),
            },
            last: None,
            at_us: 0,
            ended: false,
        })
    }

    fn update(&mut self, record: Record) -> Result<Option<View>> {
        if self.ended {
            return Err("recording contains data after its end marker".into());
        }
        let at_us = match &record {
            Record::Frame { at_us, .. }
            | Record::Reset { at_us, .. }
            | Record::End { at_us, .. } => *at_us,
            Record::Header { .. } => return Err("unexpected recording header".into()),
        };
        if at_us < self.at_us {
            return Err("recording timestamps must be nondecreasing".into());
        }
        self.at_us = at_us;
        let is_end = matches!(&record, Record::End { .. });
        match record {
            Record::Frame { events, .. } => {
                let snapshot = self.normalizer.apply(&events)?;
                let events = self.engine.update_at(&snapshot, at_us / 1000);
                if self.last.as_ref() == Some(&snapshot) && events.is_empty() {
                    return Ok(None);
                }
                self.last = Some(snapshot.clone());
                Ok(Some(View {
                    at_us,
                    snapshot: Some(snapshot),
                    phase: self.engine.phase(),
                    events,
                }))
            }
            Record::Reset { reason, .. } | Record::End { reason, .. } => {
                self.ended = is_end;
                let event = self.engine.reset(&reason);
                self.normalizer = Normalizer::new(self.normalizer.calibration.clone())?;
                self.last = None;
                Ok(Some(View {
                    at_us,
                    snapshot: None,
                    phase: self.engine.phase(),
                    events: vec![event],
                }))
            }
            Record::Header { .. } => unreachable!(),
        }
    }
}

pub struct Session {
    run: Run,
    recording: Option<BufWriter<File>>,
    json: bool,
}

impl Session {
    pub fn new(header: Header, path: Option<&Path>, json: bool) -> Result<Self> {
        let run = Run::new(header.clone())?;
        let mut recording = path
            .map(|path| {
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path)
                    .map(BufWriter::new)
                    .map_err(|e| format!("{}: {e}; recordings require a new file", path.display()))
            })
            .transpose()?;
        if let Some(writer) = &mut recording {
            write_record(
                writer,
                &Record::Header {
                    header: Box::new(header),
                },
            )?;
        }
        Ok(Self {
            run,
            recording,
            json,
        })
    }

    pub fn update(&mut self, record: Record) -> Result<()> {
        let view = self.run.update(record.clone())?;
        if let Some(writer) = &mut self.recording {
            write_record(writer, &record)?;
        }
        if let Some(view) = view {
            display(&view, self.json)?;
        }
        Ok(())
    }

    pub fn finish(&mut self, at_us: u64, reason: &str) -> Result<()> {
        self.update(Record::End {
            at_us,
            reason: reason.into(),
        })?;
        if let Some(writer) = &mut self.recording {
            writer
                .flush()
                .and_then(|()| writer.get_ref().sync_all())
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

pub fn replay(args: ReplayArgs) -> Result<()> {
    let file = File::open(&args.input).map_err(|e| format!("{}: {e}", args.input.display()))?;
    let mut lines = BufReader::new(file).lines();
    let first = lines
        .next()
        .ok_or("recording is empty")?
        .map_err(|e| e.to_string())?;
    let Record::Header { header } =
        serde_json::from_str(&first).map_err(|e| format!("recording line 1: {e}"))?
    else {
        return Err("recording must start with a header".into());
    };
    let mut run = Run::new(*header)?;
    for (index, line) in lines.enumerate() {
        let line = line.map_err(|e| e.to_string())?;
        let record = serde_json::from_str(&line)
            .map_err(|e| format!("recording line {}: {e}", index + 2))?;
        if let Some(view) = run
            .update(record)
            .map_err(|e| format!("recording line {}: {e}", index + 2))?
        {
            display(&view, args.json)?;
        }
    }
    if !run.ended {
        return Err("recording is incomplete: missing end marker; pending chord discarded".into());
    }
    Ok(())
}

fn write_record(writer: &mut BufWriter<File>, record: &Record) -> Result<()> {
    serde_json::to_writer(&mut *writer, record).map_err(|e| e.to_string())?;
    writer
        .write_all(b"\n")
        .and_then(|()| writer.flush())
        .map_err(|e| e.to_string())
}

fn display(view: &View, json: bool) -> Result<()> {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    if json {
        serde_json::to_writer(&mut out, view).map_err(|e| e.to_string())?;
        writeln!(out).map_err(|e| e.to_string())?;
    } else {
        if let Some(snapshot) = &view.snapshot {
            let dpad = snapshot
                .dpad
                .map(|d| format!("{d:?}"))
                .unwrap_or_else(|| "Invalid".into());
            writeln!(
                out,
                "{:8.3}s {:?} | D-pad {} | {} | L2 {:3.0}% R2 {:3.0}%",
                view.at_us as f64 / 1_000_000.0,
                view.phase,
                dpad,
                buttons(&snapshot.buttons),
                snapshot.l2 * 100.0,
                snapshot.r2 * 100.0
            )
            .map_err(|e| e.to_string())?;
            if !snapshot.menu_buttons.is_empty() {
                writeln!(out, "Menu buttons: {:?}", snapshot.menu_buttons)
                    .map_err(|e| e.to_string())?;
            }
            if snapshot.left_stick != Dpad::Neutral || snapshot.right_stick != Dpad::Neutral {
                writeln!(
                    out,
                    "Sticks: left {:?}, right {:?} | keyboard {:?}",
                    snapshot.left_stick,
                    snapshot.right_stick,
                    snapshot.modifiers()
                )
                .map_err(|e| e.to_string())?;
            }
        }
        for event in &view.events {
            let text = match event {
                Event::Ready => "READY".into(),
                Event::Menu {
                    button,
                    action,
                    keyboard_modifiers,
                } => format!(
                    "BUTTON {button:?} | keyboard {keyboard_modifiers:?} => {}",
                    action.label()
                ),
                Event::Cancelled { reason } => format!("CANCELLED: {reason}"),
                Event::Reset { reason, discarded } => {
                    format!("RESET: {reason}; pending chord discarded: {discarded}")
                }
                Event::Chord {
                    chord,
                    held,
                    action,
                    held_dpad,
                    keyboard_modifiers,
                } => format!(
                    "CHORD {:?}+{} | held {}{}{}",
                    chord.dpad,
                    buttons(&chord.buttons),
                    held_controls(held, *held_dpad),
                    if keyboard_modifiers.is_empty() {
                        String::new()
                    } else {
                        format!(" | keyboard {keyboard_modifiers:?}")
                    },
                    action
                        .as_ref()
                        .map(|a| format!(" => {}", a.label()))
                        .unwrap_or_default()
                ),
                Event::Unmapped {
                    chord,
                    held,
                    held_dpad,
                    keyboard_modifiers,
                } => format!(
                    "UNMAPPED {:?}+{} | held {} | keyboard {:?}",
                    chord.dpad,
                    buttons(&chord.buttons),
                    held_controls(held, *held_dpad),
                    keyboard_modifiers
                ),
            };
            writeln!(out, "{text}").map_err(|e| e.to_string())?;
        }
    }
    out.flush().map_err(|e| e.to_string())
}

fn buttons(buttons: &[Button]) -> String {
    if buttons.is_empty() {
        "-".into()
    } else {
        buttons
            .iter()
            .map(|b| format!("{b:?}"))
            .collect::<Vec<_>>()
            .join("+")
    }
}

fn held_controls(held: &[Button], dpad: Dpad) -> String {
    if dpad == Dpad::Neutral {
        buttons(held)
    } else {
        format!("D-pad {dpad:?} + {}", buttons(held))
    }
}

pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    serde_json::from_str(&fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?)
        .map_err(|e| format!("{}: {e}", path.display()))
}

pub fn write_new_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut file, value).map_err(|e| e.to_string())?;
    file.write_all(b"\n")
        .and_then(|()| file.as_file().sync_all())
        .map_err(|e| e.to_string())?;
    file.persist_noclobber(path)
        .map_err(|e| format!("{}: {e}; choose a new output file", path.display()))?;
    Ok(())
}
