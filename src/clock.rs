use std::{
    cmp::Ordering,
    fmt::Display,
    ops::{Add, Mul},
};

use crate::settings::ClockSettings;
use anyhow::{anyhow, Error};
use serde::{Deserialize, Serialize};
use strum_macros::FromRepr;

#[derive(Clone, Debug, Default, Deserialize, FromRepr, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BeatValue {
    Octuple = 128,   // large/maxima
    Quadruple = 256, // long
    Double = 512,    // breve
    Whole = 1024,    // semibreve
    Half = 2048,     // minim
    #[default]
    Quarter = 4096, // crotchet
    Eighth = 8192,   // quaver
    Sixteenth = 16384, // semiquaver
    ThirtySecond = 32768, // demisemiquaver
    SixtyFourth = 65536, // hemidemisemiquaver
    OneHundredTwentyEighth = 131072, // semihemidemisemiquaver / quasihemidemisemiquaver
    TwoHundredFiftySixth = 262144, // demisemihemidemisemiquaver
    FiveHundredTwelfth = 524288, // winner winner chicken dinner
}

impl BeatValue {
    pub fn divisor(value: BeatValue) -> f32 {
        value as u32 as f32 / 1024.0
    }

    pub fn from_divisor(divisor: f32) -> anyhow::Result<Self, anyhow::Error> {
        if let Some(value) = BeatValue::from_repr((divisor * 1024.0) as usize) {
            Ok(value)
        } else {
            Err(anyhow!("divisor {} is out of range", divisor))
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TimeSignature {
    // The top number of a time signature tells how many beats are in a
    // measure. The bottom number tells the value of a beat. For example,
    // if the bottom number is 4, then a beat is a quarter-note. And if
    // the top number is 4, then you should expect to see four beats in a
    // measure, or four quarter-notes in a measure.
    //
    // If your song is playing at 60 beats per minute, and it's 4/4,
    // then a measure's worth of the song should complete in four seconds.
    // That's because each beat takes a second (60 beats/minute,
    // 60 seconds/minute -> 60/60 beats/second = 60/60 seconds/beat),
    // and a measure takes four beats (4 beats/measure * 1 second/beat
    // = 4/1 seconds/measure).
    //
    // If your song is playing at 120 beats per minute, and it's 4/4,
    // then a measure's worth of the song should complete in two seconds.
    // That's because each beat takes a half-second (120 beats/minute,
    // 60 seconds/minute -> 120/60 beats/second = 60/120 seconds/beat),
    // and a measure takes four beats (4 beats/measure * 1/2 seconds/beat
    // = 4/2 seconds/measure).
    //
    // The relevance in this project is...
    //
    // - BPM tells how fast a beat should last in time
    // - bottom number tells what the default denomination is of a slot
    // in a pattern
    // - top number tells how many slots should be in a pattern. But
    //   we might not want to enforce this, as it seems redundant... if
    //   you want a 5/4 pattern, it seems like you can just go ahead and
    //   include 5 slots in it. The only relevance seems to be whether
    //   we'd round a 5-slot pattern in a 4/4 song to the next even measure,
    //   or just tack the next pattern directly onto the sixth beat.
    pub top: usize,
    pub bottom: usize,
}

impl TimeSignature {
    pub fn new_with(top: usize, bottom: usize) -> anyhow::Result<Self, Error> {
        if top == 0 {
            Err(anyhow!("Time signature top can't be zero."))
        } else {
            if let Ok(_) = BeatValue::from_divisor(bottom as f32) {
                Ok(Self { top, bottom })
            } else {
                Err(anyhow!("Time signature bottom was out of range."))
            }
        }
    }

    pub fn beat_value(&self) -> BeatValue {
        // It's safe to unwrap because the constructor already blew up if the
        // bottom were out of range.
        BeatValue::from_divisor(self.bottom as f32).unwrap()
    }
}

impl Default for TimeSignature {
    fn default() -> Self {
        Self { top: 4, bottom: 4 }
    }
}

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

#[derive(Clone, Debug, Default)]
pub struct Clock {
    settings: ClockSettings,

    samples: usize, // Samples since clock creation.
    seconds: f32,   // Seconds elapsed since clock creation.

    // Beats elapsed since clock creation. Not
    // https://en.wikipedia.org/wiki/Swatch_Internet_Time
    beats: f32,

    // Typically 960 ticks per second
    midi_ticks: usize,

    // True if anything unusual happened since the last tick, or there was no
    // last tick because this is the first.
    was_reset: bool,
}

impl Clock {
    pub fn new() -> Self {
        Self {
            was_reset: true,
            ..Default::default()
        }
    }

    pub fn new_with(settings: &ClockSettings) -> Self {
        Self {
            settings: settings.clone(),
            was_reset: true,
            ..Default::default()
        }
    }

    pub fn settings(&self) -> &ClockSettings {
        &self.settings
    }

    pub fn was_reset(&self) -> bool {
        self.was_reset
    }

    pub fn samples(&self) -> usize {
        self.samples
    }
    pub fn seconds(&self) -> f32 {
        self.seconds
    }
    pub fn beats(&self) -> f32 {
        self.beats
    }
    pub fn midi_ticks(&self) -> usize {
        self.midi_ticks
    }
    pub fn sample_rate(&self) -> usize {
        self.settings().sample_rate()
    }
    pub fn bpm(&self) -> f32 {
        self.settings().bpm()
    }
    pub fn set_bpm(&mut self, bpm: f32) {
        self.was_reset = true;
        self.settings.set_bpm(bpm);
        self.update();
    }

    pub fn set_samples(&mut self, value: usize) {
        self.was_reset = true;
        self.samples = value;
        self.update();
    }
    pub fn set_time_signature(&mut self, time_signature: TimeSignature) {
        self.was_reset = true;
        self.settings.set_time_signature(time_signature);
        self.update();
    }

    /// The next_slice_in_ methods return the start of the next time slice, in
    /// whatever unit is requested. The usage is to accurately identify the
    /// range of times that a given time slice includes, rather than just doing
    /// a <= comparison on each tick().
    #[allow(dead_code)]
    pub(crate) fn next_slice_in_samples(&self) -> usize {
        self.samples + 1
    }
    #[allow(dead_code)]
    pub(crate) fn next_slice_in_seconds(&self) -> f32 {
        self.seconds_for_sample(self.samples + 1)
    }
    pub(crate) fn next_slice_in_beats(&self) -> f32 {
        self.beats_for_sample(self.samples + 1)
    }
    pub(crate) fn next_slice_in_midi_ticks(&self) -> usize {
        // Because MIDI ticks (960Hz) are larger than samples (44100Hz), many of
        // the ranges computed in MidiTickSequencer::tick() are empty. A range
        // is nonzero only when the division works out so that the start is
        // barely on the left, and the end barely on the right. This means that
        // something scheduled for MIDI tick zero will actually happen around
        // sample 46 (44100/960 = 45.9375), so MIDI time is about a millisecond
        // behind where it should be.
        //
        // TODO: Come up with a better conversion method that aligns integer
        // MIDI ticks with the first range that could include them, but that
        // doesn't then schedule the tick more than once.
        self.midi_ticks_for_sample(self.samples + 1)
    }

    pub fn tick(&mut self) {
        self.was_reset = false;
        self.samples += 1;
        self.update();
    }

    fn seconds_for_sample(&self, sample: usize) -> f32 {
        sample as f32 / self.settings.sample_rate() as f32
    }
    fn beats_for_sample(&self, sample: usize) -> f32 {
        (self.settings.bpm() / 60.0) * self.seconds_for_sample(sample)
    }
    fn midi_ticks_for_sample(&self, sample: usize) -> usize {
        (self.settings.midi_ticks_per_second() as f32 * self.seconds_for_sample(sample)) as usize
    }

    fn update(&mut self) {
        self.seconds = self.seconds_for_sample(self.samples);
        self.beats = self.beats_for_sample(self.samples);
        self.midi_ticks = self.midi_ticks_for_sample(self.samples);
    }

    pub fn reset(&mut self) {
        self.was_reset = true;
        self.samples = 0;
        self.seconds = 0.0;
        self.beats = 0.0;
        self.midi_ticks = 0;
    }

    pub(crate) fn time_for(&self, unit: &ClockTimeUnit) -> f32 {
        match unit {
            ClockTimeUnit::Seconds => self.seconds(),
            ClockTimeUnit::Beats => self.beats(),
            ClockTimeUnit::Samples => todo!(),
            ClockTimeUnit::MidiTicks => todo!(),
        }
    }
}

/// This is named facetiously. f32 has problems the way I'm using it. I'd like
/// to replace with something better later on, but for now I'm going to try to
/// use the struct to get type safety and make refactoring easier later on when
/// I replace f32 with something else.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PerfectTimeUnit(pub f32);

impl Display for PerfectTimeUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
impl From<f32> for PerfectTimeUnit {
    fn from(value: f32) -> Self {
        PerfectTimeUnit(value)
    }
}
impl From<usize> for PerfectTimeUnit {
    fn from(value: usize) -> Self {
        PerfectTimeUnit(value as f32)
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
pub(crate) struct MidiTicks(pub usize);

impl MidiTicks {
    #[allow(dead_code)]
    pub(crate) const MAX: MidiTicks = MidiTicks(usize::MAX);
    pub(crate) const MIN: MidiTicks = MidiTicks(usize::MIN);
}

impl Display for MidiTicks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
impl From<f32> for MidiTicks {
    fn from(value: f32) -> Self {
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
    use more_asserts::assert_lt;

    use super::*;

    impl Clock {
        pub fn new_test() -> Self {
            Self::new_with(&ClockSettings::new_test())
        }

        pub fn new_with_sample_rate(sample_rate: usize) -> Self {
            let cs = ClockSettings::default();
            Self::new_with(&ClockSettings::new(
                sample_rate,
                cs.bpm(),
                (cs.time_signature().top, cs.time_signature().bottom),
            ))
        }

        pub fn debug_new_with_time(time: f32) -> Self {
            let mut r = Self::new();
            r.debug_set_seconds(time);
            r
        }

        pub fn debug_set_seconds(&mut self, value: f32) {
            self.was_reset = true;
            self.samples = (self.sample_rate() as f32 * value) as usize;
            self.update();
        }

        pub fn debug_set_beats(&mut self, value: f32) {
            self.was_reset = true;
            self.samples =
                (self.sample_rate() as f32 * (60.0 * value / self.settings().bpm())) as usize;
            self.update();
        }
    }

    #[test]
    fn test_clock_mainline() {
        const SAMPLE_RATE: usize = 256;
        const BPM: f32 = 128.0;
        const QUARTER_NOTE_OF_TICKS: usize = ((SAMPLE_RATE * 60) as f32 / BPM) as usize;
        const SECONDS_PER_BEAT: f32 = 60.0 / BPM;
        const ONE_SAMPLE_OF_SECONDS: f32 = 1.0 / SAMPLE_RATE as f32;

        let clock_settings = ClockSettings::new(SAMPLE_RATE, BPM, (4, 4));
        let mut clock = Clock::new_with(&clock_settings);

        // init state
        assert_eq!(clock.samples(), 0);
        assert_eq!(clock.seconds, 0.0);
        assert_eq!(clock.beats(), 0.0);

        // Check after one tick.
        clock.tick();
        assert_eq!(clock.samples(), 1);
        assert_eq!(clock.seconds, ONE_SAMPLE_OF_SECONDS);
        assert_eq!(clock.beats(), (BPM / 60.0) * ONE_SAMPLE_OF_SECONDS);

        // Check around a full quarter note of ticks.
        // minus one because we already did one tick(), then minus another to test edge
        for _ in 0..QUARTER_NOTE_OF_TICKS - 1 - 1 {
            clock.tick();
        }
        assert_eq!(clock.samples(), QUARTER_NOTE_OF_TICKS - 1);
        assert!(clock.seconds < SECONDS_PER_BEAT);
        assert_lt!(clock.beats(), 1.0);

        // Now right on the quarter note.
        clock.tick();
        assert_eq!(clock.samples(), QUARTER_NOTE_OF_TICKS);
        assert_eq!(clock.seconds, SECONDS_PER_BEAT);
        assert_eq!(clock.beats(), 1.0);

        // One full minute.
        for _ in 0..QUARTER_NOTE_OF_TICKS * (BPM - 1.0) as usize {
            clock.tick();
        }
        assert_eq!(clock.samples(), SAMPLE_RATE * 60);
        assert_eq!(clock.seconds, 60.0);
        assert_eq!(clock.beats(), BPM);
    }

    #[test]
    fn test_time_signature_valid() {
        let ts = TimeSignature::default();
        assert_eq!(ts.top, 4);
        assert_eq!(ts.bottom, 4);

        let ts = TimeSignature::new_with(ts.top, ts.bottom).ok().unwrap();
        assert!(matches!(ts.beat_value(), BeatValue::Quarter));
    }

    #[test]
    fn test_time_signature_invalid_bad_top() {
        assert!(TimeSignature::new_with(0, 4).is_err());
    }

    #[test]
    fn test_time_signature_invalid_bottom_not_power_of_two() {
        assert!(TimeSignature::new_with(4, 5).is_err());
    }

    #[test]
    fn test_time_signature_invalid_bottom_below_range() {
        assert!(TimeSignature::new_with(4, 0).is_err());
    }

    #[test]
    fn test_time_signature_invalid_bottom_above_range() {
        // 2^10 = 1024, 1024 * 1024 = 1048576, which is higher than
        // BeatValue::FiveHundredTwelfth value of 524288
        let bv = BeatValue::from_divisor(2.0f32.powi(10));
        assert!(bv.is_err());
    }

    #[test]
    fn test_clock_tells_us_when_it_jumps() {
        let mut clock = Clock::new();

        let mut next_sample = clock.samples();
        let mut first_time = true;

        for _ in 0..10 {
            assert_eq!(clock.samples(), next_sample);

            // The first time through, the clock really is reset, because it had no
            // prior tick.
            assert!(first_time || !clock.was_reset());

            first_time = false;
            next_sample = clock.next_slice_in_samples();
            clock.tick();
        }
        clock.set_samples(clock.samples() + 1);
        assert!(clock.was_reset());
        assert_eq!(clock.samples(), next_sample + 1);
        clock.tick();
        assert!(!clock.was_reset());
    }
}
