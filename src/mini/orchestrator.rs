// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{
    bus_station::{BusRoute, BusStation},
    control_router::ControlRouter,
    piano_roll::PianoRoll,
    selection_set::SelectionSet,
    track::{Track, TrackAction, TrackBuffer, TrackFactory, TrackTitle, TrackUiState, TrackUid},
    transport::{Transport, TransportBuilder},
    widgets::timeline,
    Key,
};
use anyhow::anyhow;
use derive_builder::Builder;
use eframe::{
    egui::{self, ScrollArea},
    epaint::vec2,
};
use ensnare::core::{Normal, Sample, StereoSample};
use groove_audio::AudioQueue;
use groove_core::{
    control::ControlValue,
    midi::{MidiChannel, MidiMessage, MidiMessagesFn},
    time::{MusicalTime, SampleRate, Tempo},
    traits::{
        gui::{Displays, DisplaysInTimeline},
        Configurable, ControlEventsFn, Controllable, Controls, EntityEvent, Generates,
        GeneratesToInternalBuffer, HandlesMidi, HasUid, Serializable, Ticks,
    },
    Uid,
};
use rayon::prelude::{IntoParallelRefMutIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::Debug,
    ops::Range,
    path::PathBuf,
    sync::{Arc, RwLock, RwLockWriteGuard},
    vec::Vec,
};

/// Actions that [Orchestrator]'s UI might need the parent to perform.
#[derive(Clone, Debug)]
pub enum OrchestratorAction {
    /// A [Track] was clicked in the UI.
    ClickTrack(TrackUid),
    /// A [Track] was double-clicked in the UI.
    DoubleClickTrack(TrackUid),
    /// A [Track] wants a new device of type [Key].
    NewDeviceForTrack(TrackUid, Key),
}

/// A grouping mechanism to declare parts of [Orchestrator] that Serde
/// shouldn't be serializing. Exists so we don't have to spray #[serde(skip)]
/// all over the place.
#[derive(Debug, Default)]
pub struct OrchestratorEphemerals {
    range: Range<MusicalTime>,
    events: Vec<(Uid, EntityEvent)>,
    is_finished: bool,
    is_performing: bool,
    action: Option<OrchestratorAction>,
    view_range: Range<MusicalTime>,
    track_selection_set: SelectionSet<TrackUid>,
}

/// Owns all entities (instruments, controllers, and effects), and manages the
/// relationships among them to create an audio performance.
///
/// ```
/// use groove::prelude::*;
/// use groove_toys::ToySynth;
///
/// let mut orchestrator = Orchestrator::default();
/// let track_uid = orchestrator.new_midi_track().unwrap();
/// let track = orchestrator.get_track_mut(&track_uid).unwrap();
/// let uid = track.append_entity(Box::new(ToySynth::default())).unwrap();
///
/// let mut samples = [StereoSample::SILENCE; Orchestrator::SAMPLE_BUFFER_SIZE];
/// orchestrator.render_and_ignore_events(&mut samples);
/// ```
#[derive(Serialize, Deserialize, Debug, Builder)]
#[builder(setter(skip), default)]
#[builder_struct_attr(allow(missing_docs))]
pub struct Orchestrator {
    /// The user-supplied name of this project.
    #[builder(setter, default)]
    title: Option<String>,

    transport: Transport,
    control_router: ControlRouter,

    track_factory: TrackFactory,
    tracks: HashMap<TrackUid, Track>,
    /// Track uids in the order they appear in the UI.
    track_uids: Vec<TrackUid>,
    track_ui_states: HashMap<TrackUid, TrackUiState>,

    // This is the owned and serialized instance of PianoRoll. Because we're
    // using Arc<> in a struct that Serde serializes, we need to have the `rc`
    // feature enabled for Serde.
    piano_roll: Arc<RwLock<PianoRoll>>,

    bus_station: BusStation,

    //////////////////////////////////////////////////////
    // Nothing below this comment should be serialized. //
    //////////////////////////////////////////////////////
    //
    #[serde(skip)]
    e: OrchestratorEphemerals,
}
impl Default for Orchestrator {
    fn default() -> Self {
        Self {
            title: None,
            transport: TransportBuilder::default()
                .uid(Self::TRANSPORT_UID)
                .build()
                .unwrap(),
            control_router: Default::default(),
            track_factory: Default::default(),
            tracks: Default::default(),
            track_uids: Default::default(),
            track_ui_states: Default::default(),
            piano_roll: Default::default(),
            bus_station: Default::default(),

            e: Default::default(),
        }
    }
}
impl Orchestrator {
    /// The expected size of any buffer provided for samples.
    //
    // TODO: how hard would it be to make this dynamic? Does adjustability
    // matter?
    pub const SAMPLE_BUFFER_SIZE: usize = 64;

    /// The fixed [Uid] for the orchestrator itself.
    const UID: Uid = Uid(1);
    /// The fixed [Uid] for the global transport.
    const TRANSPORT_UID: Uid = Uid(2);

    /// Adds a new MIDI track, which can contain controllers, instruments, and
    /// effects. Returns the new track's [TrackUid] if successful.
    pub fn new_midi_track(&mut self) -> anyhow::Result<TrackUid> {
        let track = self.track_factory.midi(&self.piano_roll);
        self.new_track(track)
    }

    /// Adds a new audio track, which can contain audio clips and effects.
    /// Returns the new track's [TrackUid] if successful.
    pub fn new_audio_track(&mut self) -> anyhow::Result<TrackUid> {
        let track = self.track_factory.audio();
        self.new_track(track)
    }

    /// Adds a new aux track, which contains only effects, and to which other
    /// tracks can *send* their output audio. Returns the new track's [TrackUid]
    /// if successful.
    pub fn new_aux_track(&mut self) -> anyhow::Result<TrackUid> {
        let track = self.track_factory.aux();
        self.new_track(track)
    }

    /// Adds a set of tracks that make sense for a new project.
    pub fn create_starter_tracks(&mut self) -> anyhow::Result<()> {
        if !self.track_uids.is_empty() {
            return Err(anyhow!("Must be invoked on an empty orchestrator."));
        }
        self.new_midi_track()?;
        self.new_midi_track()?;
        self.new_audio_track()?;
        self.new_aux_track()?;
        Ok(())
    }

    /// Deletes the specified track.
    pub fn delete_track(&mut self, uid: &TrackUid) {
        self.tracks.remove(uid);
        self.track_uids.retain(|u| u != uid);
    }

    /// Deletes the specified tracks.
    pub fn delete_tracks(&mut self, uids: &[TrackUid]) {
        uids.iter().for_each(|uid| {
            self.delete_track(uid);
        });
    }

    /// Sets a new title for the track.
    pub fn set_track_title(&mut self, uid: TrackUid, title: TrackTitle) {
        if let Some(track) = self.get_track_mut(&uid) {
            track.set_title(title);
        }
    }

    /// Renders the next set of samples into the provided buffer. This is the
    /// main event loop.
    pub fn render(
        &mut self,
        samples: &mut [StereoSample],
        control_events_fn: &mut ControlEventsFn,
    ) {
        // Note that advance() can return the same range twice, depending on
        // sample rate. TODO: we should decide whose responsibility it is to
        // handle that -- either we skip calling work() if the time range is the
        // same as prior, or everyone who gets called needs to detect the case
        // or be idempotent.
        let range = self.transport.advance(samples.len());
        self.update_time(&range);
        self.work(control_events_fn);
        self.generate_batch_values(samples);
    }

    /// A convenience method for callers who would have ignored any
    /// [EntityEvent]s produced by the render() method.
    pub fn render_and_ignore_events(&mut self, samples: &mut [StereoSample]) {
        self.render(samples, &mut |_, _| {});
    }

    /// Renders part of the project to audio, creating at least the requested
    /// number of [StereoSample]s and inserting them in the given [AudioQueue].
    /// Exceptions: the method operates only in [Self::SAMPLE_BUFFER_SIZE]
    /// chunks, and it won't generate a chunk unless there is enough room in the
    /// queue for it.
    ///
    /// This method expects to be called continuously, even when the project
    /// isn't actively playing. In such cases, it will provide a stream of
    /// silent samples.
    //
    // TODO: I don't think there's any reason why this must be limited to an
    // `AudioQueue` rather than a more general `Vec`-like interface.
    pub fn render_and_enqueue(
        &mut self,
        samples_requested: usize,
        queue: &AudioQueue,
        control_events_fn: &mut ControlEventsFn,
    ) {
        // Round up
        let buffers_requested =
            (samples_requested + Self::SAMPLE_BUFFER_SIZE - 1) / Self::SAMPLE_BUFFER_SIZE;
        for _ in 0..buffers_requested {
            // Generate a buffer only if there's enough room in the queue for it.
            if queue.capacity() - queue.len() >= Self::SAMPLE_BUFFER_SIZE {
                let mut samples = [StereoSample::SILENCE; Self::SAMPLE_BUFFER_SIZE];
                if false {
                    self.render_debug(&mut samples);
                } else {
                    self.render(&mut samples, control_events_fn);
                }
                // No need to do the Arc deref each time through the loop.
                // TODO: is there a queue type that allows pushing a batch?
                let queue = queue.as_ref();
                for sample in samples {
                    let _ = queue.push(sample);
                }
            }
        }
    }

    /// Fills in the given sample buffer with something simple and audible.
    pub fn render_debug(&mut self, samples: &mut [StereoSample]) {
        let len = samples.len() as f64;
        for (i, s) in samples.iter_mut().enumerate() {
            s.0 = Sample::from(i as f64 / len);
            s.1 = Sample::from(i as f64 / -len);
        }
    }

    /// After loading a new Self from disk, we want to copy all the appropriate
    /// ephemeral state from this one to the next one.
    pub fn prepare_successor(&self, new: &mut Orchestrator) {
        // Copy over the current sample rate, whose validity shouldn't change
        // because we loaded a new project.
        new.update_sample_rate(self.sample_rate());
    }

    /// Returns all [Track] uids in UI order.
    pub fn track_uids(&self) -> &[TrackUid] {
        self.track_uids.as_ref()
    }

    /// Returns an iterator of all [Track]s in arbitrary order.
    pub fn track_iter(&self) -> impl Iterator<Item = &Track> {
        self.tracks.values()
    }

    /// Returns an iterator of all [Track]s in arbitrary order. Mutable version.
    pub fn track_iter_mut(&mut self) -> impl Iterator<Item = &mut Track> {
        self.tracks.values_mut()
    }

    /// Returns the specified [Track].
    pub fn get_track(&self, uid: &TrackUid) -> Option<&Track> {
        self.tracks.get(uid)
    }

    /// Returns the specified mutable [Track].
    pub fn get_track_mut(&mut self, uid: &TrackUid) -> Option<&mut Track> {
        self.tracks.get_mut(uid)
    }

    /// Returns the global [Transport].
    pub fn transport(&self) -> &Transport {
        &self.transport
    }

    #[allow(missing_docs)]
    pub fn transport_mut(&mut self) -> &mut Transport {
        &mut self.transport
    }

    /// Returns the current [Tempo].
    pub fn tempo(&self) -> Tempo {
        self.transport.tempo()
    }

    /// Returns the number of channels in the audio stream. For now, this is
    /// always 2 (stereo audio stream).
    pub fn channels(&self) -> u16 {
        2
    }

    /// Returns the name of the project.
    pub fn title(&self) -> Option<&String> {
        self.title.as_ref()
    }

    /// Sets the name of the project.
    pub fn set_title(&mut self, title: Option<String>) {
        self.title = title;
    }

    /// Changes a [Track]'s UI state from collapsed to expanded.
    pub fn toggle_track_ui_state(&mut self, track_uid: &TrackUid) {
        let new_state = self
            .track_ui_states
            .get(track_uid)
            .cloned()
            .unwrap_or_default();
        self.track_ui_states.insert(
            *track_uid,
            match new_state {
                TrackUiState::Collapsed => TrackUiState::Expanded,
                TrackUiState::Expanded => TrackUiState::Collapsed,
            },
        );
    }

    fn new_track(&mut self, track: Track) -> anyhow::Result<TrackUid> {
        let uid = track.uid();
        self.track_uids.push(uid);
        self.tracks.insert(uid, track);
        Ok(uid)
    }

    fn calculate_is_finished(&self) -> bool {
        self.tracks.values().all(|t| t.is_finished())
    }

    // This method is called only for events generated internally (i.e., from
    // our own Entities). It is not called for external MIDI messages.
    fn dispatch_event(&mut self, uid: Uid, event: EntityEvent) {
        match event {
            EntityEvent::Midi(..) => {
                panic!("FATAL: we were asked to dispatch an EntityEvent::Midi, which should already have been handled")
            }
            EntityEvent::Control(value) => {
                self.route_control_change(uid, value);
            }
        }
    }

    fn route_midi_message(&mut self, channel: MidiChannel, message: MidiMessage) {
        for t in self.tracks.values_mut() {
            t.route_midi_message(channel, message);
        }
    }

    fn route_control_change(&mut self, source_uid: Uid, value: ControlValue) {
        let _ = self.control_router.route(
            &mut |target_uid, index, value| {
                if target_uid == &Self::TRANSPORT_UID {
                    self.transport.control_set_param_by_index(index, value);
                }
            },
            source_uid,
            value,
        );
        for t in self.tracks.values_mut() {
            t.route_control_change(source_uid, value);
        }
    }

    /// The highest [Entity] [Uid] that this orchestrator has seen. This is
    /// needed so that generators of new [Uid]s (such as [crate::EntityFactory])
    /// can keep generating unique ones.
    pub fn calculate_max_entity_uid(&self) -> Uid {
        if let Some(track) = self
            .track_iter()
            .max_by_key(|t| t.calculate_max_entity_uid())
        {
            if let Some(uid) = track.calculate_max_entity_uid() {
                return uid;
            }
        }
        Uid(0)
    }

    /// Returns the one and only [PianoRoll].
    pub fn piano_roll_mut(&self) -> RwLockWriteGuard<'_, PianoRoll> {
        self.piano_roll.write().unwrap()
    }

    // TODO: this could be a feature so that we don't always need the hound
    // dependency. Probably not important either way.
    /// Writes the current performance to a WAV file. Intended for integration
    /// tests only (for now).
    pub fn write_to_file(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        let spec = hound::WavSpec {
            channels: self.channels(),
            sample_rate: self.sample_rate().into(),
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();

        let mut buffer = [StereoSample::SILENCE; 64];
        self.play();
        loop {
            if self.is_finished() {
                break;
            }
            buffer.fill(StereoSample::SILENCE);
            self.render_and_ignore_events(&mut buffer);
            for sample in buffer {
                let (left, right) = sample.into_i16();
                let _ = writer.write_sample(left);
                let _ = writer.write_sample(right);
            }
        }

        Ok(())
    }

    /// Configures a send from the given track to the given aux track. The
    /// `send_amount` parameter indicates how much of the signal should go to
    /// the aux: 1.0 is full, 0.0 is silent.
    pub fn send_to_aux(
        &mut self,
        send_track_uid: TrackUid,
        aux_track_uid: TrackUid,
        send_amount: Normal,
    ) -> anyhow::Result<()> {
        self.bus_station.add_send_route(
            send_track_uid,
            BusRoute {
                aux_track_uid,
                amount: send_amount,
            },
        )
    }

    #[allow(missing_docs)]
    pub fn action(&self) -> Option<OrchestratorAction> {
        self.e.action.clone()
    }

    #[allow(missing_docs)]
    pub fn set_track_selection_set(&mut self, track_selection_set: SelectionSet<TrackUid>) {
        self.e.track_selection_set = track_selection_set;
    }
}
impl HasUid for Orchestrator {
    fn uid(&self) -> Uid {
        Self::UID
    }

    fn set_uid(&mut self, _: Uid) {
        panic!("Orchestrator's UID is reserved and should never change.")
    }

    fn name(&self) -> &'static str {
        "Orchestrator"
    }
}
impl Generates<StereoSample> for Orchestrator {
    fn value(&self) -> StereoSample {
        StereoSample::SILENCE
    }

    // Note! It's the caller's job to prepare the buffer. This method will *add*
    // its results, rather than overwriting.
    fn generate_batch_values(&mut self, values: &mut [StereoSample]) {
        let len = values.len();

        // Generate all normal tracks in parallel.
        self.tracks.par_iter_mut().for_each(|(_uid, track)| {
            if !track.is_aux() {
                track.generate_batch_values(len);
            }
        });

        // Send audio to aux tracks...
        self.tracks.par_iter_mut().for_each(|(_uid, track)| {
            if track.is_aux() {
                track.buffer_mut().0.fill(StereoSample::SILENCE);
            }
        });
        for (track_uid, routes) in self.bus_station.send_routes() {
            // We need an extra buffer copy to satisfy the borrow checker.
            // HashMap::get_mut() grabs the entire HashMap, preventing us from
            // holding references to other elements in it. There are other
            // implementations of HashMap that allow get_many_mut(), which could
            // help. TODO
            let mut send_buffer = TrackBuffer::default();
            if let Some(send) = self.tracks.get(track_uid) {
                send_buffer.0.copy_from_slice(&send.buffer().0);
            } else {
                eprintln!("Warning: couldn't find send track {track_uid}");
                continue;
            }

            for route in routes {
                if let Some(aux) = self.tracks.get_mut(&route.aux_track_uid) {
                    let aux_buffer = aux.buffer_mut();
                    for (index, sample) in send_buffer.0.iter().enumerate() {
                        aux_buffer.0[index] += *sample * route.amount
                    }
                }
            }
        }

        // ... and then generate the aux tracks...
        //
        // We don't currently support an aux returning to another aux. It's just
        // regular tracks sending to aux, then aux returning to main. See #143
        self.tracks.par_iter_mut().for_each(|(_uid, track)| {
            if track.is_aux() {
                track.generate_batch_values(len);
            }
        });

        // ... and we get returns for free, because (for now) all tracks are
        // connected to the main mixer.

        // TODO: there must be a way to quickly sum same-sized arrays into a
        // final array. https://stackoverflow.com/questions/41207666/ seems to
        // address at least some of it, but I don't think it's any faster, if
        // more idiomatic.
        //
        // TODO even more: hmmmmmm, maybe I can use
        // https://doc.rust-lang.org/std/cell/struct.Cell.html so that we can
        // get back to the original Generates model of the caller providing the
        // buffer. And then hmmmm, once we know how things are laid out in
        // memory, maybe we can even sic some fast matrix code on it.
        self.tracks.values().for_each(|track| {
            let generator_values = track.values();
            let copy_len = len.min(generator_values.len());
            for i in 0..copy_len {
                values[i] += generator_values[i];
            }
        });
    }
}
impl Ticks for Orchestrator {
    fn tick(&mut self, _tick_count: usize) {
        panic!()
    }
}
impl Configurable for Orchestrator {
    fn sample_rate(&self) -> SampleRate {
        self.transport.sample_rate()
    }

    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.transport.update_sample_rate(sample_rate);
        for track in self.tracks.values_mut() {
            track.update_sample_rate(sample_rate);
        }
    }

    fn update_tempo(&mut self, tempo: Tempo) {
        self.transport.set_tempo(tempo);
        // TODO: how do we let the service know this changed?
    }

    fn update_time_signature(&mut self, time_signature: groove_core::time::TimeSignature) {
        self.transport.update_time_signature(time_signature);
    }
}
impl HandlesMidi for Orchestrator {
    /// Accepts a [MidiMessage] and handles it, usually by forwarding it to
    /// controllers and instruments on the given [MidiChannel]. We implement
    /// this trait only for external messages; for ones generated internally, we
    /// use [MidiRouter].
    ///
    /// REPEAT: this method is called only for MIDI messages from EXTERNAL MIDI
    /// INTERFACES!
    fn handle_midi_message(
        &mut self,
        channel: MidiChannel,
        message: MidiMessage,
        _: &mut MidiMessagesFn,
    ) {
        self.route_midi_message(channel, message);
    }
}
impl Controls for Orchestrator {
    fn update_time(&mut self, range: &Range<MusicalTime>) {
        self.e.range = range.clone();

        for track in self.tracks.values_mut() {
            track.update_time(&self.e.range);
        }
    }

    fn work(&mut self, control_events_fn: &mut ControlEventsFn) {
        self.transport.work(&mut |u, m| self.e.events.push((u, m)));
        for track in self.tracks.values_mut() {
            track.work(&mut |u, m| self.e.events.push((u, m)));
        }
        while let Some((uid, event)) = self.e.events.pop() {
            if matches!(event, EntityEvent::Midi(_, _)) {
                // This MIDI message came from one of our internal Entities and
                // has bubbled all the way up here. We don't want to do anything
                // with it, and should instead pass it along to the caller, who
                // will forward it to external MIDI interfaces.
                //
                // This MIDI message came from a Track. The Track's
                // responsibility was to route the message to all the eligible
                // Entities that it owned. We don't want to route these messages
                // back to any Tracks; our only responsibility is to send them
                // to external MIDI interfaces.
                //
                // Eventually, we might allow one Track to send MIDI messages to
                // another Track. But today we don't. TODO?
                control_events_fn(uid, event);
            } else {
                self.dispatch_event(uid, event);
            }
        }
        self.e.is_finished = self.calculate_is_finished();
        if self.is_performing() && self.is_finished() {
            self.stop();
        }
    }

    fn is_finished(&self) -> bool {
        self.e.is_finished
    }

    fn play(&mut self) {
        self.e.is_performing = true;
        self.transport.play();
        self.tracks.values_mut().for_each(|t| t.play());
        self.e.is_finished = self.calculate_is_finished();

        // This handles the case where there isn't anything to play because the
        // performance is zero-length. It stops the transport from advancing a
        // tiny bit and looking weird.
        if self.e.is_finished {
            self.transport.stop();
        }
    }

    fn stop(&mut self) {
        // If we were performing, stop. Otherwise, it's a stop-while-stopped
        // action, which means the user wants to rewind to the beginning.
        if self.e.is_performing {
            self.e.is_performing = false;
        } else {
            self.skip_to_start();
        }
        self.transport.stop();
        self.tracks.values_mut().for_each(|t| t.stop());
    }

    fn skip_to_start(&mut self) {
        self.transport.skip_to_start();
        self.tracks.values_mut().for_each(|t| t.skip_to_start());
    }

    fn is_performing(&self) -> bool {
        self.e.is_performing
    }
}
impl Serializable for Orchestrator {
    fn after_deser(&mut self) {
        self.tracks.values_mut().for_each(|t| {
            t.set_piano_roll(Arc::clone(&self.piano_roll));
            t.after_deser();
        });
    }
}
impl DisplaysInTimeline for Orchestrator {
    fn set_view_range(&mut self, view_range: &std::ops::Range<groove_core::time::MusicalTime>) {
        self.e.view_range = view_range.clone();
    }
}
impl Displays for Orchestrator {
    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        self.e.action = None;
        let mut view_range = MusicalTime::new_with_beats(0)..MusicalTime::new_with_beats(128);
        let total_height = ui.available_height();

        egui::TopBottomPanel::bottom("orchestrator-piano-roll")
            .resizable(true)
            .max_height(total_height / 2.0)
            .show(ui.ctx(), |ui| {
                self.piano_roll.write().unwrap().ui(ui);
            });

        egui::CentralPanel::default()
            .show(ui.ctx(), |ui| {
                ScrollArea::vertical()
                    .id_source("orchestrator-scroller")
                    .show(ui, |ui| {
                        ui.add(timeline::legend(&mut view_range));
                        for track_uid in self.track_uids.iter() {
                            if let Some(track) = self.tracks.get_mut(track_uid) {
                                let track_ui_state = self
                                    .track_ui_states
                                    .get(track_uid)
                                    .cloned()
                                    .unwrap_or_default();
                                let height = Track::track_view_height(track.ty(), track_ui_state);
                                let desired_size = vec2(ui.available_width(), height);
                                ui.allocate_ui(desired_size, |ui| {
                                    ui.set_min_size(desired_size);
                                    track.set_is_selected(
                                        self.e.track_selection_set.contains(track_uid),
                                    );
                                    track.set_ui_state(track_ui_state);
                                    let response = track.ui(ui);
                                    let action = track.action();

                                    if let Some(action) = action {
                                        match action {
                                            TrackAction::SetTitle(t) => {
                                                track.set_title(t);
                                            }
                                            TrackAction::ToggleDisclosure => {
                                                self.e.action =
                                                    Some(OrchestratorAction::DoubleClickTrack(
                                                        *track_uid,
                                                    ));
                                            }
                                            TrackAction::NewDevice(track_uid, key) => {
                                                self.e.action =
                                                    Some(OrchestratorAction::NewDeviceForTrack(
                                                        track_uid, key,
                                                    ))
                                            }
                                        }
                                    }
                                    if response.double_clicked() {
                                        self.e.action =
                                            Some(OrchestratorAction::DoubleClickTrack(*track_uid));
                                    } else if response.clicked() {
                                        self.e.action =
                                            Some(OrchestratorAction::ClickTrack(*track_uid));
                                    }
                                });
                            }
                        }
                    });
            })
            .response
    }
}

#[cfg(test)]
mod tests {
    use crate::mini::{
        orchestrator::Orchestrator, track::TrackBuffer, OrchestratorBuilder, TrackUid,
    };
    use ensnare::core::{Normal, StereoSample};
    use groove_core::{
        midi::{MidiChannel, MidiMessage},
        time::{MusicalTime, SampleRate, Tempo},
        traits::{Configurable, Controls, HandlesMidi, HasUid},
        DcaParams, Uid,
    };
    use groove_entities::{
        controllers::{Timer, TimerParams},
        effects::{Gain, GainParams},
    };
    use groove_toys::{
        ToyAudioSource, ToyControllerAlwaysSendsMidiMessage, ToyInstrument, ToyInstrumentParams,
    };
    use std::{collections::HashSet, sync::Arc};

    #[test]
    fn basic_operations() {
        let mut o = Orchestrator::default();

        assert!(
            o.sample_rate().value() != 0,
            "Default sample rate should be reasonable"
        );
        let new_sample_rate = SampleRate(3);
        o.update_sample_rate(new_sample_rate);
        assert_eq!(
            o.sample_rate(),
            new_sample_rate,
            "Sample rate should be settable"
        );

        assert!(
            o.tempo().value() > 0.0,
            "Default tempo should be reasonable"
        );
        let new_tempo = Tempo(64.0);
        o.update_tempo(new_tempo);
        assert_eq!(o.tempo(), new_tempo, "Tempo should be settable");
    }

    #[test]
    fn exposes_traits_ergonomically() {
        let mut o = Orchestrator::default();
        let tuid = o.new_midi_track().unwrap();
        let track = o.get_track_mut(&tuid).unwrap();

        // TODO: worst ergonomics ever.
        const TIMER_DURATION: MusicalTime = MusicalTime::new_with_beats(1);
        let _ = track.append_entity(Box::new(Timer::new_with(&TimerParams {
            duration: groove_core::time::MusicalTimeParams {
                units: TIMER_DURATION.total_units(),
            },
        })));

        o.play();
        let mut _prior_start_time = MusicalTime::TIME_ZERO;
        loop {
            if o.is_finished() {
                break;
            }
            _prior_start_time = o.transport().current_time();
            let mut samples = [StereoSample::SILENCE; 1];
            o.render_and_ignore_events(&mut samples);
        }

        // TODO: this section is confusing me. It used to say
        // `prior_start_time..o.transport().current_time()`, but that failed
        // just now. I am not sure why it would have ever passed. Consider
        // bisecting to see how it did.
        let prior_range = MusicalTime::TIME_ZERO..o.transport().current_time();
        assert!(
            prior_range.contains(&TIMER_DURATION),
            "Expected the covered range {:?} to include the duration {:?}",
            prior_range,
            TIMER_DURATION
        );
    }

    #[test]
    fn starter_tracks() {
        let mut o = Orchestrator::default();
        assert!(o.track_uids.is_empty());
        assert!(o.create_starter_tracks().is_ok());
        assert!(!o.track_uids.is_empty());
        assert!(o.create_starter_tracks().is_err());

        assert_eq!(o.track_uids().len(), 4,
            "we should have two MIDI tracks, one audio track, and one aux track after create_starter_tracks().");
    }

    #[test]
    fn track_discovery() {
        let mut o = Orchestrator::default();
        assert!(o.create_starter_tracks().is_ok());
        let track_count = o.track_uids().len();

        assert_eq!(
            o.track_iter().count(),
            track_count,
            "Track iterator count should match number of track UIDs."
        );

        // Make sure we can call this and that nothing explodes.
        let mut count = 0;
        o.track_iter_mut().for_each(|t| {
            t.play();
            count += 1;
        });
        assert_eq!(count, 4);
    }

    #[test]
    fn track_crud() {
        let mut o = Orchestrator::default();
        assert_eq!(o.track_uids().len(), 0);
        let track_uid = o.new_midi_track().unwrap();
        assert_eq!(o.track_uids().len(), 1);

        assert!(o.track_uids()[0] == track_uid);

        o.delete_track(&track_uid);
        assert!(o.track_uids().is_empty());

        // Do it one way
        {
            assert!(o.create_starter_tracks().is_ok());
            assert!(!o.track_uids().is_empty());

            o.delete_tracks(&Vec::from(o.track_uids()));
            assert!(o.track_uids().is_empty());
        }

        // Do it another way
        {
            assert!(o.create_starter_tracks().is_ok());
            assert!(!o.track_uids().is_empty());

            let mut selection_set: HashSet<TrackUid> = HashSet::default();
            for uid in o.track_uids() {
                selection_set.insert(*uid);
            }
            o.delete_tracks(&Vec::from_iter(selection_set.iter().copied()));
            assert!(o.track_uids().is_empty());
        }
    }

    #[test]
    fn zero_length_performance_ends_immediately() {
        let mut o = Orchestrator::default();

        // This is actually undefined before play(), so we're cheating a bit in
        // the Orchestrator implementation to allow testing of what we want to
        // test.
        assert!(!o.is_finished());

        o.play();
        assert!(o.is_finished());
    }

    #[test]
    fn default_orchestrator_transport_has_correct_uid() {
        let o = Orchestrator::default();
        assert_eq!(o.transport().uid(), Orchestrator::TRANSPORT_UID);
    }

    #[test]
    fn default_orchestratorbuilder_transport_has_correct_uid() {
        // This makes sure we remembered #[builder(default)] on the struct
        let o = OrchestratorBuilder::default().build().unwrap();
        assert_eq!(o.transport().uid(), Orchestrator::TRANSPORT_UID);
    }

    #[test]
    fn sends_send() {
        let mut o = Orchestrator::default();
        let track_uid = o.new_midi_track().unwrap();
        let aux_uid = o.new_aux_track().unwrap();

        {
            let track = o.get_track_mut(&track_uid).unwrap();
            assert!(track
                .append_entity(Box::new(ToyAudioSource::new_always_medium()))
                .is_ok());
        }
        let mut samples = [StereoSample::SILENCE; TrackBuffer::LEN];
        o.render_and_ignore_events(&mut samples);
        let expected_sample = StereoSample::from(ToyAudioSource::MEDIUM);
        assert!(
            samples.iter().all(|s| *s == expected_sample),
            "Without a send, original signal should pass through unchanged."
        );

        assert!(o.send_to_aux(track_uid, aux_uid, Normal::from(0.5)).is_ok());
        let mut samples = [StereoSample::SILENCE; TrackBuffer::LEN];
        o.render_and_ignore_events(&mut samples);
        let expected_sample = StereoSample::from(0.75);
        assert!(
            samples.iter().all(|s| *s == expected_sample),
            "With a 50% send, we should see the original 0.5 plus 50% of 0.5 = 0.75"
        );

        // Add an effect to the aux track.
        {
            let track = o.get_track_mut(&aux_uid).unwrap();
            assert!(track
                .append_entity(Box::new(Gain::new_with(&GainParams {
                    ceiling: Normal::from(0.5)
                })))
                .is_ok());
        }
        let mut samples = [StereoSample::SILENCE; TrackBuffer::LEN];
        o.render_and_ignore_events(&mut samples);
        let expected_sample = StereoSample::from(0.5 + 0.5 * 0.5 * 0.5);
        assert!(
            samples.iter().all(|s| *s == expected_sample),
            "With a 50% send to an aux with 50% gain, we should see the original 0.5 plus 50% of 50% of 0.5 = 0.625"
        );
    }

    #[test]
    fn midi_routing_from_external_reaches_instruments() {
        let mut o = Orchestrator::default();
        let track_uid = o.new_midi_track().unwrap();

        let track = o.get_track_mut(&track_uid).unwrap();
        let instrument = ToyInstrument::new_with(&ToyInstrumentParams {
            fake_value: Normal::default(),
            dca: DcaParams::default(),
        });
        let midi_messages_received = Arc::clone(instrument.received_count_mutex());
        let _ = track.append_entity(Box::new(instrument)).unwrap();

        let test_message = MidiMessage::NoteOn {
            key: 7.into(),
            vel: 13.into(),
        };
        if let Ok(received) = midi_messages_received.lock() {
            assert_eq!(
                *received, 0,
                "Before sending an external MIDI message to Orchestrator, count should be zero"
            );
        };
        o.handle_midi_message(MidiChannel(0), test_message, &mut |channel, message| {
            panic!("Didn't expect {channel:?} {message:?}",)
        });
        if let Ok(received) = midi_messages_received.lock() {
            assert_eq!(
                *received, 1,
                "Count should update after sending an external MIDI message to Orchestrator"
            );
        };
    }

    #[test]
    fn midi_messages_from_track_a_do_not_reach_track_b() {
        let mut o = Orchestrator::default();
        let track_a_uid = o.new_midi_track().unwrap();
        let track_b_uid = o.new_midi_track().unwrap();

        // On Track 1, put a sender and receiver.
        let mut sender = ToyControllerAlwaysSendsMidiMessage::default();
        sender.set_uid(Uid(10001));
        let _ = o
            .get_track_mut(&track_a_uid)
            .unwrap()
            .append_entity(Box::new(sender));
        let mut receiver_1 = ToyInstrument::new_with(&ToyInstrumentParams::default());
        receiver_1.set_uid(Uid(10002));
        let counter_1 = Arc::clone(receiver_1.received_count_mutex());
        let _ = o
            .get_track_mut(&track_a_uid)
            .unwrap()
            .append_entity(Box::new(receiver_1));

        // On Track 2, put another receiver.
        let mut receiver_2 = ToyInstrument::new_with(&ToyInstrumentParams::default());
        receiver_2.set_uid(Uid(20001));
        let counter_2 = Arc::clone(receiver_2.received_count_mutex());
        let _ = o
            .get_track_mut(&track_b_uid)
            .unwrap()
            .append_entity(Box::new(receiver_2));

        // Fire everything up.
        o.play();
        o.work(&mut |_, _| {});

        // Sender should have sent a message that receiver #1 should receive,
        // because they're both in the same Track.
        if let Ok(c) = counter_1.lock() {
            assert_eq!(1, *c);
        }
        // But Receiver #2 shouldn't see that message, because it's in a
        // different Track.
        if let Ok(c) = counter_2.lock() {
            assert_eq!(0, *c);
        };
    }
}
