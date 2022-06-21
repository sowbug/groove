pub struct Clock {
    pub sample_clock: f32,
    pub sample_rate: f32,

    pub real_clock: f32,
}

impl Clock {
    pub fn tick(&mut self) {
        self.sample_clock = (self.sample_clock + 1.0) % self.sample_rate;
        self.real_clock = self.real_clock + 1. / self.sample_rate;
    }
}

pub trait ClockWatcherTrait {
    fn handle_time_slice(&mut self, clock: &Clock) -> bool;
}
