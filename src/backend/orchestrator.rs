use std::cell::RefCell;
use std::rc::Rc;

use crate::backend::clock::Clock;
use crate::backend::clock::ClockWatcherTrait;
use crate::backend::devices::DeviceTrait;
use crate::backend::instruments::old_Oscillator;
use crate::backend::effects::Mixer;
use crate::backend::sequencer::old_Sequencer;

pub struct Orchestrator {
    // https://en.wikipedia.org/wiki/Time_signature
    _time_signature_top: u32,
    _time_signature_bottom: u32,

    pub clock: Clock,

    pub master_mixer: Rc<RefCell<Mixer>>, // TODO(miket): should be private
    devices: Vec<Rc<RefCell<dyn DeviceTrait>>>,
}

impl Orchestrator {
    pub fn new() -> Orchestrator {
        Orchestrator {
            _time_signature_top: 4,
            _time_signature_bottom: 4,
            clock: Clock {
                sample_clock: 0.,
                sample_rate: 0.,
                real_clock: 0.,
            },
            master_mixer: Rc::new(RefCell::new(Mixer::new())),
            devices: Vec::new(),
        }
    }

    // pub fn tmp_add_some_notes(&mut self) {
    //     self.sequencer.add_note(60, 0.25, 0.2);
    //     self.sequencer.add_note(66, 0.50, 0.2);
    // }

    pub fn add_device(&mut self, device: Rc<RefCell<dyn DeviceTrait>>) {
        self.devices.push(device);
    }

    pub fn play(&mut self) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create("sine.wav", spec).unwrap();
        let amplitude = i16::MAX as f32;

        for t in 0..spec.sample_rate {
            for d in self.devices.clone() {
                if d.borrow().sources_midi() {
                    d.borrow_mut().tick(t as f32 / spec.sample_rate as f32);
                }
            }
            for d in self.devices.clone() {
                if d.borrow().sources_audio() {
                    d.borrow_mut().tick(t as f32 / spec.sample_rate as f32);
                }
            }
            let sample = self.master_mixer.borrow().get_audio_sample();
            writer.write_sample((sample * amplitude) as i16).unwrap();
        }
    }

    // pub fn write_sample_data<T: cpal::Sample>(
    //     &mut self,
    //     data: &mut [T],
    //     _info: &cpal::OutputCallbackInfo,
    // ) {
    //     for sample in data.iter_mut() {
    //         self.clock.tick();
    //         self.sequencer.handle_time_slice(&self.clock);
    //         let the_sample: f32 = self.sequencer.oscillator.get_sample(&self.clock);
    //         *sample = cpal::Sample::from(&the_sample);
    //     }
    // }
}
