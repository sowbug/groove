use crossbeam::deque::Worker;
use rhai::Engine;
use std::{cell::RefCell, rc::Rc};

use crate::{
    common::WaveformType::Sine, devices::{orchestrator::Orchestrator, sequencer::Sequencer, midi::MidiReader},
    primitives::oscillators::MiniOscillator, synthesizers::welsh,
};

pub struct ScriptEngine {
    engine: Engine,
}

impl ScriptEngine {
    pub fn new() -> Self {
        let mut r = Self {
            engine: Engine::new(),
        };
        r.register_methods();
        r
    }

    pub(crate) fn execute(&self, script: &str) -> Result<(), anyhow::Error> {
        self.engine.run(script);
        Ok(())
    }

    fn send_performance_to_file(
        sample_rate: u32,
        output_filename: &str,
        worker: &Worker<f32>,
    ) -> anyhow::Result<()> {
        const AMPLITUDE: f32 = i16::MAX as f32;
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(output_filename, spec).unwrap();

        while !worker.is_empty() {
            let sample = worker.pop().unwrap_or_default();
            writer.write_sample((sample * AMPLITUDE) as i16).unwrap();
        }
        Ok(())
    }

    fn new_synth() -> Rc<RefCell<welsh::Synth>> {
        Rc::new(RefCell::new(welsh::Synth::new(
            44100,
            welsh::SynthPreset::by_name(&welsh::PresetName::Piano),
        )))
    }

    fn add_synth(orchestrator: &mut Orchestrator, synth: Rc<RefCell<welsh::Synth>>) {
        orchestrator.add_device(synth.clone());
        orchestrator.add_master_mixer_source(synth.clone());

        let sequencer = Rc::new(RefCell::new(Sequencer::new()));
        orchestrator.add_device(sequencer.clone());

        let data = std::fs::read("midi_files/major-scale-spaced-notes.mid").unwrap();
        MidiReader::load_sequencer(&data, sequencer.clone());

        sequencer.borrow_mut().connect_midi_sink_for_channel(synth, 0);
    }

    fn play(orchestrator: &mut Orchestrator) {
        let worker = Worker::<f32>::new_fifo();
        let result = orchestrator.perform_to_queue(&worker);
        if result.is_err() {
            panic!("oh no");
        }
        if Self::send_performance_to_file(44100, "out.wav", &worker).is_err() {
            panic!("oh no again");
        }
    }

    fn register_methods(&mut self) {
        self.engine
            .register_type_with_name::<Orchestrator>("Orchestrator")
            .register_fn("new_orchestrator", Orchestrator::new_44100)
            .register_fn("new_synth", Self::new_synth)
            .register_fn("add_synth", Self::add_synth)
            .register_fn("play", Self::play);
    }
}
