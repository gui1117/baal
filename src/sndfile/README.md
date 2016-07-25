rust-sndfile
============

__Libsndfile__ bindings and wrappers for Rust.

__Libsndfile__ is a library for reading and writing files containing sampled sound through one standard library interface.

website: [libsndfile](http://www.mega-nerd.com/libsndfile).

# Installation

You must install __libsndfile__ to build the binding. You can download it directly from the [website](http://www.mega-nerd.com/libsndfile/#Download),
or with your favorite package management tool.

Then clone the __rust-sndfile__ repository and build it with make.

# Fork Diff

SndFile no longer implement Clone.

SndFile impl Send.

SndFile impl Drop and close method is private
