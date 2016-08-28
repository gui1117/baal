##TODO

### high priority
* [ ] ¿implement serde::serialize and serde::deserialize? is it stable now ?

### low priority
* [ ] third kind of sound: long effect -> return id on creation and position can be updated
* [ ] emscripten support with webaudio (when rustup support emscripten cross compilation)

### thought

* ! portaudio callback mustn't block so it must not acquire the mutex in channel!
  use mutex instead
* third kind of sound
