## TODO

### high priority
* [ ] ¿implement serde::serialize and serde::deserialize? is it stable now ?

### low priority
* [ ] third kind of sound: long effect -> return id on creation and position can be updated
* [ ] emscripten support with webaudio (when rustup support emscripten cross compilation)

### thought

* ! portaudio callback mustn't block so it must not acquire the mutex in channel!
  use mutex instead
  does channel can block ??

  as far as I know a channel is a vec in a mutex.
  does the receiver try recv can block during a sending process ?

  it mustn't be a problem

* third kind of sound: ambiant snd:
  * new: return an index
  * add position
  * remove position
  * clear positions
  * update volume
  * ¿list position? depend if they are store or not, may be useless

  * design choice: whereas we store all position and then compute the volume on update volume or compute continuously on add and remove
