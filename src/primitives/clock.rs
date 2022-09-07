use serde::{Deserialize, Serialize};

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
    pub top: u32,
    pub bottom: u32,
}

impl TimeSignature {
    pub(crate) fn new(top: u32, bottom: u32) -> TimeSignature {
        if top == 0 {
            panic!("Time signature top number can't be zero.");
        }
        BeatValue::from_divisor(bottom as f32); // this will panic if number is invalid.
        TimeSignature { top, bottom }
    }

    #[allow(dead_code)]
    pub fn beat_value(&self) -> BeatValue {
        BeatValue::from_divisor(self.bottom as f32)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct ClockSettings {
    #[serde(rename = "sample-rate")]
    samples_per_second: usize, // Samples per second; granularity of a tick().

    #[serde(rename = "bpm")]
    beats_per_minute: f32,

    #[serde(rename = "time-signature")]
    time_signature: TimeSignature,
}

impl ClockSettings {
    #[allow(dead_code)]
    pub(crate) fn new(
        samples_per_second: usize,
        beats_per_minute: f32,
        time_signature: (u32, u32),
    ) -> Self {
        Self {
            samples_per_second,
            beats_per_minute,
            time_signature: TimeSignature {
                top: time_signature.0,
                bottom: time_signature.1,
            },
        }
    }

    #[allow(dead_code)]
    pub(crate) fn new_defaults() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn sample_rate(&self) -> usize {
        self.samples_per_second
    }

    pub fn time_signature(&self) -> TimeSignature {
        self.time_signature
    }

    #[allow(dead_code)]
    pub(crate) fn bpm(&self) -> f32 {
        self.beats_per_minute
    }
}

impl Default for ClockSettings {
    fn default() -> Self {
        Self {
            samples_per_second: 44100,
            beats_per_minute: 128.0,
            time_signature: TimeSignature { top: 4, bottom: 4 },
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct Clock {
    settings: ClockSettings,

    pub samples: usize, // Samples since clock creation.
    pub seconds: f32, // Seconds elapsed since clock creation.
    pub beats: usize,   // Beats elapsed since clock creation.
}

impl Clock {
    pub fn new(settings: ClockSettings) -> Self {
        Self {
            settings,
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub fn settings(&self) -> &ClockSettings {
        &self.settings
    }

    pub fn tick(&mut self) {
        self.samples += 1;
        self.seconds = self.samples as f32 / self.settings.samples_per_second as f32;
        self.beats = ((self.seconds / 60.0) * self.settings.beats_per_minute).floor() as usize;
    }
}

#[cfg(test)]
mod tests {
    use std::panic;

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

    #[test]
    fn test_clock_mainline() {
        const SAMPLE_RATE: usize = 256;
        const BPM: f32 = 128.0;
        const QUARTER_NOTE_OF_TICKS: usize = ((SAMPLE_RATE * 60) as f32 / BPM) as usize;
        const SECONDS_PER_BEAT: f32 = 60.0 / BPM;

        let clock_settings = ClockSettings {
            samples_per_second: SAMPLE_RATE,
            beats_per_minute: BPM,
            time_signature: TimeSignature { top: 4, bottom: 4 },
        };
        let mut clock = Clock::new(clock_settings);

        // init state
        assert_eq!(clock.samples, 0);
        assert_eq!(clock.seconds, 0.0);

        // Check after one tick.
        clock.tick();
        assert_eq!(clock.samples, 1);
        assert_eq!(clock.seconds, 1.0 / SAMPLE_RATE as f32);
        assert_eq!(clock.beats, 0);

        // Check around a full quarter note of ticks.
        // minus one because we already did one tick(), then minus another to test edge
        for _ in 0..QUARTER_NOTE_OF_TICKS - 1 - 1 {
            clock.tick();
        }
        assert_eq!(clock.samples, QUARTER_NOTE_OF_TICKS - 1);
        assert!(clock.seconds < SECONDS_PER_BEAT as f32);
        assert_eq!(clock.beats, 0);

        // Now right on the quarter note.
        clock.tick();
        assert_eq!(clock.samples, QUARTER_NOTE_OF_TICKS);
        assert_eq!(clock.seconds, SECONDS_PER_BEAT as f32);
        assert_eq!(clock.beats, 1);

        // One full minute.
        for _ in 0..QUARTER_NOTE_OF_TICKS * (BPM - 1.0) as usize {
            clock.tick();
        }
        assert_eq!(clock.samples, SAMPLE_RATE * 60);
        assert_eq!(clock.seconds, 60.0);
        assert_eq!(clock.beats, BPM as usize);
    }

    #[test]
    fn test_time_signature() {
        let ts = TimeSignature::new(4, 4);
        assert_eq!(ts.top, 4);
        assert_eq!(ts.bottom, 4);

        assert!(matches!(ts.beat_value(), BeatValue::Quarter));

        assert!(panic::catch_unwind(|| { TimeSignature::new(0, 4) }).is_err());
        assert!(panic::catch_unwind(|| { TimeSignature::new(4, 5) }).is_err());
    }
}
