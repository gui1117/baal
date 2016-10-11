//! this module allow to play music

use rodio::decoder::Decoder;
use rodio::Sink;

use std::fs::File;
use std::sync::atomic::AtomicPtr;
use std::sync::Arc;
use std::time::Duration;
use std::sync::mpsc::Receiver;

use super::InitError;
use super::RAW_STATE;
use super::Setting;

pub struct State {
    index: Option<usize>,
    transition: MusicTransition,
    volume: f32,
    sources: Vec<Decoder<File>>,
}
impl State {
    pub fn init(setting: &Setting) -> Result<State,InitError> {
        Ok(State {
            index: None,
            transition: setting.music_transition,
            volume: setting.music_volume,
            sources: vec!(),
        })
    }
    pub fn reset(&mut self, setting: &Setting) -> Result<(),InitError> {
        *self = try!(State::init(setting));
        Ok(())
    }
}

/// set the volume of the music
/// the actual music volume is `music_volume * global_volume`
pub fn set_volume(v: f32) {
    let mut state = unsafe { (*RAW_STATE).write().unwrap() };
    state.music.volume = v;
    update_volume();
}

#[doc(hidden)]
#[inline]
pub fn update_volume() {
    unimplemented!();
}

/// return the volume of the music
pub fn volume() -> f32 {
    let state = unsafe { (*RAW_STATE).read().unwrap() };
    state.music.volume
}

/// play the music
pub fn play(music: usize) {
    // let mut state = unsafe { (*RAW_STATE).write().unwrap() };

    // state.music_index = Some(music);
    // let snd_file = SndFile::new(&state.music[music],OpenMode::Read).unwrap();
    // state.sender.send(Msg::PlayMusic(snd_file)).unwrap();
}

/// play the music if is different from the current one
pub fn play_or_continue(music: usize) {
    let must_play = if let Some(index) = index() {
        music != index
    } else {
        true
    };

    if must_play {
        play(music);
    }
}

/// pause the music
pub fn pause() {
    // let state = unsafe { (*RAW_STATE).read().unwrap() };
    // state.sender.send(Msg::PauseMusic).unwrap();
}

/// resume the music
pub fn resume() {
    // let state = unsafe { (*RAW_STATE).read().unwrap() };
    // state.sender.send(Msg::ResumeMusic).unwrap();
}

/// stop the music
pub fn stop() {
    // let mut state = unsafe { (*RAW_STATE).write().unwrap() };
    // state.music_index = None;
    // state.sender.send(Msg::StopMusic).unwrap();
}

/// return the current type of transition
pub fn transition() -> MusicTransition {
    let state = unsafe { (*RAW_STATE).read().unwrap() };
    state.music.transition
}

/// set the type of transition between musics
pub fn set_transition(trans: MusicTransition) {
    let mut state = unsafe { (*RAW_STATE).write().unwrap() };
    state.music.transition = trans;
    //TODO clear transition if current is happening
}

/// return the index of the current music if any
pub fn index() -> Option<usize> {
    let state = unsafe { (*RAW_STATE).read().unwrap() };
    state.music.index
}

/// the status of the music
#[derive(Clone,Copy,Debug,PartialEq)]
pub enum MusicStatus {
    /// the music is paused
    Pause,
    /// there is no music
    Stop,
    /// the music is played
    Play,
}

/// the type of transition between musics
#[derive(Clone,Copy,Debug,PartialEq,RustcDecodable,RustcEncodable)]
pub enum MusicTransition {
    /// the current music end smoothly and then the new one is played. (in second)
    Smooth(f32),
    /// the current music end smoothly while the new one begin smoothly. (in second)
    Overlap(f32),
    /// the current music is stopped and the new one is played.
    Instant,
}

impl MusicTransition {
    /// whether music transition is smooth
    pub fn is_smooth(&self) -> bool {
        if let &MusicTransition::Smooth(_) = self {
            true
        } else {
            false
        }
    }
}
