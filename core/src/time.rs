// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::{
    traits::{Resets, Ticks},
    ParameterType,
};
use anyhow::{anyhow, Error};
use groove_proc_macros::{Control, Params, Uid};
use std::{
    cmp::Ordering,
    fmt::Display,
    num::NonZeroUsize,
    ops::{Add, Mul},
};
use strum_macros::{FromRepr, IntoStaticStr};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

// A way to specify a time unit that Clock tracks.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
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
#[derive(Debug, Control, Params, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Clock {
    #[control]
    #[params]
    bpm: ParameterType,

    #[control]
    #[params]
    midi_ticks_per_second: usize,

    #[control]
    #[params]
    time_signature: TimeSignature,

    /// The number of frames per second. Usually 44.1KHz for CD-quality audio.
    #[cfg_attr(feature = "serialization", serde(skip))]
    sample_rate: usize,

    /// Samples since clock creation. It's called "frames" because tick() was
    /// already being used as a verb by the Ticks trait, and "samples" is a very
    /// overloaded term in digital audio. A synonymous term is "time slices,"
    /// used when the emphasis is on division of work into small parts.
    #[cfg_attr(feature = "serialization", serde(skip))]
    frames: usize,

    /// Seconds elapsed since clock creation. Derived from sample rate and
    /// elapsed frames.
    #[cfg_attr(feature = "serialization", serde(skip))]
    seconds: ParameterType,

    /// The number of measures that have elapsed according to the time
    /// signature. This is always an integer number, unlike beats, which can be
    /// fractional.
    ///
    /// TODO: is it actually useful for beats to be a float? Check and see
    /// whether the fractional use cases were actually using seconds.
    #[cfg_attr(feature = "serialization", serde(skip))]
    measures: usize,

    /// Beats elapsed since clock creation. Derived from seconds and BPM.
    #[cfg_attr(feature = "serialization", serde(skip))]
    beats: ParameterType,

    /// MIDI ticks since clock creation. Derived from seconds and
    /// midi_ticks_per_second. Typically 960 ticks per second
    #[cfg_attr(feature = "serialization", serde(skip))]
    midi_ticks: usize,

    // True if anything unusual happened since the last tick, or there was no
    // last tick because this is the first.
    #[cfg_attr(feature = "serialization", serde(skip))]
    was_reset: bool,

    #[cfg_attr(feature = "serialization", serde(skip))]
    uid: usize,
}

impl Clock {
    pub fn new_with(params: &ClockParams) -> Self {
        Self {
            sample_rate: Default::default(),
            bpm: params.bpm(),
            midi_ticks_per_second: params.midi_ticks_per_second(),
            time_signature: TimeSignature::new(&params.time_signature).unwrap(),
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
    pub fn sample_rate(&self) -> usize {
        self.sample_rate
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
        self.seek((self.sample_rate() as f64 * (60.0 * value / self.bpm)) as usize);
    }

    pub fn set_sample_rate(&mut self, sample_rate: usize) {
        self.sample_rate = sample_rate;
        self.was_reset = true;
        self.update_internals();
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

    #[cfg(feature = "iced-framework")]
    pub fn update(&mut self, message: ClockMessage) {
        match message {
            ClockMessage::Clock(s) => *self = Self::new_with(s),
            _ => self.derived_update(message),
        }
    }

    pub fn uid(&self) -> usize {
        self.uid
    }

    pub fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
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
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
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
pub struct TimeUnit(pub f64);
impl TimeUnit {
    pub fn zero() -> TimeUnit {
        TimeUnit(0.0)
    }

    pub fn infinite() -> TimeUnit {
        TimeUnit(-1.0)
    }
}
impl From<f64> for TimeUnit {
    fn from(value: f64) -> Self {
        Self(value)
    }
}
impl From<f32> for TimeUnit {
    fn from(value: f32) -> Self {
        Self(value as f64)
    }
}
impl Add<f64> for TimeUnit {
    type Output = TimeUnit;

    fn add(self, rhs: f64) -> Self::Output {
        TimeUnit(self.0 + rhs)
    }
}
impl Add<TimeUnit> for TimeUnit {
    type Output = TimeUnit;

    fn add(self, rhs: TimeUnit) -> Self::Output {
        TimeUnit(self.0 + rhs.0)
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

#[derive(Clone, Debug, Default, FromRepr, IntoStaticStr)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
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
    FiveHundredTwelfth = 524288, // winner winner chicken dinner,
}

impl BeatValue {
    pub fn divisor(value: BeatValue) -> f64 {
        value as u32 as f64 / 1024.0
    }

    pub fn from_divisor(divisor: f32) -> anyhow::Result<Self, anyhow::Error> {
        if let Some(value) = BeatValue::from_repr((divisor * 1024.0) as usize) {
            Ok(value)
        } else {
            Err(anyhow!("divisor {} is out of range", divisor))
        }
    }
}

#[derive(Clone, Control, Params, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "serialization",
    derive(Serialize, Deserialize),
    serde(rename = "control-trip", rename_all = "kebab-case")
)]
pub struct TimeSignature {
    // The top number of a time signature tells how many beats are in a measure.
    // The bottom number tells the value of a beat. For example, if the bottom
    // number is 4, then a beat is a quarter-note. And if the top number is 4,
    // then you should expect to see four beats in a measure, or four
    // quarter-notes in a measure.
    //
    // If your song is playing at 60 beats per minute, and it's 4/4, then a
    // measure's worth of the song should complete in four seconds. That's
    // because each beat takes a second (60 beats/minute, 60 seconds/minute ->
    // 60/60 beats/second = 60/60 seconds/beat), and a measure takes four beats
    // (4 beats/measure * 1 second/beat = 4/1 seconds/measure).
    //
    // If your song is playing at 120 beats per minute, and it's 4/4, then a
    // measure's worth of the song should complete in two seconds. That's
    // because each beat takes a half-second (120 beats/minute, 60
    // seconds/minute -> 120/60 beats/second = 60/120 seconds/beat), and a
    // measure takes four beats (4 beats/measure * 1/2 seconds/beat = 4/2
    // seconds/measure).
    //
    // The relevance in this project is...
    //
    // - BPM tells how fast a beat should last in time
    // - bottom number tells what the default denomination is of a slot in a
    // pattern
    // - top number tells how many slots should be in a pattern. But we might
    //   not want to enforce this, as it seems redundant... if you want a 5/4
    //   pattern, it seems like you can just go ahead and include 5 slots in it.
    //   The only relevance seems to be whether we'd round a 5-slot pattern in a
    //   4/4 song to the next even measure, or just tack the next pattern
    //   directly onto the sixth beat.
    #[control]
    #[params]
    pub top: usize,

    #[control]
    #[params]
    pub bottom: usize,
}
impl TimeSignature {
    pub fn new(params: &TimeSignatureParams) -> anyhow::Result<Self, Error> {
        Self::new_with(params.top, params.bottom)
    }

    pub fn new_with(top: usize, bottom: usize) -> anyhow::Result<Self, Error> {
        if top == 0 {
            Err(anyhow!("Time signature top can't be zero."))
        } else if BeatValue::from_divisor(bottom as f32).is_ok() {
            Ok(Self { top, bottom })
        } else {
            Err(anyhow!("Time signature bottom was out of range."))
        }
    }

    pub fn beat_value(&self) -> BeatValue {
        // It's safe to unwrap because the constructor already blew up if the
        // bottom were out of range.
        BeatValue::from_divisor(self.bottom as f32).unwrap()
    }

    pub fn set_top(&mut self, top: usize) {
        self.top = top;
    }

    pub fn set_bottom(&mut self, bottom: usize) {
        self.bottom = bottom;
    }

    pub fn top(&self) -> usize {
        self.top
    }

    pub fn bottom(&self) -> usize {
        self.bottom
    }
}
impl Default for TimeSignature {
    fn default() -> Self {
        Self { top: 4, bottom: 4 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Params)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct MusicalTime {
    /// The number of bars, or measures. Zero-indexed, so Bar #0 is the first.
    #[params]
    bars: usize,

    /// The number of beats within the current bar. The value of a bar's worth
    /// of beats is adjustable, but it's usually the top number of whatever time
    /// signature is applicable.
    ///
    /// Range implied by u8 is 0..256 beats in a single measure.
    #[params]
    beats: u8,

    /// Fractions of a beat. The unit of value is usually a sixteenth-note.
    #[params]
    parts: u8,

    /// 1/100 of a part.
    #[params]
    subparts: u8,

    /// An optional number of beats in a bar. By default, it's 4.
    beats_per_bar: u8,

    /// The number of parts in a beat. By default, it's 16.
    parts_denominator: u16,
}
impl Default for MusicalTime {
    fn default() -> Self {
        Self {
            bars: Default::default(),
            beats: Default::default(),
            parts: Default::default(),
            subparts: Default::default(),
            beats_per_bar: 4,
            parts_denominator: 16,
        }
    }
}
impl MusicalTime {
    pub fn bars(&self) -> usize {
        self.bars
    }

    pub fn set_bars(&mut self, bars: usize) {
        self.bars = bars;
    }

    pub fn beats(&self) -> u8 {
        self.beats
    }

    pub fn set_beats(&mut self, beats: u8) {
        self.beats = beats;
    }

    pub fn parts(&self) -> u8 {
        self.parts
    }

    pub fn set_parts(&mut self, parts: u8) {
        self.parts = parts;
    }

    pub fn subparts(&self) -> u8 {
        self.subparts
    }

    pub fn set_subparts(&mut self, subparts: u8) {
        self.subparts = subparts;
    }

    pub fn reset(&mut self) {
        self.bars = Default::default();
        self.beats = Default::default();
        self.parts = Default::default();
        self.subparts = Default::default();
    }

    pub fn add_bars(&mut self, bars: usize) {
        self.bars += bars;
    }

    pub fn add_beats(&mut self, beats: u8) {
        let new_units: u16 = self.beats as u16 + beats as u16;
        let bpb = self.beats_per_bar as u16;
        let overflow_units = new_units / bpb;
        let actual_units = new_units % bpb;
        if overflow_units != 0 {
            self.add_bars(overflow_units as usize);
        }
        self.beats = actual_units as u8;
    }

    // For now, we're keeping this as a quarter of the beat value, which means
    // that it's always going to range from 0..16. If we ever need more
    // precision than that, then we can add something to TimeSignature or
    // elsewhere that indicates what the custom range should be.
    pub fn add_parts(&mut self, parts: u8) {
        let new_units: u16 = self.parts as u16 + parts as u16;
        let overflow_units = new_units / self.parts_denominator;
        let actual_units = new_units % self.parts_denominator;
        if overflow_units != 0 {
            self.add_beats(overflow_units as u8);
        }
        self.parts = actual_units as u8;
    }

    pub fn add_subparts(&mut self, subparts: u8) {
        const UNIT_RANGE: u16 = 100;

        let new_units: u16 = self.subparts as u16 + subparts as u16;
        let overflow_units = new_units / UNIT_RANGE;
        let actual_units = new_units % UNIT_RANGE;
        if overflow_units != 0 {
            self.add_parts(overflow_units as u8);
        }
        self.subparts = actual_units as u8;
    }

    pub fn new_with(params: &MusicalTimeParams) -> Self {
        Self {
            bars: params.bars,
            beats: params.beats,
            parts: params.parts,
            subparts: params.subparts,
            ..Default::default()
        }
    }

    pub fn new(bars: usize, beats: u8, parts: u8, subparts: u8) -> Self {
        Self::new_with(&MusicalTimeParams {
            bars,
            beats,
            parts,
            subparts: subparts,
        })
    }

    pub fn new_with_beats_per_bar(beats_per_bar: u8) -> Self {
        Self {
            beats_per_bar,
            ..Default::default()
        }
    }

    pub fn new_from_frames(
        ts: &TimeSignature,
        tempo: Tempo,
        sample_rate: SampleRate,
        frames: usize,
    ) -> Self {
        let beats_per_bar = ts.top as f64;
        let total_beats_elapsed = (frames as f64 / sample_rate.0.get() as f64) * tempo.bps();
        let total_bars_elapsed = total_beats_elapsed / beats_per_bar;
        let bars = total_bars_elapsed.floor() as usize;
        let remaining_beats = total_beats_elapsed - total_bars_elapsed.floor() * beats_per_bar;
        let beats = remaining_beats.floor() as u8;
        let remaining_parts = remaining_beats.fract() * 16.0;
        let parts = (remaining_parts.floor()) as u8;
        let subparts = ((remaining_parts - parts as f64) * 100.0 + 0.5) as u8;

        let mut r = Self {
            bars,
            beats,
            parts,
            subparts,
            beats_per_bar: ts.top as u8,
            parts_denominator: 16,
        };

        // This is gross. Some floating-point error accumulates in this block,
        // and it sometimes leaves subparts holding a subpart-sized bag, which
        // means that we end up with an impossible subparts value of 100
        // (outside the 0..100 range). The solution is to carry the one, which
        // is a potentially complicated operation, so we delegate it to our
        // existing addition function.
        if r.subparts == 100 {
            r.subparts = 0;
            r.add_parts(1);
        }
        r
    }

    pub fn as_frames(&self, tempo: Tempo, sample_rate: SampleRate) -> usize {
        let frames_per_second: f64 = sample_rate.into();
        let seconds_per_beat = 1.0 / tempo.bps();
        let frames_per_beat = seconds_per_beat * frames_per_second;

        let bars_in_frames = (self.bars * self.beats_per_bar as usize) as f64 * frames_per_beat;
        let beats_in_frames = self.beats as f64 * frames_per_beat;
        let parts_in_frames = (self.parts as f64 / 16.0) * frames_per_beat;
        let subparts_in_frames = (self.subparts as f64 / 1600.0) * frames_per_beat;
        (bars_in_frames + beats_in_frames + parts_in_frames + subparts_in_frames + 0.5) as usize
    }
}
impl Display for MusicalTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}",
            self.bars, self.beats, self.parts, self.subparts
        )
    }
}
impl Add<Self> for MusicalTime {
    type Output = Self;

    // We look at only the left side's beats-per-bar value, rather than trying
    // to reconcile different ones.
    fn add(self, rhs: Self) -> Self::Output {
        let mut output = self;
        output.add_subparts(rhs.subparts);
        output.add_parts(rhs.parts);
        output.add_beats(rhs.beats);
        output.add_bars(rhs.bars);
        output
    }
}

/// Beats per minute.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Tempo(f64);
impl Default for Tempo {
    fn default() -> Self {
        Self(128.0)
    }
}
impl From<u16> for Tempo {
    fn from(value: u16) -> Self {
        Self(value as f64)
    }
}
impl From<f64> for Tempo {
    fn from(value: f64) -> Self {
        Self(value)
    }
}
impl Tempo {
    pub fn value(&self) -> f64 {
        self.0
    }
    pub fn bps(&self) -> f64 {
        self.0 / 60.0
    }
}

/// Samples per second. Always a positive integer; cannot be zero.
#[derive(Clone, Copy, Debug)]
pub struct SampleRate(NonZeroUsize);
impl SampleRate {
    pub fn value(&self) -> usize {
        self.0.get()
    }
}
impl Default for SampleRate {
    fn default() -> Self {
        Self(NonZeroUsize::new(44100).unwrap())
    }
}
impl From<f64> for SampleRate {
    fn from(value: f64) -> Self {
        if let Some(v) = NonZeroUsize::new(value as usize) {
            Self(v)
        } else {
            panic!("SampleRate must be a positive integer")
        }
    }
}
impl From<SampleRate> for f64 {
    fn from(value: SampleRate) -> Self {
        value.0.get() as f64
    }
}
impl From<SampleRate> for usize {
    fn from(value: SampleRate) -> Self {
        value.0.get()
    }
}
impl From<usize> for SampleRate {
    fn from(value: usize) -> Self {
        if let Some(checked_value) = NonZeroUsize::new(value) {
            Self(checked_value)
        } else {
            panic!("attempt to create SampleRate from invalid usize {}", value)
        }
    }
}

#[cfg(feature = "egui-framework")]
mod gui {
    use super::BeatValue;
    use crate::traits::gui::Shows;
    use eframe::{
        egui::{Frame, Margin},
        epaint::{Color32, Stroke, Vec2},
    };

    impl Shows for BeatValue {
        fn show(&mut self, ui: &mut eframe::egui::Ui) {
            ui.allocate_ui(Vec2::new(60.0, 24.0), |ui| {
                Self::show_beat_value(ui, &format!("{} beats", BeatValue::divisor(self.clone())));
            });
        }
    }

    impl BeatValue {
        fn show_beat_value(ui: &mut eframe::egui::Ui, label: &str) {
            Frame::none()
                .stroke(Stroke::new(2.0, Color32::GRAY))
                .fill(Color32::DARK_GRAY)
                .inner_margin(Margin::same(2.0))
                .outer_margin(Margin {
                    left: 0.0,
                    right: 0.0,
                    top: 0.0,
                    bottom: 5.0,
                })
                .show(ui, |ui| {
                    ui.label(label);
                });
        }

        pub fn show_inherited(ui: &mut eframe::egui::Ui) {
            Self::show_beat_value(ui, "inherited");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use more_asserts::assert_lt;

    const DEFAULT_BPM: ParameterType = 128.0;
    const DEFAULT_MIDI_TICKS_PER_SECOND: usize = 960;

    impl Clock {
        pub fn new_test() -> Self {
            Clock::new_with(&ClockParams {
                bpm: DEFAULT_BPM,
                midi_ticks_per_second: DEFAULT_MIDI_TICKS_PER_SECOND,
                time_signature: TimeSignatureParams { top: 4, bottom: 4 },
            })
        }

        pub fn debug_set_seconds(&mut self, value: f32) {
            self.was_reset = true;
            self.frames = (self.sample_rate() as f32 * value) as usize;
            self.update_internals();
        }
    }

    #[test]
    fn tempo() {
        let t = Tempo::default();
        assert_eq!(t.value(), 128.0);
    }

    #[test]
    fn sample_rate() {
        let sr = SampleRate::default();
        assert_eq!(sr.value(), 44100);
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
        assert_eq!(t.bars, 0);
        assert_eq!(t.beats, 0);
        assert_eq!(t.parts, 0);
        assert_eq!(t.subparts, 0);
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
        const ONE_SUBPART_IN_SECONDS: f64 = ONE_BEAT_IN_SECONDS / (16.0 * 100.0);
        assert_eq!(ONE_4_4_BAR_IN_SECONDS, 1.875);
        assert_eq!(ONE_BEAT_IN_SECONDS, 0.46875);

        for (bars, beats, parts, subparts, seconds) in [
            (0, 0, 0, 0, 0.0),
            (0, 0, 0, 1, ONE_SUBPART_IN_SECONDS),
            (0, 0, 1, 0, ONE_PART_IN_SECONDS),
            (0, 1, 0, 0, ONE_BEAT_IN_SECONDS),
            (1, 0, 0, 0, ONE_4_4_BAR_IN_SECONDS),
            (128 / 4, 0, 0, 0, 60.0),
        ] {
            let sample_rate_f64: f64 = sample_rate.into();
            let frames = (seconds * sample_rate_f64).round() as usize;
            assert_eq!(
                MusicalTime::new(bars, beats, parts, subparts).as_frames(tempo, sample_rate),
                frames,
                "Expected {}.{}.{}.{} -> {} frames",
                bars,
                beats,
                parts,
                subparts,
                frames,
            );
        }
    }

    #[test]
    fn frame_to_musical_time_conversions() {
        let ts = TimeSignature::default();
        let tempo = Tempo::default();
        let sample_rate = SampleRate::default();

        for (frames, bars, beats, parts, subparts) in [
            (0, 0, 0, 0, 0),
            (2646000, 32, 0, 0, 0), // one full minute
            (44100, 0, 2, 2, 13),   // one second = 128 bpm / 60 seconds/min =
                                    // 2.13333333 beats, which breaks down to 2
                                    // beats, 2 parts that are each 1/16 of a
                                    // beat = 2.133333 parts (yeah, that happens
                                    // to be the same as the 2.133333 for
                                    // beats), and multiply the .1333333 by 100
                                    // to get subparts.
        ] {
            assert_eq!(
                MusicalTime::new(bars, beats, parts, subparts),
                MusicalTime::new_from_frames(&ts, tempo, sample_rate, frames),
                "Expected {} frames -> {}.{}.{}.{}",
                frames,
                bars,
                beats,
                parts,
                subparts,
            );
        }
    }

    #[test]
    fn conversions_are_consistent() {
        let ts = TimeSignature::default();
        let tempo = Tempo::default();
        let sample_rate = SampleRate::default();

        for bars in 0..4 {
            for beats in 0..ts.top() as u8 {
                for parts in 0..16u8 {
                    for subparts in 0..100u8 {
                        // We do expect time -> frames -> time to be exact,
                        // because frames is (typically) higher resolution than
                        // time. But frames -> time -> frames is not expected to be exact.
                        let t = MusicalTime::new(bars, beats, parts, subparts);
                        let frames = t.as_frames(tempo, sample_rate);
                        let t_from_f =
                            MusicalTime::new_from_frames(&ts, tempo, sample_rate, frames);
                        assert_eq!(
                            t, t_from_f,
                            "{:?} -> {frames} -> {:?} <<< PROBLEM",
                            t, t_from_f
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn musical_time_math() {
        // Advancing by bar works
        let mut t = MusicalTime::default();
        t.add_bars(1);
        assert_eq!(t.beats, 0);
        assert_eq!(t.bars, 1);

        // Advancing by beat works
        let mut t = MusicalTime::default();
        t.add_beats(1);
        assert_eq!(t.beats, 1);
        let mut t = MusicalTime::new(0, 3, 0, 0);
        t.add_beats(1);
        assert_eq!(t.beats, 0);
        assert_eq!(t.bars, 1);

        // Advancing by part works
        let mut t = MusicalTime::default();
        t.add_parts(1);
        assert_eq!(t.bars, 0);
        assert_eq!(t.beats, 0);
        assert_eq!(t.parts, 1);
        let mut t = MusicalTime::new(0, 0, 15, 0);
        t.add_parts(1);
        assert_eq!(t.bars, 0);
        assert_eq!(t.beats, 1);
        assert_eq!(t.parts, 0);

        // Advancing by subpart works
        let mut t = MusicalTime::default();
        t.add_subparts(1);
        assert_eq!(t.bars, 0);
        assert_eq!(t.beats, 0);
        assert_eq!(t.parts, 0);
        assert_eq!(t.subparts, 1);
        let mut t = MusicalTime::new(0, 0, 0, 99);
        t.add_subparts(1);
        assert_eq!(t.bars, 0);
        assert_eq!(t.beats, 0);
        assert_eq!(t.parts, 1);
        assert_eq!(t.subparts, 0);

        // One more big rollover to be sure
        let mut t = MusicalTime::new(0, 3, 15, 99);
        t.add_subparts(1);
        assert_eq!(t.bars, 1);
        assert_eq!(t.beats, 0);
        assert_eq!(t.parts, 0);
        assert_eq!(t.subparts, 0);
    }

    #[test]
    fn musical_time_math_add_trait() {
        let bar_unit = MusicalTime::new(1, 0, 0, 0);
        let beat_unit = MusicalTime::new(0, 1, 0, 0);
        let part_unit = MusicalTime::new(0, 0, 1, 0);
        let subpart_unit = MusicalTime::new(0, 0, 0, 1);

        // Advancing by bar works
        let t = MusicalTime::default() + bar_unit;
        assert_eq!(t.beats, 0);
        assert_eq!(t.bars, 1);

        // Advancing by beat works
        let mut t = MusicalTime::default() + beat_unit;

        assert_eq!(t.beats, 1);
        t = t + beat_unit;
        assert_eq!(t.beats, 2);
        assert_eq!(t.bars, 0);
        t = t + beat_unit;
        assert_eq!(t.beats, 3);
        assert_eq!(t.bars, 0);
        t = t + beat_unit;
        assert_eq!(t.beats, 0);
        assert_eq!(t.bars, 1);

        // Advancing by part works
        let mut t = MusicalTime::default();
        assert_eq!(t.bars, 0);
        assert_eq!(t.beats, 0);
        for i in 0..16 {
            assert_eq!(t.parts, i);
            t = t + part_unit;
        }
        assert_eq!(t.beats, 1);
        assert_eq!(t.parts, 0);

        // Advancing by subpart works
        let mut t = MusicalTime::default();
        assert_eq!(t.beats, 0);
        assert_eq!(t.bars, 0);
        assert_eq!(t.parts, 0);
        for i in 0..100 {
            assert_eq!(t.subparts, i);
            t = t + subpart_unit;
        }
        assert_eq!(t.parts, 1);
        assert_eq!(t.subparts, 0);

        // One more big rollover to be sure
        let mut t = MusicalTime::new(0, 3, 15, 99);
        t = t + subpart_unit;
        assert_eq!(t.bars, 1);
        assert_eq!(t.beats, 0);
        assert_eq!(t.parts, 0);
        assert_eq!(t.subparts, 0);
    }

    #[test]
    fn musical_time_math_other_time_signatures() {
        let t = MusicalTime {
            bars: 0,
            beats: 8,
            parts: 15,
            subparts: 99,
            beats_per_bar: 9,
            ..Default::default()
        } + MusicalTime::new(0, 0, 0, 1);
        assert_eq!(t.bars, 1);
        assert_eq!(t.beats, 0);
        assert_eq!(t.parts, 0);
        assert_eq!(t.subparts, 0);
    }

    #[test]
    fn musical_time_overflow() {
        let ts = TimeSignature::new_with(4, 256).unwrap();

        let time_params = MusicalTimeParams {
            bars: 0,
            beats: (ts.top - 1) as u8,
            parts: 16 - 1,
            subparts: 99,
        };
        eprintln!("{:?}", time_params);

        let mut t = MusicalTime::new_with(&time_params);
        t.add_beats(1);
        assert_eq!(t.beats, 0);
        assert_eq!(t.bars, 1);

        let mut t = MusicalTime::new_with(&time_params);
        t.add_parts(1);
        assert_eq!(t.parts, 0);
        assert_eq!(t.beats, 0);
        assert_eq!(t.bars, 1);

        let mut t = MusicalTime::new_with(&time_params);
        t.add_subparts(1);
        assert_eq!(t.subparts, 0);
        assert_eq!(t.parts, 0);
        assert_eq!(t.beats, 0);
        assert_eq!(t.bars, 1);
    }
}
