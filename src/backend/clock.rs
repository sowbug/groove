#[derive(Default)]
pub struct Clock {
    pub sample_count: u32,
    pub sample_clock: u32,
    pub sample_rate: u32,

    pub real_clock: f32,
}

impl Clock {
    pub fn new(samples_per_second: u32) -> Clock {
        Clock {
            sample_rate: samples_per_second,
            ..Default::default()
        }
    }
    pub fn tick(&mut self) {
        self.sample_count += 1;
        self.sample_clock = (self.sample_clock + 1) % self.sample_rate;
        self.real_clock = self.sample_count as f32 / self.sample_rate as f32;
    }
}
