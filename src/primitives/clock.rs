use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
pub struct TimeSignature {
    pub top: u32,
    pub bottom: u32,
}

impl TimeSignature {
    pub(crate) fn new(top: u32, bottom: u32) -> TimeSignature {
        TimeSignature { top, bottom }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct ClockSettings {
    #[serde(rename = "sample-rate")]
    samples_per_second: u32, // Samples per second; granularity of a tick().

    #[serde(rename = "bpm")]
    beats_per_minute: f32,

    #[serde(rename = "time-signature")]
    time_signature: TimeSignature,
}

impl ClockSettings {
    #[allow(dead_code)]
    pub(crate) fn new(
        samples_per_second: u32,
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

    pub fn sample_rate(&self) -> u32 {
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

    pub samples: u32, // Samples since clock creation.
    pub seconds: f32, // Seconds elapsed since clock creation.
    pub beats: u32,   // Beats elapsed since clock creation.
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
        self.beats = ((self.seconds / 60.0) * self.settings.beats_per_minute).floor() as u32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl ClockSettings {
        const TEST_SAMPLE_RATE: u32 = 256;
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
        const SAMPLE_RATE: u32 = 256;
        const BPM: f32 = 128.0;
        const QUARTER_NOTE_OF_TICKS: u32 = ((SAMPLE_RATE * 60) as f32 / BPM) as u32;
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
        for _ in 0..QUARTER_NOTE_OF_TICKS * (BPM - 1.0) as u32 {
            clock.tick();
        }
        assert_eq!(clock.samples, SAMPLE_RATE * 60);
        assert_eq!(clock.seconds, 60.0);
        assert_eq!(clock.beats, BPM as u32);
    }
}
