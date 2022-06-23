use std::cell::RefCell;
use std::rc::Rc;
use crate::backend::clock::Clock;
use crate::backend::devices::DeviceTrait;
use crate::backend::effects::Mixer;

pub struct Orchestrator {
    // https://en.wikipedia.org/wiki/Time_signature
    _time_signature_top: u32,
    _time_signature_bottom: u32,

    pub clock: Clock,

    pub master_mixer: Rc<RefCell<Mixer>>, // TODO(miket): should be private
    devices: Vec<Rc<RefCell<dyn DeviceTrait>>>,
}

impl Orchestrator {
    pub fn new(sample_rate: u32) -> Orchestrator {
        Orchestrator {
            _time_signature_top: 4,
            _time_signature_bottom: 4,
            clock: Clock::new(sample_rate as f32),
            master_mixer: Rc::new(RefCell::new(Mixer::new())),
            devices: Vec::new(),
        }
    }

    pub fn add_device(&mut self, device: Rc<RefCell<dyn DeviceTrait>>) {
        self.devices.push(device);
    }

    fn tick(&mut self) -> f32 {
        for d in self.devices.clone() {
            if d.borrow().sources_midi() {
                d.borrow_mut().tick(&self.clock);
            }
        }
        for d in self.devices.clone() {
            if d.borrow().sources_audio() {
                d.borrow_mut().tick(&self.clock);
            }
        }
        self.clock.tick();
        self.master_mixer.borrow().get_audio_sample()
    }

    pub fn perform_to_file(&mut self, output_filename: &str) -> anyhow::Result<()> {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(output_filename, spec).unwrap();
        let amplitude = i16::MAX as f32;

        while self.clock.real_clock < 20.0 {
            let sample = self.tick();
            writer.write_sample((sample * amplitude) as i16).unwrap();
        }
        Ok(())
    }

    pub fn write_sample_data<T: cpal::Sample>(
        &mut self,
        data: &mut [T],
        _info: &cpal::OutputCallbackInfo,
    ) {
        for next_sample in data.iter_mut() {
            let one_sample = self.tick();
            *next_sample = cpal::Sample::from(&one_sample);
        }
    }
}