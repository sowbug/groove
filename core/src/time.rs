// Copyright (c) 2023 Mike Tsao. All rights reserved.

use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    fmt::Display,
    ops::{Add, Mul},
};

// A way to specify a time unit that Clock tracks.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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

#[cfg(obsolete)]
mod obsolete {
    /// A timekeeper that operates in terms of sample rate.
    #[derive(Debug, Control, Uid, Serialize, Deserialize)]
    pub struct Clock {
        #[control]
        bpm: ParameterType,

        #[control]
        midi_ticks_per_second: usize,

        #[control]
        time_signature: TimeSignature,

        /// The number of frames per second. Usually 44.1KHz for CD-quality audio.
        #[serde(skip)]
        sample_rate: ensnare::time::SampleRate,

        /// Samples since clock creation. It's called "frames" because tick() was
        /// already being used as a verb by the Ticks trait, and "samples" is a very
        /// overloaded term in digital audio. A synonymous term is "time slices,"
        /// used when the emphasis is on division of work into small parts.
        #[serde(skip)]
        frames: usize,

        /// Seconds elapsed since clock creation. Derived from sample rate and
        /// elapsed frames.
        #[serde(skip)]
        seconds: ParameterType,

        /// The number of measures that have elapsed according to the time
        /// signature. This is always an integer number, unlike beats, which can be
        /// fractional.
        ///
        /// TODO: is it actually useful for beats to be a float? Check and see
        /// whether the fractional use cases were actually using seconds.
        #[serde(skip)]
        measures: usize,

        /// Beats elapsed since clock creation. Derived from seconds and BPM.
        #[serde(skip)]
        beats: ParameterType,

        /// MIDI ticks since clock creation. Derived from seconds and
        /// midi_ticks_per_second. Typically 960 ticks per second
        #[serde(skip)]
        midi_ticks: usize,

        // True if anything unusual happened since the last tick, or there was no
        // last tick because this is the first.
        #[serde(skip)]
        was_reset: bool,

        #[serde(skip)]
        uid: Uid,
    }
    impl Default for Clock {
        fn default() -> Self {
            Self {
                bpm: 120.0,
                midi_ticks_per_second: 960,
                time_signature: TimeSignature::default(),
                sample_rate: Default::default(),
                frames: Default::default(),
                seconds: Default::default(),
                measures: Default::default(),
                beats: Default::default(),
                midi_ticks: Default::default(),
                was_reset: true,
                uid: Default::default(),
            }
        }
    }
    impl Clock {
        pub fn new_with(
            bpm: ParameterType,
            midi_ticks_per_second: usize,
            time_signature: TimeSignature,
        ) -> Self {
            Self {
                sample_rate: Default::default(),
                bpm,
                midi_ticks_per_second,
                time_signature,
                frames: Default::default(),
                seconds: Default::default(),
                beats: Default::default(),
                measures: Default::default(),
                midi_ticks: Default::default(),
                was_reset: true,
                uid: Default::default(),
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
        pub fn measures(&self) -> usize {
            self.measures
        }
        pub fn beats(&self) -> f64 {
            self.beats
        }
        pub fn midi_ticks(&self) -> usize {
            self.midi_ticks
        }
        pub fn bpm(&self) -> ParameterType {
            self.bpm
        }
        pub fn set_bpm(&mut self, bpm: ParameterType) {
            self.bpm = bpm;
            self.was_reset = true;
            self.update_internals();
        }

        pub fn seek(&mut self, ticks: usize) {
            self.frames = ticks;
            self.was_reset = true;
            self.update_internals();
        }
        pub fn seek_beats(&mut self, value: f64) {
            self.seek((f64::from(self.sample_rate) * (60.0 * value / self.bpm)) as usize);
        }

        /// The next_slice_in_ methods return the start of the next time slice, in
        /// whatever unit is requested. The usage is to accurately identify the
        /// range of times that a given time slice includes, rather than just doing
        /// a <= comparison on each tick().
        pub fn next_slice_in_frames(&self) -> usize {
            self.frames + 1
        }
        pub fn next_slice_in_seconds(&self) -> f64 {
            self.seconds_for_frame(self.frames + 1)
        }
        pub fn next_slice_in_beats(&self) -> f64 {
            self.beats_for_frame(self.frames + 1)
        }

        pub fn tick_batch(&mut self, count: usize) {
            self.was_reset = false;
            self.frames += count;
            self.update_internals();
        }

        /// Given a frame number, returns the number of seconds that have elapsed.
        fn seconds_for_frame(&self, frame: usize) -> f64 {
            frame as f64 / f64::from(self.sample_rate)
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

        fn update_internals(&mut self) {
            self.seconds = self.seconds_for_frame(self.frames);
            self.beats = self.beats_for_frame(self.frames);
            self.measures = self.beats as usize / self.time_signature.top;
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

        pub fn midi_ticks_per_second(&self) -> usize {
            self.midi_ticks_per_second
        }

        pub fn set_midi_ticks_per_second(&mut self, midi_ticks_per_second: usize) {
            self.midi_ticks_per_second = midi_ticks_per_second;
        }

        pub fn time_signature(&self) -> &TimeSignature {
            &self.time_signature
        }

        pub fn set_time_signature(&mut self, time_signature: TimeSignature) {
            self.time_signature = time_signature;
        }
    }
    impl Ticks for Clock {
        fn tick(&mut self, tick_count: usize) {
            // TODO: I think this logic is wrong. If the caller asks for more than
            // one tick after reset, then we swallow them without processing.
            if self.was_reset {
                // On a reset, we keep our tick counter at zero. This is so that a
                // loop can tick() us at the beginning, See
                // https://github.com/sowbug/groove/issues/84 for discussion.
                self.was_reset = false;
            } else if tick_count != 0 {
                self.frames += tick_count;
                self.update_internals();
            }
        }
    }
    impl Configurable for Clock {
        fn sample_rate(&self) -> ensnare::time::SampleRate {
            self.sample_rate
        }

        fn update_sample_rate(&mut self, sample_rate: ensnare::time::SampleRate) {
            self.sample_rate = sample_rate;
            self.was_reset = true;
            self.update_internals();

            //  these used to be part of reset() -- are they still important?
            // self.frames = 0;
            // self.seconds = 0.0;
            // self.beats = 0.0;
            // self.midi_ticks = 0;
        }
    }
}

/// This is named facetiously. f32 has problems the way I'm using it. I'd like
/// to replace with something better later on, but for now I'm going to try to
/// use the struct to get type safety and make refactoring easier later on when
/// I replace f32 with something else.
///
/// TODO: look into MMA's time representation that uses a 32-bit integer with
/// some math that stretches it out usefully.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Seconds(pub f64);
impl Seconds {
    pub fn zero() -> Seconds {
        Seconds(0.0)
    }

    pub fn infinite() -> Seconds {
        Seconds(-1.0)
    }
}
impl From<f64> for Seconds {
    fn from(value: f64) -> Self {
        Self(value)
    }
}
impl From<f32> for Seconds {
    fn from(value: f32) -> Self {
        Self(value as f64)
    }
}
impl Add<f64> for Seconds {
    type Output = Seconds;

    fn add(self, rhs: f64) -> Self::Output {
        Seconds(self.0 + rhs)
    }
}
impl Add<Seconds> for Seconds {
    type Output = Seconds;

    fn add(self, rhs: Seconds) -> Self::Output {
        Seconds(self.0 + rhs.0)
    }
}

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
    use ensnare::prelude::*;

    #[cfg(obsolete)]
    mod obsolete {
        const DEFAULT_BPM: ParameterType = 128.0;
        const DEFAULT_MIDI_TICKS_PER_SECOND: usize = 960;
    }

    #[cfg(obsolete)]
    mod obsolete {
        impl Clock {
            pub fn new_test() -> Self {
                Clock::new_with(
                    DEFAULT_BPM,
                    DEFAULT_MIDI_TICKS_PER_SECOND,
                    TimeSignature::default(),
                )
            }

            pub fn debug_set_seconds(&mut self, value: f64) {
                self.was_reset = true;
                self.frames = (f64::from(self.sample_rate) * value) as usize;
                self.update_internals();
            }
        }
    }
    #[test]
    fn tempo() {
        let t = Tempo::default();
        assert_eq!(t.value(), 128.0);
    }

    #[test]
    fn sample_rate_default_is_sane() {
        let sr = SampleRate::default();
        assert_eq!(sr.value(), 44100);
    }
    #[cfg(obsolete)]
    mod obsolete {
        #[test]
        fn clock_mainline_works() {
            const SAMPLE_RATE: SampleRate = SampleRate::new(256);
            const BPM: ParameterType = 128.0;
            const QUARTER_NOTE_OF_TICKS: usize =
                ((SAMPLE_RATE.value() as f64 * 60.0) / BPM) as usize;
            const SECONDS_PER_BEAT: f64 = 60.0 / BPM;
            const ONE_SAMPLE_OF_SECONDS: f64 = 1.0 / SAMPLE_RATE.value() as f64;

            // Initial state. The Ticks trait specifies that state is valid for the
            // frame *after* calling tick(), so here we verify that after calling
            // tick() the first time, the tick counter remains unchanged.
            let mut clock = Clock::new_test();
            clock.tick(1);
            assert_eq!(
                clock.frames(),
                0,
                "After creation and then tick(), tick counter should remain at zero."
            );
            assert_eq!(clock.seconds, 0.0);
            assert_eq!(clock.beats(), 0.0);

            // Same but after reset.
            clock.update_sample_rate(SAMPLE_RATE);
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
            assert_eq!(clock.frames(), SAMPLE_RATE.value() * 60);
            assert_eq!(clock.seconds, 60.0);
            assert_eq!(clock.beats(), BPM);
        }

        #[test]
        fn clock_tells_us_when_it_jumps() {
            let mut clock = Clock::new_test();

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

    #[test]
    fn valid_time_signatures_can_be_instantiated() {
        let ts = TimeSignature::default();
        assert_eq!(ts.top, 4);
        assert_eq!(ts.bottom, 4);

        let ts = TimeSignature::new_with(ts.top, ts.bottom).ok().unwrap();
        assert!(matches!(ts.beat_value(), BeatValue::Quarter));
    }

    #[test]
    fn time_signature_with_bad_top_is_invalid() {
        assert!(TimeSignature::new_with(0, 4).is_err());
    }

    #[test]
    fn time_signature_with_bottom_not_power_of_two_is_invalid() {
        assert!(TimeSignature::new_with(4, 5).is_err());
    }

    #[test]
    fn time_signature_invalid_bottom_below_range() {
        assert!(TimeSignature::new_with(4, 0).is_err());
    }

    #[test]
    fn time_signature_invalid_bottom_above_range() {
        // 2^10 = 1024, 1024 * 1024 = 1048576, which is higher than
        // BeatValue::FiveHundredTwelfth value of 524288
        let bv = BeatValue::from_divisor(2.0f32.powi(10));
        assert!(bv.is_err());
    }

    #[test]
    fn musical_time_at_time_zero() {
        // Default is time zero
        let t = MusicalTime::default();
        assert_eq!(t.total_bars(&TimeSignature::default()), 0);
        assert_eq!(t.total_beats(), 0);
        assert_eq!(t.parts(), 0);
        assert_eq!(t.units(), 0);
    }

    #[test]
    fn musical_time_to_frame_conversions() {
        let ts = TimeSignature::default();
        let tempo = Tempo::default();
        let sample_rate = SampleRate::default();

        // These are here to catch any change in defaults that would invalidate lots of tests.
        assert_eq!(ts.top, 4);
        assert_eq!(ts.bottom, 4);
        assert_eq!(tempo.0, 128.0);
        assert_eq!(<SampleRate as Into<usize>>::into(sample_rate), 44100);

        const ONE_4_4_BAR_IN_SECONDS: f64 = 60.0 * 4.0 / 128.0;
        const ONE_BEAT_IN_SECONDS: f64 = 60.0 / 128.0;
        const ONE_PART_IN_SECONDS: f64 = ONE_BEAT_IN_SECONDS / 16.0;
        const ONE_UNIT_IN_SECONDS: f64 = ONE_BEAT_IN_SECONDS / (16.0 * 4096.0);
        assert_eq!(ONE_4_4_BAR_IN_SECONDS, 1.875);
        assert_eq!(ONE_BEAT_IN_SECONDS, 0.46875);

        for (bars, beats, parts, units, seconds) in [
            (0, 0, 0, 0, 0.0),
            (0, 0, 0, 1, ONE_UNIT_IN_SECONDS),
            (0, 0, 1, 0, ONE_PART_IN_SECONDS),
            (0, 1, 0, 0, ONE_BEAT_IN_SECONDS),
            (1, 0, 0, 0, ONE_4_4_BAR_IN_SECONDS),
            (128 / 4, 0, 0, 0, 60.0),
        ] {
            let sample_rate_f64: f64 = sample_rate.into();
            let frames = (seconds * sample_rate_f64).round() as usize;
            let time = MusicalTime::new(&ts, bars, beats, parts, units);
            assert_eq!(
                time.as_frames(tempo, sample_rate),
                frames,
                "Expected {}.{}.{}.{} -> {} frames",
                bars,
                beats,
                parts,
                units,
                frames,
            );
        }
    }

    #[test]
    fn frame_to_musical_time_conversions() {
        let ts = TimeSignature::default();
        let tempo = Tempo::default();
        let sample_rate = SampleRate::default();

        for (frames, bars, beats, parts, units) in [
            (0, 0, 0, 0, 0),
            (2646000, 32, 0, 0, 0), // one full minute
            (44100, 0, 2, 2, 546),  // one second = 128 bpm / 60 seconds/min =
                                    // 2.13333333 beats, which breaks down to 2
                                    // beats, 2 parts that are each 1/16 of a
                                    // beat = 2.133333 parts (yeah, that happens
                                    // to be the same as the 2.133333 for
                                    // beats), and multiply the .1333333 by 4096
                                    // to get units.
        ] {
            assert_eq!(
                MusicalTime::new(&ts, bars, beats, parts, units).total_units(),
                MusicalTime::frames_to_units(tempo, sample_rate, frames),
                "Expected {} frames -> {}.{}.{}.{}",
                frames,
                bars,
                beats,
                parts,
                units,
            );
        }
    }

    #[test]
    fn musical_time_math() {
        let ts = TimeSignature::default();
        // Advancing by bar works
        let mut t = MusicalTime::default();
        t += MusicalTime::new_with_bars(&ts, 1);
        assert_eq!(t.beats(&ts), 0);
        assert_eq!(t.bars(&ts), 1);

        // Advancing by beat works
        let mut t = MusicalTime::default();
        t += MusicalTime::new_with_beats(1);
        assert_eq!(t.beats(&ts), 1);
        let mut t = MusicalTime::new(&ts, 0, (ts.top - 1) as usize, 0, 0);
        t += MusicalTime::new_with_beats(1);
        assert_eq!(t.beats(&ts), 0);
        assert_eq!(t.bars(&ts), 1);

        // Advancing by part works
        let mut t = MusicalTime::default();
        t += MusicalTime::new_with_parts(1);
        assert_eq!(t.bars(&ts), 0);
        assert_eq!(t.beats(&ts), 0);
        assert_eq!(t.parts(), 1);
        let mut t = MusicalTime::new(&ts, 0, 0, MusicalTime::PARTS_IN_BEAT - 1, 0);
        t += MusicalTime::new_with_parts(1);
        assert_eq!(t.bars(&ts), 0);
        assert_eq!(t.beats(&ts), 1);
        assert_eq!(t.parts(), 0);

        // Advancing by subpart works
        let mut t = MusicalTime::default();
        t += MusicalTime::new_with_units(1);
        assert_eq!(t.bars(&ts), 0);
        assert_eq!(t.beats(&ts), 0);
        assert_eq!(t.parts(), 0);
        assert_eq!(t.units(), 1);
        let mut t = MusicalTime::new(&ts, 0, 0, 0, MusicalTime::UNITS_IN_PART - 1);
        t += MusicalTime::new_with_units(1);
        assert_eq!(t.bars(&ts), 0);
        assert_eq!(t.beats(&ts), 0);
        assert_eq!(t.parts(), 1);
        assert_eq!(t.units(), 0);

        // One more big rollover to be sure
        let mut t = MusicalTime::new(&ts, 0, 3, 15, MusicalTime::UNITS_IN_PART - 1);
        t += MusicalTime::new_with_units(1);
        assert_eq!(t.bars(&ts), 1);
        assert_eq!(t.beats(&ts), 0);
        assert_eq!(t.parts(), 0);
        assert_eq!(t.units(), 0);
    }

    #[test]
    fn musical_time_math_add_trait() {
        let ts = TimeSignature::default();

        let bar_unit = MusicalTime::new(&ts, 1, 0, 0, 0);
        let beat_unit = MusicalTime::new(&ts, 0, 1, 0, 0);
        let part_unit = MusicalTime::new(&ts, 0, 0, 1, 0);
        let unit_unit = MusicalTime::new(&ts, 0, 0, 0, 1);

        // Advancing by bar works
        let t = MusicalTime::default() + bar_unit;
        assert_eq!(t.beats(&ts), 0);
        assert_eq!(t.bars(&ts), 1);

        // Advancing by beat works
        let mut t = MusicalTime::default() + beat_unit;

        assert_eq!(t.beats(&ts), 1);
        t = t + beat_unit;
        assert_eq!(t.beats(&ts), 2);
        assert_eq!(t.bars(&ts), 0);
        t = t + beat_unit;
        assert_eq!(t.beats(&ts), 3);
        assert_eq!(t.bars(&ts), 0);
        t = t + beat_unit;
        assert_eq!(t.beats(&ts), 0);
        assert_eq!(t.bars(&ts), 1);

        // Advancing by part works
        let mut t = MusicalTime::default();
        assert_eq!(t.bars(&ts), 0);
        assert_eq!(t.beats(&ts), 0);
        for i in 0..MusicalTime::PARTS_IN_BEAT {
            assert_eq!(t.parts(), i);
            t = t + part_unit;
        }
        assert_eq!(t.beats(&ts), 1);
        assert_eq!(t.parts(), 0);

        // Advancing by unit works
        let mut t = MusicalTime::default();
        assert_eq!(t.beats(&ts), 0);
        assert_eq!(t.bars(&ts), 0);
        assert_eq!(t.parts(), 0);
        for i in 0..MusicalTime::UNITS_IN_PART {
            assert_eq!(t.units(), i);
            t = t + unit_unit;
        }
        assert_eq!(t.parts(), 1);
        assert_eq!(t.units(), 0);

        // One more big rollover to be sure
        let mut t = MusicalTime::new(
            &ts,
            0,
            3,
            MusicalTime::PARTS_IN_BEAT - 1,
            MusicalTime::UNITS_IN_PART - 1,
        );
        t = t + unit_unit;
        assert_eq!(t.bars(&ts), 1);
        assert_eq!(t.beats(&ts), 0);
        assert_eq!(t.parts(), 0);
        assert_eq!(t.units(), 0);
    }

    #[test]
    fn musical_time_math_other_time_signatures() {
        let ts = TimeSignature { top: 9, bottom: 64 };
        let t = MusicalTime::new(&ts, 0, 8, 15, 4095) + MusicalTime::new(&ts, 0, 0, 0, 1);
        assert_eq!(t.bars(&ts), 1);
        assert_eq!(t.beats(&ts), 0);
        assert_eq!(t.parts(), 0);
        assert_eq!(t.units(), 0);
    }

    #[test]
    fn musical_time_overflow() {
        let ts = TimeSignature::new_with(4, 256).unwrap();

        let time = MusicalTime::new(
            &ts,
            0,
            (ts.top - 1) as usize,
            MusicalTime::PARTS_IN_BEAT - 1,
            MusicalTime::UNITS_IN_PART - 1,
        );

        let t = time.clone() + MusicalTime::new_with_beats(1);
        assert_eq!(t.beats(&ts), 0);
        assert_eq!(t.bars(&ts), 1);

        let t = time.clone() + MusicalTime::new_with_parts(1);
        assert_eq!(t.parts(), 0);
        assert_eq!(t.beats(&ts), 0);
        assert_eq!(t.bars(&ts), 1);

        let t = time.clone() + MusicalTime::new_with_units(1);
        assert_eq!(t.units(), 0);
        assert_eq!(t.parts(), 0);
        assert_eq!(t.beats(&ts), 0);
        assert_eq!(t.bars(&ts), 1);
    }
}
