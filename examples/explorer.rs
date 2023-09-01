// Copyright (c) 2023 Mike Tsao. All rights reserved.

use anyhow::anyhow;
use eframe::{
    egui::{
        self, vec2, warn_if_debug_build, CollapsingHeader, Frame, Id, Layout, ScrollArea, Slider,
        Ui,
    },
    emath::Align,
    CreationContext,
};
use groove::{
    app_version,
    mini::{
        register_factory_entities,
        widgets::{grid, icon, legend, wiggler},
        ControlAtlas, DragDropManager, DragDropSource, Note, PatternUid, Sequencer, DD_MANAGER,
        FACTORY,
    },
    EntityFactory,
};
use groove_core::{
    midi::MidiNote,
    time::MusicalTime,
    traits::gui::{Displays, DisplaysInTimeline},
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
        CollapsingHeader::new(Self::NAME)
            .show_background(true)
            .show_unindented(ui, |ui| {
                ui.checkbox(&mut self.hide, "Hide");
                ui.label("View range");
                let mut range_start = self.range.start.total_beats();
                let mut range_end = self.range.end.total_beats();
                if ui.add(Slider::new(&mut range_start, 0..=128)).changed() {
                    self.range.start = MusicalTime::new_with_beats(range_start);
                };
                if ui.add(Slider::new(&mut range_end, 1..=256)).changed() {
                    self.range.end = MusicalTime::new_with_beats(range_end);
                };
            })
            .header_response
    }
}

#[derive(Debug)]
struct TimelineSettings {
    hide: bool,
    range: Range<MusicalTime>,
    view_range: Range<MusicalTime>,
    control_atlas: ControlAtlas,
    sequencer: Sequencer,
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
            let mut handled_drop = false;

            // We need to be careful with this MutexGuard. Our mutable
            // operation, reset(), happens in this block, and our other usage of
            // dd is drop_target(), which takes only &self. Where things can go
            // wrong are (1) we want to create a drag source here, which is
            // possible but tricky because it's an &mut self, or (2) someone we
            // call tries to call DragDropManager::global().lock(), which would
            // cause a deadlock. I have tried to reduce the risk of that
            // happening by adding a reference to DragDropManager in the
            // drop_target closure.
            let mut dd = DragDropManager::global().lock().unwrap();

            let response = dd
                .drop_target(ui, true, |ui, dd| {
                    ui.add(timeline(
                        &mut self.sequencer,
                        &mut self.control_atlas,
                        self.range.clone(),
                        self.view_range.clone(),
                        dd,
                        &mut handled_drop,
                    ));
                })
                .response;
            if handled_drop {
                // Because we call drop_target within something that calls
                // drag_source, drop_target must take a non-mut dd. Which means
                // that drop_target needs to communicate to the caller that
                // cleanup is needed, because drop_target can't do it itself.
                dd.reset();
            }
            if dd.is_dropped(ui, &response) && dd.source().is_some() {
                dd.reset();
                // TODO: calculate real number
                eprintln!("Dropped on arrangement at beat {}", 2);
            }
        }
    }
}
impl Default for TimelineSettings {
    fn default() -> Self {
        Self {
            hide: Default::default(),
            range: MusicalTime::START..MusicalTime::new_with_beats(128),
            view_range: MusicalTime::START..MusicalTime::new_with_beats(128),
            control_atlas: Default::default(),
            sequencer: Default::default(),
        }
    }
}
impl Displays for TimelineSettings {
    fn ui(&mut self, ui: &mut Ui) -> egui::Response {
        CollapsingHeader::new(Self::NAME)
            .show_background(true)
            .show_unindented(ui, |ui| {
                ui.checkbox(&mut self.hide, "Hide");
                ui.label("Range");
                let mut range_start = self.range.start.total_beats();
                let mut range_end = self.range.end.total_beats();
                if ui.add(Slider::new(&mut range_start, 0..=1024)).changed() {
                    self.range.start = MusicalTime::new_with_beats(range_start);
                };
                if ui.add(Slider::new(&mut range_end, 0..=1024)).changed() {
                    self.range.end = MusicalTime::new_with_beats(range_end);
                };
            })
            .header_response
    }
}

fn timeline<'a>(
    sequencer: &'a mut Sequencer,
    control_atlas: &'a mut ControlAtlas,
    range: Range<MusicalTime>,
    view_range: Range<MusicalTime>,
    dd: &'a DragDropManager,
    handled_drop: &'a mut bool,
) -> impl eframe::egui::Widget + 'a {
    move |ui: &mut eframe::egui::Ui| {
        Timeline::new(sequencer, control_atlas, dd, handled_drop)
            .range(range)
            .view_range(view_range)
            .ui(ui)
    }
}

/// Draws the content area of a Timeline, which is the view of a [Track].
#[derive(Debug)]
struct Timeline<'a> {
    /// The full timespan of the project.
    range: Range<MusicalTime>,

    /// The part of the timeline that is viewable.
    view_range: Range<MusicalTime>,

    control_atlas: &'a mut ControlAtlas,
    sequencer: &'a mut Sequencer,

    dd: &'a DragDropManager,
    handled_drop: &'a mut bool,
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
        let desired_size = vec2(ui.available_width(), 64.0);
        let (_id, rect) = ui.allocate_space(desired_size);

        let response = self
            .dd
            .drop_target(ui, true, |ui, _| {
                let grid_response = ui
                    .allocate_ui_at_rect(rect, |ui| {
                        ui.add(grid(self.range.clone(), self.view_range.clone()))
                    })
                    .inner;
                let sequencer_response = ui
                    .allocate_ui_at_rect(rect, |ui| self.sequencer.ui(ui))
                    .inner;
                let control_atlas_response = ui
                    .allocate_ui_at_rect(rect, |ui| self.control_atlas.ui(ui))
                    .inner;
                grid_response | control_atlas_response | sequencer_response
            })
            .response;
        if !*self.handled_drop && self.dd.is_dropped(ui, &response) {
            *self.handled_drop = true;
            eprintln!("Dropped on something");
        }
        response
    }
}
impl<'a> Timeline<'a> {
    pub fn new(
        sequencer: &'a mut Sequencer,
        control_atlas: &'a mut ControlAtlas,
        dd: &'a DragDropManager,
        handled_drop: &'a mut bool,
    ) -> Self {
        Self {
            range: Default::default(),
            view_range: Default::default(),
            sequencer,
            control_atlas,
            dd,
            handled_drop,
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
        CollapsingHeader::new(Self::NAME)
            .show_background(true)
            .show_unindented(ui, |ui| {
                ui.checkbox(&mut self.hide, "Hide");
            })
            .header_response
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
        CollapsingHeader::new(Self::NAME)
            .show_background(true)
            .show_unindented(ui, |ui| {
                ui.checkbox(&mut self.hide, "Hide Pattern Icon");
            })
            .header_response
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
        CollapsingHeader::new(Self::NAME)
            .show_background(true)
            .show_unindented(ui, |ui| {
                ui.checkbox(&mut self.hide, "Hide");
            })
            .header_response
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
        CollapsingHeader::new(Self::NAME)
            .show_background(true)
            .show_unindented(ui, |ui| {
                ui.checkbox(&mut self.hide, "Hide");
            })
            .header_response
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
            self.legend.ui(ui);
            self.timeline.ui(ui);
            self.grid.ui(ui);
            self.pattern_icon.ui(ui);
            self.control_atlas.ui(ui);
            self.wiggler.ui(ui);
            self.debug_ui(ui);
        });
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

                    ui.heading("Timeline");
                    self.legend.show(ui);
                    self.timeline.show(ui);
                    ui.add_space(32.0);

                    ui.heading("Widgets");

                    self.grid.show(ui);
                    self.pattern_icon.show(ui);
                    self.control_atlas.show(ui);
                    self.wiggler.show(ui);
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
