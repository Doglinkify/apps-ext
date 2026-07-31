//! Input handling — keys are normalised into [`Key`] by the platform
//! backend; this module just maps keys to player actions.

use crate::platform::Key;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Action {
    TogglePlay,
    Quit,
    Open,
    SeekBackward,
    SeekForward,
    None,
}

pub fn action_for(key: Option<Key>) -> Action {
    match key {
        Some(Key::Space) => Action::TogglePlay,
        Some(Key::Q) | Some(Key::Escape) => Action::Quit,
        Some(Key::O) => Action::Open,
        Some(Key::Left) => Action::SeekBackward,
        Some(Key::Right) => Action::SeekForward,
        _ => Action::None,
    }
}
