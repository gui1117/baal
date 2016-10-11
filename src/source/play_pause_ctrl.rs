use std::time::Duration;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use rodio::Sample;
use rodio::Source;

/// Internal function that builds a `PlayPauseCtrl` object.
pub fn play_pause_ctrl<I>(input: I, pause: Arc<AtomicBool>) -> PlayPauseCtrl<I>
                  where I: Source, I::Item: Sample
{
    PlayPauseCtrl {
        input: input,
        pause: pause,
    }
}

/// Filter that modifies each sample by a given value.
#[derive(Clone, Debug)]
pub struct PlayPauseCtrl<I> where I: Source, I::Item: Sample {
    input: I,
    pause: Arc<AtomicBool>,
}

impl<I> Iterator for PlayPauseCtrl<I> where I: Source, I::Item: Sample {
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        if !self.pause.load(Ordering::Relaxed) {
            self.input.next()
        } else {
            Some(I::Item::zero_value())
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.input.size_hint()
    }
}

impl<I> ExactSizeIterator for PlayPauseCtrl<I> where I: Source + ExactSizeIterator, I::Item: Sample {
}

impl<I> Source for PlayPauseCtrl<I> where I: Source, I::Item: Sample {
    #[inline]
    fn get_current_frame_len(&self) -> Option<usize> {
        self.input.get_current_frame_len()
    }

    #[inline]
    fn get_channels(&self) -> u16 {
        self.input.get_channels()
    }

    #[inline]
    fn get_samples_rate(&self) -> u32 {
        self.input.get_samples_rate()
    }

    #[inline]
    fn get_total_duration(&self) -> Option<Duration> {
        self.input.get_total_duration()
    }
}
