// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::{
    entities::Entity,
    messages::{ControlLink, GrooveEvent, GrooveInput, Internal, Response},
    OtherEntityMessage,
};
use anyhow::anyhow;
use core::fmt::Debug;
use crossbeam::deque::Worker;
use groove_core::{
    midi::{MidiChannel, MidiMessage},
    time::{Clock, ClockNano, TimeSignature},
    traits::{Performs, Resets},
    ParameterType, StereoSample,
};
use groove_entities::{
    controllers::{PatternManager, Sequencer, SequencerNano},
    effects::Mixer,
    instruments::{Metronome, MetronomeNano},
    EntityMessage,
};
use groove_proc_macros::Uid;
use rustc_hash::{FxHashMap, FxHashSet};
use std::io::{self, Write};

#[cfg(feature = "metrics")]
use dipstick::InputScope;
#[cfg(feature = "metrics")]
use metrics::DipstickWrapper;

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

/// [Orchestrator] manages all [Entities](Entity) (controllers, effects, and
/// instruments). It also manages their virtual patch cables, virtual MIDI
/// cables, and control relationships. When you're ready to render a song, it
/// creates a stream of [StereoSample]s that can be fed to the computer's sound
/// card or exported as a WAV file.
///
/// It's not necessary to use [Orchestrator] to take advantage of this crate's
/// musical capabilities, but all the entities were designed to work smoothly
/// with it.
#[derive(Debug, Uid)]
pub struct Orchestrator {
    uid: usize,
    title: Option<String>,
    store: Store,

    // This is the master clock.
    clock: Clock,
    is_performing: bool,

    main_mixer_uid: usize,
    pattern_manager_uid: usize,
    sequencer_uid: usize,
    metronome_uid: usize,

    #[cfg(feature = "metrics")]
    metrics: DipstickWrapper,

    should_output_perf: bool,

    last_track_samples: Vec<StereoSample>,
    last_entity_samples: Vec<StereoSample>,
    main_mixer_source_uids: FxHashSet<usize>,
    last_samples: FxHashMap<usize, StereoSample>,
}
impl Orchestrator {
    // TODO: prefix these to reserve internal ID namespace
    pub const MAIN_MIXER_UVID: &str = "main-mixer";
    pub const PATTERN_MANAGER_UVID: &str = "pattern-manager";
    pub const BEAT_SEQUENCER_UVID: &str = "beat-sequencer";
    pub const METRONOME_UVID: &str = "metronome";

    #[cfg(feature = "metrics")]
    fn install_entity_metric(&mut self, uvid: Option<&str>, uid: usize) {
        let name = format!("entity {}", uvid.unwrap_or(format!("uid {uid}").as_str()));
        self.metrics
            .entity_audio_times
            .insert(uid, self.metrics.bucket.timer(name.as_str()));
    }

    fn add_with_optional_uvid(&mut self, mut entity: Entity, uvid: Option<&str>) -> usize {
        #[cfg(feature = "metrics")]
        self.metrics.entity_count.mark();

        entity.as_resets_mut().reset(self.clock.sample_rate());
        let uid = self.store.add(uvid, entity);

        #[cfg(feature = "metrics")]
        self.install_entity_metric(Some(uvid), uid);

        uid
    }

    pub fn add(&mut self, entity: Entity) -> usize {
        self.add_with_optional_uvid(entity, None)
    }

    pub fn add_with_uvid(&mut self, entity: Entity, uvid: &str) -> usize {
        self.add_with_optional_uvid(entity, Some(uvid))
    }

    pub fn entity_iter(&self) -> std::collections::hash_map::Iter<usize, Entity> {
        self.store.iter()
    }

    pub fn connections(&self) -> &[ControlLink] {
        self.store.flattened_control_links()
    }

    pub fn get(&self, uid: usize) -> Option<&Entity> {
        self.store.get(uid)
    }

    pub fn get_mut(&mut self, uid: usize) -> Option<&mut Entity> {
        self.store.get_mut(uid)
    }

    #[allow(dead_code)]
    pub(crate) fn get_by_uvid(&self, uvid: &str) -> Option<&Entity> {
        self.store.get_by_uvid(uvid)
    }

    #[allow(dead_code)]
    pub(crate) fn get_by_uvid_mut(&mut self, uvid: &str) -> Option<&mut Entity> {
        self.store.get_by_uvid_mut(uvid)
    }

    pub fn get_uid_by_uvid(&self, uvid: &str) -> Option<usize> {
        self.store.get_uid(uvid)
    }

    pub fn link_control_by_id(
        &mut self,
        source_uid: usize,
        target_uid: usize,
        control_index: usize,
    ) -> anyhow::Result<()> {
        if let Some(target) = self.store.get(target_uid) {
            if target.as_controllable().is_some() {
                if let Some(entity) = self.store.get(source_uid) {
                    if entity.as_is_controller().is_some() {
                        self.store
                            .link_control(source_uid, target_uid, control_index);
                    } else {
                        return Err(anyhow!(
                            "controller ID {} is not of a controller type",
                            source_uid
                        ));
                    }
                } else {
                    return Err(anyhow!("couldn't find controller ID {}", source_uid));
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

    pub fn link_control_by_name(
        &mut self,
        controller_uid: usize,
        target_uid: usize,
        param_name: &str,
    ) -> anyhow::Result<()> {
        if let Some(target) = self.store.get(target_uid) {
            if let Some(target) = target.as_controllable() {
                let control_index = target.control_index_for_name(param_name);
                if control_index != usize::MAX {
                    return self.link_control_by_id(controller_uid, target_uid, control_index);
                } else {
                    // TODO: return valid names so user doesn't hate us
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
    }

    pub fn unlink_control_by_id(
        &mut self,
        controller_uid: usize,
        target_uid: usize,
        control_index: usize,
    ) {
        self.store
            .unlink_control(controller_uid, target_uid, control_index);
    }

    #[cfg(test)]
    pub(crate) fn unlink_control_by_name(
        &mut self,
        controller_uid: usize,
        target_uid: usize,
        param_name: &str,
    ) {
        if let Some(target) = self.store.get(target_uid) {
            if let Some(target) = target.as_controllable() {
                let control_index = target.control_index_for_name(param_name);
                if control_index != usize::MAX {
                    self.store
                        .unlink_control(controller_uid, target_uid, control_index);
                }
            }
        }
    }

    pub fn patch(&mut self, output_uid: usize, input_uid: usize) -> anyhow::Result<()> {
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
    pub fn patch_chain_to_main_mixer(&mut self, entity_uids: &[usize]) -> anyhow::Result<()> {
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
        self.last_entity_samples.clear();
        self.last_entity_samples
            .resize(256, StereoSample::default()); // HACK HACK HACK

        for sample in samples {
            enum StackEntry {
                ToVisit(usize),
                CollectResultFor(usize, StereoSample),
            }
            #[cfg(feature = "metrics")]
            let gather_audio_start_time = self.metrics.gather_audio_fn_timer.start();

            let mut stack = Vec::new();
            let mut sum = StereoSample::default();
            stack.push(StackEntry::ToVisit(self.main_mixer_uid));

            #[cfg(feature = "metrics")]
            self.metrics.mark_stack_loop_entry.mark();
            while let Some(entry) = stack.pop() {
                #[cfg(feature = "metrics")]
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
                                #[cfg(feature = "metrics")]
                                if let Some(timer) = self.metrics.entity_audio_times.get(&uid) {
                                    let start_time = timer.start();
                                    entity.tick(1);
                                    timer.stop(start_time);
                                } else {
                                    entity.tick(1);
                                }

                                #[cfg(not(feature = "metrics"))]
                                entity.tick(1);

                                self.last_samples.insert(uid, entity.value());
                                self.last_entity_samples[uid] = entity.value();
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
                                #[cfg(feature = "metrics")]
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

                                #[cfg(not(feature = "metrics"))]
                                let entity_value = entity.transform_audio(sum);

                                sum = accumulated_sum + entity_value;
                                self.last_samples.insert(uid, entity_value);
                                self.last_entity_samples[uid] = entity_value;
                            }
                        }
                    }
                }
            }

            #[cfg(feature = "metrics")]
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

    pub fn connect_midi_downstream(
        &mut self,
        receiver_uid: usize,
        receiver_midi_channel: MidiChannel,
    ) {
        if let Some(e) = self.get(receiver_uid) {
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

    pub fn set_should_output_perf(&mut self, value: bool) {
        self.should_output_perf = value;
    }

    pub fn sequencer_uid(&self) -> usize {
        self.sequencer_uid
    }

    pub fn main_mixer_uid(&self) -> usize {
        self.main_mixer_uid
    }

    pub fn pattern_manager_uid(&self) -> usize {
        self.pattern_manager_uid
    }

    pub fn set_title(&mut self, title: Option<String>) {
        self.title = title;
    }

    pub fn new_with(clock_params: ClockNano) -> Self {
        let mut r = Self {
            uid: Default::default(),
            title: Some("Untitled".to_string()),
            clock: Clock::new_with(clock_params),
            is_performing: Default::default(),
            store: Default::default(),
            main_mixer_uid: Default::default(),
            pattern_manager_uid: Default::default(),
            sequencer_uid: Default::default(),
            metronome_uid: Default::default(),
            #[cfg(feature = "metrics")]
            metrics: Default::default(),
            should_output_perf: Default::default(),
            last_track_samples: Default::default(),
            last_entity_samples: Default::default(),
            main_mixer_source_uids: Default::default(),
            last_samples: Default::default(),
        };
        r.main_mixer_uid = r.add_with_uvid(
            Entity::Mixer(Box::new(Mixer::default())),
            Self::MAIN_MIXER_UVID,
        );
        r.pattern_manager_uid = r.add_with_uvid(
            Entity::PatternManager(Box::new(PatternManager::default())),
            Self::PATTERN_MANAGER_UVID,
        );
        r.sequencer_uid = r.add_with_uvid(
            Entity::Sequencer(Box::new(Sequencer::new_with(SequencerNano {
                bpm: r.bpm(),
            }))),
            Self::BEAT_SEQUENCER_UVID,
        );
        // See https://github.com/sowbug/groove/issues/127. This is clunky
        r.metronome_uid = r.add_with_uvid(
            Entity::Metronome(Box::new(Metronome::new_with(MetronomeNano {
                bpm: r.bpm(),
            }))),
            Self::METRONOME_UVID,
        );
        let _ = r.connect_to_main_mixer(r.metronome_uid);

        r
    }

    pub fn update(&mut self, input: GrooveInput) -> Response<GrooveEvent> {
        let mut unhandled_commands = Vec::new();
        let mut commands = Vec::new();
        commands.push(Response::single(input));
        while let Some(command) = commands.pop() {
            let mut messages = Vec::new();
            match command.0 {
                Internal::None => {}
                Internal::Single(action) => messages.push(action),
                Internal::Batch(actions) => messages.extend(actions),
            }
            while let Some(message) = messages.pop() {
                match message {
                    GrooveInput::EntityMessage(uid, event) => match event {
                        EntityMessage::Midi(channel, message) => {
                            // We could have pushed this onto the regular
                            // commands vector, and then instead of panicking on
                            // the MidiToExternal match, handle it by pushing it
                            // onto the other vector. It is slightly simpler, if
                            // less elegant, to do it this way.
                            unhandled_commands.push(Response::single(GrooveEvent::MidiToExternal(
                                channel, message,
                            )));
                            self.broadcast_midi_messages(&[(channel, message)]);
                        }
                        EntityMessage::ControlF32(value) => {
                            messages.extend(self.generate_control_update_messages(uid, value));
                        }
                        _ => todo!(),
                    },
                    GrooveInput::MidiFromExternal(channel, message) => {
                        self.broadcast_midi_messages(&[(channel, message)]);
                    }
                    GrooveInput::AddControlLink(link) => {
                        // The UI has asked us to link a control.
                        let _ = self.link_control_by_id(
                            link.source_uid,
                            link.target_uid,
                            link.control_index,
                        );
                    }
                    GrooveInput::RemoveControlLink(link) => {
                        // The UI has asked us to link a control.
                        self.unlink_control_by_id(
                            link.source_uid,
                            link.target_uid,
                            link.control_index,
                        );
                    }
                    GrooveInput::Update(uid, message) => self.update_controllable(uid, message),
                    GrooveInput::Play => self.play(),
                    GrooveInput::Stop => self.stop(),
                    GrooveInput::SkipToStart => self.skip_to_start(),
                    GrooveInput::SetSampleRate(sample_rate) => self.reset(sample_rate),
                }
            }
        }
        Response::batch(unhandled_commands)
    }

    // Call every Controller's tick() and handle their responses.
    fn handle_tick(&mut self, tick_count: usize) -> (Response<GrooveEvent>, usize) {
        let mut max_ticks_completed = 0;
        (
            Response::batch(self.store.controller_uids().fold(Vec::new(), |mut v, uid| {
                if let Some(e) = self.store.get_mut(uid) {
                    if let Some(e) = e.as_is_controller_mut() {
                        let (message_opt, ticks_completed) = e.tick(tick_count);
                        if ticks_completed > max_ticks_completed {
                            max_ticks_completed = ticks_completed;
                        }
                        if let Some(messages) = message_opt {
                            for message in messages {
                                // This is where outputs get turned into inputs.
                                v.push(self.update(GrooveInput::EntityMessage(uid, message)));
                            }
                        }
                    }
                }
                v
            })),
            max_ticks_completed,
        )
    }

    fn broadcast_midi_messages(&mut self, channel_message_tuples: &[(MidiChannel, MidiMessage)]) {
        let mut v = Vec::from(channel_message_tuples);
        while let Some((channel, message)) = v.pop() {
            if let Some(responses) = self.broadcast_midi_message(&channel, &message) {
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
                    } else {
                        panic!("tried to send MIDI to something that doesn't handle it")
                    }
                } else {
                    panic!("tried to send MIDI to a bad UID")
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

    fn generate_control_update_messages(&mut self, uid: usize, value: f32) -> Vec<GrooveInput> {
        if let Some(control_links) = self.store.control_links(uid) {
            return control_links
                .iter()
                .fold(Vec::default(), |mut v, (target_uid, param_id)| {
                    if let Some(entity) = self.store.get(*target_uid) {
                        if let Some(msg) = entity.message_for(*param_id, value.into()) {
                            v.push(GrooveInput::Update(*target_uid, msg));
                        }
                    }
                    v
                });
        }
        Vec::default()
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
    pub fn run(&mut self, buffer: &mut [StereoSample]) -> anyhow::Result<Vec<StereoSample>> {
        self.skip_to_start();
        self.play();
        let mut performance_samples = Vec::<StereoSample>::new();
        loop {
            // If we want external MIDI to work here, then we need to figure out what to do with commands.
            let (_commands, ticks_completed) = self.tick(buffer);
            performance_samples.extend(&buffer[0..ticks_completed]);
            if ticks_completed < buffer.len() {
                break;
            }
        }
        Ok(performance_samples)
    }

    pub fn run_performance(
        &mut self,
        buffer: &mut [StereoSample],
        quiet: bool,
    ) -> anyhow::Result<Performance> {
        let sample_rate = self.clock.sample_rate();
        let mut tick_count = 0;
        let performance = Performance::new_with(sample_rate);
        let progress_indicator_quantum: usize = sample_rate / 2;
        let mut next_progress_indicator: usize = progress_indicator_quantum;

        self.skip_to_start();
        self.play();
        loop {
            // If we want external MIDI to work here, then we need to figure out what to do with commands.
            let (_commands, ticks_completed) = self.tick(buffer);
            if next_progress_indicator <= tick_count {
                if !quiet {
                    print!(".");
                    io::stdout().flush().unwrap();
                }
                next_progress_indicator += progress_indicator_quantum;
            }
            tick_count += ticks_completed;
            if ticks_completed < buffer.len() {
                break;
            }
            for (i, sample) in buffer.iter().enumerate() {
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
        #[cfg(feature = "metrics")]
        if self.should_output_perf {
            self.metrics.report();
        }
        Ok(performance)
    }

    /// Runs the whole world for the given number of frames, returning each
    /// frame's output as a StereoSample.
    ///
    /// The number of frames to run is implied in the length of the sample
    /// slice.
    ///
    /// Returns the actual number of frames filled. If this number is shorter
    /// than the slice length, then the performance is complete.
    pub fn tick(&mut self, samples: &mut [StereoSample]) -> (Response<GrooveEvent>, usize) {
        let tick_count = samples.len();
        let (commands, ticks_completed) = self.handle_tick(tick_count);
        self.gather_audio(samples);

        if self.is_performing {
            self.clock.tick_batch(ticks_completed);
        }
        if ticks_completed < tick_count {
            self.is_performing = false;
        }

        (commands, ticks_completed)
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
        self.clock.sample_rate()
    }

    #[deprecated = "Call reset() instead"]
    pub fn set_sample_rate(&mut self, _: usize) {
        panic!("Call reset() instead");
    }

    pub fn time_signature(&self) -> &TimeSignature {
        &self.clock.time_signature()
    }

    pub fn bpm(&self) -> f64 {
        self.clock.bpm()
    }

    pub fn title(&self) -> Option<String> {
        // TODO: why is this so awful?
        self.title.as_ref().map(|title| title.clone())
    }

    pub fn last_audio_wad(&self) -> Vec<(usize, StereoSample)> {
        self.last_entity_samples.iter().enumerate().fold(
            Vec::default(),
            |mut v, (index, sample)| {
                v.push((index, *sample));
                v
            },
        )
    }

    fn update_controllable(&mut self, uid: usize, message: OtherEntityMessage) {
        if let Some(entity) = self.store.get_mut(uid) {
            entity.update(message)
        }
    }

    pub fn set_bpm(&mut self, bpm: ParameterType) {
        self.clock.set_bpm(bpm);
    }

    pub fn clock(&self) -> &Clock {
        &self.clock
    }
}
impl Performs for Orchestrator {
    // The difference between play() and tick() is that play() tells devices
    // that it's time to do work, and tick() actually gives them gives them a
    // slice of time to do that work. To illustrate: suppose we start calling
    // tick() without first having called play(). We'll generally get silence.
    // If a MIDI note-on event comes in from an external device, though, sound
    // will be produced, because the MIDI event itself represented work for
    // connected devices to do. Now, suppose we call play() but don't call
    // tick(). Devices will respond by getting into whatever state they need to
    // start doing work, but they won't actually get the time-slice to do the
    // work.
    fn play(&mut self) {
        self.is_performing = true;
        for entity in self.store.values_mut() {
            if let Some(controller) = entity.as_is_controller_mut() {
                controller.play();
            }
        }
    }

    // Calling stop() will not stop all audio. Devices will respond in different
    // ways to stop(). Effects and instruments generally ignore it. Controllers,
    // however, should send note-off events for any notes that they have asked
    // instruments to play, and they should stop sending new note-on events.
    //
    // TODO: if effects ignore stop, then how do we deal with a feedback loop?
    // Maybe there should be a relationship with stop() and mute. Or perhaps
    // effects that are susceptible to feedback should treat stop() differently.
    fn stop(&mut self) {
        self.is_performing = false;
        for entity in self.store.values_mut() {
            if let Some(controller) = entity.as_is_controller_mut() {
                controller.stop();
            }
        }
    }

    fn skip_to_start(&mut self) {
        self.clock.seek(0);
        for entity in self.store.values_mut() {
            if let Some(controller) = entity.as_is_controller_mut() {
                controller.skip_to_start();
            }
        }
    }
}
impl Resets for Orchestrator {
    fn reset(&mut self, sample_rate: usize) {
        self.clock.reset(sample_rate);
        self.store.reset(sample_rate);
    }
}

/// Keeps all [Entity] in one place, and manages their relationships, such as
/// patch cables.
#[derive(Debug, Default)]
pub(crate) struct Store {
    last_uid: usize,
    uid_to_item: FxHashMap<usize, Entity>,

    /// Linked controls (one entity controls another entity's parameter)
    uid_to_control: FxHashMap<usize, Vec<(usize, usize)>>,

    /// Same as uid_to_control but flattened
    flattened_control_links: Vec<ControlLink>,

    /// Patch cables
    audio_sink_uid_to_source_uids: FxHashMap<usize, Vec<usize>>,

    /// MIDI connections
    midi_channel_to_receiver_uid: FxHashMap<MidiChannel, Vec<usize>>,

    /// Human-readable UIDs to internal UIDs
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
        source_uid: usize,
        target_uid: usize,
        control_index: usize,
    ) {
        self.uid_to_control
            .entry(source_uid)
            .or_default()
            .push((target_uid, control_index));
        self.flattened_control_links.push(ControlLink {
            source_uid,
            target_uid,
            control_index,
        });
    }

    pub(crate) fn unlink_control(
        &mut self,
        source_uid: usize,
        target_uid: usize,
        control_index: usize,
    ) {
        self.uid_to_control
            .entry(source_uid)
            .or_default()
            .retain(|(uid, index)| *uid != target_uid && *index != control_index);

        // This is slow, but it's OK because it's not time-sensitive
        let link = ControlLink {
            source_uid,
            target_uid,
            control_index,
        };
        if let Some(index) = self
            .flattened_control_links
            .iter()
            .position(|item| *item == link)
        {
            self.flattened_control_links.swap_remove(index);
        }
    }

    pub(crate) fn control_links(&self, source_uid: usize) -> Option<&Vec<(usize, usize)>> {
        self.uid_to_control.get(&source_uid)
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

    fn flattened_control_links(&self) -> &[ControlLink] {
        &self.flattened_control_links
    }
}
impl Resets for Store {
    fn reset(&mut self, sample_rate: usize) {
        self.values_mut().for_each(|e| {
            e.as_resets_mut().reset(sample_rate);
        })
    }
}

#[cfg(test)]
pub mod tests {
    use super::Orchestrator;
    use crate::{
        entities::Entity,
        tests::{DEFAULT_BPM, DEFAULT_MIDI_TICKS_PER_SECOND, DEFAULT_SAMPLE_RATE},
    };
    use groove_core::{
        midi::{MidiChannel, MidiMessage},
        time::{BeatValue, Clock, ClockNano, PerfectTimeUnit, TimeSignature},
        traits::{Performs, Resets},
        Normal, StereoSample,
    };
    use groove_entities::{
        controllers::{
            Arpeggiator, ArpeggiatorNano, Note, Pattern, PatternProgrammer, Sequencer,
            SequencerNano, Timer, TimerNano,
        },
        effects::{Gain, GainNano},
    };
    use groove_toys::{ToyAudioSource, ToyAudioSourceNano, ToyInstrument, ToyInstrumentNano};

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
        let mut o = Orchestrator::new_with(ClockNano {
            bpm: DEFAULT_BPM,
            midi_ticks_per_second: DEFAULT_MIDI_TICKS_PER_SECOND,
            time_signature: TimeSignature { top: 4, bottom: 4 },
        });
        let level_1_uid = o.add(Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(
            ToyAudioSourceNano { level: 0.1 },
        ))));
        let level_2_uid = o.add(Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(
            ToyAudioSourceNano { level: 0.2 },
        ))));

        // Nothing connected: should output silence.
        let mut samples: [StereoSample; 1] = Default::default();
        o.gather_audio(&mut samples);
        assert_eq!(samples[0], StereoSample::SILENCE);

        assert!(o.connect_to_main_mixer(level_1_uid).is_ok());
        o.gather_audio(&mut samples);
        assert!(samples[0].almost_equals(StereoSample::from(0.1)));

        assert!(o.disconnect_from_main_mixer(level_1_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_2_uid).is_ok());
        o.gather_audio(&mut samples);
        assert!(samples[0].almost_equals(StereoSample::from(0.2)));

        assert!(o.unpatch_all().is_ok());
        assert!(o.connect_to_main_mixer(level_1_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_2_uid).is_ok());
        o.gather_audio(&mut samples);
        assert!(samples[0].almost_equals(StereoSample::from(0.1 + 0.2)));
    }

    #[test]
    fn gather_audio() {
        let mut o = Orchestrator::new_with(ClockNano {
            bpm: DEFAULT_BPM,
            midi_ticks_per_second: DEFAULT_MIDI_TICKS_PER_SECOND,
            time_signature: TimeSignature { top: 4, bottom: 4 },
        });
        let level_1_uid = o.add(Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(
            ToyAudioSourceNano { level: 0.1 },
        ))));
        let gain_1_uid = o.add(Entity::Gain(Box::new(Gain::new_with(
            groove_entities::effects::GainNano {
                ceiling: Normal::new(0.5),
            },
        ))));
        let level_2_uid = o.add(Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(
            ToyAudioSourceNano { level: 0.2 },
        ))));
        let level_3_uid = o.add(Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(
            ToyAudioSourceNano { level: 0.3 },
        ))));
        let level_4_uid = o.add(Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(
            ToyAudioSourceNano { level: 0.4 },
        ))));

        // Nothing connected: should output silence.
        let mut samples: [StereoSample; 1] = Default::default();
        o.gather_audio(&mut samples);
        assert_eq!(samples[0], StereoSample::SILENCE);

        // Just the single-level instrument; should get that.
        assert!(o.connect_to_main_mixer(level_1_uid).is_ok());
        o.gather_audio(&mut samples);
        assert!(samples[0].almost_equals(StereoSample::from(0.1)));

        // Gain alone; that's weird, but it shouldn't explode.
        assert!(o.disconnect_from_main_mixer(level_1_uid).is_ok());
        assert!(o.connect_to_main_mixer(gain_1_uid).is_ok());
        o.gather_audio(&mut samples);
        assert_eq!(samples[0], StereoSample::SILENCE);

        // Disconnect/reconnect and connect just the single-level instrument again.
        assert!(o.disconnect_from_main_mixer(gain_1_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_1_uid).is_ok());
        o.gather_audio(&mut samples);
        assert!(samples[0].almost_equals(StereoSample::from(0.1)));

        // Instrument to gain should result in (instrument x gain).
        assert!(o.unpatch_all().is_ok());
        assert!(o
            .patch_chain_to_main_mixer(&[level_1_uid, gain_1_uid])
            .is_ok());
        o.gather_audio(&mut samples);
        assert!(samples[0].almost_equals(StereoSample::from(0.1 * 0.5)));

        assert!(o.connect_to_main_mixer(level_2_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_3_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_4_uid).is_ok());
        o.gather_audio(&mut samples);
        assert!(samples[0].almost_equals(StereoSample::from(0.1 * 0.5 + 0.2 + 0.3 + 0.4)));

        // Same thing, but inverted order.
        assert!(o.unpatch_all().is_ok());
        assert!(o.connect_to_main_mixer(level_4_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_3_uid).is_ok());
        assert!(o.connect_to_main_mixer(level_2_uid).is_ok());
        assert!(o
            .patch_chain_to_main_mixer(&[level_1_uid, gain_1_uid])
            .is_ok());
        o.gather_audio(&mut samples);
        assert!(samples[0].almost_equals(StereoSample::from(0.1 * 0.5 + 0.2 + 0.3 + 0.4)));
    }

    #[test]
    fn gather_audio_2() {
        let mut o = Orchestrator::new_with(ClockNano {
            bpm: DEFAULT_BPM,
            midi_ticks_per_second: DEFAULT_MIDI_TICKS_PER_SECOND,
            time_signature: TimeSignature { top: 4, bottom: 4 },
        });
        let piano_1_uid = o.add(Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(
            ToyAudioSourceNano { level: 0.1 },
        ))));
        let low_pass_1_uid = o.add(Entity::Gain(Box::new(Gain::new_with(GainNano {
            ceiling: Normal::new(0.2),
        }))));
        let gain_1_uid = o.add(Entity::Gain(Box::new(Gain::new_with(GainNano {
            ceiling: Normal::new(0.4),
        }))));

        let bassline_uid = o.add(Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(
            ToyAudioSourceNano { level: 0.3 },
        ))));
        let gain_2_uid = o.add(Entity::Gain(Box::new(Gain::new_with(GainNano {
            ceiling: Normal::new(0.6),
        }))));

        let synth_1_uid = o.add(Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(
            ToyAudioSourceNano { level: 0.5 },
        ))));
        let gain_3_uid = o.add(Entity::Gain(Box::new(Gain::new_with(GainNano {
            ceiling: Normal::new(0.8),
        }))));

        let drum_1_uid = o.add(Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(
            ToyAudioSourceNano { level: 0.7 },
        ))));

        // First chain.
        assert!(o
            .patch_chain_to_main_mixer(&[piano_1_uid, low_pass_1_uid, gain_1_uid])
            .is_ok());
        let mut samples: [StereoSample; 1] = Default::default();
        o.gather_audio(&mut samples);
        let sample_chain_1 = samples[0];
        assert!(sample_chain_1.almost_equals(StereoSample::from(0.1 * 0.2 * 0.4)));

        // Second chain.
        assert!(o.unpatch_all().is_ok());
        assert!(o
            .patch_chain_to_main_mixer(&[bassline_uid, gain_2_uid])
            .is_ok());
        o.gather_audio(&mut samples);
        let sample_chain_2 = samples[0];
        assert!(sample_chain_2.almost_equals(StereoSample::from(0.3 * 0.6)));

        // Third.
        assert!(o.unpatch_all().is_ok());
        assert!(o
            .patch_chain_to_main_mixer(&[synth_1_uid, gain_3_uid])
            .is_ok());
        o.gather_audio(&mut samples);
        let sample_chain_3 = samples[0];
        assert_eq!(sample_chain_3, StereoSample::from(0.5 * 0.8));

        // Fourth.
        assert!(o.unpatch_all().is_ok());
        assert!(o.patch_chain_to_main_mixer(&[drum_1_uid]).is_ok());
        o.gather_audio(&mut samples);
        let sample_chain_4 = samples[0];
        assert!(sample_chain_4.almost_equals(StereoSample::from(0.7)));

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
        let mut o = Orchestrator::new_with(ClockNano {
            bpm: DEFAULT_BPM,
            midi_ticks_per_second: DEFAULT_MIDI_TICKS_PER_SECOND,
            time_signature: TimeSignature { top: 4, bottom: 4 },
        });
        o.reset(DEFAULT_SAMPLE_RATE);
        let instrument_1_uid = o.add(Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(
            ToyAudioSourceNano { level: 0.1 },
        ))));
        let instrument_2_uid = o.add(Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(
            ToyAudioSourceNano { level: 0.3 },
        ))));
        let instrument_3_uid = o.add(Entity::ToyAudioSource(Box::new(ToyAudioSource::new_with(
            ToyAudioSourceNano { level: 0.5 },
        ))));
        let effect_1_uid = o.add(Entity::Gain(Box::new(Gain::new_with(GainNano {
            ceiling: Normal::new(0.5),
        }))));

        assert!(o.patch_chain_to_main_mixer(&[instrument_1_uid]).is_ok());
        assert!(o.patch_chain_to_main_mixer(&[effect_1_uid]).is_ok());
        assert!(o.patch(instrument_2_uid, effect_1_uid).is_ok());
        assert!(o.patch(instrument_3_uid, effect_1_uid).is_ok());
        let mut samples: [StereoSample; 1] = Default::default();
        o.gather_audio(&mut samples);
        assert!(samples[0].almost_equals(StereoSample::from(0.1 + 0.5 * (0.3 + 0.5))));
    }

    #[test]
    fn run_buffer_size_can_be_odd_number() {
        let mut o = Orchestrator::new_with(ClockNano {
            bpm: DEFAULT_BPM,
            midi_ticks_per_second: DEFAULT_MIDI_TICKS_PER_SECOND,
            time_signature: TimeSignature { top: 4, bottom: 4 },
        });
        o.reset(DEFAULT_SAMPLE_RATE);
        let _ = o.add(Entity::Timer(Box::new(Timer::new_with(TimerNano {
            seconds: 1.0,
        }))));

        // Prime number
        let mut sample_buffer = [StereoSample::SILENCE; 17];
        let r = o.run(&mut sample_buffer);
        assert!(r.is_ok());
        assert_eq!(r.unwrap().len(), DEFAULT_SAMPLE_RATE);
    }

    #[test]
    fn orchestrator_sample_count_is_accurate_for_zero_timer() {
        let mut o = Orchestrator::new_with(ClockNano {
            bpm: DEFAULT_BPM,
            midi_ticks_per_second: DEFAULT_MIDI_TICKS_PER_SECOND,
            time_signature: TimeSignature { top: 4, bottom: 4 },
        });
        o.reset(DEFAULT_SAMPLE_RATE);
        let _ = o.add(Entity::Timer(Box::new(Timer::new_with(TimerNano {
            seconds: 0.0,
        }))));
        let mut sample_buffer = [StereoSample::SILENCE; 64];
        if let Ok(samples) = o.run(&mut sample_buffer) {
            assert_eq!(samples.len(), 0);
        } else {
            panic!("run failed");
        }
    }

    #[test]
    fn orchestrator_sample_count_is_accurate_for_short_timer() {
        let mut o = Orchestrator::new_with(ClockNano {
            bpm: DEFAULT_BPM,
            midi_ticks_per_second: DEFAULT_MIDI_TICKS_PER_SECOND,
            time_signature: TimeSignature { top: 4, bottom: 4 },
        });
        o.reset(DEFAULT_SAMPLE_RATE);
        let _ = o.add(Entity::Timer(Box::new(Timer::new_with(
            groove_entities::controllers::TimerNano {
                seconds: 1.0 / DEFAULT_SAMPLE_RATE as f64,
            },
        ))));
        let mut sample_buffer = [StereoSample::SILENCE; 64];
        if let Ok(samples) = o.run(&mut sample_buffer) {
            assert_eq!(samples.len(), 1);
        } else {
            panic!("run failed");
        }
    }

    #[test]
    fn orchestrator_sample_count_is_accurate_for_ordinary_timer() {
        let mut o = Orchestrator::new_with(ClockNano {
            bpm: DEFAULT_BPM,
            midi_ticks_per_second: DEFAULT_MIDI_TICKS_PER_SECOND,
            time_signature: TimeSignature { top: 4, bottom: 4 },
        });
        o.reset(DEFAULT_SAMPLE_RATE);
        let _ = o.add(Entity::Timer(Box::new(Timer::new_with(TimerNano {
            seconds: 1.0,
        }))));
        let mut sample_buffer = [StereoSample::SILENCE; 64];
        if let Ok(samples) = o.run(&mut sample_buffer) {
            assert_eq!(samples.len(), 44100);
        } else {
            panic!("run failed");
        }
    }

    #[test]
    fn patch_fails_with_bad_id() {
        let mut o = Orchestrator::new_with(ClockNano {
            bpm: DEFAULT_BPM,
            midi_ticks_per_second: DEFAULT_MIDI_TICKS_PER_SECOND,
            time_signature: TimeSignature { top: 4, bottom: 4 },
        });
        assert!(o.patch(3, 2).is_err());
    }

    // TODO: a bunch of these tests belong in the entities crate, but I
    // implemented them using Orchestrator, so they can't fit there now.
    // Reimplement as smaller tests.

    #[test]
    fn pattern_default_note_value() {
        let time_signature = TimeSignature::new_with(7, 4).expect("failed");
        let mut sequencer = Sequencer::new_with(SequencerNano { bpm: 128.0 });
        let mut programmer = PatternProgrammer::new_with(&time_signature);
        let pattern = Pattern {
            note_value: None,
            notes: vec![vec![Note {
                key: 1,
                velocity: 127,
                duration: PerfectTimeUnit(1.0),
            }]],
        };
        programmer.insert_pattern_at_cursor(&mut sequencer, &0, &pattern);

        assert_eq!(
            programmer.cursor(),
            PerfectTimeUnit::from(time_signature.top)
        );
    }

    #[test]
    fn random_access() {
        const INSTRUMENT_MIDI_CHANNEL: MidiChannel = 7;
        let mut o = Orchestrator::new_with(ClockNano {
            bpm: DEFAULT_BPM,
            midi_ticks_per_second: DEFAULT_MIDI_TICKS_PER_SECOND,
            time_signature: TimeSignature { top: 4, bottom: 4 },
        });
        o.reset(DEFAULT_SAMPLE_RATE);
        let mut sequencer = Box::new(Sequencer::new_with(SequencerNano { bpm: DEFAULT_BPM }));
        let mut programmer = PatternProgrammer::new_with(&TimeSignature::default());
        let mut pattern = Pattern::<Note>::default();

        const NOTE_VALUE: BeatValue = BeatValue::Quarter;
        pattern.note_value = Some(NOTE_VALUE);
        pattern.notes.push(vec![
            // Normal duration
            Note {
                key: 1,
                velocity: 40,
                duration: PerfectTimeUnit(1.0),
            },
            // A little bit shorter
            Note {
                key: 2,
                velocity: 41,
                duration: PerfectTimeUnit(0.99),
            },
            // A little bit longer
            Note {
                key: 3,
                velocity: 42,
                duration: PerfectTimeUnit(1.01),
            },
            // Zero duration!
            Note {
                key: 4,
                velocity: 43,
                duration: PerfectTimeUnit(0.0),
            },
        ]);
        programmer.insert_pattern_at_cursor(&mut sequencer, &INSTRUMENT_MIDI_CHANNEL, &pattern);

        let midi_recorder = Box::new(ToyInstrument::new_with(ToyInstrumentNano {
            fake_value: Normal::from(0.22222),
        }));
        let midi_recorder_uid = o.add(Entity::ToyInstrument(midi_recorder));
        o.connect_midi_downstream(midi_recorder_uid, INSTRUMENT_MIDI_CHANNEL);

        // Test recorder has seen nothing to start with.
        // TODO assert!(midi_recorder.debug_messages.is_empty());

        let mut o = Orchestrator::new_with(ClockNano {
            bpm: DEFAULT_BPM,
            midi_ticks_per_second: DEFAULT_MIDI_TICKS_PER_SECOND,
            time_signature: TimeSignature { top: 4, bottom: 4 },
        });
        o.reset(DEFAULT_SAMPLE_RATE);
        let _sequencer_uid = o.add(Entity::Sequencer(sequencer));

        let mut sample_buffer = [StereoSample::SILENCE; 64];
        if let Ok(samples) = o.run(&mut sample_buffer) {
            // We should have gotten one on and one off for each note in the
            // pattern.
            // TODO
            // assert_eq!(
            //     midi_recorder.debug_messages.len(),
            //     pattern.notes[0].len() * 2
            // );

            // TODO sequencer.debug_dump_events();

            // The comment below is incorrect; it was true when the beat sequencer
            // ended after sending the last note event, rather than thinking in
            // terms of full measures.
            //
            // WRONG: The clock should stop at the last note-off, which is 1.01
            // WRONG: beats past the start of the third note, which started at 2.0.
            // WRONG: Since the fourth note is zero-duration, it actually ends at 3.0,
            // WRONG: before the third note's note-off event happens.
            const LAST_BEAT: f64 = 4.0;
            assert_eq!(
                samples.len(),
                (LAST_BEAT * 60.0 / DEFAULT_BPM * DEFAULT_SAMPLE_RATE as f64).ceil() as usize
            );
        } else {
            assert!(false, "run failed");
        }

        // Start test recorder over again.
        // TODO midi_recorder.debug_messages.clear();

        // Rewind clock to start.
        o.skip_to_start();
        o.play();
        let mut samples = [StereoSample::SILENCE; 1];
        // This shouldn't explode.
        let _ = o.tick(&mut samples);

        // Only the first time slice's events should have fired.
        // TODO assert_eq!(midi_recorder.debug_messages.len(), 1);

        // Fast-forward to the end. Nothing else should fire. This is because
        // any tick() should do work for just the slice specified.
        //clock.debug_set_seconds(10.0);
        let _ = o.tick(&mut samples);
        // TODO assert_eq!(midi_recorder.debug_messages.len(), 1);

        // Start test recorder over again.
        // TODO midi_recorder.debug_messages.clear();

        // Move just past first note.
        // clock.set_samples(1); TODO: I don't think this is actually testing anything
        // because I don't think clock was connected to orchestrator

        let mut sample_buffer = [StereoSample::SILENCE; 64];

        // Keep going until just before half of second beat. We should see the
        // first note off (not on!) and the second note on/off.
        let _ = o.add(Entity::Timer(Box::new(Timer::new_with(TimerNano {
            seconds: 2.0,
        }))));
        assert!(o.run(&mut sample_buffer).is_ok());
        // TODO assert_eq!(midi_recorder.debug_messages.len(), 3);

        // Keep ticking through start of second beat. Should see one more event:
        // #3 on.
        assert!(o.run(&mut sample_buffer).is_ok());
        // TODO dbg!(&midi_recorder.debug_messages);
        // TODO assert_eq!(midi_recorder.debug_messages.len(), 4);
    }

    // A pattern of all zeroes should last as long as a pattern of nonzeroes.
    #[test]
    fn empty_pattern() {
        let time_signature = TimeSignature::default();
        let mut sequencer = Box::new(Sequencer::new_with(SequencerNano { bpm: DEFAULT_BPM }));
        let mut programmer = PatternProgrammer::new_with(&time_signature);

        let note_pattern = vec![Note {
            key: 0,
            velocity: 127,
            duration: PerfectTimeUnit(1.0),
        }];
        let pattern = Pattern {
            note_value: Some(BeatValue::Quarter),
            notes: vec![note_pattern],
        };

        assert_eq!(pattern.notes.len(), 1); // one track of notes
        assert_eq!(pattern.notes[0].len(), 1); // one note in track

        programmer.insert_pattern_at_cursor(&mut sequencer, &0, &pattern);
        assert_eq!(
            programmer.cursor(),
            PerfectTimeUnit::from(time_signature.top)
        );
        assert_eq!(sequencer.debug_events().len(), 0);

        let mut o = Orchestrator::new_with(ClockNano {
            bpm: DEFAULT_BPM,
            midi_ticks_per_second: DEFAULT_MIDI_TICKS_PER_SECOND,
            time_signature: TimeSignature { top: 4, bottom: 4 },
        });
        o.reset(DEFAULT_SAMPLE_RATE);
        let _ = o.add(Entity::Sequencer(sequencer));
        let mut sample_buffer = [StereoSample::SILENCE; 64];
        if let Ok(result) = o.run(&mut sample_buffer) {
            assert_eq!(
                result.len(),
                ((60.0 * 4.0 / DEFAULT_BPM) * DEFAULT_SAMPLE_RATE as f64).ceil() as usize
            );
        }
    }

    // Orchestrator sends a Tick message to everyone in an undefined order, and
    // routes the resulting messages to everyone in yet another undefined order.
    // This causes a problem. If we have a sequencer driving an arpeggiator, and
    // the two together are supposed to play a note at Time 0, then it's
    // possible that the events will happen as follows:
    //
    // Tick to Arp -> nothing emitted, because it's not playing Tick to
    // Sequencer -> emit Midi, delivered straight to Arp
    //
    // and that's pretty much it, because the event loop is done. Worse, the Arp
    // will never send the note-on MIDI message to its downstream instrument(s),
    // because by the time of its next Tick (when it calculates when to send
    // stuff), it's Time 1, but the note should have been sent at Time 0, so
    // that note-on is skipped.
    #[test]
    fn sequencer_to_arp_to_instrument_works() {
        let mut clock = Clock::new_with(ClockNano {
            bpm: DEFAULT_BPM,
            midi_ticks_per_second: DEFAULT_MIDI_TICKS_PER_SECOND,
            time_signature: TimeSignature { top: 4, bottom: 4 },
        });
        let mut sequencer = Box::new(Sequencer::new_with(SequencerNano { bpm: clock.bpm() }));
        const MIDI_CHANNEL_SEQUENCER_TO_ARP: MidiChannel = 7;
        const MIDI_CHANNEL_ARP_TO_INSTRUMENT: MidiChannel = 8;
        let mut arpeggiator = Box::new(Arpeggiator::new_with(
            MIDI_CHANNEL_ARP_TO_INSTRUMENT,
            ArpeggiatorNano { bpm: clock.bpm() },
        ));
        let instrument = Box::new(ToyInstrument::new_with(ToyInstrumentNano {
            fake_value: Normal::from(0.332948),
        }));
        let mut o = Orchestrator::new_with(ClockNano {
            bpm: DEFAULT_BPM,
            midi_ticks_per_second: DEFAULT_MIDI_TICKS_PER_SECOND,
            time_signature: TimeSignature { top: 4, bottom: 4 },
        });
        arpeggiator.reset(DEFAULT_SAMPLE_RATE);
        o.reset(DEFAULT_SAMPLE_RATE);
        clock.reset(DEFAULT_SAMPLE_RATE);

        sequencer.insert(
            PerfectTimeUnit(0.0),
            MIDI_CHANNEL_SEQUENCER_TO_ARP,
            MidiMessage::NoteOn {
                key: 99.into(),
                vel: 88.into(),
            },
        );

        let _sequencer_uid = o.add(Entity::Sequencer(sequencer));
        let arpeggiator_uid = o.add(Entity::Arpeggiator(arpeggiator));
        o.connect_midi_downstream(arpeggiator_uid, MIDI_CHANNEL_SEQUENCER_TO_ARP);
        let instrument_uid = o.add(Entity::ToyInstrument(instrument));
        o.connect_midi_downstream(instrument_uid, MIDI_CHANNEL_ARP_TO_INSTRUMENT);

        let _ = o.connect_to_main_mixer(instrument_uid);
        let mut buffer = [StereoSample::SILENCE; 64];
        let performance = o.run(&mut buffer);
        if let Ok(samples) = performance {
            assert!(samples.iter().any(|s| *s != StereoSample::SILENCE));

            // TODO: this assertion fails for serious reasons: Orchestrator
            // calls entity tick() methods in arbitrary order, so depending on
            // how the hash-table gods are feeling, the sequencer might trigger
            // the arp on this cycle, or the arp might run first and decide
            // there's nothing to do. If we want to get this right, then either
            // there's an iterative step where we keep dispatching messages
            // until there are no more (and risk cycles), or beforehand we
            // determine the dependency graph (and detect cycles), and then call
            // everyone in the right order.
            #[cfg(disabled)]
            assert_ne!(samples[0], StereoSample::SILENCE, "if the sequencer drove the arp, and the arp drove the instrument, then we should hear sound on sample #0");
        } else {
            panic!("run failed");
        }
    }
}
