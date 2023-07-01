// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{
    control_router::ControlRouter,
    entity_factory::{Thing, ThingStore, ThingType},
    midi_router::MidiRouter,
    sequencer::MiniSequencer,
};
use crate::mini::sequencer::MiniSequencerParams;
use eframe::{
    egui::{self, Frame, Layout, Margin, Response, Sense, Ui},
    emath::{self, Align},
    epaint::{pos2, vec2, Color32, Pos2, Rect, RectShape, Rounding, Shape, Stroke, Vec2},
};
use groove_core::{
    control::ControlValue,
    midi::MidiChannel,
    time::{SampleRate, Tempo, TimeSignature},
    traits::{
        gui::Shows, Configurable, ControlMessagesFn, Controls, GeneratesToInternalBuffer, Performs,
        Ticks,
    },
    StereoSample, Uid,
};
use groove_entities::EntityMessage;
use serde::{Deserialize, Serialize};

/// An ephemeral identifier for a [Track] in the current project.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct TrackIndex(pub usize);

#[derive(Debug)]
pub enum TrackElementAction {
    MoveDeviceLeft(usize),
    MoveDeviceRight(usize),
    RemoveDevice(usize),
}

#[derive(Debug)]
pub enum TrackAction {
    Select(TrackIndex, bool),
    SelectClear,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub enum TrackType {
    #[default]
    Midi,
    Audio,
    Send,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TrackFactory {
    next_midi: usize,
    next_audio: usize,
    next_send: usize,
}
impl Default for TrackFactory {
    fn default() -> Self {
        Self {
            next_midi: 1,
            next_audio: 1,
            next_send: 1,
        }
    }
}
impl TrackFactory {
    pub fn midi(&mut self) -> Track {
        let name = format!("MIDI {}", self.next_midi);
        self.next_midi += 1;
        Track {
            name,
            ty: TrackType::Midi,
            sequencer: Some(MiniSequencer::new_with(
                &MiniSequencerParams::default(),
                MidiChannel(0),
            )),
            ..Default::default()
        }
    }

    pub fn audio(&mut self) -> Track {
        let name = format!("Audio {}", self.next_audio);
        self.next_audio += 1;
        Track {
            name,
            ty: TrackType::Audio,
            ..Default::default()
        }
    }

    pub fn send(&mut self) -> Track {
        let name = format!("Send {}", self.next_send);
        self.next_send += 1;
        Track {
            name,
            ty: TrackType::Send,
            ..Default::default()
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Track {
    name: String,
    ty: TrackType,

    sequencer: Option<MiniSequencer>,
    thing_store: ThingStore,
    controllers: Vec<Uid>,
    instruments: Vec<Uid>,
    effects: Vec<Uid>,

    midi_router: MidiRouter,
    control_router: ControlRouter,

    // Whether the track is selected in the UI.
    is_selected: bool,

    #[serde(skip, default = "Track::init_buffer")]
    buffer: [StereoSample; 64],
}
impl Default for Track {
    fn default() -> Self {
        Self {
            name: String::from("Untitled"),
            ty: Default::default(),
            sequencer: Default::default(),
            thing_store: Default::default(),
            controllers: Default::default(),
            instruments: Default::default(),
            effects: Default::default(),
            midi_router: Default::default(),
            control_router: Default::default(),
            is_selected: Default::default(),
            buffer: [StereoSample::default(); 64],
        }
    }
}
impl Track {
    fn init_buffer() -> [StereoSample; 64] {
        [StereoSample::default(); 64]
    }

    pub fn is_send(&self) -> bool {
        matches!(self.ty, TrackType::Send)
    }

    // TODO: for now the only way to add something new to a Track is to append it.
    pub fn append_thing(&mut self, thing: Box<dyn Thing>) -> Uid {
        let uid = thing.uid();
        match thing.thing_type() {
            ThingType::Unknown => {
                panic!("append_thing({:#?}) -> unknown type", thing)
            }
            ThingType::Controller => {
                self.controllers.push(uid);
            }
            ThingType::Effect => {
                self.effects.push(uid);
            }
            ThingType::Instrument => {
                self.instruments.push(uid);
            }
        }
        self.thing_store.add(thing);

        // TODO: for now, everyone's on channel 0
        self.midi_router.connect(uid, MidiChannel(0));

        uid
    }

    pub fn remove_thing(&mut self, uid: &Uid) -> Option<Box<dyn Thing>> {
        if let Some(thing) = self.thing_store.remove(uid) {
            match thing.thing_type() {
                ThingType::Unknown => eprintln!("Warning: removed thing id {uid} of unknown type"),
                ThingType::Controller => self.controllers.retain(|e| e != uid),
                ThingType::Effect => self.effects.retain(|e| e != uid),
                ThingType::Instrument => self.instruments.retain(|e| e != uid),
            }
            Some(thing)
        } else {
            None
        }
    }

    // pub fn insert_thing(&mut self, index: usize, uid: Uid) -> Result<()> {
    //     match
    //     if index > self.things.len() {
    //         return Err(anyhow!(
    //             "can't insert at {} in {}-length vec",
    //             index,
    //             self.things.len()
    //         ));
    //     }
    //     self.things.insert(index, uid);
    //     Ok(())
    // }

    // pub fn insert_controller(&mut self, index: usize, e: Box<dyn NewIsController>) -> Result<()> {
    //     if index > self.controllers.len() {
    //         return Err(anyhow!(
    //             "can't insert at {} in {}-length vec",
    //             index,
    //             self.controllers.len()        self.midi_router.connect(uid, MidiChannel(0));

    //         ));
    //     }
    //     self.controllers.insert(index, e);
    //     Ok(())
    // }

    // pub fn insert_effect(&mut self, index: usize, e: Box<dyn NewIsEffect>) -> Result<()> {
    //     if index > self.effects.len() {
    //         return Err(anyhow!(
    //             "can't insert at {} in {}-length vec",
    //             index,
    //             self.effects.len()
    //         ));
    //     }
    //     self.effects.insert(index, e);
    //     Ok(())
    // }

    // pub fn insert_instrument(&mut self, index: usize, e: Box<dyn NewIsInstrument>) -> Result<()> {
    //     if index > self.instruments.len() {
    //         return Err(anyhow!(
    //             "can't insert at {} in {}-length vec",
    //             index,
    //             self.instruments.len()
    //         ));
    //     }
    //     self.instruments.insert(index, e);
    //     Ok(())
    // }

    // fn shift_device_left(&mut self, index: usize) -> Result<()> {
    //     if index >= self.things.len() {
    //         return Err(anyhow!("Index {index} out of bounds."));
    //     }
    //     if index == 0 {
    //         return Err(anyhow!("Can't move leftmost item farther left."));
    //     }
    //     let element = self.things.remove(index);
    //     self.insert_thing(index - 1, element)
    // }
    // fn shift_device_right(&mut self, index: usize) -> Result<()> {
    //     if index >= self.things.len() {
    //         return Err(anyhow!("Index {index} out of bounds."));
    //     }
    //     if index == self.things.len() - 1 {
    //         return Err(anyhow!("Can't move rightmost item farther right."));
    //     }
    //     let element = self.things.remove(index);
    //     self.insert_thing(index + 1, element)
    // }

    // fn shift_controller_left(&mut self, index: usize) -> Result<()> {
    //     if index >= self.controllers.len() {
    //         return Err(anyhow!("Index {index} out of bounds."));
    //     }
    //     if index == 0 {
    //         return Err(anyhow!("Can't move leftmost item farther left."));
    //     }
    //     let element = self.controllers.remove(index);
    //     self.insert_controller(index - 1, element)
    // }
    // fn shift_controller_right(&mut self, index: usize) -> Result<()> {
    //     if index >= self.controllers.len() {
    //         return Err(anyhow!("Index {index} out of bounds."));
    //     }
    //     if index == self.controllers.len() - 1 {
    //         return Err(anyhow!("Can't move rightmost item farther right."));
    //     }
    //     let element = self.controllers.remove(index);
    //     self.insert_controller(index + 1, element)
    // }

    // fn shift_effect_left(&mut self, index: usize) -> Result<()> {
    //     if index >= self.effects.len() {
    //         return Err(anyhow!("Index {index} out of bounds."));
    //     }
    //     if index == 0 {
    //         return Err(anyhow!("Can't move leftmost item farther left."));
    //     }
    //     let element = self.effects.remove(index);
    //     self.insert_effect(index - 1, element)
    // }
    // fn shift_effect_right(&mut self, index: usize) -> Result<()> {
    //     if index >= self.effects.len() {
    //         return Err(anyhow!("Index {index} out of bounds."));
    //     }
    //     if index == self.effects.len() - 1 {
    //         return Err(anyhow!("Can't move rightmost item farther right."));
    //     }
    //     let element = self.effects.remove(index);
    //     self.insert_effect(index + 1, element)
    // }

    // fn shift_instrument_left(&mut self, index: usize) -> Result<()> {
    //     if index >= self.instruments.len() {
    //         return Err(anyhow!("Index {index} out of bounds."));
    //     }
    //     if index == 0 {
    //         return Err(anyhow!("Can't move leftmost item farther left."));
    //     }
    //     let element = self.instruments.remove(index);
    //     self.insert_instrument(index - 1, element)
    // }
    // fn shift_instrument_right(&mut self, index: usize) -> Result<()> {
    //     if index >= self.instruments.len() {
    //         return Err(anyhow!("Index {index} out of bounds."));
    //     }
    //     if index == self.instruments.len() - 1 {
    //         return Err(anyhow!("Can't move rightmost item farther right."));
    //     }
    //     let element = self.instruments.remove(index);
    //     self.insert_instrument(index + 1, element)
    // }

    fn button_states(index: usize, len: usize) -> (bool, bool) {
        let left = index != 0;
        let right = len > 1 && index != len - 1;
        (left, right)
    }

    fn draw_temp_squiggles(&self, ui: &mut Ui) -> Response {
        ui.ctx().request_repaint();

        let color = if ui.visuals().dark_mode {
            Color32::from_additive_luminance(196)
        } else {
            Color32::from_black_alpha(240)
        };

        let (response, painter) =
            ui.allocate_painter(vec2(ui.available_width(), 64.0), Sense::click());

        let time = ui.input(|i| i.time);
        let to_screen = emath::RectTransform::from_to(
            Rect::from_x_y_ranges(0.0..=1.0, -1.0..=1.0),
            response.rect,
        );

        let mut shapes = vec![];
        if self.is_selected {
            shapes.push(Shape::Rect(RectShape::filled(
                painter.clip_rect(),
                Rounding::none(),
                Color32::DARK_BLUE,
            )));
        }

        for &mode in &[2, 3, 5] {
            let mode = mode as f64;
            let n = 120;
            let speed = 1.5;

            let points: Vec<Pos2> = (0..=n)
                .map(|i| {
                    let t = i as f64 / (n as f64);
                    let amp = (time * speed * mode).sin() / mode;
                    let y = amp * (t * std::f64::consts::TAU / 2.0 * mode).sin();
                    to_screen * pos2(t as f32, y as f32)
                })
                .collect();

            let thickness = 10.0 / mode as f32;
            shapes.push(Shape::line(points, Stroke::new(thickness, color)));
        }

        shapes.push(Shape::LineSegment {
            points: [to_screen * pos2(0.0, 1.0), to_screen * pos2(1.0, 1.0)],
            stroke: Stroke { width: 1.0, color },
        });

        painter.extend(shapes);

        response
    }

    fn show_midi(&mut self, ui: &mut Ui) -> Response {
        if let Some(sequencer) = self.sequencer.as_mut() {
            sequencer.show_arrangement(ui)
        } else {
            eprintln!("Hmmm, no sequencer in a MIDI track?");
            ui.allocate_ui(ui.available_size(), |_ui| {}).response
        }
    }

    fn show_audio(&mut self, ui: &mut Ui) -> Response {
        self.draw_temp_squiggles(ui)
    }

    // TODO: ordering should be controllers, instruments, then effects. Within
    // those groups, the user can reorder as desired (but instrument order
    // doesn't matter because they're all simultaneous)
    #[must_use]
    pub fn show_detail(&mut self, ui: &mut Ui) -> Option<TrackAction> {
        let style = ui.visuals().widgets.inactive;
        let action = None;

        ui.with_layout(
            egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
            |ui| {
                let desired_size = Vec2::new(ui.available_width(), 256.0 - style.fg_stroke.width);
                ui.set_min_size(desired_size);
                ui.set_max_size(desired_size);

                ui.horizontal_centered(|ui| {
                    let desired_size = Vec2::new(512.0, ui.available_height());

                    let mut action = None;

                    if let Some(sequencer) = self.sequencer.as_mut() {
                        if let Some(a) = Self::add_track_element(ui, 0, false, false, true, |ui| {
                            sequencer.show(ui);
                        }) {
                            action = Some(a);
                        };
                    }

                    let len = self.controllers.len();
                    for (index, uid) in self.controllers.iter().enumerate() {
                        let index = index + 1;
                        ui.allocate_ui(desired_size, |ui| {
                            let (show_left, show_right) = Self::button_states(index, len);
                            if let Some(a) = Self::add_track_element(
                                ui,
                                index,
                                show_left,
                                show_right,
                                true,
                                |ui| {
                                    if let Some(e) = self.thing_store.get_mut(uid) {
                                        e.show(ui);
                                    }
                                },
                            ) {
                                action = Some(a);
                            };
                        });
                    }
                    let len = self.instruments.len();
                    for (index, uid) in self.instruments.iter().enumerate() {
                        let index = index + 1;
                        ui.allocate_ui(desired_size, |ui| {
                            let (show_left, show_right) = Self::button_states(index, len);
                            if let Some(a) = Self::add_track_element(
                                ui,
                                index,
                                show_left,
                                show_right,
                                true,
                                |ui| {
                                    if let Some(e) = self.thing_store.get_mut(uid) {
                                        e.show(ui);
                                    }
                                },
                            ) {
                                action = Some(a);
                            };
                        });
                    }
                    let len = self.effects.len();
                    for (index, uid) in self.effects.iter().enumerate() {
                        let index = index + 1;
                        ui.allocate_ui(desired_size, |ui| {
                            let (show_left, show_right) = Self::button_states(index, len);
                            if let Some(a) = Self::add_track_element(
                                ui,
                                index,
                                show_left,
                                show_right,
                                true,
                                |ui| {
                                    if let Some(e) = self.thing_store.get_mut(uid) {
                                        e.show(ui);
                                    }
                                },
                            ) {
                                action = Some(a);
                            };
                        });
                    }

                    // check action
                    // if let Some(action) = action {
                    // match action {
                    //     TrackElementAction::MoveDeviceLeft(index) => {
                    //         let _ = self.shift_device_left(index);
                    //     }
                    //     TrackElementAction::MoveDeviceRight(index) => {
                    //         let _ = self.shift_device_right(index);
                    //     }
                    //     TrackElementAction::RemoveDevice(index) => {
                    //         let _ = self.remove_thing(index);
                    //     }
                    // }
                    // }
                });
            },
        );
        action
    }

    fn add_track_element(
        ui: &mut Ui,
        index: usize,
        show_left_button: bool,
        show_right_button: bool,
        show_delete_button: bool,
        add_contents: impl FnOnce(&mut Ui),
    ) -> Option<TrackElementAction> {
        let mut action = None;
        let style = ui.visuals().widgets.inactive;
        Frame::none()
            .stroke(style.fg_stroke)
            .inner_margin(Margin::same(2.0))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                        if show_left_button {
                            if ui.button("<").clicked() {
                                action = Some(TrackElementAction::MoveDeviceLeft(index));
                            }
                        }
                        if show_right_button {
                            if ui.button(">").clicked() {
                                action = Some(TrackElementAction::MoveDeviceRight(index));
                            }
                        }
                        if show_delete_button {
                            if ui.button("x").clicked() {
                                action = Some(TrackElementAction::RemoveDevice(index));
                            }
                        }
                    });
                    ui.vertical(|ui| {
                        add_contents(ui);
                    });
                });
            });
        action
    }

    pub fn show(&mut self, ui: &mut Ui) -> Response {
        ui.allocate_ui(vec2(ui.available_width(), 64.0), |ui| {
            Frame::default()
                .stroke(Stroke {
                    width: if self.is_selected { 2.0 } else { 0.0 },
                    color: Color32::YELLOW,
                })
                .show(ui, |ui| {
                    let response = Frame::default()
                        .fill(Color32::GRAY)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.text_edit_singleline(&mut self.name);
                                ui.allocate_response(
                                    ui.available_size_before_wrap(),
                                    Sense::click(),
                                )
                            })
                            .inner
                        })
                        .inner;
                    match self.ty {
                        TrackType::Midi => {
                            self.show_midi(ui);
                        }
                        TrackType::Audio => {
                            self.show_audio(ui);
                        }
                        TrackType::Send => {
                            // For now, the title bar is enough for a send track, which holds only effects.
                        }
                    }
                    response
                })
                .inner
        })
        .inner
    }

    pub fn remove_selected_patterns(&mut self) {
        if let Some(sequencer) = self.sequencer.as_mut() {
            sequencer.remove_selected_patterns();
        }
    }

    pub fn selected(&self) -> bool {
        self.is_selected
    }

    pub fn set_selected(&mut self, selected: bool) {
        self.is_selected = selected;
    }

    pub fn buffer(&self) -> [StereoSample; 64] {
        self.buffer
    }

    pub fn route_midi_message(
        &mut self,
        channel: MidiChannel,
        message: groove_core::midi::MidiMessage,
    ) {
        if let Err(e) = self
            .midi_router
            .route(&mut self.thing_store, channel, message)
        {
            eprintln!("While routing: {e}");
        }
    }

    pub fn route_control_change(&mut self, uid: Uid, value: ControlValue) {
        if let Err(e) = self.control_router.route(&mut self.thing_store, uid, value) {
            eprintln!("While routing control change: {e}")
        }
    }
}
impl GeneratesToInternalBuffer<StereoSample> for Track {
    fn generate_batch_values(&mut self, len: usize) -> usize {
        if len > self.buffer.len() {
            eprintln!(
                "requested {} samples but buffer is only len {}",
                len,
                self.buffer.len()
            );
            return 0;
        }

        self.buffer.fill(StereoSample::SILENCE);
        for uid in self.instruments.iter() {
            if let Some(e) = self.thing_store.get_mut(uid) {
                if let Some(e) = e.as_instrument_mut() {
                    // Note that we're expecting everyone to ADD to the buffer,
                    // not to overwrite! TODO: convert all instruments to have
                    // internal buffers
                    e.generate_batch_values(&mut self.buffer);
                }
            }
        }

        // TODO: change this trait to operate on batches.
        for uid in self.effects.iter() {
            if let Some(e) = self.thing_store.get_mut(uid) {
                if let Some(e) = e.as_effect_mut() {
                    for sample in self.buffer.iter_mut() {
                        *sample = e.transform_audio(*sample);
                    }
                }
            }
        }

        self.buffer.len()
    }

    fn values(&self) -> &[StereoSample] {
        &self.buffer
    }
}
impl Ticks for Track {
    fn tick(&mut self, tick_count: usize) {
        self.thing_store.tick(tick_count);
    }
}
impl Configurable for Track {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.thing_store.update_sample_rate(sample_rate);
    }

    fn update_tempo(&mut self, tempo: Tempo) {
        self.thing_store.update_tempo(tempo);
    }

    fn update_time_signature(&mut self, time_signature: TimeSignature) {
        self.thing_store.update_time_signature(time_signature);
    }
}

// TODO: I think this is wrong and misguided. If MIDI messages are handled by
// Track, then each Track needs to record who's receiving on which channel, and
// messages can't be sent from a device on one track to one on a different
// track. While that could make parallelism easier, it doesn't seem intuitively
// correct, because in a real studio you'd be able to hook up MIDI cables
// independently of audio cables.
#[cfg(never)]
impl HandlesMidi for Track {
    fn handle_midi_message(
        &mut self,
        channel: MidiChannel,
        message: MidiMessage,
        messages_fn: &mut dyn FnMut(Uid, MidiChannel, MidiMessage),
    ) {
        for e in self.controllers.iter_mut() {
            e.handle_midi_message(channel, &message, messages_fn);
        }
        for e in self.instruments.iter_mut() {
            e.handle_midi_message(channel, &message, messages_fn);
        }
    }
}
impl Controls for Track {
    type Message = EntityMessage;

    fn update_time(&mut self, range: &std::ops::Range<groove_core::time::MusicalTime>) {
        self.thing_store.update_time(range);
    }

    fn work(&mut self, control_messages_fn: &mut ControlMessagesFn<Self::Message>) {
        self.thing_store.work(control_messages_fn);
    }

    fn is_finished(&self) -> bool {
        self.thing_store.is_finished()
    }
}
impl Performs for Track {
    fn play(&mut self) {
        self.thing_store.play();
    }

    fn stop(&mut self) {
        self.thing_store.stop();
    }

    fn skip_to_start(&mut self) {
        self.thing_store.skip_to_start();
    }

    fn is_performing(&self) -> bool {
        self.thing_store.is_performing()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use groove_core::traits::HasUid;
    use groove_toys::{ToyInstrument, ToyInstrumentParams};

    #[test]
    fn basic_track_operations() {
        let mut t = Track::default();
        assert!(t.controllers.is_empty());
        assert!(t.effects.is_empty());
        assert!(t.instruments.is_empty());

        // Create an instrument and add it to a track.
        let mut instrument = ToyInstrument::new_with(&ToyInstrumentParams::default());
        instrument.set_uid(Uid(1));
        let id1 = t.append_thing(Box::new(instrument));

        // Add a second instrument to the track.
        let mut instrument = ToyInstrument::new_with(&ToyInstrumentParams::default());
        instrument.set_uid(Uid(2));
        let id2 = t.append_thing(Box::new(instrument));

        assert_ne!(id1, id2, "Don't forget to assign UIDs!");

        assert_eq!(
            t.instruments[0], id1,
            "first appended entity should be at index 0"
        );
        assert_eq!(
            t.instruments[1], id2,
            "second appended entity should be at index 1"
        );
        assert_eq!(
            t.instruments.len(),
            2,
            "there should be exactly as many entities as added"
        );

        let instrument = t.remove_thing(&id1).unwrap();
        assert_eq!(instrument.uid(), id1, "removed the right instrument");
        assert_eq!(t.instruments.len(), 1, "removed exactly one instrument");
        assert_eq!(
            t.instruments[0], id2,
            "the remaining instrument should be the one we left"
        );
        assert!(
            t.thing_store.get(&id1).is_none(),
            "it should be gone from the store"
        );
    }
}
