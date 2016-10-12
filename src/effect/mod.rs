//! this module allow to play short and persistent sound effects
//!
//! be careful that `set_volume`, `set_listener`, `set_distance_model`
//! only affect future short sound effects

pub mod persistent;
pub mod short;

use rodio::decoder::Decoder;
use rodio::Sink;
use rodio::Endpoint;
use rodio::Source;
use rodio::source::Buffered;

use std::fs::File;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;

use super::InitError;
use super::RAW_STATE;
use super::Setting;
use super::source;

#[doc(hidden)]
pub struct State {
    listener: [f32;3],
    distance_model: DistanceModel,
    volume: f32,
    final_volume: Arc<AtomicPtr<f32>>,
    pause: Arc<AtomicBool>,
    persistent_positions: Vec<Vec<[f32;3]>>,
    persistent_final_volumes: Vec<Arc<AtomicPtr<f32>>>,
    _persistent_sinks: Vec<Sink>,
    short_sinks: Vec<Sink>,
    short_sources: Vec<Buffered<Decoder<File>>>,
}
impl State {
    #[doc(hidden)]
    pub fn init(setting: &Setting, endpoint: &Endpoint) -> Result<State,InitError> {
        let pause = Arc::new(AtomicBool::new(false));
        let final_volume = Arc::new(AtomicPtr::new(&mut (setting.effect_volume * setting.global_volume)));

        let mut persistent_final_volumes = vec!();
        let mut persistent_positions = vec!();
        let mut persistent_sinks = vec!();

        for source in &setting.persistent_effects {
            let p_final_volume = Arc::new(AtomicPtr::new(&mut 0f32));

            let path = setting.effect_dir.join(source);
            let file = try!(File::open(path.clone()).map_err(|e| InitError::FileOpenError(source.clone(), e)));
            let source = try!(Decoder::new(file).map_err(|e| InitError::DecodeError(source.clone(), e)));
            let source = source::amplify_ctrl(source, p_final_volume.clone());
            let source = source::amplify_ctrl(source, final_volume.clone());
            let source = source::play_pause_ctrl(source, pause.clone());

            let sink = Sink::new(endpoint);
            sink.append(source);

            persistent_positions.push(vec!());
            persistent_final_volumes.push(p_final_volume);
            persistent_sinks.push(sink);
        }

        let mut short_sources = vec!();

        for source in &setting.short_effects {
            let path = setting.effect_dir.join(source);
            let file = try!(File::open(path.clone()).map_err(|e| InitError::FileOpenError(source.clone(), e)));
            let source = try!(Decoder::new(file).map_err(|e| InitError::DecodeError(source.clone(), e)));
            let source = source.buffered();

            short_sources.push(source);
        }

        Ok(State {
            listener: [0f32;3],
            distance_model: setting.distance_model.clone(),
            pause: pause,
            final_volume: final_volume,
            volume: setting.effect_volume,

            persistent_positions: persistent_positions,
            persistent_final_volumes: persistent_final_volumes,
            _persistent_sinks: persistent_sinks,

            short_sinks: vec!(),
            short_sources: short_sources
        })
    }
    #[doc(hidden)]
    pub fn reset(&mut self, setting: &Setting, endpoint: &Endpoint) -> Result<(),InitError> {
        *self = try!(State::init(setting, endpoint));
        Ok(())
    }
}

/// set the volume of sound effects
/// take effect for future sounds effects only
pub fn set_volume(v: f32) {
    let mut state = unsafe { (*RAW_STATE).write().unwrap() };
    state.effect.volume = v;
    update_volume();
}

#[doc(hidden)]
#[inline]
pub fn update_volume() {
    let state = unsafe { (*RAW_STATE).read().unwrap() };
    state.effect.final_volume.store(&mut (state.effect.volume * state.global_volume), Relaxed);
}


/// return the volume of sound effects
pub fn volume() -> f32 {
    let state = unsafe { (*RAW_STATE).read().unwrap() };
    state.effect.volume
}

/// pause all effects
pub fn pause() {
    let state = unsafe { (*RAW_STATE).read().unwrap() };
    state.effect.pause.store(true,Relaxed);
}

/// resume all effects
pub fn resume() {
    let state = unsafe { (*RAW_STATE).read().unwrap() };
    state.effect.pause.store(false,Relaxed);
}

/// return whereas effects are paused
pub fn is_paused() -> bool {
    let state = unsafe { (*RAW_STATE).read().unwrap() };
    state.effect.pause.load(Relaxed)
}

/// set the position of the listener
pub fn set_listener(pos: [f32;3]) {
    let mut state = unsafe { (*RAW_STATE).write().unwrap() };
    state.effect.listener = pos;
}

/// return the position of the listener
pub fn listener() -> [f32;3] {
    let state = unsafe { (*RAW_STATE).read().unwrap() };
    state.effect.listener
}

/// set the distance model
pub fn set_distance_model(d: DistanceModel) {
    let mut state = unsafe { (*RAW_STATE).write().unwrap() };
    state.effect.distance_model = d;
}

/// distance model, used to compute sound effects volumes.
#[derive(Clone,Debug,PartialEq)]
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
