use super::Performance;
use crate::{
    clock::Clock,
    common::{MonoSample, MONO_SAMPLE_SILENCE},
    effects::mixer::Mixer,
    messages::{EntityMessage, GrooveMessage, MessageBounds},
    metrics::DipstickWrapper,
    midi::{patterns::PatternManager, MidiChannel, MidiMessage},
    settings::ClockSettings,
    traits::{
        BoxedEntity, EvenNewerCommand, HasUid, Internal, IsController, Terminates, Updateable,
    },
    MIDI_CHANNEL_RECEIVE_ALL,
};
use anyhow::anyhow;
use dipstick::InputScope;
use rustc_hash::FxHashMap;
use std::{
    io::{self, Write},
    marker::PhantomData,
};

pub type GrooveOrchestrator = Orchestrator<GrooveMessage>;

#[derive(Debug)]
pub struct Orchestrator<M: MessageBounds> {
    uid: usize,
    clock_settings: ClockSettings,
    store: Store<EntityMessage>,
    main_mixer_uid: usize,
    pattern_manager: PatternManager, // TODO: one of these things is not like the others

    metrics: DipstickWrapper,
    enable_dev_experiment: bool,

    _phantom: PhantomData<M>,
}
impl<M: MessageBounds> IsController for Orchestrator<M> {}
impl<M: MessageBounds> Updateable for Orchestrator<M> {
    default type Message = M;

    #[allow(unused_variables)]
    default fn update(
        &mut self,
        clock: &Clock,
        message: Self::Message,
    ) -> EvenNewerCommand<Self::Message> {
        todo!()
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

    #[allow(dead_code)]
    pub(crate) fn new() -> Self {
        Default::default()
    }

    pub fn clock_settings(&self) -> &ClockSettings {
        &self.clock_settings
    }

    pub(crate) fn set_clock_settings(&mut self, clock_settings: &ClockSettings) {
        self.clock_settings = clock_settings.clone();
    }

    #[allow(dead_code)]
    pub(crate) fn store(&self) -> &Store<EntityMessage> {
        &self.store
    }

    #[allow(dead_code)]
    pub(crate) fn store_mut(&mut self) -> &mut Store<EntityMessage> {
        &mut self.store
    }

    fn install_entity_metric(&mut self, uvid: Option<&str>, uid: usize) {
        let name = format!("entity {}", uvid.unwrap_or(format!("uid {}", uid).as_str()));
        self.metrics
            .entity_audio_times
            .insert(uid, self.metrics.bucket.timer(name.as_str()));
    }

    pub fn add(&mut self, uvid: Option<&str>, entity: BoxedEntity<EntityMessage>) -> usize {
        self.metrics.entity_count.mark();
        let uid = self.store.add(uvid, entity);
        self.install_entity_metric(uvid, uid);
        uid
    }

    #[allow(dead_code)]
    pub(crate) fn get(&self, uvid: &str) -> Option<&BoxedEntity<EntityMessage>> {
        self.store.get_by_uvid(uvid)
    }

    #[allow(dead_code)]
    pub(crate) fn get_mut(&mut self, uvid: &str) -> Option<&mut BoxedEntity<EntityMessage>> {
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

    #[allow(dead_code)]
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

    /// Given a slice of entity_uids, patches them as a chain to the main mixer,
    /// with the first item being the farthest from the mixer, and the last's
    /// output plugged directly into the mixer.
    ///
    /// TODO: when we get more interactive, we'll need to think more
    /// transactionally, and validate the whole chain before plugging in
    /// anything.
    #[allow(dead_code)]
    pub(crate) fn patch_chain_to_main_mixer(
        &mut self,
        entity_uids: &[usize],
    ) -> anyhow::Result<()> {
        let mut previous_entity_uid = None;
        for &entity_uid in entity_uids {
            if let Some(previous_uid) = previous_entity_uid {
                self.patch(previous_uid, entity_uid)?;
            }
            previous_entity_uid = Some(entity_uid);
        }
        if let Some(previous_uid) = previous_entity_uid {
            self.patch(previous_uid, self.main_mixer_uid)?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) fn unpatch(&mut self, output_uid: usize, input_uid: usize) -> anyhow::Result<()> {
        self.store.unpatch(output_uid, input_uid);
        Ok(()) // TODO: do we ever care about this result?
    }

    pub(crate) fn connect_to_main_mixer(&mut self, source_uid: usize) -> anyhow::Result<()> {
        self.patch(source_uid, self.main_mixer_uid)
    }

    #[allow(dead_code)]
    pub(crate) fn disconnect_from_main_mixer(&mut self, source_uid: usize) -> anyhow::Result<()> {
        self.unpatch(source_uid, self.main_mixer_uid)
    }

    #[allow(dead_code)]
    pub(crate) fn unpatch_all(&mut self) -> anyhow::Result<()> {
        self.store.unpatch_all()
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
    // interview question. The reason functional recursion wouldn't fly is that
    // the Rust borrow checker won't let us call ourselves if we've already
    // borrowed ourselves &mut, which goes for any of our fields.
    //
    // TODO: this loop never changes unless the Orchestrator composition does.
    // We should snapshot it the first time and then just whiz through the
    // snapshot the other million times.
    //
    // The basic idea: start by pushing the root node as a to-visit onto the
    // stack. Then loop and process the top item on the stack. For a to-visit,
    // either it's a leaf (eval add to the running sum), or it's a node (push a
    // marker with the current sum, then push the children as to-visit). When a
    // marker pops up, eval with the current sum (nodes are effects, so they
    // take an input), then add to the running sum.
    fn gather_audio(&mut self, clock: &Clock) -> MonoSample {
        enum StackEntry {
            ToVisit(usize),
            CollectResultFor(usize, MonoSample),
        }
        let gather_audio_start_time = self.metrics.gather_audio_fn_timer.start();
        let mut stack = Vec::new();
        let mut sum = MONO_SAMPLE_SILENCE;
        stack.push(StackEntry::ToVisit(self.main_mixer_uid));

        self.metrics.mark_stack_loop_entry.mark();
        while let Some(entry) = stack.pop() {
            self.metrics.mark_stack_loop_iteration.mark();
            match entry {
                StackEntry::ToVisit(uid) => {
                    // We've never seen this node before.
                    //
                    // I thought about checking for patch cables to determine
                    // whether it's an instrument (leaf) or effect (node). The
                    // hope was to avoid an entity lookup. But we have to look
                    // up the patch cables. So I think it's six of one, a
                    // half-dozen of another.
                    if let Some(entity) = self.store.get_mut(uid) {
                        match entity {
                            // If it's a leaf, eval it now and add it to the
                            // running sum.
                            BoxedEntity::Instrument(entity) => {
                                if let Some(timer) = self.metrics.entity_audio_times.get(&uid) {
                                    let start_time = timer.start();
                                    sum += entity.source_audio(clock);
                                    timer.stop(start_time);
                                } else {
                                    sum += entity.source_audio(clock);
                                }
                            }
                            // If it's a node, push its children on the stack, then evaluate the result.
                            BoxedEntity::Effect(_) => {
                                // Tell us to process sum.
                                stack.push(StackEntry::CollectResultFor(uid, sum));
                                sum = MONO_SAMPLE_SILENCE;
                                if let Some(source_uids) = self.store.patches(uid) {
                                    for &source_uid in &source_uids.to_vec() {
                                        debug_assert!(source_uid != uid);
                                        stack.push(StackEntry::ToVisit(source_uid));
                                    }
                                } else {
                                    // an effect is at the end of a chain. This
                                    // should be harmless (but probably
                                    // confusing for the end user; might want to
                                    // flag it).
                                }
                            }
                            BoxedEntity::Controller(_) => {}
                        }
                    }
                }
                // We're returning to this node after evaluating its children.
                // TODO: it's a shame we have to look up the node twice. I still
                // think it's better to look it up once to avoid the patch-cable
                // lookup for instruments and controllers. And if we're going to
                // optimize for avoiding lookups, we might as well unroll the
                // whole tree and zip through it, as mentioned earlier.
                StackEntry::CollectResultFor(uid, accumulated_sum) => {
                    if let Some(entity) = self.store.get_mut(uid) {
                        match entity {
                            BoxedEntity::Instrument(_) => {}
                            BoxedEntity::Effect(entity) => {
                                sum = accumulated_sum
                                    + if let Some(timer) = self.metrics.entity_audio_times.get(&uid)
                                    {
                                        let start_time = timer.start();
                                        let transform_audio = entity.transform_audio(clock, sum);
                                        timer.stop(start_time);
                                        transform_audio
                                    } else {
                                        entity.transform_audio(clock, sum)
                                    };
                            }
                            BoxedEntity::Controller(_) => {}
                        }
                    }
                }
            }
        }
        self.metrics
            .gather_audio_fn_timer
            .stop(gather_audio_start_time);
        sum
    }

    fn send_unhandled_entity_message(
        &mut self,
        clock: &Clock,
        uid: usize,
        message: EntityMessage,
    ) -> EvenNewerCommand<EntityMessage> {
        if let Some(target) = self.store.get_mut(uid) {
            match target {
                // TODO: everyone is the same...
                BoxedEntity::Controller(e) => e.update(clock, message),
                BoxedEntity::Instrument(e) => e.update(clock, message),
                BoxedEntity::Effect(e) => e.update(clock, message),
            }
        } else {
            EvenNewerCommand::none()
        }
    }

    #[allow(unused_variables)]
    pub(crate) fn connect_midi_upstream(&self, source_uid: usize) {}

    pub fn connect_midi_downstream(
        &mut self,
        receiver_uid: usize,
        receiver_midi_channel: MidiChannel,
    ) {
        self.store
            .connect_midi_receiver(receiver_uid, receiver_midi_channel);
    }

    #[allow(dead_code)]
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

    pub fn set_enable_dev_experiment(&mut self, enabled: bool) {
        self.enable_dev_experiment = enabled;
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
            metrics: Default::default(),
            enable_dev_experiment: Default::default(),
            _phantom: Default::default(),
        };
        let main_mixer = Box::new(Mixer::default());
        r.main_mixer_uid = r.add(
            Some(Orchestrator::<M>::MAIN_MIXER_UVID),
            BoxedEntity::Effect(main_mixer),
        );
        r
    }
}
impl Updateable for GrooveOrchestrator {
    type Message = GrooveMessage;

    fn update(&mut self, clock: &Clock, message: Self::Message) -> EvenNewerCommand<Self::Message> {
        let mut unhandled_commands = Vec::new();
        let mut commands = Vec::new();
        commands.push(EvenNewerCommand::single(message.clone()));
        while let Some(command) = commands.pop() {
            let mut messages = Vec::new();
            match command.0 {
                Internal::None => {}
                Internal::Single(action) => messages.push(action),
                Internal::Batch(actions) => messages.extend(actions),
            }
            while let Some(message) = messages.pop() {
                match message {
                    Self::Message::Nop => {}
                    Self::Message::Tick => {
                        commands.push(self.handle_tick(clock));
                    }
                    Self::Message::EntityMessage(uid, message) => match message {
                        EntityMessage::Nop => panic!("this should never be sent"),
                        EntityMessage::Midi(channel, message) => {
                            // We could have pushed this onto the regular
                            // commands vector, and then instead of panicking on
                            // the MidiToExternal match, handle it by pushing it
                            // onto the other vector. It is slightly simpler, if
                            // less elegant, to do it this way.
                            unhandled_commands.push(EvenNewerCommand::single(
                                GrooveMessage::MidiToExternal(channel, message),
                            ));
                            commands.push(self.broadcast_midi_message(clock, channel, message));
                        }
                        EntityMessage::ControlF32(value) => {
                            commands.push(self.dispatch_control_f32(clock, uid, value));
                        }
                        EntityMessage::UpdateF32(_, _) => {
                            self.send_unhandled_entity_message(clock, uid, message);
                        }
                        EntityMessage::UpdateParam0F32(_) => todo!(),
                        EntityMessage::UpdateParam0String(_) => todo!(),
                        EntityMessage::UpdateParam0U8(_) => todo!(),
                        EntityMessage::UpdateParam1F32(_) => todo!(),
                        EntityMessage::UpdateParam1U8(_) => todo!(),
                        EntityMessage::Enable(_) => todo!(),
                        EntityMessage::PatternMessage(_, _) => todo!(),
                        EntityMessage::MutePressed(_) => todo!(),
                        EntityMessage::EnablePressed(_) => todo!(),
                        _ => {
                            // EntityMessage::Tick
                            commands
                                .push(self.dispatch_and_wrap_entity_message(clock, uid, message));
                        }
                    },
                    Self::Message::MidiFromExternal(channel, message) => {
                        commands.push(self.broadcast_midi_message(clock, channel, message));
                    }
                    Self::Message::MidiToExternal(_, _) => {
                        panic!("Orchestrator should not handle MidiToExternal");
                    }
                    Self::Message::AudioOutput(_) => {
                        panic!("AudioOutput shouldn't exist at this point in the pipeline");
                    }
                    Self::Message::OutputComplete => {
                        panic!("OutputComplete shouldn't exist at this point in the pipeline");
                    }
                }
            }
        }
        if let GrooveMessage::Tick = message {
            unhandled_commands.push(EvenNewerCommand::single(GrooveMessage::AudioOutput(
                self.gather_audio(clock),
            )));
            if self.are_all_finished() {
                unhandled_commands.push(EvenNewerCommand::single(GrooveMessage::OutputComplete));
            }
        }
        EvenNewerCommand::batch(unhandled_commands)
    }
}

impl GrooveOrchestrator {
    // Send a tick to every Controller and return their responses.
    fn handle_tick(&mut self, clock: &Clock) -> EvenNewerCommand<GrooveMessage> {
        EvenNewerCommand::batch(
            self.store()
                .controller_uids()
                .fold(Vec::new(), |mut v, uid| {
                    v.push(self.dispatch_and_wrap_entity_message(clock, uid, EntityMessage::Tick));
                    v
                }),
        )
    }

    fn dispatch_and_wrap_entity_message(
        &mut self,
        clock: &Clock,
        uid: usize,
        message: EntityMessage,
    ) -> EvenNewerCommand<GrooveMessage> {
        Self::entity_command_to_groove_command(
            uid,
            self.send_unhandled_entity_message(clock, uid, message),
        )
    }

    fn entity_command_to_groove_command(
        uid: usize,
        command: EvenNewerCommand<EntityMessage>,
    ) -> EvenNewerCommand<GrooveMessage> {
        match command.0 {
            Internal::None => EvenNewerCommand::none(),
            Internal::Single(message) => {
                EvenNewerCommand::single(GrooveMessage::EntityMessage(uid, message))
            }
            Internal::Batch(messages) => {
                EvenNewerCommand::batch(messages.iter().map(move |message| {
                    EvenNewerCommand::single(GrooveMessage::EntityMessage(uid, message.clone()))
                }))
            }
        }
    }

    fn broadcast_midi_message(
        &mut self,
        clock: &Clock,
        channel: u8,
        message: MidiMessage,
    ) -> EvenNewerCommand<GrooveMessage> {
        let mut receiver_uids = Vec::new();
        receiver_uids.extend(self.store.midi_receivers(MIDI_CHANNEL_RECEIVE_ALL));
        receiver_uids.extend(self.store.midi_receivers(channel));
        receiver_uids.dedup();
        if receiver_uids.is_empty() {
            return EvenNewerCommand::none();
        }
        EvenNewerCommand::batch(receiver_uids.iter().fold(Vec::new(), |mut v, uid| {
            v.push(self.dispatch_and_wrap_entity_message(
                clock,
                *uid,
                EntityMessage::Midi(channel, message),
            ));
            v
        }))
    }

    // NOTE! This returns only Command::none(). Let's see if we can live with
    // UpdateF32 being terminal.
    fn dispatch_control_f32(
        &mut self,
        clock: &Clock,
        uid: usize,
        value: f32,
    ) -> EvenNewerCommand<GrooveMessage> {
        let control_links = self.store.control_links(uid).clone();
        for (target_uid, param_id) in control_links {
            self.dispatch_and_wrap_entity_message(
                clock,
                target_uid,
                EntityMessage::UpdateF32(param_id, value),
            );
        }
        EvenNewerCommand::none()
    }

    pub fn peek_command(command: &EvenNewerCommand<GrooveMessage>) -> (MonoSample, bool) {
        let mut debug_matched_audio_output = false;
        let mut sample = MONO_SAMPLE_SILENCE;
        let mut done = false;
        match &(*command).0 {
            Internal::None => {}
            Internal::Single(message) => match message {
                // GrooveMessage::Nop - never sent
                // GrooveMessage::Tick - Ticks should go only downstream
                // GrooveMessage::EntityMessage - shouldn't escape from Orchestrator
                // GrooveMessage::MidiFromExternal - should go only downstream
                // GrooveMessage::MidiToExternal - ignore and let app handle it
                // GrooveMessage::AudioOutput - let app handle it
                GrooveMessage::AudioOutput(msg_sample) => {
                    debug_matched_audio_output = true;
                    sample = *msg_sample;
                }
                GrooveMessage::OutputComplete => {
                    done = true;
                }
                _ => {}
            },
            Internal::Batch(messages) => {
                messages.iter().for_each(|message| match message {
                    GrooveMessage::AudioOutput(msg_sample) => {
                        debug_matched_audio_output = true;
                        sample = *msg_sample;
                    }
                    GrooveMessage::OutputComplete => {
                        done = true;
                    }
                    _ => {}
                });
            }
        }
        debug_assert!(debug_matched_audio_output);
        (sample, done)
    }
}

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
            // TODO: maybe this should be Commands, with one as a sample, and an
            // occasional one as a done message.
            let command = orchestrator.update(clock, GrooveMessage::Tick);
            let (sample, done) = Orchestrator::peek_command(&command);
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
            let command = orchestrator.update(clock, GrooveMessage::Tick);
            let (sample, done) = Orchestrator::peek_command(&command);
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
        println!();
        orchestrator.metrics.report();
        Ok(performance)
    }
}

#[derive(Debug, Default)]
pub struct Store<M> {
    last_uid: usize,
    uid_to_item: FxHashMap<usize, BoxedEntity<M>>,

    // Linked controls (one entity controls another entity's parameter)
    uid_to_control: FxHashMap<usize, Vec<(usize, usize)>>,

    // Patch cables
    audio_sink_uid_to_source_uids: FxHashMap<usize, Vec<usize>>,

    // MIDI connections
    midi_channel_to_receiver_uid: FxHashMap<MidiChannel, Vec<usize>>,

    uvid_to_uid: FxHashMap<String, usize>,
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

    pub(crate) fn iter(&self) -> std::collections::hash_map::Iter<usize, BoxedEntity<M>> {
        self.uid_to_item.iter()
    }

    pub(crate) fn values(&self) -> std::collections::hash_map::Values<usize, BoxedEntity<M>> {
        self.uid_to_item.values()
    }

    #[allow(dead_code)]
    pub(crate) fn values_mut(
        &mut self,
    ) -> std::collections::hash_map::ValuesMut<usize, BoxedEntity<M>> {
        self.uid_to_item.values_mut()
    }

    pub(crate) fn controller_uids(&self) -> impl Iterator<Item = usize> {
        self.uid_to_item
            .values()
            .fold(Vec::new(), |mut v, e| {
                match e {
                    BoxedEntity::Controller(e) => {
                        v.push(e.uid());
                    }
                    BoxedEntity::Effect(_) => {}
                    BoxedEntity::Instrument(_) => {}
                };
                v
            })
            .into_iter()
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

    pub(crate) fn control_links(&mut self, controller_uid: usize) -> &Vec<(usize, usize)> {
        self.uid_to_control.entry(controller_uid).or_default()
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

    fn unpatch_all(&mut self) -> Result<(), anyhow::Error> {
        self.audio_sink_uid_to_source_uids.clear();
        Ok(())
    }

    pub(crate) fn patches(&self, input_uid: usize) -> Option<&Vec<usize>> {
        self.audio_sink_uid_to_source_uids.get(&input_uid)
    }

    pub(crate) fn midi_receivers(&mut self, channel: MidiChannel) -> &Vec<usize> {
        self.midi_channel_to_receiver_uid
            .entry(channel)
            .or_default()
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
        effects::gain::Gain,
        messages::{tests::TestMessage, EntityMessage},
        traits::{BoxedEntity, EvenNewerCommand, Internal, Updateable},
        utils::AudioSource,
        GrooveMessage,
    };
    use assert_approx_eq::assert_approx_eq;
    use midly::MidiMessage;

    pub type TestOrchestrator = Orchestrator<TestMessage>;

    impl Updateable for TestOrchestrator {
        type Message = TestMessage;

        fn update(
            &mut self,
            clock: &Clock,
            message: Self::Message,
        ) -> EvenNewerCommand<Self::Message> {
            match message {
                Self::Message::Nop => EvenNewerCommand::none(),
                Self::Message::Tick => EvenNewerCommand::batch(self.store.values_mut().fold(
                    Vec::new(),
                    |mut vec: Vec<EvenNewerCommand<Self::Message>>, item| {
                        match item {
                            BoxedEntity::Controller(entity) => {
                                let command = entity.update(clock, EntityMessage::Tick);

                                match command.0 {
                                    Internal::None => vec.push(EvenNewerCommand::none()),
                                    Internal::Single(action) => {
                                        let command = EvenNewerCommand::single(
                                            Self::Message::EntityMessage(entity.uid(), action),
                                        );
                                        vec.push(command);
                                    }
                                    Internal::Batch(actions) => {
                                        let commands = actions.iter().map(|message| {
                                            EvenNewerCommand::single(Self::Message::EntityMessage(
                                                entity.uid(),
                                                message.clone(),
                                            ))
                                        });
                                        vec.push(EvenNewerCommand::batch(commands.into_iter()));
                                    }
                                }
                            }
                            _ => {}
                        }
                        vec
                    },
                )),
                Self::Message::EntityMessage(uid, message) => {
                    if let Some(entity) = self.store.get_mut(uid) {
                        let (uid, command) = match entity {
                            BoxedEntity::Controller(entity) => {
                                (entity.uid(), entity.update(clock, message))
                            }
                            BoxedEntity::Effect(entity) => {
                                (entity.uid(), entity.update(clock, message))
                            }
                            BoxedEntity::Instrument(entity) => {
                                (entity.uid(), entity.update(clock, message))
                            }
                        };
                        let mut vec = Vec::new();
                        match command.0 {
                            Internal::None => vec.push(EvenNewerCommand::none()),
                            Internal::Single(action) => vec.push(EvenNewerCommand::single(
                                Self::Message::EntityMessage(uid, action),
                            )),
                            Internal::Batch(actions) => vec.push(EvenNewerCommand::batch(
                                actions
                                    .iter()
                                    .map(|message| {
                                        EvenNewerCommand::single(Self::Message::EntityMessage(
                                            uid,
                                            message.clone(),
                                        ))
                                    })
                                    .into_iter(),
                            )),
                        }
                        EvenNewerCommand::batch(vec)
                    } else {
                        EvenNewerCommand::none()
                    }
                }
            }
        }
    }

    #[derive(Debug, Default)]
    pub struct Runner {}
    impl Runner {
        pub fn run(
            &mut self,
            orchestrator: &mut Box<TestOrchestrator>,
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
            orchestrator: &mut Box<TestOrchestrator>,
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
            orchestrator: &mut TestOrchestrator,
            clock: &Clock,
            message: TestMessage,
        ) {
            match message {
                TestMessage::Nop => todo!(),
                TestMessage::Tick => todo!(),
                TestMessage::EntityMessage(uid, message) => match message {
                    EntityMessage::ControlF32(value) => {
                        self.handle_msg_control_f32(orchestrator, clock, uid, value)
                    }
                    EntityMessage::Midi(channel, message) => {
                        self.handle_msg_midi(orchestrator, clock, channel, message)
                    }
                    _ => todo!(),
                },
            }
        }

        fn handle_msg_control_f32(
            &mut self,
            orchestrator: &mut TestOrchestrator,
            clock: &Clock,
            uid: usize,
            value: f32,
        ) {
            let vec = orchestrator.store.control_links(uid).clone();
            for (target_uid, param_id) in vec {
                self.send_msg_update_f32(orchestrator, clock, target_uid, param_id, value);
            }
        }

        fn handle_msg_midi(
            &mut self,
            orchestrator: &mut TestOrchestrator,
            clock: &Clock,
            channel: u8,
            message: MidiMessage,
        ) {
            for receiver_uid in orchestrator.store.midi_receivers(channel).to_vec() {
                // TODO: can this loop?
                if let Some(target) = orchestrator.store.get_mut(receiver_uid) {
                    let message = EntityMessage::Midi(channel, message);
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

        fn send_msg_update_f32(
            &mut self,
            orchestrator: &mut TestOrchestrator,
            clock: &Clock,
            target_uid: usize,
            param_id: usize,
            value: f32,
        ) {
            self.send_msg(
                orchestrator,
                clock,
                target_uid,
                EntityMessage::UpdateF32(param_id, value),
            );
        }

        fn send_msg(
            &mut self,
            orchestrator: &mut TestOrchestrator,
            clock: &Clock,
            target_uid: usize,
            message: EntityMessage,
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

        pub fn send_msg_enable(
            &mut self,
            orchestrator: &mut TestOrchestrator,
            clock: &Clock,
            target_uid: usize,
            enabled: bool,
        ) {
            self.send_msg(
                orchestrator,
                clock,
                target_uid,
                EntityMessage::Enable(enabled),
            );
        }
    }

    #[test]
    fn test_orchestrator_gather_audio_basic() {
        let mut o = Orchestrator::<GrooveMessage>::default();
        let level_1_uid = o.add(
            None,
            BoxedEntity::Instrument(Box::new(AudioSource::new_with(0.1))),
        );
        let level_2_uid = o.add(
            None,
            BoxedEntity::Instrument(Box::new(AudioSource::new_with(0.2))),
        );

        let clock = Clock::default();

        // Nothing connected: should output silence.
        assert_eq!(o.gather_audio(&clock), MONO_SAMPLE_SILENCE);

        assert!(o.connect_to_main_mixer(level_1_uid).is_ok());
        assert_eq!(o.gather_audio(&clock), 0.1);

        assert!(o.disconnect_from_main_mixer(level_1_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_2_uid).is_ok());
        assert_eq!(o.gather_audio(&clock), 0.2);

        assert!(o.unpatch_all().is_ok());
        assert!(o.connect_to_main_mixer(level_1_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_2_uid).is_ok());
        assert_eq!(o.gather_audio(&clock), 0.1 + 0.2);
    }

    #[test]
    fn test_orchestrator_gather_audio() {
        let mut o = Orchestrator::<GrooveMessage>::default();
        let level_1_uid = o.add(
            None,
            BoxedEntity::Instrument(Box::new(AudioSource::new_with(0.1))),
        );
        let gain_1_uid = o.add(None, BoxedEntity::Effect(Box::new(Gain::new_with(0.5))));
        let level_2_uid = o.add(
            None,
            BoxedEntity::Instrument(Box::new(AudioSource::new_with(0.2))),
        );
        let level_3_uid = o.add(
            None,
            BoxedEntity::Instrument(Box::new(AudioSource::new_with(0.3))),
        );
        let level_4_uid = o.add(
            None,
            BoxedEntity::Instrument(Box::new(AudioSource::new_with(0.4))),
        );
        let clock = Clock::default();

        // Nothing connected: should output silence.
        assert_eq!(o.gather_audio(&clock), MONO_SAMPLE_SILENCE);

        // Just the single-level instrument; should get that.
        assert!(o.connect_to_main_mixer(level_1_uid).is_ok());
        assert_eq!(o.gather_audio(&clock), 0.1);

        // Gain alone; that's weird, but it shouldn't explode.
        assert!(o.disconnect_from_main_mixer(level_1_uid).is_ok());
        assert!(o.connect_to_main_mixer(gain_1_uid).is_ok());
        assert_eq!(o.gather_audio(&clock), MONO_SAMPLE_SILENCE);

        // Disconnect/reconnect and connect just the single-level instrument again.
        assert!(o.disconnect_from_main_mixer(gain_1_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_1_uid).is_ok());
        assert_eq!(o.gather_audio(&clock), 0.1);

        // Instrument to gain should result in (instrument x gain).
        assert!(o.unpatch_all().is_ok());
        assert!(o
            .patch_chain_to_main_mixer(&vec![level_1_uid, gain_1_uid])
            .is_ok());
        assert_approx_eq!(o.gather_audio(&clock), 0.1 * 0.5);

        assert!(o.connect_to_main_mixer(level_2_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_3_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_4_uid).is_ok());
        assert_approx_eq!(o.gather_audio(&clock), 0.1 * 0.5 + 0.2 + 0.3 + 0.4);

        // Same thing, but inverted order.
        assert!(o.unpatch_all().is_ok());
        assert!(o.connect_to_main_mixer(level_4_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_3_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_2_uid).is_ok());
        assert!(o
            .patch_chain_to_main_mixer(&vec![level_1_uid, gain_1_uid])
            .is_ok());
        assert_approx_eq!(o.gather_audio(&clock), 0.1 * 0.5 + 0.2 + 0.3 + 0.4);
    }

    #[test]
    fn test_orchestrator_gather_audio_2() {
        let clock = Clock::default();
        let mut o = Orchestrator::<GrooveMessage>::default();
        let piano_1_uid = o.add(
            None,
            BoxedEntity::Instrument(Box::new(AudioSource::new_with(0.1))),
        );
        let low_pass_1_uid = o.add(None, BoxedEntity::Effect(Box::new(Gain::new_with(0.2))));
        let gain_1_uid = o.add(None, BoxedEntity::Effect(Box::new(Gain::new_with(0.4))));

        let bassline_uid = o.add(
            None,
            BoxedEntity::Instrument(Box::new(AudioSource::new_with(0.3))),
        );
        let gain_2_uid = o.add(None, BoxedEntity::Effect(Box::new(Gain::new_with(0.6))));

        let synth_1_uid = o.add(
            None,
            BoxedEntity::Instrument(Box::new(AudioSource::new_with(0.5))),
        );
        let gain_3_uid = o.add(None, BoxedEntity::Effect(Box::new(Gain::new_with(0.8))));

        let drum_1_uid = o.add(
            None,
            BoxedEntity::Instrument(Box::new(AudioSource::new_with(0.7))),
        );

        // First chain.
        assert!(o
            .patch_chain_to_main_mixer(&vec![piano_1_uid, low_pass_1_uid, gain_1_uid])
            .is_ok());
        let sample_chain_1 = o.gather_audio(&clock);
        assert_approx_eq!(sample_chain_1, 0.1 * 0.2 * 0.4);

        // Second chain.
        assert!(o.unpatch_all().is_ok());
        assert!(o
            .patch_chain_to_main_mixer(&vec![bassline_uid, gain_2_uid])
            .is_ok());
        let sample_chain_2 = o.gather_audio(&clock);
        assert_approx_eq!(sample_chain_2, 0.3 * 0.6);

        // Third.
        assert!(o.unpatch_all().is_ok());
        assert!(o
            .patch_chain_to_main_mixer(&vec![synth_1_uid, gain_3_uid])
            .is_ok());
        let sample_chain_3 = o.gather_audio(&clock);
        assert_approx_eq!(sample_chain_3, 0.5 * 0.8);

        // Fourth.
        assert!(o.unpatch_all().is_ok());
        assert!(o.patch_chain_to_main_mixer(&vec![drum_1_uid]).is_ok());
        let sample_chain_4 = o.gather_audio(&clock);
        assert_approx_eq!(sample_chain_4, 0.7);

        // Now start over and successively add. This is first and second chains together.
        assert!(o.unpatch_all().is_ok());
        assert!(o
            .patch_chain_to_main_mixer(&vec![piano_1_uid, low_pass_1_uid, gain_1_uid])
            .is_ok());
        assert!(o
            .patch_chain_to_main_mixer(&vec![bassline_uid, gain_2_uid])
            .is_ok());
        assert_approx_eq!(o.gather_audio(&clock), sample_chain_1 + sample_chain_2);

        // Plus third.
        assert!(o
            .patch_chain_to_main_mixer(&vec![synth_1_uid, gain_3_uid])
            .is_ok());
        assert_approx_eq!(
            o.gather_audio(&clock),
            sample_chain_1 + sample_chain_2 + sample_chain_3
        );

        // Plus fourth.
        assert!(o.patch_chain_to_main_mixer(&vec![drum_1_uid]).is_ok());
        assert_approx_eq!(
            o.gather_audio(&clock),
            sample_chain_1 + sample_chain_2 + sample_chain_3 + sample_chain_4
        );
    }

    #[test]
    fn test_orchestrator_gather_audio_with_branches() {
        let clock = Clock::default();
        let mut o = Orchestrator::<GrooveMessage>::default();
        let instrument_1_uid = o.add(
            None,
            BoxedEntity::Instrument(Box::new(AudioSource::new_with(0.1))),
        );
        let instrument_2_uid = o.add(
            None,
            BoxedEntity::Instrument(Box::new(AudioSource::new_with(0.3))),
        );
        let instrument_3_uid = o.add(
            None,
            BoxedEntity::Instrument(Box::new(AudioSource::new_with(0.5))),
        );
        let effect_1_uid = o.add(None, BoxedEntity::Effect(Box::new(Gain::new_with(0.5))));

        assert!(o.patch_chain_to_main_mixer(&vec![instrument_1_uid]).is_ok());
        assert!(o.patch_chain_to_main_mixer(&vec![effect_1_uid]).is_ok());
        assert!(o.patch(instrument_2_uid, effect_1_uid).is_ok());
        assert!(o.patch(instrument_3_uid, effect_1_uid).is_ok());
        assert_eq!(o.gather_audio(&clock), 0.1 + 0.5 * (0.3 + 0.5));
    }
}
