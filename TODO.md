##TODO

### high priority
* [x] replace mutex by reader/writer lock
* [x] music transition TOTEST
* [x] reexport yaml-rust
* [x] status() -> Status // receive stop message from thread TOTEST
* [ ] ¿implement serde::serialize and serde::deserialize?
* [ ] third kind of sound: long effect -> return id on creation and position can be updated
* [ ] no longer use yaml-rust but derive decodable if possible
* [ ] no longer status for music but music::play use music id, transition and bool (if restart if already in play)
* [ ] maybe no longer state in mutex: just channels
* [ ] update doc

* three kind of sound: music, sound, long sound
  it would complicate the api

### low priority
* [ ] emscripten support with webaudio (when rustup support emscripten cross compilation)

##THOUGHT

standard interface for sound and effect
batch of sound
batch sound are extended and resize as vector but the minimun is in the setting
when user sound drop send sound drop and then it free the sound in the batch when it finished
when playing a sound you send a String ? it uses a hashmap to store the sound
