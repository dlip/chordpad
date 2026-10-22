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

#[test]
#[ignore = "creates an exclusively grabbed test gamepad; requires /dev/uinput and input-node access"]
fn linux_typing_loop_handles_signals_gestures_and_disconnect() {
    let mut keys = AttributeSet::<KeyCode>::new();
    for code in [304, 305, 307, 308, 310, 311] {
        keys.insert(KeyCode(code));
    }
    let mut builder = VirtualDevice::builder()
        .unwrap()
        .name("padtype test gamepad")
        .with_keys(&keys)
        .unwrap();
    for (code, min, max) in [(2, 0, 255), (5, 0, 255), (16, -1, 1), (17, -1, 1)] {
        builder = builder
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode(code),
                AbsInfo::new(0, min, max, 0, 0, 0),
            ))
            .unwrap();
    }
    let mut gamepad = builder.build().unwrap();
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
    let signal = |sig| assert_eq!(unsafe { libc::kill(process.0.id() as i32, sig) }, 0);
    signal(libc::SIGUSR2);
    until(&rx, &mut log, "READY");
    emit(&mut gamepad, &[(1, 304, 1)]);
    emit(&mut gamepad, &[(1, 304, 0)]);
    until(&rx, &mut log, "OUT: WOULD TYPE 't'");
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
    assert!(process.0.wait().unwrap().success());
    log.extend(rx.try_iter());
    assert_eq!(
        log.iter().filter(|s| s.starts_with("OUT:")).count(),
        3,
        "{log:?}"
    );
    println!(
        "Signal pause/resume, reserved gesture, shifted output and mid-stroke disconnect passed"
    );
}
