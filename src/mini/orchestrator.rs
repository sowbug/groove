// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::mini::MiniSequencer;

use super::{
    entity_factory::{EntityFactory, NewIsController, NewIsEffect, NewIsInstrument},
    track::{Track, TrackAction, TrackFactory},
};
use anyhow::{anyhow, Result};
use eframe::egui::{self, Ui};
use groove_audio::AudioQueue;
use groove_core::{
    midi::{MidiChannel, MidiMessage},
    time::{MusicalTime, SampleRate, Tempo, TimeSignature},
    traits::{gui::Shows, Configurable, Generates, HandlesMidi, Ticks},
    StereoSample, Uid,
};
use groove_entities::{
    controllers::Arpeggiator,
    effects::{BiQuadFilterLowPass24db, Reverb},
    instruments::{Drumkit, WelshSynth},
};
use groove_toys::{ToyInstrument, ToySynth};
use rayon::prelude::{IntoParallelRefMutIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

/// Owns all entities (instruments, controllers, and effects), and manages the
/// relationships among them to create an audio performance.
#[derive(Serialize, Deserialize, Debug)]
pub struct MiniOrchestrator {
    title: Option<String>,
    time_signature: TimeSignature,
    tempo: Tempo,

    tracks: Vec<Track>,
    track_factory: TrackFactory,

    // If one track is selected, then this is set.
    single_track_selection_position: Option<usize>,

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
    musical_time: MusicalTime,
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
            single_track_selection_position: None,

            sample_rate: Default::default(),
            frames: Default::default(),
            musical_time: Default::default(),
        }
    }
}
impl MiniOrchestrator {
    #[allow(dead_code)]
    fn sample_rate(&self) -> SampleRate {
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

    // Fills in the given sample buffer with something simple and audible.
    #[allow(dead_code)]
    fn debug_sample_buffer(&mut self, samples: &mut [StereoSample]) {
        let len = samples.len() as f64;
        for (i, s) in samples.iter_mut().enumerate() {
            *s = StereoSample::from(i as f64 / len);
        }
    }

    /// Renders part of the project to audio, creating the requested number of
    /// [StereoSample]s and inserting them in the given [AudioQueue].
    pub fn provide_audio(&mut self, queue: &AudioQueue, samples_requested: usize) {
        const SAMPLE_BUFFER_SIZE: usize = 64;
        let mut samples = [StereoSample::SILENCE; SAMPLE_BUFFER_SIZE];

        // Round up
        let buffers_requested = (samples_requested + SAMPLE_BUFFER_SIZE - 1) / SAMPLE_BUFFER_SIZE;
        for _ in 0..buffers_requested {
            self.batch_values(&mut samples);
            for sample in samples {
                let _ = queue.push(sample);
            }
        }
    }

    /// Accepts a [MidiMessage] and handles it, usually by forwarding it to
    /// controllers and instruments on the given [MidiChannel].
    // TODO: we're ignoring channels at the moment.
    #[allow(unused_variables)]
    pub fn handle_midi(&mut self, channel: MidiChannel, message: MidiMessage) {
        for track in self.tracks.iter_mut() {
            track.handle_midi_message(&message, &mut |channel, message| {
                eprintln!("TODO discarding {}/{:?}", channel, message)
            });
        }
    }

    /// After loading a new Self from disk, we want to copy all the appropriate
    /// ephemeral state from this one to the next one.
    pub fn prepare_successor(&self, new: &mut MiniOrchestrator) {
        new.set_sample_rate(self.sample_rate());
    }

    #[allow(dead_code)]
    fn add_controller(&mut self, track_index: usize, mut e: Box<dyn NewIsController>) -> Uid {
        e.update_sample_rate(self.sample_rate);
        let uid = e.uid();
        self.tracks[track_index].append_controller(e);
        uid
    }

    #[allow(dead_code)]
    fn add_effect(&mut self, track_index: usize, mut e: Box<dyn NewIsEffect>) -> Uid {
        e.update_sample_rate(self.sample_rate);
        let uid = e.uid();
        self.tracks[track_index].append_effect(e);
        uid
    }

    #[allow(dead_code)]
    fn add_instrument(
        &mut self,
        track_index: usize,
        mut e: Box<dyn NewIsInstrument>,
    ) -> Result<Uid> {
        e.update_sample_rate(self.sample_rate);
        let uid = e.uid();
        self.tracks[track_index].append_instrument(e);
        Ok(uid)
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

    fn show_tracks(
        &mut self,
        ui: &mut Ui,
        _factory: &EntityFactory,
        is_control_only_down: bool,
    ) -> Option<TrackAction> {
        let mut action = None;

        // Non-send tracks are first
        for (index, track) in self.tracks.iter_mut().enumerate() {
            if !track.is_send() {
                if track.show(ui).clicked() {
                    action = Some(TrackAction::Select(index, is_control_only_down));
                }
            }
        }

        // Send tracks are last
        for (index, track) in self.tracks.iter_mut().enumerate() {
            if track.is_send() {
                if track.show(ui).clicked() {
                    action = Some(TrackAction::Select(index, is_control_only_down));
                }
            }
        }
        action
    }

    /// Renders the project's GUI.
    pub fn show_with(
        &mut self,
        ui: &mut egui::Ui,
        factory: &EntityFactory,
        is_control_only_down: bool,
    ) {
        if let Some(action) = self.show_tracks(ui, factory, is_control_only_down) {
            self.handle_track_action(factory, action);
        }
        if let Some(selected) = self.single_track_selection_position {
            let bottom = egui::TopBottomPanel::bottom("orchestrator-bottom-panel").resizable(true);
            bottom.show_inside(ui, |ui| {
                self.tracks[selected].show_detail(ui, factory, selected);
            });
        }
    }

    fn handle_track_action(&mut self, factory: &EntityFactory, action: TrackAction) {
        match action {
            TrackAction::NewController(track, key) => {
                // TODO: will instruments ever exist outside of tracks? If not,
                // then why go through the new/add/push sequence?
                if let Some(e) = factory.new_controller(&key) {
                    self.tracks[track].append_controller(e);
                }
            }
            TrackAction::NewEffect(track, key) => {
                if let Some(e) = factory.new_effect(&key) {
                    self.tracks[track].append_effect(e);
                }
            }
            TrackAction::NewInstrument(track, key) => {
                if let Some(e) = factory.new_instrument(&key) {
                    self.tracks[track].append_instrument(e);
                }
            }
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
    pub fn select_track(&mut self, index: usize, add_to_selections: bool) {
        let existing = self.tracks[index].selected();
        if !add_to_selections {
            self.clear_track_selections();
        }
        self.tracks[index].set_selected(!existing);
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
        self.single_track_selection_position = if count == 1 {
            self.tracks.iter().position(|t| t.selected())
        } else {
            None
        };
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
}
impl Generates<StereoSample> for MiniOrchestrator {
    fn value(&self) -> StereoSample {
        StereoSample::SILENCE
    }

    fn batch_values(&mut self, values: &mut [StereoSample]) {
        let len = values.len();
        self.tracks.par_iter_mut().for_each(|track| {
            track.batch_it_up(len);
        });
        self.tracks.iter().for_each(|t| {
            for i in 0..len {
                values[i] += t.buffer()[i];
            }
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

#[typetag::serde]
impl NewIsController for Arpeggiator {}
#[typetag::serde]
impl NewIsController for MiniSequencer {}
#[typetag::serde]
impl NewIsInstrument for Drumkit {}
#[typetag::serde]
impl NewIsInstrument for WelshSynth {}
#[typetag::serde]
impl NewIsInstrument for ToySynth {}
#[typetag::serde]
impl NewIsInstrument for ToyInstrument {}
#[typetag::serde]
impl NewIsEffect for BiQuadFilterLowPass24db {}
#[typetag::serde]
impl NewIsEffect for Reverb {}

#[cfg(test)]
mod tests {
    use crate::mini::orchestrator::MiniOrchestrator;
    use groove_toys::{ToyInstrument, ToyInstrumentParams};

    #[test]
    fn mini_orchestrator_basic_operations() {
        let mut o = MiniOrchestrator::default();

        // A new orchestrator should have at least one track.
        assert!(!o.tracks.is_empty());

        let id1 = o
            .add_instrument(
                0,
                Box::new(ToyInstrument::new_with(&ToyInstrumentParams::default())),
            )
            .unwrap();
        let id2 = o
            .add_instrument(
                0,
                Box::new(ToyInstrument::new_with(&ToyInstrumentParams::default())),
            )
            .unwrap();
        assert_eq!(o.tracks()[0].instruments()[0].uid(), id1);
        assert_eq!(o.tracks()[0].instruments()[1].uid(), id2);

        assert!(o.tracks.len() > 1);
        let id3 = o
            .add_instrument(
                1,
                Box::new(ToyInstrument::new_with(&ToyInstrumentParams::default())),
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
