//! this module allow to play short sound effects
//!
//! ```lua
//! volume = global_volume * effect_volume * distance(position,listener_position)
//! ```
//!
//! but once a sound effect is played at a volume it doesn't change its volume anymore
//!
//! this can lead to weird effects for not so short sound effects and with moving source

use rodio::Sink;
use rodio::Source;

use super::super::RAW_STATE;
use super::super::source;

/// play the sound effect at the volume: `global_volume * effect_volume *
/// distance(position, listener_position)`
pub fn play(effect: usize, pos: [f32;3]) {
    let mut state = unsafe { (*RAW_STATE).write().unwrap() };
    let distance_volume = state.effect.distance_model.distance(pos,state.effect.listener);
    if distance_volume > 0. {
        let source = state.effect.short_sources[effect].clone().amplify(distance_volume);
        let source = source::amplify_ctrl(source, state.effect.final_volume.clone());
        let source = source::play_pause_ctrl(source, state.effect.pause.clone());

        let sink = Sink::new(&state.endpoint);
        sink.append(source);

        state.effect.short_sinks.push(sink);
    }
}

/// play the sound effect at the position of the listener
/// i.e. volume is `global_volume * effect_volume`
pub fn play_on_listener(effect: usize) {
    play(effect,super::listener());
}

/// stop all short sound effects
pub fn stop_all() {
    let mut state = unsafe { (*RAW_STATE).write().unwrap() };
    state.effect.short_sinks.clear();
}
