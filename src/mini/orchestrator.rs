// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{
    control_router::ControlRouter,
    entity_factory::EntityFactory,
    track::{Track, TrackAction, TrackFactory, TrackTitle, TrackUid},
    transport::Transport,
    Key,
};
use crate::egui_widgets::ArrangementView;
use anyhow::{anyhow, Result};
use eframe::egui::{self, Ui};
use groove_audio::AudioQueue;
use groove_core::{
    control::ControlValue,
    midi::{MidiChannel, MidiMessage, MidiMessagesFn},
    time::{MusicalTime, SampleRate, Tempo},
    traits::{
        Configurable, ControlEventsFn, Controllable, Controls, Generates,
        GeneratesToInternalBuffer, HandlesMidi, HasUid, Performs, Serializable, Thing, ThingEvent,
        Ticks,
    },
    Sample, StereoSample, Uid,
};
use rayon::prelude::{IntoParallelRefMutIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    ops::Range,
    sync::Arc,
};

/// A grouping mechanism to declare parts of [MiniOrchestrator] that Serde
/// shouldn't be serializing. Exists so we don't have to spray #[serde(skip)]
/// all over the place.
#[derive(Debug, Default)]
pub struct OrchestratorEphemerals {
    range: Range<MusicalTime>,
    events: Vec<(Uid, ThingEvent)>,
    is_finished: bool,
    is_performing: bool,
}

/// Owns all entities (instruments, controllers, and effects), and manages the
/// relationships among them to create an audio performance.
#[derive(Serialize, Deserialize, Debug)]
pub struct MiniOrchestrator {
    /// The user-supplied name of this project.
    title: Option<String>,
    transport: Transport,
    control_router: ControlRouter,

    track_factory: TrackFactory,
    tracks: HashMap<TrackUid, Track>,
    ordered_track_uids: Vec<TrackUid>,
    selected_track_uids: HashSet<TrackUid>,

    //////////////////////////////////////////////////////
    // Nothing below this comment should be serialized. //
    //////////////////////////////////////////////////////
    //
    #[serde(skip)]
    e: OrchestratorEphemerals,
    #[serde(skip)]
    entity_factory: Option<Arc<EntityFactory>>,
    #[serde(skip)]
    arrangement_view: ArrangementView,
}
impl Default for MiniOrchestrator {
    fn default() -> Self {
        let mut track_factory = TrackFactory::default();
        let track_vec = vec![
            track_factory.midi(),
            track_factory.midi(),
            track_factory.send(),
        ];
        let mut ordered_track_uids = Vec::default();
        let tracks = track_vec.into_iter().fold(HashMap::default(), |mut v, t| {
            ordered_track_uids.push(t.uid());
            v.insert(t.uid(), t);
            v
        });
        Self {
            title: None,
            transport: Default::default(),
            control_router: Default::default(),

            track_factory,
            tracks,
            ordered_track_uids,
            selected_track_uids: Default::default(),

            arrangement_view: Default::default(),

            e: Default::default(),
            entity_factory: None,
        }
    }
}
impl MiniOrchestrator {
    /// The expected size of any buffer provided for samples.
    //
    // TODO: how hard would it be to make this dynamic? Does adjustability
    // matter?
    pub const SAMPLE_BUFFER_SIZE: usize = 64;

    /// Creates a new [MiniOrchestrator] with an [EntityFactory]. Note that an
    /// [Arc] wraps the factory, which implies that the factory must be fully
    /// equipped by the time it's given to the orchestrator.
    pub fn new_with(entity_factory: Arc<EntityFactory>) -> Self {
        let transport_uid = entity_factory.mint_uid();
        let mut r = Self {
            entity_factory: Some(entity_factory),
            ..Default::default()
        };
        r.transport.set_uid(transport_uid);
        r
    }

    /// The current [SampleRate] used to render the current project. Typically
    /// something like 44.1KHz.
    pub fn sample_rate(&self) -> SampleRate {
        self.transport.sample_rate()
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
    pub fn enqueue_next_samples(&mut self, queue: &AudioQueue, samples_requested: usize) {
        // Round up
        let buffers_requested =
            (samples_requested + Self::SAMPLE_BUFFER_SIZE - 1) / Self::SAMPLE_BUFFER_SIZE;
        for _ in 0..buffers_requested {
            // Generate a buffer only if there's enough room in the queue for it.
            if queue.capacity() - queue.len() >= Self::SAMPLE_BUFFER_SIZE {
                let mut samples = [StereoSample::SILENCE; Self::SAMPLE_BUFFER_SIZE];
                if false {
                    self.generate_next_debug_samples(&mut samples);
                } else {
                    self.generate_next_samples(&mut samples);
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
    pub fn generate_next_debug_samples(&mut self, samples: &mut [StereoSample]) {
        let len = samples.len() as f64;
        for (i, s) in samples.iter_mut().enumerate() {
            s.0 = Sample::from(i as f64 / len);
            s.1 = Sample::from(i as f64 / -len);
        }
    }

    /// Renders the next set of samples into the provided buffer. This is the
    /// main event loop.
    pub fn generate_next_samples(&mut self, samples: &mut [StereoSample]) {
        // Note that advance() can return the same range twice, depending on
        // sample rate. TODO: we should decide whose responsibility it is to
        // handle that -- either we skip calling work() if the time range is the
        // same as prior, or everyone who gets called needs to detect the case
        // or be idempotent.
        let range = self.transport.advance(samples.len());
        self.update_time(&range);
        self.work(&mut |_, _| panic!("work() was supposed to handle all events"));
        self.generate_batch_values(samples);
    }

    /// After loading a new Self from disk, we want to copy all the appropriate
    /// ephemeral state from this one to the next one.
    pub fn prepare_successor(&self, new: &mut MiniOrchestrator) {
        // Copy over the current sample rate, whose validity shouldn't change
        // because we loaded a new project.
        new.update_sample_rate(self.sample_rate());

        // [EntityFactory] needs its internal new-uid counter to be higher than
        // any existing [Uid] in the project, so that it doesn't mint duplicate
        // [Uid]s. This is a bit cumbersome.
        if let Some(factory) = &self.entity_factory {
            factory.set_next_uid_expensively(&new.max_uid());
            new.entity_factory = Some(Arc::clone(factory));
        }
    }

    /// Returns the maximum known Uid in use among all tracks.
    fn max_uid(&self) -> Uid {
        if let Some(max_uid) = self.tracks.values().map(|t| t.max_uid()).max() {
            max_uid
        } else {
            Uid(0)
        }
    }

    // fn show_tracks(&mut self, ui: &mut Ui) -> Option<TrackAction> {
    //     let mut action = None;

    //     // Non-send tracks are first, then send tracks
    //     let uids = &self.ordered_track_uids.clone();
    //     let uids: Vec<&TrackUid> = uids
    //         .iter()
    //         .filter(|uid| !self.tracks.get(uid).unwrap().is_send())
    //         .chain(
    //             self.ordered_track_uids
    //                 .iter()
    //                 .filter(|uid| self.tracks.get(uid).unwrap().is_send()),
    //         )
    //         .collect();

    //     for uid in uids {
    //         if let Some(track) = self.tracks.get_mut(uid) {
    //             let (response, a) = track.show(ui, self.selected_track_uids.contains(&uid));
    //             if a.is_some() {
    //                 action = a;
    //             }
    //             if response.clicked() {
    //                 action = Some(TrackAction::Select(*uid));
    //             }
    //         }
    //     }
    //     action
    // }

    /// Renders the project's GUI.
    pub fn show_with(&mut self, ui: &mut Ui, is_control_only_down: bool) {
        self.arrangement_view.set_viewable_time_range(
            MusicalTime::new_with_beats(0)..MusicalTime::new_with_beats(128),
        );

        // TODO: this displays tracks in random order. see commented-out
        // show_tracks() above

        let tracks = self
            .tracks
            .values()
            .filter(|t| !t.is_send())
            .chain(self.tracks.values().filter(|t| t.is_send()));

        if let Some(action) = self
            .arrangement_view
            .show(ui, tracks, &|uid| self.is_track_selected(&uid))
        {
            self.handle_track_action(action, is_control_only_down);
        }

        if let Some(track_uid) = self.get_single_selected_uid() {
            let mut action = None;
            if let Some(track) = self.tracks.get_mut(&track_uid) {
                let bottom =
                    egui::TopBottomPanel::bottom("orchestrator-bottom-panel").resizable(true);
                bottom.show_inside(ui, |_ui| {});
                action = track.show_detail(ui);
            }
            if let Some(action) = action {
                self.handle_track_action(action, is_control_only_down);
            }
        }
    }

    fn handle_track_action(&mut self, action: TrackAction, is_control_only_down: bool) {
        match action {
            TrackAction::Select(uid) => {
                self.select_track(&uid, is_control_only_down);
            }
            TrackAction::SelectClear => {
                self.clear_track_selections();
            }
            TrackAction::SetTitle(index, title) => self.set_track_title(index, title),
        }
    }

    fn new_track(&mut self, track: Track) {
        let uid = track.uid();
        self.ordered_track_uids.push(uid);
        self.tracks.insert(uid, track);
    }

    #[allow(missing_docs)]
    pub fn new_midi_track(&mut self) {
        let track = self.track_factory.midi();
        self.new_track(track)
    }

    #[allow(missing_docs)]
    pub fn new_audio_track(&mut self) {
        let track = self.track_factory.audio();
        self.new_track(track)
    }

    #[allow(missing_docs)]
    pub fn new_send_track(&mut self) {
        let track = self.track_factory.send();
        self.new_track(track)
    }

    #[allow(missing_docs)]
    #[allow(dead_code)]
    pub fn delete_track(&mut self, uid: &TrackUid) {
        self.tracks.remove(uid);
        self.ordered_track_uids.retain(|u| u != uid);
    }

    #[allow(missing_docs)]
    pub fn delete_selected_tracks(&mut self) {
        self.selected_track_uids
            .clone()
            .iter()
            .for_each(|uid| self.delete_track(uid));
        self.selected_track_uids.clear();
    }

    /// Adds the given track to the selection set, or else replaces the set with
    /// this single item.
    pub fn select_track(&mut self, uid: &TrackUid, add_to_selections: bool) {
        self.selected_track_uids.insert(*uid);
        let existing = self.is_track_selected(uid);
        if !add_to_selections {
            self.selected_track_uids.clear();
        }
        if existing {
            self.selected_track_uids.insert(*uid);
        } else {
            self.selected_track_uids.remove(&uid);
        }
    }

    #[allow(missing_docs)]
    // TODO: this doesn't make sense. Should restrict to operating on a single
    // track.
    pub fn remove_selected_patterns(&mut self) {
        let selected_uids = self.selected_track_uids.clone();
        selected_uids.iter().for_each(|uid| {
            if let Some(track) = self.get_track_mut(uid) {
                track.remove_selected_patterns();
            }
        });
    }

    /// Returns the name of the project.
    pub fn title(&self) -> Option<&String> {
        self.title.as_ref()
    }

    #[allow(missing_docs)]
    #[allow(dead_code)]
    pub fn set_title(&mut self, title: Option<String>) {
        self.title = title;
    }

    fn clear_track_selections(&mut self) {
        self.selected_track_uids.clear()
    }

    /// Adds a new thing with the specified [Key] to the currently selected
    /// single track. Fails if anything but exactly one track is selected.
    pub fn add_thing_by_key_to_selected_track(&mut self, key: &Key) -> Result<Uid> {
        if let Some(track_uid) = self.get_single_selected_uid() {
            self.add_thing_by_key_to_track(key, &track_uid)
        } else {
            Err(anyhow!("A single track was not selected"))
        }
    }

    /// If exactly one track is selected, returns its [TrackUid]. Otherwise
    /// returns `None`.
    pub fn get_single_selected_uid(&self) -> Option<TrackUid> {
        // TODO: this is icky. Is there a better way to get a single value out of a HashSet?
        if self.is_one_track_selected() {
            if let Some(uid) = self.selected_track_uids.iter().next() {
                return Some(uid.clone());
            }
        }
        None
    }

    #[allow(missing_docs)]
    pub fn is_any_track_selected(&self) -> bool {
        !self.selected_track_uids.is_empty()
    }

    #[allow(missing_docs)]
    pub fn is_one_track_selected(&self) -> bool {
        self.selected_track_uids.len() == 1
    }

    /// Adds a new thing with the specified [Key] to the track with the
    /// specified [TrackIndex].
    pub fn add_thing_by_key_to_track(&mut self, key: &Key, track_uid: &TrackUid) -> Result<Uid> {
        if let Some(factory) = &self.entity_factory {
            if let Some(e) = factory.new_thing(key) {
                self.add_thing(e, track_uid)
            } else {
                Err(anyhow!("key {key} not found"))
            }
        } else {
            Err(anyhow!("there is no entity factory"))
        }
    }

    /// Adds the given thing, returning an assigned [Uid] if successful.
    /// [MiniOrchestrator] takes ownership.
    pub fn add_thing(&mut self, mut thing: Box<dyn Thing>, track_uid: &TrackUid) -> Result<Uid> {
        thing.update_sample_rate(self.sample_rate());
        let uid = thing.uid();
        if let Some(track) = self.tracks.get_mut(track_uid) {
            track.append_thing(thing);
            Ok(uid)
        } else {
            Err(anyhow!("Track UID {track_uid} not found"))
        }
    }

    fn calculate_is_finished(&self) -> bool {
        self.tracks.values().all(|t| t.is_finished())
    }

    fn dispatch_event(&mut self, uid: Uid, event: ThingEvent) {
        match event {
            ThingEvent::Midi(channel, message) => {
                self.route_midi_message(channel, message);
            }
            ThingEvent::Control(value) => {
                self.route_control_change(uid, value);
            }
            _ => {
                panic!(
                    "New system doesn't use event {:?}. Consider deleting it!",
                    event
                )
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
                if target_uid == &self.transport.uid() {
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

    #[allow(missing_docs)]
    pub fn transport(&self) -> &Transport {
        &self.transport
    }

    #[allow(missing_docs)]
    pub fn transport_mut(&mut self) -> &mut Transport {
        &mut self.transport
    }

    fn set_track_title(&mut self, uid: TrackUid, title: TrackTitle) {
        if let Some(track) = self.get_track_mut(&uid) {
            track.set_title(title);
        }
    }

    fn is_track_selected(&self, uid: &TrackUid) -> bool {
        self.selected_track_uids.contains(uid)
    }

    #[allow(dead_code)]
    fn get_track(&self, uid: &TrackUid) -> Option<&Track> {
        self.tracks.get(uid)
    }

    fn get_track_mut(&mut self, uid: &TrackUid) -> Option<&mut Track> {
        self.tracks.get_mut(uid)
    }

    #[allow(missing_docs)]
    pub fn ordered_track_uids(&self) -> &[TrackUid] {
        self.ordered_track_uids.as_ref()
    }
}
impl Generates<StereoSample> for MiniOrchestrator {
    fn value(&self) -> StereoSample {
        StereoSample::SILENCE
    }

    // Note! It's the caller's job to prepare the buffer. This method will *add*
    // its results, rather than overwriting.
    fn generate_batch_values(&mut self, values: &mut [StereoSample]) {
        let len = values.len();
        self.tracks.par_iter_mut().for_each(|(_uid, track)| {
            track.generate_batch_values(len);
        });

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
impl Ticks for MiniOrchestrator {
    fn tick(&mut self, _tick_count: usize) {
        panic!()
    }
}
impl Configurable for MiniOrchestrator {
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
impl HandlesMidi for MiniOrchestrator {
    /// Accepts a [MidiMessage] and handles it, usually by forwarding it to
    /// controllers and instruments on the given [MidiChannel]. We implement
    /// this trait only for external messages; for ones generated internally, we
    /// use [MidiRouter].
    fn handle_midi_message(
        &mut self,
        channel: MidiChannel,
        message: MidiMessage,
        _: &mut MidiMessagesFn,
    ) {
        self.route_midi_message(channel, message);
    }
}
impl Performs for MiniOrchestrator {
    fn play(&mut self) {
        self.e.is_performing = true;
        self.transport.play();
        self.tracks.values_mut().for_each(|t| t.play());
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
impl Controls for MiniOrchestrator {
    fn update_time(&mut self, range: &Range<MusicalTime>) {
        self.e.range = range.clone();

        for track in self.tracks.values_mut() {
            track.update_time(&self.e.range);
        }
    }

    fn work(&mut self, _: &mut ControlEventsFn) {
        self.transport.work(&mut |u, m| self.e.events.push((u, m)));
        for track in self.tracks.values_mut() {
            track.work(&mut |u, m| self.e.events.push((u, m)));
        }
        while let Some((uid, event)) = self.e.events.pop() {
            self.dispatch_event(uid, event);
        }
        self.e.is_finished = self.calculate_is_finished();
        if self.is_performing() && self.is_finished() {
            self.stop();
        }
    }

    fn is_finished(&self) -> bool {
        self.e.is_finished
    }
}
impl Serializable for MiniOrchestrator {
    fn after_deser(&mut self) {
        self.tracks.values_mut().for_each(|t| t.after_deser());
    }
}

#[cfg(test)]
mod tests {
    use crate::mini::orchestrator::MiniOrchestrator;
    use groove_core::{
        time::{MusicalTime, SampleRate, Tempo},
        traits::{Configurable, Controls, Performs},
        StereoSample,
    };
    use groove_entities::controllers::{Timer, TimerParams};

    #[test]
    fn basic_operations() {
        let mut o = MiniOrchestrator::default();

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
        let mut o = MiniOrchestrator::default();

        // TODO: worst ergonomics ever.
        const TIMER_DURATION: MusicalTime = MusicalTime::new_with_beats(1);
        let track_uid = o.ordered_track_uids()[0];
        let _ = o.add_thing(
            Box::new(Timer::new_with(&TimerParams {
                duration: groove_core::time::MusicalTimeParams {
                    units: TIMER_DURATION.total_units(),
                },
            })),
            &track_uid,
        );

        o.play();
        let mut prior_start_time = MusicalTime::TIME_ZERO;
        loop {
            if o.is_finished() {
                break;
            }
            prior_start_time = o.transport().current_time();
            let mut samples = [StereoSample::SILENCE; 1];
            o.generate_next_samples(&mut samples);
        }
        let prior_range = prior_start_time..o.transport().current_time();
        assert!(prior_range.contains(&TIMER_DURATION));
    }
}
