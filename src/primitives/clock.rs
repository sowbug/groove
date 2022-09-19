use serde::{Deserialize, Serialize};

use crate::settings::ClockSettings;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum BeatValue {
    Whole,
    Half,
    Quarter,
    Eighth,
    Sixteenth,
    ThirtySecond,
    SixtyFourth,
    OneHundredTwentyEighth,
    TwoHundredFiftySixth,
    FiveHundredTwelfth,
}

impl BeatValue {
    pub fn divisor(&self) -> f32 {
        match self {
            BeatValue::Whole => 1.0,
            BeatValue::Half => 2.0,
            BeatValue::Quarter => 4.0,
            BeatValue::Eighth => 8.0,
            BeatValue::Sixteenth => 16.0,
            BeatValue::ThirtySecond => 32.0,
            BeatValue::SixtyFourth => 64.0,
            BeatValue::OneHundredTwentyEighth => 128.0,
            BeatValue::TwoHundredFiftySixth => 256.0,
            BeatValue::FiveHundredTwelfth => 512.0,
        }
    }

    pub fn from_divisor(divisor: f32) -> Self {
        match divisor as u32 {
            1 => BeatValue::Whole,
            2 => BeatValue::Half,
            4 => BeatValue::Quarter,
            8 => BeatValue::Eighth,
            16 => BeatValue::Sixteenth,
            32 => BeatValue::ThirtySecond,
            64 => BeatValue::SixtyFourth,
            128 => BeatValue::OneHundredTwentyEighth,
            256 => BeatValue::TwoHundredFiftySixth,
            512 => BeatValue::FiveHundredTwelfth,
            _ => {
                panic!("unrecognized divisor for time signature: {}", divisor);
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
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
    pub top: u32,
    pub bottom: u32,
}

impl Default for TimeSignature {
    fn default() -> Self {
        Self { top: 4, bottom: 4 }
    }
}

impl TimeSignature {
    pub(crate) fn new(top: u32, bottom: u32) -> Self {
        if top == 0 {
            panic!("Time signature top number can't be zero.");
        }
        BeatValue::from_divisor(bottom as f32); // this will panic if number is invalid.
        TimeSignature { top, bottom }
    }

    #[allow(dead_code)]
    /// ```
    /// let ts = libgroove::primitives::clock::TimeSignature::new_defaults();
    /// assert_eq!(ts.top, 4); 
    /// assert_eq!(ts.bottom, 4);
    /// ``` 
    pub fn beat_value(&self) -> BeatValue {
        BeatValue::from_divisor(self.bottom as f32)
    }

    #[allow(dead_code)]
    pub fn new_defaults() -> Self {
        Self::new(4, 4)
    }
}
#[derive(Default, Debug, Clone)]
pub struct Clock {
    settings: ClockSettings,

    pub samples: usize, // Samples since clock creation.
    pub seconds: f32,   // Seconds elapsed since clock creation.
    pub beats: f32,     // Beats elapsed since clock creation.
}

impl Clock {
    pub fn new(settings: &ClockSettings) -> Self {
        Self {
            settings: settings.clone(),
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub fn settings(&self) -> &ClockSettings {
        &self.settings
    }

    pub fn tick(&mut self) {
        self.samples += 1;
        self.seconds = self.samples as f32 / self.settings.sample_rate() as f32;
        self.beats = (self.settings.bpm() / 60.0) * self.seconds;
    }
}

#[cfg(test)]
mod tests {
    use std::panic;

    use more_asserts::assert_lt;

    use super::*;

    impl ClockSettings {
        const TEST_SAMPLE_RATE: usize = 256;
        const TEST_BPM: f32 = 99.;
        pub fn new_test() -> Self {
            Self::new(
                ClockSettings::TEST_SAMPLE_RATE,
                ClockSettings::TEST_BPM,
                (4, 4),
            )
        }
    }

    impl Clock {
        pub fn new_test() -> Self {
            Self::new(&ClockSettings::new_test())
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
        let mut clock = Clock::new(&clock_settings);

        // init state
        assert_eq!(clock.samples, 0);
        assert_eq!(clock.seconds, 0.0);
        assert_eq!(clock.beats, 0.0);

        // Check after one tick.
        clock.tick();
        assert_eq!(clock.samples, 1);
        assert_eq!(clock.seconds, ONE_SAMPLE_OF_SECONDS);
        assert_eq!(clock.beats, (BPM / 60.0) * ONE_SAMPLE_OF_SECONDS);

        // Check around a full quarter note of ticks.
        // minus one because we already did one tick(), then minus another to test edge
        for _ in 0..QUARTER_NOTE_OF_TICKS - 1 - 1 {
            clock.tick();
        }
        assert_eq!(clock.samples, QUARTER_NOTE_OF_TICKS - 1);
        assert!(clock.seconds < SECONDS_PER_BEAT as f32);
        assert_lt!(clock.beats, 1.0);

        // Now right on the quarter note.
        clock.tick();
        assert_eq!(clock.samples, QUARTER_NOTE_OF_TICKS);
        assert_eq!(clock.seconds, SECONDS_PER_BEAT as f32);
        assert_eq!(clock.beats, 1.0);

        // One full minute.
        for _ in 0..QUARTER_NOTE_OF_TICKS * (BPM - 1.0) as usize {
            clock.tick();
        }
        assert_eq!(clock.samples, SAMPLE_RATE * 60);
        assert_eq!(clock.seconds, 60.0);
        assert_eq!(clock.beats, BPM);
    }

    #[test]
    fn test_time_signature_valid() {
        let ts = TimeSignature::new(4, 4);
        assert_eq!(ts.top, 4);
        assert_eq!(ts.bottom, 4);

        assert!(matches!(ts.beat_value(), BeatValue::Quarter));
    }

    #[test]
    #[should_panic]
    fn test_time_signature_invalid_bad_top() {
        TimeSignature::new(0, 4);
    }

    #[test]
    #[should_panic]
    fn test_time_signature_invalid_bottom_not_power_of_two() {
        TimeSignature::new(4, 5);
    }

    #[test]
    #[should_panic]
    fn test_time_signature_invalid_bottom_below_range() {
        TimeSignature::new(4, 0);
    }

    #[test]
    #[should_panic]
    fn test_time_signature_invalid_bottom_above_range() {
        // 2^10 = 1024
        TimeSignature::new(4, BeatValue::from_divisor(2.0f32.powi(10)).divisor() as u32);
    }
}
