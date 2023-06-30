// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{
    entity_factory::{EntityFactory, Thing},
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
        Performs, Ticks,
    },
    Sample, StereoSample, Uid,
};
use groove_entities::EntityMessage;
use rayon::prelude::{IntoParallelRefMutIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, ops::Range, sync::Arc};

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

    // The current time range for Controls::work().
    #[serde(skip)]
    range: Range<MusicalTime>,

    #[serde(skip)]
    entity_factory: Option<Arc<EntityFactory>>,

    #[serde(skip)]
    messages: Vec<EntityMessage>,

    #[serde(skip)]
    is_finished: bool,

    #[serde(skip)]
    is_performing: bool,
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

            sample_rate: Default::default(),
            frames: Default::default(),
            current_time: Default::default(),
            range: Default::default(),
            entity_factory: None,
            messages: Default::default(),
            is_finished: Default::default(),
            is_performing: Default::default(),
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

    /// Renders the next set of samples into the provided buffer. This is the
    /// main event loop.
    pub fn generate_next_samples(&mut self, samples: &mut [StereoSample]) {
        // Calculate the work time range. Note that we make sure the range is
        // length > 0, which can mean that we will call update_time() twice with
        // the same range if the sample rate is extremely high. TODO: we should
        // decide whose responsibility it is to handle that -- either we skip
        // calling work() if the time range is the same as prior, or everyone
        // who gets called needs to detect the case or be idempotent.
        let start = self.current_time;
        let units = 1.max(MusicalTime::frames_to_units(
            self.tempo,
            self.sample_rate,
            samples.len(),
        ));
        let length = MusicalTime::new_with_units(units);
        let range = start..start + length;
        self.update_time(&range);
        self.work(&mut |_| {});
        self.generate_batch_values(samples);
        self.current_time += length;
    }

    /// After loading a new Self from disk, we want to copy all the appropriate
    /// ephemeral state from this one to the next one.
    pub fn prepare_successor(&self, new: &mut MiniOrchestrator) {
        new.set_sample_rate(self.sample_rate());

        // TODO: refresh EntityFactory's relaxed counter(s) to account for
        // existing items
    }

    // #[allow(dead_code)]
    // fn move_controller(
    //     &mut self,
    //     old_track_index: usize,
    //     old_item_index: usize,
    //     new_track_index: usize,
    //     new_item_index: usize,
    // ) -> Result<()> {
    //     if let Some(e) = self.tracks[old_track_index].remove_controller(old_item_index) {
    //         self.tracks[new_track_index].insert_controller(new_item_index, e)
    //     } else {
    //         Err(anyhow!("controller not found"))
    //     }
    // }

    // #[allow(dead_code)]
    // fn move_effect(
    //     &mut self,
    //     old_track_index: usize,
    //     old_item_index: usize,
    //     new_track_index: usize,
    //     new_item_index: usize,
    // ) -> Result<()> {
    //     if let Some(e) = self.tracks[old_track_index].remove_effect(old_item_index) {
    //         self.tracks[new_track_index].insert_effect(new_item_index, e)
    //     } else {
    //         Err(anyhow!("effect not found"))
    //     }
    // }

    // #[allow(dead_code)]
    // fn move_instrument(
    //     &mut self,
    //     old_track_index: usize,
    //     old_item_index: usize,
    //     new_track_index: usize,
    //     new_item_index: usize,
    // ) -> Result<()> {
    //     if let Some(e) = self.tracks[old_track_index].remove_instrument(old_item_index) {
    //         self.tracks[new_track_index].insert_instrument(new_item_index, e)
    //     } else {
    //         Err(anyhow!("instrument not found"))
    //     }
    // }

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

    /// Adds a new thing with the specified [Key] to the currently selected
    /// single track. Fails if anything but exactly one track is selected.
    pub fn add_thing_by_key_to_selected_track(&mut self, key: &Key) -> Result<Uid> {
        if let Some(track) = self.single_track_selection() {
            self.add_thing_by_key_to_track(key, track)
        } else {
            Err(anyhow!("A single track was not selected"))
        }
    }

    /// Adds a new thing with the specified [Key] to the track with the specified [TrackIndex].
    pub fn add_thing_by_key_to_track(&mut self, key: &Key, track: TrackIndex) -> Result<Uid> {
        if let Some(factory) = &self.entity_factory {
            if let Some(e) = factory.new_thing(key) {
                self.add_thing(e, track)
            } else {
                Err(anyhow!("key {key} not found"))
            }
        } else {
            Err(anyhow!("there is no entity factory"))
        }
    }

    /// Adds the given thing, returning an assigned [Uid] if successful.
    /// [MiniOrchestrator] takes ownership.
    pub fn add_thing(&mut self, mut thing: Box<dyn Thing>, track: TrackIndex) -> Result<Uid> {
        thing.update_sample_rate(self.sample_rate);
        let uid = thing.uid();
        self.tracks[track.0].append_thing(thing);
        Ok(uid)
    }

    // /// The entities receiving on the given MIDI channel.
    // pub fn midi_receivers(&mut self, channel: &MidiChannel) -> &Vec<Uid> {
    //     self.mi
    //     self.midi_channel_to_receiver_uid
    //         .entry(*channel)
    //         .or_default()
    // }

    // /// Connect an entity to the given MIDI channel.
    // pub fn connect_midi_receiver(&mut self, receiver_uid: Uid, channel: MidiChannel) {
    //     self.midi_channel_to_receiver_uid
    //         .entry(channel)
    //         .or_default()
    //         .push(receiver_uid);
    // }

    // /// Disconnect an entity from the given MIDI channel.
    // pub fn disconnect_midi_receiver(&mut self, receiver_uid: Uid, channel: MidiChannel) {
    //     self.midi_channel_to_receiver_uid
    //         .entry(channel)
    //         .or_default()
    //         .retain(|&uid| uid != receiver_uid);
    // }

    /// Returns the global music clock.
    pub fn current_time(&self) -> MusicalTime {
        self.current_time
    }

    fn calculate_is_finished(&self) -> bool {
        self.tracks.iter().all(|t| t.is_finished())
    }

    fn dispatch_message(&mut self, message: EntityMessage) {
        match message {
            EntityMessage::Midi(channel, message) => {
                self.handle_midi_message(channel, message, &mut |_, _| {})
            }
            EntityMessage::ControlF32(_) => todo!(),
            EntityMessage::HandleControlF32(_, _) => todo!(),
        }
    }

    fn route_midi_message(&mut self, channel: MidiChannel, message: MidiMessage) {
        // TODO: I'm starting to want messages_fn to be an Option<> and to be
        // able to panic when someone unexpectedly sets it. Or maybe there is a
        // different trait, like one routes and one handles...
        for t in self.tracks.iter_mut() {
            t.route_midi_message(channel, message);
        }
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
            let copy_len = len.min(generator_values.len());
            for i in 0..copy_len {
                values[i] = generator_values[i];
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
impl HandlesMidi for MiniOrchestrator {
    /// Accepts a [MidiMessage] and handles it, usually by forwarding it to
    /// controllers and instruments on the given [MidiChannel]. We implement
    /// this trait only for external messages; for ones generated internally, we
    /// use [MidiRouter].
    fn handle_midi_message(
        &mut self,
        channel: MidiChannel,
        message: MidiMessage,
        _: &mut dyn FnMut(MidiChannel, MidiMessage),
    ) {
        self.route_midi_message(channel, message);
    }
}
impl Performs for MiniOrchestrator {
    fn play(&mut self) {
        self.is_performing = true;
        self.tracks.iter_mut().for_each(|t| t.play());
    }

    fn stop(&mut self) {
        // If we were performing, stop. Otherwise, it's a stop-while-stopped
        // action, which means the user wants to rewind to the beginning.
        if self.is_performing {
            self.is_performing = false;
        } else {
            self.skip_to_start();
        }
        self.tracks.iter_mut().for_each(|t| t.stop());
    }

    fn skip_to_start(&mut self) {
        self.current_time = MusicalTime::START;
        self.tracks.iter_mut().for_each(|t| t.skip_to_start());
    }

    fn is_performing(&self) -> bool {
        self.is_performing
    }
}
impl Controls for MiniOrchestrator {
    type Message = EntityMessage;

    fn update_time(&mut self, range: &Range<MusicalTime>) {
        self.range = range.clone();

        for track in self.tracks.iter_mut() {
            track.update_time(&self.range);
        }
    }

    fn work(&mut self, _: &mut dyn FnMut(Self::Message)) {
        for track in self.tracks.iter_mut() {
            track.work(&mut |m| self.messages.push(m));
        }
        while let Some(message) = self.messages.pop() {
            self.dispatch_message(message);
        }
        self.is_finished = self.calculate_is_finished();
    }

    fn is_finished(&self) -> bool {
        self.is_finished
    }
}

#[cfg(test)]
mod tests {
    use crate::mini::{orchestrator::MiniOrchestrator, TrackIndex};
    use groove_core::{
        time::{MusicalTime, SampleRate, Tempo},
        traits::{Controls, Performs},
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
        o.set_sample_rate(new_sample_rate);
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
        o.set_tempo(new_tempo);
        assert_eq!(o.tempo(), new_tempo, "Tempo should be settable");
    }

    #[test]
    fn exposes_traits_ergonomically() {
        let mut o = MiniOrchestrator::default();

        // TODO: worst ergonomics ever.
        const TIMER_DURATION: MusicalTime = MusicalTime::new_with_beats(1);
        let _ = o.add_thing(
            Box::new(Timer::new_with(&TimerParams {
                duration: groove_core::time::MusicalTimeParams {
                    units: TIMER_DURATION.total_units(),
                },
            })),
            TrackIndex(0),
        );

        o.play();
        let mut prior_start_time = MusicalTime::start_of_time();
        loop {
            if o.is_finished() {
                break;
            }
            prior_start_time = o.current_time;
            let mut samples = [StereoSample::SILENCE; 1];
            o.generate_next_samples(&mut samples);
        }
        let prior_range = prior_start_time..o.current_time();
        assert!(prior_range.contains(&TIMER_DURATION));
    }
}
