// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! The [Explorer] example is a sandbox for developing egui components and
//! widgets.

use anyhow::anyhow;
use eframe::{
    egui::{
        self, vec2, warn_if_debug_build, CollapsingHeader, Frame, Id, Layout, ScrollArea, Slider,
        Ui,
    },
    emath::{Align, RectTransform},
    epaint::Rect,
    CreationContext,
};
use groove::{
    app_version,
    mini::{
        register_factory_entities,
        widgets::{grid, icon, legend, wiggler},
        ControlAtlas, DragDropManager, DragDropSource, ESSequencer, ESSequencerBuilder, Note,
        PatternUid, Sequencer, DD_MANAGER, FACTORY,
    },
    EntityFactory,
};
use groove_core::{
    midi::MidiNote,
    time::MusicalTime,
    traits::{
        gui::{Displays, DisplaysInTimeline},
        Serializable,
    },
};
use std::{ops::Range, sync::Mutex};

#[derive(Debug)]
struct LegendSettings {
    hide: bool,
    range: Range<MusicalTime>,
}
impl LegendSettings {
    const NAME: &str = "Legend";

    fn show(&mut self, ui: &mut Ui) {
        if !self.hide {
            ui.add(legend(&mut self.range));
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
    range: Range<MusicalTime>,
    view_range: Range<MusicalTime>,
    control_atlas: ControlAtlas,
    sequencer: ESSequencer,
    focused: FocusedComponent,
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
            ui.add(timeline(
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
        let mut sequencer = ESSequencerBuilder::default()
            .random(MusicalTime::START..MusicalTime::new_with_beats(128))
            .build()
            .unwrap();
        sequencer.after_deser(); // TODO LAME
        Self {
            hide: Default::default(),
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

fn timeline<'a>(
    sequencer: &'a mut ESSequencer,
    control_atlas: &'a mut ControlAtlas,
    range: Range<MusicalTime>,
    view_range: Range<MusicalTime>,
    focused: FocusedComponent,
) -> impl eframe::egui::Widget + 'a {
    move |ui: &mut eframe::egui::Ui| {
        Timeline::new(sequencer, control_atlas)
            .range(range)
            .view_range(view_range)
            .focused(focused)
            .ui(ui)
    }
}

#[derive(Debug, Default, Copy, Clone)]
enum FocusedComponent {
    #[default]
    ControlAtlas,
    Sequencer,
}
impl FocusedComponent {
    fn next(&self) -> Self {
        match self {
            FocusedComponent::ControlAtlas => FocusedComponent::Sequencer,
            FocusedComponent::Sequencer => FocusedComponent::ControlAtlas,
        }
    }
}

/// Draws the content area of a Timeline, which is the view of a [Track].
#[derive(Debug)]
struct Timeline<'a> {
    /// The full timespan of the project.
    range: Range<MusicalTime>,

    /// The part of the timeline that is viewable.
    view_range: Range<MusicalTime>,

    /// Which component is currently enabled,
    focused: FocusedComponent,

    control_atlas: &'a mut ControlAtlas,
    sequencer: &'a mut ESSequencer,
}
impl<'a> DisplaysInTimeline for Timeline<'a> {
    fn set_view_range(&mut self, view_range: &std::ops::Range<groove_core::time::MusicalTime>) {
        self.view_range = view_range.clone();
        self.control_atlas.set_view_range(view_range);
        self.sequencer.set_view_range(view_range);
    }
}
impl<'a> Displays for Timeline<'a> {
    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let mut from_screen = RectTransform::identity(Rect::NOTHING);
        let dd = DragDropManager::global().lock().unwrap();
        let response = dd
            .drop_target(ui, true, |ui, _| {
                let desired_size = vec2(ui.available_width(), 64.0);
                let (_id, rect) = ui.allocate_space(desired_size);
                from_screen = RectTransform::from_to(
                    rect,
                    Rect::from_x_y_ranges(
                        self.view_range.start.total_units() as f32
                            ..=self.view_range.end.total_units() as f32,
                        rect.top()..=rect.bottom(),
                    ),
                );
                self.ui_not_focused(ui, rect, self.focused);
                self.ui_focused(ui, rect, self.focused)
            })
            .response;
        if dd.is_dropped(ui, &response) && dd.source().is_some() {
            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let time_pos = from_screen * pointer_pos;
                let time = MusicalTime::new_with_units(time_pos.x as usize);
                eprintln!(
                    "Source {:?} dropped on timeline at point screen/view_range {:?}/{:?} -> {time}",
                    dd.source().unwrap(),
                    pointer_pos,
                    from_screen * pointer_pos
                );
            } else {
                eprintln!("Dropped on timeline at unknown position");
            }
        }
        response
    }
}
impl<'a> Timeline<'a> {
    pub fn new(sequencer: &'a mut ESSequencer, control_atlas: &'a mut ControlAtlas) -> Self {
        Self {
            range: Default::default(),
            view_range: Default::default(),
            focused: Default::default(),
            sequencer,
            control_atlas,
        }
    }
    fn range(mut self, range: Range<MusicalTime>) -> Self {
        self.range = range;
        self
    }

    fn view_range(mut self, view_range: Range<MusicalTime>) -> Self {
        self.set_view_range(&view_range);
        self
    }

    fn focused(mut self, component: FocusedComponent) -> Self {
        self.focused = component;
        self
    }

    // Draws the Timeline component that is currently focused.
    fn ui_focused(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        component: FocusedComponent,
    ) -> egui::Response {
        match component {
            FocusedComponent::ControlAtlas => {
                ui.add_enabled_ui(true, |ui| {
                    ui.allocate_ui_at_rect(rect, |ui| self.control_atlas.ui(ui))
                        .inner
                })
                .inner
            }
            FocusedComponent::Sequencer => {
                ui.add_enabled_ui(true, |ui| {
                    ui.allocate_ui_at_rect(rect, |ui| self.sequencer.ui(ui))
                        .inner
                })
                .inner
            }
        }
    }

    // Draws the Timeline components that are not currently focused.
    fn ui_not_focused(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        which: FocusedComponent,
    ) -> egui::Response {
        // The Grid is always disabled and drawn first.
        let mut response = ui
            .add_enabled_ui(false, |ui| {
                ui.allocate_ui_at_rect(rect, |ui| {
                    ui.add(grid(self.range.clone(), self.view_range.clone()))
                })
                .inner
            })
            .inner;

        // Now go through and draw the components that are *not* enabled.
        if !matches!(which, FocusedComponent::ControlAtlas) {
            response |= ui
                .add_enabled_ui(false, |ui| {
                    ui.allocate_ui_at_rect(rect, |ui| self.control_atlas.ui(ui))
                        .inner
                })
                .inner;
        }
        if !matches!(which, FocusedComponent::Sequencer) {
            response |= ui
                .add_enabled_ui(false, |ui| {
                    ui.allocate_ui_at_rect(rect, |ui| self.sequencer.ui(ui))
                        .inner
                })
                .inner;
        }
        response
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
            ui.add(grid(self.range.clone(), self.view_range.clone()));
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
        }
    }
}
impl Displays for PatternIconSettings {
    fn ui(&mut self, ui: &mut Ui) -> egui::Response {
        ui.checkbox(&mut self.hide, "Hide Pattern Icon")
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
            DragDropManager::global().lock().unwrap().drag_source(
                ui,
                Id::new("pattern icon"),
                DragDropSource::Pattern(PatternUid(99)),
                |ui| {
                    ui.add(icon(self.duration, &self.notes));
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

#[derive(Debug, Default)]
struct ESSequencerSettings {
    hide: bool,
    sequencer: ESSequencer,
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
            ui.add(wiggler());
        }
    }
}

#[derive(Debug, Default)]
struct Explorer {
    legend: LegendSettings,
    grid: GridSettings,
    pattern_icon: PatternIconSettings,
    timeline: TimelineSettings,
    control_atlas: ControlAtlasSettings,
    sequencer: SequencerSettings,
    es_sequencer: ESSequencerSettings,
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
            Self::wrap_settings(GridSettings::NAME, ui, |ui| self.grid.ui(ui));
            Self::wrap_settings(PatternIconSettings::NAME, ui, |ui| self.pattern_icon.ui(ui));
            Self::wrap_settings(ControlAtlasSettings::NAME, ui, |ui| {
                self.control_atlas.ui(ui)
            });
            Self::wrap_settings(SequencerSettings::NAME, ui, |ui| self.sequencer.ui(ui));
            Self::wrap_settings(ESSequencerSettings::NAME, ui, |ui| self.es_sequencer.ui(ui));
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
    }

    fn show_right(&mut self, ui: &mut Ui) {
        ScrollArea::horizontal().show(ui, |ui| ui.label("Under Construction"));
    }

    fn show_center(&mut self, ui: &mut Ui) {
        Frame::default()
            .stroke(ui.style().visuals.window_stroke)
            .show(ui, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    self.timeline.set_view_range(&self.legend.range);
                    self.control_atlas.set_view_range(&self.legend.range);
                    self.grid.set_view_range(&self.legend.range);
                    self.sequencer.set_view_range(&self.legend.range);
                    self.es_sequencer.set_view_range(&self.legend.range);

                    ui.heading("Timeline");
                    self.legend.show(ui);
                    self.timeline.show(ui);
                    ui.add_space(32.0);

                    ui.separator();

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
                    Self::wrap_item(WigglerSettings::NAME, ui, |ui| self.wiggler.show(ui));
                });
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
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1366.0, 768.0)),
        ..Default::default()
    };

    if FACTORY
        .set(register_factory_entities(EntityFactory::default()))
        .is_err()
    {
        return Err(anyhow!("Couldn't initialize EntityFactory"));
    }
    if DD_MANAGER
        .set(Mutex::new(DragDropManager::default()))
        .is_err()
    {
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
