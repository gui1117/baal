extern crate baal;

use std::thread;
use std::time::Duration;

#[test]
fn persistent() {
    let setting = baal::Setting {
        effect_dir: "assets/effects".into(),
        music_dir: "assets/musics".into(),

        global_volume: 0.5,
        music_volume: 0.5,
        effect_volume: 0.5,

        distance_model: baal::effect::DistanceModel::Linear(1.,4.),

        music_transition: baal::music::MusicTransition::Instant,

        short_effects: vec!(),
        persistent_effects: vec!("electro_fly_from_xonotic_game.ogg".into()),
        musics: vec!(),
    };

    baal::init(&setting).expect("init baal");

    baal::effect::persistent::add_position(0,[0.0,0.0,0.0]);

    baal::effect::persistent::update_volume(0);
    baal::effect::persistent::clear_positions(0);
    baal::effect::persistent::add_position(0,[1.2,1.2,1.2]);

    thread::sleep(Duration::from_secs(2));

    baal::effect::persistent::update_volume_for_all();
    baal::effect::persistent::clear_positions_for_all();

    thread::sleep(Duration::from_secs(2));

    baal::effect::persistent::add_position(0,[1.2,1.2,1.2]);
    baal::effect::persistent::add_position(0,[1.2,1.2,1.2]);
    baal::effect::persistent::add_position(0,[1.2,1.2,1.2]);
    baal::effect::persistent::add_position(0,[1.2,1.2,1.2]);
    baal::effect::persistent::add_position(0,[1.2,1.2,1.2]);
    baal::effect::persistent::add_position(0,[1.2,1.2,1.2]);

    baal::effect::persistent::update_volume_for_all();

    thread::sleep(Duration::from_secs(2));

    baal::effect::pause();

    baal::effect::persistent::update_volume_for_all();
    thread::sleep(Duration::from_secs(2));

    baal::effect::resume();

    thread::sleep(Duration::from_secs(2));

    baal::close();
}
