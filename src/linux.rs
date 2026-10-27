use crate::{
    Result,
    controller::{Packet, *},
    inspector::{self, CalibrateArgs, Header, InspectArgs, Record, Session},
    model::{Button, Side},
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,
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

fn kernel_driver(path: &Path) -> Option<String> {
    let node = fs::canonicalize(path).ok()?;
    let sysfs = fs::canonicalize(
        Path::new("/sys/class/input")
            .join(node.file_name()?)
            .join("device"),
    )
    .ok()?;
    sysfs.ancestors().find_map(|parent| {
        let driver = fs::read_link(parent.join("driver")).ok()?;
        driver.file_name()?.to_str().map(str::to_owned)
    })
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
        driver: kernel_driver(path),
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
    if device.driver.as_deref() == Some("xpad") {
        // xpad uses the legacy BTN_X/BTN_Y aliases rather than geometric positions.
        map.buttons.insert(Button::X, KeyCode::BTN_NORTH.0);
        map.buttons.insert(Button::Y, KeyCode::BTN_WEST.0);
    }
    map.menu_buttons
        .retain(|_, code| device.keys.contains(code));
    if !(device.axes.contains_key(&16) && device.axes.contains_key(&17)) {
        map.dpad = DpadSource::Buttons {
            up: 544,
            down: 545,
            left: 546,
            right: 547,
        };
    }
    for (button, candidates, key) in [
        // Prefer dedicated trigger axes: Z/RZ can also describe the right stick.
        (Button::L2, [10, 2, 21], 312),
        (Button::R2, [9, 5, 20], 313),
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
    Tick,
}

struct ControlFlags {
    pause: Arc<AtomicBool>,
    resume: Arc<AtomicBool>,
}

fn at_us(started: Instant) -> u64 {
    started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
}

fn finish_reason(
    stop: &AtomicBool,
    duration: Option<u64>,
    started: Instant,
) -> Option<&'static str> {
    if stop.load(Ordering::SeqCst) {
        Some("interrupted")
    } else if duration.is_some_and(|seconds| started.elapsed() >= Duration::from_secs(seconds)) {
        Some("duration elapsed")
    } else {
        None
    }
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
        if let Some(reason) = finish_reason(stop, duration, started) {
            return Ok(reason);
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
                if controls.is_some() {
                    consume(at_us(started), LiveEvent::Tick)?;
                }
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
    let layout = args
        .layout
        .as_deref()
        .map(|path| crate::typing::TypingLayout::parse(inspector::read_json(path)?))
        .transpose()?;
    let stop = stopped()?;
    let mut session = Session::new(
        Header::with_layout(calibration.clone(), layout),
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
            LiveEvent::Tick => unreachable!(),
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
            Event::Chord { .. } | Event::Menu { .. } => {}
        }
    }
}

fn control_commands<O: crate::typing::Output>(
    controls: &ControlFlags,
    runtime: &mut crate::typing::Runtime<O>,
) -> Result<()> {
    if controls.pause.swap(false, Ordering::SeqCst) {
        feedback(&runtime.set_paused(true)?);
    }
    if controls.resume.swap(false, Ordering::SeqCst) {
        feedback(&runtime.set_paused(false)?);
    }
    Ok(())
}

fn typing_status(paused: bool) {
    eprintln!(
        "{}",
        if paused {
            "PAUSED"
        } else {
            "ACTIVE: release all controls to arm"
        }
    );
}

fn reopen(
    path: &Path,
    expected: &DeviceInfo,
    calibration_path: Option<&Path>,
    grab: bool,
) -> Result<(RawDevice, Calibration)> {
    let mut device = open(path)?;
    let info = info(&device, path)?;
    if info.id != expected.id || info.name != expected.name {
        return Err(format!(
            "{} belongs to a different controller; waiting for {} ({:04x}:{:04x})",
            path.display(),
            expected.name,
            expected.id.vendor,
            expected.id.product
        ));
    }
    let calibration = load_mapping(calibration_path, &info)?;
    if grab {
        device
            .grab()
            .map_err(|e| format!("cannot grab controller: {e}"))?;
    }
    Ok((device, calibration))
}

pub fn type_layout(args: crate::typing::TypeArgs) -> Result<()> {
    use crate::{
        output,
        typing::{DryRun, Output, Runtime, TypingLayout},
    };
    if args.duration == Some(0) {
        return Err("duration must be positive".into());
    }
    let layout = TypingLayout::parse(inspector::read_json(&args.layout)?)?;
    for action in layout.actions()? {
        output::stroke(&action)?;
    }
    layout.validate_pause(&args.pause_buttons)?;
    let mut device = open(&args.device)?;
    let info = info(&device, &args.device)?;
    let mut calibration = load_mapping(args.calibration.as_deref(), &info)?;
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
    let mut runtime =
        Runtime::with_layout(layout, &args.pause_buttons, !args.start_active, output)?;
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
    let result = (|| loop {
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
                    LiveEvent::Tick => {}
                    LiveEvent::Frame(events) => {
                        feedback(&runtime.update(&normalizer.apply(&events)?)?)
                    }
                    LiveEvent::Reset(reason) => {
                        feedback(&runtime.reset(reason)?);
                        normalizer = Normalizer::new(calibration.clone())?;
                    }
                    LiveEvent::Control(events) => {
                        feedback(&runtime.reset("control command")?);
                        normalizer = Normalizer::new(calibration.clone())?;
                        control_commands(&controls, &mut runtime)?;
                        feedback(&runtime.update(&normalizer.apply(&events)?)?);
                    }
                }
                feedback(&runtime.repeat()?);
                if runtime.paused() != before {
                    typing_status(runtime.paused());
                }
                Ok(())
            },
        );
        if args.grab {
            let _ = device.ungrab();
        }
        let reason = result?;
        if reason != "disconnected" {
            return Ok(reason);
        }
        feedback(&runtime.reset("disconnected")?);
        drop(device);
        eprintln!(
            "Waiting for controller at {} to reconnect; Ctrl-C or SIGTERM stops.",
            args.device.display()
        );
        let mut last_error = None;
        loop {
            if let Some(reason) = finish_reason(&stop, args.duration, started) {
                return Ok(reason);
            }
            let before = runtime.paused();
            control_commands(&controls, &mut runtime)?;
            if runtime.paused() != before {
                typing_status(runtime.paused());
            }
            match reopen(&args.device, &info, args.calibration.as_deref(), args.grab) {
                Ok((reattached, mapping)) => {
                    device = reattached;
                    calibration = mapping;
                    normalizer = Normalizer::new(calibration.clone())?;
                    eprintln!(
                        "Reconnected {}. {}; release all controls before typing.",
                        info.name,
                        if runtime.paused() { "PAUSED" } else { "ACTIVE" }
                    );
                    break;
                }
                Err(error) => {
                    if last_error.as_ref() != Some(&error) {
                        eprintln!("Reconnect: {error}");
                        last_error = Some(error);
                    }
                }
            }
            thread::sleep(Duration::from_millis(250));
        }
    })();
    let cleanup = runtime.reset(result.as_ref().copied().unwrap_or("input/output error"));
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
    fn test_device(driver: Option<&str>) -> DeviceInfo {
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
        DeviceInfo {
            path: "/dev/input/test".into(),
            name: "test".into(),
            driver: driver.map(str::to_owned),
            id: DeviceId {
                vendor: 1,
                product: 1,
            },
            keys: vec![304, 305, 307, 308, 310, 311, 314, 315],
            axes,
        }
    }

    #[test]
    fn automatic_mapping_includes_both_sticks_with_unsigned_centers() {
        let info = test_device(None);
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
    fn pocket_triggers_use_brake_and_gas_instead_of_idle_z_and_rz() {
        let mut device = test_device(Some("hid-generic"));
        device.name = "GameSir-Pocket 1".into();
        device.id = DeviceId {
            vendor: 0x3537,
            product: 0x1132,
        };
        device.keys = (304..=319).collect();
        device.axes.retain(|code, _| ![3, 4].contains(code));
        for code in [0, 1, 2, 5, 9, 10] {
            device.axes.insert(
                code,
                Axis {
                    minimum: 0,
                    maximum: 255,
                    value: if [9, 10].contains(&code) { 0 } else { 128 },
                },
            );
        }
        let calibration = mapping(&device).unwrap();
        assert_eq!(calibration.triggers[&Button::L2].source(), (ABS, 10));
        assert_eq!(calibration.triggers[&Button::R2].source(), (ABS, 9));
        let mut normalizer = Normalizer::new(calibration).unwrap();
        let axis = |code, value| RawEvent {
            event_type: ABS,
            code,
            value,
        };
        assert!(
            normalizer
                .apply(&[axis(0, 128), axis(1, 128), axis(2, 128), axis(5, 128)])
                .unwrap()
                .is_neutral()
        );
        for (left, right, expected) in [
            (255, 0, vec![Button::L2]),
            (255, 255, vec![Button::L2, Button::R2]),
            (0, 255, vec![Button::R2]),
            (0, 0, vec![]),
        ] {
            let snapshot = normalizer
                .apply(&[
                    RawEvent {
                        event_type: KEY,
                        code: 312,
                        value: i32::from(left != 0),
                    },
                    axis(10, left),
                    RawEvent {
                        event_type: KEY,
                        code: 313,
                        value: i32::from(right != 0),
                    },
                    axis(9, right),
                ])
                .unwrap();
            assert_eq!(snapshot.buttons, expected);
            assert_eq!(snapshot.l2, f64::from(left != 0));
            assert_eq!(snapshot.r2, f64::from(right != 0));
        }
    }

    #[test]
    fn automatic_menu_mapping_only_uses_available_buttons() {
        use crate::model::MenuButton;
        let mut device = test_device(None);
        let calibration = mapping(&device).unwrap();
        assert_eq!(calibration.menu_buttons[&MenuButton::Start], 315);
        assert_eq!(calibration.menu_buttons[&MenuButton::Select], 314);
        device.keys.retain(|code| *code != 315);
        let calibration = mapping(&device).unwrap();
        assert_eq!(calibration.menu_buttons.len(), 1);
        assert_eq!(calibration.menu_buttons[&MenuButton::Select], 314);
        device.keys.retain(|code| *code != 314);
        assert!(mapping(&device).unwrap().menu_buttons.is_empty());
    }

    #[test]
    fn xpad_mapping_keeps_adjacent_x_and_y_chords_valid() {
        use crate::{
            chord::{Event, Recognizer},
            model::*,
        };
        let binding = |c, buttons, held| Binding {
            action: Action::Character(c),
            chord: Chord {
                dpad: Dpad::Neutral,
                buttons,
            },
            held,
            held_dpad: Dpad::Neutral,
        };
        let layout = Layout {
            schema_version: SCHEMA_VERSION,
            cost_model_version: COST_MODEL_VERSION,
            strategy: Strategy {
                independent: true,
                name: "independent".into(),
                shift: Some(Button::L2),
                layer: Some(Button::R2),
            },
            profile: Profile {
                exclude_opposite_face_pairs: true,
                ..Profile::default()
            },
            weights: Weights::default(),
            normalization: Metrics {
                effort: 1.0,
                activations: 1.0,
                movement: 1.0,
                coverage: 1.0,
                learning: 1.0,
            },
            recognition: "independent-release".into(),
            bindings: vec![
                binding('g', vec![Button::B, Button::Y], vec![]),
                binding('G', vec![Button::B, Button::Y], vec![Button::L2]),
                binding('x', vec![Button::A, Button::X], vec![]),
                binding('X', vec![Button::A, Button::X], vec![Button::L2]),
            ],
        };
        let calibration = mapping(&test_device(Some("xpad"))).unwrap();
        assert_eq!(calibration.buttons[&Button::X], 307);
        assert_eq!(calibration.buttons[&Button::Y], 308);
        let mut normalizer = Normalizer::new(calibration).unwrap();
        let mut engine = Recognizer::new(
            layout.strategy.clone(),
            layout.profile.clone(),
            Some(layout),
        )
        .unwrap();
        let raw = |code, value| RawEvent {
            event_type: KEY,
            code,
            value,
        };
        engine.update(&normalizer.apply(&[]).unwrap());
        for (shifted, codes, expected) in [
            (false, [305, 308], 'g'),
            (false, [304, 307], 'x'),
            (true, [305, 308], 'G'),
            (true, [304, 307], 'X'),
        ] {
            let shift = RawEvent {
                event_type: ABS,
                code: 2,
                value: if shifted { 255 } else { 0 },
            };
            engine.update(&normalizer.apply(&[shift, raw(codes[0], 1)]).unwrap());
            engine.update(&normalizer.apply(&[raw(codes[1], 1)]).unwrap());
            assert!(
                engine
                    .update(&normalizer.apply(&[raw(codes[0], 0)]).unwrap())
                    .is_empty()
            );
            let events = engine.update(&normalizer.apply(&[raw(codes[1], 0)]).unwrap());
            assert!(
                matches!(events.as_slice(),[Event::Chord { action:Some(Action::Character(c)), .. }] if *c == expected),
                "{events:?}"
            );
        }
        engine.update(
            &normalizer
                .apply(&[
                    RawEvent {
                        event_type: ABS,
                        code: 2,
                        value: 0,
                    },
                    raw(304, 1),
                    raw(308, 1),
                ])
                .unwrap(),
        );
        let events = engine.update(&normalizer.apply(&[raw(304, 0), raw(308, 0)]).unwrap());
        assert!(matches!(events.as_slice(), [Event::Cancelled { .. }]));
        let generic = mapping(&test_device(Some("hid-generic"))).unwrap();
        assert_eq!(generic.buttons[&Button::X], KeyCode::BTN_WEST.0);
        assert_eq!(generic.buttons[&Button::Y], KeyCode::BTN_NORTH.0);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("custom.json");
        fs::write(
            &path,
            serde_json::to_string(&Calibration::default()).unwrap(),
        )
        .unwrap();
        let mut custom_device = test_device(Some("xpad"));
        custom_device.keys.extend([317, 318]);
        let custom = load_mapping(Some(&path), &custom_device).unwrap();
        assert_eq!(custom.buttons[&Button::X], 308);
        assert_eq!(custom.buttons[&Button::Y], 307);
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
