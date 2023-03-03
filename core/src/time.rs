// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::{
    traits::{Resets, Ticks},
    ParameterType,
};
use std::{
    cmp::Ordering,
    fmt::Display,
    ops::{Add, Mul},
};

// A way to specify a time unit that Clock tracks.
#[derive(Clone, Debug, Default)]
pub enum ClockTimeUnit {
    #[default]
    Seconds,
    #[allow(dead_code)]
    Beats,
    #[allow(dead_code)]
    Samples,
    #[allow(dead_code)]
    MidiTicks,
}

/// A timekeeper that operates in terms of sample rate.
#[derive(Clone, Debug)]
pub struct Clock {
    /// The number of frames per second. Usually 44.1KHz for CD-quality audio.
    sample_rate: usize,
    bpm: ParameterType,
    midi_ticks_per_second: usize,

    /// Samples since clock creation. It's called "frames" because tick() was
    /// already being used as a verb by the Ticks trait, and "samples" is a very
    /// overloaded term in digital audio. A synonymous term is "time slices,"
    /// used when the emphasis is on division of work into small parts.
    frames: usize,

    seconds: f64, // Seconds elapsed since clock creation.
    // Beats elapsed since clock creation. Not
    // https://en.wikipedia.org/wiki/Swatch_Internet_Time
    beats: f64,

    // Typically 960 ticks per second
    midi_ticks: usize,

    // True if anything unusual happened since the last tick, or there was no
    // last tick because this is the first.
    was_reset: bool,
}

impl Clock {
    pub fn new_with(
        sample_rate: usize,
        beats_per_minute: ParameterType,
        midi_ticks_per_second: usize,
    ) -> Self {
        Self {
            sample_rate,
            bpm: beats_per_minute,
            midi_ticks_per_second,
            frames: Default::default(),
            seconds: Default::default(),
            beats: Default::default(),
            midi_ticks: Default::default(),
            was_reset: true,
        }
    }

    pub fn was_reset(&self) -> bool {
        self.was_reset
    }

    pub fn frames(&self) -> usize {
        self.frames
    }
    pub fn seconds(&self) -> f64 {
        self.seconds
    }
    pub fn beats(&self) -> f64 {
        self.beats
    }
    pub fn midi_ticks(&self) -> usize {
        self.midi_ticks
    }
    pub fn sample_rate(&self) -> usize {
        self.sample_rate
    }
    pub fn bpm(&self) -> ParameterType {
        self.bpm
    }
    pub fn set_bpm(&mut self, bpm: ParameterType) {
        self.bpm = bpm;
        self.was_reset = true;
        self.update();
    }

    pub fn seek(&mut self, ticks: usize) {
        self.frames = ticks;
        self.was_reset = true;
        self.update();
    }
    pub fn set_sample_rate(&mut self, sample_rate: usize) {
        self.sample_rate = sample_rate;
        self.was_reset = true;
        self.update();
    }

    /// The next_slice_in_ methods return the start of the next time slice, in
    /// whatever unit is requested. The usage is to accurately identify the
    /// range of times that a given time slice includes, rather than just doing
    /// a <= comparison on each tick().
    #[allow(dead_code)]
    #[deprecated]
    pub fn next_slice_in_frames(&self) -> usize {
        self.frames + 1
    }
    #[allow(dead_code)]
    #[deprecated]
    pub fn next_slice_in_seconds(&self) -> f64 {
        self.seconds_for_frame(self.frames + 1)
    }
    #[deprecated]
    pub fn next_slice_in_beats(&self) -> f64 {
        self.beats_for_frame(self.frames + 1)
    }

    pub fn tick_batch(&mut self, count: usize) {
        self.was_reset = false;
        self.frames += count;
        self.update();
    }

    /// Given a frame number, returns the number of seconds that have elapsed.
    fn seconds_for_frame(&self, frame: usize) -> f64 {
        frame as f64 / self.sample_rate as f64
    }
    /// Given a frame number, returns the number of beats that have elapsed.
    fn beats_for_frame(&self, frame: usize) -> f64 {
        (self.bpm / 60.0) * self.seconds_for_frame(frame)
    }
    /// Given a frame number, returns the number of MIDI ticks that have
    /// elapsed.
    fn midi_ticks_for_frame(&self, frame: usize) -> usize {
        (self.midi_ticks_per_second as f64 * self.seconds_for_frame(frame)) as usize
    }

    fn update(&mut self) {
        self.seconds = self.seconds_for_frame(self.frames);
        self.beats = self.beats_for_frame(self.frames);
        self.midi_ticks = self.midi_ticks_for_frame(self.frames);
    }

    pub fn time_for(&self, unit: &ClockTimeUnit) -> f64 {
        match unit {
            ClockTimeUnit::Seconds => self.seconds(),
            ClockTimeUnit::Beats => self.beats(),
            ClockTimeUnit::Samples => todo!(),
            ClockTimeUnit::MidiTicks => todo!(),
        }
    }
}
impl Ticks for Clock {
    fn tick(&mut self, tick_count: usize) {
        if self.was_reset {
            // On a reset, we keep our tick counter at zero. This is so that a
            // loop can tick() us at the beginning, See
            // https://github.com/sowbug/groove/issues/84 for discussion.
            self.was_reset = false;
        } else if tick_count != 0 {
            self.frames += tick_count;
            self.update();
        }
    }
}
impl Resets for Clock {
    fn reset(&mut self, sample_rate: usize) {
        self.set_sample_rate(sample_rate);
        self.was_reset = true;
        self.frames = 0;
        self.seconds = 0.0;
        self.beats = 0.0;
        self.midi_ticks = 0;
    }
}

/// This is named facetiously. f32 has problems the way I'm using it. I'd like
/// to replace with something better later on, but for now I'm going to try to
/// use the struct to get type safety and make refactoring easier later on when
/// I replace f32 with something else.
///
/// TODO: look into MMA's time representation that uses a 32-bit integer with
/// some math that stretches it out usefully.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PerfectTimeUnit(pub f64);

impl Display for PerfectTimeUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
impl From<f32> for PerfectTimeUnit {
    fn from(value: f32) -> Self {
        PerfectTimeUnit(value as f64)
    }
}
impl From<usize> for PerfectTimeUnit {
    fn from(value: usize) -> Self {
        PerfectTimeUnit(value as f64)
    }
}
impl Add for PerfectTimeUnit {
    type Output = PerfectTimeUnit;
    fn add(self, rhs: Self) -> Self::Output {
        PerfectTimeUnit(self.0 + rhs.0)
    }
}
impl Mul for PerfectTimeUnit {
    type Output = PerfectTimeUnit;
    fn mul(self, rhs: Self) -> Self::Output {
        PerfectTimeUnit(self.0 * rhs.0)
    }
}
impl PartialOrd for PerfectTimeUnit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}
impl Ord for PerfectTimeUnit {
    fn cmp(&self, other: &Self) -> Ordering {
        if self > other {
            return Ordering::Greater;
        }
        if self < other {
            return Ordering::Less;
        }
        Ordering::Equal
    }
}
impl Eq for PerfectTimeUnit {}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MidiTicks(pub usize);

#[allow(dead_code)]
impl MidiTicks {
    pub const MAX: MidiTicks = MidiTicks(usize::MAX);
    pub const MIN: MidiTicks = MidiTicks(usize::MIN);
}

impl Display for MidiTicks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
impl From<f64> for MidiTicks {
    fn from(value: f64) -> Self {
        MidiTicks(value as usize)
    }
}
impl Add for MidiTicks {
    type Output = MidiTicks;
    fn add(self, rhs: Self) -> Self::Output {
        MidiTicks(self.0 + rhs.0)
    }
}
impl Mul for MidiTicks {
    type Output = MidiTicks;
    fn mul(self, rhs: Self) -> Self::Output {
        MidiTicks(self.0 * rhs.0)
    }
}
impl PartialOrd for MidiTicks {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}
impl Ord for MidiTicks {
    fn cmp(&self, other: &Self) -> Ordering {
        if self > other {
            return Ordering::Greater;
        }
        if self < other {
            return Ordering::Less;
        }
        Ordering::Equal
    }
}
impl Eq for MidiTicks {}

#[cfg(test)]
mod tests {
    use super::*;
    use more_asserts::assert_lt;

    const DEFAULT_BPM: ParameterType = 128.0;
    const DEFAULT_MIDI_TICKS_PER_SECOND: usize = 960;
    const DEFAULT_SAMPLE_RATE: usize = 44100;

    impl Clock {
        pub fn new_test() -> Self {
            Clock::new_with(
                DEFAULT_SAMPLE_RATE,
                DEFAULT_BPM,
                DEFAULT_MIDI_TICKS_PER_SECOND,
            )
        }

        pub fn new_with_sample_rate(sample_rate: usize) -> Self {
            Self::new_with(sample_rate, DEFAULT_BPM, DEFAULT_MIDI_TICKS_PER_SECOND)
        }

        // pub fn debug_new_with_time(time: f32) -> Self {
        //     let mut r = Self::new();
        //     r.debug_set_seconds(time);
        //     r
        // }

        pub fn debug_set_seconds(&mut self, value: f32) {
            self.was_reset = true;
            self.frames = (self.sample_rate() as f32 * value) as usize;
            self.update();
        }

        pub fn debug_set_beats(&mut self, value: f64) {
            self.was_reset = true;
            self.frames = (self.sample_rate() as f64 * (60.0 * value / self.bpm)) as usize;
            self.update();
        }
    }

    #[test]
    fn clock_mainline_works() {
        const SAMPLE_RATE: usize = 256;
        const BPM: ParameterType = 128.0;
        const QUARTER_NOTE_OF_TICKS: usize = ((SAMPLE_RATE * 60) as f64 / BPM) as usize;
        const SECONDS_PER_BEAT: f64 = 60.0 / BPM;
        const ONE_SAMPLE_OF_SECONDS: f64 = 1.0 / SAMPLE_RATE as f64;

        // Initial state. The Ticks trait specifies that state is valid for the
        // frame *after* calling tick(), so here we verify that after calling
        // tick() the first time, the tick counter remains unchanged.
        let mut clock = Clock::new_with(SAMPLE_RATE, BPM, DEFAULT_MIDI_TICKS_PER_SECOND);
        clock.tick(1);
        assert_eq!(
            clock.frames(),
            0,
            "After creation and then tick(), tick counter should remain at zero."
        );
        assert_eq!(clock.seconds, 0.0);
        assert_eq!(clock.beats(), 0.0);

        // Same but after reset.
        clock.reset(SAMPLE_RATE);
        clock.tick(1);
        assert_eq!(
            clock.frames(),
            0,
            "After reset() and then tick(), tick counter should remain at zero."
        );

        // Check after one tick.
        clock.tick(1);
        assert_eq!(clock.frames(), 1);
        assert_eq!(clock.seconds, ONE_SAMPLE_OF_SECONDS);
        assert_eq!(clock.beats(), (BPM / 60.0) * ONE_SAMPLE_OF_SECONDS);

        // Check around a full quarter note of ticks. minus one because we
        // already did one tick(), then minus another to test edge
        clock.tick(QUARTER_NOTE_OF_TICKS - 1 - 1);
        assert_eq!(clock.frames(), QUARTER_NOTE_OF_TICKS - 1);
        assert!(clock.seconds < SECONDS_PER_BEAT);
        assert_lt!(clock.beats(), 1.0);

        // Now right on the quarter note.
        clock.tick(1);
        assert_eq!(clock.frames(), QUARTER_NOTE_OF_TICKS);
        assert_eq!(clock.seconds, SECONDS_PER_BEAT);
        assert_eq!(clock.beats(), 1.0);

        // One full minute.
        clock.tick(QUARTER_NOTE_OF_TICKS * (BPM - 1.0) as usize);
        assert_eq!(clock.frames(), SAMPLE_RATE * 60);
        assert_eq!(clock.seconds, 60.0);
        assert_eq!(clock.beats(), BPM);
    }

    #[test]
    fn test_clock_tells_us_when_it_jumps() {
        let mut clock = Clock::new_with(
            DEFAULT_SAMPLE_RATE,
            DEFAULT_BPM,
            DEFAULT_MIDI_TICKS_PER_SECOND,
        );

        let mut next_sample = clock.frames();
        let mut first_time = true;

        for _ in 0..10 {
            clock.tick(1);
            assert_eq!(clock.frames(), next_sample);

            // The first time through, the clock really is reset, because it had
            // no prior tick.
            assert!(first_time || !clock.was_reset());

            first_time = false;
            next_sample = clock.next_slice_in_frames();
        }
        clock.seek(clock.frames() + 1);
        assert!(clock.was_reset());
        assert_eq!(clock.frames(), next_sample);
        clock.tick(1);
        assert!(!clock.was_reset());
    }
}
