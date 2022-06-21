use crate::clock::Clock;
use crate::clock::ClockWatcherTrait;
use crate::sequencer::Sequencer;

pub struct Orchestrator {
    // https://en.wikipedia.org/wiki/Time_signature
    _time_signature_top: u32,
    _time_signature_bottom: u32,

    sequencer: Sequencer,

    pub clock: Clock,
}
impl Orchestrator {
    pub fn new() -> Orchestrator {
        Orchestrator {
            _time_signature_top: 4,
            _time_signature_bottom: 4,
            sequencer: Sequencer::new(),
            clock: Clock {
                sample_clock: 0.,
                sample_rate: 0.,
                real_clock: 0.,
            },
        }
    }

    pub fn tmp_add_some_notes(&mut self) {
        self.sequencer.add_note(60, 0.25, 0.2);
        self.sequencer.add_note(66, 0.50, 0.2);
    }

    pub fn write_sample_data<T: cpal::Sample>(
        &mut self,
        data: &mut [T],
        _info: &cpal::OutputCallbackInfo,
    ) {
        for sample in data.iter_mut() {
            self.clock.tick();
            self.sequencer.handle_time_slice(&self.clock);
            let the_sample: f32 = self.sequencer.oscillator.get_sample(&self.clock);
            *sample = cpal::Sample::from(&the_sample);
        }
    }
}
