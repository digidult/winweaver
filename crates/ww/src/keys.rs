//! The keys of the commands of the window, each with its text, named once. A program adds its
//! own with `Keys::new`.

use eframe::egui::{Key, KeyboardShortcut, Modifiers};

/// A keyboard shortcut with its text.
pub struct Keys {
    pub text: &'static str,
    pub modifiers: Modifiers,
    pub key: Key,
}

impl Keys {
    pub const fn new(text: &'static str, modifiers: Modifiers, key: Key) -> Self {
        Self {
            text,
            modifiers,
            key,
        }
    }

    pub fn shortcut(&self) -> KeyboardShortcut {
        KeyboardShortcut::new(self.modifiers, self.key)
    }
}

pub const CTRL_SHIFT: Modifiers = Modifiers {
    alt: false,
    ctrl: true,
    shift: true,
    mac_cmd: false,
    command: false,
};

pub const NEW: Keys = Keys::new("Ctrl+N", Modifiers::CTRL, Key::N);
pub const OPEN: Keys = Keys::new("Ctrl+O", Modifiers::CTRL, Key::O);
pub const SAVE: Keys = Keys::new("Ctrl+S", Modifiers::CTRL, Key::S);
pub const SAVE_AS: Keys = Keys::new("Ctrl+Shift+S", CTRL_SHIFT, Key::S);
pub const CLOSE: Keys = Keys::new("Ctrl+W", Modifiers::CTRL, Key::W);
pub const SETTINGS: Keys = Keys::new("Ctrl+,", Modifiers::CTRL, Key::Comma);
pub const FIND: Keys = Keys::new("Ctrl+F", Modifiers::CTRL, Key::F);
pub const NEXT_TAB: Keys = Keys::new("Ctrl+Tab", Modifiers::CTRL, Key::Tab);
pub const PREVIOUS_TAB: Keys = Keys::new("Ctrl+Shift+Tab", CTRL_SHIFT, Key::Tab);
pub const TAB_LEFT: Keys = Keys::new("Ctrl+Shift+PgUp", CTRL_SHIFT, Key::PageUp);
pub const TAB_RIGHT: Keys = Keys::new("Ctrl+Shift+PgDn", CTRL_SHIFT, Key::PageDown);
