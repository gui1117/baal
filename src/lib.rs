//! baal (BAsic Audio Library) is build on top of [rodio](https://crates.io/crates/rodio)
//!
//! **it is still in early development**
//!
//! it allows to play three different kind of sounds:
//!
//! * short effects like for shoots
//! * persistent effects like for fans and other ambiant sounds
//! * musics
//!
//! due to rodio backend it support WAV and Vorbis audio format
//!
//! there is no spatialisation
//!
//! see the example and tests for usages

#![warn(missing_docs)]

extern crate rodio;
extern crate mut_static;
#[macro_use] extern crate error_chain;
#[macro_use] extern crate lazy_static;

// use rodio::decoder::DecoderError;
use mut_static::MutStatic;

use std::path::PathBuf;

// use effect::DistanceModel;
// use music::MusicTransition;

lazy_static! {
    static ref STATE: MutStatic<State> = MutStatic::new();
}

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    links {
        MutStatic(mut_static::Error, mut_static::ErrorKind);
    }

    foreign_links {
    }

    errors {
        NoAudioDeviceAvailable {
            description("No audio device available")
        }
    }
}

struct State {
    global_volume: f32,
    endpoint: rodio::Endpoint,
    // music: music::State,
    // effect: effect::State,
}

impl State {
    fn init(setting: &Setting) -> Result<State> {
        let endpoint = rodio::get_default_endpoint()
            .ok_or(ErrorKind::NoAudioDeviceAvailable)?;

        Ok(State {
            global_volume: setting.global_volume,
            // effect: try!(effect::State::init(setting, &endpoint)),
            // music: try!(music::State::init(setting)),
            endpoint: endpoint,
        })
    }
}

/// init the audio player
pub fn init(setting: &Setting) -> Result<()> {
    STATE.set(State::init(setting)?)?;
    Ok(())
}

// /// close the audio player, it can be init again.
// pub fn close() {
//     unsafe {
//         if !RAW_STATE.is_null() {
//             let mutex_state = Box::from_raw(RAW_STATE);
//             let _ = mutex_state.read().unwrap();
//         }
//         RAW_STATE = 0 as *mut RwLock<State>;
//     }
// }

// /// reset audio from setting on the fly
// pub fn reset(setting: &Setting) -> Result<(),InitError> {
//     unsafe {
//         let mut state = (*RAW_STATE).write().unwrap();

//         try!(state.reset(setting));

//         Ok(())
//     }
// }

/// set the global volume
pub fn set_global_volume(v: f32) {
    let mut state = unsafe { (*RAW_STATE).write().unwrap() };
    state.global_volume = v;
    update_volume(&mut *state);
}

// #[inline]
// fn update_volume(state: &mut State) {
//     music::update_volume(state);
//     effect::update_volume(state);
// }

// /// return the global volume
// pub fn global_volume() -> f32 {
//     let state = unsafe { (*RAW_STATE).read().unwrap() };
//     state.global_volume
// }

