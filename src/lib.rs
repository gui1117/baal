//! baal (BAsic Audio Library) is build on top of [rodio](https://crates.io/crates/rodio)
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

pub mod music;
pub mod effect;

mod source;

use std::sync::RwLock;
use std::path::PathBuf;
use std::fmt;
use std::io;

use rodio::decoder::DecoderError;

use effect::DistanceModel;
use music::MusicTransition;

static mut RAW_STATE: *mut RwLock<State> = 0 as *mut RwLock<State>;

#[derive(Clone,Debug,PartialEq)]
/// set musics, effects, volumes and audio player.
///
/// impl rustc_decodable and rustc_encodable
pub struct Setting {
    /// the base directory of effects
    pub effect_dir: PathBuf,

    /// the base directory of musics
    pub music_dir: PathBuf,

    /// global volume in [0,1]
    pub global_volume: f32,

    /// music volume in [0,1]
    pub music_volume: f32,

    /// effect volume in [0,1]
    pub effect_volume: f32,

    /// distance model for effect volume computation
    pub distance_model: DistanceModel,

    /// the kind of transition between musics
    pub music_transition: MusicTransition,

    /// the list of short effects
    ///
    /// each effect is identified by its position in the vector
    pub short_effects: Vec<PathBuf>,

    /// the list of persistent effects
    ///
    /// each effect is identified by its position in the vector
    pub persistent_effects: Vec<PathBuf>,

    /// the list of music
    ///
    /// each music is identified by its position in the vector
    pub musics: Vec<PathBuf>,
}

/// error possible on init
#[derive(Debug)]
pub enum InitError {
    /// baal has already been initialiazed
    DoubleInit,
    /// no endpoint available
    NoDefaultEndpoint,
    /// failed to open file
    FileOpenError(PathBuf, io::Error),
    /// failed to decode file
    DecodeError(PathBuf, DecoderError),
}

impl fmt::Display for InitError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use self::InitError::*;
        match *self {
            DoubleInit => write!(fmt, "baal has already been initialized"),
            NoDefaultEndpoint => write!(fmt, "no endpoint available"),
            FileOpenError(ref source, ref error) => write!(fmt, "cannot open file {} : {}", source.to_string_lossy(), error),
            DecodeError(ref source, ref error) => write!(fmt, "cannot decode file {} : {:?}", source.to_string_lossy(), error),
        }
    }
}

#[doc(hidden)]
pub struct State {
    global_volume: f32,
    endpoint: rodio::Endpoint,
    music: music::State,
    effect: effect::State,
}

impl State {
    fn init(setting: &Setting) -> Result<State,InitError> {
        let endpoint = try!(rodio::get_default_endpoint().ok_or(InitError::NoDefaultEndpoint));

        Ok(State {
            global_volume: setting.global_volume,
            effect: try!(effect::State::init(setting, &endpoint)),
            music: try!(music::State::init(setting)),
            endpoint: endpoint,
        })
    }
    fn reset(&mut self, setting: &Setting) -> Result<(),InitError> {
        self.global_volume = setting.global_volume;
        try!(self.music.reset(setting));
        try!(self.effect.reset(setting, &self.endpoint));

        Ok(())
    }
}

/// init the audio player
pub fn init(setting: &Setting) -> Result<(), InitError> {
    unsafe {
        if !RAW_STATE.is_null() {
            return Err(InitError::DoubleInit);
        }
        let box_state = Box::new(RwLock::new(try!(State::init(setting))));
        RAW_STATE = Box::into_raw(box_state);

        Ok(())
    }
}

/// close the audio player, it can be init again.
pub fn close() {
    unsafe {
        if !RAW_STATE.is_null() {
            let mutex_state = Box::from_raw(RAW_STATE);
            let _ = mutex_state.read().unwrap();
        }
        RAW_STATE = 0 as *mut RwLock<State>;
    }
}

/// reset audio from setting on the fly
pub fn reset(setting: &Setting) -> Result<(),InitError> {
    unsafe {
        let mut state = (*RAW_STATE).write().unwrap();

        try!(state.reset(setting));

        Ok(())
    }
}

/// set the global volume
pub fn set_global_volume(v: f32) {
    let mut state = unsafe { (*RAW_STATE).write().unwrap() };
    state.global_volume = v;
    update_volume(&mut *state);
}

#[inline]
fn update_volume(state: &mut State) {
    music::update_volume(state);
    effect::update_volume(state);
}

/// return the global volume
pub fn global_volume() -> f32 {
    let state = unsafe { (*RAW_STATE).read().unwrap() };
    state.global_volume
}

