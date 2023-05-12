use crate::EntityMessage;
use groove_core::{
    time::{Clock, ClockParams},
    traits::{
        Generates, HandlesMidi, IsController, IsInstrument, Performs, Resets, Ticks,
        TicksWithMessages,
    },
    StereoSample,
};
use groove_proc_macros::{Control, Params, Uid};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[derive(Control, Params, Debug, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Integrated {
    uid: usize,

    clock: Clock,

    #[cfg_attr(feature = "serialization", serde(skip))]
    value: StereoSample,

    patterns: Vec<Pattern>,
}
impl IsController for Integrated {}
impl IsInstrument for Integrated {}
impl Performs for Integrated {
    fn play(&mut self) {
        todo!()
    }

    fn stop(&mut self) {
        todo!()
    }

    fn skip_to_start(&mut self) {
        todo!()
    }
}
impl HandlesMidi for Integrated {
    fn handle_midi_message(
        &mut self,
        _message: &midly::MidiMessage,
    ) -> Option<Vec<(groove_core::midi::MidiChannel, midly::MidiMessage)>> {
        None
    }
}
impl Ticks for Integrated {
    fn tick(&mut self, _tick_count: usize) {
        self.value = StereoSample::SILENCE;
    }
}
impl TicksWithMessages for Integrated {
    type Message = EntityMessage;

    fn tick(&mut self, tick_count: usize) -> (Option<Vec<Self::Message>>, usize) {
        self.clock.tick(tick_count);
        (None, tick_count)
    }
}
impl Resets for Integrated {
    fn reset(&mut self, sample_rate: usize) {
        self.clock.reset(sample_rate);
    }
}
impl Generates<StereoSample> for Integrated {
    fn value(&self) -> StereoSample {
        self.value
    }

    fn batch_values(&mut self, _values: &mut [StereoSample]) {
        todo!()
    }
}
impl Integrated {
    pub fn new_with(_params: &IntegratedParams) -> Self {
        Self {
            uid: Default::default(),
            clock: Clock::new_with(&ClockParams::default()),
            value: Default::default(),
            patterns: Default::default(),
        }
    }
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
struct Pattern {
    notes: Vec<Note>,
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
struct Note {
    key: u8,
}

#[cfg(feature = "egui-framework")]
mod gui {
    use super::Integrated;
    use eframe::{egui::Button, epaint::Vec2};
    use egui_extras::Size;
    use egui_extras_xt::displays::SegmentedDisplayWidget;
    use egui_grid::{Grid, GridBuilder};
    use groove_core::traits::gui::Shows;

    impl Integrated {
        fn add_named_button(&mut self, grid: &mut Grid, label: &str) {
            let cell_size = Vec2::new(60.0, 60.0);
            grid.cell(|ui| {
                if ui.add_sized(cell_size, Button::new(label)).clicked() {
                    eprintln!("clicked {}", label);
                };
            });
        }
    }

    impl Shows for Integrated {
        fn show(&mut self, ui: &mut eframe::egui::Ui) {
            ui.set_min_size(Vec2::new(320.0, 560.0)); // 1.75 aspect ratio
            ui.add_space(64.0);
            ui.add(SegmentedDisplayWidget::sixteen_segment("MUSIC").digit_height(72.0));
            ui.add_space(16.0);
            let mut g = GridBuilder::new().spacing(3.0, 3.0);
            for _ in 0..5 {
                g = g.new_row(Size::exact(72.0)).cells(Size::exact(60.0), 5);
            }
            g.show(ui, |mut grid| {
                let cell_size = Vec2::new(60.0, 60.0);
                self.add_named_button(&mut grid, "sound");
                self.add_named_button(&mut grid, "pattern");
                self.add_named_button(&mut grid, "bpm");
                grid.cell(|ui| {
                    ui.set_min_size(cell_size);
                    let mut value = 0.0;
                    ui.centered_and_justified(|ui| {
                        if ui
                            .add(
                                egui_extras_xt::knobs::AngleKnob::new(&mut value)
                                    .diameter(cell_size.x / 2.0)
                                    .animated(true),
                            )
                            .changed()
                        {
                            eprintln!("a is {}", value);
                        };
                    });
                });
                grid.cell(|ui| {
                    ui.set_min_size(cell_size);
                    let mut value = 0.0;
                    ui.centered_and_justified(|ui| {
                        if ui
                            .add(
                                egui_extras_xt::knobs::AngleKnob::new(&mut value)
                                    .diameter(cell_size.x / 2.0)
                                    .animated(true),
                            )
                            .changed()
                        {
                            eprintln!("b is {}", value);
                        };
                    });
                });
                let labels = vec![
                    "1", "2", "3", "4", "solo", "5", "6", "7", "8", "FX", "9", "10", "11", "12",
                    "play", "13", "14", "15", "16", "write",
                ];
                for label in labels {
                    self.add_named_button(&mut grid, label);
                }
            });
            // Frame::none()
            //     .stroke(Stroke::new(1.0, Color32::YELLOW))
            //     .fill(Color32::DARK_GRAY)
            //     .show(ui, |ui| );
        }
    }
}
