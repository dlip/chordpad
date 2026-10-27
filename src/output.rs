use crate::{
    Result,
    model::{Action, Key, Layout, Modifier},
    typing::Output,
};
use evdev::{AttributeSet, InputEvent, KeyCode, uinput::VirtualDevice};
use std::io;

pub fn key_code(key: Key) -> KeyCode {
    use Key::*;
    match key {
        Backspace => KeyCode::KEY_BACKSPACE,
        Escape => KeyCode::KEY_ESC,
        Enter => KeyCode::KEY_ENTER,
        Tab => KeyCode::KEY_TAB,
        Space => KeyCode::KEY_SPACE,
        Delete => KeyCode::KEY_DELETE,
        Insert => KeyCode::KEY_INSERT,
        Left => KeyCode::KEY_LEFT,
        Right => KeyCode::KEY_RIGHT,
        Up => KeyCode::KEY_UP,
        Down => KeyCode::KEY_DOWN,
        Home => KeyCode::KEY_HOME,
        End => KeyCode::KEY_END,
        PageUp => KeyCode::KEY_PAGEUP,
        PageDown => KeyCode::KEY_PAGEDOWN,
        A => KeyCode::KEY_A,
        B => KeyCode::KEY_B,
        C => KeyCode::KEY_C,
        D => KeyCode::KEY_D,
        E => KeyCode::KEY_E,
        F => KeyCode::KEY_F,
        G => KeyCode::KEY_G,
        H => KeyCode::KEY_H,
        I => KeyCode::KEY_I,
        J => KeyCode::KEY_J,
        K => KeyCode::KEY_K,
        L => KeyCode::KEY_L,
        M => KeyCode::KEY_M,
        N => KeyCode::KEY_N,
        O => KeyCode::KEY_O,
        P => KeyCode::KEY_P,
        Q => KeyCode::KEY_Q,
        R => KeyCode::KEY_R,
        S => KeyCode::KEY_S,
        T => KeyCode::KEY_T,
        U => KeyCode::KEY_U,
        V => KeyCode::KEY_V,
        W => KeyCode::KEY_W,
        X => KeyCode::KEY_X,
        Y => KeyCode::KEY_Y,
        Z => KeyCode::KEY_Z,
        F1 => KeyCode::KEY_F1,
        F2 => KeyCode::KEY_F2,
        F3 => KeyCode::KEY_F3,
        F4 => KeyCode::KEY_F4,
        F5 => KeyCode::KEY_F5,
        F6 => KeyCode::KEY_F6,
        F7 => KeyCode::KEY_F7,
        F8 => KeyCode::KEY_F8,
        F9 => KeyCode::KEY_F9,
        F10 => KeyCode::KEY_F10,
        F11 => KeyCode::KEY_F11,
        F12 => KeyCode::KEY_F12,
        PrintScreen => KeyCode::KEY_SYSRQ,
        MediaPlayPause => KeyCode::KEY_PLAYPAUSE,
        MediaNext => KeyCode::KEY_NEXTSONG,
        MediaPrevious => KeyCode::KEY_PREVIOUSSONG,
        MediaStop => KeyCode::KEY_STOPCD,
        VolumeUp => KeyCode::KEY_VOLUMEUP,
        VolumeDown => KeyCode::KEY_VOLUMEDOWN,
        Mute => KeyCode::KEY_MUTE,
    }
}

pub fn stroke(action: &Action) -> Result<Vec<KeyCode>> {
    action.validate()?;
    match action {
        Action::Key(key) => Ok(vec![key_code(*key)]),
        Action::Shortcut(shortcut) => Ok(shortcut
            .modifiers
            .iter()
            .map(|m| match m {
                Modifier::Ctrl => KeyCode::KEY_LEFTCTRL,
                Modifier::Alt => KeyCode::KEY_LEFTALT,
                Modifier::Shift => KeyCode::KEY_LEFTSHIFT,
                Modifier::Meta => KeyCode::KEY_LEFTMETA,
            })
            .chain(std::iter::once(key_code(shortcut.key)))
            .collect()),
        Action::Character(c) => {
            let (base, shifted) = action.group(true);
            let Action::Character(base) = base else {
                unreachable!()
            };
            let key = match base {
                '\n' => KeyCode::KEY_ENTER,
                '\t' => KeyCode::KEY_TAB,
                ' ' => KeyCode::KEY_SPACE,
                'a' => KeyCode::KEY_A,
                'b' => KeyCode::KEY_B,
                'c' => KeyCode::KEY_C,
                'd' => KeyCode::KEY_D,
                'e' => KeyCode::KEY_E,
                'f' => KeyCode::KEY_F,
                'g' => KeyCode::KEY_G,
                'h' => KeyCode::KEY_H,
                'i' => KeyCode::KEY_I,
                'j' => KeyCode::KEY_J,
                'k' => KeyCode::KEY_K,
                'l' => KeyCode::KEY_L,
                'm' => KeyCode::KEY_M,
                'n' => KeyCode::KEY_N,
                'o' => KeyCode::KEY_O,
                'p' => KeyCode::KEY_P,
                'q' => KeyCode::KEY_Q,
                'r' => KeyCode::KEY_R,
                's' => KeyCode::KEY_S,
                't' => KeyCode::KEY_T,
                'u' => KeyCode::KEY_U,
                'v' => KeyCode::KEY_V,
                'w' => KeyCode::KEY_W,
                'x' => KeyCode::KEY_X,
                'y' => KeyCode::KEY_Y,
                'z' => KeyCode::KEY_Z,
                '0' => KeyCode::KEY_0,
                '1' => KeyCode::KEY_1,
                '2' => KeyCode::KEY_2,
                '3' => KeyCode::KEY_3,
                '4' => KeyCode::KEY_4,
                '5' => KeyCode::KEY_5,
                '6' => KeyCode::KEY_6,
                '7' => KeyCode::KEY_7,
                '8' => KeyCode::KEY_8,
                '9' => KeyCode::KEY_9,
                '`' => KeyCode::KEY_GRAVE,
                '-' => KeyCode::KEY_MINUS,
                '=' => KeyCode::KEY_EQUAL,
                '[' => KeyCode::KEY_LEFTBRACE,
                ']' => KeyCode::KEY_RIGHTBRACE,
                '\\' => KeyCode::KEY_BACKSLASH,
                ';' => KeyCode::KEY_SEMICOLON,
                '\'' => KeyCode::KEY_APOSTROPHE,
                ',' => KeyCode::KEY_COMMA,
                '.' => KeyCode::KEY_DOT,
                '/' => KeyCode::KEY_SLASH,
                _ => {
                    return Err(format!(
                        "US keyboard output does not support character {c:?}; use a layout containing printable ASCII, Enter and Tab"
                    ));
                }
            };
            Ok(if shifted {
                vec![KeyCode::KEY_LEFTSHIFT, key]
            } else {
                vec![key]
            })
        }
    }
}

pub fn stroke_with_modifiers(action: &Action, modifiers: &[Modifier]) -> Result<Vec<KeyCode>> {
    let keys = stroke(action)?;
    if modifiers.is_empty() {
        return Ok(keys);
    }
    let key = *keys.last().unwrap();
    let mut combined = vec![];
    for (modifier, code) in [
        (Modifier::Ctrl, KeyCode::KEY_LEFTCTRL),
        (Modifier::Alt, KeyCode::KEY_LEFTALT),
        (Modifier::Shift, KeyCode::KEY_LEFTSHIFT),
        (Modifier::Meta, KeyCode::KEY_LEFTMETA),
    ] {
        if modifiers.contains(&modifier) || keys[..keys.len() - 1].contains(&code) {
            combined.push(code);
        }
    }
    combined.push(key);
    Ok(combined)
}

pub fn validate_layout(layout: &Layout) -> Result<()> {
    layout.validate()?;
    for binding in &layout.bindings {
        stroke(&binding.action)?;
    }
    Ok(())
}

pub fn virtual_device() -> Result<VirtualDevice> {
    let mut keys = AttributeSet::<KeyCode>::new();
    // Keyboard codes only, including the standard US section and consumer keys.
    for code in 1..=127 {
        keys.insert(KeyCode(code));
    }
    for key in [
        Key::MediaPlayPause,
        Key::MediaNext,
        Key::MediaPrevious,
        Key::MediaStop,
    ] {
        keys.insert(key_code(key));
    }
    VirtualDevice::builder()
        .and_then(|builder| builder.name("chordpad virtual keyboard").with_keys(&keys))
        .and_then(|builder| builder.build())
        .map_err(|e| format!("/dev/uinput: {e}; read/write access is required for keyboard output (use --dry-run to test without it)"))
}

pub trait KeySink {
    fn emit(&mut self, events: &[InputEvent]) -> io::Result<()>;
}

impl KeySink for VirtualDevice {
    fn emit(&mut self, events: &[InputEvent]) -> io::Result<()> {
        VirtualDevice::emit(self, events)
    }
}

pub struct Keyboard<S: KeySink> {
    sink: S,
    pressed: Vec<KeyCode>,
    held_modifiers: Vec<KeyCode>,
}

impl<S: KeySink> Keyboard<S> {
    pub fn new(sink: S) -> Self {
        Self {
            sink,
            pressed: vec![],
            held_modifiers: vec![],
        }
    }

    fn transition(&mut self, keys: &[KeyCode]) -> Result<()> {
        let up: Vec<_> = self
            .pressed
            .iter()
            .rev()
            .filter(|k| !keys.contains(k))
            .map(|k| InputEvent::new(1, k.0, 0))
            .collect();
        if !up.is_empty() {
            self.sink
                .emit(&up)
                .map_err(|e| format!("keyboard release failed: {e}"))?;
            self.pressed.retain(|k| keys.contains(k));
        }
        let down: Vec<_> = keys
            .iter()
            .filter(|k| !self.pressed.contains(k))
            .map(|k| InputEvent::new(1, k.0, 1))
            .collect();
        // A failed write can still deliver some presses; keep them for cleanup.
        let added: Vec<_> = keys
            .iter()
            .filter(|k| !self.pressed.contains(k))
            .copied()
            .collect();
        self.pressed.extend(added);
        if !down.is_empty() {
            self.sink
                .emit(&down)
                .map_err(|e| format!("keyboard press failed: {e}"))?;
        }
        Ok(())
    }

    fn transition_or_release(&mut self, keys: &[KeyCode]) -> Result<()> {
        if let Err(error) = self.transition(keys) {
            let _ = self.release_all();
            return Err(error);
        }
        Ok(())
    }
}

impl<S: KeySink> Output for Keyboard<S> {
    fn send(&mut self, action: &Action) -> Result<()> {
        self.send_modified(action, &[])
    }

    fn send_modified(&mut self, action: &Action, modifiers: &[Modifier]) -> Result<()> {
        let keys = stroke_with_modifiers(action, modifiers)?;
        self.transition_or_release(&keys)?;
        self.transition_or_release(&self.held_modifiers.clone())
    }

    fn set_modifiers(&mut self, modifiers: &[Modifier]) -> Result<()> {
        let keys: Vec<_> = [
            (Modifier::Ctrl, KeyCode::KEY_LEFTCTRL),
            (Modifier::Alt, KeyCode::KEY_LEFTALT),
            (Modifier::Shift, KeyCode::KEY_LEFTSHIFT),
            (Modifier::Meta, KeyCode::KEY_LEFTMETA),
        ]
        .into_iter()
        .filter(|(m, _)| modifiers.contains(m))
        .map(|(_, k)| k)
        .collect();
        self.transition_or_release(&keys)?;
        self.held_modifiers = keys;
        Ok(())
    }

    fn release_all(&mut self) -> Result<()> {
        self.held_modifiers.clear();
        if self.pressed.is_empty() {
            return Ok(());
        }
        let up: Vec<_> = self
            .pressed
            .iter()
            .rev()
            .map(|k| InputEvent::new(1, k.0, 0))
            .collect();
        self.sink
            .emit(&up)
            .map_err(|e| format!("keyboard release failed: {e}"))?;
        self.pressed.clear();
        Ok(())
    }
}

impl<S: KeySink> Drop for Keyboard<S> {
    fn drop(&mut self) {
        let _ = self.release_all();
    }
}
