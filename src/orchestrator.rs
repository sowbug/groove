use crate::{
    clock::{Clock, WatchedClock},
    common::{rrc_clone, rrc_downgrade, MonoSample, Rrc, Ww, MONO_SAMPLE_SILENCE},
    controllers::{BigMessage, ControlPath},
    effects::mixer::Mixer,
    id_store::IdStore,
    messages::GrooveMessage,
    midi::{patterns::PatternManager, MidiBus, MidiChannel, MidiMessage, MIDI_CHANNEL_RECEIVE_ALL},
    traits::{
        BoxedEntity, EvenNewerCommand, EvenNewerIsUpdateable, HasUid, Internal, IsEffect,
        IsMidiEffect, MakesIsViewable, MessageBounds, NewIsController, NewUpdateable, SinksAudio,
        SinksMidi, SinksUpdates, SourcesAudio, SourcesMidi, Terminates, WatchesClock,
    },
};
use anyhow::anyhow;
use crossbeam::deque::Worker;
use std::{
    collections::HashMap,
    io::{self, Write},
};

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

#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub(crate) enum OrchestratorMessage {
    #[default]
    None,
    GotAnF32(f32),
    Tick(Clock),
    Midi(Clock, u8, MidiMessage),
}

#[allow(dead_code)]
pub(crate) type BoxedEffect = Box<dyn IsEffect>;
//pub(crate) type BoxedMidiEffect = Box<dyn IsMidiEffect<Message = OrchestratorMessage>>;
#[allow(dead_code)]
pub(crate) type BoxedSourcesAudio = Box<dyn SourcesAudio>;
#[allow(dead_code)]
pub(crate) type Updateable = dyn EvenNewerIsUpdateable<Message = OrchestratorMessage>;
#[allow(dead_code)]
pub(crate) type BoxedUpdateable = Box<Updateable>;

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum Uid {
    OrchestratorUpdateable(usize),
    SourcesAudio(usize),
    IsEffect(usize),
    IsMidiEffect(usize),
}

pub type GrooveOrchestrator = Orchestrator<GrooveMessage>;

#[derive(Debug)]
pub struct Orchestrator<M: MessageBounds> {
    uid: usize,
    store: Store<M>,
    main_mixer_uid: usize,
    pattern_manager: PatternManager, // TODO: one of these things is not like the others
}
impl<M: MessageBounds> NewIsController for Orchestrator<M> {}
impl<M: MessageBounds> NewUpdateable for Orchestrator<M> {
    type Message = M;

    fn update(&mut self, clock: &Clock, message: Self::Message) -> EvenNewerCommand<Self::Message> {
        EvenNewerCommand::batch(self.store.values_mut().fold(
            Vec::new(),
            |mut vec: Vec<EvenNewerCommand<Self::Message>>, item| {
                match item {
                    BoxedEntity::Controller(entity) => {
                        let command = entity.update(clock, message.clone());

                        vec.push(command);
                    }
                    _ => {}
                }
                vec
            },
        ))
    }
}
impl<M: MessageBounds> Terminates for Orchestrator<M> {
    fn is_finished(&self) -> bool {
        true
    }
}
impl<M: MessageBounds> HasUid for Orchestrator<M> {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}
impl<M: MessageBounds> Orchestrator<M> {
    pub const MAIN_MIXER_UVID: &str = "main-mixer";

    pub(crate) fn new() -> Self {
        Default::default()
    }

    pub(crate) fn store(&self) -> &Store<M> {
        &self.store
    }

    pub(crate) fn store_mut(&mut self) -> &mut Store<M> {
        &mut self.store
    }

    pub fn add(&mut self, uvid: Option<&str>, entity: BoxedEntity<M>) -> usize {
        self.store.add(uvid, entity)
    }

    pub(crate) fn get(&mut self, uvid: &str) -> Option<&BoxedEntity<M>> {
        self.store.get_by_uvid(uvid)
    }

    pub(crate) fn get_uid(&mut self, uvid: &str) -> Option<usize> {
        self.store.get_uid(uvid)
    }

    pub(crate) fn link_control(
        &mut self,
        controller_uid: usize,
        target_uid: usize,
        param_name: &str,
    ) {
        if let Some(target) = self.store.get(target_uid) {
            let param_id = match target {
                // TODO: everyone's the same... design issue?
                BoxedEntity::Controller(e) => e.param_id_for_name(param_name),
                BoxedEntity::Effect(e) => e.param_id_for_name(param_name),
                BoxedEntity::Instrument(e) => e.param_id_for_name(param_name),
            };

            if let Some(controller) = self.store.get(controller_uid) {
                if let BoxedEntity::Controller(_controller) = controller {
                    self.store
                        .link_control(controller_uid, target_uid, param_id);
                }
            }
        }
    }

    pub(crate) fn unlink_control(&mut self, controller_uid: usize, target_uid: usize) {
        self.store.unlink_control(controller_uid, target_uid);
    }

    pub(crate) fn patch(&mut self, output_uid: usize, input_uid: usize) -> anyhow::Result<()> {
        // TODO: detect loops

        // Validate that input_uid refers to something that has audio input
        if let Some(input) = self.store.get(input_uid) {
            match input {
                // TODO: there could be things that have audio input but
                // don't transform, like an audio recorder (or technically a
                // main mixer).
                BoxedEntity::Effect(_) => {}
                _ => {
                    return Err(anyhow!("Item {:?} doesn't transform audio", input));
                }
            }
        } else {
            return Err(anyhow!("Couldn't find input_uid {}", input_uid));
        }

        // Validate that source_uid refers to something that outputs audio
        if let Some(output) = self.store.get(output_uid) {
            match output {
                BoxedEntity::Controller(_) => {
                    return Err(anyhow!("Item {:?} doesn't output audio", output));
                }
                _ => {}
            }
        } else {
            return Err(anyhow!("Couldn't find output_uid {}", output_uid));
        }

        // We've passed our checks. Record it.
        self.store.patch(output_uid, input_uid);
        Ok(())
    }

    pub(crate) fn unpatch(&mut self, output_uid: usize, input_uid: usize) {
        self.store.unpatch(output_uid, input_uid);
    }

    pub(crate) fn connect_to_main_mixer(&mut self, source_uid: usize) -> anyhow::Result<()> {
        self.patch(source_uid, self.main_mixer_uid)
    }

    pub(crate) fn are_all_finished(&mut self) -> bool {
        self.store.values().all(|item| match item {
            // TODO: seems like just one kind needs this
            BoxedEntity::Controller(entity) => entity.is_finished(),
            BoxedEntity::Effect(_) => true,
            BoxedEntity::Instrument(_) => true,
        })
    }

    // This (probably) embarrassing method is supposed to be a naturally
    // recursive algorithm expressed iteratively. Yeah, just like the Google
    // interview question. The reason functional recursion wouldn't fly is
    // that the Rust borrow checker won't let us call ourselves if we've
    // already borrowed ourselves &mut, which goes for any of our fields.
    // TODO: simplify
    pub(crate) fn gather_audio(&mut self, clock: &mut Clock) -> MonoSample {
        enum StackEntry {
            ToVisit(usize),
            CollectResultFor(usize),
            Result(MonoSample),
        }
        let mut stack = Vec::new();
        let mut sum = MONO_SAMPLE_SILENCE;
        stack.push(StackEntry::ToVisit(self.main_mixer_uid));

        while let Some(entry) = stack.pop() {
            match entry {
                StackEntry::ToVisit(uid) => {
                    // We've never seen this node before.
                    if let Some(entity) = self.store.get_mut(uid) {
                        match entity {
                            // If it's a leaf, eval it now and add it to the
                            // running sum.
                            BoxedEntity::Instrument(entity) => {
                                sum += entity.source_audio(clock);
                            }
                            // If it's a node, eval its leaves, then eval
                            // its nodes, then process the result.
                            BoxedEntity::Effect(_) => {
                                // Tell us to process sum.
                                stack.push(StackEntry::CollectResultFor(uid));
                                if let Some(source_uids) = self.store.patches(uid) {
                                    let source_uids = source_uids.to_vec();
                                    // Eval leaves
                                    for source_uid in &source_uids {
                                        if let Some(entity) = self.store.get_mut(*source_uid) {
                                            match entity {
                                                BoxedEntity::Controller(_) => {}
                                                BoxedEntity::Effect(_) => {}
                                                BoxedEntity::Instrument(e) => {
                                                    sum += e.source_audio(clock);
                                                }
                                            }
                                        }
                                    }
                                    stack.push(StackEntry::Result(sum));
                                    sum = MONO_SAMPLE_SILENCE;

                                    // Eval nodes
                                    for source_uid in &source_uids {
                                        if let Some(entity) = self.store.get_mut(*source_uid) {
                                            match entity {
                                                BoxedEntity::Controller(_) => {}
                                                BoxedEntity::Effect(_) => {
                                                    stack.push(StackEntry::ToVisit(*source_uid))
                                                }
                                                BoxedEntity::Instrument(_) => {}
                                            }
                                        }
                                    }
                                } else {
                                    // an effect is at the end of a chain.
                                    // This should be harmless (but probably
                                    // confusing for the end user; might
                                    // want to flag it).
                                }
                            }
                            BoxedEntity::Controller(_) => {}
                        }
                    }
                }
                StackEntry::Result(sample) => sum += sample,
                StackEntry::CollectResultFor(uid) => {
                    if let Some(entity) = self.store.get_mut(uid) {
                        match entity {
                            BoxedEntity::Instrument(_) => {}
                            BoxedEntity::Effect(entity) => {
                                stack.push(StackEntry::Result(entity.transform_audio(clock, sum)));
                                sum = MONO_SAMPLE_SILENCE;
                            }
                            BoxedEntity::Controller(_) => {}
                        }
                    }
                }
            }
        }
        sum
    }

    pub(crate) fn connect_midi_upstream(&self, source_uid: usize) {
        dbg!(&source_uid);
    }

    pub(crate) fn connect_midi_downstream(
        &mut self,
        receiver_uid: usize,
        receiver_midi_channel: MidiChannel,
    ) {
        self.store
            .connect_midi_receiver(receiver_uid, receiver_midi_channel);
    }

    pub(crate) fn disconnect_midi_downstream(
        &mut self,
        receiver_uid: usize,
        receiver_midi_channel: MidiChannel,
    ) {
        self.store
            .disconnect_midi_receiver(receiver_uid, receiver_midi_channel);
    }

    pub fn pattern_manager(&self) -> &PatternManager {
        &self.pattern_manager
    }

    pub fn pattern_manager_mut(&mut self) -> &mut PatternManager {
        &mut self.pattern_manager
    }
}
impl<M: MessageBounds> Default for Orchestrator<M> {
    fn default() -> Self {
        let mut r = Self {
            uid: Default::default(),
            store: Default::default(),
            main_mixer_uid: Default::default(),
            pattern_manager: Default::default(),
        };
        let main_mixer = Box::new(Mixer::default());
        r.main_mixer_uid = r.add(
            Some(Orchestrator::<M>::MAIN_MIXER_UVID),
            BoxedEntity::Effect(main_mixer),
        );
        r
    }
}
impl Orchestrator<GrooveMessage> {
    fn send_control_f32(&mut self, clock: &Clock, uid: usize, value: f32) {
        if let Some(links) = self.store.control_links(uid) {
            let links = links.to_vec();
            for (target_uid, param) in links {
                if let Some(target) = self.store.get_mut(target_uid) {
                    match target {
                        // TODO: everyone is the same...
                        BoxedEntity::Controller(e) => {
                            e.update(clock, GrooveMessage::UpdateF32(param, value));
                        }
                        BoxedEntity::Instrument(e) => {
                            e.update(clock, GrooveMessage::UpdateF32(param, value));
                        }
                        BoxedEntity::Effect(e) => {
                            e.update(clock, GrooveMessage::UpdateF32(param, value));
                        }
                    }
                }
            }
        }
    }
}
impl NewUpdateable for Orchestrator<GrooveMessage> {}

#[derive(Debug, Default)]
pub struct GrooveRunner {}
impl GrooveRunner {
    pub fn run(
        &mut self,
        orchestrator: &mut Box<Orchestrator<GrooveMessage>>,
        clock: &mut Clock,
    ) -> anyhow::Result<Vec<MonoSample>> {
        let mut samples = Vec::<MonoSample>::new();
        loop {
            let (sample, done) = self.loop_once(orchestrator, clock);
            if done {
                break;
            }
            samples.push(sample);
        }
        Ok(samples)
    }

    fn loop_once(
        &mut self,
        orchestrator: &mut Box<Orchestrator<GrooveMessage>>,
        clock: &mut Clock,
    ) -> (MonoSample, bool) {
        let command = orchestrator.update(clock, GrooveMessage::Tick);
        match command.0 {
            Internal::None => {}
            Internal::Single(message) => {
                self.handle_message(orchestrator, clock, message);
            }
            Internal::Batch(messages) => {
                for message in messages {
                    self.handle_message(orchestrator, clock, message);
                }
            }
        }
        return if orchestrator.are_all_finished() {
            (MONO_SAMPLE_SILENCE, true)
        } else {
            let sample = orchestrator.gather_audio(clock);
            clock.tick();
            (sample, false)
        };
    }

    fn handle_message(
        &mut self,
        orchestrator: &mut Orchestrator<GrooveMessage>,
        clock: &Clock,
        message: GrooveMessage,
    ) {
        match message {
            GrooveMessage::ControlF32(uid, value) => {
                self.handle_msg_control_f32(orchestrator, clock, uid, value)
            }
            GrooveMessage::Midi(channel, message) => {
                self.handle_msg_midi(orchestrator, clock, channel, message)
            }
            _ => todo!(),
        }
    }

    fn handle_msg_control_f32(
        &mut self,
        orchestrator: &mut Orchestrator<GrooveMessage>,
        clock: &Clock,
        uid: usize,
        value: f32,
    ) {
        if let Some(e) = orchestrator.store.control_links(uid) {
            // TODO: is this clone() necessary? I got lazy because its' a
            // mut borrow of orchestrator inside a non-mut block.
            for (target_uid, param_id) in e.clone() {
                self.send_msg_update_f32(orchestrator, clock, target_uid, param_id, value);
            }
        }
    }

    fn send_msg_update_f32(
        &mut self,
        orchestrator: &mut Orchestrator<GrooveMessage>,
        clock: &Clock,
        target_uid: usize,
        param_id: usize,
        value: f32,
    ) {
        self.send_msg(
            orchestrator,
            clock,
            target_uid,
            GrooveMessage::UpdateF32(param_id, value),
        );
    }

    fn send_msg(
        &mut self,
        orchestrator: &mut Orchestrator<GrooveMessage>,
        clock: &Clock,
        target_uid: usize,
        message: GrooveMessage,
    ) {
        if let Some(target) = orchestrator.store.get_mut(target_uid) {
            match target {
                // TODO: everyone is the same...
                BoxedEntity::Controller(e) => {
                    e.update(clock, message);
                }
                BoxedEntity::Instrument(e) => {
                    e.update(clock, message);
                }
                BoxedEntity::Effect(e) => {
                    e.update(clock, message);
                }
            }
        }
    }

    fn handle_msg_midi(
        &mut self,
        orchestrator: &mut Orchestrator<GrooveMessage>,
        clock: &Clock,
        channel: u8,
        message: MidiMessage,
    ) {
        if let Some(receiver_uids) = orchestrator.store.midi_receivers(channel) {
            for receiver_uid in receiver_uids.to_vec() {
                // TODO: can this loop?
                if let Some(target) = orchestrator.store.get_mut(receiver_uid) {
                    let message = GrooveMessage::Midi(channel, message);
                    match target {
                        BoxedEntity::Controller(e) => {
                            e.update(clock, message);
                        }
                        BoxedEntity::Instrument(e) => {
                            e.update(clock, message);
                        }
                        BoxedEntity::Effect(e) => {
                            e.update(clock, message);
                        }
                    }
                }
            }
        }
    }

    fn send_msg_enable(
        &mut self,
        orchestrator: &mut Orchestrator<GrooveMessage>,
        clock: &Clock,
        target_uid: usize,
        enabled: bool,
    ) {
        self.send_msg(
            orchestrator,
            clock,
            target_uid,
            GrooveMessage::Enable(enabled),
        );
    }
}

#[derive(Debug, Default)]
pub struct Store<M> {
    last_uid: usize,
    uid_to_item: HashMap<usize, BoxedEntity<M>>,

    // Linked controls (one entity controls another entity's parameter)
    uid_to_control: HashMap<usize, Vec<(usize, usize)>>,

    // Patch cables
    audio_sink_uid_to_source_uids: HashMap<usize, Vec<usize>>,

    // MIDI connections
    midi_channel_to_receiver_uid: HashMap<MidiChannel, Vec<usize>>,

    uvid_to_uid: HashMap<String, usize>,
}

impl<M> Store<M> {
    pub(crate) fn add(&mut self, uvid: Option<&str>, mut entity: BoxedEntity<M>) -> usize {
        let uid = self.get_next_uid();
        match entity {
            BoxedEntity::Controller(ref mut e) => e.set_uid(uid),
            BoxedEntity::Effect(ref mut e) => e.set_uid(uid),
            BoxedEntity::Instrument(ref mut e) => e.set_uid(uid),
        }

        self.uid_to_item.insert(uid, entity);
        if let Some(uvid) = uvid {
            self.uvid_to_uid.insert(uvid.to_string(), uid);
        }
        uid
    }

    pub(crate) fn get(&self, uid: usize) -> Option<&BoxedEntity<M>> {
        self.uid_to_item.get(&uid)
    }

    pub(crate) fn get_mut(&mut self, uid: usize) -> Option<&mut BoxedEntity<M>> {
        self.uid_to_item.get_mut(&uid)
    }

    pub(crate) fn get_by_uvid(&self, uvid: &str) -> Option<&BoxedEntity<M>> {
        if let Some(uid) = self.uvid_to_uid.get(uvid) {
            self.uid_to_item.get(&uid)
        } else {
            None
        }
    }

    pub(crate) fn get_by_uvid_mut(&mut self, uvid: &str) -> Option<&mut BoxedEntity<M>> {
        if let Some(uid) = self.uvid_to_uid.get(uvid) {
            self.uid_to_item.get_mut(&uid)
        } else {
            None
        }
    }

    pub(crate) fn get_uid(&self, uvid: &str) -> Option<usize> {
        self.uvid_to_uid.get(uvid).copied()
    }

    pub(crate) fn values(&self) -> std::collections::hash_map::Values<usize, BoxedEntity<M>> {
        self.uid_to_item.values()
    }

    pub(crate) fn values_mut(
        &mut self,
    ) -> std::collections::hash_map::ValuesMut<usize, BoxedEntity<M>> {
        self.uid_to_item.values_mut()
    }

    fn get_next_uid(&mut self) -> usize {
        self.last_uid += 1;
        self.last_uid
    }

    pub(crate) fn link_control(
        &mut self,
        controller_uid: usize,
        target_uid: usize,
        param_id: usize,
    ) {
        self.uid_to_control
            .entry(controller_uid)
            .or_default()
            .push((target_uid, param_id));
    }

    pub(crate) fn unlink_control(&mut self, controller_uid: usize, target_uid: usize) {
        self.uid_to_control
            .entry(controller_uid)
            .or_default()
            .retain(|(uid, _)| *uid != target_uid);
    }

    pub(crate) fn control_links(&self, controller_uid: usize) -> Option<&Vec<(usize, usize)>> {
        self.uid_to_control.get(&controller_uid)
    }

    pub(crate) fn patch(&mut self, output_uid: usize, input_uid: usize) {
        self.audio_sink_uid_to_source_uids
            .entry(input_uid)
            .or_default()
            .push(output_uid);
    }

    pub(crate) fn unpatch(&mut self, output_uid: usize, input_uid: usize) {
        self.audio_sink_uid_to_source_uids
            .entry(input_uid)
            .or_default()
            .retain(|&uid| uid != output_uid);
    }

    pub(crate) fn patches(&self, input_uid: usize) -> Option<&Vec<usize>> {
        self.audio_sink_uid_to_source_uids.get(&input_uid)
    }

    pub(crate) fn midi_receivers(&self, channel: MidiChannel) -> Option<&Vec<usize>> {
        self.midi_channel_to_receiver_uid.get(&channel)
    }

    pub(crate) fn connect_midi_receiver(&mut self, receiver_uid: usize, channel: MidiChannel) {
        self.midi_channel_to_receiver_uid
            .entry(channel)
            .or_default()
            .push(receiver_uid);
    }

    pub(crate) fn disconnect_midi_receiver(&mut self, receiver_uid: usize, channel: MidiChannel) {
        self.midi_channel_to_receiver_uid
            .entry(channel)
            .or_default()
            .retain(|&uid| uid != receiver_uid);
    }
}

/// Orchestrator takes a description of a song and turns it into an in-memory
/// representation that is ready to render to sound.
#[derive(Debug, Default)]
pub struct OldOrchestrator {
    clock: WatchedClock, // owns all WatchesClock
    id_store: IdStore,
    main_mixer: Mixer<GrooveMessage>,
    midi_bus: Rrc<MidiBus>,

    // We don't have owning Vecs for WatchesClock or IsMidiEffect because both
    // of those are owned by WatchedClock.
    audio_sources: Vec<Rrc<dyn SourcesAudio>>,
    effects: Vec<Rrc<dyn IsEffect>>,

    pattern_manager: PatternManager,

    // temp - doesn't belong here. something like a controlcontrolcontroller
    control_paths: Vec<Rrc<ControlPath>>,

    // GUI
    viewable_makers: Vec<Ww<dyn MakesIsViewable>>,
    is_playing: bool,

    // The new system
    last_updateable_id: usize,
    updateables: HashMap<usize, Ww<dyn SinksUpdates>>,
    id_to_updateable_uid: HashMap<String, usize>,
}

impl OldOrchestrator {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn set_watched_clock(&mut self, clock: WatchedClock) {
        self.clock = clock;
    }

    // TODO - pub so that the app can drive slices. Maybe move to IOHelper
    pub fn tick(&mut self) -> (MonoSample, bool) {
        self.is_playing = true;
        let (done, messages) = self.clock.visit_watchers();
        let inner_clock = &self.clock.inner_clock().clone();
        self.update(inner_clock, &messages);
        if done {
            self.is_playing = false;
            return (MONO_SAMPLE_SILENCE, true);
        }
        let sample = self.main_mixer.source_audio(inner_clock);
        self.clock.tick();
        (sample, false)
    }

    fn update(&mut self, clock: &Clock, messages: &[BigMessage]) {
        for message in messages {
            match message {
                BigMessage::SmallMessage(uid, message) => {
                    if let Some(target) = self.updateables.get(uid) {
                        if let Some(target) = target.upgrade() {
                            target.borrow_mut().update(clock, message.clone());
                        }
                    }
                }
            }
        }
    }

    pub fn perform(&mut self) -> anyhow::Result<Performance> {
        let sample_rate = self.clock.inner_clock().sample_rate();
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

    pub fn perform_to_worker(&mut self, worker: &mut Worker<f32>) -> anyhow::Result<()> {
        let sample_rate = self.clock.inner_clock().sample_rate();
        let progress_indicator_quantum: usize = sample_rate / 2;
        let mut next_progress_indicator: usize = progress_indicator_quantum;
        self.clock.reset();
        loop {
            let (sample, done) = self.tick();
            worker.push(sample);
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
        Ok(())
    }

    pub fn add_main_mixer_source(&mut self, device: Ww<dyn SourcesAudio>) {
        self.main_mixer.add_audio_source(device);
    }

    pub fn register_clock_watcher(
        &mut self,
        id: Option<&str>,
        clock_watcher: Rrc<dyn WatchesClock>,
    ) -> String {
        let id = self.id_store.add_clock_watcher_by_id(id, &clock_watcher);
        self.clock.add_watcher(clock_watcher);
        id
    }

    pub fn register_audio_source(
        &mut self,
        id: Option<&str>,
        audio_source: Rrc<dyn SourcesAudio>,
    ) -> String {
        let id = self.id_store.add_audio_source_by_id(id, &audio_source);
        self.audio_sources.push(audio_source);
        id
    }

    pub fn register_effect(&mut self, id: Option<&str>, effect: Rrc<dyn IsEffect>) -> String {
        let id = self.id_store.add_effect_by_id(id, &effect);
        self.effects.push(effect);
        id
    }

    pub fn register_midi_effect(
        &mut self,
        id: Option<&str>,
        midi_effect: Rrc<dyn IsMidiEffect>,
        channel: MidiChannel,
    ) -> String {
        self.connect_to_downstream_midi_bus(
            channel,
            rrc_downgrade::<dyn IsMidiEffect>(&midi_effect),
        );
        self.connect_to_upstream_midi_bus(rrc_clone::<dyn IsMidiEffect>(&midi_effect));

        let id = self.id_store.add_midi_effect_by_id(id, &midi_effect);
        self.clock.add_watcher(midi_effect);
        id
    }

    pub fn register_control_path(&mut self, id: Option<&str>, path: Rrc<ControlPath>) -> String {
        let id = self.id_store.add_control_path_by_id(id, &path);
        self.control_paths.push(path);
        id
    }

    pub fn register_viewable(&mut self, viewable: Rrc<dyn MakesIsViewable>) {
        self.viewable_makers.push(rrc_downgrade(&viewable));
    }

    pub fn register_updateable(&mut self, id: Option<&str>, updateable: Rrc<dyn SinksUpdates>) {
        self.last_updateable_id += 1;
        self.updateables
            .insert(self.last_updateable_id, rrc_downgrade(&updateable));

        if let Some(id) = id {
            self.id_to_updateable_uid
                .insert(id.to_string(), self.last_updateable_id);
        }
    }

    /// If you're connecting an instrument downstream of MidiBus, it means that
    /// the instrument wants to hear what other instruments have to say.
    pub fn connect_to_downstream_midi_bus(
        &mut self,
        channel: MidiChannel,
        instrument: Ww<dyn SinksMidi>,
    ) {
        self.midi_bus
            .borrow_mut()
            .add_midi_sink(channel, instrument);
    }

    /// If you're connecting an instrument upstream of MidiBus, it means that
    /// the instrument has something to say to other instruments.
    pub fn connect_to_upstream_midi_bus(&mut self, instrument: Rrc<dyn SourcesMidi>) {
        instrument.borrow_mut().add_midi_sink(
            MIDI_CHANNEL_RECEIVE_ALL,
            rrc_downgrade::<MidiBus>(&self.midi_bus),
        );
    }

    pub fn audio_source_by(&self, id: &str) -> anyhow::Result<Ww<dyn SourcesAudio>> {
        if let Some(item) = self.id_store.audio_source_by(id) {
            Ok(item)
        } else {
            Err(anyhow::Error::msg(format!(
                "SourcesAudio id {} not found",
                id
            )))
        }
    }

    pub fn audio_sink_by(&self, id: &str) -> anyhow::Result<Ww<dyn SinksAudio>> {
        if id == "main-mixer" {
            panic!("special case this");
        }
        if let Some(item) = self.id_store.audio_sink_by(id) {
            Ok(item)
        } else {
            Err(anyhow::Error::msg(format!(
                "SinksAudio id {} not found",
                id
            )))
        }
    }

    pub fn control_path_by(&self, id: &str) -> anyhow::Result<Ww<ControlPath>> {
        if let Some(item) = self.id_store.control_path_by(id) {
            Ok(item)
        } else {
            Err(anyhow::Error::msg(format!(
                "ControlPath id {} not found",
                id
            )))
        }
    }

    pub fn updateable_by(&self, uid: usize) -> anyhow::Result<Ww<dyn SinksUpdates>> {
        if let Some(item) = self.updateables.get(&uid) {
            Ok(item.clone())
        } else {
            Err(anyhow::Error::msg(format!(
                "Updateable uid {} not found",
                uid
            )))
        }
    }

    pub fn updateable_uid_by(&self, id: &str) -> anyhow::Result<usize> {
        if let Some(uid) = self.id_to_updateable_uid.get(id) {
            Ok(*uid)
        } else {
            Err(anyhow::Error::msg(format!(
                "Updateable UID not found by id {}",
                id
            )))
        }
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

    pub fn viewables(&self) -> &[Ww<dyn MakesIsViewable>] {
        &self.viewable_makers
    }

    pub fn viewables_mut(&mut self) -> &mut Vec<Ww<dyn MakesIsViewable>> {
        &mut self.viewable_makers
    }

    pub fn elapsed_seconds(&self) -> f32 {
        self.clock.inner_clock().seconds()
    }

    pub fn elapsed_beats(&self) -> f32 {
        self.clock.inner_clock().beats()
    }

    pub fn handle_external_midi(&mut self, stamp: u64, channel: u8, message: MidiMessage) {
        dbg!(stamp, channel, message);
    }

    pub fn reset_clock(&mut self) {
        self.clock.reset();
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    pub fn pattern_manager(&self) -> &PatternManager {
        &self.pattern_manager
    }

    pub fn pattern_manager_mut(&mut self) -> &mut PatternManager {
        &mut self.pattern_manager
    }

    pub fn mixer(&self) -> &Mixer<GrooveMessage> {
        &self.main_mixer
    }
}

#[cfg(test)]

pub mod tests {
    use midly::MidiMessage;

    use crate::{
        clock::Clock,
        common::{MonoSample, MONO_SAMPLE_SILENCE},
        messages::tests::TestMessage,
        traits::{BoxedEntity, Internal, NewIsEffect, NewUpdateable},
    };

    use super::Orchestrator;

    #[derive(Debug, Default)]
    pub struct Runner {
        // state_checker is an optional IsEffect that verifies expected state
        // after all each loop iteration's commands have been acted upon.
        //
        // It is an effect because it is intended to monitor another thing's
        // output, which is more like an effect than a controller or an
        // instrument.
        state_checker: Option<Box<dyn NewIsEffect<Message = TestMessage>>>,
    }
    impl Runner {
        pub fn run(
            &mut self,
            orchestrator: &mut Box<Orchestrator<TestMessage>>,
            clock: &mut Clock,
        ) -> anyhow::Result<Vec<MonoSample>> {
            let mut samples = Vec::<MonoSample>::new();
            loop {
                let (sample, done) = self.loop_once(orchestrator, clock);
                if done {
                    break;
                }
                samples.push(sample);
            }
            Ok(samples)
        }

        pub fn loop_once(
            &mut self,
            orchestrator: &mut Box<Orchestrator<TestMessage>>,
            clock: &mut Clock,
        ) -> (MonoSample, bool) {
            let command = orchestrator.update(clock, TestMessage::Tick);
            match command.0 {
                Internal::None => {}
                Internal::Single(message) => {
                    self.handle_message(orchestrator, clock, message);
                }
                Internal::Batch(messages) => {
                    for message in messages {
                        self.handle_message(orchestrator, clock, message);
                    }
                }
            }
            if let Some(checker) = &mut self.state_checker {
                // This one is treated specially in that it is guaranteed to
                // run after everyone else's update() calls for this tick.
                checker.update(clock, TestMessage::Tick);
            }
            return if orchestrator.are_all_finished() {
                (MONO_SAMPLE_SILENCE, true)
            } else {
                let sample = orchestrator.gather_audio(clock);
                clock.tick();
                (sample, false)
            };
        }

        fn handle_message(
            &mut self,
            orchestrator: &mut Orchestrator<TestMessage>,
            clock: &Clock,
            message: TestMessage,
        ) {
            match message {
                TestMessage::ControlF32(uid, value) => {
                    self.handle_msg_control_f32(orchestrator, clock, uid, value)
                }
                TestMessage::Midi(channel, message) => {
                    self.handle_msg_midi(orchestrator, clock, channel, message)
                }
                _ => todo!(),
            }
        }

        fn handle_msg_control_f32(
            &mut self,
            orchestrator: &mut Orchestrator<TestMessage>,
            clock: &Clock,
            uid: usize,
            value: f32,
        ) {
            if let Some(e) = orchestrator.store().control_links(uid) {
                // TODO: is this clone() necessary? I got lazy because its' a
                // mut borrow of orchestrator inside a non-mut block.
                for (target_uid, param_id) in e.clone() {
                    self.send_msg_update_f32(orchestrator, clock, target_uid, param_id, value);
                }
            }
        }

        fn send_msg_update_f32(
            &mut self,
            orchestrator: &mut Orchestrator<TestMessage>,
            clock: &Clock,
            target_uid: usize,
            param_id: usize,
            value: f32,
        ) {
            self.send_msg(
                orchestrator,
                clock,
                target_uid,
                TestMessage::UpdateF32(param_id, value),
            );
        }

        fn send_msg(
            &mut self,
            orchestrator: &mut Orchestrator<TestMessage>,
            clock: &Clock,
            target_uid: usize,
            message: TestMessage,
        ) {
            if let Some(target) = orchestrator.store_mut().get_mut(target_uid) {
                match target {
                    // TODO: everyone is the same...
                    BoxedEntity::Controller(e) => {
                        e.update(clock, message);
                    }
                    BoxedEntity::Instrument(e) => {
                        e.update(clock, message);
                    }
                    BoxedEntity::Effect(e) => {
                        e.update(clock, message);
                    }
                }
            }
        }

        fn handle_msg_midi(
            &mut self,
            orchestrator: &mut Orchestrator<TestMessage>,
            clock: &Clock,
            channel: u8,
            message: MidiMessage,
        ) {
            if let Some(receiver_uids) = orchestrator.store().midi_receivers(channel) {
                for receiver_uid in receiver_uids.to_vec() {
                    // TODO: can this loop?
                    if let Some(target) = orchestrator.store_mut().get_mut(receiver_uid) {
                        let message = TestMessage::Midi(channel, message);
                        match target {
                            BoxedEntity::Controller(e) => {
                                e.update(clock, message);
                            }
                            BoxedEntity::Instrument(e) => {
                                e.update(clock, message);
                            }
                            BoxedEntity::Effect(e) => {
                                e.update(clock, message);
                            }
                        }
                    }
                }
            }
        }

        pub fn send_msg_enable(
            &mut self,
            orchestrator: &mut Orchestrator<TestMessage>,
            clock: &Clock,
            target_uid: usize,
            enabled: bool,
        ) {
            self.send_msg(
                orchestrator,
                clock,
                target_uid,
                TestMessage::Enable(enabled),
            );
        }

        pub(crate) fn add_state_checker(
            &mut self,
            state_checker: Box<dyn NewIsEffect<Message = TestMessage>>,
        ) {
            self.state_checker = Some(state_checker);
        }
    }
}
