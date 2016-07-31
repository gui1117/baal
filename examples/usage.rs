extern crate baal;

use std::thread;
use std::time::Duration;

fn main() {
    let setting = baal::Setting {
        channels: 2,
        sample_rate: 44100.,
        frames_per_buffer: 64,

        effect_dir: "assets/effects".into(),
        music_dir: "assets/musics".into(),

        global_volume: 0.5,
        music_volume: 0.5,
        effect_volume: 0.5,

        distance_model: baal::effect::DistanceModel::Linear(10.,110.),

        music_loop: true,

        music_transition: baal::music::MusicTransition::Instant,

        effect: vec!(("explosion.ogg".into(),1),("stereo_explosion.ogg".into(),1)),
        music: vec!(),

        check_level: baal::CheckLevel::Always,
    };

    baal::init(&setting).unwrap();
    baal::music::play(0);

    for i in 0..7 {
        let p = (i*20) as f64;
        baal::effect::play(0,&[p,0.,0.]);
        thread::sleep(Duration::from_millis(1));
        baal::effect::play(1,&[p,0.,0.]);
        thread::sleep(Duration::from_millis(400));
    }

    baal::close();
}
