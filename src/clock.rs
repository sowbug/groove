use anyhow::{anyhow, Error};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    fmt::Display,
    ops::{Add, Mul},
};
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
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
    pub top: usize,
    pub bottom: usize,
}

impl TimeSignature {
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
}

impl Default for TimeSignature {
    fn default() -> Self {
        Self { top: 4, bottom: 4 }
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
}
