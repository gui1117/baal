##TODO

### high priority
* [x] replace mutex by reader/writer lock
* [ ] music playlist and transition
* [ ] ¿implement serde::serialize and serde::deserialize?
* [ ] ¿third kind of sound: long effect -> return id on creation and position can be updated?
* [x] reexport yaml-rust
* [ ] let device index being configurable

### low priority
* [ ] emscripten support with webaudio (when rustup support emscripten cross compilation)

##THOUGHT

transition can be smooth(dt), overlap(dt), instant

callback when music end so user can implement whatever he want
-> bad, this method is called in the critic part

music status have a field stop

rewrite the music module

```
status() -> Status // receive message from thread
Status {
	Pause,
	Stop,
	Play,
}
index() -> Option<usize>
looping() -> bool
set_looping(bool)
transition() -> bool
set_transition(trans)
```

