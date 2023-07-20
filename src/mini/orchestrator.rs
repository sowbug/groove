// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{
    control_router::ControlRouter,
    selection_set::SelectionSet,
    track::{Track, TrackAction, TrackFactory, TrackTitle, TrackUid},
    transport::{Transport, TransportBuilder},
};
use anyhow::{anyhow, Result};
use eframe::{
    egui::{self, Frame, Ui},
    emath::{self, Align2},
    epaint::{pos2, vec2, Color32, FontId, Rect, Shape, Stroke},
};
use groove_audio::AudioQueue;
use groove_core::{
    control::ControlValue,
    midi::{MidiChannel, MidiMessage, MidiMessagesFn},
    time::{MusicalTime, SampleRate, Tempo},
    traits::{
        Configurable, ControlEventsFn, Controllable, Controls, Generates,
        GeneratesToInternalBuffer, HandlesMidi, Performs, Serializable, Thing, ThingEvent, Ticks,
    },
    Sample, StereoSample, Uid,
};
use rayon::prelude::{IntoParallelRefMutIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Debug, ops::Range};

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

    //////////////////////////////////////////////////////
    // Nothing below this comment should be serialized. //
    //////////////////////////////////////////////////////
    //
    #[serde(skip)]
    e: OrchestratorEphemerals,
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
            transport: TransportBuilder::default()
                .uid(Self::TRANSPORT_UID)
                .build()
                .unwrap(),
            control_router: Default::default(),

            track_factory,
            tracks,
            ordered_track_uids,

            e: Default::default(),
        }
    }
}
impl MiniOrchestrator {
    /// The expected size of any buffer provided for samples.
    //
    // TODO: how hard would it be to make this dynamic? Does adjustability
    // matter?
    pub const SAMPLE_BUFFER_SIZE: usize = 64;

    const TRANSPORT_UID: Uid = Uid(1);

    /// Adds the given [Thing] (instrument, controller, or entity), returning an
    /// assigned [Uid] if successful. [MiniOrchestrator] takes ownership.
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
    }

    /// Renders the project's GUI.
    #[must_use]
    pub fn show_with(
        &mut self,
        ui: &mut Ui,
        track_selection_set: &SelectionSet<TrackUid>,
    ) -> Option<TrackAction> {
        let mut action = self.ui_arrangement(
            ui,
            MusicalTime::new_with_beats(0)..MusicalTime::new_with_beats(128),
            track_selection_set,
        );

        if track_selection_set.len() == 1 {
            for track_uid in track_selection_set.iter() {
                if let Some(track) = self.tracks.get_mut(&track_uid) {
                    let bottom =
                        egui::TopBottomPanel::bottom("orchestrator-bottom-panel").resizable(true);
                    bottom.show_inside(ui, |_ui| {});
                    if let Some(a) = track.show_detail(ui) {
                        action = Some(a);
                    }
                }
            }
        }
        action
    }

    fn ui_arrangement<'a>(
        &mut self,
        ui: &mut Ui,
        viewable_time_range: Range<MusicalTime>,
        track_selection_set: &SelectionSet<TrackUid>,
    ) -> Option<TrackAction> {
        let mut action = None;

        Frame::canvas(ui.style()).show(ui, |ui| {
            const LEGEND_HEIGHT: f32 = 16.0;
            let (_id, rect) = ui.allocate_space(vec2(ui.available_width(), LEGEND_HEIGHT));
            let to_screen =
                emath::RectTransform::from_to(Rect::from_x_y_ranges(0.0..=1.0, 0.0..=1.0), rect);

            let font_id = FontId::proportional(12.0);
            let beat_count = (viewable_time_range.end.total_beats()
                - viewable_time_range.start.total_beats()) as usize;
            let skip = if beat_count > 100 {
                10
            } else if beat_count > 10 {
                2
            } else {
                1
            };
            for (i, beat) in (viewable_time_range.start.total_beats()
                ..viewable_time_range.end.total_beats())
                .enumerate()
            {
                if i != 0 && i != beat_count - 1 && i % skip != 0 {
                    continue;
                }
                let percentage = i as f32 / beat_count as f32;
                let beat_plus_one = beat + 1;
                let pos = to_screen * pos2(percentage, 0.0);
                let pos = pos2(pos.x, rect.bottom() - 1.0);
                ui.painter().text(
                    pos,
                    Align2::CENTER_BOTTOM,
                    format!("{beat_plus_one}"),
                    font_id.clone(),
                    Color32::YELLOW,
                );
            }
            let mut shapes = vec![];

            let left_x = (to_screen * pos2(0.0, 0.0)).x;
            let right_x = (to_screen * pos2(1.0, 0.0)).x;
            let line_points = [
                pos2(left_x, rect.bottom() - 1.0),
                pos2(right_x, rect.bottom() - 1.0),
            ];

            shapes.push(Shape::line_segment(
                line_points,
                Stroke {
                    color: Color32::YELLOW,
                    width: 1.0,
                },
            ));
            ui.painter().extend(shapes);

            // Non-send tracks are first, then send tracks
            let uids: Vec<&TrackUid> = self
                .ordered_track_uids
                .iter()
                .filter(|uid| !self.tracks.get(uid).unwrap().is_send())
                .chain(
                    self.ordered_track_uids
                        .iter()
                        .filter(|uid| self.tracks.get(uid).unwrap().is_send()),
                )
                .collect();
            let uids: Vec<TrackUid> = uids.iter().map(|u| (*u).clone()).collect();

            for uid in uids {
                let is_selected = track_selection_set.contains(&uid);
                if let Some(track) = self.get_track_mut(&uid) {
                    ui.allocate_ui(vec2(ui.available_width(), 64.0), |ui| {
                        Frame::default()
                            .stroke(Stroke {
                                width: if is_selected { 2.0 } else { 0.0 },
                                color: Color32::YELLOW,
                            })
                            .show(ui, |ui| {
                                let (response, a) = track.show(ui);
                                if let Some(a) = a {
                                    action = Some(a);
                                }
                                if response.clicked() {
                                    action = Some(TrackAction::Click(track.uid()))
                                };
                            })
                    });
                }
            }
        });
        action
    }

    /// Adds a new MIDI track, which can contain controllers, instruments, and
    /// effects. Returns the new track's [TrackUid] if successful.
    pub fn new_midi_track(&mut self) -> anyhow::Result<TrackUid> {
        let track = self.track_factory.midi();
        self.new_track(track)
    }

    /// Adds a new audio track, which can contain audio clips and effects.
    /// Returns the new track's [TrackUid] if successful.
    pub fn new_audio_track(&mut self) -> anyhow::Result<TrackUid> {
        let track = self.track_factory.audio();
        self.new_track(track)
    }

    /// Adds a new send track, which contains only effects, and which receives
    /// its input audio from other tracks. Returns the new track's [TrackUid] if
    /// successful.
    pub fn new_send_track(&mut self) -> anyhow::Result<TrackUid> {
        let track = self.track_factory.send();
        self.new_track(track)
    }

    fn new_track(&mut self, track: Track) -> anyhow::Result<TrackUid> {
        let uid = track.uid();
        self.ordered_track_uids.push(uid);
        self.tracks.insert(uid, track);
        Ok(uid)
    }

    /// Deletes the specified track.
    pub fn delete_track(&mut self, uid: &TrackUid) {
        self.tracks.remove(uid);
        self.ordered_track_uids.retain(|u| u != uid);
    }

    /// Sets a new title for the track.
    pub fn set_track_title(&mut self, uid: TrackUid, title: TrackTitle) {
        if let Some(track) = self.get_track_mut(&uid) {
            track.set_title(title);
        }
    }

    // #[allow(missing_docs)]
    // pub fn delete_selected_tracks(&mut self) {
    //     self.track_selection_set
    //         .clone()
    //         .iter()
    //         .for_each(|uid| self.delete_track(uid));
    //     self.track_selection_set.clear();
    // }

    /// Returns the name of the project.
    pub fn title(&self) -> Option<&String> {
        self.title.as_ref()
    }

    #[allow(missing_docs)]
    #[allow(dead_code)]
    pub fn set_title(&mut self, title: Option<String>) {
        self.title = title;
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

    #[allow(missing_docs)]
    pub fn transport(&self) -> &Transport {
        &self.transport
    }

    #[allow(missing_docs)]
    pub fn transport_mut(&mut self) -> &mut Transport {
        &mut self.transport
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
