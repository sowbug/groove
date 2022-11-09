use crate::{
    common::{rrc, rrc_clone, rrc_downgrade, Rrc},
    effects::{bitcrusher::Bitcrusher, limiter::Limiter},
    midi::{programmers::MidiSmfReader, sequencers::MidiTickSequencer, MidiChannel},
    settings::patches::SynthPatch,
    synthesizers::welsh,
    traits::{IsMidiInstrument, SinksAudio, SinksMidi, SourcesAudio, SourcesMidi},
    IOHelper, Orchestrator,
};
use rhai::{Engine, EvalAltResult};
use std::path::PathBuf;

#[derive(Default)]
pub struct ScriptEngine {
    engine: Engine,
}

impl ScriptEngine {
    pub fn new() -> Self {
        let mut r = Self::default();
        r.register_methods();
        r
    }

    pub fn execute_file(&self, filename: &str) -> Result<(), anyhow::Error> {
        let mut path = PathBuf::new();
        path.set_file_name(filename);
        let result = self.engine.run_file(path);
        if result.is_ok() {
            Ok(())
        } else {
            Self::unpack_error(result);
            Err(anyhow::Error::msg("oops"))
        }
    }

    fn unpack_error(result: Result<(), Box<EvalAltResult>>) {
        let err = result.err().unwrap();
        match err.unwrap_inner() {
            rhai::EvalAltResult::ErrorArithmetic(a, b) => {
                panic!("{a:?} {b:?}");
            }
            rhai::EvalAltResult::ErrorArrayBounds(a, b, c) => {
                panic!("{a:?} {b:?} {c:?}");
            }
            rhai::EvalAltResult::ErrorAssignmentToConstant(a, b) => {
                panic!("{a:?} {b:?}");
            }
            rhai::EvalAltResult::ErrorBitFieldBounds(a, b, c) => {
                panic!("{a:?} {b:?} {c:?}");
            }
            rhai::EvalAltResult::ErrorCustomSyntax(a, b, c) => {
                panic!("{a:?} {b:?} {c:?}");
            }
            rhai::EvalAltResult::ErrorDataRace(a, b) => {
                panic!("{a:?} {b:?}");
            }
            rhai::EvalAltResult::ErrorDataTooLarge(a, b) => {
                panic!("{a:?} {b:?}");
            }
            rhai::EvalAltResult::ErrorDotExpr(a, b) => {
                panic!("{a:?} {b:?}");
            }
            rhai::EvalAltResult::ErrorFor(a) => {
                panic!("{a:?}");
            }
            rhai::EvalAltResult::ErrorForbiddenVariable(a, b) => {
                panic!("{a:?} {b:?}");
            }
            rhai::EvalAltResult::ErrorFunctionNotFound(a, b) => {
                panic!(
                    "rhai::EvalAltResult::ErrorFunctionNotFound({:?}, {:?})",
                    a, b
                );
            }
            rhai::EvalAltResult::ErrorInFunctionCall(a, b, c, d) => {
                panic!("{a:?} {b:?} {c:?} {d:?}");
            }
            rhai::EvalAltResult::ErrorInModule(a, b, c) => {
                panic!("{a:?} {b:?} {c:?}");
            }
            rhai::EvalAltResult::ErrorIndexNotFound(a, b) => {
                panic!("{a:?} {b:?}");
            }
            rhai::EvalAltResult::ErrorIndexingType(a, b) => {
                panic!("{a:?} {b:?}");
            }
            rhai::EvalAltResult::ErrorMismatchDataType(a, b, c) => {
                panic!("{a:?} {b:?} {c:?}");
            }
            rhai::EvalAltResult::ErrorMismatchOutputType(a, b, c) => {
                panic!("{a:?} {b:?} {c:?}");
            }
            rhai::EvalAltResult::ErrorModuleNotFound(a, b) => {
                panic!("{a:?} {b:?}");
            }
            rhai::EvalAltResult::ErrorParsing(a, b) => {
                panic!("{a:?} {b:?}")
            }
            rhai::EvalAltResult::ErrorPropertyNotFound(a, b) => {
                panic!("{a:?} {b:?}");
            }
            rhai::EvalAltResult::ErrorRuntime(a, b) => {
                panic!("{a:?} {b:?}");
            }
            rhai::EvalAltResult::ErrorStackOverflow(a) => {
                panic!("{a:?}");
            }
            rhai::EvalAltResult::ErrorStringBounds(a, b, c) => {
                panic!("{a:?} {b:?} {c:?}");
            }
            rhai::EvalAltResult::ErrorSystem(a, b) => {
                panic!("{a:?} {b:?}");
            }
            rhai::EvalAltResult::ErrorTerminated(a, b) => {
                panic!("{a:?} {b:?}");
            }
            rhai::EvalAltResult::ErrorTooManyModules(a) => {
                panic!("{a:?}");
            }
            rhai::EvalAltResult::ErrorTooManyOperations(a) => {
                panic!("{a:?}");
            }
            rhai::EvalAltResult::ErrorUnboundThis(a) => {
                panic!("{a:?}");
            }
            rhai::EvalAltResult::ErrorVariableExists(a, b) => {
                panic!("{a:?} {b:?}");
            }
            rhai::EvalAltResult::ErrorVariableNotFound(a, b) => {
                panic!("{a:?} {b:?}");
            }
            _ => {
                panic!();
            }
        }
    }

    fn new_orchestrator() -> Orchestrator {
        Orchestrator::new()
    }

    fn new_synth() -> Rrc<dyn IsMidiInstrument> {
        welsh::Synth::new_wrapped_with(0, 44100, SynthPatch::by_name(&welsh::PatchName::Piano))
    }

    fn new_sequencer(orchestrator: &mut Orchestrator) -> Rrc<MidiTickSequencer> {
        let r = rrc(MidiTickSequencer::new());
        let clock_watcher = rrc_clone(&r);
        orchestrator.register_clock_watcher(None, clock_watcher);
        r
    }

    fn new_bitcrusher() -> Rrc<Bitcrusher> {
        Bitcrusher::new_wrapped_with(8)
    }

    #[allow(dead_code)]
    fn new_limiter() -> Rrc<Limiter> {
        Limiter::new_wrapped_with(0.0, 1.0)
    }

    fn register_root_audio_source(orchestrator: &mut Orchestrator, device: Rrc<dyn SourcesAudio>) {
        // TODO: detect duplicate adds
        orchestrator.register_audio_source(None, rrc_clone(&device));
        orchestrator.add_main_mixer_source(rrc_downgrade(&device));
    }

    fn register_root_midi_instrument(
        orchestrator: &mut Orchestrator,
        device: Rrc<dyn IsMidiInstrument>,
    ) {
        // TODO: detect duplicate adds
        let audio_source = rrc_clone(&device);
        orchestrator.register_audio_source(None, audio_source);
        let device = rrc_downgrade(&device);
        orchestrator.add_main_mixer_source(device);
    }

    fn load_midi_file(sequencer: Rrc<MidiTickSequencer>, filename: &str) {
        let data = std::fs::read(filename).unwrap();
        MidiSmfReader::program_sequencer(&data, &mut sequencer.borrow_mut());
    }

    fn connect_audio_sink_to_source(sink: Rrc<dyn SinksAudio>, source: Rrc<dyn SourcesAudio>) {
        // TODO: detect duplicate adds
        sink.borrow_mut().add_audio_source(rrc_downgrade(&source));
    }

    fn play(orchestrator: &mut Orchestrator) {
        if let Ok(performance) = orchestrator.perform() {
            if let Ok(_result) = IOHelper::send_performance_to_file(performance, "out.wav") {
                // yay
            } else {
                panic!("rats");
            }
        } else {
            panic!("oh no");
        }
    }

    fn add_midi_sink(upstream: Rrc<dyn SourcesMidi>, downstream: Rrc<dyn SinksMidi>, channel: i64) {
        upstream
            .borrow_mut()
            .add_midi_sink(channel as MidiChannel, rrc_downgrade(&downstream));
    }

    fn add_midi_sink_as_midi_instrument(
        upstream: Rrc<dyn SourcesMidi>,
        downstream: Rrc<dyn IsMidiInstrument>,
        channel: i64,
    ) {
        let sink = rrc_downgrade(&downstream);
        upstream
            .borrow_mut()
            .add_midi_sink(channel as MidiChannel, sink);
    }

    fn add_midi_sink_to_midi_sequencer(
        upstream: Rrc<MidiTickSequencer>,
        downstream: Rrc<dyn SinksMidi>,
        channel: i64,
    ) {
        upstream
            .borrow_mut()
            .add_midi_sink(channel as MidiChannel, rrc_downgrade(&downstream));
    }

    fn add_midi_sink_as_midi_instrument_to_midi_sequencer(
        upstream: Rrc<MidiTickSequencer>,
        downstream: Rrc<dyn IsMidiInstrument>,
        channel: i64,
    ) {
        let sink = rrc_downgrade(&downstream);
        upstream
            .borrow_mut()
            .add_midi_sink(channel as MidiChannel, sink);
    }

    fn register_methods(&mut self) {
        self.engine
            .register_type_with_name::<Orchestrator>("Orchestrator")
            .register_type_with_name::<welsh::Synth>("Synth")
            .register_fn("Orchestrator", Self::new_orchestrator)
            .register_fn("Synth", Self::new_synth)
            .register_fn("add_audio_source", Self::register_root_audio_source)
            .register_fn("add_audio_source", Self::register_root_midi_instrument)
            .register_fn("Sequencer", Self::new_sequencer)
            .register_fn("load_midi_file", Self::load_midi_file)
            .register_fn("add_midi_sink", Self::add_midi_sink)
            .register_fn("add_midi_sink", Self::add_midi_sink_to_midi_sequencer)
            .register_fn("add_midi_sink", Self::add_midi_sink_as_midi_instrument)
            .register_fn(
                "add_midi_sink",
                Self::add_midi_sink_as_midi_instrument_to_midi_sequencer,
            )
            .register_fn("new_bitcrusher", Self::new_bitcrusher)
            .register_fn("add_audio_source", Self::connect_audio_sink_to_source)
            .register_fn("play", Self::play);
    }
}
