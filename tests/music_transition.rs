extern crate baal;

use std::thread;
use std::time::Duration;

#[test]
fn test() {
    let setting = baal::Setting {
        effect_dir: "assets/effects".into(),
        music_dir: "assets/musics".into(),

        global_volume: 0.5,
        music_volume: 0.5,
        effect_volume: 0.5,

        distance_model: baal::effect::DistanceModel::Linear(10.,110.),

        music_transition: baal::music::MusicTransition::Instant,

        short_effects: vec!(),
        persistent_effects: vec!(),
        musics: vec!("first_call_kevin_macleod_incompetech.ogg".into()),
    };

    baal::init(&setting).expect("fail to init baal");

    thread::sleep(Duration::from_secs(1));

    baal::music::play(0);
    thread::sleep(Duration::from_secs(4));

    baal::music::play(0);
    thread::sleep(Duration::from_secs(4));

    baal::music::set_transition(baal::music::MusicTransition::Smooth(Duration::from_secs(2)));
    baal::music::play(0);
    thread::sleep(Duration::from_secs(4));

    baal::music::set_transition(baal::music::MusicTransition::Overlap(Duration::from_secs(2)));
    baal::music::play(0);
    thread::sleep(Duration::from_secs(10));
    baal::close();
}
