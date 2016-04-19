extern crate baal;
extern crate yaml_rust;

use yaml_rust::yaml::YamlLoader;
use std::thread;
use std::time::Duration;

static YAML_CONFIG: &'static str =
"---
channels: 1
sample_rate: 44100.
frames_per_buffer: 64

effect_dir: assets/effects
music_dir: assets/musics

global_volume: 0.
music_volume: 0.8
effect_volume: 0.3

distance_model: [Pow2,10.,110.]
music_loop: true

effect:
    - [shoot.ogg,10]
    - [hit.ogg,10]

music:
    - village.ogg
...
";

#[test]
fn test() {
    let yaml_config = YamlLoader::load_from_str(YAML_CONFIG).expect("fail to load yaml config");

    let setting = baal::Setting::from_yaml(&yaml_config[0]).expect("fail to create setting");

    baal::init(&setting).expect("fail to init baal");

    let child = std::thread::spawn(|| {
        for _ in 0..20 {
            baal::effect::play(0,&[0.,0.,0.]);
            thread::sleep(Duration::from_millis(1));
        }
    });
    for _ in 0..20 {
        baal::reset(&setting).expect("fail to reset baal");
        thread::sleep(Duration::from_millis(1));
    }
    child.join().unwrap();
    baal::close();
}
