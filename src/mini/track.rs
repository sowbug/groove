// Copyright (c) 2023 Mike Tsao. All rights reserved.

use super::{
    control_atlas::ControlAtlas,
    control_router::ControlRouter,
    entity_factory::ThingStore,
    humidifier::Humidifier,
    midi_router::MidiRouter,
    piano_roll::PianoRoll,
    sequencer::{Sequencer, SequencerAction},
    widgets::{title_bar, wiggler},
    DragDropManager, DragDropSource, Key,
};
use anyhow::anyhow;
use eframe::{
    egui::{self, Frame, Layout, Margin, Response, Sense, Ui},
    emath::Align,
    epaint::{vec2, Color32, Stroke, Vec2},
};
use groove_core::{
    control::ControlValue,
    midi::MidiChannel,
    time::{MusicalTime, SampleRate, Tempo, TimeSignature},
    traits::{
        gui::Displays, Configurable, ControlEventsFn, Controls, GeneratesToInternalBuffer,
        Serializable, Thing, ThingEvent, Ticks,
    },
    IsUid, Normal, StereoSample, Uid,
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
#[derive(Debug)]
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

/// A collection of instruments, effects, and controllers that combine to
/// produce a single source of audio.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Track {
    uid: TrackUid,
    title: TrackTitle,
    ty: TrackType,

    thing_store: ThingStore,

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
    buffer: TrackBuffer,

    #[serde(skip)]
    is_sequencer_open: bool,

    #[serde(skip)]
    piano_roll: Arc<RwLock<PianoRoll>>,
}
impl Track {
    #[allow(missing_docs)]
    pub fn is_aux(&self) -> bool {
        matches!(self.ty, TrackType::Aux)
    }

    // TODO: for now the only way to add something new to a Track is to append it.
    #[allow(missing_docs)]
    pub fn append_thing(&mut self, thing: Box<dyn Thing>) -> anyhow::Result<Uid> {
        let uid = thing.uid();

        // Some things are hybrids, so they can appear in multiple lists. That's
        // why we don't have if-else here.
        if thing.as_controller().is_some() {
            self.controllers.push(uid);
        }
        if thing.as_effect().is_some() {
            self.effects.push(uid);
        }
        if thing.as_instrument().is_some() {
            self.instruments.push(uid);
        }
        if thing.as_handles_midi().is_some() {
            // TODO: for now, everyone's on channel 0
            self.midi_router.connect(uid, MidiChannel(0));
        }

        self.thing_store.add(thing)
    }

    #[allow(missing_docs)]
    pub fn remove_thing(&mut self, uid: &Uid) -> Option<Box<dyn Thing>> {
        if let Some(thing) = self.thing_store.remove(uid) {
            if thing.as_controller().is_some() {
                self.controllers.retain(|e| e != uid)
            }
            if thing.as_effect().is_some() {
                self.effects.retain(|e| e != uid);
            }
            if thing.as_instrument().is_some() {
                self.instruments.retain(|e| e != uid);
            }
            Some(thing)
        } else {
            None
        }
    }

    /// Returns the [Thing] having the given [Uid], if it exists.
    pub fn thing(&self, uid: &Uid) -> Option<&Box<dyn Thing>> {
        self.thing_store.get(uid)
    }

    /// Returns the mutable [Thing] having the given [Uid], if it exists.
    pub fn thing_mut(&mut self, uid: &Uid) -> Option<&mut Box<dyn Thing>> {
        self.thing_store.get_mut(uid)
    }

    fn button_states(index: usize, len: usize) -> (bool, bool) {
        let left = index != 0;
        let right = len > 1 && index != len - 1;
        (left, right)
    }

    #[deprecated]
    fn show_midi(
        &mut self,
        ui: &mut Ui,
        view_range: &Range<MusicalTime>,
    ) -> (Response, Option<SequencerAction>) {
        //        self.sequencer.ui_arrangement(ui, view_range)
        panic!()
    }

    fn show_audio(&self, ui: &mut Ui, _view_range: &Range<MusicalTime>) -> Response {
        ui.add(wiggler())
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
                                    if let Some(e) = self.thing_store.get_mut(uid) {
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
                                    if let Some(e) = self.thing_store.get_mut(uid) {
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
                                    if let Some(e) = self.thing_store.get_mut(uid) {
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
            });
        action
    }

    #[must_use]
    #[allow(missing_docs)]
    #[deprecated]
    pub fn show(
        &mut self,
        ui: &mut Ui,
        view_range: &Range<MusicalTime>,
    ) -> (Response, Option<TrackAction>) {
        let mut action = None;

        let response = Frame::default()
            .fill(Color32::GRAY)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let mut title = self.title.0.clone();
                    if ui.text_edit_singleline(&mut title).changed() {
                        action = Some(TrackAction::SetTitle(TrackTitle(title)));
                    };

                    // This is the thing that senses a plain click and returns
                    // the Response that tells the caller whether to select this
                    // track.
                    ui.allocate_response(ui.available_size_before_wrap(), Sense::click())
                })
                .inner
            })
            .inner;
        match self.ty {
            TrackType::Midi => {
                self.show_midi(ui, view_range);
            }
            TrackType::Audio => {
                self.show_audio(ui, view_range);
            }
            TrackType::Aux => {
                // For now, the title bar is enough for a aux track, which holds only effects.
            }
        }
        (response, action)
    }

    /// Main entry point for egui rendering. Returns a [Response] and an
    /// optional [TrackAction] for cases where the [Response] can't represent
    /// what happened.
    #[must_use]
    #[allow(missing_docs)]
    pub fn show_2(
        &mut self,
        ui: &mut Ui,
        view_range: &Range<MusicalTime>,
        ui_state: TrackUiState,
        is_selected: bool,
    ) -> (Response, Option<TrackAction>) {
        let mut action = None;

        // The inner_margin() should be half of the Frame stroke width to leave
        // room for it. Thanks vikrinox on the egui Discord.
        let response = Frame::default()
            .inner_margin(Margin::same(0.5))
            .stroke(Stroke {
                width: 1.0,
                color: {
                    if is_selected {
                        Color32::YELLOW
                    } else {
                        Color32::DARK_GRAY
                    }
                },
            })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.set_min_height(Self::track_view_height(self.ty, ui_state));

                    // The `Response` is based on the title bar, so
                    // clicking/dragging on the title bar affects the `Track` as a
                    // whole.
                    let response = ui.add(title_bar(&mut self.title.0));

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
                            ui.set_max_height(Self::arrangement_view_height(ui_state));

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
                                        TrackType::Midi => self.ui_contents_midi(
                                            ui,
                                            view_range,
                                            ui_state,
                                            is_selected,
                                        ),
                                        TrackType::Audio => {
                                            self.ui_contents_audio(ui, ui_state, is_selected)
                                        }
                                        _ => panic!(),
                                    }
                                    self.control_atlas.ui(ui);
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
                                if let Some(track_action) = self.ui_device_view(ui, ui_state) {
                                    action = Some(track_action);
                                }
                            });
                    });
                    response
                })
                .inner
            })
            .inner;
        (response, action)
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
    fn ui_contents_midi(
        &mut self,
        ui: &mut Ui,
        view_range: &Range<MusicalTime>,
        _ui_state: TrackUiState,
        _is_selected: bool,
    ) {
        let (_response, _action) = self.sequencer.ui_arrangement(ui, self.uid, view_range);
    }

    /// Renders an audio [Track]'s arrangement view, which is an overview of some or
    /// all of the track's project timeline.
    fn ui_contents_audio(&mut self, ui: &mut Ui, _ui_state: TrackUiState, _is_selected: bool) {
        ui.add(wiggler());
    }

    #[must_use]
    fn ui_device_view(&mut self, ui: &mut Ui, ui_state: TrackUiState) -> Option<TrackAction> {
        let mut action = None;
        let mut drag_and_drop_action = None;
        let desired_size = vec2(128.0, Self::device_view_height(ui_state));

        ui.horizontal(|ui| {
            if self.is_sequencer_open {
                egui::Window::new("Sequencer")
                    .open(&mut self.is_sequencer_open)
                    .show(ui.ctx(), |ui| {
                        self.sequencer.ui(ui);
                    });
            } else {
                Self::ui_device(ui, &mut self.sequencer, desired_size);
                if ui.button("open").clicked() {
                    self.is_sequencer_open = !self.is_sequencer_open;
                }
            }
            for thing in self.thing_store.iter_mut() {
                Self::ui_device(ui, thing.as_mut(), desired_size);
            }

            let can_accept = if let Some(source) = DragDropManager::source() {
                match source {
                    DragDropSource::NewDevice(_) => true,
                    DragDropSource::Pattern(_) => false,
                    DragDropSource::ControlTrip(_) => false,
                }
            } else {
                false
            };
            let mut r = DragDropManager::drop_target(ui, can_accept, |ui| {
                ui.allocate_ui_with_layout(
                    desired_size,
                    Layout::centered_and_justified(egui::Direction::LeftToRight),
                    |ui| {
                        ui.label(if self.thing_store.is_empty() {
                            "Drag things here"
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

            // if r.response.hovered() {
            //     hovered = true;
            // }
        });

        // //        if hovered {
        // //            eprintln!("hovered {:?}", drag_and_drop_action);
        // if let Some(dd_action) = drag_and_drop_action {
        //     if ui.input(|i| i.pointer.any_released()) {
        //         match dd_action {
        //             DragDropSource::NewDevice(key) => {
        //                 action = Some(TrackAction::NewDevice(self.uid, key));

        //                 // This is important to let the manager know that
        //                 // you've handled the drop.
        //                 dd.reset();
        //             }
        //             DragDropSource::Pattern(_) => eprintln!("I don't think so {:?}", dd_action),
        //         }
        //     }
        // }
        // //    }

        action
    }

    fn ui_device(ui: &mut Ui, thing: &mut dyn Thing, desired_size: Vec2) {
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
                    thing.ui(ui);
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
            .route(&mut self.thing_store, channel, message)
        {
            eprintln!("While routing: {e}");
        }
    }

    #[allow(missing_docs)]
    pub fn route_control_change(&mut self, uid: Uid, value: ControlValue) {
        if let Err(e) = self.control_router.route(
            &mut |target_uid, index, value| {
                if let Some(e) = self.thing_store.get_mut(target_uid) {
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
        self.piano_roll = Arc::clone(&piano_roll);
        self.sequencer.set_piano_roll(piano_roll);
    }

    #[allow(missing_docs)]
    pub fn sequencer_mut(&mut self) -> &mut Sequencer {
        &mut self.sequencer
    }

    /// Sets the wet/dry of an effect in the chain.
    pub fn set_humidity(&mut self, effect_uid: Uid, humidity: Normal) -> anyhow::Result<()> {
        if let Some(thing) = self.thing(&effect_uid) {
            if thing.as_effect().is_some() {
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
        self.thing_store.calculate_max_entity_uid()
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
        &self.buffer
    }

    /// Returns a writable version of the internal buffer.
    pub fn buffer_mut(&mut self) -> &mut TrackBuffer {
        &mut self.buffer
    }

    /// Returns the [ControlAtlas].
    pub fn control_atlas_mut(&mut self) -> &mut ControlAtlas {
        &mut self.control_atlas
    }
}
impl GeneratesToInternalBuffer<StereoSample> for Track {
    fn generate_batch_values(&mut self, len: usize) -> usize {
        if len > self.buffer.0.len() {
            eprintln!(
                "requested {} samples but buffer is only len {}",
                len,
                self.buffer.0.len()
            );
            return 0;
        }

        if !self.is_aux() {
            // We're a regular track. Start with a fresh buffer and let each
            // instrument do its thing.
            self.buffer.0.fill(StereoSample::SILENCE);
        } else {
            // We're an aux track. We leave the internal buffer as-is, with the
            // expectation that the caller has already filled it with the signal
            // we should be processing.
        }

        for uid in self.instruments.iter() {
            if let Some(e) = self.thing_store.get_mut(uid) {
                if let Some(e) = e.as_instrument_mut() {
                    // Note that we're expecting everyone to ADD to the buffer,
                    // not to overwrite! TODO: convert all instruments to have
                    // internal buffers
                    e.generate_batch_values(&mut self.buffer.0);
                }
            }
        }

        // TODO: change this trait to operate on batches.
        for uid in self.effects.iter() {
            if let Some(e) = self.thing_store.get_mut(uid) {
                if let Some(e) = e.as_effect_mut() {
                    let humidity = self.humidifier.get_humidity_by_uid(uid);
                    if humidity == Normal::zero() {
                        continue;
                    }
                    for sample in self.buffer.0.iter_mut() {
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

        self.buffer.0.len()
    }

    fn values(&self) -> &[StereoSample] {
        &self.buffer.0
    }
}
impl Ticks for Track {
    fn tick(&mut self, tick_count: usize) {
        self.thing_store.tick(tick_count);
    }
}
impl Configurable for Track {
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sequencer.update_sample_rate(sample_rate);

        self.control_atlas.update_sample_rate(sample_rate);
        self.thing_store.update_sample_rate(sample_rate);
    }

    fn update_tempo(&mut self, tempo: Tempo) {
        self.sequencer.update_tempo(tempo);
        self.control_atlas.update_tempo(tempo);
        self.thing_store.update_tempo(tempo);
    }

    fn update_time_signature(&mut self, time_signature: TimeSignature) {
        self.sequencer.update_time_signature(time_signature);
        self.control_atlas.update_time_signature(time_signature);
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
    fn update_time(&mut self, range: &std::ops::Range<groove_core::time::MusicalTime>) {
        self.sequencer.update_time(range);
        self.control_atlas.update_time(range);
        self.thing_store.update_time(range);
    }

    fn work(&mut self, control_events_fn: &mut ControlEventsFn) {
        self.sequencer.work(&mut |uid, event| match event {
            // #145: we don't want MIDI messages to escape this track. This is a
            // temporary fix because we do want Orchestrator to route to
            // external MIDI devices, so eventually we will need to pass them
            // along.
            ThingEvent::Midi(channel, message) => {
                let _ = self
                    .midi_router
                    .route(&mut self.thing_store, channel, message);
            }
            ThingEvent::Control(_) => {
                control_events_fn(uid, event);
            }
            ThingEvent::HandleControl(_, _) => todo!(),
        });
        self.control_atlas.work(control_events_fn);
        self.thing_store.work(control_events_fn);
    }

    fn is_finished(&self) -> bool {
        self.sequencer.is_finished()
            && self.control_atlas.is_finished()
            && self.thing_store.is_finished()
    }

    fn play(&mut self) {
        self.sequencer.play();
        self.thing_store.play();
    }

    fn stop(&mut self) {
        self.sequencer.stop();
        self.thing_store.stop();
    }

    fn skip_to_start(&mut self) {
        self.sequencer.skip_to_start();
        self.thing_store.skip_to_start();
    }

    fn is_performing(&self) -> bool {
        self.sequencer.is_performing() || self.thing_store.is_performing()
    }
}
impl Serializable for Track {
    fn after_deser(&mut self) {
        self.sequencer.after_deser();
        self.thing_store.after_deser();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use groove_core::traits::HasUid;
    use groove_toys::{ToyEffect, ToyInstrument, ToyInstrumentParams};

    #[test]
    fn basic_track_operations() {
        let mut t = Track::default();
        assert!(t.controllers.is_empty());
        assert!(t.effects.is_empty());
        assert!(t.instruments.is_empty());

        // Create an instrument and add it to a track.
        let mut instrument = ToyInstrument::new_with(&ToyInstrumentParams::default());
        instrument.set_uid(Uid(1));
        let id1 = t.append_thing(Box::new(instrument)).unwrap();

        // Add a second instrument to the track.
        let mut instrument = ToyInstrument::new_with(&ToyInstrumentParams::default());
        instrument.set_uid(Uid(2));
        let id2 = t.append_thing(Box::new(instrument)).unwrap();

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

        let mut effect = ToyEffect::default();
        effect.set_uid(Uid(3));
        let effect_id1 = t.append_thing(Box::new(effect)).unwrap();
        let mut effect = ToyEffect::default();
        effect.set_uid(Uid(4));
        let effect_id2 = t.append_thing(Box::new(effect)).unwrap();

        assert_eq!(t.effects[0], effect_id1);
        assert_eq!(t.effects[1], effect_id2);
        assert!(t.move_effect(effect_id1, 1).is_ok());
        assert_eq!(
            t.effects[0], effect_id2,
            "After moving effects, id2 should be first"
        );
        assert_eq!(t.effects[1], effect_id1);
    }
}
