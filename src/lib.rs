//! baal is a cross-plateform audio api for games that focus on simplicity.
//!
//! ##Features
//!
//! * yaml configuration so you can easily test sounds without recompile
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

#[macro_use]
extern crate yaml_rust;
extern crate portaudio;

mod sndfile;

use yaml_rust::yaml::Yaml;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::sync::Mutex;
use sndfile::{SndFile, OpenMode, SeekMode};
use portaudio as pa;
use std::thread;
use std::path::{Path, PathBuf};
use std::ops::Rem;
use effect::DistanceModel;

static mut RAW_STATE: *mut Mutex<State> = 0 as *mut Mutex<State>;

/// check at init if all music are OK
/// otherwise it may panic when playing the music
#[derive(Debug,Clone,Copy,PartialEq)]
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

#[derive(Clone,Debug,PartialEq)]
/// set musics, effects, volumes and audio player.
pub struct Setting {
    /// number of channels: 1 or 2 only
    pub channels: i32,
    /// sample rate: mostly 44_100
    pub sample_rate: f64,
    /// number of frame per buffer: 64 is good
    pub frames_per_buffer: u32,

    /// the base directory of effects
    pub effect_dir: String,
    /// the base directory of musics
    pub music_dir: String,

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

    /// the list of effect, and number of loading of each, correspond
    /// to the number of effect playable at the same time
    /// for example a sword that can be played up to 10 times at the same time ("sword.ogg",10)
    ///
    /// each effect is identified by its position in the vector
    pub effect: Vec<(String,u32)>,
    /// the list of music
    ///
    /// each music is identified by its position in the vector
    pub music: Vec<String>,

    /// check level: always, debug or never
    pub check_level: CheckLevel,
}

impl Setting {
    /// import setting from yaml:
    ///
    /// ```yaml
    /// ---
    /// check_level: always
    ///
    /// channels: 2
    /// sample_rate: 44100.
    /// frames_per_buffer: 64
    ///
    /// effect_dir: assets/effects
    /// music_dir: assets/musics
    ///
    /// global_volume: 0.5
    /// music_volume: 0.8
    /// effect_volume: 0.3
    ///
    /// distance_model: [pow2,10.,110.]
    /// music_loop: true
    ///
    /// effect:
    ///     - [shoot.ogg,10]
    ///     - [hit.ogg,10]
    ///
    /// music:
    ///     - village.ogg
    /// ...
    /// ```
    ///
    pub fn from_yaml(code: &Yaml) -> Result<Self,String> {
        let hash = try!(code.as_hash().ok_or_else(|| "config must be an associative array"));

        let check_level = {
            let check_level_str = try!(try!(hash.get(&Yaml::String(String::from("check_level")))
                .ok_or_else(|| "config map must have check_level key")).as_str()
                .ok_or_else(|| "check_level must be a string"));

            match check_level_str {
                "always" | "Always" => CheckLevel::Always,
                "debug" | "Debug" => CheckLevel::Debug,
                "never" | "Never" => CheckLevel::Never,
                _ => return Err("check level is not a correct enum".into()),
            }
        };

        let distance_model = {
            let vec = try!(try!(hash.get(&Yaml::String(String::from("distance_model")))
                .ok_or_else(|| "config map must have distance_model key")).as_vec()
                .ok_or_else(|| "distance model must be vector"));

            match try!(vec[0].as_str().ok_or_else(|| "distance model first element must be the string of the enum")) {
                "linear" | "Linear" => DistanceModel::Linear(
                    try!(vec[1].as_f64().ok_or_else(|| "linear distance model second element must be a float")),
                    try!(vec[2].as_f64().ok_or_else(|| "linear distance model third element must be a float"))),
                "pow2" | "Pow2" => DistanceModel::Pow2(
                    try!(vec[1].as_f64().ok_or_else(|| "exponential distance model second element must be a float")),
                    try!(vec[2].as_f64().ok_or_else(|| "exponential distance model third element must be a float"))),
                _ => return Err("distance model first element is not a correct enum".into()),
            }
        };

        let effect = {
            let key = try!(hash.get(&Yaml::String(String::from("effect")))
                           .ok_or_else(|| "config map must have effect key"));

            if let Some(vec) = key.as_vec() {
                let mut res = Vec::new();
                for y in vec {
                    let y_vec = try!(y.as_vec().ok_or_else(|| "element of effect list must be a vector"));
                    let name = try!(y_vec[0].as_str().ok_or_else(|| "first element of effect list must be a string")).into();
                    let loads = try!(y_vec[1].as_i64().ok_or_else(|| "second element of effect list must be an integer")) as u32;
                    res.push((name,loads));
                }
                res
            } else if key.is_null() {
                vec!()
            } else {
                return Err("effect must a list or null".into());
            }
        };

        let music = {
            let key = try!(hash.get(&Yaml::String(String::from("music"))).ok_or_else(|| "config map must have music key"));
            if let Some(vec) = key.as_vec() {
                let mut res = Vec::new();
                for y in vec {
                    res.push(try!(y.as_str().ok_or_else(|| "element of music must a string")).into());
                }
                res
            } else if key.is_null() {
                vec!()
            } else {
                return Err("music must a list or null".into());
            }
        };

        Ok(Setting {
            channels: try!(try!(hash.get(&Yaml::String(String::from("channels")))
                .ok_or_else(|| "config map must have a channels key")).as_i64()
                .ok_or_else(|| "channels must be integer")) as i32,

            sample_rate: try!(try!(hash.get(&Yaml::String(String::from("sample_rate")))
                .ok_or_else(|| "config map must have a sample_rate key")).as_f64()
                .ok_or_else(|| "sample_rate must be float")),

            frames_per_buffer: try!(try!(hash.get(&Yaml::String(String::from("frames_per_buffer")))
                .ok_or_else(|| "config map must have a frames_per_buffer key")).as_i64()
                .ok_or_else(|| "frames_per_buffer must be integer")) as u32,

            music_dir: String::from(try!(try!(hash.get(&Yaml::String(String::from("music_dir")))
                .ok_or_else(|| "config map must have a music_dir key")).as_str()
                .ok_or_else(|| "music_dir must be string"))),

            effect_dir: String::from(try!(try!(hash.get(&Yaml::String(String::from("effect_dir")))
                .ok_or_else(|| "config map must have a effect_dir key")).as_str()
                .ok_or_else(|| "effect_dir must be string"))),

            global_volume: try!(try!(hash.get(&Yaml::String(String::from("global_volume")))
                .ok_or_else(|| "config map must have a global_volume key")).as_f64()
                .ok_or_else(|| "global volume must be float")) as f32,

            music_volume: try!(try!(hash.get(&Yaml::String(String::from("music_volume")))
                .ok_or_else(|| "config map must have a music_volume key")).as_f64()
                .ok_or_else(|| "music volume must be float")) as f32,

            effect_volume: try!(try!(hash.get(&Yaml::String(String::from("effect_volume")))
                .ok_or_else(|| "config map must have a effect_volume key")).as_f64()
                .ok_or_else(|| "effect volume must be float")) as f32,

            distance_model: distance_model,

            music_loop: try!(try!(hash.get(&Yaml::String(String::from("music_loop")))
                .ok_or_else(|| "config map must have a music_loop key")).as_bool()
                .ok_or_else(|| "music_loop must be bool")),

            effect: effect,
            music: music,
            check_level: check_level,
        })
    }
}

pub mod effect {
    //! this module allow to play sound effect
    //!
    //! volume = `global_volume * effect_volume * distance([x,y,z],listener_position)`
    //!
    //! but once a sound effect is played at a volume it doesn't change its volume anymore
    //! this can lead to weird effects for long sound effects

    use super::{RAW_STATE, Msg};

    /// set the volume of sound effects
    /// take effect for future sounds effects only
    pub fn set_volume(v: f32) {
        let mut state = unsafe { (*RAW_STATE).lock().unwrap() };
        state.effect_volume = v;
    }

    /// get the volume of sound effects
    pub fn volume() -> f32 {
        let state = unsafe { (*RAW_STATE).lock().unwrap() };
        state.effect_volume
    }

    /// play the sound effect at the volume: `global_volume * effect_volume *
    /// distance([x,y,z],listener_position)`
    pub fn play(effect: usize, pos: &[f64;3]) {
        let state = unsafe { (*RAW_STATE).lock().unwrap() };
        let volume = state.global_volume * state.effect_volume * state.distance_model.distance(pos,&state.listener);
        if volume > 0. {
            state.sender.send(Msg::PlayEffect(effect,volume)).unwrap();
        }
    }

    /// stop all sound effects
    pub fn stop_all() {
        let state = unsafe { (*RAW_STATE).lock().unwrap() };
        state.sender.send(Msg::StopEffect).unwrap();
    }

    /// set the position of the listener
    pub fn set_listener(x: f64, y: f64, z: f64) {
        let mut state = unsafe { (*RAW_STATE).lock().unwrap() };
        state.listener = [x,y,z];
    }

    /// return the position of the listener
    pub fn listener() -> [f64;3] {
        let state = unsafe { (*RAW_STATE).lock().unwrap() };
        state.listener
    }

    /// set the distance model
    /// take effect for future sounds effects only
    pub fn set_distance_model(d: DistanceModel) {
        let mut state = unsafe { (*RAW_STATE).lock().unwrap() };
        state.distance_model = d;
    }

    /// distance model, used to compute sounds effects volume.
    #[derive(Clone,Debug,PartialEq)]
    pub enum DistanceModel {
        /// if d <= a then 1
        ///
        /// if a <= d <= b then 1-((d-a)/(b-a))
        ///
        /// if d >= b then 0
        Linear(f64,f64),
        /// if d <= a then 1
        ///
        /// if a <= d <= b then (1-((d-a)/(b-a)))^2
        ///
        /// if d >= b then 0
        Pow2(f64,f64),
    }

    impl DistanceModel {
        fn distance(&self, pos: &[f64;3], listener: &[f64;3]) -> f32 {
            let d = pos.iter()
                .zip(listener)
                .map(|(a,b)| (a-b).powi(2))
                .fold(0.,|sum,i| sum+i)
                .sqrt();

            match *self {
                DistanceModel::Linear(a,b) => {
                    if d <= a {
                        1.
                    } else if d <= b {
                        1. - ((d-a)/(b-a)) as f32
                    } else {
                        0.
                    }
                }
                DistanceModel::Pow2(a,b) => {
                    if d <= a {
                        1.
                    } else if d <= b {
                        (1. - ((d-a)/(b-a)) as f32).powi(2)
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
        assert_eq!(d.distance(&origin,&origin), 1.);
        assert_eq!(d.distance(&origin,&[10.,0.,0.]), 1.);
        assert_eq!(d.distance(&origin,&[60.,0.,0.]), 0.5);
        assert!(d.distance(&origin,&[100.,0.,0.]) - 0.1 < 0.00001);
        assert_eq!(d.distance(&origin,&[150.,0.,0.]), 0.);
    }

}

pub mod music {
    //! this module allow to play music

    use super::{RAW_STATE, Msg};
    use super::sndfile::{SndFile, OpenMode};

    /// set the volume of the music
    /// the actual music volume is `music_volume * global_volume`
    pub fn set_volume(v: f32) {
        let mut state = unsafe { (*RAW_STATE).lock().unwrap() };
        state.music_volume = v;
        state.sender.send(Msg::SetMusicVolume(state.music_volume*state.global_volume)).unwrap();
    }

    /// get the volume of the music
    pub fn volume() -> f32 {
        let state = unsafe { (*RAW_STATE).lock().unwrap() };
        state.music_volume
    }

    /// seek the music to a given frame
    pub fn seek(frame: i64) {
        let state = unsafe { (*RAW_STATE).lock().unwrap() };
        state.sender.send(Msg::SeekMusic(frame)).unwrap();
    }

    /// play the music
    pub fn play(music: usize) {
        let mut state = unsafe { (*RAW_STATE).lock().unwrap() };

        state.music_status.pause = false;
        state.music_status.id = Some(music);
        let snd_file = SndFile::new(&state.music[music],OpenMode::Read).unwrap();
        state.sender.send(Msg::PlayMusic(snd_file)).unwrap();
    }

    /// pause the music
    pub fn pause() {
        let mut state = unsafe { (*RAW_STATE).lock().unwrap() };
        state.music_status.pause = true;
        state.sender.send(Msg::PauseMusic).unwrap();
    }

    /// resume the music
    pub fn resume() {
        let mut state = unsafe { (*RAW_STATE).lock().unwrap() };
        state.music_status.pause = false;
        state.sender.send(Msg::ResumeMusic).unwrap();
    }

    /// stop the music
    pub fn stop() {
        let mut state = unsafe { (*RAW_STATE).lock().unwrap() };
        state.music_status.pause = false;
        state.music_status.id = None;
        state.sender.send(Msg::StopMusic).unwrap();
    }

    /// return the current status of the music
    pub fn status() -> MusicStatus {
        let state = unsafe { (*RAW_STATE).lock().unwrap() };
        state.music_status
    }

    /// set whereas music loop or not
    pub fn set_loop(l: bool) {
        let mut state = unsafe { (*RAW_STATE).lock().unwrap() };
        state.music_status.looping = l;
        state.sender.send(Msg::SetMusicLoop(l)).unwrap();
    }

    /// return whereas music loop or not.
    pub fn is_looping() -> bool {
        let state = unsafe { (*RAW_STATE).lock().unwrap() };
        state.music_status.looping
    }

    /// the status of the music
    #[derive(Clone,Copy,Debug,PartialEq)]
    pub struct MusicStatus {
        /// the Id of the music played if any
        pub id: Option<usize>,
        /// whereas the music is paused
        pub pause: bool,
        /// whereas the music is looping
        pub looping: bool,
    }
}
use music::MusicStatus;

/// set the global volume
pub fn set_volume(v: f32) {
        let mut state = unsafe { (*RAW_STATE).lock().unwrap() };
    state.global_volume = v;
    state.sender.send(Msg::SetMusicVolume(state.music_volume*state.global_volume)).unwrap();
}

/// get the global volume
pub fn volume() -> f32 {
        let state = unsafe { (*RAW_STATE).lock().unwrap() };
    state.global_volume
}

/// stop music and effects
pub fn stop() {
        let state = unsafe { (*RAW_STATE).lock().unwrap() };
    state.sender.send(Msg::StopMusic).unwrap();
    state.sender.send(Msg::StopEffect).unwrap();
}

/// error possible on init
#[derive(Debug)]
pub enum InitError {
    /// portaudio error
    PortAudio(pa::error::Error),
    /// sndfile error and the file corresponding
    SndFile((sndfile::SndFileError,String)),
    /// samplerate of this file doesn't match the setting
    SampleRate(String),
    /// channels of this file cannot be handled properly: not 1 or 2
    Channels(String),
    /// output channels cannot be handledis properly: not 1 or 2
    OutputChannels,
    /// baal has already been initialiazed
    DoubleInit,
}

fn check_setting(setting: &Setting) -> Result<(),InitError> {
    if setting.channels != 1 && setting.channels != 2 {
        return Err(InitError::OutputChannels);
    }
    if setting.check_level.check() {
        for i in &setting.music {
            let file = Path::new(&setting.music_dir).join(Path::new(&i));
            let snd_file = try!(SndFile::new(file.as_path(),OpenMode::Read)
                                .map_err(|sfe| InitError::SndFile((sfe,i.clone()))));
            let snd_info = snd_file.get_sndinfo();
            if snd_info.samplerate as f64 != setting.sample_rate {
                return Err(InitError::SampleRate(i.clone()));
            }
            if snd_info.channels != 1 && snd_info.channels != 2 {
                return Err(InitError::Channels(i.clone()));
            }
        }
    }
    for &(ref i,_) in &setting.effect {
        let file = Path::new(&setting.effect_dir).join(Path::new(i));
        let snd_file = try!(SndFile::new(file.as_path(),OpenMode::Read)
                            .map_err(|sfe| InitError::SndFile((sfe,i.clone()))));
        let snd_info = snd_file.get_sndinfo();
        if snd_info.samplerate as f64 != setting.sample_rate {
            return Err(InitError::SampleRate(i.clone()));
        }
        if snd_info.channels != 1 && snd_info.channels != 2 {
            return Err(InitError::Channels(i.clone()));
        }
    }
    Ok(())
}

fn init_state(setting: &Setting, sender: Sender<Msg>, abort_sender: Sender<()>) {
    let state = State::from_setting(&setting,sender,abort_sender);

    unsafe {
        let box_state = Box::new(Mutex::new(state));
        RAW_STATE = Box::into_raw(box_state);
    }
}

fn init_stream(setting: &Setting, receiver: Receiver<Msg>, abort_receiver: Receiver<()>) -> Result<(), InitError> {
    let mut effect: Vec<Effect> = setting.effect.iter()
        .map(|&(ref name,nbr)| Effect::new(
                Path::new(&setting.effect_dir)
                .join(Path::new(&name))
                .as_path()
                ,nbr as usize
                ,setting.channels)
            )
        .collect();

    let mut music = Music::new((setting.global_volume*setting.music_volume as f32),setting.music_loop,setting.channels);

    let mut buffer_one: Vec<f32> = (0..setting.frames_per_buffer).map(|i| i as f32).collect();
    let mut buffer_two: Vec<f32> = (0..2*setting.frames_per_buffer).map(|i| i as f32).collect();

    let pa = try!(pa::PortAudio::new().map_err(|e| InitError::PortAudio(e)));

    let settings = try!(pa.default_output_stream_settings(setting.channels, setting.sample_rate, setting.frames_per_buffer)
                        .map_err(|e| InitError::PortAudio(e)));

    let callback = move |pa::OutputStreamCallbackArgs { buffer, frames, .. }| {
        // if the buffer set to 0 ?
        for elt in buffer.iter_mut() { *elt = 0.; }

        let frames = frames as i64;

        while let Ok(msg) = receiver.try_recv() {
            match msg {
                Msg::PlayEffect(n,vol) => effect[n].play(vol),
                Msg::SetMusicVolume(vol) => music.set_volume(vol),
                Msg::PlayMusic(snd_file) => music.set_music(snd_file),
                Msg::PauseMusic => music.pause(),
                Msg::ResumeMusic => music.resume(),
                Msg::SeekMusic(frame) => music.seek(frame),
                Msg::StopMusic => music.stop(),
                Msg::StopEffect => for e in &mut effect { e.stop(); },
                Msg::SetMusicLoop(l) => music.set_loop(l),
            }
        }

        music.fill_buffer(buffer, &mut buffer_one,&mut buffer_two, frames);

        for e in &mut effect {
            e.fill_buffer(buffer, &mut buffer_one, &mut buffer_two, frames);
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

    init_state(setting, sender, abort_sender);

    try!(init_stream(setting, receiver, abort_receiver));

    Ok(())
}

/// close the audio player, it can be init again.
pub fn close() {
    unsafe {
        if !RAW_STATE.is_null() {
            let mutex_state = Box::from_raw(RAW_STATE);
            let state = mutex_state.lock().unwrap();
            state.abort_sender.send(()).unwrap();
        }
        RAW_STATE = 0 as *mut Mutex<State>;
    }
}

/// reset audio from setting on the fly
pub fn reset(setting: &Setting) -> Result<(),InitError> {
    try!(check_setting(setting));

    let (sender,receiver) = channel();
    let (abort_sender,abort_receiver) = channel();

    let old_raw_state = unsafe { RAW_STATE };

    init_state(setting, sender, abort_sender);

    // drop old state
    {
        let old_mutex_state = unsafe { Box::from_raw(old_raw_state) };
        let old_state = old_mutex_state.lock().unwrap();
        old_state.abort_sender.send(()).unwrap();
    }

    try!(init_stream(setting, receiver, abort_receiver));
    Ok(())
}

struct State {
    music_status: MusicStatus,
    sender: Sender<Msg>,
    abort_sender: Sender<()>,
    listener: [f64;3],
    distance_model: DistanceModel,
    global_volume: f32,
    music_volume: f32,
    effect_volume: f32,
    music: Vec<PathBuf>,
}

impl State {
    fn from_setting(s: &Setting,sender: Sender<Msg>,abort_sender: Sender<()>) -> State {
        let music_dir = Path::new(&s.music_dir);
        let music: Vec<PathBuf> = s.music.iter().map(|name| music_dir.join(Path::new(&name))).collect();

        let music_status = MusicStatus {
            looping: s.music_loop,
            pause: false,
            id: None,
        };

        State {
            sender: sender,
            abort_sender: abort_sender,
            listener: [0.,0.,0.],
            music_status: music_status,
            distance_model: s.distance_model.clone(),
            global_volume: s.global_volume,
            music_volume: s.music_volume,
            effect_volume: s.effect_volume,
            music: music,
        }
    }
}

#[derive(Debug)]
enum Msg {
    PlayMusic(SndFile),
    SetMusicVolume(f32),
    PauseMusic,
    ResumeMusic,
    SeekMusic(i64),
    StopMusic,
    PlayEffect(usize,f32),
    StopEffect,
    SetMusicLoop(bool),
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

#[derive(Debug)]
struct Effect {
    start: usize,
    end: usize,
    batch: Vec<SndFile>,
    volume: Vec<f32>,
    channel_conv: ChannelConv,
}

impl Effect {
    fn new(path: &Path, capacity: usize, output_channels: i32) -> Effect {
        let mut batch = Vec::with_capacity(capacity);
        let mut volume = Vec::with_capacity(capacity);

        for _ in 0..capacity {
            batch.push(SndFile::new(path,OpenMode::Read).unwrap()); // unwrap because already checked
            volume.push(0.);
        }

        let channel_conv = ChannelConv::from_channels(batch[0].get_sndinfo().channels,output_channels);

        Effect {
            start: 0,
            end: 0,
            batch: batch,
            volume: volume,
            channel_conv: channel_conv,
        }
    }

    fn fill_buffer(&mut self, buffer_output: &mut [f32], buffer_one: &mut [f32], buffer_two: &mut [f32], frames: i64) {
        let range = if self.start < self.end {
            (self.start..self.end).chain(0..0)
        } else {
            (0..self.end).chain(self.start..self.batch.len())
        };

        for i in range {
            let frame = self.channel_conv.fill_buffer(
                &mut self.batch[i],
                self.volume[i],
                buffer_output,
                buffer_one,
                buffer_two,
                frames);

            if frame == 0 {
                self.start = (self.start+1).rem(self.batch.len());
            }
        }
    }

    fn stop(&mut self) {
        self.start = 0;
        self.end = 0;
    }

    fn play(&mut self,volume: f32) {
        self.volume[self.end] = volume;
        self.batch[self.end].seek(0,SeekMode::SeekSet);

        self.end = (self.end+1).rem(self.batch.len());
        if self.start == self.end {
            self.start = (self.start+1).rem(self.batch.len());
        }
    }
}

#[derive(Debug)]
struct Music {
    snd_file: Option<SndFile>,
    pause: bool,
    volume: f32,
    looping: bool,
    channel_conv: ChannelConv,
    output_channels: i32,
}

impl Music {
    fn new(volume: f32, looping: bool, output_channels: i32) -> Music {
        Music {
            snd_file: None,
            pause: false,
            volume: volume,
            looping: looping,
            channel_conv: ChannelConv::OneIntoOne,
            output_channels: output_channels,
        }
    }

    fn fill_buffer(&mut self, buffer_output: &mut [f32], buffer_one: &mut [f32], buffer_two: &mut [f32], frames: i64) {
        let destroy_snd_file = if let Some(ref mut snd_file) = self.snd_file {
            if self.pause {
                false
            } else {
                let frame = self.channel_conv.fill_buffer(
                    snd_file,
                    self.volume,
                    buffer_output,
                    buffer_one,
                    buffer_two,
                    frames);

                if frame == 0 {
                    if self.looping {
                        snd_file.seek(0,SeekMode::SeekSet);
                        false
                    } else {
                        true
                    }
                } else {
                    false
                }
            }
        } else {
            false
        };

        if destroy_snd_file {
            self.snd_file = None;
        }
    }

    fn stop(&mut self) {
        self.snd_file = None;
    }

    fn pause(&mut self) {
        self.pause = true;
    }

    fn resume(&mut self) {
        self.pause = false;
    }

    fn seek(&mut self, frame: i64) {
        if let Some(ref mut snd_file) = self.snd_file {
            snd_file.seek(frame,SeekMode::SeekSet);
        }
    }

    fn set_music(&mut self, snd_file: SndFile) {
        self.channel_conv = ChannelConv::from_channels(snd_file.get_sndinfo().channels,self.output_channels);
        self.snd_file = Some(snd_file);
    }

    fn set_loop(&mut self, looping: bool) {
        self.looping = looping;
    }

    fn set_volume(&mut self, v: f32) {
        self.volume = v;
    }
}

#[test]
fn test_complete_configuration() {
    use yaml_rust::yaml::YamlLoader;

    let s = Setting {
        check_level: CheckLevel::Always,
        channels: 2,
        sample_rate: 44_100f64,
        frames_per_buffer: 64,

        effect_dir: String::from("assets/effects"),
        music_dir: String::from("assets/musics"),

        global_volume: 0.5,
        music_volume: 0.5,
        effect_volume: 0.5,

        distance_model: DistanceModel::Linear(10.,100.),
        music_loop: true,

        effect: vec![(String::from("shoot.ogg"),10),(String::from("hit.ogg"),10)],
        music: vec![String::from("village.ogg"),String::from("forest.ogg")],
    };
    let doc = YamlLoader::load_from_str(
"---
check_level: always
channels: 2
sample_rate: 44100.
frames_per_buffer: 64

effect_dir: assets/effects
music_dir: assets/musics

global_volume: 0.5
music_volume: 0.5
effect_volume: 0.5

distance_model: [Linear,10.,100.]
music_loop: true

effect:
    - [shoot.ogg,10]
    - [hit.ogg,10]

music:
    - village.ogg
    - forest.ogg
...
").unwrap();
    assert_eq!(s,Setting::from_yaml(&doc[0]).unwrap());
}

#[test]
fn test_minimal_configuration() {
    use yaml_rust::yaml::YamlLoader;

    let s = Setting {
        check_level: CheckLevel::Never,
        channels: 2,
        sample_rate: 44_100f64,
        frames_per_buffer: 64,

        effect_dir: String::from("assets/effects"),
        music_dir: String::from("assets/musics"),

        global_volume: 0.5,
        music_volume: 0.5,
        effect_volume: 0.5,

        distance_model: DistanceModel::Linear(10.,100.),
        music_loop: false,

        effect: vec![],
        music: vec![],
    };
    let doc = YamlLoader::load_from_str(
"---
check_level: never
channels: 2
sample_rate: 44100.
frames_per_buffer: 64

effect_dir: assets/effects
music_dir: assets/musics

global_volume: 0.5
music_volume: 0.5
effect_volume: 0.5

distance_model: [Linear,10.,100.]
music_loop: false
effect:
music:
...
").unwrap();
    assert_eq!(s,Setting::from_yaml(&doc[0]).unwrap());
}

