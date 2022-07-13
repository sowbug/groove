#[derive(Default, Debug)]
pub struct Clock {
    // Immutable after creation.
    samples_per_second: u32, // Samples per second; granularity of a tick().

    // Mutable after creation.
    pub time_signature_numerator: u32,
    time_signature_denominator: u32,
    pub beats_per_minute: f32,

    // Updated on each tick().
    pub samples: u32, // Samples since clock creation.
    pub seconds: f32, // Seconds elapsed since clock creation.
    pub beats: u32,   // Beats elapsed since clock creation.
}

impl Clock {
    pub fn new(
        samples_per_second: u32,
        time_signature_numerator: u32,
        time_signature_denominator: u32,
        beats_per_minute: f32,
    ) -> Self {
        Self {
            samples_per_second,
            time_signature_numerator,
            time_signature_denominator,
            beats_per_minute,
            ..Default::default()
        }
    }
    pub fn sample_rate(&self) -> u32 {
        self.samples_per_second
    }
    pub fn tick(&mut self) {
        self.samples += 1;
        self.seconds = self.samples as f32 / self.samples_per_second as f32;
        self.beats = ((self.seconds / 60.0) * self.beats_per_minute).floor() as u32;
    }
}

pub struct TimeSignature {
    numerator: u32,
    denominator: u32,
}

impl TimeSignature {
    pub fn new(numerator: u32, denominator: u32) -> Self {
        Self {
            numerator,
            denominator,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Clock {
        pub const TEST_SAMPLE_RATE: u32 = 256;
        pub const TEST_BPM: f32 = 99.;
        pub fn new_test() -> Self {
            Self::new(Clock::TEST_SAMPLE_RATE, 4, 4, Clock::TEST_BPM)
        }
    }

    #[test]
    fn test_clock_mainline() {
        const SAMPLE_RATE: u32 = 256;
        const BPM: f32 = 128.0;
        const QUARTER_NOTE_OF_TICKS: u32 = ((SAMPLE_RATE * 60) as f32 / BPM) as u32;
        const SECONDS_PER_BEAT: f32 = 60.0 / BPM;

        let mut clock = Clock::new(SAMPLE_RATE, 4, 4, 128.);

        // init state
        assert_eq!(clock.samples_per_second, SAMPLE_RATE);
        assert_eq!(clock.samples, 0);
        assert_eq!(clock.seconds, 0.0);
        assert_eq!(clock.time_signature_numerator, 4);
        assert_eq!(clock.time_signature_denominator, 4);
        assert_eq!(clock.beats_per_minute, 128.0);

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
