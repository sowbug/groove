use crossbeam::deque::Worker;
use rhai::Engine;
use std::{cell::RefCell, rc::Rc};

use crate::{
    devices::{midi::MidiReader, orchestrator::Orchestrator, sequencer::Sequencer},
    synthesizers::welsh,
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

    pub(crate) fn execute_file(&self, filename: &str) -> Result<(), anyhow::Error> {
        let script = std::fs::read_to_string(filename).unwrap();
        self.execute(script.as_str())
    }

    pub(crate) fn execute(&self, script: &str) -> Result<(), anyhow::Error> {
        let result = self.engine.run(script);
        if result.is_ok() {
            Ok(())
        } else {
            let err = result.err().unwrap();
            match err.unwrap_inner() {
                rhai::EvalAltResult::ErrorArithmetic(a, b) => {
                    panic!("{:?} {:?}", a, b);
                }
                rhai::EvalAltResult::ErrorArrayBounds(a, b, c) => {
                    panic!("{:?} {:?} {:?}", a, b, c);
                }
                rhai::EvalAltResult::ErrorAssignmentToConstant(a, b) => {
                    panic!("{:?} {:?}", a, b);
                }
                rhai::EvalAltResult::ErrorBitFieldBounds(a, b, c) => {
                    panic!("{:?} {:?} {:?}", a, b, c);
                }
                rhai::EvalAltResult::ErrorCustomSyntax(a, b, c) => {
                    panic!("{:?} {:?} {:?}", a, b, c);
                }
                rhai::EvalAltResult::ErrorDataRace(a, b) => {
                    panic!("{:?} {:?}", a, b);
                }
                rhai::EvalAltResult::ErrorDataTooLarge(a, b) => {
                    panic!("{:?} {:?}", a, b);
                }
                rhai::EvalAltResult::ErrorDotExpr(a, b) => {
                    panic!("{:?} {:?}", a, b);
                }
                rhai::EvalAltResult::ErrorFor(a) => {
                    panic!("{:?}", a);
                }
                rhai::EvalAltResult::ErrorForbiddenVariable(a, b) => {
                    panic!("{:?} {:?}", a, b);
                }
                rhai::EvalAltResult::ErrorFunctionNotFound(a, b) => {
                    panic!("{:?} {:?}", a, b);
                }
                rhai::EvalAltResult::ErrorInFunctionCall(a, b, c, d) => {
                    panic!("{:?} {:?} {:?} {:?}", a, b, c, d);
                }
                rhai::EvalAltResult::ErrorInModule(a, b, c) => {
                    panic!("{:?} {:?} {:?}", a, b, c);
                }
                rhai::EvalAltResult::ErrorIndexNotFound(a, b) => {
                    panic!("{:?} {:?}", a, b);
                }
                rhai::EvalAltResult::ErrorIndexingType(a, b) => {
                    panic!("{:?} {:?}", a, b);
                }
                rhai::EvalAltResult::ErrorMismatchDataType(a, b, c) => {
                    panic!("{:?} {:?} {:?}", a, b, c);
                }
                rhai::EvalAltResult::ErrorMismatchOutputType(a, b, c) => {
                    panic!("{:?} {:?} {:?}", a, b, c);
                }
                rhai::EvalAltResult::ErrorModuleNotFound(a, b) => {
                    panic!("{:?} {:?}", a, b);
                }
                rhai::EvalAltResult::ErrorParsing(a, b) => {
                    panic!("{:?} {:?}", a, b)
                }
                rhai::EvalAltResult::ErrorPropertyNotFound(a, b) => {
                    panic!("{:?} {:?}", a, b);
                }
                rhai::EvalAltResult::ErrorRuntime(a, b) => {
                    panic!("{:?} {:?}", a, b);
                }
                rhai::EvalAltResult::ErrorStackOverflow(a) => {
                    panic!("{:?}", a);
                }
                rhai::EvalAltResult::ErrorStringBounds(a, b, c) => {
                    panic!("{:?} {:?} {:?}", a, b, c);
                }
                rhai::EvalAltResult::ErrorSystem(a, b) => {
                    panic!("{:?} {:?}", a, b);
                }
                rhai::EvalAltResult::ErrorTerminated(a, b) => {
                    panic!("{:?} {:?}", a, b);
                }
                rhai::EvalAltResult::ErrorTooManyModules(a) => {
                    panic!("{:?}", a);
                }
                rhai::EvalAltResult::ErrorTooManyOperations(a) => {
                    panic!("{:?}", a);
                }
                rhai::EvalAltResult::ErrorUnboundThis(a) => {
                    panic!("{:?}", a);
                }
                rhai::EvalAltResult::ErrorVariableExists(a, b) => {
                    panic!("{:?} {:?}", a, b);
                }
                rhai::EvalAltResult::ErrorVariableNotFound(a, b) => {
                    panic!("{:?} {:?}", a, b);
                }
                _ => {
                    panic!();
                }
            }
        }
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

    fn new_sequencer() -> Rc<RefCell<Sequencer>> {
        Rc::new(RefCell::new(Sequencer::new()))
    }

    fn add_synth(orchestrator: &mut Orchestrator, device: Rc<RefCell<welsh::Synth>>) {
        orchestrator.add_device(device.clone());
        orchestrator.add_master_mixer_source(device.clone());
    }

    fn add_sequencer(orchestrator: &mut Orchestrator, device: Rc<RefCell<Sequencer>>) {
        orchestrator.add_device(device.clone());
    }

    fn load_file(sequencer: Rc<RefCell<Sequencer>>, filename: &str) {
        let data = std::fs::read(filename).unwrap();
        MidiReader::load_sequencer(&data, sequencer.clone());
    }

    fn connect_synth(sequencer: Rc<RefCell<Sequencer>>, device: Rc<RefCell<welsh::Synth>>) {
        sequencer
            .borrow_mut()
            .connect_midi_sink_for_channel(device, 0);
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
            .register_fn("new_sequencer", Self::new_sequencer)
            .register_fn("add_sequencer", Self::add_sequencer)
            .register_fn("load_file", Self::load_file)
            .register_fn("connect_synth", Self::connect_synth)
            .register_fn("play", Self::play);
    }
}
