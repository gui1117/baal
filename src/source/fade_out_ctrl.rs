use std::time::Duration;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;

use rodio::Sample;
use rodio::Source;

/// Internal function that builds a `FadeOutCtrl` object.
pub fn fade_out_ctrl<I>(input: I, duration: Duration, signal: Arc<AtomicBool>) -> FadeOutCtrl<I>
                  where I: Source, I::Item: Sample
{
    let duration = duration.as_secs() * 1000000000 + duration.subsec_nanos() as u64;

    FadeOutCtrl {
        input: input,
        signal: signal,
        remaining_ns: duration as f32,
        total_ns: duration as f32,
    }
}

/// Filter that modifies each sample by a given value.
#[derive(Clone, Debug)]
pub struct FadeOutCtrl<I> where I: Source, I::Item: Sample {
    input: I,
    signal: Arc<AtomicBool>,
    remaining_ns: f32,
    total_ns: f32,
}

impl<I> Iterator for FadeOutCtrl<I> where I: Source, I::Item: Sample {
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        if !self.signal.load(Relaxed) {
            self.input.next()
        } else if self.remaining_ns > 0.0 {
            let factor = self.remaining_ns / self.total_ns;
            self.remaining_ns -= 1000000000.0 / (self.input.get_samples_rate() as f32 *
                                                 self.get_channels() as f32);
            self.input.next().map(|value| value.amplify(factor))
        } else {
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.input.size_hint()
    }
}

impl<I> Source for FadeOutCtrl<I> where I: Source, I::Item: Sample {
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
