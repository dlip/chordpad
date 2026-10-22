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

pub fn validate_layout(layout: &Layout) -> Result<()> {
    layout.validate()?;
    for binding in &layout.bindings {
        stroke(&binding.action)?;
    }
    Ok(())
}

pub fn virtual_device() -> Result<VirtualDevice> {
    let mut keys = AttributeSet::<KeyCode>::new();
    // Keyboard codes only, including the standard US section and navigation keys.
    for code in 1..=127 {
        keys.insert(KeyCode(code));
    }
    VirtualDevice::builder()
        .and_then(|builder| builder.name("padtype virtual keyboard").with_keys(&keys))
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
}

impl<S: KeySink> Keyboard<S> {
    pub fn new(sink: S) -> Self {
        Self {
            sink,
            pressed: vec![],
        }
    }
}

impl<S: KeySink> Output for Keyboard<S> {
    fn send(&mut self, action: &Action) -> Result<()> {
        let keys = stroke(action)?;
        self.release_all()?;
        // Track attempted presses too: an error may follow a partial write.
        self.pressed = keys;
        let down: Vec<_> = self
            .pressed
            .iter()
            .map(|k| InputEvent::new(1, k.0, 1))
            .collect();
        if let Err(error) = self.sink.emit(&down) {
            let _ = self.release_all();
            return Err(format!("keyboard press failed: {error}"));
        }
        self.release_all()
    }

    fn release_all(&mut self) -> Result<()> {
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
