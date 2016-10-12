use std::time::Duration;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use rodio::Sample;
use rodio::Source;

/// Internal function that builds a `AmplifyCtrl` object.
pub fn amplify_ctrl<I>(input: I, factor: Arc<AtomicUsize>) -> AmplifyCtrl<I>
                  where I: Source, I::Item: Sample
{
    AmplifyCtrl {
        input: input,
        factor: factor,
    }
}

/// Filter that modifies each sample by a given value.
#[derive(Clone, Debug)]
pub struct AmplifyCtrl<I> where I: Source, I::Item: Sample {
    input: I,
    factor: Arc<AtomicUsize>,
}

impl<I> Iterator for AmplifyCtrl<I> where I: Source, I::Item: Sample {
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        self.input.next().map(|value| value.amplify(self.factor.load(Ordering::Relaxed) as f32 / 10_000f32))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.input.size_hint()
    }
}

impl<I> ExactSizeIterator for AmplifyCtrl<I> where I: Source + ExactSizeIterator, I::Item: Sample {
}

impl<I> Source for AmplifyCtrl<I> where I: Source, I::Item: Sample {
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
