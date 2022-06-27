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
