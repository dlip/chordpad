use super::layout;
use evdev::{
    AbsInfo, AbsoluteAxisCode, AttributeSet, InputEvent, KeyCode, UinputAbsSetup,
    raw_stream::RawDevice, uinput::VirtualDevice,
};
use padtype::{
    model::*,
    output::{self, Keyboard},
    typing::Output,
};
use std::{
    fs::OpenOptions,
    io::{self, BufRead, BufReader},
    os::unix::fs::OpenOptionsExt,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

fn event_node(device: &mut VirtualDevice) -> PathBuf {
    let start = Instant::now();
    loop {
        if let Some(path) = device.enumerate_dev_nodes_blocking().unwrap().next() {
            let path = path.unwrap();
            if OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(&path)
                .is_ok()
            {
                return path;
            }
        }
        assert!(
            start.elapsed() < Duration::from_secs(3),
            "virtual event node did not appear"
        );
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
#[ignore = "creates and exclusively captures a virtual keyboard; requires /dev/uinput and input-node access"]
fn uinput_round_trip_has_balanced_keys_and_report_boundaries() {
    let mut device = output::virtual_device().unwrap();
    let path = event_node(&mut device);
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
        .unwrap();
    let mut capture = RawDevice::from_fd(file.into()).unwrap();
    // Capture only this newly created keyboard before sending anything.
    capture.grab().unwrap();
    let mut keyboard = Keyboard::new(device);
    let actions: Vec<_> = (' '..='~')
        .chain(['\n', '\t'])
        .map(Action::Character)
        .chain([
            Action::Key(Key::Backspace),
            Action::Key(Key::Escape),
            Action::Key(Key::Left),
            Action::Key(Key::F1),
            Action::Key(Key::F2),
            Action::Key(Key::F3),
            Action::Key(Key::F4),
            Action::Key(Key::F5),
            Action::Key(Key::F6),
            Action::Key(Key::F7),
            Action::Key(Key::F8),
            Action::Key(Key::F9),
            Action::Key(Key::F10),
            Action::Key(Key::F11),
            Action::Key(Key::F12),
            Action::Key(Key::MediaPlayPause),
            Action::Key(Key::MediaNext),
            Action::Key(Key::MediaPrevious),
            Action::Key(Key::MediaStop),
            Action::Key(Key::VolumeUp),
            Action::Key(Key::VolumeDown),
            Action::Key(Key::Mute),
            Action::Shortcut(Shortcut {
                modifiers: vec![Modifier::Ctrl, Modifier::Shift],
                key: Key::C,
            }),
        ])
        .collect();
    let mut expected = vec![];
    let mut actual = vec![];
    for action in &actions {
        keyboard.send(action).unwrap();
        let keys = output::stroke(action).unwrap();
        expected.extend(keys.iter().map(|k| (1, k.0, 1)));
        expected.push((0, 0, 0));
        expected.extend(keys.iter().rev().map(|k| (1, k.0, 0)));
        expected.push((0, 0, 0));
        let start = Instant::now();
        while actual.len() < expected.len() && start.elapsed() < Duration::from_secs(2) {
            match capture.fetch_events() {
                Ok(events) => {
                    actual.extend(events.map(|e| (e.event_type().0, e.code(), e.value())))
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5))
                }
                Err(e) => panic!("{e}"),
            }
        }
        assert_eq!(actual, expected, "{}", action.label());
    }
    assert_eq!(actual, expected);
    assert!(capture.get_key_state().unwrap().iter().next().is_none());
    keyboard.release_all().unwrap();
    drop(keyboard);
    println!(
        "Captured {} actions / {} evdev events; no keys remain pressed",
        actions.len(),
        actual.len()
    );
}

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn pump(input: impl io::Read + Send + 'static, prefix: &'static str, tx: Sender<String>) {
    thread::spawn(move || {
        for line in BufReader::new(input).lines() {
            let Ok(line) = line else {
                break;
            };
            if tx.send(format!("{prefix}{line}")).is_err() {
                break;
            }
        }
    });
}

fn until(rx: &Receiver<String>, log: &mut Vec<String>, needle: &str) {
    loop {
        let line = rx
            .recv_timeout(Duration::from_secs(3))
            .unwrap_or_else(|e| panic!("waiting for {needle:?}: {e}; log: {log:?}"));
        let found = line.contains(needle);
        log.push(line);
        if found {
            break;
        }
    }
}

fn emit(device: &mut VirtualDevice, values: &[(u16, u16, i32)]) {
    device
        .emit(
            &values
                .iter()
                .map(|&(kind, code, value)| InputEvent::new(kind, code, value))
                .collect::<Vec<_>>(),
        )
        .unwrap();
}

fn gamepad(name: &str, trigger_maximum: i32) -> VirtualDevice {
    let mut keys = AttributeSet::<KeyCode>::new();
    for code in [304, 305, 307, 308, 310, 311, 314, 315] {
        keys.insert(KeyCode(code));
    }
    let mut builder = VirtualDevice::builder()
        .unwrap()
        .name(name)
        .with_keys(&keys)
        .unwrap();
    for (code, min, max) in [
        (2, 0, trigger_maximum),
        (5, 0, trigger_maximum),
        (16, -1, 1),
        (17, -1, 1),
    ] {
        builder = builder
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode(code),
                AbsInfo::new(0, min, max, 0, 0, 0),
            ))
            .unwrap();
    }
    builder.build().unwrap()
}

#[test]
#[ignore = "creates exclusively grabbed test gamepads; requires /dev/uinput and input-node access"]
fn linux_typing_loop_handles_signals_gestures_and_reconnect() {
    let mut gamepad = gamepad("padtype test gamepad", 255);
    let path = event_node(&mut gamepad);
    let dir = tempfile::tempdir().unwrap();
    let controller = dir.path().join("controller");
    std::os::unix::fs::symlink(&path, &controller).unwrap();
    let layout_path = dir.path().join("layout.json");
    std::fs::write(&layout_path, serde_json::to_string(&layout()).unwrap()).unwrap();
    let mut process = Process(
        Command::new(env!("CARGO_BIN_EXE_padtype"))
            .args(["type", "--device"])
            .arg(&controller)
            .arg("--layout")
            .arg(layout_path)
            .args(["--dry-run", "--grab", "--duration", "15"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let (tx, rx) = mpsc::channel();
    pump(process.0.stdout.take().unwrap(), "OUT: ", tx.clone());
    pump(process.0.stderr.take().unwrap(), "ERR: ", tx);
    let mut log = vec![];
    until(&rx, &mut log, "PAUSED");
    let pid = process.0.id() as i32;
    let signal = |sig| assert_eq!(unsafe { libc::kill(pid, sig) }, 0);
    signal(libc::SIGUSR2);
    until(&rx, &mut log, "READY");
    emit(&mut gamepad, &[(1, 304, 1)]);
    emit(&mut gamepad, &[(1, 304, 0)]);
    until(&rx, &mut log, "OUT: WOULD TYPE 't'");
    for (code, action) in [(315, "Enter"), (314, "Escape")] {
        emit(&mut gamepad, &[(1, code, 1)]);
        emit(&mut gamepad, &[(1, code, 0)]);
        until(&rx, &mut log, &format!("OUT: WOULD TYPE {action}"));
    }
    emit(&mut gamepad, &[(1, 304, 1)]);
    // Confirm the pause command discards a stroke even without its release report.
    thread::sleep(Duration::from_millis(30));
    signal(libc::SIGUSR1);
    until(&rx, &mut log, "pending stroke discarded: true");
    until(&rx, &mut log, "PAUSED");
    emit(&mut gamepad, &[(1, 304, 0)]);
    signal(libc::SIGUSR2);
    until(&rx, &mut log, "READY");
    emit(&mut gamepad, &[(3, 5, 255)]);
    emit(&mut gamepad, &[(1, 304, 1)]);
    emit(&mut gamepad, &[(1, 304, 0)]);
    emit(&mut gamepad, &[(3, 5, 0)]);
    until(&rx, &mut log, "OUT: WOULD TYPE 'T'");
    for status in ["PAUSED", "ACTIVE:"] {
        emit(&mut gamepad, &[(1, 310, 1), (1, 311, 1)]);
        emit(&mut gamepad, &[(3, 2, 255), (3, 5, 255)]);
        until(&rx, &mut log, status);
        emit(
            &mut gamepad,
            &[(1, 310, 0), (1, 311, 0), (3, 2, 0), (3, 5, 0)],
        );
    }
    until(&rx, &mut log, "READY");
    emit(&mut gamepad, &[(1, 304, 1)]);
    emit(&mut gamepad, &[(1, 304, 0)]);
    until(&rx, &mut log, "OUT: WOULD TYPE 't'");
    emit(&mut gamepad, &[(1, 304, 1)]);
    thread::sleep(Duration::from_millis(30));
    let disconnect_start = log.len();
    drop(gamepad);
    until(&rx, &mut log, "disconnected;");
    assert!(
        log[disconnect_start..]
            .iter()
            .any(|s| s.contains("pending stroke discarded: true")),
        "{log:?}"
    );
    until(&rx, &mut log, "Waiting for controller");
    assert!(process.0.try_wait().unwrap().is_none());
    signal(libc::SIGUSR1);
    until(&rx, &mut log, "PAUSED");

    // A different controller occupying the selected path must not be used.
    let mut other = self::gamepad("padtype unrelated gamepad", 255);
    let other_path = event_node(&mut other);
    std::fs::remove_file(&controller).unwrap();
    std::os::unix::fs::symlink(other_path, &controller).unwrap();
    until(&rx, &mut log, "belongs to a different controller");
    emit(&mut other, &[(1, 304, 1)]);
    emit(&mut other, &[(1, 304, 0)]);

    let mut reattached = self::gamepad("padtype test gamepad", 1023);
    let reattached_path = event_node(&mut reattached);
    assert_ne!(reattached_path, path);
    // Held controls on reattachment must not complete a pre-disconnect stroke.
    emit(&mut reattached, &[(1, 304, 1)]);
    std::fs::remove_file(&controller).unwrap();
    std::os::unix::fs::symlink(&reattached_path, &controller).unwrap();
    until(&rx, &mut log, "Reconnected padtype test gamepad. PAUSED");
    let mut capture = RawDevice::open(&reattached_path).unwrap();
    assert_eq!(
        capture.grab().unwrap_err().raw_os_error(),
        Some(libc::EBUSY)
    );
    signal(libc::SIGUSR2);
    until(&rx, &mut log, "ACTIVE:");
    emit(&mut reattached, &[(1, 304, 0)]);
    until(&rx, &mut log, "READY");
    // The reloaded mapping must use the new trigger range, not the old 0..255.
    emit(&mut reattached, &[(3, 5, 300)]);
    emit(&mut reattached, &[(1, 304, 1)]);
    emit(&mut reattached, &[(1, 304, 0)]);
    until(&rx, &mut log, "OUT: WOULD TYPE 't'");
    emit(&mut reattached, &[(3, 5, 1023)]);
    emit(&mut reattached, &[(1, 304, 1)]);
    emit(&mut reattached, &[(1, 304, 0)]);
    until(&rx, &mut log, "OUT: WOULD TYPE 'T'");
    drop(reattached);
    until(&rx, &mut log, "Waiting for controller");
    signal(libc::SIGUSR1);
    until(&rx, &mut log, "PAUSED");
    signal(libc::SIGUSR2);
    until(&rx, &mut log, "ACTIVE:");
    signal(libc::SIGTERM);
    assert!(process.0.wait().unwrap().success());
    log.extend(rx.try_iter());
    assert_eq!(
        log.iter().filter(|s| s.starts_with("OUT:")).count(),
        7,
        "{log:?}"
    );
    println!(
        "Signals, gestures, reconnect with a new event node, grab restoration, held input and stop while disconnected passed"
    );
}

#[test]
#[ignore = "creates an exclusively grabbed test gamepad; requires /dev/uinput and input-node access"]
fn linux_navigation_repeats_without_new_reports_and_stops_on_release() {
    let mut gamepad = gamepad("padtype navigation test gamepad", 255);
    let path = event_node(&mut gamepad);
    let dir = tempfile::tempdir().unwrap();
    let layout_path = dir.path().join("layout.json");
    std::fs::write(
        &layout_path,
        include_str!("../../android/app/src/main/assets/default-layout.json"),
    )
    .unwrap();
    let mut process = Process(
        Command::new(env!("CARGO_BIN_EXE_padtype"))
            .args(["type", "--device"])
            .arg(path)
            .arg("--layout")
            .arg(layout_path)
            .args(["--dry-run", "--grab", "--start-active", "--duration", "5"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let (tx, rx) = mpsc::channel();
    pump(process.0.stdout.take().unwrap(), "OUT: ", tx.clone());
    pump(process.0.stderr.take().unwrap(), "ERR: ", tx);
    let mut log = vec![];
    until(&rx, &mut log, "READY");
    emit(&mut gamepad, &[(3, 5, 255), (1, 310, 1)]);
    emit(&mut gamepad, &[(3, 16, -1)]);
    until(&rx, &mut log, "OUT: WOULD TYPE Left");
    until(&rx, &mut log, "OUT: WOULD TYPE Left");
    // An editor receives fresh Shift on subsequent repeats while navigation stays held.
    emit(&mut gamepad, &[(3, 2, 255)]);
    until(&rx, &mut log, "OUT: WOULD TYPE [Shift] + Left");
    emit(&mut gamepad, &[(3, 16, 0), (1, KeyCode::BTN_NORTH.0, 1)]);
    until(&rx, &mut log, "OUT: WOULD TYPE [Shift] + PageUp");
    until(&rx, &mut log, "OUT: WOULD TYPE [Shift] + PageUp");
    emit(&mut gamepad, &[(3, 5, 0)]);
    // A following menu event confirms the trigger release was consumed.
    emit(&mut gamepad, &[(1, 315, 1)]);
    emit(&mut gamepad, &[(1, 315, 0)]);
    until(&rx, &mut log, "OUT: WOULD TYPE [Shift] + Enter");
    let after_release = log.len();
    emit(&mut gamepad, &[(1, KeyCode::BTN_NORTH.0, 0), (1, 310, 0)]);
    thread::sleep(Duration::from_millis(180));
    log.extend(rx.try_iter());
    assert!(
        !log[after_release..].iter().any(|s| s.starts_with("OUT:")),
        "{log:?}"
    );
    assert_eq!(
        unsafe { libc::kill(process.0.id() as i32, libc::SIGTERM) },
        0
    );
    assert!(process.0.wait().unwrap().success());
}

#[test]
#[ignore = "creates an exclusively grabbed test gamepad; requires /dev/uinput and input-node access"]
fn linux_function_layer_uses_number_combos_without_leaking_space() {
    let mut gamepad = gamepad("padtype function test gamepad", 255);
    let path = event_node(&mut gamepad);
    let dir = tempfile::tempdir().unwrap();
    let layout_path = dir.path().join("layout.json");
    std::fs::write(
        &layout_path,
        include_str!("../../android/app/src/main/assets/default-layout.json"),
    )
    .unwrap();
    let mut process = Process(
        Command::new(env!("CARGO_BIN_EXE_padtype"))
            .args(["type", "--device"])
            .arg(path)
            .arg("--layout")
            .arg(layout_path)
            .args(["--dry-run", "--grab", "--start-active", "--duration", "5"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let (tx, rx) = mpsc::channel();
    pump(process.0.stdout.take().unwrap(), "OUT: ", tx.clone());
    pump(process.0.stderr.take().unwrap(), "ERR: ", tx);
    let mut log = vec![];
    until(&rx, &mut log, "READY");
    emit(&mut gamepad, &[(3, 5, 255), (1, 311, 1)]);
    emit(&mut gamepad, &[(3, 17, -1)]);
    thread::sleep(Duration::from_millis(40));
    log.extend(rx.try_iter());
    assert!(!log.iter().any(|s| s.starts_with("OUT:")), "{log:?}");
    emit(&mut gamepad, &[(3, 17, 0)]);
    until(&rx, &mut log, "OUT: WOULD TYPE F1");
    emit(&mut gamepad, &[(3, 2, 255)]);
    emit(&mut gamepad, &[(3, 16, -1)]);
    emit(&mut gamepad, &[(3, 16, 0)]);
    until(&rx, &mut log, "OUT: WOULD TYPE [Shift] + F7");
    for (button, key) in [
        (KeyCode::BTN_EAST.0, "F10"),
        (KeyCode::BTN_WEST.0, "F11"),
        (KeyCode::BTN_NORTH.0, "F12"),
    ] {
        emit(&mut gamepad, &[(1, button, 1)]);
        emit(&mut gamepad, &[(1, button, 0)]);
        until(&rx, &mut log, &format!("OUT: WOULD TYPE [Shift] + {key}"));
    }
    emit(&mut gamepad, &[(3, 2, 0), (3, 5, 0), (1, 311, 0)]);
    emit(&mut gamepad, &[(1, 315, 1)]);
    emit(&mut gamepad, &[(1, 315, 0)]);
    until(&rx, &mut log, "OUT: WOULD TYPE Enter");
    assert_eq!(
        unsafe { libc::kill(process.0.id() as i32, libc::SIGTERM) },
        0
    );
    assert!(process.0.wait().unwrap().success());
    log.extend(rx.try_iter());
    assert_eq!(
        log.iter().filter(|s| s.starts_with("OUT:")).count(),
        6,
        "{log:?}"
    );
}

#[test]
#[ignore = "creates an exclusively grabbed test gamepad; requires /dev/uinput and input-node access"]
fn disconnected_wait_respects_the_session_duration() {
    let mut gamepad = gamepad("padtype duration test gamepad", 255);
    let path = event_node(&mut gamepad);
    let dir = tempfile::tempdir().unwrap();
    let layout_path = dir.path().join("layout.json");
    std::fs::write(&layout_path, serde_json::to_string(&layout()).unwrap()).unwrap();
    let mut process = Process(
        Command::new(env!("CARGO_BIN_EXE_padtype"))
            .args(["type", "--device"])
            .arg(path)
            .arg("--layout")
            .arg(layout_path)
            .args(["--dry-run", "--grab", "--start-active", "--duration", "2"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let (tx, rx) = mpsc::channel();
    pump(process.0.stdout.take().unwrap(), "OUT: ", tx.clone());
    pump(process.0.stderr.take().unwrap(), "ERR: ", tx);
    let mut log = vec![];
    until(&rx, &mut log, "READY");
    drop(gamepad);
    until(&rx, &mut log, "Waiting for controller");
    until(&rx, &mut log, "duration elapsed;");
    assert!(process.0.wait().unwrap().success());
    println!("Session duration includes time waiting for reconnection");
}
