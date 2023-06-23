// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::entities::{NewIsController, NewIsEffect, NewIsInstrument};
use super::entity_factory::{EntityFactory, EntityType, Key};
use super::sequencer::MiniSequencer;
use crate::mini::sequencer::MiniSequencerParams;
use anyhow::{anyhow, Result};
use eframe::{
    egui::{self, Frame, Layout, Margin, Response, Sense, Ui},
    emath::{self, Align},
    epaint::{pos2, vec2, Color32, Pos2, Rect, RectShape, Rounding, Shape, Stroke, Vec2},
};
use groove_core::{
    midi::{MidiChannel, MidiMessage},
    time::{SampleRate, Tempo, TimeSignature},
    traits::{gui::Shows, Configurable, Generates, HandlesMidi, Ticks},
    StereoSample,
};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum TrackElementAction {
    MoveControllerLeft(usize),
    MoveControllerRight(usize),
    RemoveController(usize),
    MoveEffectLeft(usize),
    MoveEffectRight(usize),
    RemoveEffect(usize),
    MoveInstrumentLeft(usize),
    MoveInstrumentRight(usize),
    RemoveInstrument(usize),
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum TrackAction {
    NewController(usize, Key),
    NewEffect(usize, Key),
    NewInstrument(usize, Key),
    Select(usize, bool),
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
                MidiChannel::new(0),
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
    controllers: Vec<Box<dyn NewIsController>>,
    instruments: Vec<Box<dyn NewIsInstrument>>,
    effects: Vec<Box<dyn NewIsEffect>>,

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
            controllers: Default::default(),
            instruments: Default::default(),
            effects: Default::default(),
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

    pub fn instruments(&self) -> &[Box<dyn NewIsInstrument>] {
        &self.instruments
    }

    // TODO: this is getting cumbersome! Think about that uber-trait!

    #[allow(dead_code)]
    fn controller(&self, index: usize) -> Option<&Box<dyn NewIsController>> {
        self.controllers.get(index)
    }

    #[allow(dead_code)]
    fn controller_mut(&mut self, index: usize) -> Option<&mut Box<dyn NewIsController>> {
        self.controllers.get_mut(index)
    }

    #[allow(dead_code)]
    fn effect(&self, index: usize) -> Option<&Box<dyn NewIsEffect>> {
        self.effects.get(index)
    }

    #[allow(dead_code)]
    fn effect_mut(&mut self, index: usize) -> Option<&mut Box<dyn NewIsEffect>> {
        self.effects.get_mut(index)
    }

    #[allow(dead_code)]
    fn instrument(&self, index: usize) -> Option<&Box<dyn NewIsInstrument>> {
        self.instruments.get(index)
    }

    #[allow(dead_code)]
    fn instrument_mut(&mut self, index: usize) -> Option<&mut Box<dyn NewIsInstrument>> {
        self.instruments.get_mut(index)
    }

    pub fn append_controller(&mut self, e: Box<dyn NewIsController>) {
        self.controllers.push(e);
    }

    pub fn append_effect(&mut self, e: Box<dyn NewIsEffect>) {
        self.effects.push(e);
    }

    pub fn append_instrument(&mut self, e: Box<dyn NewIsInstrument>) {
        self.instruments.push(e);
    }

    pub fn remove_controller(&mut self, index: usize) -> Option<Box<dyn NewIsController>> {
        Some(self.controllers.remove(index))
    }

    pub fn remove_effect(&mut self, index: usize) -> Option<Box<dyn NewIsEffect>> {
        Some(self.effects.remove(index))
    }

    pub fn remove_instrument(&mut self, index: usize) -> Option<Box<dyn NewIsInstrument>> {
        Some(self.instruments.remove(index))
    }

    pub fn insert_controller(&mut self, index: usize, e: Box<dyn NewIsController>) -> Result<()> {
        if index > self.controllers.len() {
            return Err(anyhow!(
                "can't insert at {} in {}-length vec",
                index,
                self.controllers.len()
            ));
        }
        self.controllers.insert(index, e);
        Ok(())
    }

    pub fn insert_effect(&mut self, index: usize, e: Box<dyn NewIsEffect>) -> Result<()> {
        if index > self.effects.len() {
            return Err(anyhow!(
                "can't insert at {} in {}-length vec",
                index,
                self.effects.len()
            ));
        }
        self.effects.insert(index, e);
        Ok(())
    }

    pub fn insert_instrument(&mut self, index: usize, e: Box<dyn NewIsInstrument>) -> Result<()> {
        if index > self.instruments.len() {
            return Err(anyhow!(
                "can't insert at {} in {}-length vec",
                index,
                self.instruments.len()
            ));
        }
        self.instruments.insert(index, e);
        Ok(())
    }

    fn shift_controller_left(&mut self, index: usize) -> Result<()> {
        if index >= self.controllers.len() {
            return Err(anyhow!("Index {index} out of bounds."));
        }
        if index == 0 {
            return Err(anyhow!("Can't move leftmost item farther left."));
        }
        let element = self.controllers.remove(index);
        self.insert_controller(index - 1, element)
    }
    fn shift_controller_right(&mut self, index: usize) -> Result<()> {
        if index >= self.controllers.len() {
            return Err(anyhow!("Index {index} out of bounds."));
        }
        if index == self.controllers.len() - 1 {
            return Err(anyhow!("Can't move rightmost item farther right."));
        }
        let element = self.controllers.remove(index);
        self.insert_controller(index + 1, element)
    }

    fn shift_effect_left(&mut self, index: usize) -> Result<()> {
        if index >= self.effects.len() {
            return Err(anyhow!("Index {index} out of bounds."));
        }
        if index == 0 {
            return Err(anyhow!("Can't move leftmost item farther left."));
        }
        let element = self.effects.remove(index);
        self.insert_effect(index - 1, element)
    }
    fn shift_effect_right(&mut self, index: usize) -> Result<()> {
        if index >= self.effects.len() {
            return Err(anyhow!("Index {index} out of bounds."));
        }
        if index == self.effects.len() - 1 {
            return Err(anyhow!("Can't move rightmost item farther right."));
        }
        let element = self.effects.remove(index);
        self.insert_effect(index + 1, element)
    }

    fn shift_instrument_left(&mut self, index: usize) -> Result<()> {
        if index >= self.instruments.len() {
            return Err(anyhow!("Index {index} out of bounds."));
        }
        if index == 0 {
            return Err(anyhow!("Can't move leftmost item farther left."));
        }
        let element = self.instruments.remove(index);
        self.insert_instrument(index - 1, element)
    }
    fn shift_instrument_right(&mut self, index: usize) -> Result<()> {
        if index >= self.instruments.len() {
            return Err(anyhow!("Index {index} out of bounds."));
        }
        if index == self.instruments.len() - 1 {
            return Err(anyhow!("Can't move rightmost item farther right."));
        }
        let element = self.instruments.remove(index);
        self.insert_instrument(index + 1, element)
    }

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
    pub fn show_detail(&mut self, ui: &mut Ui, factory: &EntityFactory, _track_index: usize) {
        let style = ui.visuals().widgets.inactive;

        ui.with_layout(
            egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
            |ui| {
                let desired_size = Vec2::new(ui.available_width(), 256.0 - style.fg_stroke.width);
                ui.set_min_size(desired_size);
                ui.set_max_size(desired_size);

                ui.horizontal(|ui| {
                    ui.menu_button("+", |ui| {
                        ui.menu_button("Controllers", |ui| {
                            factory.controller_keys().for_each(|k| {
                                if ui.button(k.to_string()).clicked() {
                                    if let Some(e) = factory.new_controller(k) {
                                        self.append_controller(e);
                                    }
                                    ui.close_menu();
                                }
                            });
                        });
                        ui.menu_button("Instruments", |ui| {
                            factory.instrument_keys().for_each(|k| {
                                if ui.button(k.to_string()).clicked() {
                                    if let Some(e) = factory.new_instrument(k) {
                                        self.append_instrument(e);
                                    }
                                    ui.close_menu();
                                }
                            });
                        });
                        ui.menu_button("Effects", |ui| {
                            factory.effect_keys().for_each(|k| {
                                if ui.button(k.to_string()).clicked() {
                                    if let Some(e) = factory.new_effect(k) {
                                        self.append_effect(e);
                                    }
                                    ui.close_menu();
                                }
                            });
                        });
                    });
                });
                ui.add(egui::Separator::default().grow(8.0));

                ui.horizontal_centered(|ui| {
                    let desired_size = Vec2::new(512.0, ui.available_height());

                    let mut action = None;

                    if let Some(sequencer) = self.sequencer.as_mut() {
                        if let Some(a) = Self::add_track_element(
                            ui,
                            0,
                            EntityType::Controller,
                            false,
                            false,
                            true,
                            |ui| {
                                sequencer.show(ui);
                            },
                        ) {
                            action = Some(a);
                        };
                    }

                    // controller
                    let len = self.controllers.len();
                    for (index, e) in self.controllers.iter_mut().enumerate() {
                        let index = index + 1;
                        ui.allocate_ui(desired_size, |ui| {
                            let (show_left, show_right) = Self::button_states(index, len);
                            if let Some(a) = Self::add_track_element(
                                ui,
                                index,
                                EntityType::Controller,
                                show_left,
                                show_right,
                                true,
                                |ui| {
                                    e.show(ui);
                                },
                            ) {
                                action = Some(a);
                            };
                        });
                    }

                    // instrument
                    for (index, e) in self.instruments.iter_mut().enumerate() {
                        ui.allocate_ui(desired_size, |ui| {
                            // Instrument order in a track doesn't matter, so left/right are always off.
                            if let Some(a) = Self::add_track_element(
                                ui,
                                index,
                                EntityType::Instrument,
                                false,
                                false,
                                true,
                                |ui| {
                                    ui.set_min_size(desired_size);
                                    ui.set_max_size(desired_size);
                                    e.show(ui);
                                },
                            ) {
                                action = Some(a);
                            };
                        });
                    }

                    // effect
                    let len = self.effects.len();
                    for (index, e) in self.effects.iter_mut().enumerate() {
                        ui.allocate_ui(desired_size, |ui| {
                            let (show_left, show_right) = Self::button_states(index, len);
                            if let Some(a) = Self::add_track_element(
                                ui,
                                index,
                                EntityType::Effect,
                                show_left,
                                show_right,
                                true,
                                |ui| {
                                    ui.set_min_size(desired_size);
                                    ui.set_max_size(desired_size);
                                    e.show(ui);
                                },
                            ) {
                                action = Some(a);
                            };
                        });
                    }

                    // check action
                    if let Some(action) = action {
                        match action {
                            TrackElementAction::MoveControllerLeft(index) => {
                                let _ = self.shift_controller_left(index);
                            }
                            TrackElementAction::MoveControllerRight(index) => {
                                let _ = self.shift_controller_right(index);
                            }
                            TrackElementAction::RemoveController(index) => {
                                let _ = self.remove_controller(index);
                            }
                            TrackElementAction::MoveEffectLeft(index) => {
                                let _ = self.shift_effect_left(index);
                            }
                            TrackElementAction::MoveEffectRight(index) => {
                                let _ = self.shift_effect_right(index);
                            }
                            TrackElementAction::RemoveEffect(index) => {
                                let _ = self.remove_effect(index);
                            }
                            TrackElementAction::MoveInstrumentLeft(index) => {
                                let _ = self.shift_instrument_left(index);
                            }
                            TrackElementAction::MoveInstrumentRight(index) => {
                                let _ = self.shift_instrument_right(index);
                            }
                            TrackElementAction::RemoveInstrument(index) => {
                                let _ = self.remove_instrument(index);
                            }
                        }
                    }
                });
            },
        );
    }

    fn add_track_element(
        ui: &mut Ui,
        index: usize,
        entity_type: EntityType,
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
                                action = match entity_type {
                                    EntityType::Controller => {
                                        Some(TrackElementAction::MoveControllerLeft(index))
                                    }
                                    EntityType::Effect => {
                                        Some(TrackElementAction::MoveEffectLeft(index))
                                    }
                                    EntityType::Instrument => {
                                        Some(TrackElementAction::MoveInstrumentLeft(index))
                                    }
                                    EntityType::None => None,
                                };
                            }
                        }
                        if show_right_button {
                            if ui.button(">").clicked() {
                                action = match entity_type {
                                    EntityType::Controller => {
                                        Some(TrackElementAction::MoveControllerRight(index))
                                    }
                                    EntityType::Effect => {
                                        Some(TrackElementAction::MoveEffectRight(index))
                                    }
                                    EntityType::Instrument => {
                                        Some(TrackElementAction::MoveInstrumentRight(index))
                                    }
                                    EntityType::None => None,
                                };
                            }
                        }
                        if show_delete_button {
                            if ui.button("x").clicked() {
                                action = match entity_type {
                                    EntityType::Controller => {
                                        Some(TrackElementAction::RemoveController(index))
                                    }
                                    EntityType::Effect => {
                                        Some(TrackElementAction::RemoveEffect(index))
                                    }
                                    EntityType::Instrument => {
                                        Some(TrackElementAction::RemoveInstrument(index))
                                    }
                                    EntityType::None => None,
                                };
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

    pub fn batch_it_up(&mut self, len: usize) {
        debug_assert_eq!(len, self.buffer.len());

        for e in self.instruments.iter_mut() {
            e.batch_values(&mut self.buffer);
        }
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
}
impl Generates<StereoSample> for Track {
    fn value(&self) -> StereoSample {
        StereoSample::SILENCE
    }

    fn batch_values(&mut self, values: &mut [StereoSample]) {
        for e in self.instruments.iter_mut() {
            e.batch_values(values);
        }
    }
}
impl Ticks for Track {
    fn tick(&mut self, _tick_count: usize) {
        todo!()
    }
}
impl Configurable for Track {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        // I was excited when I read about Iterator's .chain() to condense
        // repetitive code like this, but it's trickier than I expected because
        // they're all different types. I'm using a common trait (Configurable),
        // but I'd need to either #![feature(trait_upcasting)] (and use
        // nightly), or implement as_configurable() methods on each struct,
        // which is totally doable (and I might in fact do it soon, see the
        // "create the uber-trait" TODO elsewhere in this file), but I'm not
        // going to do it right now. TODO
        for e in self.controllers.iter_mut() {
            e.update_sample_rate(sample_rate);
        }
        for e in self.effects.iter_mut() {
            e.update_sample_rate(sample_rate);
        }
        for e in self.instruments.iter_mut() {
            e.update_sample_rate(sample_rate);
        }
    }

    fn update_tempo(&mut self, _tempo: Tempo) {
        todo!()
    }

    fn update_time_signature(&mut self, _time_signature: TimeSignature) {
        todo!()
    }
}
impl HandlesMidi for Track {
    fn handle_midi_message(
        &mut self,
        message: &MidiMessage,
        messages_fn: &mut dyn FnMut(MidiChannel, MidiMessage),
    ) {
        for e in self.controllers.iter_mut() {
            e.handle_midi_message(&message, messages_fn);
        }
        for e in self.instruments.iter_mut() {
            e.handle_midi_message(&message, messages_fn);
        }
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
        let instrument = ToyInstrument::new_with(&ToyInstrumentParams::default());
        let id1 = instrument.uid();
        t.append_instrument(Box::new(instrument));

        // Add a second instrument to the track.
        let instrument = ToyInstrument::new_with(&ToyInstrumentParams::default());
        let id2 = instrument.uid();
        t.append_instrument(Box::new(instrument));

        // Ordering within track is correct, and we can move items around
        // depending on where they are.
        assert_eq!(t.instruments[0].uid(), id1);
        assert_eq!(t.instruments[1].uid(), id2);
        assert!(t.shift_instrument_left(0).is_err()); // Already leftmost.
        assert!(t.shift_instrument_right(1).is_err()); // Already rightmost.
        assert!(t.shift_instrument_left(1).is_ok());
        assert_eq!(t.instruments[0].uid(), id2);
        assert_eq!(t.instruments[1].uid(), id1);

        let instrument = t.remove_instrument(0).unwrap();
        assert_eq!(instrument.uid(), id2);
        assert_eq!(t.instruments.len(), 1);
    }
}
