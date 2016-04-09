#baal

BAsic Audio Library

documentation:

baal is a cross-plateform audio api for games that focus on simplicity.

it doesn't provide spatialization.

mp3 is not available because of licensing fees
use ogg vorbis or other format instead

for more information about format available see [libsndfile#features](http://www.mega-nerd.com/libsndfile/#features)

for more information about why not mp3 as lots of other foss handle it see [libsndfile#whynotmp3](http://www.mega-nerd.com/libsndfile/FAQ.html#Q020)

feature:

* yaml configuration so you can easily test sounds without recompile
* music player
* effect player

#dependencies

* libsndfile:

  On Ubuntu / Debian:
  ```sh
  apt-get install libsndfile1-dev
  ```

  from website: [libsndfile](http://www.mega-nerd.com/libsndfile/#Download)

* portaudio:

  rust-portaudio will try to detect portaudio on your system and,
  failing that (or if given the PORTAUDIO\_ONLY\_STATIC environment variable on the build process),
  will download and build portaudio statically.
  If this fails please let us know!
  In the mean-time, you can manually download and install [PortAudio](http://www.portaudio.com/download.html) yourself.

#TODO

* channels conversion
