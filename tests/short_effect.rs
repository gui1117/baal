extern crate baal;

use std::thread;
use std::time::Duration;

#[test]
fn persistent() {
    let setting = baal::Setting {
        effect_dir: "assets/musics".into(),
        music_dir: "assets/musics".into(),

        global_volume: 0.5,
        music_volume: 0.5,
        effect_volume: 0.5,

        distance_model: baal::effect::DistanceModel::Linear(1.,4.),

        music_transition: baal::music::MusicTransition::Instant,

        short_effects: vec!("first_call_kevin_macleod_incompetech.ogg".into()),
        persistent_effects: vec!(),
        musics: vec!(),
    };

    baal::init(&setting).expect("init baal");

    baal::effect::short::play(0,[0.0,0.0,0.0]);
    thread::sleep(Duration::from_secs(1));
    baal::effect::short::play(0,[0.0,0.0,0.0]);
    thread::sleep(Duration::from_secs(1));
    baal::effect::short::play(0,[0.0,0.0,0.0]);
    thread::sleep(Duration::from_secs(1));
    baal::effect::short::play(0,[0.0,0.0,0.0]);
    thread::sleep(Duration::from_secs(1));
    baal::effect::short::play(0,[0.0,0.0,0.0]);
    thread::sleep(Duration::from_secs(1));

    baal::effect::short::stop_all();
    thread::sleep(Duration::from_secs(1));

    baal::close();
}
