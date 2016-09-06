//! baal is a cross-plateform audio api for games that focus on simplicity.
//!
//! ##Features
//!
//! * channel conversion: 1 or 2 for files and 1 or 2 for audio output
//! * music player: detail in [music mode](./music/index.html)
//! * effect player: detail in [effect mode](./effect/index.html)
//! * no mp3, use ogg vorbis or other format instead
//! * no spatialization
//!
//! for more information about format available see [libsndfile#features](http://www.mega-nerd.com/libsndfile/#features)
//!
//! for more information about why not mp3 as lots of other foss handle it see [libsndfile#whynotmp3](http://www.mega-nerd.com/libsndfile/FAQ.html#Q020)
//!
//!
//! ##Dependencies
//!
//! * libsndfile:
//!
//!   From the website: [libsndfile](http://www.mega-nerd.com/libsndfile/#Download)
//!
//!   On Ubuntu / Debian:
//!
//!   ```shell
//!   apt-get install libsndfile1-dev
//!   ```
//!
//! * portaudio:
//!
//!   rust-portaudio will try to detect portaudio on your system and,
//!   failing that (or if given the PORTAUDIO\_ONLY\_STATIC environment variable on the build process),
//!   will download and build portaudio statically.
//!   If this fails please let us know!
//!   In the mean-time, you can manually download and install [PortAudio](http://www.portaudio.com/download.html) yourself.

#![warn(missing_docs)]

extern crate rustc_serialize;
extern crate portaudio;

mod sndfile;
pub use sndfile::SeekMode;

use std::sync::mpsc::{Sender, Receiver, channel};
use std::sync::RwLock;
use sndfile::{SndFile, OpenMode};
use portaudio as pa;
use std::thread;
use std::path::{Path, PathBuf};
use std::ops::Rem;
use std::fmt;

use effect::DistanceModel;
use music::MusicStatus;
use music::MusicTransition;

static mut RAW_STATE: *mut RwLock<State> = 0 as *mut RwLock<State>;

/// check at init if all music are OK
/// otherwise it may panic when playing the music
#[derive(Debug,Clone,Copy,PartialEq,RustcEncodable,RustcDecodable)]
pub enum CheckLevel {
    /// always check all music
    Always,
    /// check all music in debug mode only
    Debug,
    /// dont check music
    Never,
}

impl CheckLevel {
    fn check(&self) -> bool {
        match *self {
            CheckLevel::Always => true,
            CheckLevel::Never => false,
            CheckLevel::Debug => {
                let mut debug = false;
                debug_assert!({
                    debug = true;
                    true
                });
                debug
            }
        }
    }
}

#[derive(Clone,Debug,PartialEq,RustcEncodable,RustcDecodable)]
/// set musics, effects, volumes and audio player.
///
/// impl rustc_decodable and rustc_encodable
pub struct Setting {
    /// number of channels: 1 or 2 only
    pub channels: i32,

    /// sample rate: mostly 44_100
    pub sample_rate: f64,

    /// number of frame per buffer: 64 is good
    pub frames_per_buffer: u32,

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

    /// whereas the music must loop or not
    pub music_loop: bool,

    /// the kind of transition between musics
    pub music_transition: MusicTransition,

    /// the list of short effects, and number of loading of each, correspond
    /// to the number of effect playable at the same time
    /// for example a sword that can be played up to 10 times at the same time ("sword.ogg",10)
    ///
    /// each effect is identified by its position in the vector
    pub short_effect: Vec<(PathBuf,u32)>,

    /// the list of persistent effects
    ///
    /// each effect is identified by its position in the vector
    pub persistent_effect: Vec<PathBuf>,

    /// the list of music
    ///
    /// each music is identified by its position in the vector
    pub music: Vec<PathBuf>,

    /// check level: always, debug or never
    pub check_level: CheckLevel,
}

pub mod effect {
    //! this module allow to play short and persistent sound effects
    //!
    //! be careful that `set_volume`, `set_listener`, `set_distance_model`
    //! only affect future short sound effects

    use super::RAW_STATE;

    /// set the volume of sound effects
    /// take effect for future sounds effects only
    pub fn set_volume(v: f32) {
        let mut state = unsafe { (*RAW_STATE).write().unwrap() };
        state.effect_volume = v;
    }

    /// return the volume of sound effects
    pub fn volume() -> f32 {
        let state = unsafe { (*RAW_STATE).read().unwrap() };
        state.effect_volume
    }

    pub mod short {
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
    }

    pub mod persistent {
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

        use super::super::{RAW_STATE, Msg};

        /// add a new source of the effect
        pub fn add_position(effect: usize, pos: [f32;3]) {
            let mut state = unsafe { (*RAW_STATE).write().unwrap() };
            state.persistent_effect_positions[effect].push(pos);
        }

        /// add a vec of new sources of the effect
        pub fn add_positions(effect: usize, pos: Vec<[f32;3]>) {
            let mut state = unsafe { (*RAW_STATE).write().unwrap() };
            for pos in pos {
                state.persistent_effect_positions[effect].push(pos);
            }
        }

        /// add a vec of new sources of the effects
        pub fn add_positions_for_all(all: Vec<(usize,Vec<[f32;3]>)>) {
            let mut state = unsafe { (*RAW_STATE).write().unwrap() };
            for (effect,pos) in all {
                for pos in pos {
                    state.persistent_effect_positions[effect].push(pos);
                }
            }
        }

        /// remove all sources of the effect
        pub fn clear_positions(effect: usize) {
            let mut state = unsafe { (*RAW_STATE).write().unwrap() };
            state.persistent_effect_positions[effect].clear()
        }

        /// update the volume of effect computed from sources position and listener position at the
        /// moment of this call
        pub fn update_volume(effect: usize) {
            use std::ops::Mul;

            let state = unsafe { (*RAW_STATE).read().unwrap() };
            let v = state.persistent_effect_positions[effect].iter()
                .fold(0f32, |acc, &pos| acc + state.distance_model.distance(pos,state.listener))
                .mul(state.effect_volume)
                .mul(state.global_volume);

            state.sender.send(Msg::UpdatePersistentEffectVolume(effect,v)).unwrap();
        }

        /// pause all persistent effects
        pub fn mute_all() {
            let mut state = unsafe { (*RAW_STATE).write().unwrap() };
            if !state.persistent_mute {
                state.persistent_mute = true;
                state.sender.send(Msg::SetAllPersistentMute(true)).unwrap();
            }
        }

        /// resume all persistent effects
        pub fn unmute_all() {
            let mut state = unsafe { (*RAW_STATE).write().unwrap() };
            if state.persistent_mute {
                state.persistent_mute = false;
                state.sender.send(Msg::SetAllPersistentMute(false)).unwrap();
            }
        }

        /// return whereas persistent effects are muted
        pub fn is_all_mute() -> bool {
            let state = unsafe { (*RAW_STATE).read().unwrap() };
            state.persistent_mute
        }

        /// remove all sources of all effects
        pub fn clear_positions_for_all() {
            let mut state = unsafe { (*RAW_STATE).write().unwrap() };
            for p in &mut state.persistent_effect_positions {
                p.clear()
            }
        }

        /// update the volume of all effect
        pub fn update_volume_for_all() {
            use std::ops::Mul;

            let state = unsafe { (*RAW_STATE).read().unwrap() };

            let mut volumes = Vec::with_capacity(state.persistent_effect_positions.len());

            for effect_positions in &state.persistent_effect_positions {
                volumes.push(effect_positions.iter()
                    .fold(0f32, |acc, &pos| acc + state.distance_model.distance(pos,state.listener))
                    .mul(state.effect_volume)
                    .mul(state.global_volume));
            }

            state.sender.send(Msg::UpdatePersistentEffectsVolume(volumes)).unwrap();
        }
    }

    /// set the position of the listener
    pub fn set_listener(pos: [f32;3]) {
        let mut state = unsafe { (*RAW_STATE).write().unwrap() };
        state.listener = pos;
    }

    /// return the position of the listener
    pub fn listener() -> [f32;3] {
        let state = unsafe { (*RAW_STATE).read().unwrap() };
        state.listener
    }

    /// set the distance model
    pub fn set_distance_model(d: DistanceModel) {
        let mut state = unsafe { (*RAW_STATE).write().unwrap() };
        state.distance_model = d;
    }

    /// distance model, used to compute sound effects volumes.
    #[derive(Clone,Debug,PartialEq,RustcDecodable,RustcEncodable)]
    pub enum DistanceModel {
        /// if d <= a then 1
        ///
        /// if a <= d <= b then 1-((d-a)/(b-a))
        ///
        /// if d >= b then 0
        Linear(f32,f32),
        /// if d <= a then 1
        ///
        /// if a <= d <= b then (1-((d-a)/(b-a)))^2
        ///
        /// if d >= b then 0
        Pow2(f32,f32),
    }

    impl DistanceModel {
        fn distance(&self, pos: [f32;3], listener: [f32;3]) -> f32 {
            let d = pos.iter()
                .zip(&listener)
                .map(|(a,b)| (a-b).powi(2))
                .fold(0.,|sum,i| sum+i)
                .sqrt();

            match *self {
                DistanceModel::Linear(a,b) => {
                    if d <= a {
                        1.
                    } else if d <= b {
                        1. - ((d-a)/(b-a))
                    } else {
                        0.
                    }
                }
                DistanceModel::Pow2(a,b) => {
                    if d <= a {
                        1.
                    } else if d <= b {
                        (1. - ((d-a)/(b-a))).powi(2)
                    } else {
                        0.
                    }
                }
            }
        }
    }

    #[test]
    fn test_distance() {
        let origin = [0.,0.,0.];
        let d = DistanceModel::Linear(10.,110.);
        assert_eq!(d.distance(origin,origin), 1.);
        assert_eq!(d.distance(origin,[10.,0.,0.]), 1.);
        assert_eq!(d.distance(origin,[60.,0.,0.]), 0.5);
        assert!(d.distance(origin,[100.,0.,0.]) - 0.1 < 0.00001);
        assert_eq!(d.distance(origin,[150.,0.,0.]), 0.);
    }

}

pub mod music {
    //! this module allow to play music

    use super::{RAW_STATE, Msg};
    use super::sndfile::{SndFile, OpenMode};

    /// set the volume of the music
    /// the actual music volume is `music_volume * global_volume`
    pub fn set_volume(v: f32) {
        let mut state = unsafe { (*RAW_STATE).write().unwrap() };
        state.music_volume = v;
        state.sender.send(Msg::SetMusicVolume(state.music_volume*state.global_volume)).unwrap();
    }

    /// return the volume of the music
    pub fn volume() -> f32 {
        let state = unsafe { (*RAW_STATE).read().unwrap() };
        state.music_volume
    }

    /// seek the music to a given frame
    pub fn seek(frame: i64, mode: super::SeekMode) {
        let state = unsafe { (*RAW_STATE).read().unwrap() };
        state.sender.send(Msg::SeekMusic(frame,mode)).unwrap();
    }

    /// play the music
    pub fn play(music: usize) {
        let mut state = unsafe { (*RAW_STATE).write().unwrap() };

        state.music_index = Some(music);
        let snd_file = SndFile::new(&state.music[music],OpenMode::Read).unwrap();
        state.sender.send(Msg::PlayMusic(snd_file)).unwrap();
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
        let state = unsafe { (*RAW_STATE).read().unwrap() };
        state.sender.send(Msg::PauseMusic).unwrap();
    }

    /// resume the music
    pub fn resume() {
        let state = unsafe { (*RAW_STATE).read().unwrap() };
        state.sender.send(Msg::ResumeMusic).unwrap();
    }

    /// stop the music
    pub fn stop() {
        let mut state = unsafe { (*RAW_STATE).write().unwrap() };
        state.music_index = None;
        state.sender.send(Msg::StopMusic).unwrap();
    }

    /// return the current status of the music
    ///
    /// note that music status is updated on audio output call
    /// so there is a delay between calling fn play(_) and
    /// having the status updated
    pub fn status() -> MusicStatus {
        let mut state = unsafe { (*RAW_STATE).write().unwrap() };

        while let Ok(status) = state.music_status_receiver.try_recv() {
            state.music_status = status;
        }
        state.music_status
    }

    /// set whereas music loops or not
    pub fn set_looping(l: bool) {
        let mut state = unsafe { (*RAW_STATE).write().unwrap() };
        state.music_looping = l;
        state.sender.send(Msg::SetMusicLoop(l)).unwrap();
    }

    /// return whereas music loop or not.
    pub fn is_looping() -> bool {
        let state = unsafe { (*RAW_STATE).read().unwrap() };
        state.music_looping
    }

    /// return the current type of transition
    pub fn transition() -> MusicTransition {
        let state = unsafe { (*RAW_STATE).read().unwrap() };
        state.music_transition
    }

    /// set the type of transition between musics
    pub fn set_transition(trans: MusicTransition) {
        let mut state = unsafe { (*RAW_STATE).write().unwrap() };
        state.music_transition = trans;
        state.sender.send(Msg::SetMusicTransition(trans)).unwrap();
    }

    /// return the index of the current music if any
    pub fn index() -> Option<usize> {
        let state = unsafe { (*RAW_STATE).read().unwrap() };
        state.music_index
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
}

/// set the global volume
pub fn set_volume(v: f32) {
    let mut state = unsafe { (*RAW_STATE).write().unwrap() };
    state.global_volume = v;
    state.sender.send(Msg::SetMusicVolume(state.music_volume*state.global_volume)).unwrap();
}

/// return the global volume
pub fn volume() -> f32 {
    let state = unsafe { (*RAW_STATE).read().unwrap() };
    state.global_volume
}

/// stop music and effects
pub fn stop() {
    let state = unsafe { (*RAW_STATE).read().unwrap() };
    state.sender.send(Msg::StopMusic).unwrap();
    state.sender.send(Msg::StopAllShortEffects).unwrap();
}

/// error possible on init
#[derive(Debug)]
pub enum InitError {
    /// portaudio error
    PortAudio(pa::error::Error),
    /// sndfile error and the file corresponding
    SndFile((sndfile::SndFileError,PathBuf)),
    /// samplerate of this file doesn't match the setting
    SampleRate(PathBuf),
    /// channels of this file cannot be handled properly: must be 1 or 2
    Channels(PathBuf),
    /// output channels cannot be handled properly: must be 1 or 2
    OutputChannels,
    /// baal has already been initialiazed
    DoubleInit,
}

impl fmt::Display for InitError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use self::InitError::*;
        match *self {
            PortAudio(ref e) => write!(fmt,"portaudio error: {}",e),
            SndFile((ref e,ref s)) => write!(fmt,"sndfile error while loading {}: {}",s.to_string_lossy(),e.desc()),
            SampleRate(ref s) => write!(fmt,"sample rate of {} doesn't match the setting",s.to_string_lossy()),
            Channels(ref s) => write!(fmt,"channels of {} cannot be handled properly: must be 1 or 2",s.to_string_lossy()),
            OutputChannels => write!(fmt,"output channels cannot be handled properly: must be 1 or 2"),
            DoubleInit => write!(fmt,"baal has already been initialized"),
        }
    }
}

fn check_setting(setting: &Setting) -> Result<(),InitError> {
    if setting.channels != 1 && setting.channels != 2 {
        return Err(InitError::OutputChannels);
    }
    if setting.check_level.check() {
        for name in &setting.music {
            let file = setting.music_dir.as_path().join(name.as_path());
            let snd_file = try!(SndFile::new(file.as_path(),OpenMode::Read)
                                .map_err(|sfe| InitError::SndFile((sfe,name.clone()))));
            let snd_info = snd_file.get_sndinfo();
            if (snd_info.samplerate as f64 - setting.sample_rate).abs() > std::f64::EPSILON {
                return Err(InitError::SampleRate(name.clone()));
            }
            if snd_info.channels != 1 && snd_info.channels != 2 {
                return Err(InitError::Channels(name.clone()));
            }
        }
    }
    for name in setting.short_effect.iter().map(|&(ref name,_)| name).chain(setting.persistent_effect.iter()) {
        let file = setting.effect_dir.as_path().join(name.as_path());
        let snd_file = try!(SndFile::new(file.as_path(),OpenMode::Read)
                            .map_err(|sfe| InitError::SndFile((sfe,name.clone()))));
        let snd_info = snd_file.get_sndinfo();
        if (snd_info.samplerate as f64 - setting.sample_rate).abs() > std::f64::EPSILON {
            return Err(InitError::SampleRate(name.clone()));
        }
        if snd_info.channels != 1 && snd_info.channels != 2 {
            return Err(InitError::Channels(name.clone()));
        }
    }
    Ok(())
}

fn init_state(setting: &Setting, music_status_receiver: Receiver<MusicStatus>, sender: Sender<Msg>, abort_sender: Sender<()>) {
    let state = State::from_setting(setting,music_status_receiver,sender,abort_sender);

    unsafe {
        let box_state = Box::new(RwLock::new(state));
        RAW_STATE = Box::into_raw(box_state);
    }
}

fn init_stream(setting: &Setting, music_status_sender: Sender<MusicStatus>, receiver: Receiver<Msg>, abort_receiver: Receiver<()>) -> Result<(), InitError> {
    let mut short_effect: Vec<ShortEffect> = setting.short_effect.iter()
        .map(|&(ref name,nbr)| ShortEffect::new(
                setting.effect_dir.as_path().join(name.as_path()).as_path()
                ,nbr as usize
                ,setting.channels)
            )
        .collect();

    let mut persistent_effect: Vec<PersistentEffect> = setting.persistent_effect.iter()
        .map(|name| PersistentEffect::new(
                setting.effect_dir.as_path().join(name.as_path()).as_path()
                ,setting.channels)
            )
        .collect();
    let mut persistent_effect_pause = false;

    let mut music = Music::new((setting.global_volume*setting.music_volume as f32),setting.music_loop,setting.music_transition,setting.channels,setting.sample_rate as f32,music_status_sender);

    let mut buffer_one: Vec<f32> = (0..setting.frames_per_buffer).map(|i| i as f32).collect();
    let mut buffer_two: Vec<f32> = (0..2*setting.frames_per_buffer).map(|i| i as f32).collect();

    let pa = try!(pa::PortAudio::new().map_err(InitError::PortAudio));

    let settings = try!(pa.default_output_stream_settings(setting.channels, setting.sample_rate, setting.frames_per_buffer)
                        .map_err(InitError::PortAudio));

    let callback = move |pa::OutputStreamCallbackArgs { buffer, frames, .. }| {
        // is the buffer already set to 0 ?
        for elt in buffer.iter_mut() { *elt = 0.; }

        let frames = frames as i64;

        while let Ok(msg) = receiver.try_recv() {
            match msg {
                Msg::PlayShortEffect(n,vol) => short_effect[n].play(vol),
                Msg::SetMusicVolume(vol) => music.set_volume(vol),
                Msg::PlayMusic(snd_file) => music.set_music(snd_file),
                Msg::PauseMusic => music.pause(),
                Msg::ResumeMusic => music.resume(),
                Msg::SeekMusic(frame,mode) => music.seek(frame,mode),
                Msg::StopMusic => music.stop(),
                Msg::StopAllShortEffects => for e in &mut short_effect { e.stop(); },
                Msg::SetMusicLoop(l) => music.set_loop(l),
                Msg::SetMusicTransition(trans) => music.set_transition(trans),
                Msg::SetAllPersistentMute(p) => persistent_effect_pause = p,
                Msg::UpdatePersistentEffectVolume(effect,volume) => persistent_effect[effect].volume = volume,
                Msg::UpdatePersistentEffectsVolume(volumes) => {
                    for (&v,e) in volumes.iter().zip(persistent_effect.iter_mut()) {
                        e.volume = v;
                    }
                },
            }
        }

        music.fill_buffer(buffer, &mut buffer_one,&mut buffer_two, frames);

        for e in &mut short_effect {
            e.fill_buffer(buffer, &mut buffer_one, &mut buffer_two, frames);
        }

        if !persistent_effect_pause {
            for e in &mut persistent_effect {
                e.fill_buffer(buffer, &mut buffer_one, &mut buffer_two, frames);
            }
        }

        pa::Continue
    };

    thread::spawn(move || {
        let mut stream = pa.open_non_blocking_stream(settings, callback).expect("fail to open non blocking audio stream");

        stream.start().expect("fail to start audio stream");

        abort_receiver.recv().expect("audio thread abort error");
    });

    Ok(())
}

/// init the audio player
pub fn init(setting: &Setting) -> Result<(), InitError> {
    unsafe { if !RAW_STATE.is_null() {
        return Err(InitError::DoubleInit);
    }};

    try!(check_setting(setting));

    let (sender,receiver) = channel();
    let (abort_sender,abort_receiver) = channel();
    let (music_status_sender, music_status_receiver) = channel();

    init_state(setting, music_status_receiver, sender, abort_sender);

    try!(init_stream(setting, music_status_sender, receiver, abort_receiver));

    Ok(())
}

/// close the audio player, it can be init again.
pub fn close() {
    unsafe {
        if !RAW_STATE.is_null() {
            let mutex_state = Box::from_raw(RAW_STATE);
            let state = mutex_state.read().unwrap();
            state.abort_sender.send(()).unwrap();
        }
        RAW_STATE = 0 as *mut RwLock<State>;
    }
}

/// reset audio from setting on the fly
pub fn reset(setting: &Setting) -> Result<(),InitError> {
    try!(check_setting(setting));

    let (sender,receiver) = channel();
    let (abort_sender,abort_receiver) = channel();
    let (music_status_sender, music_status_receiver) = channel();

    let old_raw_state = unsafe { RAW_STATE };

    init_state(setting, music_status_receiver, sender, abort_sender);

    // drop old state
    {
        let old_mutex_state = unsafe { Box::from_raw(old_raw_state) };
        let old_state = old_mutex_state.read().unwrap();
        old_state.abort_sender.send(()).unwrap();
    }

    try!(init_stream(setting, music_status_sender, receiver, abort_receiver));
    Ok(())
}

struct State {
    music_looping: bool,
    music_status: MusicStatus,
    music_index: Option<usize>,
    music_transition: MusicTransition,
    music_status_receiver: Receiver<MusicStatus>,
    sender: Sender<Msg>,
    abort_sender: Sender<()>,
    listener: [f32;3],
    distance_model: DistanceModel,
    global_volume: f32,
    music_volume: f32,
    effect_volume: f32,
    music: Vec<PathBuf>,
    persistent_effect_positions: Vec<Vec<[f32;3]>>,
    persistent_mute: bool,
}

impl State {
    fn from_setting(s: &Setting,music_status_receiver: Receiver<MusicStatus>, sender: Sender<Msg>,abort_sender: Sender<()>) -> State {
        let music_dir = Path::new(&s.music_dir);
        let music: Vec<PathBuf> = s.music.iter().map(|name| music_dir.join(Path::new(&name))).collect();

        State {
            music_looping: s.music_loop,
            music_status: MusicStatus::Stop,
            music_index: None,
            music_transition: s.music_transition,
            music_status_receiver: music_status_receiver,
            sender: sender,
            abort_sender: abort_sender,
            listener: [0.,0.,0.],
            distance_model: s.distance_model.clone(),
            global_volume: s.global_volume,
            music_volume: s.music_volume,
            effect_volume: s.effect_volume,
            music: music,
            persistent_effect_positions: s.persistent_effect.iter().map(|_| vec!()).collect(),
            persistent_mute: false,
        }
    }
}

#[derive(Debug)]
enum Msg {
    PlayMusic(SndFile),
    SetMusicVolume(f32),
    PauseMusic,
    ResumeMusic,
    SetMusicTransition(MusicTransition),
    SeekMusic(i64,SeekMode),
    StopMusic,
    PlayShortEffect(usize,f32),
    StopAllShortEffects,
    SetMusicLoop(bool),
    SetAllPersistentMute(bool),
    UpdatePersistentEffectVolume(usize,f32),
    UpdatePersistentEffectsVolume(Vec<f32>),
}

#[derive(Debug,Clone,Copy)]
enum ChannelConv {
    TwoIntoOne,
    OneIntoTwo,
    TwoIntoTwo,
    OneIntoOne,
}

impl ChannelConv {
    fn from_channels(input: i32, output: i32) -> ChannelConv {
        match input {
            1 => match output {
                1 => ChannelConv::OneIntoOne,
                2 => ChannelConv::OneIntoTwo,
                _ => panic!("intern error: sndfile channels is not 1 or 2")
            },
            2 => match output {
                1 => ChannelConv::TwoIntoOne,
                2 => ChannelConv::TwoIntoTwo,
                _ => panic!("intern error: sndfile channels is not 1 or 2")
            },
            _ => panic!("intern error: output channels is not 1 or 2")
        }
    }

    fn fill_buffer(&self, sndfile: &mut SndFile, volume: f32, buffer_output: &mut [f32], buffer_one: &mut [f32], buffer_two: &mut [f32], frames: i64) -> i64 {
        match *self {
            ChannelConv::TwoIntoOne => {
                let frame = sndfile.readf_f32(buffer_two,frames);
                for k in 0..buffer_output.len() {
                    buffer_output[k] += (buffer_two[2*k]+buffer_two[2*k+1])/2.*volume;
                }
                frame
            },
            ChannelConv::TwoIntoTwo => {
                let frame = sndfile.readf_f32(buffer_two,frames);
                for k in 0..buffer_output.len() {
                    buffer_output[k] += buffer_two[k]*volume;
                }
                frame
            },
            ChannelConv::OneIntoTwo => {
                let frame = sndfile.readf_f32(buffer_one,frames);
                for k in 0..buffer_one.len() {
                    buffer_output[2*k] += buffer_one[k]*volume;
                    buffer_output[2*k+1] += buffer_one[k]*volume;
                }
                frame
            },
            ChannelConv::OneIntoOne => {
                let frame = sndfile.readf_f32(buffer_one,frames);
                for k in 0..buffer_output.len() {
                    buffer_output[k] += buffer_one[k]*volume;
                }
                frame
            },
        }
    }
}

struct PersistentEffect {
    snd_file: SndFile,
    volume: f32,
    channel_conv: ChannelConv,
}

impl PersistentEffect {
    fn new(path: &Path, output_channels: i32) -> Self {
        let snd_file = SndFile::new(path,OpenMode::Read).unwrap(); // unwrap because already checked
        let channel_conv = ChannelConv::from_channels(snd_file.get_sndinfo().channels,output_channels);

        PersistentEffect {
            snd_file: snd_file,
            channel_conv: channel_conv,
            volume: 0f32,
        }
    }
    fn fill_buffer(&mut self, buffer_output: &mut [f32], buffer_one: &mut [f32], buffer_two: &mut [f32], frames: i64) {
        if self.volume == 0. { return }

        let frame = self.channel_conv.fill_buffer(
            &mut self.snd_file,
            self.volume,
            buffer_output,
            buffer_one,
            buffer_two,
            frames);

        if frame == 0 {
            self.snd_file.seek(0,SeekMode::SeekSet);
        }
    }
}

#[derive(Debug)]
struct ShortEffect {
    start_end: Option<(usize,usize)>,
    batch: Vec<SndFile>,
    volume: Vec<f32>,
    channel_conv: ChannelConv,
}

impl ShortEffect {
    fn new(path: &Path, capacity: usize, output_channels: i32) -> Self {
        let mut batch = Vec::with_capacity(capacity);
        let mut volume = Vec::with_capacity(capacity);

        for _ in 0..capacity {
            batch.push(SndFile::new(path,OpenMode::Read).unwrap()); // unwrap because already checked
            volume.push(0.);
        }

        let channel_conv = ChannelConv::from_channels(batch[0].get_sndinfo().channels,output_channels);

        ShortEffect {
            start_end: None,
            batch: batch,
            volume: volume,
            channel_conv: channel_conv,
        }
    }

    fn fill_buffer(&mut self, buffer_output: &mut [f32], buffer_one: &mut [f32], buffer_two: &mut [f32], frames: i64) {
        self.start_end = if let Some((start, mut end)) = self.start_end {
            let range = if start <= end {
                (start..end+1).chain(0..0)
            } else {
                (0..end+1).chain(start..self.batch.len())
            };

            let mut ended = false;
            for i in range {
                let frame = self.channel_conv.fill_buffer(
                    &mut self.batch[i],
                    self.volume[i],
                    buffer_output,
                    buffer_one,
                    buffer_two,
                    frames);

                if frame == 0 {
                    ended = true;
                    if end == 0 {
                        end = self.batch.len() - 1;
                    } else {
                        end -= 1;
                    }
                }
            }

            if ended && (end + 1).rem(self.batch.len()) == start {
                None
            } else {
                Some((start,end))
            }
        } else { self.start_end };
    }

    fn stop(&mut self) {
        self.start_end = None;
    }

    fn play(&mut self,volume: f32) {
        self.start_end = if let Some((start,end)) = self.start_end {
            let new_end = (end + 1).rem(self.batch.len());

            self.volume[new_end] = volume;
            self.batch[new_end].seek(0,SeekMode::SeekSet);

            if new_end == start {
                Some(((start+1).rem(self.batch.len()),new_end))
            } else {
                Some((start,new_end))
            }
        } else {
            self.volume[0] = volume;
            self.batch[0].seek(0,SeekMode::SeekSet);
            Some((0,0))
        };
    }
}

#[derive(Debug)]
struct Music {
    sample_rate: f32,
    status_sender: Sender<MusicStatus>,
    snd_file: Option<SndFile>,
    transitional_snd_file: Option<SndFile>,
    transition_frame: i64,
    transition_type: MusicTransition,
    pause: bool,
    volume: f32,
    looping: bool,
    channel_conv: ChannelConv,
    output_channels: i32,
}

impl Music {
    fn new(volume: f32, looping: bool, transition: MusicTransition, output_channels: i32, sample_rate: f32, status_sender: Sender<MusicStatus>) -> Music {
        Music {
            sample_rate: sample_rate,
            status_sender: status_sender,
            snd_file: None,
            transitional_snd_file: None,
            pause: false,
            volume: volume,
            looping: looping,
            transition_type: transition,
            transition_frame: 0,
            channel_conv: ChannelConv::OneIntoOne,
            output_channels: output_channels,
        }
    }

    fn fill_buffer(&mut self, buffer_output: &mut [f32], buffer_one: &mut [f32], buffer_two: &mut [f32], frames: i64) {
        if self.pause { return; }

        let destroy_snd_file = if let Some(ref mut snd_file) = self.snd_file {
            if self.transitional_snd_file.is_some() && self.transition_type.is_smooth() {
                false
            } else {
                let volume = if self.transitional_snd_file.is_some() {
                    match self.transition_type {
                        MusicTransition::Overlap(transition_time) => {
                            self.volume * self.transition_frame as f32 / (transition_time * self.sample_rate)
                        },
                        MusicTransition::Instant => panic!("music transition is instant and there is a transitional snd file"),
                        MusicTransition::Smooth(_) => unreachable!(),
                    }
                } else {
                    self.volume
                };

                let frame = self.channel_conv.fill_buffer(snd_file, volume, buffer_output, buffer_one, buffer_two, frames);

                if frame == 0 {
                    if self.looping {
                        snd_file.seek(0,SeekMode::SeekSet);
                        false
                    } else { true }
                } else { false }
            }
        } else { false };

        if destroy_snd_file {
            let _ = self.status_sender.send(MusicStatus::Stop);
            self.snd_file = None;
        }

        let destroy_transitional_snd_file = if let Some(ref mut snd_file) = self.transitional_snd_file {
            let transition_frames = match self.transition_type {
                MusicTransition::Instant => panic!("music transition is instant and there is a transitional snd file"),
                MusicTransition::Overlap(t) | MusicTransition::Smooth(t) => t * self.sample_rate,
            };

            let volume = self.volume * (1. - self.transition_frame as f32 / transition_frames);
            let frame = self.channel_conv.fill_buffer(snd_file, volume, buffer_output, buffer_one, buffer_two, frames);

            self.transition_frame += frame;
            self.transition_frame >= transition_frames as i64 || frame == 0
        } else { false };

        if destroy_transitional_snd_file {
            self.transition_frame = 0;
            self.transitional_snd_file = None;
        };
    }

    fn set_transition(&mut self, trans: MusicTransition) {
        if let MusicTransition::Instant = trans {
            self.transitional_snd_file = None;
        }
        self.transition_type = trans;
    }

    fn stop(&mut self) {
        let _ = self.status_sender.send(MusicStatus::Stop);
        self.snd_file = None;
    }

    fn pause(&mut self) {
        if self.snd_file.is_some() {
            let _ = self.status_sender.send(MusicStatus::Pause);
        }
        self.pause = true;
    }

    fn resume(&mut self) {
        if self.snd_file.is_some() {
            let _ = self.status_sender.send(MusicStatus::Play);
        }
        self.pause = false;
    }

    fn seek(&mut self, frame: i64, mode: SeekMode) {
        if let Some(ref mut snd_file) = self.snd_file {
            snd_file.seek(frame,mode);
        }
    }

    fn set_music(&mut self, snd_file: SndFile) {
        let _ = self.status_sender.send(MusicStatus::Play);
        self.channel_conv = ChannelConv::from_channels(snd_file.get_sndinfo().channels,self.output_channels);
        match self.transition_type {
            MusicTransition::Instant => {
                self.snd_file = Some(snd_file);
            },
            MusicTransition::Smooth(_) | MusicTransition::Overlap(_) => {
                self.transitional_snd_file = self.snd_file.take();
                self.snd_file = Some(snd_file);
                self.transition_frame = 0;
            }
        }
    }

    fn set_loop(&mut self, looping: bool) {
        self.looping = looping;
    }

    fn set_volume(&mut self, v: f32) {
        self.volume = v;
    }
}
