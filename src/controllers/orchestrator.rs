use crate::{
    clock::Clock,
    common::{MonoSample, MONO_SAMPLE_SILENCE},
    effects::mixer::Mixer,
    messages::{GrooveMessage, MessageBounds},
    midi::{patterns::PatternManager, MidiChannel, MidiMessage},
    settings::ClockSettings,
    traits::{
        BoxedEntity, EvenNewerCommand, HasUid, Internal, IsController, Terminates, Updateable,
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

pub type GrooveOrchestrator = Orchestrator<GrooveMessage>;

#[derive(Debug)]
pub struct Orchestrator<M: MessageBounds> {
    uid: usize,
    clock_settings: ClockSettings,
    store: Store<M>,
    main_mixer_uid: usize,
    pattern_manager: PatternManager, // TODO: one of these things is not like the others
}
impl<M: MessageBounds> IsController for Orchestrator<M> {}
impl<M: MessageBounds> Updateable for Orchestrator<M> {
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

    pub fn clock_settings(&self) -> &ClockSettings {
        &self.clock_settings
    }

    pub(crate) fn set_clock_settings(&mut self, clock_settings: &ClockSettings) {
        self.clock_settings = clock_settings.clone();
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

    pub(crate) fn get(&self, uvid: &str) -> Option<&BoxedEntity<M>> {
        self.store.get_by_uvid(uvid)
    }

    pub(crate) fn get_mut(&mut self, uvid: &str) -> Option<&mut BoxedEntity<M>> {
        self.store.get_by_uvid_mut(uvid)
    }

    pub(crate) fn get_uid(&self, uvid: &str) -> Option<usize> {
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
            clock_settings: Default::default(),
            store: Default::default(),
            main_mixer_uid: Default::default(),
            pattern_manager: Default::default(), // TODO: this should be added like main_mixer
        };
        let main_mixer = Box::new(Mixer::default());
        r.main_mixer_uid = r.add(
            Some(Orchestrator::<M>::MAIN_MIXER_UVID),
            BoxedEntity::Effect(main_mixer),
        );
        r
    }
}
impl Updateable for GrooveOrchestrator {}

#[derive(Debug, Default)]
pub struct GrooveRunner {}
impl GrooveRunner {
    pub fn run(
        &mut self,
        orchestrator: &mut Box<GrooveOrchestrator>,
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

    pub fn run_performance(
        &mut self,
        orchestrator: &mut Box<GrooveOrchestrator>,
        clock: &mut Clock,
    ) -> anyhow::Result<Performance> {
        let sample_rate = orchestrator.clock_settings().sample_rate();
        let performance = Performance::new_with(sample_rate);
        let progress_indicator_quantum: usize = sample_rate / 2;
        let mut next_progress_indicator: usize = progress_indicator_quantum;
        clock.reset();
        loop {
            let (sample, done) = self.loop_once(orchestrator, clock);
            if next_progress_indicator <= clock.samples() {
                print!(".");
                io::stdout().flush().unwrap();
                next_progress_indicator += progress_indicator_quantum;
            }
            if done {
                break;
            }
            performance.worker.push(sample);
        }
        Ok(performance)
    }

    fn loop_once(
        &mut self,
        orchestrator: &mut Box<GrooveOrchestrator>,
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
        orchestrator: &mut GrooveOrchestrator,
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
        orchestrator: &mut GrooveOrchestrator,
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
        orchestrator: &mut GrooveOrchestrator,
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
        orchestrator: &mut GrooveOrchestrator,
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
        orchestrator: &mut GrooveOrchestrator,
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
        orchestrator: &mut GrooveOrchestrator,
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

    #[allow(dead_code)]
    pub(crate) fn debug_dump_profiling(&self) {
        println!("last_uid: {}", self.last_uid);
        println!("uid_to_item: {}", self.uid_to_item.len());
        println!("uid_to_control: {}", self.uid_to_control.len());
        println!(
            "audio_sink_uid_to_source_uids: {}",
            self.audio_sink_uid_to_source_uids.len()
        );
        println!(
            "midi_channel_to_receiver_uid: {}",
            self.midi_channel_to_receiver_uid.len()
        );
        println!("uvid_to_uid: {}", self.uvid_to_uid.len());
    }

}

#[cfg(test)]
pub mod tests {
    use super::Orchestrator;
    use crate::{
        clock::Clock,
        common::{MonoSample, MONO_SAMPLE_SILENCE},
        messages::tests::TestMessage,
        traits::{BoxedEntity, Internal, IsEffect, Updateable},
    };
    use midly::MidiMessage;

    #[derive(Debug, Default)]
    pub struct Runner {
        // state_checker is an optional IsEffect that verifies expected state
        // after all each loop iteration's commands have been acted upon.
        //
        // It is an effect because it is intended to monitor another thing's
        // output, which is more like an effect than a controller or an
        // instrument.
        state_checker: Option<Box<dyn IsEffect<Message = TestMessage>>>,
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
            state_checker: Box<dyn IsEffect<Message = TestMessage>>,
        ) {
            self.state_checker = Some(state_checker);
        }
    }
}
