//! this module allow to play short sound effects
//!
//! ```lua
//! volume = global_volume * effect_volume * distance(position,listener_position)
//! ```
//!
//! but once a sound effect is played at a volume it doesn't change its volume anymore
//!
//! this can lead to weird effects for not so short sound effects and with moving source

use super::super::{RAW_STATE, Msg};

/// play the sound effect at the volume: `global_volume * effect_volume *
/// distance(position, listener_position)`
pub fn play(effect: usize, pos: [f32;3]) {
    let state = unsafe { (*RAW_STATE).read().unwrap() };
    let volume = state.global_volume * state.effect_volume * state.distance_model.distance(pos,state.listener);
    if volume > 0. {
        state.sender.send(Msg::PlayShortEffect(effect,volume)).unwrap();
    }
}

/// play the sound effect at the position of the listener
/// i.e. volume is `global_volume * effect_volume`
pub fn play_on_listener(effect: usize) {
    play(effect,super::listener());
}

/// stop all short sound effects
pub fn stop_all() {
    let state = unsafe { (*RAW_STATE).read().unwrap() };
    state.sender.send(Msg::StopAllShortEffects).unwrap();
}
