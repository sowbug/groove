// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{
    entities::{NewIsController, NewIsEffect, NewIsInstrument},
    entity_factory::EntityFactory,
    track::{Track, TrackAction, TrackFactory, TrackIndex},
    Key,
};
use anyhow::{anyhow, Result};
use eframe::egui::{self, Ui};
use groove_audio::AudioQueue;
use groove_core::{
    midi::{MidiChannel, MidiMessage},
    time::{MusicalTime, SampleRate, Tempo, TimeSignature},
    traits::{
        gui::Shows, Configurable, Controls, Generates, GeneratesToInternalBuffer, HandlesMidi,
        Ticks,
    },
    Sample, StereoSample, Uid,
};
use groove_entities::EntityMessage;
use rayon::prelude::{IntoParallelRefMutIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Debug, ops::Range, sync::Arc};

/// Owns all entities (instruments, controllers, and effects), and manages the
/// relationships among them to create an audio performance.
#[derive(Serialize, Deserialize, Debug)]
pub struct MiniOrchestrator {
    /// The user-supplied name of this project.
    title: Option<String>,
    /// The current global time signature.
    time_signature: TimeSignature,
    /// The current beats per minute.
    tempo: Tempo,

    track_factory: TrackFactory,
    tracks: Vec<Track>,
    // If one track is selected, then this is set.
    single_track_selection: Option<TrackIndex>,

    /// MIDI connections
    midi_channel_to_receiver_uid: HashMap<MidiChannel, Vec<Uid>>,

    //////////////////////////////////////////////////////
    // Nothing below this comment should be serialized. //
    //////////////////////////////////////////////////////
    //
    #[serde(skip)]
    sample_rate: SampleRate,

    #[serde(skip)]
    #[allow(dead_code)]
    frames: usize,

    #[serde(skip)]
    #[allow(dead_code)]
    current_time: MusicalTime,

    #[serde(skip)]
    entity_factory: Option<Arc<EntityFactory>>,

    #[serde(skip)]
    messages: Vec<EntityMessage>,
}
impl Default for MiniOrchestrator {
    fn default() -> Self {
        let mut track_factory = TrackFactory::default();
        Self {
            title: None,
            time_signature: Default::default(),
            tempo: Default::default(),

            tracks: vec![
                track_factory.midi(),
                track_factory.midi(),
                track_factory.send(),
            ],
            track_factory,
            single_track_selection: None,

            midi_channel_to_receiver_uid: Default::default(),

            sample_rate: Default::default(),
            frames: Default::default(),
            current_time: Default::default(),
            entity_factory: None,
            messages: Default::default(),
        }
    }
}
impl MiniOrchestrator {
    /// The expected size of any buffer provided for samples.
    //
    // TODO: how hard would it be to make this dynamic? Does adjustability
    // matter?
    pub const SAMPLE_BUFFER_SIZE: usize = 64;

    /// Creates a new [MiniOrchestrator] with a (hopefully) initialized
    /// [EntityFactory].
    pub fn new_with(entity_factory: Arc<EntityFactory>) -> Self {
        Self {
            entity_factory: Some(entity_factory),
            ..Default::default()
        }
    }

    /// The current [SampleRate] used to render the current project. Typically
    /// something like 44.1KHz.
    pub fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    /// Sets a new global [SampleRate] for the project.
    pub fn set_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        for track in self.tracks.iter_mut() {
            track.update_sample_rate(sample_rate);
        }
    }

    /// Returns the current [Tempo].
    pub fn tempo(&self) -> Tempo {
        self.tempo
    }

    #[allow(dead_code)]
    fn set_tempo(&mut self, tempo: Tempo) {
        self.tempo = tempo;
    }

    /// Returns the number of channels in the audio stream. For now, this is
    /// always 2 (stereo audio stream).
    pub fn channels(&self) -> u16 {
        2
    }

    /// Fills in the given sample buffer with something simple and audible.
    pub fn debug_sample_buffer(&mut self, samples: &mut [StereoSample]) {
        let len = samples.len() as f64;
        for (i, s) in samples.iter_mut().enumerate() {
            s.0 = Sample::from(i as f64 / len);
            s.1 = Sample::from(i as f64 / -len);
        }
    }

    /// Whether we're currently playing a performance.
    pub fn is_performing(&self) -> bool {
        self.current_time.total_beats() < 16
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
                self.generate_next_samples(&mut samples);
                for sample in samples {
                    let _ = queue.push(sample);
                }
            }
        }
    }

    /// Renders the next set of samples into the provided buffer.
    pub fn generate_next_samples(&mut self, samples: &mut [StereoSample]) {
        let start = self.current_time;
        let length = MusicalTime::new_with_units(MusicalTime::frames_to_units(
            self.tempo,
            self.sample_rate,
            samples.len(),
        ));
        let range = start..start + length;
        self.let_controllers_work(range);
        self.let_audio_devices_work(samples);
        self.current_time += length;
    }

    fn let_controllers_work(&mut self, range: Range<MusicalTime>) {
        for track in self.tracks.iter_mut() {
            track.update_time(&range);
        }
        for track in self.tracks.iter_mut() {
            track.work(&mut |m| self.messages.push(m));
        }
    }

    fn let_audio_devices_work(&mut self, samples: &mut [StereoSample]) {
        self.generate_batch_values(samples);
    }

    /// After loading a new Self from disk, we want to copy all the appropriate
    /// ephemeral state from this one to the next one.
    pub fn prepare_successor(&self, new: &mut MiniOrchestrator) {
        new.set_sample_rate(self.sample_rate());

        // TODO: refresh EntityFactory's relaxed counter(s) to account for
        // existing items
    }

    #[allow(dead_code)]
    fn move_controller(
        &mut self,
        old_track_index: usize,
        old_item_index: usize,
        new_track_index: usize,
        new_item_index: usize,
    ) -> Result<()> {
        if let Some(e) = self.tracks[old_track_index].remove_controller(old_item_index) {
            self.tracks[new_track_index].insert_controller(new_item_index, e)
        } else {
            Err(anyhow!("controller not found"))
        }
    }

    #[allow(dead_code)]
    fn move_effect(
        &mut self,
        old_track_index: usize,
        old_item_index: usize,
        new_track_index: usize,
        new_item_index: usize,
    ) -> Result<()> {
        if let Some(e) = self.tracks[old_track_index].remove_effect(old_item_index) {
            self.tracks[new_track_index].insert_effect(new_item_index, e)
        } else {
            Err(anyhow!("effect not found"))
        }
    }

    #[allow(dead_code)]
    fn move_instrument(
        &mut self,
        old_track_index: usize,
        old_item_index: usize,
        new_track_index: usize,
        new_item_index: usize,
    ) -> Result<()> {
        if let Some(e) = self.tracks[old_track_index].remove_instrument(old_item_index) {
            self.tracks[new_track_index].insert_instrument(new_item_index, e)
        } else {
            Err(anyhow!("instrument not found"))
        }
    }

    fn show_tracks(&mut self, ui: &mut Ui, is_control_only_down: bool) -> Option<TrackAction> {
        let mut action = None;

        // Non-send tracks are first
        for (index, track) in self.tracks.iter_mut().enumerate() {
            if !track.is_send() {
                if track.show(ui).clicked() {
                    action = Some(TrackAction::Select(TrackIndex(index), is_control_only_down));
                }
            }
        }

        // Send tracks are last
        for (index, track) in self.tracks.iter_mut().enumerate() {
            if track.is_send() {
                if track.show(ui).clicked() {
                    action = Some(TrackAction::Select(TrackIndex(index), is_control_only_down));
                }
            }
        }
        action
    }

    /// Renders the project's GUI.
    pub fn show_with(&mut self, ui: &mut egui::Ui, is_control_only_down: bool) {
        if let Some(action) = self.show_tracks(ui, is_control_only_down) {
            self.handle_track_action(action);
        }
        if let Some(selected) = self.single_track_selection {
            let bottom = egui::TopBottomPanel::bottom("orchestrator-bottom-panel").resizable(true);
            bottom.show_inside(ui, |ui| {
                if let Some(action) = self.tracks[selected.0].show_detail(ui) {
                    self.handle_track_action(action);
                }
            });
        }
    }

    fn handle_track_action(&mut self, action: TrackAction) {
        match action {
            // TrackAction::NewController(track, key) => {
            //     let _ = self.add_controller_by_key(&key, track);
            // }
            // TrackAction::NewEffect(track, key) => {
            //     let _ = self.add_effect_by_key(&key, track);
            // }
            // TrackAction::NewInstrument(track, key) => {
            //     let _ = self.add_instrument_by_key(&key, track);
            // }
            TrackAction::Select(index, add_to_selections) => {
                self.select_track(index, add_to_selections);
            }
            TrackAction::SelectClear => {
                self.clear_track_selections();
            }
        }
    }

    #[allow(missing_docs)]
    pub fn new_midi_track(&mut self) {
        self.tracks.push(self.track_factory.midi());
    }

    #[allow(missing_docs)]
    pub fn new_audio_track(&mut self) {
        self.tracks.push(self.track_factory.audio());
    }

    #[allow(missing_docs)]
    pub fn new_send_track(&mut self) {
        self.tracks.push(self.track_factory.send());
    }

    #[allow(missing_docs)]
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    #[allow(missing_docs)]
    #[allow(dead_code)]
    pub fn delete_track(&mut self, index: usize) {
        self.tracks.remove(index);
    }

    #[allow(missing_docs)]
    pub fn delete_selected_tracks(&mut self) {
        self.tracks.retain(|t| !t.selected());
    }

    /// Adds the given track to the selection set, or else replaces the set with
    /// this single item.
    pub fn select_track(&mut self, track: TrackIndex, add_to_selections: bool) {
        let existing = self.tracks[track.0].selected();
        if !add_to_selections {
            self.clear_track_selections();
        }
        self.tracks[track.0].set_selected(!existing);
    }

    #[allow(missing_docs)]
    pub fn remove_selected_patterns(&mut self) {
        self.tracks.iter_mut().for_each(|t| {
            if t.selected() {
                t.remove_selected_patterns();
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

    /// Does housekeeping whenever the track selection changes.
    //
    // It's important for this to run at either the start or the end of the
    // update block. It tells the UI whether exactly one track is selected.
    //
    // TODO: this should actually be tied to selection changes. I originally
    // tied it to GUI updates when I was trying to figure out the design. I
    // think I was concerned about calculating it too often. But that was never
    // going to be an issue if it were driven by the GUI, because there are no
    // batch changes there.
    pub fn update_track_selection_tracking(&mut self) {
        let count = self.tracks.iter().filter(|t| t.selected()).count();
        self.single_track_selection = if count == 1 {
            Some(TrackIndex(
                self.tracks.iter().position(|t| t.selected()).unwrap(),
            ))
        } else {
            None
        };
    }

    /// If a single track is selected, returns its [TrackIndex]. Otherwise returns `None`.
    pub fn single_track_selection(&self) -> Option<TrackIndex> {
        self.single_track_selection
    }

    fn clear_track_selections(&mut self) {
        self.tracks.iter_mut().for_each(|t| {
            t.set_selected(false);
        });
    }

    #[allow(missing_docs)]
    pub fn is_any_track_selected(&self) -> bool {
        self.tracks.iter().any(|t| t.selected())
    }

    /// Adds a new controller with the specified [Key] to the currently selected
    /// single track. Fails if anything but exactly one track is selected.
    pub fn add_controller_by_key_to_selected_track(&mut self, key: &Key) -> Result<Uid> {
        if let Some(track) = self.single_track_selection() {
            self.add_controller_by_key_to_track(key, track)
        } else {
            Err(anyhow!("A single track was not selected"))
        }
    }

    /// Adds a new controller with the specified [Key] to the track with the specified [TrackIndex].
    pub fn add_controller_by_key_to_track(&mut self, key: &Key, track: TrackIndex) -> Result<Uid> {
        if let Some(factory) = &self.entity_factory {
            if let Some(e) = factory.new_controller(key) {
                self.add_controller(e, track)
            } else {
                Err(anyhow!("controller key {key} not found"))
            }
        } else {
            Err(anyhow!("there is no entity factory"))
        }
    }

    fn add_controller(
        &mut self,
        mut e: Box<dyn NewIsController>,
        track: TrackIndex,
    ) -> Result<Uid> {
        e.update_sample_rate(self.sample_rate);
        let uid = e.uid();
        self.tracks[track.0].append_controller(e);
        Ok(uid)
    }

    /// Adds a new effect with the specified [Key] to the currently selected
    /// single track. Fails if anything but exactly one track is selected.
    pub fn add_effect_by_key_to_selected_track(&mut self, key: &Key) -> Result<Uid> {
        if let Some(track) = self.single_track_selection() {
            self.add_effect_by_key_to_track(key, track)
        } else {
            Err(anyhow!("A single track was not selected"))
        }
    }

    /// Adds a new effect with the specified [Key] to the track with the specified [TrackIndex].
    pub fn add_effect_by_key_to_track(&mut self, key: &Key, track: TrackIndex) -> Result<Uid> {
        if let Some(factory) = &self.entity_factory {
            if let Some(e) = factory.new_effect(key) {
                self.add_effect(e, track)
            } else {
                Err(anyhow!("effect key {key} not found"))
            }
        } else {
            Err(anyhow!("there is no entity factory"))
        }
    }

    fn add_effect(&mut self, mut e: Box<dyn NewIsEffect>, track: TrackIndex) -> Result<Uid> {
        e.update_sample_rate(self.sample_rate);
        let uid = e.uid();
        self.tracks[track.0].append_effect(e);
        Ok(uid)
    }

    /// Adds a new instrument with the specified [Key] to the currently selected
    /// single track. Fails if anything but exactly one track is selected.
    pub fn add_instrument_by_key_to_selected_track(&mut self, key: &Key) -> Result<Uid> {
        if let Some(track) = self.single_track_selection() {
            self.add_instrument_by_key_to_track(key, track)
        } else {
            Err(anyhow!("A single track was not selected"))
        }
    }

    /// Adds a new instrument with the specified [Key] to the track with the specified [TrackIndex].
    pub fn add_instrument_by_key_to_track(&mut self, key: &Key, track: TrackIndex) -> Result<Uid> {
        if let Some(factory) = &self.entity_factory {
            if let Some(e) = factory.new_instrument(key) {
                self.add_instrument(e, track)
            } else {
                Err(anyhow!("instrument key {key} not found"))
            }
        } else {
            Err(anyhow!("there is no entity factory"))
        }
    }

    fn add_instrument(
        &mut self,
        mut e: Box<dyn NewIsInstrument>,
        track: TrackIndex,
    ) -> Result<Uid> {
        e.update_sample_rate(self.sample_rate);
        let uid = e.uid();
        self.tracks[track.0].append_instrument(e);
        Ok(uid)
    }

    /// The entities receiving on the given MIDI channel.
    pub fn midi_receivers(&mut self, channel: &MidiChannel) -> &Vec<Uid> {
        self.midi_channel_to_receiver_uid
            .entry(*channel)
            .or_default()
    }

    /// Connect an entity to the given MIDI channel.
    pub fn connect_midi_receiver(&mut self, receiver_uid: Uid, channel: MidiChannel) {
        self.midi_channel_to_receiver_uid
            .entry(channel)
            .or_default()
            .push(receiver_uid);
    }

    /// Disconnect an entity from the given MIDI channel.
    pub fn disconnect_midi_receiver(&mut self, receiver_uid: Uid, channel: MidiChannel) {
        self.midi_channel_to_receiver_uid
            .entry(channel)
            .or_default()
            .retain(|&uid| uid != receiver_uid);
    }

    /// Returns the global music clock.
    pub fn current_time(&self) -> MusicalTime {
        self.current_time
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
        self.tracks.par_iter_mut().for_each(|track| {
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
        self.tracks.iter().for_each(|track| {
            let generator_values = track.values();
            generator_values.iter().enumerate().for_each(|(i, v)| {
                values[i] += *v;
            });
        });
    }
}
impl Ticks for MiniOrchestrator {
    fn tick(&mut self, _tick_count: usize) {
        panic!()
    }
}
impl Configurable for MiniOrchestrator {}
impl Shows for MiniOrchestrator {
    fn show(&mut self, ui: &mut egui::Ui) {
        ui.label("not used");
    }
}
impl HandlesMidi for MiniOrchestrator {
    /// Accepts a [MidiMessage] and handles it, usually by forwarding it to
    /// controllers and instruments on the given [MidiChannel].
    fn handle_midi_message(
        &mut self,
        channel: MidiChannel,
        message: &MidiMessage,
        messages_fn: &mut dyn FnMut(MidiChannel, MidiMessage),
    ) {
        for track in self.tracks.iter_mut() {
            track.handle_midi_message(channel, &message, &mut |channel, message| {
                // TODO: this isn't enough -- we need to dispatch these messages
                // to other devices on the channel/bus, detect loops, etc. I'm
                // not even sure it's right to bubble these back to the caller.
                messages_fn(channel, message);
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::mini::{orchestrator::MiniOrchestrator, TrackIndex};
    use groove_core::midi::MidiChannel;
    use groove_entities::controllers::{ToyController, ToyControllerParams};
    use groove_toys::{ToyEffect, ToyEffectParams, ToyInstrument, ToyInstrumentParams};

    #[test]
    fn mini_orchestrator_basic_operations() {
        let mut o = MiniOrchestrator::default();

        // A new orchestrator should have at least one track.
        assert!(!o.tracks.is_empty());

        let id1 = o
            .add_instrument(
                Box::new(ToyInstrument::new_with(&ToyInstrumentParams::default())),
                TrackIndex(0),
            )
            .unwrap();
        let id2 = o
            .add_instrument(
                Box::new(ToyInstrument::new_with(&ToyInstrumentParams::default())),
                TrackIndex(0),
            )
            .unwrap();
        assert_eq!(o.tracks()[0].instruments()[0].uid(), id1);
        assert_eq!(o.tracks()[0].instruments()[1].uid(), id2);

        let id1 = o
            .add_controller(
                Box::new(ToyController::new_with(
                    &ToyControllerParams::default(),
                    MidiChannel(0),
                )),
                TrackIndex(0),
            )
            .unwrap();
        assert_eq!(o.tracks()[0].controllers()[0].uid(), id1);

        let id1 = o
            .add_effect(
                Box::new(ToyEffect::new_with(&ToyEffectParams::default())),
                TrackIndex(0),
            )
            .unwrap();
        assert_eq!(o.tracks()[0].effects()[0].uid(), id1);

        assert!(o.tracks.len() > 1);
        let id3 = o
            .add_instrument(
                Box::new(ToyInstrument::new_with(&ToyInstrumentParams::default())),
                TrackIndex(1),
            )
            .unwrap();

        // Moving something to another track works.
        assert_eq!(o.tracks()[0].instruments().len(), 2);
        assert_eq!(o.tracks()[1].instruments().len(), 1);
        assert!(o.move_instrument(1, 0, 0, 0).is_ok());
        assert_eq!(o.tracks[0].instruments().len(), 3);
        assert_eq!(o.tracks[1].instruments().len(), 0);
        assert_eq!(o.tracks[0].instruments()[0].uid(), id3);
    }
}
