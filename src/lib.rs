//! TODO doc

#![warn(missing_docs)]

extern crate rustc_serialize;
extern crate rodio;

pub mod music;

mod source;

use std::sync::RwLock;
use std::path::{Path, PathBuf};
use std::fmt;

// use effect::DistanceModel;
use music::MusicTransition;

static mut RAW_STATE: *mut RwLock<State> = 0 as *mut RwLock<State>;

#[derive(Clone,Debug,PartialEq,RustcEncodable,RustcDecodable)]
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

    // /// distance model for effect volume computation
    // pub distance_model: DistanceModel,

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


/// set the global volume
pub fn set_global_volume(v: f32) {
    let mut state = unsafe { (*RAW_STATE).write().unwrap() };
    state.global_volume = v;
    update_volume();
}

#[inline]
fn update_volume() {
    music::update_volume();
}

/// return the global volume
pub fn global_volume() -> f32 {
    let state = unsafe { (*RAW_STATE).read().unwrap() };
    state.global_volume
}

/// error possible on init
#[derive(Debug)]
pub enum InitError {
    /// baal has already been initialiazed
    DoubleInit,
    /// no endpoint available
    NoDefaultEndpoint,
}

impl fmt::Display for InitError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use self::InitError::*;
        match *self {
            DoubleInit => write!(fmt, "baal has already been initialized"),
            NoDefaultEndpoint => write!(fmt, "no endpoint available"),
        }
    }
}

/// init the audio player
pub fn init(setting: &Setting) -> Result<(), InitError> {
    unsafe {
        if !RAW_STATE.is_null() {
            return Err(InitError::DoubleInit);
        }
        *RAW_STATE = RwLock::new(try!(State::init(setting)));
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

struct State {
    global_volume: f32,
    endpoint: rodio::Endpoint,
    music: music::State,
}

impl State {
    fn init(setting: &Setting) -> Result<State,InitError> {
        let endpoint = try!(rodio::get_default_endpoint().ok_or(InitError::NoDefaultEndpoint));

        Ok(State {
            endpoint: endpoint,
            global_volume: setting.global_volume,
            music: try!(music::State::init(setting)),
        })
    }
    fn reset(&mut self, setting: &Setting) -> Result<(),InitError> {
        self.global_volume = setting.global_volume;
        try!(self.music.reset(setting));

        Ok(())
    }
}
