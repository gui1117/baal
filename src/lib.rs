//! baal is an audio api for games that focus on simplicity.
//!
//! it doesn't provide spatialization.
//!
//! feature:
//!
//! * yaml configuration so you can easily test sounds without recompile
//! * music player: detail in music mode
//! * effect player: detail in effect mode

//TODO conversion for channels

#[macro_use]
extern crate lazy_static;
extern crate yaml_rust;
extern crate portaudio;
extern crate sndfile;

use yaml_rust::yaml::Yaml;
use std::sync::mpsc::{Sender, channel};
use std::sync::Mutex;
use sndfile::{SndFile, OpenMode, SeekMode};
use portaudio as pa;
use std::thread;
use std::path::{Path, PathBuf};
use std::ops::Rem;
use effect::DistanceModel;

static mut RAW_STATE: *mut Mutex<State> = 0 as *mut Mutex<State>;

lazy_static! {
    static ref STATE: Mutex<State> = {
        unsafe {
            if !RAW_STATE.is_null() {
                *Box::from_raw(RAW_STATE)
            } else {
                panic!("audio not initiated");
            }
        }
    };
}

#[derive(Clone,Debug,PartialEq)]
/// set musics, effects, volumes and audio player.
pub struct Setting {
    /// number of channels: 1
    pub channels: i32,
    /// sample rate: mostly 44_100
    pub sample_rate: f64,
    /// number of frame per buffer: 64
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
}

impl Setting {
    /// import setting from yaml:
    ///
    ///```
    ///---
    ///channels: 1
    ///sample_rate: 44100.
    ///frames_per_buffer: 64
    ///
    ///effect_dir: assets/effects
    ///music_dir: assets/musics
    ///
    ///global_volume: 0.5
    ///music_volume: 0.8
    ///effect_volume: 0.3
    ///
    ///distance_model: [Pow2,10.,110.]
    ///music_loop: true
    ///
    ///effect:
    ///    - [shoot.ogg,10]
    ///    - [hit.ogg,10]
    ///
    ///music:
    ///    - village.ogg
    ///...
    ///```
    ///
    pub fn from_yaml(code: &Yaml) -> Self {
        let hash = code.as_hash().expect("config must be an associative array");
        let distance_model = {
            let vec = hash.get(&Yaml::String(String::from("distance_model")))
                .expect("config map must have distance_model key").as_vec().expect("distance model must be vector");
            match vec[0].as_str().expect("distance model first element must be the string of the enum") {
                "Linear" => DistanceModel::Linear(
                    vec[1].as_f64().expect("linear distance model second element must be a float"),
                    vec[2].as_f64().expect("linear distance model third element must be a float")
                    ),
                "Pow2" => DistanceModel::Pow2(
                    vec[1].as_f64().expect("exponential distance model second element must be a float"),
                    vec[2].as_f64().expect("exponential distance model third element must be a float")
                    ),
                    _ => panic!("distance model first element is not a correct enum"),
            }
        };
        let effect = {
            let key = hash.get(&Yaml::String(String::from("effect"))).expect("config map must have effect key");
            if let Some(vec) = key.as_vec() {
                vec.iter()
                    .map(|y| {
                        let vec = y.as_vec().expect("element of effect list must be a vector");
                        (String::from(vec[0].as_str().expect("first element of effect list must be a string")),
                        vec[1].as_i64().expect("second element of effect list must be an integer") as u32)
                    }).collect()
            } else if key.is_null() {
                vec!()
            } else {
                panic!("effect must a list or null");
            }
        };

        let music = {
            let key = hash.get(&Yaml::String(String::from("music"))).expect("config map must have music key");
            if let Some(vec) = key.as_vec() {
                vec.iter()
                    .map(|y| String::from(y.as_str().expect("element of music must a string")))
                    .collect()
            } else if key.is_null() {
                vec!()
            } else {
                panic!("music must a list or null");
            }
        };

        Setting {
            channels: hash.get(&Yaml::String(String::from("channels")))
                .expect("config map must have a channels key").as_i64().expect("channels must be integer") as i32,
            sample_rate: hash.get(&Yaml::String(String::from("sample_rate")))
                .expect("config map must have a sample_rate key").as_f64().expect("sample_rate must be float"),
            frames_per_buffer: hash.get(&Yaml::String(String::from("frames_per_buffer")))
                .expect("config map must have a frames_per_buffer key").as_i64().expect("frames_per_buffer must be integer") as u32,

            music_dir: String::from(hash.get(&Yaml::String(String::from("music_dir")))
                .expect("config map must have a music_dir key").as_str().expect("music_dir must be string")),
            effect_dir: String::from(hash.get(&Yaml::String(String::from("effect_dir")))
                .expect("config map must have a effect_dir key").as_str().expect("effect_dir must be string")),

            global_volume: hash.get(&Yaml::String(String::from("global_volume")))
                .expect("config map must have a global_volume key").as_f64().expect("global volume must be float") as f32,
            music_volume: hash.get(&Yaml::String(String::from("music_volume")))
                .expect("config map must have a music_volume key").as_f64().expect("music volume must be float") as f32,
            effect_volume: hash.get(&Yaml::String(String::from("effect_volume")))
                .expect("config map must have a effect_volume key").as_f64().expect("effect volume must be float") as f32,

            distance_model: distance_model,
            music_loop: hash.get(&Yaml::String(String::from("music_loop")))
                .expect("config map must have a music_loop key").as_bool().expect("music_loop must be bool"),

            effect: effect,
            music: music,
        }
    }
}

pub mod effect {
    //! this module allow to play sound effect
    //!
    //! volume = `global_volume * effect_volume * distance([x,y,z],listener_position)`
    //!
    //! but once a sound effect is played at a volume it doesn't change its volume anymore
    //! this can lead to weird effects for long sound effects

    use super::{STATE, Msg};

    /// set the volume of sound effects
    /// take effect for future sounds effects only
    pub fn set_volume(v: f32) {
        let mut state = STATE.lock().unwrap();
        state.effect_volume = v;
    }

    /// get the volume of sound effects
    pub fn volume() -> f32 {
        let state = STATE.lock().unwrap();
        state.effect_volume
    }

    /// play the sound effect at the volume: `global_volume * effect_volume *
    /// distance([x,y,z],listener_position)`
    pub fn play(effect: usize, pos: &[f64;3]) {
        let state = STATE.lock().unwrap();
        let volume = state.global_volume * state.effect_volume * state.distance_model.distance(pos,&state.listener);
        if volume > 0. {
            state.sender.send(Msg::PlayEffect(effect,volume)).unwrap();
        }
    }

    /// stop all sound effects
    pub fn stop_all() {
        let state = STATE.lock().unwrap();
        state.sender.send(Msg::StopEffect).unwrap();
    }

    /// set the position of the listener
    pub fn set_listener(x: f64, y: f64, z: f64) {
        let mut state = STATE.lock().unwrap();
        state.listener = [x,y,z];
    }

    /// return the position of the listener
    pub fn listener() -> [f64;3] {
        let state = STATE.lock().unwrap();
        state.listener
    }

    /// set the distance model
    /// take effect for future sounds effects only
    pub fn set_distance_model(d: DistanceModel) {
        let mut state = STATE.lock().unwrap();
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
}

pub mod music {
    //! this module allow to play music

    use super::{STATE, Msg};
    use super::sndfile::{SndFile, OpenMode};

    /// set the volume of the music
    /// the actual music volume is `music_volume * global_volume`
    pub fn music_set_volume(v: f32) {
        let mut state = STATE.lock().unwrap();
        state.music_volume = v;
        state.sender.send(Msg::SetMusicVolume(state.music_volume*state.global_volume)).unwrap();
    }

    /// get the volume of the music
    pub fn music_volume() -> f32 {
        let state = STATE.lock().unwrap();
        state.music_volume
    }

    /// seek the music to a given frame
    pub fn music_seek(frame: i64) {
        let state = STATE.lock().unwrap();
        state.sender.send(Msg::SeekMusic(frame)).unwrap();
    }

    /// play the music
    pub fn music_play(music: usize) {
        let mut state = STATE.lock().unwrap();
        state.music_status.pause = false;
        state.music_status.id = Some(music);
        let snd_file = SndFile::new(&state.music[music],OpenMode::Read).unwrap();
        state.sender.send(Msg::PlayMusic(snd_file)).unwrap();
    }

    /// pause the music
    pub fn music_pause() {
        let mut state = STATE.lock().unwrap();
        state.music_status.pause = true;
        state.sender.send(Msg::PauseMusic).unwrap();
    }

    /// resume the music
    pub fn music_resume() {
        let mut state = STATE.lock().unwrap();
        state.music_status.pause = false;
        state.sender.send(Msg::ResumeMusic).unwrap();
    }

    /// stop the music
    pub fn music_stop() {
        let mut state = STATE.lock().unwrap();
        state.music_status.pause = false;
        state.music_status.id = None;
        state.sender.send(Msg::StopMusic).unwrap();
    }

    /// return the current status of the music
    pub fn music_status() -> MusicStatus {
        let state = STATE.lock().unwrap();
        state.music_status
    }

    /// set whereas music loop or not
    pub fn music_set_loop(l: bool) {
        let mut state = STATE.lock().unwrap();
        state.music_status.looping = l;
        state.sender.send(Msg::SetMusicLoop(l)).unwrap();
    }

    /// return whereas music loop or not.
    pub fn music_loop() -> bool {
        let state = STATE.lock().unwrap();
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
    let mut state = STATE.lock().unwrap();
    state.global_volume = v;
    state.sender.send(Msg::SetMusicVolume(state.music_volume*state.global_volume)).unwrap();
}

/// get the global volume
pub fn volume() -> f32 {
    let state = STATE.lock().unwrap();
    state.global_volume
}

/// stop music and effects
pub fn stop() {
    let state = STATE.lock().unwrap();
    state.sender.send(Msg::StopMusic).unwrap();
    state.sender.send(Msg::StopEffect).unwrap();
}

/// init the audio player
pub fn init(setting: Setting) {
    let (sender,receiver) = channel();
    let (abort_sender,abort_receiver) = channel();

    let channels = setting.channels;

    let sample_rate = setting.sample_rate;

    let frames_per_buffer = setting.frames_per_buffer;

    let buffer_size = (channels as usize) * (frames_per_buffer as usize);

    let mut effect: Vec<Effect> = setting.effect.iter()
        .map(|&(ref name,nbr)| Effect::new(
                Path::new(&setting.effect_dir)
                .join(Path::new(&name))
                .as_path()
                ,nbr as usize)
            )
        .collect();

    let mut music = Music::new((setting.global_volume*setting.music_volume as f32),setting.music_loop);

    let state = State::from_setting(setting,sender,abort_sender);

    unsafe {
        assert!(RAW_STATE.is_null());
        let box_state = Box::new(Mutex::new(state));
        RAW_STATE = Box::into_raw(box_state);
    }

    thread::spawn(move || {
        let mut buffer_p: Vec<f32> = (0..buffer_size).map(|i| i as f32).collect();

        let pa = pa::PortAudio::new().unwrap();

        let settings = pa.default_output_stream_settings(channels, sample_rate, frames_per_buffer).unwrap();

        let callback = move |pa::OutputStreamCallbackArgs { buffer, frames, .. }| {
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

            music.fill_buffer(buffer,frames);

            for e in &mut effect {
                e.fill_buffer(buffer,&mut buffer_p ,frames);
            }

            pa::Continue
        };

        let mut stream = pa.open_non_blocking_stream(settings, callback).unwrap();

        stream.start().unwrap();

        match abort_receiver.recv() {
            Ok(()) => (),
            Err(_) => (),
        }
    });
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
    fn from_setting(s: Setting,sender: Sender<Msg>,abort_sender: Sender<()>) -> State {
        let music_dir = Path::new(&s.music_dir);
        let music: Vec<PathBuf> = s.music.iter().map(|name| music_dir.join(Path::new(&name))).collect();

        let music_status = MusicStatus {
            looping: s.music_loop,
            pause: false,
            id: None,
        };

        debug_assert!({
            for m in &music {
                let snd_file = SndFile::new(m,OpenMode::Read).expect(&format!("cannot load {:#?}",m));
                let snd_info = snd_file.get_sndinfo();
                if snd_info.samplerate as f64 != s.sample_rate {
                    panic!("samplerate of {:#?} differ from setting",m);
                }
                if snd_info.channels != s.channels {
                    panic!("channels of {:#?} differ from setting",m);
                }
            }
            true
        });

        State {
            sender: sender,
            abort_sender: abort_sender,
            listener: [0.,0.,0.],
            music_status: music_status,
            distance_model: s.distance_model,
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

#[derive(Debug)]
struct Effect {
    start: usize,
    end: usize,
    batch: Vec<SndFile>,
    volume: Vec<f32>,
}

impl Effect {
    fn new(path: &Path, capacity: usize) -> Effect {
        let mut batch = Vec::with_capacity(capacity);
        let mut volume = Vec::with_capacity(capacity);

        for _ in 0..capacity {
            batch.push(SndFile::new(path,OpenMode::Read).expect("cannot load effect file"));
            volume.push(0.);
        }

        Effect {
            start: 0,
            end: 0,
            batch: batch,
            volume: volume,
        }
    }

    fn fill_buffer(&mut self, buffer: &mut [f32], buffer_p: &mut [f32], frames: i64) {
        let range = if self.start <= self.end {
            (self.start..self.end).chain(0..0)
        } else {
            (0..self.end).chain(self.start..self.batch.len())
        };

        for i in range {
            let frame = self.batch[i].readf_f32(buffer_p,frames);
            for k in 0..buffer.len() {
                buffer[k] += buffer_p[k]*self.volume[i];
            }
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
}

impl Music {
    fn new(volume: f32, looping: bool) -> Music {
        Music {
            snd_file: None,
            pause: false,
            volume: volume,
            looping: looping,
        }
    }

    fn fill_buffer(&mut self, buffer: &mut [f32], frames: i64) {
        let destroy_snd_file = if let Some(ref mut snd_file) = self.snd_file {
            if !self.pause {
                let frame = snd_file.readf_f32(buffer,frames);
                for elt in buffer {
                    *elt *= self.volume;
                }
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
            } else {
                for elt in buffer { *elt = 0.; }
                false
            }
        } else {
            for elt in buffer { *elt = 0.; }
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
    assert_eq!(s,Setting::from_yaml(&doc[0]));
}

#[test]
fn test_minimal_configuration() {
    use yaml_rust::yaml::YamlLoader;

    let s = Setting {
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
    assert_eq!(s,Setting::from_yaml(&doc[0]));
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

