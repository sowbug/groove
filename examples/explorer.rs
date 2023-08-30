// Copyright (c) 2023 Mike Tsao. All rights reserved.

use anyhow::anyhow;
use eframe::{
    egui::{
        self, vec2, warn_if_debug_build, Frame, Id, Label, Layout, Response, ScrollArea, Sense,
        Slider, Ui,
    },
    emath::{Align, RectTransform},
    epaint::{pos2, Color32, Rect, Rounding, Stroke},
    CreationContext,
};
use groove::{
    app_version,
    mini::{
        register_factory_entities,
        widgets::{arrangement_legend, pattern_icon},
        ControlAtlas, DragDropManager, DragDropSource, Note, PatternUid, FACTORY,
    },
    EntityFactory,
};
use groove_core::{
    midi::MidiNote,
    time::MusicalTime,
    traits::gui::{Displays, DisplaysInTimeline},
};
use std::ops::Range;

#[derive(Debug)]
struct ArrangementLegendSettings {
    hide: bool,
    range: Range<MusicalTime>,
}
impl Default for ArrangementLegendSettings {
    fn default() -> Self {
        Self {
            hide: Default::default(),
            range: MusicalTime::START..MusicalTime::new_with_beats(128),
        }
    }
}
impl Displays for ArrangementLegendSettings {
    fn ui(&mut self, ui: &mut Ui) -> egui::Response {
        ui.allocate_ui(ui.available_size(), |ui| {
            ui.checkbox(&mut self.hide, "Hide Arrangement Legend");
            ui.label("start/end");
            let mut range_start = self.range.start.total_beats();
            let mut range_end = self.range.end.total_beats();
            if ui.add(Slider::new(&mut range_start, 0..=128)).changed() {
                self.range.start = MusicalTime::new_with_beats(range_start);
            };
            if ui.add(Slider::new(&mut range_end, 1..=256)).changed() {
                self.range.end = MusicalTime::new_with_beats(range_end);
            };
        })
        .response
    }
}

#[derive(Debug)]
struct ArrangementSettings {
    hide: bool,
    range: Range<MusicalTime>,
}
impl Default for ArrangementSettings {
    fn default() -> Self {
        Self {
            hide: Default::default(),
            range: MusicalTime::START..MusicalTime::new_with_beats(128),
        }
    }
}
impl Displays for ArrangementSettings {
    fn ui(&mut self, ui: &mut Ui) -> egui::Response {
        ui.allocate_ui(ui.available_size(), |ui| {
            ui.checkbox(&mut self.hide, "Hide Arrangement");
            ui.label("start/end");
            let mut range_start = self.range.start.total_beats();
            let mut range_end = self.range.end.total_beats();
            if ui.add(Slider::new(&mut range_start, 0..=1024)).changed() {
                self.range.start = MusicalTime::new_with_beats(range_start);
            };
            if ui.add(Slider::new(&mut range_end, 0..=1024)).changed() {
                self.range.end = MusicalTime::new_with_beats(range_end);
            };
        })
        .response
    }
}

#[derive(Debug)]
struct Arrangement<'a> {
    // Whether a drop source is currently hovering over this widget.
    handled_drop: &'a mut bool,
    range: Range<MusicalTime>,
}
impl<'a> Arrangement<'a> {
    fn new(handled_drop: &'a mut bool) -> Self {
        Self {
            handled_drop,
            range: MusicalTime::START..MusicalTime::new_with_beats(128),
        }
    }
    fn range(mut self, range: Range<MusicalTime>) -> Self {
        self.range = range;
        self
    }

    fn ui_content(self, ui: &mut Ui, dnd: &DragDropManager) -> Response {
        let Self {
            handled_drop,
            range,
        } = self;
        let desired_size = vec2(ui.available_width(), 64.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());
        if response.clicked() {
            eprintln!("the empty space got a click");
        }

        let to_screen = RectTransform::from_to(
            eframe::epaint::Rect::from_x_y_ranges(
                range.start.total_beats() as f32..=range.end.total_beats() as f32,
                0.0..=1.0,
            ),
            rect,
        );

        let painter = ui.painter_at(rect);
        // This could have been done as just "rect", but I wanted to make sure
        // to_screen is working and that everyone's using it.
        let painting_rect = Rect::from_two_pos(
            to_screen * pos2(range.start.total_beats() as f32, 0.0),
            to_screen * pos2(range.end.total_beats() as f32, 1.0),
        );
        painter.rect(
            painting_rect,
            Rounding::same(2.0),
            Color32::LIGHT_GRAY,
            Stroke::default(),
        );

        for i in 0..10 {
            let pattern_start = MusicalTime::new_with_beats(i * 8);
            let pattern_end = MusicalTime::new_with_beats(i * 8 + 4);

            let pattern_start_beats = pattern_start.total_beats();
            let pattern_end_beats = pattern_end.total_beats();

            let pattern_rect = Rect::from_two_pos(
                to_screen * pos2(pattern_start_beats as f32, 0.0),
                to_screen * pos2(pattern_end_beats as f32, 1.0),
            );

            let _ = ui
                .allocate_ui_at_rect(pattern_rect, |ui| {
                    let response = dnd
                        .drop_target(ui, true, |ui| ui.add(fill_widget()))
                        .response;
                    if !*handled_drop && dnd.is_dropped(ui, response) {
                        *handled_drop = true;
                        eprintln!("Dropped on arranged pattern {i}");
                    }
                })
                .response;
        }

        response
    }
}

fn fill_widget() -> impl eframe::egui::Widget {
    move |ui: &mut eframe::egui::Ui| FillWidget::new().ui(ui)
}

struct FillWidget {}
impl FillWidget {
    fn new() -> Self {
        Self {}
    }
}
impl Displays for FillWidget {
    fn ui(&mut self, ui: &mut Ui) -> Response {
        // let desired_size = ui.available_size();
        // ui.set_min_size(desired_size);
        // ui.set_max_size(desired_size);
        let desired_size = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

        let painter = ui.painter_at(rect);
        painter.rect(
            rect,
            Rounding::same(2.0),
            Color32::DARK_GREEN,
            Stroke {
                width: 1.0,
                color: Color32::LIGHT_GREEN,
            },
        );
        response
    }
}

fn arrangement<'a>(
    dnd: &'a DragDropManager,
    range: Range<MusicalTime>,
    handled_drop: &'a mut bool,
) -> impl eframe::egui::Widget + 'a {
    move |ui: &mut eframe::egui::Ui| {
        Arrangement::new(handled_drop)
            .range(range)
            .ui_content(ui, dnd)
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
        ui.allocate_ui(ui.available_size(), |ui| {
            ui.checkbox(&mut self.hide, "Hide Pattern Icon");
        })
        .response
    }
}
impl PatternIconSettings {
    fn note(key: MidiNote, start: MusicalTime, duration: MusicalTime) -> Note {
        Note {
            key: key as u8,
            range: start..start + duration,
        }
    }
}

#[derive(Debug)]
struct ControlAtlasSettings {
    hide: bool,
    range: Range<MusicalTime>,
}
impl Default for ControlAtlasSettings {
    fn default() -> Self {
        Self {
            hide: Default::default(),
            range: MusicalTime::START..MusicalTime::new_with_beats(128),
        }
    }
}
impl Displays for ControlAtlasSettings {
    fn ui(&mut self, ui: &mut Ui) -> egui::Response {
        ui.allocate_ui(ui.available_size(), |ui| {
            ui.checkbox(&mut self.hide, "Hide ControlAtlas");
            ui.label("start/end");
            let mut range_start = self.range.start.total_beats();
            let mut range_end = self.range.end.total_beats();
            if ui.add(Slider::new(&mut range_start, 0..=1024)).changed() {
                self.range.start = MusicalTime::new_with_beats(range_start);
            };
            if ui.add(Slider::new(&mut range_end, 0..=1024)).changed() {
                self.range.end = MusicalTime::new_with_beats(range_end);
            };
        })
        .response
    }
}

#[derive(Debug, Default)]
struct Explorer {
    dnd: DragDropManager,
    arrangement_legend: ArrangementLegendSettings,
    pattern_icon: PatternIconSettings,
    arrangement: ArrangementSettings,
    control_atlas_settings: ControlAtlasSettings,
    control_atlas: ControlAtlas,
}
impl Explorer {
    pub const APP_NAME: &str = "Explorer";

    pub fn new(_cc: &CreationContext) -> Self {
        Self {
            ..Default::default()
        }
    }

    fn show_top(&mut self, ui: &mut Ui) {
        ui.label("top");
        ui.separator();
        ui.label("top 2");
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
            self.arrangement_legend.ui(ui);
            ui.separator();

            self.pattern_icon.ui(ui);
            ui.separator();

            self.arrangement.ui(ui);
            ui.separator();

            self.control_atlas_settings.ui(ui);
            ui.separator();

            let mut debug_on_hover = ui.ctx().debug_on_hover();
            ui.checkbox(&mut debug_on_hover, "🐛 Debug on hover")
                .on_hover_text("Show structure of the ui when you hover with the mouse");
            ui.ctx().set_debug_on_hover(debug_on_hover);
        });
    }

    fn show_right(&mut self, ui: &mut Ui) {
        ScrollArea::horizontal().show(ui, |ui| ui.label("Under Construction"));
    }

    fn show_center(&mut self, ui: &mut Ui) {
        Frame::default()
            .stroke(ui.style().visuals.window_stroke)
            .show(ui, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    // Legend
                    if !self.arrangement_legend.hide {
                        ui.add(arrangement_legend(self.arrangement_legend.range.clone()));
                    }
                    ui.separator();

                    // Pattern Icon
                    if !self.pattern_icon.hide {
                        self.dnd.drag_source(
                            ui,
                            Id::new("pattern icon"),
                            DragDropSource::Pattern(PatternUid(99)),
                            |ui| {
                                ui.add(pattern_icon(
                                    self.pattern_icon.duration,
                                    &self.pattern_icon.notes,
                                ));
                            },
                        );
                    }
                    ui.separator();

                    // Arrangement
                    if !self.arrangement.hide {
                        let mut handled_drop = false;
                        let response = self
                            .dnd
                            .drop_target(ui, true, |ui| {
                                ui.add(arrangement(
                                    &self.dnd,
                                    self.arrangement.range.clone(),
                                    &mut handled_drop,
                                ));
                            })
                            .response;
                        if handled_drop {
                            // Because we call drop_target within something that
                            // calls drag_source, drop_target must take a
                            // non-mut dnd. Which means that drop_target needs
                            // to communicate to the caller that cleanup is
                            // needed, because drop_target can't do it itself.
                            self.dnd.reset();
                        }
                        if self.dnd.is_dropped(ui, response) && self.dnd.source().is_some() {
                            self.dnd.reset();
                            eprintln!("Dropped on arrangement at beat {}", 2);
                        }
                    }
                    ui.separator();

                    // Control Atlas
                    if !self.control_atlas_settings.hide {
                        self.control_atlas
                            .set_view_range(&self.arrangement_legend.range);
                        self.control_atlas.ui(ui);
                    }
                    ui.separator();

                    // How big the paint surface should be
                    let desired_size = vec2(ui.available_width(), 64.0);
                    // Ask Ui to turn that Vec2 into a laid-out area
                    let (_id, rect) = ui.allocate_space(desired_size);
                    // Get the portion of the Ui painter corresponding to the area we want to paint
                    let painter = ui.painter_at(rect);

                    // Example of painting within the region
                    // For easier painting, use the to_screen approach to transform local coords to the screen rect as
                    // demonstrated in https://github.com/emilk/egui/blob/master/crates/egui_demo_lib/src/demo/paint_bezier.rs#L72
                    painter.rect_filled(rect, Rounding::default(), Color32::DARK_GRAY);

                    // Now ask Ui to allocate a rect that's the same as the one we just painted on,
                    // and set the cursor to the start of that region.
                    if ui
                        .allocate_ui_at_rect(rect, |ui| {
                            ui.allocate_response(ui.available_size(), Sense::click())
                        })
                        .inner
                        .clicked()
                    {
                        eprintln!("space #1 clicked");
                    }

                    if ui
                        .allocate_ui_at_rect(rect, |ui| {
                            ui.add(Label::new(
                                "I'm a widget being drawn on top of a painted surface!",
                            ));
                            ui.button("#1")
                        })
                        .inner
                        .clicked()
                    {
                        eprintln!("button #1 (passed to thing #1) clicked");
                    };

                    if ui
                        .allocate_ui_at_rect(rect, |ui| {
                            ui.label("I'm writing over everything");
                            ui.separator();
                            ui.button("#2")
                        })
                        .inner
                        .clicked()
                    {
                        eprintln!("button #2 (passed to thing #2) clicked");
                    }
                });
            });
    }
}
impl eframe::App for Explorer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.dnd.reset();
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

    let factory = register_factory_entities(EntityFactory::default());
    if FACTORY.set(factory).is_err() {
        return Err(anyhow!("Couldn't initialize EntityFactory"));
    }

    if let Err(e) = eframe::run_native(
        Explorer::APP_NAME,
        options,
        Box::new(|cc| Box::new(Explorer::new(cc))),
    ) {
        Err(anyhow!("eframe::run_native(): {:?}", e))
    } else {
        Ok(())
    }
}
