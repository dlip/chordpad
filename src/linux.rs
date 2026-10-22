use crate::{
    Result,
    controller::{Packet, *},
    inspector::{self, CalibrateArgs, Header, InspectArgs, Record, Session},
    model::{Button, Layout, Side},
};
use evdev::{KeyCode, raw_stream::RawDevice};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Serialize)]
pub struct Axis {
    pub minimum: i32,
    pub maximum: i32,
    pub value: i32,
}

#[derive(Debug, Serialize)]
pub struct DeviceInfo {
    pub path: PathBuf,
    pub name: String,
    pub id: DeviceId,
    pub keys: Vec<u16>,
    pub axes: BTreeMap<u16, Axis>,
}

fn open(path: &Path) -> Result<RawDevice> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
        .map_err(|e| {
            format!(
                "{}: {e}; read access to this gamepad's event node is required",
                path.display()
            )
        })?;
    let device = RawDevice::from_fd(file.into()).map_err(|e| format!("{}: {e}", path.display()))?;
    if !is_gamepad(&device) {
        return Err(format!(
            "{} is not a Linux gamepad event device",
            path.display()
        ));
    }
    Ok(device)
}

fn is_gamepad(device: &RawDevice) -> bool {
    let keys: Vec<_> = device
        .supported_keys()
        .into_iter()
        .flat_map(|keys| keys.iter())
        .map(|k| k.0)
        .collect();
    let axes: Vec<_> = device
        .supported_absolute_axes()
        .into_iter()
        .flat_map(|axes| axes.iter())
        .map(|a| a.0)
        .collect();
    gamepad_capabilities(&keys, &axes)
}

fn gamepad_capabilities(keys: &[u16], axes: &[u16]) -> bool {
    !keys.contains(&KeyCode::KEY_A.0)
        && [304, 305, 307, 308, 310, 311]
            .iter()
            .all(|code| keys.contains(code))
        && (axes.contains(&16) && axes.contains(&17)
            || [544, 545, 546, 547].iter().all(|code| keys.contains(code)))
}

fn info(device: &RawDevice, path: &Path) -> Result<DeviceInfo> {
    let id = device.input_id();
    let axes = device
        .get_absinfo()
        .map_err(|e| e.to_string())?
        .map(|(code, a)| {
            (
                code.0,
                Axis {
                    minimum: a.minimum(),
                    maximum: a.maximum(),
                    value: a.value(),
                },
            )
        })
        .collect();
    Ok(DeviceInfo {
        path: path.into(),
        name: device.name().unwrap_or("Unnamed gamepad").into(),
        id: DeviceId {
            vendor: id.vendor(),
            product: id.product(),
        },
        keys: device
            .supported_keys()
            .into_iter()
            .flat_map(|keys| keys.iter())
            .map(|k| k.0)
            .collect(),
        axes,
    })
}

pub fn devices(json: bool) -> Result<()> {
    let mut devices = vec![];
    let mut unreadable = 0;
    for entry in fs::read_dir("/dev/input").map_err(|e| format!("/dev/input: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_name().to_string_lossy().starts_with("event") {
            continue;
        }
        let file = match OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(entry.path())
        {
            Ok(file) => file,
            Err(_) => {
                unreadable += 1;
                continue;
            }
        };
        let Ok(device) = RawDevice::from_fd(file.into()) else {
            continue;
        };
        if is_gamepad(&device) {
            devices.push(info(&device, &entry.path())?);
        }
    }
    devices.sort_by(|a, b| a.path.cmp(&b.path));
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&devices).map_err(|e| e.to_string())?
        );
    } else {
        if devices.is_empty() {
            println!("No readable Linux gamepad devices found.");
        }
        for device in devices {
            println!(
                "{}  {}  ({:04x}:{:04x})",
                device.path.display(),
                device.name,
                device.id.vendor,
                device.id.product
            );
            match mapping(&device) {
                Ok(calibration) => {
                    println!("  D-pad: {:?}", calibration.dpad);
                    for (button, trigger) in calibration.triggers {
                        println!("  {button:?}: {trigger:?}");
                    }
                }
                Err(error) => println!("  Mapping needs configuration: {error}"),
            }
        }
    }
    if unreadable != 0 {
        eprintln!(
            "{unreadable} input nodes could not be opened; only readable gamepads are listed."
        );
    }
    Ok(())
}

fn mapping(device: &DeviceInfo) -> Result<Calibration> {
    let mut map = Calibration {
        device: Some(device.id.clone()),
        ..Calibration::default()
    };
    if !(device.axes.contains_key(&16) && device.axes.contains_key(&17)) {
        map.dpad = DpadSource::Buttons {
            up: 544,
            down: 545,
            left: 546,
            right: 547,
        };
    }
    for (button, candidates, key) in [
        (Button::L2, [2, 10, 21], 312),
        (Button::R2, [5, 9, 20], 313),
    ] {
        let source = if let Some(code) = candidates
            .into_iter()
            .find(|code| device.axes.contains_key(code))
        {
            let a = &device.axes[&code];
            Trigger::axis(code, a.minimum, a.maximum)
        } else if device.keys.contains(&key) {
            Trigger::Key { code: key }
        } else {
            return Err(format!(
                "no standard input source for {button:?}; supply a calibration mapping"
            ));
        };
        map.triggers.insert(button, source);
    }
    for (side, x, y) in [(Side::Left, 0, 1), (Side::Right, 3, 4)] {
        if device.axes.contains_key(&x) && device.axes.contains_key(&y) {
            let axis = |code| {
                let info = &device.axes[&code];
                StickAxis {
                    code,
                    minimum: info.minimum,
                    maximum: info.maximum,
                    center: ((i64::from(info.minimum) + i64::from(info.maximum)) / 2) as i32,
                    inverted: false,
                }
            };
            map.thumbsticks
                .insert(side, Thumbstick::new(axis(x), axis(y)));
        }
    }
    if !map.thumbsticks.is_empty() {
        map.schema_version = 2;
    }
    validate_mapping(&map, device)?;
    Ok(map)
}

fn validate_mapping(calibration: &Calibration, device: &DeviceInfo) -> Result<()> {
    calibration.validate()?;
    if calibration
        .device
        .as_ref()
        .is_some_and(|id| *id != device.id)
    {
        return Err("calibration belongs to a different controller vendor/product".into());
    }
    for (kind, code) in calibration.sources() {
        if !(kind == KEY && device.keys.contains(&code)
            || kind == ABS && device.axes.contains_key(&code))
        {
            return Err(format!(
                "device does not report mapped event source {kind}:{code}"
            ));
        }
    }
    Ok(())
}

fn load_mapping(path: Option<&Path>, device: &DeviceInfo) -> Result<Calibration> {
    let calibration = match path {
        Some(path) => inspector::read_json(path)?,
        None => mapping(device)?,
    };
    validate_mapping(&calibration, device)?;
    Ok(calibration)
}

fn snapshot(device: &RawDevice, calibration: &Calibration) -> io::Result<Vec<RawEvent>> {
    let keys = device.get_key_state()?;
    let axes = device.get_abs_state()?;
    Ok(calibration
        .sources()
        .into_iter()
        .map(|(event_type, code)| RawEvent {
            event_type,
            code,
            value: if event_type == KEY {
                i32::from(keys.contains(KeyCode(code)))
            } else {
                axes[code as usize].value
            },
        })
        .collect())
}

fn stopped() -> Result<Arc<AtomicBool>> {
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    ctrlc::set_handler(move || flag.store(true, Ordering::SeqCst)).map_err(|e| e.to_string())?;
    Ok(stop)
}

fn drain(device: &mut RawDevice) -> io::Result<()> {
    loop {
        match device.fetch_events() {
            Ok(events) => {
                if events.count() == 0 {
                    return Ok(());
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(()),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
}

fn refresh(device: &mut RawDevice, calibration: &Calibration) -> Result<Option<Vec<RawEvent>>> {
    match drain(device).and_then(|()| snapshot(device, calibration)) {
        Ok(events) => Ok(Some(events)),
        Err(error) if matches!(error.raw_os_error(), Some(libc::ENODEV | libc::EIO)) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

enum LiveEvent {
    Frame(Vec<RawEvent>),
    Reset(&'static str),
    Control(Vec<RawEvent>),
}

struct ControlFlags {
    pause: Arc<AtomicBool>,
    resume: Arc<AtomicBool>,
}

fn at_us(started: Instant) -> u64 {
    started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
}

fn drive(
    device: &mut RawDevice,
    calibration: &Calibration,
    stop: &AtomicBool,
    duration: Option<u64>,
    started: Instant,
    controls: Option<&ControlFlags>,
    mut consume: impl FnMut(u64, LiveEvent) -> Result<()>,
) -> Result<&'static str> {
    let Some(initial) = refresh(device, calibration)? else {
        return Ok("disconnected");
    };
    consume(0, LiveEvent::Frame(initial))?;
    let mut reader = ReportReader::default();
    loop {
        if stop.load(Ordering::SeqCst) {
            return Ok("interrupted");
        }
        if duration.is_some_and(|seconds| started.elapsed() >= Duration::from_secs(seconds)) {
            return Ok("duration elapsed");
        }
        if controls
            .is_some_and(|c| c.pause.load(Ordering::SeqCst) || c.resume.load(Ordering::SeqCst))
        {
            let Some(events) = refresh(device, calibration)? else {
                return Ok("disconnected");
            };
            reader = ReportReader::default();
            consume(at_us(started), LiveEvent::Control(events))?;
        }
        let events = match device.fetch_events() {
            Ok(events) => events.collect::<Vec<_>>(),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(5));
                continue;
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) if matches!(error.raw_os_error(), Some(libc::ENODEV | libc::EIO)) => {
                return Ok("disconnected");
            }
            Err(error) => return Err(error.to_string()),
        };
        if events.is_empty() {
            return Ok("disconnected");
        }
        for event in events {
            let raw = RawEvent {
                event_type: event.event_type().0,
                code: event.code(),
                value: event.value(),
            };
            match reader.push(raw) {
                Some(Packet::Dropped) => {
                    consume(at_us(started), LiveEvent::Reset("input events dropped"))?
                }
                Some(Packet::Resync) => {
                    let Some(events) = refresh(device, calibration)? else {
                        return Ok("disconnected");
                    };
                    consume(at_us(started), LiveEvent::Frame(events))?;
                    break;
                }
                Some(Packet::StateReset) => {
                    consume(at_us(started), LiveEvent::Reset("synthetic input report"))?;
                    let Some(events) = refresh(device, calibration)? else {
                        return Ok("disconnected");
                    };
                    consume(at_us(started), LiveEvent::Frame(events))?;
                    break;
                }
                Some(Packet::Frame(events)) => consume(at_us(started), LiveEvent::Frame(events))?,
                None => {}
            }
        }
    }
}

pub fn inspect(args: InspectArgs) -> Result<()> {
    if args.duration == Some(0) {
        return Err("duration must be positive".into());
    }
    let mut device = open(&args.device)?;
    let info = info(&device, &args.device)?;
    let calibration = load_mapping(args.calibration.as_deref(), &info)?;
    let layout: Option<Layout> = args
        .layout
        .as_deref()
        .map(inspector::read_json)
        .transpose()?;
    let stop = stopped()?;
    let mut session = Session::new(
        Header::new(calibration.clone(), layout),
        args.record.as_deref(),
        args.json,
    )?;
    eprintln!(
        "Inspecting {} ({}). Release all controls to arm; Ctrl-C stops. No keyboard output.",
        info.name,
        args.device.display()
    );
    let started = Instant::now();
    let result = drive(
        &mut device,
        &calibration,
        &stop,
        args.duration,
        started,
        None,
        |at_us, event| match event {
            LiveEvent::Frame(events) => session.update(Record::Frame { at_us, events }),
            LiveEvent::Reset(reason) => session.update(Record::Reset {
                at_us,
                reason: reason.into(),
            }),
            LiveEvent::Control(_) => unreachable!(),
        },
    );
    session.finish(
        at_us(started),
        result.as_ref().copied().unwrap_or("input error"),
    )?;
    result.map(|_| ())
}

fn feedback(events: &[crate::chord::Event]) {
    use crate::chord::Event;
    for event in events {
        match event {
            Event::Ready => eprintln!("READY"),
            Event::Reset { reason, discarded } => {
                eprintln!("{reason}; pending stroke discarded: {discarded}")
            }
            Event::Cancelled { reason } => eprintln!("CANCELLED: {reason}"),
            Event::Unmapped {
                chord,
                held,
                held_dpad,
                keyboard_modifiers,
            } => eprintln!(
                "UNMAPPED: {chord:?}, held {held:?}, held D-pad {held_dpad:?}, keyboard {keyboard_modifiers:?}"
            ),
            Event::Chord { .. } => {}
        }
    }
}

pub fn type_layout(args: crate::typing::TypeArgs) -> Result<()> {
    use crate::{
        output,
        typing::{DryRun, Output, Runtime, validate_pause},
    };
    if args.duration == Some(0) {
        return Err("duration must be positive".into());
    }
    let layout: Layout = inspector::read_json(&args.layout)?;
    output::validate_layout(&layout)?;
    validate_pause(&layout, &args.pause_buttons)?;
    let mut device = open(&args.device)?;
    let info = info(&device, &args.device)?;
    let calibration = load_mapping(args.calibration.as_deref(), &info)?;
    let mut normalizer = Normalizer::new(calibration.clone())?;
    let stop = stopped()?;
    let pause = Arc::new(AtomicBool::new(false));
    let resume = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(libc::SIGUSR1, pause.clone()).map_err(|e| e.to_string())?;
    signal_hook::flag::register(libc::SIGUSR2, resume.clone()).map_err(|e| e.to_string())?;
    let controls = ControlFlags { pause, resume };
    let output: Box<dyn Output> = if args.dry_run {
        Box::new(DryRun)
    } else {
        let keyboard = output::Keyboard::new(output::virtual_device()?);
        // Allow desktop input consumers to discover the new keyboard before arming.
        thread::sleep(Duration::from_secs(1));
        Box::new(keyboard)
    };
    if args.grab {
        device
            .grab()
            .map_err(|e| format!("cannot grab controller: {e}"))?;
    }
    let mut runtime = Runtime::new(layout, &args.pause_buttons, !args.start_active, output)?;
    eprintln!(
        "{} {} (PID {}). {}. Toggle with {:?}; release all controls after toggling.",
        if args.dry_run {
            "Dry run:"
        } else {
            "Typing with"
        },
        info.name,
        std::process::id(),
        if runtime.paused() {
            "PAUSED"
        } else {
            "ACTIVE: release all controls to arm"
        },
        args.pause_buttons
    );
    eprintln!(
        "Pause: kill -USR1 {}; resume: kill -USR2 {}; stop: Ctrl-C or SIGTERM. Output assumes US keyboard layout, Caps Lock off.",
        std::process::id(),
        std::process::id()
    );
    let started = Instant::now();
    let result = drive(
        &mut device,
        &calibration,
        &stop,
        args.duration,
        started,
        Some(&controls),
        |_, event| {
            let before = runtime.paused();
            match event {
                LiveEvent::Frame(events) => feedback(&runtime.update(&normalizer.apply(&events)?)?),
                LiveEvent::Reset(reason) => {
                    feedback(&runtime.reset(reason)?);
                    normalizer = Normalizer::new(calibration.clone())?;
                }
                LiveEvent::Control(events) => {
                    feedback(&runtime.reset("control command")?);
                    normalizer = Normalizer::new(calibration.clone())?;
                    if controls.pause.swap(false, Ordering::SeqCst) {
                        feedback(&runtime.set_paused(true)?);
                    }
                    if controls.resume.swap(false, Ordering::SeqCst) {
                        feedback(&runtime.set_paused(false)?);
                    }
                    feedback(&runtime.update(&normalizer.apply(&events)?)?);
                }
            }
            if runtime.paused() != before {
                eprintln!(
                    "{}",
                    if runtime.paused() {
                        "PAUSED"
                    } else {
                        "ACTIVE: release all controls to arm"
                    }
                );
            }
            Ok(())
        },
    );
    let cleanup = runtime.reset(result.as_ref().copied().unwrap_or("input/output error"));
    if args.grab {
        let _ = device.ungrab();
    }
    feedback(&cleanup?);
    result.map(|_| ())
}

#[derive(Clone, Copy)]
struct Observed {
    minimum: i32,
    maximum: i32,
    last: i32,
}

fn observe(device: &RawDevice, seconds: u64, stop: &AtomicBool) -> Result<BTreeMap<u16, Observed>> {
    let start = Instant::now();
    let mut observed = BTreeMap::<u16, Observed>::new();
    loop {
        if stop.load(Ordering::SeqCst) {
            return Err("calibration interrupted; no file written".into());
        }
        for (code, axis) in device.get_absinfo().map_err(|e| e.to_string())? {
            let value = axis.value();
            let range = observed.entry(code.0).or_insert(Observed {
                minimum: value,
                maximum: value,
                last: value,
            });
            range.minimum = range.minimum.min(value);
            range.maximum = range.maximum.max(value);
            range.last = value;
        }
        if start.elapsed() >= Duration::from_secs(seconds) {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    Ok(observed)
}

pub fn calibrate(args: CalibrateArgs) -> Result<()> {
    if args.rest_seconds == 0 || args.sweep_seconds == 0 {
        return Err("calibration durations must be positive".into());
    }
    if fs::symlink_metadata(&args.output).is_ok() {
        return Err("calibration output already exists; choose a new file".into());
    }
    let device = open(&args.device)?;
    let info = info(&device, &args.device)?;
    let mut calibration = load_mapping(args.calibration.as_deref(), &info)?;
    if !calibration
        .triggers
        .values()
        .any(|t| matches!(t, Trigger::Axis { .. }))
    {
        return Err("this mapping has no analog triggers to calibrate".into());
    }
    let stop = stopped()?;
    eprintln!(
        "Release L2/R2 and leave them untouched for {} seconds.",
        args.rest_seconds
    );
    let rest = observe(&device, args.rest_seconds, &stop)?;
    for trigger in calibration.triggers.values() {
        if let Trigger::Axis { code, .. } = *trigger {
            let range = rest[&code];
            let axis = &info.axes[&code];
            let span = f64::from(axis.maximum) - f64::from(axis.minimum);
            if f64::from(range.maximum) - f64::from(range.minimum) > (span * 0.05).max(1.0) {
                return Err(
                    "trigger moved during the rest phase; retry with both triggers released".into(),
                );
            }
        }
    }
    eprintln!(
        "Fully press and release both L2/R2 repeatedly for {} seconds.",
        args.sweep_seconds
    );
    let sweep = observe(&device, args.sweep_seconds, &stop)?;
    for trigger in calibration.triggers.values_mut() {
        if let Trigger::Axis { code, .. } = *trigger {
            let range = sweep[&code];
            let axis = &info.axes[&code];
            *trigger = trigger.calibrated(
                rest[&code].last,
                range.minimum,
                range.maximum,
                f64::from(axis.maximum) - f64::from(axis.minimum),
            )?;
        }
    }
    calibration.device = Some(info.id);
    calibration.validate()?;
    inspector::write_new_json(&args.output, &calibration)?;
    println!("Saved calibration to {}", args.output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn automatic_mapping_includes_both_sticks_with_unsigned_centers() {
        let mut axes = BTreeMap::new();
        for code in 0..=5 {
            axes.insert(
                code,
                Axis {
                    minimum: 0,
                    maximum: if [2, 5].contains(&code) { 255 } else { 65535 },
                    value: 0,
                },
            );
        }
        for code in [16, 17] {
            axes.insert(
                code,
                Axis {
                    minimum: -1,
                    maximum: 1,
                    value: 0,
                },
            );
        }
        let info = DeviceInfo {
            path: "/dev/input/test".into(),
            name: "test".into(),
            id: DeviceId {
                vendor: 1,
                product: 1,
            },
            keys: vec![304, 305, 307, 308, 310, 311],
            axes,
        };
        let calibration = mapping(&info).unwrap();
        assert_eq!(calibration.schema_version, 2);
        assert_eq!(calibration.thumbsticks.len(), 2);
        assert_eq!(calibration.thumbsticks[&Side::Left].x.center, 32767);
        assert_eq!(calibration.thumbsticks[&Side::Right].x.code, 3);
        assert!(
            Normalizer::new(calibration)
                .unwrap()
                .apply(&[])
                .unwrap()
                .is_neutral()
        );
    }

    #[test]
    fn virtual_keyboards_advertising_gamepad_codes_are_excluded() {
        let keys: Vec<_> = (1..560).collect();
        assert!(!gamepad_capabilities(&keys, &[]));
        assert!(gamepad_capabilities(
            &[304, 305, 307, 308, 310, 311],
            &[2, 5, 16, 17]
        ));
        assert!(gamepad_capabilities(
            &[304, 305, 307, 308, 310, 311, 544, 545, 546, 547],
            &[]
        ));
        assert!(!gamepad_capabilities(&[304, 305], &[16, 17]));
    }

    #[test]
    fn face_button_mapping_matches_linux_physical_positions() {
        let calibration = Calibration::default();
        for (button, code) in [
            (Button::A, KeyCode::BTN_SOUTH),
            (Button::B, KeyCode::BTN_EAST),
            (Button::X, KeyCode::BTN_WEST),
            (Button::Y, KeyCode::BTN_NORTH),
            (Button::L1, KeyCode::BTN_TL),
            (Button::R1, KeyCode::BTN_TR),
        ] {
            assert_eq!(calibration.buttons[&button], code.0);
        }
    }
}
