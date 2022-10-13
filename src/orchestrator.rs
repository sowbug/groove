use crate::common::{rrc, MonoSample, Rrc, Ww, MONO_SAMPLE_SILENCE};
use crate::control::ControlPath;
use crate::id_store::IdStore;
use crate::midi::{MidiBus, MidiChannel, MIDI_CHANNEL_RECEIVE_ALL};
use crate::patterns::Pattern;
use crate::settings::song::SongSettings;
use crate::traits::{
    IsEffect, IsMidiEffect, MakesControlSink, SinksAudio, SinksMidi, SourcesAudio, SourcesMidi,
    WatchesClock,
};
use crate::{clock::WatchedClock, effects::mixer::Mixer};
use crossbeam::deque::Worker;
use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;

#[derive(Debug)]
pub struct Performance {
    pub sample_rate: usize,
    pub worker: Worker<MonoSample>,
}

impl Performance {
    pub fn new_with(sample_rate: usize) -> Self {
        Self {
            sample_rate,
            worker: Worker::<MonoSample>::new_fifo(),
        }
    }
}

/// Orchestrator takes a description of a song and turns it into an in-memory representation that is ready to render to sound.
#[derive(Debug)]
pub struct Orchestrator {
    clock: WatchedClock, // owns all WatchesClock
    id_store: IdStore,
    main_mixer: Box<Mixer>,
    midi_bus: Rrc<MidiBus>,

    // We don't have owning Vecs for WatchesClock or IsMidiEffect because
    // both of those are owned by WatchedClock.
    audio_sources: Vec<Rrc<dyn SourcesAudio>>,
    effects: Vec<Rrc<dyn IsEffect>>,

    // temp - doesn't belong here. something like a controlcontrolcontroller
    patterns: Vec<Rrc<Pattern>>,
    control_paths: Vec<Rrc<ControlPath>>,
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self {
            clock: WatchedClock::default(),
            id_store: IdStore::default(),
            main_mixer: Box::new(Mixer::default()),
            midi_bus: rrc(MidiBus::default()),
            audio_sources: Vec::new(),
            effects: Vec::new(),
            patterns: Vec::new(),
            control_paths: Vec::new(),
        }
    }
}

impl Orchestrator {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_with(settings: &SongSettings) -> Self {
        if let Ok(o) = settings.instantiate() {
            o
        } else {
            panic!()
        }
    }

    pub fn set_watched_clock(&mut self, clock: WatchedClock) {
        self.clock = clock;
    }

    fn tick(&mut self) -> (MonoSample, bool) {
        if self.clock.visit_watchers() {
            return (MONO_SAMPLE_SILENCE, true);
        }
        let sample = self.main_mixer.source_audio(self.clock.inner_clock());
        self.clock.tick();
        (sample, false)
    }

    pub fn perform(&mut self) -> anyhow::Result<Performance> {
        let sample_rate = self.clock.inner_clock().settings().sample_rate();
        let performance = Performance::new_with(sample_rate);
        let progress_indicator_quantum: usize = sample_rate / 2;
        let mut next_progress_indicator: usize = progress_indicator_quantum;
        self.clock.reset();
        loop {
            let (sample, done) = self.tick();
            performance.worker.push(sample);
            if next_progress_indicator <= self.clock.inner_clock().samples() {
                print!(".");
                io::stdout().flush().unwrap();
                next_progress_indicator += progress_indicator_quantum;
            }
            if done {
                break;
            }
        }
        println!();
        Ok(performance)
    }

    pub fn add_main_mixer_source(&mut self, device: Ww<dyn SourcesAudio>) {
        self.main_mixer.add_audio_source(device);
    }

    pub fn register_clock_watcher(
        &mut self,
        id: Option<&str>,
        clock_watcher: Rc<RefCell<dyn WatchesClock>>,
    ) -> String {
        let id = self.id_store.add_clock_watcher_by_id(id, &clock_watcher);
        self.clock.add_watcher(clock_watcher);
        id
    }

    pub fn register_audio_source(
        &mut self,
        id: Option<&str>,
        audio_source: Rc<RefCell<dyn SourcesAudio>>,
    ) -> String {
        let id = self.id_store.add_audio_source_by_id(id, &audio_source);
        self.audio_sources.push(audio_source);
        id
    }

    pub fn register_effect(
        &mut self,
        id: Option<&str>,
        effect: Rc<RefCell<dyn IsEffect>>,
    ) -> String {
        let id = self.id_store.add_effect_by_id(id, &effect);
        self.effects.push(effect);
        id
    }

    pub fn register_midi_effect(
        &mut self,
        id: Option<&str>,
        midi_effect: Rc<RefCell<dyn IsMidiEffect>>,
        channel: MidiChannel,
    ) -> String {
        let instrument = Rc::downgrade(&midi_effect);
        self.connect_to_downstream_midi_bus(channel, instrument);
        let instrument = Rc::clone(&midi_effect);
        self.connect_to_upstream_midi_bus(instrument);

        let id = self.id_store.add_midi_effect_by_id(id, &midi_effect);
        self.clock.add_watcher(midi_effect);
        id
    }

    pub fn register_pattern(&mut self, id: Option<&str>, pattern: Rrc<Pattern>) -> String {
        let id = self.id_store.add_pattern_by_id(id, &pattern);
        self.patterns.push(pattern);
        id
    }

    pub fn register_control_path(&mut self, id: Option<&str>, path: Rrc<ControlPath>) -> String {
        let id = self.id_store.add_control_path_by_id(id, &path);
        self.control_paths.push(path);
        id
    }

    /// If you're connecting an instrument downstream of MidiBus, it means that the
    /// instrument wants to hear what other instruments have to say.
    pub fn connect_to_downstream_midi_bus(
        &mut self,
        channel: MidiChannel,
        instrument: Ww<dyn SinksMidi>,
    ) {
        self.midi_bus
            .borrow_mut()
            .add_midi_sink(channel, instrument);
    }

    /// If you're connecting an instrument upstream of MidiBus, it means that the
    /// instrument has something to say to other instruments.
    pub fn connect_to_upstream_midi_bus(&mut self, instrument: Rrc<dyn SourcesMidi>) {
        let sink = Rc::downgrade(&self.midi_bus);
        instrument
            .borrow_mut()
            .add_midi_sink(MIDI_CHANNEL_RECEIVE_ALL, sink);
    }

    pub fn get_audio_source_by_id(&self, id: &str) -> Ww<dyn SourcesAudio> {
        if let Some(item) = self.id_store.get_audio_source_by_id(id) {
            return item;
        }
        panic!("SourcesAudio id {} not found", id);
    }

    pub fn get_audio_sink_by_id(&self, id: &str) -> Ww<dyn SinksAudio> {
        if id == "main-mixer" {
            panic!("special case this");
        }
        if let Some(item) = self.id_store.get_audio_sink_by_id(id) {
            return item;
        }
        panic!("SinksAudio id {} not found", id);
    }

    pub fn get_makes_control_sink_by_id(&self, id: &str) -> Ww<dyn MakesControlSink> {
        if let Some(item) = self.id_store.get_makes_control_sink_by_id(id) {
            return item;
        }
        panic!("MakesControlSink id {} not found", id);
    }

    pub fn get_pattern_by_id(&self, id: &str) -> Ww<Pattern> {
        if let Some(item) = self.id_store.get_pattern_by_id(id) {
            return item;
        }
        panic!("Pattern id {} not found", id);
    }

    pub fn get_control_path_by_id(&self, id: &str) -> Ww<ControlPath> {
        if let Some(item) = self.id_store.get_control_path_by_id(id) {
            return item;
        }
        panic!("ControlPath id {} not found", id);
    }

    //________________________
    // Begin stuff I need for the GUI app
    //________________________

    pub fn bpm(&self) -> f32 {
        self.clock.inner_clock().settings().bpm()
    }

    pub fn set_bpm(&mut self, bpm: f32) {
        // TODO something something https://en.wikipedia.org/wiki/Law_of_Demeter
        self.clock.inner_clock_mut().settings_mut().set_bpm(bpm);
    }

    pub fn main_mixer(&self) -> &dyn SinksAudio {
        &(*self.main_mixer)
    }
}
