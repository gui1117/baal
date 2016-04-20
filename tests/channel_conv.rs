extern crate baal;
extern crate yaml_rust;

use yaml_rust::yaml::YamlLoader;
use std::thread;
use std::time::Duration;

static ONE_CHANNEL_YAML_CONFIG: &'static str =
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
    - [explosion.ogg,1]
    - [stereo_explosion.ogg,1]
music:
...
";

static TWO_CHANNEL_YAML_CONFIG: &'static str =
"---
check_level: always
channels: 2
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
    - [explosion.ogg,1]
    - [stereo_explosion.ogg,1]
music:
...
";

#[test]
fn channel_conv() {
    let one_channel_yaml_config = YamlLoader::load_from_str(ONE_CHANNEL_YAML_CONFIG).unwrap();
    let two_channel_yaml_config = YamlLoader::load_from_str(TWO_CHANNEL_YAML_CONFIG).unwrap();

    let one_channel_setting = baal::Setting::from_yaml(&one_channel_yaml_config[0]).unwrap();
    let two_channel_setting = baal::Setting::from_yaml(&two_channel_yaml_config[0]).unwrap();

    baal::init(&one_channel_setting).unwrap();

    baal::effect::play(0,&[0.,0.,0.]);
    thread::sleep(Duration::from_secs(2));
    baal::effect::play(1,&[0.,0.,0.]);
    thread::sleep(Duration::from_secs(5));

    baal::reset(&two_channel_setting).unwrap();

    baal::effect::play(0,&[0.,0.,0.]);
    thread::sleep(Duration::from_secs(2));
    baal::effect::play(1,&[0.,0.,0.]);
    thread::sleep(Duration::from_secs(5));

    baal::close();
}
