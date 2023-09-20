// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{
    control_atlas::ControlAtlas,
    control_router::ControlRouter,
    entity_factory::EntityStore,
    humidifier::Humidifier,
    midi_router::MidiRouter,
    piano_roll::PianoRoll,
    sequencer::Sequencer,
    widgets::{control, placeholder, track},
    DragDropManager, DragDropSource, Key,
};
use anyhow::anyhow;
use eframe::{
    egui::{self, Frame, Layout, Margin, Ui},
    emath::Align,
    epaint::{vec2, Color32, Stroke, Vec2},
};
use ensnare::core::{Normal, StereoSample};
use groove_core::{
    control::ControlValue,
    midi::MidiChannel,
    time::{MusicalTime, SampleRate, Tempo, TimeSignature},
    traits::{
        gui::{Displays, DisplaysInTimeline},
        Configurable, ControlEventsFn, Controls, Entity, EntityEvent, GeneratesToInternalBuffer,
        Serializable, Ticks,
    },
    IsUid, Uid,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    ops::Range,
    option::Option,
    sync::{Arc, RwLock},
};

/// Identifies a [Track].
#[derive(Copy, Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TrackUid(pub usize);
impl Default for TrackUid {
    fn default() -> Self {
        Self(1)
    }
}
impl IsUid for TrackUid {
    fn increment(&mut self) -> &Self {
        self.0 += 1;
        self
    }
}
impl Display for TrackUid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

#[derive(Debug)]
pub enum TrackElementAction {
    MoveDeviceLeft(usize),
    MoveDeviceRight(usize),
    RemoveDevice(usize),
}

#[derive(Debug)]
pub enum TrackDetailAction {}

#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub enum TrackAction {
    SetTitle(TrackTitle),
    ToggleDisclosure,
    NewDevice(TrackUid, Key),
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub enum TrackType {
    #[default]
    Midi,
    Audio,
    Aux,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct TrackFactory {
    next_uid: TrackUid,
}
impl TrackFactory {
    fn next_uid(&mut self) -> TrackUid {
        let uid = self.next_uid;
        self.next_uid.increment();
        uid
    }

    pub fn midi(&mut self, piano_roll: &Arc<RwLock<PianoRoll>>) -> Track {
        let uid = self.next_uid();
        let title = TrackTitle(format!("MIDI {}", uid));

        let mut t = Track {
            uid,
            title,
            ty: TrackType::Midi,
            ..Default::default()
        };
        t.sequencer_mut().set_piano_roll(Arc::clone(piano_roll));

        t
    }

    pub fn audio(&mut self) -> Track {
        let uid = self.next_uid();
        let title = TrackTitle(format!("Audio {}", uid));
        Track {
            uid,
            title,
            ty: TrackType::Audio,
            ..Default::default()
        }
    }

    pub fn aux(&mut self) -> Track {
        let uid = self.next_uid();
        let title = TrackTitle(format!("Aux {}", uid));
        Track {
            uid,
            title,
            ty: TrackType::Aux,
            ..Default::default()
        }
    }
}
#[derive(Debug)]
pub struct TrackBuffer(pub [StereoSample; Self::LEN]);
impl TrackBuffer {
    pub const LEN: usize = 64;
}
impl Default for TrackBuffer {
    fn default() -> Self {
        Self([StereoSample::default(); Self::LEN])
    }
}

/// Newtype for track title string.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrackTitle(pub String);
impl Default for TrackTitle {
    fn default() -> Self {
        Self("Untitled".to_string())
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy)]
pub enum TrackUiState {
    #[default]
    Collapsed,
    Expanded,
}

#[derive(Debug, Default)]
pub struct TrackEphemerals {
    buffer: TrackBuffer,
    is_sequencer_open: bool,
    piano_roll: Arc<RwLock<PianoRoll>>,
    action: Option<TrackAction>,
    view_range: Range<MusicalTime>,
    is_selected: bool,
    ui_state: TrackUiState,
}

/// A collection of instruments, effects, and controllers that combine to
/// produce a single source of audio.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Track {
    uid: TrackUid,
    title: TrackTitle,
    ty: TrackType,

    entity_store: EntityStore,

    sequencer: Sequencer,
    midi_router: MidiRouter,

    /// [ControlAtlas] manages the sources of Control events. It generates
    /// events but does not handle their routing.
    control_atlas: ControlAtlas,
    /// [ControlRouter] manages the destinations of Control events. It does not
    /// generate events, but when events are generated, it knows where to route
    /// them.
    control_router: ControlRouter,

    controllers: Vec<Uid>,
    instruments: Vec<Uid>,
    effects: Vec<Uid>,

    humidifier: Humidifier,

    #[serde(skip)]
    e: TrackEphemerals,
}
impl Track {
    #[allow(missing_docs)]
    pub fn is_aux(&self) -> bool {
        matches!(self.ty, TrackType::Aux)
    }

    // TODO: for now the only way to add something new to a Track is to append it.
    #[allow(missing_docs)]
    pub fn append_entity(&mut self, entity: Box<dyn Entity>) -> anyhow::Result<Uid> {
        let uid = entity.uid();

        // Some entities are hybrids, so they can appear in multiple lists.
        // That's why we don't have if-else here.
        if entity.as_controller().is_some() {
            self.controllers.push(uid);
        }
        if entity.as_effect().is_some() {
            self.effects.push(uid);
        }
        if entity.as_instrument().is_some() {
            self.instruments.push(uid);
        }
        if entity.as_handles_midi().is_some() {
            // TODO: for now, everyone's on channel 0
            self.midi_router.connect(uid, MidiChannel(0));
        }

        self.entity_store.add(entity)
    }

    #[allow(missing_docs)]
    pub fn remove_entity(&mut self, uid: &Uid) -> Option<Box<dyn Entity>> {
        if let Some(entity) = self.entity_store.remove(uid) {
            if entity.as_controller().is_some() {
                self.controllers.retain(|e| e != uid)
            }
            if entity.as_effect().is_some() {
                self.effects.retain(|e| e != uid);
            }
            if entity.as_instrument().is_some() {
                self.instruments.retain(|e| e != uid);
            }
            Some(entity)
        } else {
            None
        }
    }

    /// Returns the [Entity] having the given [Uid], if it exists.
    pub fn entity(&self, uid: &Uid) -> Option<&Box<dyn Entity>> {
        self.entity_store.get(uid)
    }

    /// Returns the mutable [Entity] having the given [Uid], if it exists.
    pub fn entity_mut(&mut self, uid: &Uid) -> Option<&mut Box<dyn Entity>> {
        self.entity_store.get_mut(uid)
    }

    fn button_states(index: usize, len: usize) -> (bool, bool) {
        let left = index != 0;
        let right = len > 1 && index != len - 1;
        (left, right)
    }

    /// Shows the detail view for the selected track.
    // TODO: ordering should be controllers, instruments, then effects. Within
    // those groups, the user can reorder as desired (but instrument order
    // doesn't matter because they're all simultaneous)
    #[must_use]
    pub fn ui_detail(&mut self, ui: &mut Ui) -> Option<TrackDetailAction> {
        let style = ui.visuals().widgets.inactive;
        let action = None;

        ui.with_layout(
            egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
            |ui| {
                let desired_size = Vec2::new(ui.available_width(), 256.0 - style.fg_stroke.width);
                ui.set_min_size(desired_size);
                ui.set_max_size(desired_size);

                ui.horizontal_centered(|ui| {
                    let desired_size = Vec2::new(384.0, ui.available_height());

                    let mut action = None;

                    if let Some(a) = Self::add_track_element(ui, 0, false, false, false, |ui| {
                        ui.allocate_ui(vec2(256.0, ui.available_height()), |ui| {
                            self.sequencer.ui(ui);
                        });
                    }) {
                        action = Some(a);
                    };

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
                                    if let Some(e) = self.entity_store.get_mut(uid) {
                                        e.ui(ui);
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
                                    if let Some(e) = self.entity_store.get_mut(uid) {
                                        e.ui(ui);
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
                                    if let Some(e) = self.entity_store.get_mut(uid) {
                                        e.ui(ui);
                                    }
                                },
                            ) {
                                action = Some(a);
                            };
                        });
                    }
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
                    ui.allocate_ui(vec2(384.0, ui.available_height()), |ui| {
                        ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                            if show_left_button && ui.button("<").clicked() {
                                action = Some(TrackElementAction::MoveDeviceLeft(index));
                            }
                            if show_right_button && ui.button(">").clicked() {
                                action = Some(TrackElementAction::MoveDeviceRight(index));
                            }
                            if show_delete_button && ui.button("x").clicked() {
                                action = Some(TrackElementAction::RemoveDevice(index));
                            }
                        });
                        ui.vertical(|ui| {
                            add_contents(ui);
                        });
                    });
                });
            });
        action
    }

    pub(crate) fn track_view_height(track_type: TrackType, ui_state: TrackUiState) -> f32 {
        if matches!(track_type, TrackType::Aux) {
            Self::device_view_height(ui_state)
        } else {
            Self::arrangement_view_height(ui_state) + Self::device_view_height(ui_state)
        }
    }

    const fn arrangement_view_height(_ui_state: TrackUiState) -> f32 {
        64.0
    }

    const fn device_view_height(ui_state: TrackUiState) -> f32 {
        match ui_state {
            TrackUiState::Collapsed => 32.0,
            TrackUiState::Expanded => 96.0,
        }
    }

    /// Renders a MIDI [Track]'s arrangement view, which is an overview of some or
    /// all of the track's project timeline.
    fn ui_contents_midi(&mut self, ui: &mut Ui) {
        let (_response, _action) = self.sequencer.ui_arrangement(ui, self.uid);
    }

    /// Renders an audio [Track]'s arrangement view, which is an overview of some or
    /// all of the track's project timeline.
    fn ui_contents_audio(&mut self, ui: &mut Ui) {
        ui.add(placeholder::wiggler());
    }

    #[must_use]
    fn ui_device_view(&mut self, ui: &mut Ui) -> Option<TrackAction> {
        let mut action = None;
        let mut drag_and_drop_action = None;
        let desired_size = vec2(128.0, Self::device_view_height(self.e.ui_state));

        ui.horizontal(|ui| {
            if self.e.is_sequencer_open {
                egui::Window::new("Sequencer")
                    .open(&mut self.e.is_sequencer_open)
                    .show(ui.ctx(), |ui| {
                        self.sequencer.ui(ui);
                    });
            } else {
                Self::ui_device(ui, &mut self.sequencer, desired_size);
                if ui.button("open").clicked() {
                    self.e.is_sequencer_open = !self.e.is_sequencer_open;
                }
            }
            self.entity_store.iter_mut().for_each(|e| {
                Self::ui_device(ui, e.as_mut(), desired_size);
            });

            let can_accept = if let Some(source) = DragDropManager::source() {
                match source {
                    DragDropSource::NewDevice(_) => true,
                    DragDropSource::Pattern(_) => false,
                    DragDropSource::ControlTrip(_) => false,
                }
            } else {
                false
            };
            let r = DragDropManager::drop_target(ui, can_accept, |ui| {
                ui.allocate_ui_with_layout(
                    desired_size,
                    Layout::centered_and_justified(egui::Direction::LeftToRight),
                    |ui| {
                        ui.label(if self.entity_store.is_empty() {
                            "Drag stuff here"
                        } else {
                            "+"
                        })
                    },
                );
            });

            // super::drag_drop::DragDropTarget::Track(self.uid),

            if DragDropManager::is_dropped(ui, &r.response) {
                if let Some(source) = DragDropManager::source() {
                    match source {
                        DragDropSource::NewDevice(key) => {
                            drag_and_drop_action = Some(DragDropSource::NewDevice(key.clone()));
                            action = Some(TrackAction::NewDevice(self.uid, key.clone()));
                            DragDropManager::reset();
                        }
                        DragDropSource::Pattern(_) => eprintln!(
                            "nope - I'm a device drop target, not a pattern target {:?}",
                            source
                        ),
                        DragDropSource::ControlTrip(_) => {
                            eprintln!("NOPE!")
                        }
                    }
                }
            }
        });

        action
    }

    fn ui_device(ui: &mut Ui, entity: &mut dyn Entity, desired_size: Vec2) {
        ui.allocate_ui(desired_size, |ui| {
            ui.set_min_size(desired_size);
            ui.set_max_size(desired_size);
            Frame::default()
                .stroke(Stroke {
                    width: 0.5,
                    color: Color32::DARK_GRAY,
                })
                .inner_margin(2.0)
                .show(ui, |ui| {
                    entity.ui(ui);
                });
        });
    }

    #[allow(missing_docs)]
    pub fn remove_selected_patterns(&mut self) {
        self.sequencer.remove_selected_arranged_patterns();
    }

    #[allow(missing_docs)]
    pub fn route_midi_message(
        &mut self,
        channel: MidiChannel,
        message: groove_core::midi::MidiMessage,
    ) {
        if let Err(e) = self
            .midi_router
            .route(&mut self.entity_store, channel, message)
        {
            eprintln!("While routing: {e}");
        }
    }

    #[allow(missing_docs)]
    pub fn route_control_change(&mut self, uid: Uid, value: ControlValue) {
        if let Err(e) = self.control_router.route(
            &mut |target_uid, index, value| {
                if let Some(e) = self.entity_store.get_mut(target_uid) {
                    if let Some(e) = e.as_controllable_mut() {
                        e.control_set_param_by_index(index, value);
                    }
                }
            },
            uid,
            value,
        ) {
            eprintln!("While routing control change: {e}")
        }
    }

    pub(crate) fn set_title(&mut self, title: TrackTitle) {
        self.title = title;
    }

    #[allow(missing_docs)]
    pub fn uid(&self) -> TrackUid {
        self.uid
    }

    pub(crate) fn ty(&self) -> TrackType {
        self.ty
    }

    #[allow(missing_docs)]
    pub fn set_piano_roll(&mut self, piano_roll: Arc<RwLock<PianoRoll>>) {
        self.e.piano_roll = Arc::clone(&piano_roll);
        self.sequencer.set_piano_roll(piano_roll);
    }

    #[allow(missing_docs)]
    pub fn sequencer_mut(&mut self) -> &mut Sequencer {
        &mut self.sequencer
    }

    /// Sets the wet/dry of an effect in the chain.
    pub fn set_humidity(&mut self, effect_uid: Uid, humidity: Normal) -> anyhow::Result<()> {
        if let Some(entity) = self.entity(&effect_uid) {
            if entity.as_effect().is_some() {
                self.humidifier.set_humidity_by_uid(effect_uid, humidity);
                Ok(())
            } else {
                Err(anyhow!("{effect_uid} is not an effect"))
            }
        } else {
            Err(anyhow!("{effect_uid} not found"))
        }
    }

    pub(crate) fn calculate_max_entity_uid(&self) -> Option<Uid> {
        self.entity_store.calculate_max_entity_uid()
    }

    /// Moves the indicated effect to a new position within the effects chain.
    /// Zero is the first position.
    pub fn move_effect(&mut self, uid: Uid, new_index: usize) -> anyhow::Result<()> {
        if new_index >= self.effects.len() {
            Err(anyhow!(
                "Can't move {uid} to {new_index} when we have only {} items",
                self.effects.len()
            ))
        } else if self.effects.contains(&uid) {
            self.effects.retain(|e| e != &uid);
            self.effects.insert(new_index, uid);
            Ok(())
        } else {
            Err(anyhow!("Effect {uid} not found"))
        }
    }

    /// Returns the [ControlRouter].
    pub fn control_router_mut(&mut self) -> &mut ControlRouter {
        &mut self.control_router
    }

    /// Returns an immutable reference to the internal buffer.
    pub fn buffer(&self) -> &TrackBuffer {
        &self.e.buffer
    }

    /// Returns a writable version of the internal buffer.
    pub fn buffer_mut(&mut self) -> &mut TrackBuffer {
        &mut self.e.buffer
    }

    /// Returns the [ControlAtlas].
    pub fn control_atlas_mut(&mut self) -> &mut ControlAtlas {
        &mut self.control_atlas
    }

    #[allow(missing_docs)]
    pub fn action(&self) -> Option<TrackAction> {
        self.e.action.clone()
    }

    #[allow(missing_docs)]
    pub fn set_is_selected(&mut self, selected: bool) {
        self.e.is_selected = selected;
    }

    #[allow(missing_docs)]
    pub fn set_ui_state(&mut self, ui_state: TrackUiState) {
        self.e.ui_state = ui_state;
    }
}
impl GeneratesToInternalBuffer<StereoSample> for Track {
    fn generate_batch_values(&mut self, len: usize) -> usize {
        if len > self.e.buffer.0.len() {
            eprintln!(
                "requested {} samples but buffer is only len {}",
                len,
                self.e.buffer.0.len()
            );
            return 0;
        }

        if !self.is_aux() {
            // We're a regular track. Start with a fresh buffer and let each
            // instrument do its thing.
            self.e.buffer.0.fill(StereoSample::SILENCE);
        } else {
            // We're an aux track. We leave the internal buffer as-is, with the
            // expectation that the caller has already filled it with the signal
            // we should be processing.
        }

        for uid in self.instruments.iter() {
            if let Some(e) = self.entity_store.get_mut(uid) {
                if let Some(e) = e.as_instrument_mut() {
                    // Note that we're expecting everyone to ADD to the buffer,
                    // not to overwrite! TODO: convert all instruments to have
                    // internal buffers
                    e.generate_batch_values(&mut self.e.buffer.0);
                }
            }
        }

        // TODO: change this trait to operate on batches.
        for uid in self.effects.iter() {
            if let Some(e) = self.entity_store.get_mut(uid) {
                if let Some(e) = e.as_effect_mut() {
                    let humidity = self.humidifier.get_humidity_by_uid(uid);
                    if humidity == Normal::zero() {
                        continue;
                    }
                    for sample in self.e.buffer.0.iter_mut() {
                        *sample = self.humidifier.transform_audio(
                            humidity,
                            *sample,
                            e.transform_audio(*sample),
                        );
                    }
                }
            }
        }

        // See #146 TODO - at this point we might want to gather any events
        // produced during the effects stage.

        self.e.buffer.0.len()
    }

    fn values(&self) -> &[StereoSample] {
        &self.e.buffer.0
    }
}
impl Ticks for Track {
    fn tick(&mut self, tick_count: usize) {
        self.entity_store.tick(tick_count);
    }
}
impl Configurable for Track {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sequencer.update_sample_rate(sample_rate);
        self.control_atlas.update_sample_rate(sample_rate);
        self.entity_store.update_sample_rate(sample_rate);
    }

    fn update_tempo(&mut self, tempo: Tempo) {
        self.sequencer.update_tempo(tempo);
        self.control_atlas.update_tempo(tempo);
        self.entity_store.update_tempo(tempo);
    }

    fn update_time_signature(&mut self, time_signature: TimeSignature) {
        self.sequencer.update_time_signature(time_signature);
        self.control_atlas.update_time_signature(time_signature);
        self.entity_store.update_time_signature(time_signature);
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
    fn update_time(&mut self, range: &Range<MusicalTime>) {
        self.sequencer.update_time(range);
        self.control_atlas.update_time(range);
        self.entity_store.update_time(range);
    }

    fn work(&mut self, control_events_fn: &mut ControlEventsFn) {
        // Create a place to hold MIDI messages that we need to route.
        let mut midi_events = Vec::default();

        // Peek at incoming events before handing them to control_events_fn.
        let mut handler = |uid, event| {
            match event {
                // We need to route MIDI messages to all eligible Entities in
                // this Track, so we save them up.
                EntityEvent::Midi(channel, message) => {
                    midi_events.push((channel, message));
                }
                EntityEvent::Control(_) => {}
            }
            control_events_fn(uid, event);
        };

        // Let everyone work and possibly generate messages.
        self.sequencer.work(&mut handler);
        self.control_atlas.work(&mut handler);
        self.entity_store.work(&mut handler);

        // We've accumulated all the MIDI messages. Route them to our own
        // MidiRouter. They've already been forwarded to the caller via
        // control_events_fn.
        midi_events.into_iter().for_each(|(channel, message)| {
            let _ = self
                .midi_router
                .route(&mut self.entity_store, channel, message);
        });
    }

    fn is_finished(&self) -> bool {
        self.sequencer.is_finished()
            && self.control_atlas.is_finished()
            && self.entity_store.is_finished()
    }

    fn play(&mut self) {
        self.sequencer.play();
        self.entity_store.play();
    }

    fn stop(&mut self) {
        self.sequencer.stop();
        self.entity_store.stop();
    }

    fn skip_to_start(&mut self) {
        self.sequencer.skip_to_start();
        self.entity_store.skip_to_start();
    }

    fn is_performing(&self) -> bool {
        self.sequencer.is_performing() || self.entity_store.is_performing()
    }
}
impl Serializable for Track {
    fn after_deser(&mut self) {
        self.sequencer.after_deser();
        self.entity_store.after_deser();
    }
}
impl DisplaysInTimeline for Track {
    fn set_view_range(&mut self, view_range: &Range<MusicalTime>) {
        self.sequencer.set_view_range(view_range);
        self.e.view_range = view_range.clone();
    }
}
impl Displays for Track {
    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        self.e.action = None;

        // The inner_margin() should be half of the Frame stroke width to leave
        // room for it. Thanks vikrinox on the egui Discord.
        Frame::default()
            .inner_margin(Margin::same(0.5))
            .stroke(Stroke {
                width: 1.0,
                color: {
                    if self.e.is_selected {
                        Color32::YELLOW
                    } else {
                        Color32::DARK_GRAY
                    }
                },
            })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.set_min_height(Self::track_view_height(self.ty, self.e.ui_state));

                    // The `Response` is based on the title bar, so
                    // clicking/dragging on the title bar affects the `Track` as a
                    // whole.
                    let response = ui.add(track::title_bar(&mut self.title.0));

                    // Take up all the space we're given, even if we can't fill
                    // it with widget content.
                    ui.set_min_size(ui.available_size());

                    // The frames shouldn't have space between them.
                    ui.style_mut().spacing.item_spacing = Vec2::ZERO;

                    // Build the track content with the device view beneath it.
                    ui.vertical(|ui| {
                        // Only MIDI/audio tracks have content.
                        if !matches!(self.ty, TrackType::Aux) {
                            // Reserve space for the device view.
                            ui.set_max_height(Self::arrangement_view_height(self.e.ui_state));

                            // Draw the arrangement view.
                            Frame::default()
                                .inner_margin(Margin::same(0.5))
                                .outer_margin(Margin::same(0.5))
                                .stroke(Stroke {
                                    width: 1.0,
                                    color: Color32::DARK_GRAY,
                                })
                                .show(ui, |ui| {
                                    ui.set_min_size(ui.available_size());
                                    match self.ty {
                                        TrackType::Midi => self.ui_contents_midi(ui),
                                        TrackType::Audio => self.ui_contents_audio(ui),
                                        _ => panic!(),
                                    }
                                    ui.add(control::atlas(
                                        &mut self.control_atlas,
                                        &mut self.control_router,
                                        self.e.view_range.clone(),
                                    ));
                                });
                        }

                        // Now the device view.
                        Frame::default()
                            .inner_margin(Margin::same(0.5))
                            .outer_margin(Margin::same(0.5))
                            .stroke(Stroke {
                                width: 1.0,
                                color: Color32::DARK_GRAY,
                            })
                            .show(ui, |ui| {
                                if let Some(track_action) = self.ui_device_view(ui) {
                                    self.e.action = Some(track_action);
                                }
                            });
                    });
                    response
                })
                .inner
            })
            .inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use groove_core::traits::HasUid;
    use groove_toys::{
        ToyControllerAlwaysSendsMidiMessage, ToyEffect, ToyInstrument, ToyInstrumentParams,
    };

    #[test]
    fn basic_track_operations() {
        let mut t = Track::default();
        assert!(t.controllers.is_empty());
        assert!(t.effects.is_empty());
        assert!(t.instruments.is_empty());

        // Create an instrument and add it to a track.
        let mut instrument = ToyInstrument::new_with(&ToyInstrumentParams::default());
        instrument.set_uid(Uid(1));
        let id1 = t.append_entity(Box::new(instrument)).unwrap();

        // Add a second instrument to the track.
        let mut instrument = ToyInstrument::new_with(&ToyInstrumentParams::default());
        instrument.set_uid(Uid(2));
        let id2 = t.append_entity(Box::new(instrument)).unwrap();

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

        let instrument = t.remove_entity(&id1).unwrap();
        assert_eq!(instrument.uid(), id1, "removed the right instrument");
        assert_eq!(t.instruments.len(), 1, "removed exactly one instrument");
        assert_eq!(
            t.instruments[0], id2,
            "the remaining instrument should be the one we left"
        );
        assert!(
            t.entity_store.get(&id1).is_none(),
            "it should be gone from the store"
        );

        let mut effect = ToyEffect::default();
        effect.set_uid(Uid(3));
        let effect_id1 = t.append_entity(Box::new(effect)).unwrap();
        let mut effect = ToyEffect::default();
        effect.set_uid(Uid(4));
        let effect_id2 = t.append_entity(Box::new(effect)).unwrap();

        assert_eq!(t.effects[0], effect_id1);
        assert_eq!(t.effects[1], effect_id2);
        assert!(t.move_effect(effect_id1, 1).is_ok());
        assert_eq!(
            t.effects[0], effect_id2,
            "After moving effects, id2 should be first"
        );
        assert_eq!(t.effects[1], effect_id1);
    }

    // We expect that a MIDI message will be routed to the eligible Entities in
    // the same Track, and forwarded to the work() caller, presumably to decide
    // whether to send it to other destination(s) such as external MIDI
    // interfaces.
    #[test]
    fn midi_messages_sent_to_caller_and_sending_track_instruments() {
        let mut t = Track::default();

        let mut sender = ToyControllerAlwaysSendsMidiMessage::default();
        sender.set_uid(Uid(2001));
        let _sender_id = t.append_entity(Box::new(sender)).unwrap();

        let mut receiver = ToyInstrument::new_with(&ToyInstrumentParams::default());
        receiver.set_uid(Uid(2002));
        let counter = Arc::clone(receiver.received_count_mutex());
        let _receiver_id = t.append_entity(Box::new(receiver)).unwrap();

        let mut external_midi_messages = 0;
        t.play();
        t.work(&mut |_uid, _event| {
            external_midi_messages += 1;
        });

        if let Ok(c) = counter.lock() {
            assert_eq!(
                *c, 1,
                "The receiving instrument in the track should have received the message"
            );
        };

        assert_eq!(
            external_midi_messages, 1,
            "After one work(), one MIDI message should have emerged for external processing"
        );
    }
}
