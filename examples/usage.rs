extern crate baal;
extern crate yaml_rust;

use yaml_rust::yaml::YamlLoader;
use std::thread;
use std::time::Duration;

static YAML_CONFIG: &'static str =
"---
check_level: always
channels: 1
sample_rate: 44100.
frames_per_buffer: 64

effect_dir: assets/effects
music_dir: assets/musics

global_volume: 0.5
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

fn main() {
    let yaml_config = YamlLoader::load_from_str(YAML_CONFIG).unwrap();

    let setting = baal::Setting::from_yaml(&yaml_config[0]).unwrap();

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
