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
use crossbeam::deque::Worker;
use dipstick::InputScope;
use rustc_hash::FxHashMap;
use std::{
    io::{self, Write},
    marker::PhantomData,
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

    #[allow(dead_code)]
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
    // interview question. The reason functional recursion wouldn't fly is that
    // the Rust borrow checker won't let us call ourselves if we've already
    // borrowed ourselves &mut, which goes for any of our fields.
    //
    // TODO: simplify
    //
    // TODO: this loop never changes unless the Orchestrator composition does.
    // We should snapshot it the first time and then just whiz through the
    // snapshot the other million times.
    pub(crate) fn gather_audio(&mut self, clock: &mut Clock) -> MonoSample {
        enum StackEntry {
            ToVisit(usize),
            CollectResultFor(usize),
            Result(MonoSample),
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
                            // If it's a node, eval its leaves, then eval
                            // its nodes, then process the result.
                            BoxedEntity::Effect(_) => {
                                // Tell us to process sum.
                                stack.push(StackEntry::CollectResultFor(uid));
                                if let Some(source_uids) = self.store.patches(uid) {
                                    let source_uids = source_uids.to_vec();
                                    // Eval leaves
                                    for source_uid in &source_uids {
                                        debug_assert!(*source_uid != uid);
                                        if let Some(entity) = self.store.get_mut(*source_uid) {
                                            match entity {
                                                BoxedEntity::Controller(_) => {}
                                                BoxedEntity::Effect(_) => {}
                                                BoxedEntity::Instrument(entity) => {
                                                    if let Some(timer) = self
                                                        .metrics
                                                        .entity_audio_times
                                                        .get(&source_uid)
                                                    {
                                                        let start_time = timer.start();
                                                        sum += entity.source_audio(clock);
                                                        timer.stop(start_time);
                                                    } else {
                                                        sum += entity.source_audio(clock);
                                                    }
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
                                                    debug_assert!(
                                                        *source_uid != self.main_mixer_uid
                                                    );
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
                                let value = if let Some(timer) =
                                    self.metrics.entity_audio_times.get(&uid)
                                {
                                    let start_time = timer.start();
                                    let transform_audio = entity.transform_audio(clock, sum);
                                    timer.stop(start_time);
                                    transform_audio
                                } else {
                                    entity.transform_audio(clock, sum)
                                };
                                stack.push(StackEntry::Result(value));
                                sum = MONO_SAMPLE_SILENCE;
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

    // Working rule: if we return a Command, it means that we couldn't handle it
    // ourselves.
    fn update(&mut self, clock: &Clock, message: Self::Message) -> EvenNewerCommand<Self::Message> {
        loop {
            let command = match message {
                Self::Message::Nop => break,
                Self::Message::Tick => self.handle_tick(clock),
                Self::Message::EntityMessage(uid, message) => {
                    self.handle_entity_message(clock, uid, message)
                }
                Self::Message::MidiFromExternal(channel, message) => {
                    self.handle_midi_from_external(clock, channel, message)
                }
            };
            match command.0 {
                Internal::None => EvenNewerCommand::none(),
                Internal::Single(message) => todo!(),
                Internal::Batch(_) => todo!(),
            }
        }
        EvenNewerCommand::none()
    }
}

impl GrooveOrchestrator {
    // Send a tick to every Controller and return their responses.
    fn handle_tick(&mut self, clock: &Clock) -> EvenNewerCommand<GrooveMessage> {
        let mut v = Vec::new();
        for uid in self.store().controller_uids() {
            let command = Self::entity_command_to_groove_command(
                uid,
                self.dispatch_entity_message(clock, uid, EntityMessage::Tick),
            );
            v.push(command);
        }
        EvenNewerCommand::batch(v)
    }

    fn handle_entity_message(
        &mut self,
        clock: &Clock,
        uid: usize,
        message: EntityMessage,
    ) -> EvenNewerCommand<GrooveMessage> {
        let command = if let Some(target) = self.store.get_mut(uid) {
            match target {
                // TODO: everyone is the same...
                BoxedEntity::Controller(e) => e.update(clock, message),
                BoxedEntity::Instrument(e) => e.update(clock, message),
                BoxedEntity::Effect(e) => e.update(clock, message),
            }
        } else {
            EvenNewerCommand::none()
        };
        Self::entity_command_to_groove_command(command)
    }

    fn handle_midi_from_external(
        &mut self,
        clock: &Clock,
        channel: u8,
        message: MidiMessage,
    ) -> EvenNewerCommand<GrooveMessage> {
        let mut receiver_uids = Vec::new();
        if let Some(receiver_all_uids) = self.store.midi_receivers(MIDI_CHANNEL_RECEIVE_ALL) {
            receiver_uids.extend(receiver_all_uids);
        }
        if let Some(receiver_channel_uids) = self.store.midi_receivers(channel) {
            receiver_uids.extend(receiver_channel_uids);
        }
        receiver_uids.dedup();
        if receiver_uids.is_empty() {
            return EvenNewerCommand::none();
        }
        let mut v = Vec::new();
        for uid in receiver_uids.to_vec() {
            // TODO: can this loop?
            if let Some(target) = self.store.get_mut(uid) {
                let message = EntityMessage::Midi(channel, message);
                dbg!(&channel, &message);
                let command = Self::entity_command_to_groove_command(
                    uid,
                    self.handle_entity_message(clock, uid, message),
                );
            }
        }
        EvenNewerCommand::batch(v)
    }

    /// EntityMessages in BoxedEntity update() calls have made it all the way up
    /// to us (or they came from the GUI). Let's look at them and decide what to
    /// do. If we have any work to do, we'll emit it in the form of one or more
    /// GrooveMessages.
    fn dispatch_entity_message(
        &mut self,
        clock: &Clock,
        uid: usize,
        message: EntityMessage,
    ) -> EvenNewerCommand<EntityMessage> {
        match self.handle_entity_message(clock, uid, message).0 {
            Internal::None => EvenNewerCommand::none(),
            Internal::Single(action) => self.handle_entity_message(clock, uid, action),
            Internal::Batch(actions) => {
                let mut v = Vec::new();
                for action in actions {
                    v.push(self.handle_entity_message(clock, uid, action))
                }
                EvenNewerCommand::batch(v)
            }
        }
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
                    EvenNewerCommand::single(GrooveMessage::EntityMessage(uid, *message))
                }))
            }
        }
    }

    fn handle_entity_command(
        &mut self,
        clock: &Clock,
        uid: usize,
        command: EvenNewerCommand<EntityMessage>,
    ) -> EvenNewerCommand<GrooveMessage> {
        match command.0 {
            Internal::None => EvenNewerCommand::none(),
            Internal::Single(action) => Self::entity_command_to_groove_command(
                uid,
                self.handle_entity_message(clock, uid, action),
            ),
            Internal::Batch(actions) => {
                let mut v = Vec::new();
                for action in actions {
                    v.push(self.handle_entity_message(clock, uid, action))
                }
            }
        }
        v
    }

    fn __handle_entity_message(
        &mut self,
        clock: &Clock,
        uid: usize,
        message: EntityMessage,
    ) -> EvenNewerCommand<EntityMessage> {
        match message {
            EntityMessage::Nop => panic!(),
            EntityMessage::Tick => panic!("Tick shouldn't be returned as an update reply."),
            EntityMessage::Midi(channel, message) => {
                // TODO: fix this method name ("dispatch_midi_from_external")
                self.handle_midi_from_external(clock, channel, message)
            }
            EntityMessage::ControlF32(value) => self.dispatch_control_f32(clock, uid, value),
            EntityMessage::UpdateF32(_, _) => {
                panic!("UpdateF32 should be sent only by Orchestrator")
            }

            // TODO: send these to the target indicated by the UID
            EntityMessage::UpdateParam0F32(_) => todo!(),
            EntityMessage::UpdateParam0String(_) => todo!(),
            EntityMessage::UpdateParam0U8(_) => todo!(),
            EntityMessage::UpdateParam1F32(_) => todo!(),
            EntityMessage::UpdateParam1U8(_) => todo!(),

            // TODO: probably junk
            EntityMessage::Enable(_) => todo!(),
            EntityMessage::PatternMessage(_, _) => todo!(),
            EntityMessage::MutePressed(_) => todo!(),
            EntityMessage::EnablePressed(_) => todo!(),
        }
    }

    // NOTE! This returns only Command::none(). Let's see if we can live with
    // UpdateF32 being terminal.
    fn dispatch_control_f32(
        &mut self,
        clock: &Clock,
        uid: usize,
        value: f32,
    ) -> EvenNewerCommand<GrooveMessage> {
        if let Some(e) = self.store.control_links(uid) {
            for (target_uid, param_id) in e.clone() {
                self.handle_entity_message(
                    clock,
                    target_uid,
                    EntityMessage::UpdateF32(param_id, value),
                );
            }
        }
        EvenNewerCommand::none()
    }

    // fn wrap_entity_message_command(
    //     uid: usize,
    //     command: EvenNewerCommand<EntityMessage>,
    // ) -> EvenNewerCommand<GrooveMessage> {
    //     match command.0 {
    //         Internal::None => EvenNewerCommand::none(),
    //         Internal::Single(action) => {
    //             EvenNewerCommand::single(GrooveMessage::EntityMessage(uid, action))
    //         }
    //         Internal::Batch(actions) => EvenNewerCommand::batch(
    //             actions
    //                 .iter()
    //                 .map(|message| {
    //                     EvenNewerCommand::single(GrooveMessage::EntityMessage(uid, message.clone()))
    //                 })
    //                 .into_iter(),
    //         ),
    //     }
    // }

    // fn dispatch_entity_message_and_wrap(
    //     clock: &Clock,
    //     message: &EntityMessage,
    //     entity: &mut BoxedEntity<EntityMessage>,
    // ) -> EvenNewerCommand<GrooveMessage> {
    //     let (uid, command) = match entity {
    //         BoxedEntity::Controller(entity) => {
    //             (entity.uid(), entity.update(clock, message.clone()))
    //         }
    //         BoxedEntity::Effect(entity) => (
    //             entity.uid(),
    //             match message {
    //                 EntityMessage::Tick => {
    //                     // Effects don't get tick messages
    //                     EvenNewerCommand::none()
    //                 }
    //                 _ => entity.update(clock, message.clone()),
    //             },
    //         ),
    //         BoxedEntity::Instrument(entity) => (
    //             entity.uid(),
    //             match message {
    //                 EntityMessage::Tick => {
    //                     // Instruments don't get tick messages
    //                     EvenNewerCommand::none()
    //                 }
    //                 _ => entity.update(clock, message.clone()),
    //             },
    //         ),
    //     };
    //     Self::wrap_entity_message_command(uid, command)
    // }

    // fn dispatch_entity_message(
    //     &mut self,
    //     clock: &Clock,
    //     uid: usize,
    //     message: EntityMessage,
    // ) -> EvenNewerCommand<GrooveMessage> {
    //     match message {
    //         EntityMessage::Nop => {
    //             if let Some(entity) = self.store.get_mut(uid) {
    //                 return Self::dispatch_entity_message_and_wrap(clock, &message, entity);
    //             }
    //         }
    //         EntityMessage::Tick => todo!(),
    //         EntityMessage::Midi(channel, message) => {
    //             // TODO: I think I should forward these upward so they get out to external MIDI
    //             self.dispatch_midi_from_external(clock, channel, message);
    //         }
    //         EntityMessage::ControlF32(value) => {
    //             if let Some(e) = self.store.control_links(uid) {
    //                 for (target_uid, param_id) in e.clone() {
    //                     self.update_entity(
    //                         clock,
    //                         target_uid,
    //                         EntityMessage::UpdateF32(param_id, value),
    //                     );
    //                 }
    //             }
    //         }
    //         EntityMessage::UpdateF32(_, _) => todo!(),
    //         EntityMessage::UpdateParam0F32(_) => todo!(),
    //         EntityMessage::UpdateParam0String(_) => todo!(),
    //         EntityMessage::UpdateParam0U8(_) => todo!(),
    //         EntityMessage::UpdateParam1F32(_) => todo!(),
    //         EntityMessage::UpdateParam1U8(_) => todo!(),
    //         EntityMessage::Enable(_) => todo!(),
    //         EntityMessage::PatternMessage(_, _) => todo!(),
    //         EntityMessage::MutePressed(_) => todo!(),
    //         EntityMessage::EnablePressed(_) => todo!(),
    //     }
    //     EvenNewerCommand::none()
    // }
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
        println!();
        orchestrator.metrics.report();
        Ok(performance)
    }

    pub fn loop_once(
        &mut self,
        orchestrator: &mut Box<GrooveOrchestrator>,
        clock: &mut Clock,
    ) -> (MonoSample, bool) {
        let command = orchestrator.update(clock, GrooveMessage::Tick);
        self.handle_command(orchestrator, clock, command);
        return if orchestrator.are_all_finished() {
            (MONO_SAMPLE_SILENCE, true)
        } else {
            let sample = orchestrator.gather_audio(clock);
            clock.tick();
            (sample, false)
        };
    }

    fn handle_command(
        &mut self,
        orchestrator: &mut Box<Orchestrator<GrooveMessage>>,
        clock: &mut Clock,
        command: EvenNewerCommand<GrooveMessage>,
    ) {
        let mut command = command;
        loop {
            match command.0 {
                Internal::None => {
                    break;
                }
                Internal::Single(message) => {
                    command = self.handle_message(orchestrator, clock, message);
                }
                Internal::Batch(messages) => {
                    if messages.is_empty() {
                        break;
                    }
                    command = EvenNewerCommand::batch(
                        messages
                            .iter()
                            .map(|message| {
                                self.handle_message(orchestrator, clock, message.clone())
                            })
                            .into_iter(),
                    );
                }
            }
        }
    }

    fn handle_message(
        &mut self,
        orchestrator: &mut GrooveOrchestrator,
        clock: &Clock,
        message: GrooveMessage,
    ) -> EvenNewerCommand<GrooveMessage> {
        match message {
            GrooveMessage::Nop => todo!(),
            GrooveMessage::Tick => panic!("Ticks should go only downstream"),
            GrooveMessage::EntityMessage(uid, message) => {
                panic!("EntityMessages shouldn't escape from Orchestrator")
            }
            GrooveMessage::MidiFromExternal(_, _) => {
                panic!("MidiFromExternal should go only downstream")
            }
        }
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

    #[allow(dead_code)]
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
    use midly::MidiMessage;

    use super::Orchestrator;
    use crate::{
        clock::Clock,
        common::{MonoSample, MONO_SAMPLE_SILENCE},
        messages::{tests::TestMessage, EntityMessage},
        traits::{BoxedEntity, EvenNewerCommand, Internal, IsEffect, Updateable},
    };

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
    pub struct Runner {
        // state_checker is an optional IsEffect that verifies expected state
        // after all each loop iteration's commands have been acted upon.
        //
        // It is an effect because it is intended to monitor another thing's
        // output, which is more like an effect than a controller or an
        // instrument.
        state_checker: Option<Box<dyn IsEffect<Message = TestMessage, ViewMessage = TestMessage>>>,
    }
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
            if let Some(e) = orchestrator.store.control_links(uid) {
                // TODO: is this clone() necessary? I got lazy because its' a
                // mut borrow of orchestrator inside a non-mut block.
                for (target_uid, param_id) in e.clone() {
                    self.send_msg_update_f32(orchestrator, clock, target_uid, param_id, value);
                }
            }
        }

        fn handle_msg_midi(
            &mut self,
            orchestrator: &mut TestOrchestrator,
            clock: &Clock,
            channel: u8,
            message: MidiMessage,
        ) {
            if let Some(receiver_uids) = orchestrator.store.midi_receivers(channel) {
                for receiver_uid in receiver_uids.to_vec() {
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

        #[allow(dead_code)]
        pub(crate) fn add_state_checker(
            &mut self,
            state_checker: Box<dyn IsEffect<Message = TestMessage, ViewMessage = TestMessage>>,
        ) {
            self.state_checker = Some(state_checker);
        }
    }
}
