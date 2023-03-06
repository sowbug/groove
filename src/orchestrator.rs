use crate::{
    entities::Entity,
    helpers::IOHelper,
    messages::GrooveMessage,
    messages::{Internal, Response},
    metrics::DipstickWrapper,
    utils::Paths,
};
use anyhow::anyhow;
use core::fmt::Debug;
use crossbeam::deque::Worker;
use dipstick::InputScope;
use groove_core::{
    midi::{MidiChannel, MidiMessage},
    time::TimeSignature,
    ParameterType, StereoSample,
};
use groove_entities::{
    controllers::{BeatSequencer, PatternManager},
    effects::Mixer,
    EntityMessage,
};
use groove_macros::Uid;
use rustc_hash::{FxHashMap, FxHashSet};
use std::io::{self, Write};

/// A Performance holds the output of an Orchestrator run.
#[derive(Debug)]
pub struct Performance {
    pub sample_rate: usize,
    pub worker: Worker<StereoSample>,
}

impl Performance {
    pub fn new_with(sample_rate: usize) -> Self {
        Self {
            sample_rate,
            worker: Worker::<StereoSample>::new_fifo(),
        }
    }
}

#[derive(Debug, Uid)]
pub struct Orchestrator {
    uid: usize,
    title: Option<String>,
    store: Store,

    sample_rate: usize,
    time_signature: TimeSignature,
    bpm: ParameterType,

    main_mixer_uid: usize,
    pattern_manager_uid: usize,
    beat_sequencer_uid: usize,

    metrics: DipstickWrapper,
    enable_dev_experiment: bool,
    should_output_perf: bool,

    last_track_samples: Vec<StereoSample>,
    main_mixer_source_uids: FxHashSet<usize>,
    last_samples: FxHashMap<usize, StereoSample>,
}
impl Orchestrator {
    // TODO: prefix these to reserve internal ID namespace
    pub const MAIN_MIXER_UVID: &str = "main-mixer";
    pub const PATTERN_MANAGER_UVID: &str = "pattern-manager";
    pub const BEAT_SEQUENCER_UVID: &str = "beat-sequencer";

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

    pub fn add(&mut self, uvid: Option<&str>, entity: Entity) -> usize {
        self.metrics.entity_count.mark();
        let uid = self.store.add(uvid, entity);
        self.install_entity_metric(uvid, uid);
        uid
    }

    #[allow(dead_code)]
    pub(crate) fn get(&self, uvid: &str) -> Option<&Entity> {
        self.store.get_by_uvid(uvid)
    }

    #[allow(dead_code)]
    pub(crate) fn get_mut(&mut self, uvid: &str) -> Option<&mut Entity> {
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
            let outputs_audio =
                output.as_is_instrument().is_some() || output.as_is_effect().is_some();
            if !outputs_audio {
                return Err(anyhow!(
                    "Output device doesn't output audio and can't be patched into input device"
                ));
            }
        } else {
            return Err(anyhow!("Couldn't find output_uid {}", output_uid));
        }

        // We've passed our checks. Record it.
        self.store.patch(output_uid, input_uid);

        if input_uid == self.main_mixer_uid {
            self.main_mixer_source_uids.insert(output_uid);
        }
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
        if input_uid == self.main_mixer_uid {
            self.main_mixer_source_uids.remove(&input_uid);
        }
        self.store.unpatch(output_uid, input_uid);
        Ok(()) // TODO: do we ever care about this result?
    }

    #[allow(dead_code)]
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
    fn gather_audio(&mut self, samples: &mut [StereoSample]) {
        // TODO: we are wasting work by putting stuff in the last-sample hash
        // map for all but the last iteration of this loop.
        self.last_samples.clear();
        for sample in samples {
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
                                    entity.tick(1);
                                    timer.stop(start_time);
                                } else {
                                    entity.tick(1);
                                }
                                self.last_samples.insert(uid, entity.value());
                                sum += entity.value();
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
                                let entity_value = if let Some(timer) =
                                    self.metrics.entity_audio_times.get(&uid)
                                {
                                    let start_time = timer.start();
                                    let transformed_audio = entity.transform_audio(sum);
                                    timer.stop(start_time);
                                    transformed_audio
                                } else {
                                    entity.transform_audio(sum)
                                };
                                sum = accumulated_sum + entity_value;
                                self.last_samples.insert(uid, entity_value);
                            }
                        }
                    }
                }
            }
            self.metrics
                .gather_audio_fn_timer
                .stop(gather_audio_start_time);
            *sample = sum;
        }
        self.last_track_samples.clear();
        for uid in self.main_mixer_source_uids.iter() {
            if let Some(sample) = self.last_samples.get(uid) {
                self.last_track_samples.push(*sample);
            }
        }
    }

    #[allow(unused_variables)]
    pub(crate) fn connect_midi_upstream(&self, source_uid: usize) {}

    pub fn connect_midi_downstream(
        &mut self,
        receiver_uid: usize,
        receiver_midi_channel: MidiChannel,
    ) {
        if let Some(e) = self.store().get(receiver_uid) {
            if e.as_handles_midi().is_some() {
                self.store
                    .connect_midi_receiver(receiver_uid, receiver_midi_channel);
            } else {
                eprintln!(
                    "Warning: trying to connect device ID {}, but it does not handle MIDI",
                    receiver_uid
                );
            }
        } else {
            eprintln!("Warning: tried to connect nonexistent device to MIDI");
        }
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

    pub fn new_with(sample_rate: usize, bpm: ParameterType) -> Self {
        let time_signature = TimeSignature::default();
        let mut r = Self {
            uid: Default::default(),
            title: Some("Untitled".to_string()),
            sample_rate,
            time_signature,
            bpm,
            store: Default::default(),
            main_mixer_uid: Default::default(),
            pattern_manager_uid: Default::default(),
            beat_sequencer_uid: Default::default(),
            metrics: Default::default(),
            enable_dev_experiment: Default::default(),
            should_output_perf: Default::default(),
            last_track_samples: Default::default(),
            main_mixer_source_uids: Default::default(),
            last_samples: Default::default(),
        };
        r.main_mixer_uid = r.add(
            Some(Orchestrator::MAIN_MIXER_UVID),
            Entity::Mixer(Box::new(Mixer::default())),
        );
        r.pattern_manager_uid = r.add(
            Some(Orchestrator::PATTERN_MANAGER_UVID),
            Entity::PatternManager(Box::new(PatternManager::default())),
        );
        r.beat_sequencer_uid = r.add(
            Some(Orchestrator::BEAT_SEQUENCER_UVID),
            Entity::BeatSequencer(Box::new(BeatSequencer::new_with(sample_rate, bpm))),
        );
        r.connect_midi_upstream(r.beat_sequencer_uid);

        r
    }

    pub(crate) fn update(&mut self, message: GrooveMessage) -> Response<GrooveMessage> {
        let mut unhandled_commands = Vec::new();
        let mut commands = Vec::new();
        commands.push(Response::single(message));
        while let Some(command) = commands.pop() {
            let mut messages = Vec::new();
            match command.0 {
                Internal::None => {}
                Internal::Single(action) => messages.push(action),
                Internal::Batch(actions) => messages.extend(actions),
            }
            while let Some(message) = messages.pop() {
                match message {
                    GrooveMessage::EntityMessage(uid, message) => match message {
                        EntityMessage::Midi(channel, message) => {
                            // We could have pushed this onto the regular
                            // commands vector, and then instead of panicking on
                            // the MidiToExternal match, handle it by pushing it
                            // onto the other vector. It is slightly simpler, if
                            // less elegant, to do it this way.
                            unhandled_commands.push(Response::single(
                                GrooveMessage::MidiToExternal(channel, message),
                            ));
                            self.broadcast_midi_messages(&[(channel, message)]);
                        }
                        EntityMessage::ControlF32(value) => {
                            self.dispatch_control_f32(uid, value);
                        }
                        _ => todo!(),
                    },
                    GrooveMessage::MidiFromExternal(channel, message) => {
                        self.broadcast_midi_messages(&[(channel, message)]);
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
        Response::batch(unhandled_commands)
    }

    // Call every Controller's tick() and return their responses. This is
    // pub(crate) only for testing by arpeggiator, which is bad
    //
    // TODO: figure out how to help Arpeggiator test without exposing these
    // internals.
    pub(crate) fn handle_tick(&mut self, tick_count: usize) -> (Response<GrooveMessage>, usize) {
        let mut max_ticks_completed = 0;
        (
            Response::batch(
                self.store()
                    .controller_uids()
                    .fold(Vec::new(), |mut v, uid| {
                        if let Some(e) = self.store_mut().get_mut(uid) {
                            if let Some(e) = e.as_is_controller_mut() {
                                let (message_opt, ticks_completed) = e.tick(tick_count);
                                if ticks_completed > max_ticks_completed {
                                    max_ticks_completed = ticks_completed;
                                }
                                if let Some(messages) = message_opt {
                                    // TODO clone, ouch
                                    v.push(Self::entity_command_to_groove_command(
                                        uid,
                                        Response::batch(
                                            messages.iter().map(|m| Response::single(m.clone())),
                                        ),
                                    ));
                                }
                            }
                        }
                        v
                    }),
            ),
            max_ticks_completed,
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

    fn broadcast_midi_messages(&mut self, channel_message_tuples: &[(MidiChannel, MidiMessage)]) {
        let mut v = Vec::from(channel_message_tuples);
        for (channel, message) in channel_message_tuples {
            if let Some(responses) = self.broadcast_midi_message(channel, message) {
                v.extend(responses);
            }
        }
    }

    fn broadcast_midi_message(
        &mut self,
        channel: &u8,
        message: &MidiMessage,
    ) -> Option<Vec<(MidiChannel, MidiMessage)>> {
        let receiver_uids = self.store.midi_receivers(channel).clone();
        if receiver_uids.is_empty() {
            return None;
        }
        let midi_messages_in_response = receiver_uids.iter().fold(
            Vec::new(),
            |mut v: Vec<(MidiChannel, MidiMessage)>, uid: &usize| {
                let uid = *uid;
                if let Some(e) = self.store.get_mut(uid) {
                    if let Some(e) = e.as_handles_midi_mut() {
                        if let Some(messages) = e.handle_midi_message(message) {
                            v.extend(messages);
                        }
                    }
                }
                v
            },
        );
        if midi_messages_in_response.is_empty() {
            None
        } else {
            Some(midi_messages_in_response)
        }
    }

    fn dispatch_control_f32(&mut self, uid: usize, value: f32) {
        let control_links = self.store.control_links(uid).clone();
        for (target_uid, param_id) in control_links {
            if let Some(entity) = self.store.get_mut(target_uid) {
                if let Some(entity) = entity.as_controllable_mut() {
                    entity.set_by_control_index(
                        param_id,
                        groove_core::control::F32ControlValue(value),
                    );
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
    pub fn run(&mut self, samples: &mut [StereoSample]) -> anyhow::Result<Vec<StereoSample>> {
        let mut performance_samples = Vec::<StereoSample>::new();
        loop {
            let ticks_completed = self.tick(samples);
            performance_samples.extend(&samples[0..ticks_completed]);
            if ticks_completed < samples.len() {
                break;
            }
        }
        Ok(performance_samples)
    }

    pub fn run_performance(
        &mut self,
        samples: &mut [StereoSample],
        quiet: bool,
    ) -> anyhow::Result<Performance> {
        let mut tick_count = 0;
        let performance = Performance::new_with(self.sample_rate);
        let progress_indicator_quantum: usize = self.sample_rate / 2;
        let mut next_progress_indicator: usize = progress_indicator_quantum;

        loop {
            let ticks_completed = self.tick(samples);
            if next_progress_indicator <= tick_count {
                if !quiet {
                    print!(".");
                    io::stdout().flush().unwrap();
                }
                next_progress_indicator += progress_indicator_quantum;
            }
            tick_count += ticks_completed;
            if ticks_completed < samples.len() {
                break;
            }
            for (i, sample) in samples.iter().enumerate() {
                if i < ticks_completed {
                    performance.worker.push(*sample);
                } else {
                    break;
                }
            }
        }
        if !quiet {
            println!();
        }
        if self.should_output_perf {
            self.metrics.report();
        }
        Ok(performance)
    }

    pub fn reset(&mut self) {
        self.store.reset(self.sample_rate);
    }

    /// Runs the whole world for the given number of frames, returning each
    /// frame's output as a StereoSample.
    ///
    /// The number of frames to run is implied in the length of the sample
    /// slice.
    ///
    /// Returns the actual number of frames filled. If this number is shorter
    /// than the slice length, then the performance is complete.
    pub(crate) fn tick(&mut self, samples: &mut [StereoSample]) -> usize {
        let tick_count = samples.len();
        let (commands, ticks_completed) = self.handle_tick(tick_count);
        match commands.0 {
            Internal::None => {}
            Internal::Single(message) => {
                self.update(message);
            }
            Internal::Batch(messages) => {
                for message in messages {
                    self.update(message);
                }
            }
        }
        self.gather_audio(samples);

        ticks_completed
    }

    pub fn last_track_sample(&self, index: usize) -> &StereoSample {
        if let Some(uid) = self.main_mixer_source_uids.get(&index) {
            if let Some(sample) = self.last_track_samples.get(*uid) {
                return sample;
            }
        }
        &StereoSample::SILENCE
    }

    pub fn track_samples(&self) -> &[StereoSample] {
        &self.last_track_samples
    }

    pub fn sample_rate(&self) -> usize {
        self.sample_rate
    }

    pub fn set_sample_rate(&mut self, sample_rate: usize) {
        self.sample_rate = sample_rate;
    }

    pub fn time_signature(&self) -> TimeSignature {
        self.time_signature
    }

    pub fn bpm(&self) -> f64 {
        self.bpm
    }
}

#[derive(Debug, Default)]
pub struct Store {
    last_uid: usize,
    uid_to_item: FxHashMap<usize, Entity>,

    // Linked controls (one entity controls another entity's parameter)
    uid_to_control: FxHashMap<usize, Vec<(usize, usize)>>,

    // Patch cables
    audio_sink_uid_to_source_uids: FxHashMap<usize, Vec<usize>>,

    // MIDI connections
    midi_channel_to_receiver_uid: FxHashMap<MidiChannel, Vec<usize>>,

    uvid_to_uid: FxHashMap<String, usize>,
}

impl Store {
    pub(crate) fn add(&mut self, uvid: Option<&str>, mut entity: Entity) -> usize {
        let uid = self.get_next_uid();
        entity.as_has_uid_mut().set_uid(uid);

        self.uid_to_item.insert(uid, entity);
        if let Some(uvid) = uvid {
            self.uvid_to_uid.insert(uvid.to_string(), uid);
        }
        uid
    }

    pub(crate) fn get(&self, uid: usize) -> Option<&Entity> {
        self.uid_to_item.get(&uid)
    }

    pub fn get_mut(&mut self, uid: usize) -> Option<&mut Entity> {
        self.uid_to_item.get_mut(&uid)
    }

    pub(crate) fn get_by_uvid(&self, uvid: &str) -> Option<&Entity> {
        if let Some(uid) = self.uvid_to_uid.get(uvid) {
            self.uid_to_item.get(uid)
        } else {
            None
        }
    }

    pub(crate) fn get_by_uvid_mut(&mut self, uvid: &str) -> Option<&mut Entity> {
        if let Some(uid) = self.uvid_to_uid.get(uvid) {
            self.uid_to_item.get_mut(uid)
        } else {
            None
        }
    }

    pub(crate) fn get_uid(&self, uvid: &str) -> Option<usize> {
        self.uvid_to_uid.get(uvid).copied()
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<usize, Entity> {
        self.uid_to_item.iter()
    }

    #[allow(dead_code)]
    pub(crate) fn values(&self) -> std::collections::hash_map::Values<usize, Entity> {
        self.uid_to_item.values()
    }

    pub(crate) fn values_mut(&mut self) -> std::collections::hash_map::ValuesMut<usize, Entity> {
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

    pub(crate) fn midi_receivers(&mut self, channel: &MidiChannel) -> &Vec<usize> {
        self.midi_channel_to_receiver_uid
            .entry(*channel)
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

    fn reset(&mut self, sample_rate: usize) {
        self.values_mut().for_each(|e| {
            if let Some(e) = e.as_is_controller_mut() {
                e.reset(sample_rate);
            } else if let Some(e) = e.as_is_instrument_mut() {
                e.reset(sample_rate);
            }
        })
    }
}

#[cfg(test)]
pub mod tests {
    use super::Orchestrator;
    use crate::{
        entities::Entity,
        {DEFAULT_BPM, DEFAULT_SAMPLE_RATE},
    };
    use groove_core::{
        midi::{MidiChannel, MidiMessage},
        Normal, StereoSample,
    };
    use groove_entities::{controllers::Timer, effects::Gain};
    use groove_toys::ToyAudioSource;

    impl Orchestrator {
        /// Warning! This method exists only as a debug shortcut to
        /// enable/disable test instruments. That's why it drops any reply
        /// messages on the floor, rather than routing them.
        pub fn debug_send_midi_note(&mut self, channel: MidiChannel, on: bool) {
            let message = if on {
                MidiMessage::NoteOn {
                    key: 17.into(),
                    vel: 127.into(),
                }
            } else {
                MidiMessage::NoteOff {
                    key: 17.into(),
                    vel: 127.into(),
                }
            };
            let reply = self.broadcast_midi_message(&channel, &message);

            assert!(
                reply.is_none(),
                concat!(
                "You might be using a special-purpose utility method to trigger a MIDI controller. "
                    , "That's bad. The messages being generated in response are not being routed.")
            );
        }
    }

    #[test]
    fn gather_audio_basic() {
        let mut o = Orchestrator::new_with(DEFAULT_SAMPLE_RATE, DEFAULT_BPM);
        let level_1_uid = o.add(
            None,
            Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(0.1))),
        );
        let level_2_uid = o.add(
            None,
            Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(0.2))),
        );

        // Nothing connected: should output silence.
        let mut samples: [StereoSample; 1] = Default::default();
        o.gather_audio(&mut samples);
        assert_eq!(samples[0], StereoSample::SILENCE);

        assert!(o.connect_to_main_mixer(level_1_uid).is_ok());
        o.gather_audio(&mut samples);
        assert!(samples[0].almost_equals(StereoSample::new_from_single_f64(0.1)));

        assert!(o.disconnect_from_main_mixer(level_1_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_2_uid).is_ok());
        o.gather_audio(&mut samples);
        assert!(samples[0].almost_equals(StereoSample::new_from_single_f64(0.2)));

        assert!(o.unpatch_all().is_ok());
        assert!(o.connect_to_main_mixer(level_1_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_2_uid).is_ok());
        o.gather_audio(&mut samples);
        assert!(samples[0].almost_equals(StereoSample::new_from_single_f64(0.1 + 0.2)));
    }

    #[test]
    fn gather_audio() {
        let mut o = Orchestrator::new_with(DEFAULT_SAMPLE_RATE, DEFAULT_BPM);
        let level_1_uid = o.add(
            None,
            Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(0.1))),
        );
        let gain_1_uid = o.add(
            None,
            Entity::Gain(Box::new(Gain::new_with(Normal::new(0.5)))),
        );
        let level_2_uid = o.add(
            None,
            Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(0.2))),
        );
        let level_3_uid = o.add(
            None,
            Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(0.3))),
        );
        let level_4_uid = o.add(
            None,
            Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(0.4))),
        );

        // Nothing connected: should output silence.
        let mut samples: [StereoSample; 1] = Default::default();
        o.gather_audio(&mut samples);
        assert_eq!(samples[0], StereoSample::SILENCE);

        // Just the single-level instrument; should get that.
        assert!(o.connect_to_main_mixer(level_1_uid).is_ok());
        o.gather_audio(&mut samples);
        assert!(samples[0].almost_equals(StereoSample::new_from_single_f64(0.1)));

        // Gain alone; that's weird, but it shouldn't explode.
        assert!(o.disconnect_from_main_mixer(level_1_uid).is_ok());
        assert!(o.connect_to_main_mixer(gain_1_uid).is_ok());
        o.gather_audio(&mut samples);
        assert_eq!(samples[0], StereoSample::SILENCE);

        // Disconnect/reconnect and connect just the single-level instrument again.
        assert!(o.disconnect_from_main_mixer(gain_1_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_1_uid).is_ok());
        o.gather_audio(&mut samples);
        assert!(samples[0].almost_equals(StereoSample::new_from_single_f64(0.1)));

        // Instrument to gain should result in (instrument x gain).
        assert!(o.unpatch_all().is_ok());
        assert!(o
            .patch_chain_to_main_mixer(&[level_1_uid, gain_1_uid])
            .is_ok());
        o.gather_audio(&mut samples);
        assert!(samples[0].almost_equals(StereoSample::new_from_single_f64(0.1 * 0.5)));

        assert!(o.connect_to_main_mixer(level_2_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_3_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_4_uid).is_ok());
        o.gather_audio(&mut samples);
        assert!(samples[0].almost_equals(StereoSample::new_from_single_f64(
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
        o.gather_audio(&mut samples);
        assert!(samples[0].almost_equals(StereoSample::new_from_single_f64(
            0.1 * 0.5 + 0.2 + 0.3 + 0.4
        )));
    }

    #[test]
    fn gather_audio_2() {
        let mut o = Orchestrator::new_with(DEFAULT_SAMPLE_RATE, DEFAULT_BPM);
        let piano_1_uid = o.add(
            None,
            Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(0.1))),
        );
        let low_pass_1_uid = o.add(
            None,
            Entity::Gain(Box::new(Gain::new_with(Normal::new(0.2)))),
        );
        let gain_1_uid = o.add(
            None,
            Entity::Gain(Box::new(Gain::new_with(Normal::new(0.4)))),
        );

        let bassline_uid = o.add(
            None,
            Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(0.3))),
        );
        let gain_2_uid = o.add(
            None,
            Entity::Gain(Box::new(Gain::new_with(Normal::new(0.6)))),
        );

        let synth_1_uid = o.add(
            None,
            Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(0.5))),
        );
        let gain_3_uid = o.add(
            None,
            Entity::Gain(Box::new(Gain::new_with(Normal::new(0.8)))),
        );

        let drum_1_uid = o.add(
            None,
            Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(0.7))),
        );

        // First chain.
        assert!(o
            .patch_chain_to_main_mixer(&[piano_1_uid, low_pass_1_uid, gain_1_uid])
            .is_ok());
        let mut samples: [StereoSample; 1] = Default::default();
        o.gather_audio(&mut samples);
        let sample_chain_1 = samples[0];
        assert!(sample_chain_1.almost_equals(StereoSample::new_from_single_f64(0.1 * 0.2 * 0.4)));

        // Second chain.
        assert!(o.unpatch_all().is_ok());
        assert!(o
            .patch_chain_to_main_mixer(&[bassline_uid, gain_2_uid])
            .is_ok());
        o.gather_audio(&mut samples);
        let sample_chain_2 = samples[0];
        assert!(sample_chain_2.almost_equals(StereoSample::new_from_single_f64(0.3 * 0.6)));

        // Third.
        assert!(o.unpatch_all().is_ok());
        assert!(o
            .patch_chain_to_main_mixer(&[synth_1_uid, gain_3_uid])
            .is_ok());
        o.gather_audio(&mut samples);
        let sample_chain_3 = samples[0];
        assert_eq!(sample_chain_3, StereoSample::new_from_single_f64(0.5 * 0.8));

        // Fourth.
        assert!(o.unpatch_all().is_ok());
        assert!(o.patch_chain_to_main_mixer(&[drum_1_uid]).is_ok());
        o.gather_audio(&mut samples);
        let sample_chain_4 = samples[0];
        assert!(sample_chain_4.almost_equals(StereoSample::new_from_single_f64(0.7)));

        // Now start over and successively add. This is first and second chains together.
        assert!(o.unpatch_all().is_ok());
        assert!(o
            .patch_chain_to_main_mixer(&[piano_1_uid, low_pass_1_uid, gain_1_uid])
            .is_ok());
        assert!(o
            .patch_chain_to_main_mixer(&[bassline_uid, gain_2_uid])
            .is_ok());
        o.gather_audio(&mut samples);
        assert_eq!(samples[0], sample_chain_1 + sample_chain_2);

        // Plus third.
        assert!(o
            .patch_chain_to_main_mixer(&[synth_1_uid, gain_3_uid])
            .is_ok());
        o.gather_audio(&mut samples);
        assert_eq!(samples[0], sample_chain_1 + sample_chain_2 + sample_chain_3);

        // Plus fourth.
        assert!(o.patch_chain_to_main_mixer(&[drum_1_uid]).is_ok());
        o.gather_audio(&mut samples);
        assert!(samples[0]
            .almost_equals(sample_chain_1 + sample_chain_2 + sample_chain_3 + sample_chain_4));
    }

    #[test]
    fn gather_audio_with_branches() {
        let mut o = Orchestrator::new_with(DEFAULT_SAMPLE_RATE, DEFAULT_BPM);
        let instrument_1_uid = o.add(
            None,
            Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(0.1))),
        );
        let instrument_2_uid = o.add(
            None,
            Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(0.3))),
        );
        let instrument_3_uid = o.add(
            None,
            Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(0.5))),
        );
        let effect_1_uid = o.add(
            None,
            Entity::Gain(Box::new(Gain::new_with(Normal::new(0.5)))),
        );

        assert!(o.patch_chain_to_main_mixer(&[instrument_1_uid]).is_ok());
        assert!(o.patch_chain_to_main_mixer(&[effect_1_uid]).is_ok());
        assert!(o.patch(instrument_2_uid, effect_1_uid).is_ok());
        assert!(o.patch(instrument_3_uid, effect_1_uid).is_ok());
        let mut samples: [StereoSample; 1] = Default::default();
        o.gather_audio(&mut samples);
        assert!(
            samples[0].almost_equals(StereoSample::new_from_single_f64(0.1 + 0.5 * (0.3 + 0.5)))
        );
    }

    #[test]
    fn run_buffer_size_can_be_odd_number() {
        let mut o = Orchestrator::new_with(DEFAULT_SAMPLE_RATE, DEFAULT_BPM);
        let _ = o.add(
            None,
            Entity::Timer(Box::new(Timer::new_with(DEFAULT_SAMPLE_RATE, 1.0))),
        );

        // Prime number
        let mut sample_buffer = [StereoSample::SILENCE; 17];
        let r = o.run(&mut sample_buffer);
        assert!(r.is_ok());
        assert_eq!(r.unwrap().len(), DEFAULT_SAMPLE_RATE);
    }

    #[test]
    fn orchestrator_sample_count_is_accurate_for_zero_timer() {
        let mut o = Orchestrator::new_with(DEFAULT_SAMPLE_RATE, DEFAULT_BPM);
        let _ = o.add(
            None,
            Entity::Timer(Box::new(Timer::new_with(DEFAULT_SAMPLE_RATE, 0.0))),
        );
        let mut sample_buffer = [StereoSample::SILENCE; 64];
        if let Ok(samples) = o.run(&mut sample_buffer) {
            assert_eq!(samples.len(), 0);
        } else {
            panic!("run failed");
        }
    }

    #[test]
    fn orchestrator_sample_count_is_accurate_for_short_timer() {
        let mut o = Orchestrator::new_with(DEFAULT_SAMPLE_RATE, DEFAULT_BPM);
        let _ = o.add(
            None,
            Entity::Timer(Box::new(Timer::new_with(
                DEFAULT_SAMPLE_RATE,
                1.0 / DEFAULT_SAMPLE_RATE as f32,
            ))),
        );
        let mut sample_buffer = [StereoSample::SILENCE; 64];
        if let Ok(samples) = o.run(&mut sample_buffer) {
            assert_eq!(samples.len(), 1);
        } else {
            panic!("run failed");
        }
    }

    #[test]
    fn orchestrator_sample_count_is_accurate_for_ordinary_timer() {
        let mut o = Orchestrator::new_with(DEFAULT_SAMPLE_RATE, DEFAULT_BPM);
        let _ = o.add(
            None,
            Entity::Timer(Box::new(Timer::new_with(DEFAULT_SAMPLE_RATE, 1.0))),
        );
        let mut sample_buffer = [StereoSample::SILENCE; 64];
        if let Ok(samples) = o.run(&mut sample_buffer) {
            assert_eq!(samples.len(), 44100);
        } else {
            panic!("run failed");
        }
    }

    #[test]
    fn test_patch_fails_with_bad_id() {
        let mut o = Orchestrator::new_with(DEFAULT_SAMPLE_RATE, DEFAULT_BPM);
        assert!(o.patch(3, 2).is_err());
    }
}
