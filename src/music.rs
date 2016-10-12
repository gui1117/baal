//! this module allow to play music

use rodio::decoder::Decoder;
use rodio::Sink;
use rodio::Source;

use std::fs::File;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;
use std::time::Duration;
use std::path::PathBuf;

use super::InitError;
use super::RAW_STATE;
use super::Setting;
use super::source;

struct Current {
    index: usize,
    fade_out: Arc<AtomicBool>,
    sink: Sink,
}

#[doc(hidden)]
pub struct State {
    transition: MusicTransition,
    volume: f32,
    final_volume: Arc<AtomicUsize>,
    pause: Arc<AtomicBool>,
    sources: Vec<PathBuf>,
    current: Option<Current>,
}
impl State {
    #[doc(hidden)]
    pub fn init(setting: &Setting) -> Result<State,InitError> {
        let mut sources = vec!();

        for source in &setting.musics {
            let path = setting.music_dir.join(source);
            let file = try!(File::open(path.clone()).map_err(|e| InitError::FileOpenError(source.clone(), e)));
            try!(Decoder::new(file).map_err(|e| InitError::DecodeError(source.clone(), e)));

            sources.push(path);
        }

        Ok(State {
            transition: setting.music_transition,
            final_volume: Arc::new(AtomicUsize::new((setting.music_volume * setting.global_volume * 10_000f32) as usize)),
            pause: Arc::new(AtomicBool::new(false)),
            volume: setting.music_volume,
            sources: sources,
            current: None,
        })
    }
    #[doc(hidden)]
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
    update_volume(&mut *state);
}

#[doc(hidden)]
#[inline]
pub fn update_volume(state: &mut super::State) {
    state.music.final_volume.store((state.music.volume * state.global_volume * 10_000f32) as usize, Relaxed);
}

/// return the volume of the music
pub fn volume() -> f32 {
    let state = unsafe { (*RAW_STATE).read().unwrap() };
    state.music.volume
}

/// play the music
pub fn play(music: usize) {
    let mut state = unsafe { (*RAW_STATE).write().unwrap() };
    play_inner(music, &mut state);
}

#[inline]
fn play_inner(music: usize, state: &mut super::State) {
    use self::MusicTransition::*;

    stop_inner(state);

    let fade_out = Arc::new(AtomicBool::new(false));
    let sink = Sink::new(&state.endpoint);

    let source = Decoder::new(File::open(state.music.sources[music].clone()).unwrap()).unwrap();
    let source = match state.music.transition {
        Smooth(duration) => {
            let source = source::fade_out_ctrl(source, duration, fade_out.clone());
            let source = source.fade_in(duration);
            let source = source::wait(source, duration);
            source
        },
        Overlap(duration) => {
            let source = source::fade_out_ctrl(source, duration, fade_out.clone());
            let source = source.fade_in(duration);
            let source = source::wait(source, Duration::new(0, 0));
            source
        }
        Instant => {
            let source = source::fade_out_ctrl(source, Duration::new(0, 0), fade_out.clone());
            let source = source.fade_in(Duration::new(0, 0));
            let source = source::wait(source, Duration::new(0, 0));
            source
        },
    };
    let source = source::amplify_ctrl(source, state.music.final_volume.clone());
    let source = source::play_pause_ctrl(source, state.music.pause.clone());

    sink.append(source);

    state.music.current = Some(Current {
        index: music,
        sink: sink,
        fade_out: fade_out,
    });
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
    state.music.pause.store(true,Relaxed);
}

/// resume the music
pub fn resume() {
    let state = unsafe { (*RAW_STATE).read().unwrap() };
    state.music.pause.store(false,Relaxed);
}

/// return whereas music is paused
pub fn is_paused() -> bool {
    let state = unsafe { (*RAW_STATE).read().unwrap() };
    state.music.pause.load(Relaxed)
}

/// stop the music
pub fn stop() {
    let mut state = unsafe { (*RAW_STATE).write().unwrap() };
    stop_inner(&mut state);
}

#[inline]
fn stop_inner(state: &mut super::State) {
    if let Some(current) = state.music.current.take() {
        current.fade_out.store(true,Relaxed);
        current.sink.detach();
    }
}

/// return whereas music is stopped
pub fn is_stopped() -> bool {
    let state = unsafe { (*RAW_STATE).read().unwrap() };
    state.music.current.is_none()
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
}

/// return the index of the current music if any
pub fn index() -> Option<usize> {
    let state = unsafe { (*RAW_STATE).read().unwrap() };
    state.music.current.as_ref().map(|current| current.index)
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
#[derive(Clone,Copy,Debug,PartialEq)]
pub enum MusicTransition {
    /// the current music end smoothly and then the new one is played.
    Smooth(Duration),
    /// the current music end smoothly while the new one begin smoothly.
    Overlap(Duration),
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
