#[derive(Default)]
pub struct Clock {
    pub sample_rate: u32,   // Samples per second; granularity of this clock.
    pub samples: u32,       // Samples since this clock was created.
    pub seconds: f32,       // Seconds elapsed since this clock was created.
}

impl Clock {
    pub fn new(samples_per_second: u32) -> Clock {
        Clock {
            sample_rate: samples_per_second,
            ..Default::default()
        }
    }
    pub fn tick(&mut self) {
        self.samples += 1;
        self.seconds = self.samples as f32 / self.sample_rate as f32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_clock_mainline() {
        const SAMPLE_RATE: u32 = 256;
        let mut clock = Clock::new(SAMPLE_RATE);

        // init state
        assert_eq!(clock.sample_rate, SAMPLE_RATE);
        assert_eq!(clock.samples, 0);
        assert_eq!(clock.seconds, 0.0);

        // after a tick
        clock.tick();
        assert_eq!(clock.samples, 1);
        assert_eq!(clock.seconds, 1.0 / SAMPLE_RATE as f32);
    }
}
