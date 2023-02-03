use super::Performance;
use crate::{
    clock::Clock,
    effects::mixer::Mixer,
    entities::BoxedEntity,
    messages::{EntityMessage, GrooveMessage, MessageBounds},
    metrics::DipstickWrapper,
    midi::{patterns::PatternManager, MidiChannel, MidiMessage},
    settings::ClockSettings,
    traits::{HasUid, Internal, Response, Terminates},
    BeatSequencer, IOHelper, Paths, StereoSample,
};
use anyhow::anyhow;
use dipstick::InputScope;
use groove_macros::Uid;
use rustc_hash::FxHashMap;
use std::{
    io::{self, Write},
    marker::PhantomData,
};

pub type GrooveOrchestrator = Orchestrator<GrooveMessage>;

#[derive(Debug, Uid)]
pub struct Orchestrator<M: MessageBounds> {
    uid: usize,
    title: Option<String>,
    clock_settings: ClockSettings,
    store: Store,

    main_mixer_uid: usize,
    pattern_manager_uid: usize,
    beat_sequencer_uid: usize,

    metrics: DipstickWrapper,
    enable_dev_experiment: bool,
    should_output_perf: bool,

    _phantom: PhantomData<M>,
}
impl<M: MessageBounds> Terminates for Orchestrator<M> {
    fn is_finished(&self) -> bool {
        true
    }
}
impl<M: MessageBounds> Orchestrator<M> {
    // TODO: prefix these to reserve internal ID namespace
    pub const MAIN_MIXER_UVID: &str = "main-mixer";
    pub const PATTERN_MANAGER_UVID: &str = "pattern-manager";
    pub const BEAT_SEQUENCER_UVID: &str = "beat-sequencer";

    pub fn clock_settings(&self) -> &ClockSettings {
        &self.clock_settings
    }

    pub(crate) fn set_clock_settings(&mut self, clock_settings: &ClockSettings) {
        self.clock_settings = clock_settings.clone();
    }

    pub fn set_sample_rate(&mut self, sample_rate: usize) {
        self.clock_settings.set_sample_rate(sample_rate);
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn store_mut(&mut self) -> &mut Store {
        &mut self.store
    }

    fn install_entity_metric(&mut self, uvid: Option<&str>, uid: usize) {
        let name = format!("entity {}", uvid.unwrap_or(format!("uid {uid}").as_str()));
        self.metrics
            .entity_audio_times
            .insert(uid, self.metrics.bucket.timer(name.as_str()));
    }

    pub fn add(&mut self, uvid: Option<&str>, entity: BoxedEntity) -> usize {
        self.metrics.entity_count.mark();
        let uid = self.store.add(uvid, entity);
        self.install_entity_metric(uvid, uid);
        uid
    }

    #[allow(dead_code)]
    pub(crate) fn get(&self, uvid: &str) -> Option<&BoxedEntity> {
        self.store.get_by_uvid(uvid)
    }

    #[allow(dead_code)]
    pub(crate) fn get_mut(&mut self, uvid: &str) -> Option<&mut BoxedEntity> {
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
    ) -> anyhow::Result<()> {
        if let Some(target) = self.store.get(target_uid) {
            if let Some(target) = target.as_controllable() {
                let param_id = target.control_index_for_name(param_name);
                if param_id != usize::MAX {
                    if let Some(entity) = self.store.get(controller_uid) {
                        if entity.as_is_controller().is_some() {
                            self.store
                                .link_control(controller_uid, target_uid, param_id);
                        } else {
                            return Err(anyhow!(
                                "controller ID {} is not of a controller type",
                                controller_uid
                            ));
                        }
                    } else {
                        return Err(anyhow!("couldn't find controller ID {}", controller_uid));
                    }
                } else {
                    return Err(anyhow!(
                        "target ID {} does not have a controllable parameter named `{}`",
                        target_uid,
                        param_name
                    ));
                }
            } else {
                return Err(anyhow!(
                    "target ID {} is not of a controllable type",
                    target_uid
                ));
            }
        } else {
            return Err(anyhow!("couldn't find target ID {}", target_uid));
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) fn unlink_control(&mut self, controller_uid: usize, target_uid: usize) {
        self.store.unlink_control(controller_uid, target_uid);
    }

    pub(crate) fn patch(&mut self, output_uid: usize, input_uid: usize) -> anyhow::Result<()> {
        // TODO: detect loops

        // Validate that input_uid refers to something that has audio input
        if let Some(input) = self.store.get(input_uid) {
            // TODO: there could be things that have audio input but
            // don't transform, like an audio recorder (or technically a
            // main mixer).
            if input.as_is_effect().is_none() {
                // We don't put the IDs in the error message because they're
                // internal, rather than UVID (user-visible IDs), and won't be
                // helpful to understand the problem. It's important for the
                // caller to produce a meaningful error message with UVIDs.
                return Err(anyhow!(
                    "Input device doesn't transform audio and can't be patched from output device"
                ));
            }
        } else {
            return Err(anyhow!("Couldn't find input_uid {input_uid}"));
        }

        // Validate that source_uid refers to something that outputs audio
        if let Some(output) = self.store.get(output_uid) {
            if output.as_is_controller().is_some() {
                return Err(anyhow!(
                    "Output device doesn't output audio and can't be patched into input device"
                ));
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
        self.store.values().all(|item| {
            if let Some(item) = item.as_terminates() {
                item.is_finished()
            } else {
                true
            }
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
    fn gather_audio(&mut self, clock: &Clock) -> StereoSample {
        enum StackEntry {
            ToVisit(usize),
            CollectResultFor(usize, StereoSample),
        }
        let gather_audio_start_time = self.metrics.gather_audio_fn_timer.start();
        let mut stack = Vec::new();
        let mut sum = StereoSample::default();
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
                        // If it's a leaf, eval it now and add it to the
                        // running sum.
                        if let Some(entity) = entity.as_is_instrument_mut() {
                            if let Some(timer) = self.metrics.entity_audio_times.get(&uid) {
                                let start_time = timer.start();
                                sum += entity.source_audio(clock);
                                timer.stop(start_time);
                            } else {
                                sum += entity.source_audio(clock);
                            }
                        } else if entity.as_is_effect().is_some() {
                            // If it's a node, push its children on the stack,
                            // then evaluate the result.

                            // Tell us to process sum.
                            stack.push(StackEntry::CollectResultFor(uid, sum));
                            sum = StereoSample::default();
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
                        if let Some(entity) = entity.as_is_effect_mut() {
                            sum = accumulated_sum
                                + if let Some(timer) = self.metrics.entity_audio_times.get(&uid) {
                                    let start_time = timer.start();
                                    let transformed_audio = entity.transform_audio(clock, sum);
                                    timer.stop(start_time);
                                    transformed_audio
                                } else {
                                    entity.transform_audio(clock, sum)
                                };
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
    ) -> Response<EntityMessage> {
        if let Some(target) = self.store.get_mut(uid) {
            target.as_updateable_mut().update(clock, message)
        } else {
            Response::none()
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

    pub fn set_enable_dev_experiment(&mut self, enabled: bool) {
        self.enable_dev_experiment = enabled;
    }

    pub fn set_should_output_perf(&mut self, value: bool) {
        self.should_output_perf = value;
    }

    pub fn beat_sequencer_uid(&self) -> usize {
        self.beat_sequencer_uid
    }

    pub fn main_mixer_uid(&self) -> usize {
        self.main_mixer_uid
    }

    pub fn pattern_manager_uid(&self) -> usize {
        self.pattern_manager_uid
    }

    pub(crate) fn set_title(&mut self, title: Option<String>) {
        self.title = title;
    }
}
impl<M: MessageBounds> Default for Orchestrator<M> {
    fn default() -> Self {
        let mut r = Self {
            uid: Default::default(),
            title: Some("Untitled".to_string()),
            clock_settings: Default::default(),
            store: Default::default(),
            main_mixer_uid: Default::default(),
            pattern_manager_uid: Default::default(),
            beat_sequencer_uid: Default::default(),
            metrics: Default::default(),
            enable_dev_experiment: Default::default(),
            should_output_perf: Default::default(),
            _phantom: Default::default(),
        };
        r.main_mixer_uid = r.add(
            Some(Orchestrator::<M>::MAIN_MIXER_UVID),
            BoxedEntity::Mixer(Box::new(Mixer::default())),
        );
        r.pattern_manager_uid = r.add(
            Some(Orchestrator::<M>::PATTERN_MANAGER_UVID),
            BoxedEntity::PatternManager(Box::new(PatternManager::default())),
        );
        r.beat_sequencer_uid = r.add(
            Some(Orchestrator::<M>::BEAT_SEQUENCER_UVID),
            BoxedEntity::BeatSequencer(Box::new(BeatSequencer::default())),
        );
        r.connect_midi_upstream(r.beat_sequencer_uid);

        r
    }
}
impl GrooveOrchestrator {
    //type Message = GrooveMessage;

    pub(crate) fn update(
        &mut self,
        clock: &Clock,
        message: GrooveMessage,
    ) -> Response<GrooveMessage> {
        let mut unhandled_commands = Vec::new();
        let mut commands = Vec::new();
        commands.push(Response::single(message.clone()));
        while let Some(command) = commands.pop() {
            let mut messages = Vec::new();
            match command.0 {
                Internal::None => {}
                Internal::Single(action) => messages.push(action),
                Internal::Batch(actions) => messages.extend(actions),
            }
            while let Some(message) = messages.pop() {
                match message {
                    GrooveMessage::Nop => {}
                    GrooveMessage::Tick => {
                        commands.push(self.handle_tick(clock));
                    }
                    GrooveMessage::EntityMessage(uid, message) => match message {
                        EntityMessage::Nop => panic!("this should never be sent"),
                        EntityMessage::Midi(channel, message) => {
                            // We could have pushed this onto the regular
                            // commands vector, and then instead of panicking on
                            // the MidiToExternal match, handle it by pushing it
                            // onto the other vector. It is slightly simpler, if
                            // less elegant, to do it this way.
                            unhandled_commands.push(Response::single(
                                GrooveMessage::MidiToExternal(channel, message),
                            ));
                            commands.push(self.broadcast_midi_message(clock, channel, message));
                        }
                        EntityMessage::ControlF32(value) => {
                            self.dispatch_control_f32(uid, value);
                        }
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
                    GrooveMessage::MidiFromExternal(channel, message) => {
                        commands.push(self.broadcast_midi_message(clock, channel, message));
                    }
                    GrooveMessage::MidiToExternal(_, _) => {
                        panic!("Orchestrator should not handle MidiToExternal");
                    }
                    GrooveMessage::AudioOutput(_) => {
                        panic!("AudioOutput shouldn't exist at this point in the pipeline");
                    }
                    GrooveMessage::OutputComplete => {
                        panic!("OutputComplete shouldn't exist at this point in the pipeline");
                    }
                    GrooveMessage::LoadProject(filename) => {
                        let mut path = Paths::project_path();
                        path.push(filename.clone());
                        if let Ok(settings) =
                            IOHelper::song_settings_from_yaml_file(path.to_str().unwrap())
                        {
                            if let Ok(instance) = settings.instantiate(false) {
                                let title = instance.title.clone();
                                *self = instance;
                                unhandled_commands.push(Response::single(
                                    GrooveMessage::LoadedProject(filename, title),
                                ));
                            }
                        }
                    }
                    GrooveMessage::LoadedProject(_, _) => {
                        panic!("this is only sent by us, never received")
                    }
                }
            }
        }
        if let GrooveMessage::Tick = message {
            unhandled_commands.push(Response::single(GrooveMessage::AudioOutput(
                self.gather_audio(clock),
            )));
            if self.are_all_finished() {
                unhandled_commands.push(Response::single(GrooveMessage::OutputComplete));
            }
        }
        Response::batch(unhandled_commands)
    }
}

impl GrooveOrchestrator {
    // Send a tick to every Controller and return their responses.
    fn handle_tick(&mut self, clock: &Clock) -> Response<GrooveMessage> {
        Response::batch(
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
    ) -> Response<GrooveMessage> {
        Self::entity_command_to_groove_command(
            uid,
            self.send_unhandled_entity_message(clock, uid, message),
        )
    }

    fn entity_command_to_groove_command(
        uid: usize,
        command: Response<EntityMessage>,
    ) -> Response<GrooveMessage> {
        match command.0 {
            Internal::None => Response::none(),
            Internal::Single(message) => {
                Response::single(GrooveMessage::EntityMessage(uid, message))
            }
            Internal::Batch(messages) => Response::batch(messages.iter().map(move |message| {
                Response::single(GrooveMessage::EntityMessage(uid, message.clone()))
            })),
        }
    }

    fn broadcast_midi_message(
        &mut self,
        clock: &Clock,
        channel: u8,
        message: MidiMessage,
    ) -> Response<GrooveMessage> {
        let mut receiver_uids = Vec::new();
        receiver_uids.extend(self.store.midi_receivers(channel));
        receiver_uids.dedup();
        if receiver_uids.is_empty() {
            return Response::none();
        }
        Response::batch(receiver_uids.iter().fold(Vec::new(), |mut v, uid| {
            v.push(self.dispatch_and_wrap_entity_message(
                clock,
                *uid,
                EntityMessage::Midi(channel, message),
            ));
            v
        }))
    }

    fn dispatch_control_f32(&mut self, uid: usize, value: f32) {
        let control_links = self.store.control_links(uid).clone();
        for (target_uid, param_id) in control_links {
            if let Some(entity) = self.store.get_mut(target_uid) {
                if let Some(entity) = entity.as_controllable_mut() {
                    entity.set_by_control_index(param_id, crate::common::F32ControlValue(value));
                }
            }
        }
    }

    pub fn peek_command(command: &Response<GrooveMessage>) -> (StereoSample, bool) {
        let mut debug_matched_audio_output = false;
        let mut sample = StereoSample::default();
        let mut done = false;
        match &command.0 {
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

    // TODO: we're not very crisp about what "done" means. I think the current
    // ordering in this loop is correct so that all the following are true:
    //
    // - It's possible to have a zero-length song (no samples returned)
    // - Calling run() twice won't hand back the same sample, because the clock
    //   always advances.
    //
    // The lack of crispness is that I'm not sure everyone agrees when they
    // should return true in the Terminates trait.
    //
    // TODO: unit-test it!
    pub fn run(&mut self, clock: &mut Clock) -> anyhow::Result<Vec<StereoSample>> {
        let mut samples = Vec::<StereoSample>::new();
        loop {
            let command = self.update(clock, GrooveMessage::Tick);
            let (sample, done) = Self::peek_command(&command);
            clock.tick();
            if done {
                break;
            }
            samples.push(sample);
        }
        Ok(samples)
    }

    pub fn run_performance(
        &mut self,
        clock: &mut Clock,
        quiet: bool,
    ) -> anyhow::Result<Performance> {
        let sample_rate = clock.sample_rate();
        let performance = Performance::new_with(sample_rate);
        let progress_indicator_quantum: usize = sample_rate / 2;
        let mut next_progress_indicator: usize = progress_indicator_quantum;
        clock.reset();
        loop {
            let command = self.update(clock, GrooveMessage::Tick);
            let (sample, done) = Orchestrator::<GrooveMessage>::peek_command(&command);
            if next_progress_indicator <= clock.samples() {
                if !quiet {
                    print!(".");
                    io::stdout().flush().unwrap();
                }
                next_progress_indicator += progress_indicator_quantum;
            }
            clock.tick();
            if done {
                break;
            }
            performance.worker.push(sample);
        }
        if !quiet {
            println!();
        }
        if self.should_output_perf {
            self.metrics.report();
        }
        Ok(performance)
    }
}

#[derive(Debug, Default)]
pub struct Store {
    last_uid: usize,
    uid_to_item: FxHashMap<usize, BoxedEntity>,

    // Linked controls (one entity controls another entity's parameter)
    uid_to_control: FxHashMap<usize, Vec<(usize, usize)>>,

    // Patch cables
    audio_sink_uid_to_source_uids: FxHashMap<usize, Vec<usize>>,

    // MIDI connections
    midi_channel_to_receiver_uid: FxHashMap<MidiChannel, Vec<usize>>,

    uvid_to_uid: FxHashMap<String, usize>,
}

impl Store {
    pub(crate) fn add(&mut self, uvid: Option<&str>, mut entity: BoxedEntity) -> usize {
        let uid = self.get_next_uid();
        entity.as_has_uid_mut().set_uid(uid);

        self.uid_to_item.insert(uid, entity);
        if let Some(uvid) = uvid {
            self.uvid_to_uid.insert(uvid.to_string(), uid);
        }
        uid
    }

    pub(crate) fn get(&self, uid: usize) -> Option<&BoxedEntity> {
        self.uid_to_item.get(&uid)
    }

    pub fn get_mut(&mut self, uid: usize) -> Option<&mut BoxedEntity> {
        self.uid_to_item.get_mut(&uid)
    }

    pub(crate) fn get_by_uvid(&self, uvid: &str) -> Option<&BoxedEntity> {
        if let Some(uid) = self.uvid_to_uid.get(uvid) {
            self.uid_to_item.get(uid)
        } else {
            None
        }
    }

    pub(crate) fn get_by_uvid_mut(&mut self, uvid: &str) -> Option<&mut BoxedEntity> {
        if let Some(uid) = self.uvid_to_uid.get(uvid) {
            self.uid_to_item.get_mut(uid)
        } else {
            None
        }
    }

    pub(crate) fn get_uid(&self, uvid: &str) -> Option<usize> {
        self.uvid_to_uid.get(uvid).copied()
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<usize, BoxedEntity> {
        self.uid_to_item.iter()
    }

    pub(crate) fn values(&self) -> std::collections::hash_map::Values<usize, BoxedEntity> {
        self.uid_to_item.values()
    }

    #[allow(dead_code)]
    pub(crate) fn values_mut(
        &mut self,
    ) -> std::collections::hash_map::ValuesMut<usize, BoxedEntity> {
        self.uid_to_item.values_mut()
    }

    pub(crate) fn controller_uids(&self) -> impl Iterator<Item = usize> {
        self.uid_to_item
            .values()
            .fold(Vec::new(), |mut v, e| {
                if e.as_is_controller().is_some() {
                    v.push(e.as_has_uid().uid());
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
        common::Normal,
        effects::gain::Gain,
        entities::BoxedEntity,
        messages::{tests::TestMessage, EntityMessage},
        traits::{Internal, Response, Updateable},
        utils::{AudioSource, Timer},
        GrooveMessage, StereoSample,
    };
    //    use assert_approx_eq::assert_approx_eq;
    use midly::MidiMessage;

    pub type TestOrchestrator = Orchestrator<TestMessage>;

    impl Updateable for TestOrchestrator {
        type Message = TestMessage;

        fn update(&mut self, clock: &Clock, message: Self::Message) -> Response<Self::Message> {
            let mut unhandled_commands = Vec::new();
            let mut commands = Vec::new();
            commands.push(Response::single(message.clone()));
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
                                unhandled_commands.push(Response::single(
                                    Self::Message::MidiToExternal(channel, message),
                                ));
                                commands.push(self.broadcast_midi_message(clock, channel, message));
                            }
                            EntityMessage::ControlF32(value) => {
                                self.dispatch_control_f32(uid, value);
                            }
                            EntityMessage::Enable(_) => todo!(),
                            EntityMessage::PatternMessage(_, _) => todo!(),
                            EntityMessage::MutePressed(_) => todo!(),
                            EntityMessage::EnablePressed(_) => todo!(),
                            _ => {
                                // EntityMessage::Tick
                                commands.push(
                                    self.dispatch_and_wrap_entity_message(clock, uid, message),
                                );
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
            if let Self::Message::Tick = message {
                unhandled_commands.push(Response::single(Self::Message::AudioOutput(
                    self.gather_audio(clock),
                )));
                if self.are_all_finished() {
                    unhandled_commands.push(Response::single(Self::Message::OutputComplete));
                }
            }
            Response::batch(unhandled_commands)
        }
    }
    impl Orchestrator<TestMessage> {
        pub fn run(&mut self, clock: &mut Clock) -> anyhow::Result<Vec<StereoSample>> {
            let mut samples = Vec::<StereoSample>::new();
            loop {
                // TODO: maybe this should be Commands, with one as a sample, and an
                // occasional one as a done message.
                let command = self.update(clock, TestMessage::Tick);
                let (sample, done) = Self::peek_command(&command);
                clock.tick();
                if done {
                    break;
                }
                samples.push(sample);
            }
            Ok(samples)
        }

        // Send a tick to every Controller and return their responses.
        fn handle_tick(&mut self, clock: &Clock) -> Response<TestMessage> {
            Response::batch(
                self.store()
                    .controller_uids()
                    .fold(Vec::new(), |mut v, uid| {
                        v.push(self.dispatch_and_wrap_entity_message(
                            clock,
                            uid,
                            EntityMessage::Tick,
                        ));
                        v
                    }),
            )
        }

        fn dispatch_and_wrap_entity_message(
            &mut self,
            clock: &Clock,
            uid: usize,
            message: EntityMessage,
        ) -> Response<TestMessage> {
            Self::entity_command_to_groove_command(
                uid,
                self.send_unhandled_entity_message(clock, uid, message),
            )
        }

        fn entity_command_to_groove_command(
            uid: usize,
            command: Response<EntityMessage>,
        ) -> Response<TestMessage> {
            match command.0 {
                Internal::None => Response::none(),
                Internal::Single(message) => {
                    Response::single(TestMessage::EntityMessage(uid, message))
                }
                Internal::Batch(messages) => Response::batch(messages.iter().map(move |message| {
                    Response::single(TestMessage::EntityMessage(uid, message.clone()))
                })),
            }
        }

        fn broadcast_midi_message(
            &mut self,
            clock: &Clock,
            channel: u8,
            message: MidiMessage,
        ) -> Response<TestMessage> {
            let mut receiver_uids = Vec::new();
            receiver_uids.extend(self.store.midi_receivers(channel));
            receiver_uids.dedup();
            if receiver_uids.is_empty() {
                return Response::none();
            }
            Response::batch(receiver_uids.iter().fold(Vec::new(), |mut v, uid| {
                v.push(self.dispatch_and_wrap_entity_message(
                    clock,
                    *uid,
                    EntityMessage::Midi(channel, message),
                ));
                v
            }))
        }

        fn dispatch_control_f32(&mut self, uid: usize, value: f32) {
            let control_links = self.store.control_links(uid).clone();
            for (target_uid, param_id) in control_links {
                if let Some(entity) = self.store.get_mut(target_uid) {
                    if let Some(entity) = entity.as_controllable_mut() {
                        entity
                            .set_by_control_index(param_id, crate::common::F32ControlValue(value));
                    }
                }
            }
        }

        pub fn peek_command(command: &Response<TestMessage>) -> (StereoSample, bool) {
            let mut debug_matched_audio_output = false;
            let mut sample = StereoSample::default();
            let mut done = false;
            match &command.0 {
                Internal::None => {}
                Internal::Single(message) => match message {
                    // Message::Nop - never sent
                    // Message::Tick - Ticks should go only downstream
                    // Message::EntityMessage - shouldn't escape from Orchestrator
                    // Message::MidiFromExternal - should go only downstream
                    // Message::MidiToExternal - ignore and let app handle it
                    // Message::AudioOutput - let app handle it
                    TestMessage::AudioOutput(msg_sample) => {
                        debug_matched_audio_output = true;
                        sample = *msg_sample;
                    }
                    TestMessage::OutputComplete => {
                        done = true;
                    }
                    _ => {}
                },
                Internal::Batch(messages) => {
                    messages.iter().for_each(|message| match message {
                        TestMessage::AudioOutput(msg_sample) => {
                            debug_matched_audio_output = true;
                            sample = *msg_sample;
                        }
                        TestMessage::OutputComplete => {
                            done = true;
                        }
                        _ => {}
                    });
                }
            }
            debug_assert!(debug_matched_audio_output);
            (sample, done)
        }

        pub fn debug_send_msg_enable(&mut self, clock: &Clock, uid: usize, enabled: bool) {
            self.send_unhandled_entity_message(clock, uid, EntityMessage::Enable(enabled));
        }
    }

    #[test]
    fn test_orchestrator_gather_audio_basic() {
        let mut o = Orchestrator::<GrooveMessage>::default();
        let level_1_uid = o.add(
            None,
            BoxedEntity::AudioSource(Box::new(AudioSource::new_with(0.1))),
        );
        let level_2_uid = o.add(
            None,
            BoxedEntity::AudioSource(Box::new(AudioSource::new_with(0.2))),
        );

        let clock = Clock::default();

        // Nothing connected: should output silence.
        assert_eq!(o.gather_audio(&clock), StereoSample::SILENCE);

        assert!(o.connect_to_main_mixer(level_1_uid).is_ok());
        assert!(o
            .gather_audio(&clock)
            .almost_equals(StereoSample::new_from_single_f64(0.1)));

        assert!(o.disconnect_from_main_mixer(level_1_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_2_uid).is_ok());
        assert!(o
            .gather_audio(&clock)
            .almost_equals(StereoSample::new_from_single_f64(0.2)));

        assert!(o.unpatch_all().is_ok());
        assert!(o.connect_to_main_mixer(level_1_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_2_uid).is_ok());
        assert!(o
            .gather_audio(&clock)
            .almost_equals(StereoSample::new_from_single_f64(0.1 + 0.2)));
    }

    #[test]
    fn test_orchestrator_gather_audio() {
        let mut o = Orchestrator::<GrooveMessage>::default();
        let level_1_uid = o.add(
            None,
            BoxedEntity::AudioSource(Box::new(AudioSource::new_with(0.1))),
        );
        let gain_1_uid = o.add(
            None,
            BoxedEntity::Gain(Box::new(Gain::new_with(Normal::new(0.5)))),
        );
        let level_2_uid = o.add(
            None,
            BoxedEntity::AudioSource(Box::new(AudioSource::new_with(0.2))),
        );
        let level_3_uid = o.add(
            None,
            BoxedEntity::AudioSource(Box::new(AudioSource::new_with(0.3))),
        );
        let level_4_uid = o.add(
            None,
            BoxedEntity::AudioSource(Box::new(AudioSource::new_with(0.4))),
        );
        let clock = Clock::default();

        // Nothing connected: should output silence.
        assert_eq!(o.gather_audio(&clock), StereoSample::SILENCE);

        // Just the single-level instrument; should get that.
        assert!(o.connect_to_main_mixer(level_1_uid).is_ok());
        assert!(o
            .gather_audio(&clock)
            .almost_equals(StereoSample::new_from_single_f64(0.1)));

        // Gain alone; that's weird, but it shouldn't explode.
        assert!(o.disconnect_from_main_mixer(level_1_uid).is_ok());
        assert!(o.connect_to_main_mixer(gain_1_uid).is_ok());
        assert_eq!(o.gather_audio(&clock), StereoSample::SILENCE);

        // Disconnect/reconnect and connect just the single-level instrument again.
        assert!(o.disconnect_from_main_mixer(gain_1_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_1_uid).is_ok());
        assert!(o
            .gather_audio(&clock)
            .almost_equals(StereoSample::new_from_single_f64(0.1)));

        // Instrument to gain should result in (instrument x gain).
        assert!(o.unpatch_all().is_ok());
        assert!(o
            .patch_chain_to_main_mixer(&[level_1_uid, gain_1_uid])
            .is_ok());
        assert!(o
            .gather_audio(&clock)
            .almost_equals(StereoSample::new_from_single_f64(0.1 * 0.5)));

        assert!(o.connect_to_main_mixer(level_2_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_3_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_4_uid).is_ok());
        assert!(o
            .gather_audio(&clock)
            .almost_equals(StereoSample::new_from_single_f64(
                0.1 * 0.5 + 0.2 + 0.3 + 0.4
            )));

        // Same thing, but inverted order.
        assert!(o.unpatch_all().is_ok());
        assert!(o.connect_to_main_mixer(level_4_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_3_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_2_uid).is_ok());
        assert!(o
            .patch_chain_to_main_mixer(&[level_1_uid, gain_1_uid])
            .is_ok());
        assert!(o
            .gather_audio(&clock)
            .almost_equals(StereoSample::new_from_single_f64(
                0.1 * 0.5 + 0.2 + 0.3 + 0.4
            )));
    }

    #[test]
    fn test_orchestrator_gather_audio_2() {
        let clock = Clock::default();
        let mut o = Orchestrator::<GrooveMessage>::default();
        let piano_1_uid = o.add(
            None,
            BoxedEntity::AudioSource(Box::new(AudioSource::new_with(0.1))),
        );
        let low_pass_1_uid = o.add(
            None,
            BoxedEntity::Gain(Box::new(Gain::new_with(Normal::new(0.2)))),
        );
        let gain_1_uid = o.add(
            None,
            BoxedEntity::Gain(Box::new(Gain::new_with(Normal::new(0.4)))),
        );

        let bassline_uid = o.add(
            None,
            BoxedEntity::AudioSource(Box::new(AudioSource::new_with(0.3))),
        );
        let gain_2_uid = o.add(
            None,
            BoxedEntity::Gain(Box::new(Gain::new_with(Normal::new(0.6)))),
        );

        let synth_1_uid = o.add(
            None,
            BoxedEntity::AudioSource(Box::new(AudioSource::new_with(0.5))),
        );
        let gain_3_uid = o.add(
            None,
            BoxedEntity::Gain(Box::new(Gain::new_with(Normal::new(0.8)))),
        );

        let drum_1_uid = o.add(
            None,
            BoxedEntity::AudioSource(Box::new(AudioSource::new_with(0.7))),
        );

        // First chain.
        assert!(o
            .patch_chain_to_main_mixer(&[piano_1_uid, low_pass_1_uid, gain_1_uid])
            .is_ok());
        let sample_chain_1 = o.gather_audio(&clock);
        assert!(sample_chain_1.almost_equals(StereoSample::new_from_single_f64(0.1 * 0.2 * 0.4)));

        // Second chain.
        assert!(o.unpatch_all().is_ok());
        assert!(o
            .patch_chain_to_main_mixer(&[bassline_uid, gain_2_uid])
            .is_ok());
        let sample_chain_2 = o.gather_audio(&clock);
        assert!(sample_chain_2.almost_equals(StereoSample::new_from_single_f64(0.3 * 0.6)));

        // Third.
        assert!(o.unpatch_all().is_ok());
        assert!(o
            .patch_chain_to_main_mixer(&[synth_1_uid, gain_3_uid])
            .is_ok());
        let sample_chain_3 = o.gather_audio(&clock);
        assert_eq!(sample_chain_3, StereoSample::new_from_single_f64(0.5 * 0.8));

        // Fourth.
        assert!(o.unpatch_all().is_ok());
        assert!(o.patch_chain_to_main_mixer(&[drum_1_uid]).is_ok());
        let sample_chain_4 = o.gather_audio(&clock);
        assert!(sample_chain_4.almost_equals(StereoSample::new_from_single_f64(0.7)));

        // Now start over and successively add. This is first and second chains together.
        assert!(o.unpatch_all().is_ok());
        assert!(o
            .patch_chain_to_main_mixer(&[piano_1_uid, low_pass_1_uid, gain_1_uid])
            .is_ok());
        assert!(o
            .patch_chain_to_main_mixer(&[bassline_uid, gain_2_uid])
            .is_ok());
        assert_eq!(o.gather_audio(&clock), sample_chain_1 + sample_chain_2);

        // Plus third.
        assert!(o
            .patch_chain_to_main_mixer(&[synth_1_uid, gain_3_uid])
            .is_ok());
        assert_eq!(
            o.gather_audio(&clock),
            sample_chain_1 + sample_chain_2 + sample_chain_3
        );

        // Plus fourth.
        assert!(o.patch_chain_to_main_mixer(&[drum_1_uid]).is_ok());
        assert!(o
            .gather_audio(&clock)
            .almost_equals(sample_chain_1 + sample_chain_2 + sample_chain_3 + sample_chain_4));
    }

    #[test]
    fn test_orchestrator_gather_audio_with_branches() {
        let clock = Clock::default();
        let mut o = Orchestrator::<GrooveMessage>::default();
        let instrument_1_uid = o.add(
            None,
            BoxedEntity::AudioSource(Box::new(AudioSource::new_with(0.1))),
        );
        let instrument_2_uid = o.add(
            None,
            BoxedEntity::AudioSource(Box::new(AudioSource::new_with(0.3))),
        );
        let instrument_3_uid = o.add(
            None,
            BoxedEntity::AudioSource(Box::new(AudioSource::new_with(0.5))),
        );
        let effect_1_uid = o.add(
            None,
            BoxedEntity::Gain(Box::new(Gain::new_with(Normal::new(0.5)))),
        );

        assert!(o.patch_chain_to_main_mixer(&[instrument_1_uid]).is_ok());
        assert!(o.patch_chain_to_main_mixer(&[effect_1_uid]).is_ok());
        assert!(o.patch(instrument_2_uid, effect_1_uid).is_ok());
        assert!(o.patch(instrument_3_uid, effect_1_uid).is_ok());
        assert!(o
            .gather_audio(&clock)
            .almost_equals(StereoSample::new_from_single_f64(0.1 + 0.5 * (0.3 + 0.5))));
    }

    #[test]
    fn test_orchestrator_sample_count_is_accurate() {
        let mut o = Orchestrator::<GrooveMessage>::default();
        let _ = o.add(None, BoxedEntity::Timer(Box::new(Timer::new_with(1.0))));
        let mut clock = Clock::default();
        if let Ok(samples) = o.run(&mut clock) {
            assert_eq!(samples.len(), 44100);
        } else {
            panic!("run failed");
        }
    }

    #[test]
    fn test_patch_fails_with_bad_id() {
        let mut o = Orchestrator::<GrooveMessage>::default();
        assert!(o.patch(3, 2).is_err());
    }
}
