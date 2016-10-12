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

        short_effects: vec!("shoot.ogg".into(),"hit.ogg".into()),
        persistent_effects: vec!(),
        musics: vec!("village.ogg".into()),
    };

    baal::init(&setting).expect("fail to init baal");

    baal::music::play(0);
    thread::sleep(Duration::from_secs(3));
    assert!(baal::music::is_stopped());
    baal::close();
}
