//! this module allow to play persistent sound effects
//!
//! ```lua
//! volume = global_volume * effect_volume * sum(distance(position,listener_position))
//! ```
//!
//! but once a sound effect is played at a volume it doesn't change its volume anymore
//!
//! this can lead to weird effects for not so short sound effects and with moving source
//!
//! also if its volume is zero then the sound is not played at all

use super::super::RAW_STATE;

use std::sync::atomic::Ordering::Relaxed;

/// add a new source of the effect
pub fn add_position(effect: usize, pos: [f32;3]) {
    let mut state = unsafe { (*RAW_STATE).write().unwrap() };
    state.effect.persistent_positions[effect].push(pos);
}

/// add a vec of new sources of the effect
pub fn add_positions(effect: usize, mut pos: Vec<[f32;3]>) {
    let mut state = unsafe { (*RAW_STATE).write().unwrap() };
    state.effect.persistent_positions[effect].append(&mut pos);
}

/// add a vec of new sources of the effects
pub fn add_positions_for_all(all: Vec<(usize,Vec<[f32;3]>)>) {
    let mut state = unsafe { (*RAW_STATE).write().unwrap() };
    for (effect,mut pos) in all {
        state.effect.persistent_positions[effect].append(&mut pos);
    }
}

/// remove all sources of the effect
pub fn clear_positions(effect: usize) {
    let mut state = unsafe { (*RAW_STATE).write().unwrap() };
    state.effect.persistent_positions[effect].clear()
}

/// remove all sources of all effects
pub fn clear_positions_for_all() {
    let mut state = unsafe { (*RAW_STATE).write().unwrap() };
    for p in &mut state.effect.persistent_positions {
        p.clear()
    }
}

/// update the volume of effect computed from sources position and listener position at the
/// moment of this call
pub fn update_volume(effect: usize) {
    let state = unsafe { (*RAW_STATE).read().unwrap() };
    let volume = state.effect.persistent_positions[effect].iter()
        .fold(0f32, |acc, &pos| acc + state.effect.distance_model.distance(pos,state.effect.listener));

    state.effect.persistent_final_volumes[effect].store((volume * 10_000f32) as usize, Relaxed);
}

/// update the volume of all effect
pub fn update_volume_for_all() {
    let state = unsafe { (*RAW_STATE).read().unwrap() };

    for (positions,final_volume) in state.effect.persistent_positions.iter().zip(state.effect.persistent_final_volumes.iter()) {
        let volume = positions.iter()
            .fold(0f32, |acc, &pos| acc + state.effect.distance_model.distance(pos,state.effect.listener));

        final_volume.store((volume * 10_000f32) as usize, Relaxed);
    }
}
