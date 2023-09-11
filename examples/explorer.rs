// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! The [Explorer] example is a sandbox for developing egui components and
//! widgets.

use anyhow::anyhow;
use eframe::{
    egui::{
        self, warn_if_debug_build, CollapsingHeader, Id, Layout, ScrollArea, Slider, Style, Ui,
    },
    emath::Align,
    epaint::vec2,
    CreationContext,
};
use groove::{
    app_version,
    mini::{
        register_factory_entities,
        widgets::{pattern, placeholder, timeline, track},
        ControlAtlas, DragDropEvent, DragDropManager, DragDropSource, ESSequencer,
        ESSequencerBuilder, Note, PatternUid, PianoRoll, Sequencer, ThingStore, TrackTitle,
        TrackUid,
    },
    EntityFactory,
};
use groove_core::{
    midi::MidiNote,
    time::MusicalTime,
    traits::{
        gui::{Displays, DisplaysInTimeline},
        Thing,
    },
    Uid,
};
use std::ops::Range;

#[derive(Debug)]
struct LegendSettings {
    hide: bool,
    range: Range<MusicalTime>,
}
impl LegendSettings {
    const NAME: &str = "Legend";

    fn show(&mut self, ui: &mut Ui) {
        if !self.hide {
            ui.add(timeline::legend(&mut self.range));
        }
    }
}
impl Default for LegendSettings {
    fn default() -> Self {
        Self {
            hide: Default::default(),
            range: MusicalTime::START..MusicalTime::new_with_beats(128),
        }
    }
}
impl Displays for LegendSettings {
    fn ui(&mut self, ui: &mut Ui) -> egui::Response {
        ui.checkbox(&mut self.hide, "Hide");
        ui.label("View range");
        let mut range_start = self.range.start.total_beats();
        let mut range_end = self.range.end.total_beats();
        let start_response = ui.add(Slider::new(&mut range_start, 0..=128));
        if start_response.changed() {
            self.range.start = MusicalTime::new_with_beats(range_start);
        };
        let end_response = ui.add(Slider::new(&mut range_end, 1..=256));
        if end_response.changed() {
            self.range.end = MusicalTime::new_with_beats(range_end);
        };
        start_response | end_response
    }
}

#[derive(Debug)]
struct TimelineSettings {
    hide: bool,
    track_uid: TrackUid,
    range: Range<MusicalTime>,
    view_range: Range<MusicalTime>,
    control_atlas: ControlAtlas,
    sequencer: ESSequencer,
    focused: timeline::FocusedComponent,
}
impl DisplaysInTimeline for TimelineSettings {
    fn set_view_range(&mut self, view_range: &std::ops::Range<groove_core::time::MusicalTime>) {
        self.view_range = view_range.clone();
    }
}
impl TimelineSettings {
    const NAME: &str = "Timeline";

    fn show(&mut self, ui: &mut Ui) {
        if !self.hide {
            ui.add(timeline::timeline(
                self.track_uid,
                &mut self.sequencer,
                &mut self.control_atlas,
                self.range.clone(),
                self.view_range.clone(),
                self.focused,
            ));
        }
    }
}
impl Default for TimelineSettings {
    fn default() -> Self {
        let sequencer = ESSequencerBuilder::default()
            .random(MusicalTime::START..MusicalTime::new_with_beats(128))
            .build()
            .unwrap();
        Self {
            hide: Default::default(),
            track_uid: TrackUid(123),
            range: MusicalTime::START..MusicalTime::new_with_beats(128),
            view_range: MusicalTime::START..MusicalTime::new_with_beats(128),
            control_atlas: Default::default(),
            sequencer,
            focused: Default::default(),
        }
    }
}
impl Displays for TimelineSettings {
    fn ui(&mut self, ui: &mut Ui) -> egui::Response {
        ui.checkbox(&mut self.hide, "Hide");
        if ui.button("Next").clicked() {
            self.focused = self.focused.next();
        }
        ui.label("Range");
        let mut range_start = self.range.start.total_beats();
        let mut range_end = self.range.end.total_beats();
        let start_response = ui.add(Slider::new(&mut range_start, 0..=1024));
        if start_response.changed() {
            self.range.start = MusicalTime::new_with_beats(range_start);
        };
        let end_response = ui.add(Slider::new(&mut range_end, 0..=1024));
        if end_response.changed() {
            self.range.end = MusicalTime::new_with_beats(range_end);
        };
        start_response | end_response
    }
}

/// Wraps a [DevicePalette] as a [Widget](eframe::egui::Widget).
pub fn device_palette<'a>(entity_factory: &'a EntityFactory) -> impl eframe::egui::Widget + 'a {
    move |ui: &mut eframe::egui::Ui| DevicePalette::new(entity_factory).ui(ui)
}

#[derive(Debug)]
struct DevicePalette<'a> {
    entity_factory: &'a EntityFactory,
}
impl<'a> DevicePalette<'a> {
    fn new(entity_factory: &'a EntityFactory) -> Self {
        Self { entity_factory }
    }
}
impl<'a> Displays for DevicePalette<'a> {
    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let desired_size = vec2(ui.available_width(), 32.0);
        ui.allocate_ui(desired_size, |ui| {
            ui.horizontal_centered(|ui| {
                for key in self.entity_factory.sorted_keys() {
                    DragDropManager::drag_source(
                        ui,
                        Id::new(key),
                        DragDropSource::NewDevice(key.clone()),
                        |ui| {
                            ui.label(key.to_string());
                        },
                    );
                }
            })
            .response
        })
        .response
    }
}

#[derive(Debug, Default)]
struct DevicePaletteSettings {
    hide: bool,
}
impl DevicePaletteSettings {
    const NAME: &str = "Device Palette";

    fn show(&mut self, ui: &mut Ui) {
        if !self.hide {
            ui.add(device_palette(EntityFactory::global()));
        }
    }
}
impl Displays for DevicePaletteSettings {
    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.checkbox(&mut self.hide, "Hide")
    }
}

/// Wraps a [DeviceChain] as a [Widget](eframe::egui::Widget). Mutates many things.
pub fn device_chain<'a>(
    track_uid: TrackUid,
    store: &'a mut ThingStore,
    controllers: &'a mut Vec<Uid>,
    instruments: &'a mut Vec<Uid>,
    effects: &'a mut Vec<Uid>,
    action: &'a mut Option<DeviceChainAction>,
) -> impl eframe::egui::Widget + 'a {
    move |ui: &mut eframe::egui::Ui| {
        DeviceChain::new(track_uid, store, controllers, instruments, effects, action).ui(ui)
    }
}

#[derive(Debug)]
pub enum DeviceChainAction {
    NewDevice(groove::mini::Key),
}

#[derive(Debug)]
struct DeviceChain<'a> {
    track_uid: TrackUid,
    store: &'a mut ThingStore,
    controllers: &'a mut Vec<Uid>,
    instruments: &'a mut Vec<Uid>,
    effects: &'a mut Vec<Uid>,

    action: &'a mut Option<DeviceChainAction>,

    is_large_size: bool,
}
impl<'a> DeviceChain<'a> {
    fn new(
        track_uid: TrackUid,
        store: &'a mut ThingStore,
        controllers: &'a mut Vec<Uid>,
        instruments: &'a mut Vec<Uid>,
        effects: &'a mut Vec<Uid>,
        action: &'a mut Option<DeviceChainAction>,
    ) -> Self {
        Self {
            track_uid,
            store,
            controllers,
            instruments,
            effects,
            action,
            is_large_size: false,
        }
    }

    fn is_large_size(mut self, is_large_size: bool) -> Self {
        self.is_large_size = is_large_size;
        self
    }

    fn can_accept(&self) -> bool {
        if let Some(source) = DragDropManager::source() {
            match source {
                DragDropSource::NewDevice(_) => true,
                _ => false,
            }
        } else {
            false
        }
    }

    fn check_drop(&mut self) {
        if let Some(source) = DragDropManager::source() {
            match source {
                DragDropSource::NewDevice(key) => {
                    *self.action = Some(DeviceChainAction::NewDevice(key))
                }
                _ => {}
            }
        }
    }
}
impl<'a> Displays for DeviceChain<'a> {
    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let desired_size = if self.is_large_size {
            vec2(ui.available_width(), 256.0)
        } else {
            vec2(ui.available_width(), 32.0)
        };
        ui.allocate_ui(desired_size, |ui| {
            ui.horizontal_top(|ui| {
                self.controllers
                    .iter()
                    .chain(self.instruments.iter().chain(self.effects.iter()))
                    .for_each(|uid| {
                        if let Some(entity) = self.store.get_mut(uid) {
                            entity.ui(ui);
                        }
                    });
                let response =
                    DragDropManager::drop_target(ui, self.can_accept(), |ui| ui.label("+"))
                        .response;
                if DragDropManager::is_dropped(ui, &response) {
                    self.check_drop();
                }
            })
        })
        .response
    }
}

#[derive(Debug, Default)]
struct DeviceChainSettings {
    hide: bool,
    is_large_size: bool,
    track_uid: TrackUid,
    store: ThingStore,
    controllers: Vec<Uid>,
    instruments: Vec<Uid>,
    effects: Vec<Uid>,
    action: Option<DeviceChainAction>,
}
impl DeviceChainSettings {
    const NAME: &str = "Device Chain";

    fn show(&mut self, ui: &mut Ui) {
        if !self.hide {
            ui.add(device_chain(
                self.track_uid,
                &mut self.store,
                &mut self.controllers,
                &mut self.instruments,
                &mut self.effects,
                &mut self.action,
            ));
        }
    }

    fn check_and_reset_action(&mut self) -> Option<DeviceChainAction> {
        self.action.take()
    }

    // This duplicates some code in Orchestrator.
    pub fn append_thing(&mut self, thing: Box<dyn Thing>) -> anyhow::Result<Uid> {
        let uid = thing.uid();
        if thing.as_controller().is_some() {
            self.controllers.push(uid);
        }
        if thing.as_effect().is_some() {
            self.effects.push(uid);
        }
        if thing.as_instrument().is_some() {
            self.instruments.push(uid);
        }
        self.store.add(thing)
    }
}
impl Displays for DeviceChainSettings {
    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.checkbox(&mut self.hide, "Hide") | ui.checkbox(&mut self.is_large_size, "Large size")
    }
}

#[derive(Debug)]
struct GridSettings {
    hide: bool,
    range: Range<MusicalTime>,
    view_range: Range<MusicalTime>,
}
impl GridSettings {
    const NAME: &str = "Grid";

    fn show(&mut self, ui: &mut Ui) {
        if !self.hide {
            ui.add(timeline::grid(self.range.clone(), self.view_range.clone()));
        }
    }
}
impl Default for GridSettings {
    fn default() -> Self {
        Self {
            hide: Default::default(),
            range: MusicalTime::START..MusicalTime::new_with_beats(128),
            view_range: MusicalTime::START..MusicalTime::new_with_beats(128),
        }
    }
}
impl DisplaysInTimeline for GridSettings {
    fn set_view_range(&mut self, view_range: &std::ops::Range<groove_core::time::MusicalTime>) {
        self.view_range = view_range.clone();
    }
}
impl Displays for GridSettings {
    fn ui(&mut self, ui: &mut Ui) -> egui::Response {
        ui.checkbox(&mut self.hide, "Hide")
    }
}

#[derive(Debug)]
struct PatternIconSettings {
    hide: bool,
    duration: MusicalTime,
    notes: Vec<Note>,
    is_selected: bool,
}
impl Default for PatternIconSettings {
    fn default() -> Self {
        Self {
            hide: Default::default(),
            duration: MusicalTime::new_with_beats(4),
            notes: vec![
                Self::note(
                    MidiNote::C4,
                    MusicalTime::START,
                    MusicalTime::DURATION_WHOLE,
                ),
                Self::note(
                    MidiNote::G4,
                    MusicalTime::START + MusicalTime::DURATION_WHOLE,
                    MusicalTime::DURATION_WHOLE,
                ),
            ],
            is_selected: Default::default(),
        }
    }
}
impl Displays for PatternIconSettings {
    fn ui(&mut self, ui: &mut Ui) -> egui::Response {
        ui.checkbox(&mut self.hide, "Hide Pattern Icon")
            | ui.checkbox(&mut self.is_selected, "Show selected")
    }
}
impl PatternIconSettings {
    const NAME: &str = "Pattern Icon";
    fn note(key: MidiNote, start: MusicalTime, duration: MusicalTime) -> Note {
        Note {
            key: key as u8,
            range: start..start + duration,
        }
    }

    fn show(&mut self, ui: &mut Ui) {
        // Pattern Icon
        if !self.hide {
            DragDropManager::drag_source(
                ui,
                Id::new("pattern icon"),
                DragDropSource::Pattern(PatternUid(99)),
                |ui| {
                    ui.add(pattern::icon(self.duration, &self.notes, self.is_selected));
                },
            );
        }
    }
}

#[derive(Debug, Default)]
struct ControlAtlasSettings {
    hide: bool,
    control_atlas: ControlAtlas,
}
impl Displays for ControlAtlasSettings {
    fn ui(&mut self, ui: &mut Ui) -> egui::Response {
        ui.checkbox(&mut self.hide, "Hide")
    }
}
impl DisplaysInTimeline for ControlAtlasSettings {
    fn set_view_range(&mut self, view_range: &std::ops::Range<groove_core::time::MusicalTime>) {
        self.control_atlas.set_view_range(view_range);
    }
}
impl ControlAtlasSettings {
    const NAME: &str = "Control Atlas";

    fn show(&mut self, ui: &mut Ui) {
        if !self.hide {
            self.control_atlas.ui(ui);
        }
    }
}

#[derive(Debug, Default)]
struct SequencerSettings {
    hide: bool,
    sequencer: Sequencer,
}
impl Displays for SequencerSettings {
    fn ui(&mut self, ui: &mut Ui) -> egui::Response {
        ui.checkbox(&mut self.hide, "Hide")
    }
}
impl DisplaysInTimeline for SequencerSettings {
    fn set_view_range(&mut self, view_range: &std::ops::Range<groove_core::time::MusicalTime>) {
        self.sequencer.set_view_range(view_range);
    }
}
impl SequencerSettings {
    const NAME: &str = "Sequencer";

    fn show(&mut self, ui: &mut Ui) {
        if !self.hide {
            self.sequencer.ui(ui);
        }
    }
}

#[derive(Debug)]
struct ESSequencerSettings {
    hide: bool,
    sequencer: ESSequencer,
}
impl Default for ESSequencerSettings {
    fn default() -> Self {
        Self {
            hide: Default::default(),
            sequencer: ESSequencerBuilder::default()
                .random(MusicalTime::START..MusicalTime::new_with_beats(128))
                .build()
                .unwrap(),
        }
    }
}

impl Displays for ESSequencerSettings {
    fn ui(&mut self, ui: &mut Ui) -> egui::Response {
        ui.checkbox(&mut self.hide, "Hide")
    }
}
impl DisplaysInTimeline for ESSequencerSettings {
    fn set_view_range(&mut self, view_range: &std::ops::Range<groove_core::time::MusicalTime>) {
        self.sequencer.set_view_range(view_range);
    }
}
impl ESSequencerSettings {
    const NAME: &str = "Even Smaller Sequencer";

    fn show(&mut self, ui: &mut Ui) {
        if !self.hide {
            self.sequencer.ui(ui);
        }
    }
}

#[derive(Debug)]
struct TitleBarSettings {
    hide: bool,
    title: TrackTitle,
}
impl Default for TitleBarSettings {
    fn default() -> Self {
        Self {
            hide: Default::default(),
            title: Default::default(),
        }
    }
}
impl Displays for TitleBarSettings {
    fn ui(&mut self, ui: &mut Ui) -> egui::Response {
        ui.checkbox(&mut self.hide, "Hide");
        ui.text_edit_singleline(&mut self.title.0)
    }
}
impl TitleBarSettings {
    const NAME: &str = "Title Bar";

    fn show(&mut self, ui: &mut Ui) {
        if !self.hide {
            ui.add(track::title_bar(&mut self.title.0));
        }
    }
}

#[derive(Debug, Default)]
struct PianoRollSettings {
    hide: bool,
    piano_roll: PianoRoll,
}
impl Displays for PianoRollSettings {
    fn ui(&mut self, ui: &mut Ui) -> egui::Response {
        ui.checkbox(&mut self.hide, "Hide")
    }
}
impl PianoRollSettings {
    const NAME: &str = "Piano Roll";

    fn show(&mut self, ui: &mut Ui) {
        if !self.hide {
            self.piano_roll.ui(ui);
        }
    }
}

#[derive(Debug)]
struct WigglerSettings {
    hide: bool,
}
impl Default for WigglerSettings {
    fn default() -> Self {
        Self {
            hide: Default::default(),
        }
    }
}
impl Displays for WigglerSettings {
    fn ui(&mut self, ui: &mut Ui) -> egui::Response {
        ui.checkbox(&mut self.hide, "Hide")
    }
}
impl WigglerSettings {
    const NAME: &str = "Wiggler";

    fn show(&mut self, ui: &mut Ui) {
        if !self.hide {
            ui.add(placeholder::wiggler());
        }
    }
}

#[derive(Debug, Default)]
struct Explorer {
    legend: LegendSettings,
    grid: GridSettings,
    timeline: TimelineSettings,
    device_palette: DevicePaletteSettings,
    device_chain: DeviceChainSettings,
    control_atlas: ControlAtlasSettings,
    sequencer: SequencerSettings,
    es_sequencer: ESSequencerSettings,
    pattern_icon: PatternIconSettings,
    title_bar: TitleBarSettings,
    piano_roll: PianoRollSettings,
    wiggler: WigglerSettings,
}
impl Explorer {
    pub const NAME: &str = "Explorer";

    pub fn new(_cc: &CreationContext) -> Self {
        Self {
            ..Default::default()
        }
    }

    fn show_top(&mut self, ui: &mut Ui) {
        ui.label("This is the top section");
    }

    fn show_bottom(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            warn_if_debug_build(ui);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(app_version())
            });
        });
    }

    fn show_left(&mut self, ui: &mut Ui) {
        ScrollArea::horizontal().show(ui, |ui| {
            Self::wrap_settings(LegendSettings::NAME, ui, |ui| self.legend.ui(ui));
            Self::wrap_settings(TimelineSettings::NAME, ui, |ui| self.timeline.ui(ui));
            Self::wrap_settings(DevicePaletteSettings::NAME, ui, |ui| {
                self.device_palette.ui(ui)
            });
            Self::wrap_settings(DeviceChainSettings::NAME, ui, |ui| self.device_chain.ui(ui));
            Self::wrap_settings(PianoRollSettings::NAME, ui, |ui| self.piano_roll.ui(ui));
            Self::wrap_settings(GridSettings::NAME, ui, |ui| self.grid.ui(ui));
            Self::wrap_settings(PatternIconSettings::NAME, ui, |ui| self.pattern_icon.ui(ui));
            Self::wrap_settings(ControlAtlasSettings::NAME, ui, |ui| {
                self.control_atlas.ui(ui)
            });
            Self::wrap_settings(SequencerSettings::NAME, ui, |ui| self.sequencer.ui(ui));
            Self::wrap_settings(ESSequencerSettings::NAME, ui, |ui| self.es_sequencer.ui(ui));
            Self::wrap_settings(TitleBarSettings::NAME, ui, |ui| self.title_bar.ui(ui));
            Self::wrap_settings(WigglerSettings::NAME, ui, |ui| self.wiggler.ui(ui));
            self.debug_ui(ui);
        });
    }

    fn wrap_settings(
        name: &str,
        ui: &mut Ui,
        add_body: impl FnOnce(&mut Ui) -> eframe::egui::Response,
    ) {
        CollapsingHeader::new(name)
            .show_background(true)
            .show_unindented(ui, add_body);
    }

    fn wrap_item(name: &str, ui: &mut Ui, add_body: impl FnOnce(&mut Ui)) {
        ui.heading(name);
        add_body(ui);
        ui.separator();
    }

    fn debug_ui(&mut self, ui: &mut Ui) {
        let mut debug_on_hover = ui.ctx().debug_on_hover();
        ui.checkbox(&mut debug_on_hover, "🐛 Debug on hover")
            .on_hover_text("Show structure of the ui when you hover with the mouse");
        ui.ctx().set_debug_on_hover(debug_on_hover);

        let style: Style = (*ui.ctx().style()).clone();
        let new_visuals = style.visuals.light_dark_small_toggle_button(ui);
        if let Some(visuals) = new_visuals {
            ui.ctx().set_visuals(visuals);
        }
    }

    fn show_right(&mut self, ui: &mut Ui) {
        ScrollArea::horizontal().show(ui, |ui| ui.label("Under Construction"));
    }

    fn show_center(&mut self, ui: &mut Ui) {
        ScrollArea::vertical().show(ui, |ui| {
            self.timeline.set_view_range(&self.legend.range);
            self.control_atlas.set_view_range(&self.legend.range);
            self.grid.set_view_range(&self.legend.range);
            self.sequencer.set_view_range(&self.legend.range);
            self.es_sequencer.set_view_range(&self.legend.range);

            ui.heading("Timeline");
            self.legend.show(ui);
            self.timeline.show(ui);

            Self::wrap_item(DevicePaletteSettings::NAME, ui, |ui| {
                self.device_palette.show(ui)
            });
            Self::wrap_item(DeviceChainSettings::NAME, ui, |ui| {
                self.device_chain.show(ui)
            });
            Self::wrap_item(PianoRollSettings::NAME, ui, |ui| self.piano_roll.show(ui));

            Self::wrap_item(GridSettings::NAME, ui, |ui| self.grid.show(ui));
            Self::wrap_item(PatternIconSettings::NAME, ui, |ui| {
                self.pattern_icon.show(ui)
            });
            Self::wrap_item(ControlAtlasSettings::NAME, ui, |ui| {
                self.control_atlas.show(ui)
            });
            Self::wrap_item(SequencerSettings::NAME, ui, |ui| self.sequencer.show(ui));
            Self::wrap_item(ESSequencerSettings::NAME, ui, |ui| {
                self.es_sequencer.show(ui)
            });
            Self::wrap_item(TitleBarSettings::NAME, ui, |ui| self.title_bar.show(ui));
            Self::wrap_item(WigglerSettings::NAME, ui, |ui| self.wiggler.show(ui));
        });
    }
}
impl eframe::App for Explorer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let top = egui::TopBottomPanel::top("top-panel")
            .resizable(false)
            .exact_height(64.0);
        let bottom = egui::TopBottomPanel::bottom("bottom-panel")
            .resizable(false)
            .exact_height(24.0);
        let left = egui::SidePanel::left("left-panel")
            .resizable(true)
            .default_width(160.0)
            .width_range(160.0..=480.0);
        let right = egui::SidePanel::right("right-panel")
            .resizable(true)
            .default_width(160.0)
            .width_range(160.0..=480.0);
        let center = egui::CentralPanel::default();

        top.show(ctx, |ui| {
            self.show_top(ui);
        });
        bottom.show(ctx, |ui| {
            self.show_bottom(ui);
        });
        left.show(ctx, |ui| {
            self.show_left(ui);
        });
        right.show(ctx, |ui| {
            self.show_right(ui);
        });
        center.show(ctx, |ui| {
            self.show_center(ui);
        });

        // TODO: this is bad design because it does non-GUI processing during
        // the update() method. It's OK here because this is a widget explorer,
        // not a time-critical app.
        if let Some(action) = self.device_chain.check_and_reset_action() {
            match action {
                DeviceChainAction::NewDevice(key) => {
                    eprintln!("DeviceChainAction::NewDevice({key})");
                    if let Some(thing) = EntityFactory::global().new_thing(&key) {
                        let _ = self.device_chain.append_thing(thing);
                    }
                }
            }
        }
        let events = DragDropManager::take_and_clear_events();
        events.iter().for_each(|e| match e {
            DragDropEvent::AddDeviceToTrack(key, track_uid) => {
                eprintln!("DragDropEvent::AddDeviceToTrack {key} {track_uid}")
            }
            DragDropEvent::AddPatternToTrack(pattern_uid, track_uid, position) => {
                eprintln!(
                    "DragDropEvent::AddPatternToTrack {pattern_uid} {track_uid} {position:?}"
                );
                if let Some(pattern) = self.piano_roll.piano_roll.get_pattern(pattern_uid) {
                    let _ = self.timeline.sequencer.insert_pattern(pattern, *position);
                }
            }
        });
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1366.0, 768.0)),
        ..Default::default()
    };

    if EntityFactory::initialize(register_factory_entities(EntityFactory::default())).is_err() {
        return Err(anyhow!("Couldn't initialize EntityFactory"));
    }
    if DragDropManager::initialize(DragDropManager::default()).is_err() {
        return Err(anyhow!("Couldn't set DragDropManager once_cell"));
    }

    if let Err(e) = eframe::run_native(
        Explorer::NAME,
        options,
        Box::new(|cc| Box::new(Explorer::new(cc))),
    ) {
        Err(anyhow!("eframe::run_native(): {:?}", e))
    } else {
        Ok(())
    }
}
