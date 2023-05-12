use crate::EntityMessage;
use groove_core::{
    time::{Clock, ClockParams, TimeSignatureParams},
    traits::{
        Generates, HandlesMidi, IsController, IsInstrument, Performs, Resets, Ticks,
        TicksWithMessages,
    },
    ParameterType, StereoSample,
};
use groove_proc_macros::{Control, Params, Uid};

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, PartialEq)]
enum EngineState {
    #[default]
    Idle,
    Playing,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum ButtonState {
    #[default]
    Idle,
    Held,
    Blinking,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum RenderState {
    #[default]
    Normal,
    Sound,
    Pattern,
    Bpm,
    Solo,
    Fx,
    Write,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub enum Tempo {
    HipHop,
    #[default]
    Disco,
    Techno,
}

#[derive(Control, Params, Debug, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Integrated {
    uid: usize,

    #[params]
    clock: Clock,

    tempo: Tempo,
    tempo_override: Option<u8>,

    a: f32,
    b: f32,
    swing: u8,

    patterns: [Pattern; 16],

    #[cfg_attr(feature = "serialization", serde(skip))]
    value: StereoSample,

    #[cfg_attr(feature = "serialization", serde(skip))]
    engine_state: EngineState,

    #[cfg_attr(feature = "serialization", serde(skip))]
    render_state: RenderState,

    #[cfg_attr(feature = "serialization", serde(skip))]
    button_state: [ButtonState; Self::BUTTON_COUNT],

    #[cfg_attr(feature = "serialization", serde(skip))]
    active_pattern: u8,

    #[cfg_attr(feature = "serialization", serde(skip))]
    active_sound: u8,

    #[cfg_attr(feature = "serialization", serde(skip))]
    blink_is_on: bool,
}
impl IsController for Integrated {}
impl IsInstrument for Integrated {}
impl Performs for Integrated {
    fn play(&mut self) {
        self.clock.seek(0);
        self.engine_state = EngineState::Playing;
    }

    fn stop(&mut self) {
        self.engine_state = EngineState::Idle;
    }

    fn skip_to_start(&mut self) {
        self.play();
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
impl Default for Integrated {
    fn default() -> Self {
        Self {
            uid: Default::default(),
            clock: Clock::new_with(&ClockParams {
                bpm: Self::bpm_values(Tempo::default()) as ParameterType,
                midi_ticks_per_second: 960,
                time_signature: TimeSignatureParams { top: 4, bottom: 4 },
            }),
            tempo: Default::default(),
            tempo_override: Default::default(),

            a: 0.5,
            b: 0.5,
            swing: 0,
            patterns: Default::default(),

            value: Default::default(),
            engine_state: Default::default(),
            render_state: Default::default(),
            button_state: [ButtonState::Idle; Self::BUTTON_COUNT],
            active_pattern: Default::default(),
            active_sound: Default::default(),
            blink_is_on: Default::default(),
        }
    }
}
impl Integrated {
    const BUTTON_COUNT: usize = 25;
    pub fn new_with(params: &IntegratedParams) -> Self {
        Self {
            clock: Clock::new_with(params.clock()),
            ..Default::default()
        }
    }

    fn handle_pad_click(&mut self, number: u8) {
        match self.render_state {
            RenderState::Normal => {
                eprintln!("demoing sound {}", number);
            }
            RenderState::Sound => {
                self.active_sound = number;
                eprintln!("selected sound {}", self.active_sound);
            }
            RenderState::Pattern => {
                self.active_pattern = number;
                eprintln!("selected pattern {}", self.active_pattern);
            }
            RenderState::Bpm => todo!(),
            RenderState::Solo => todo!(),
            RenderState::Fx => todo!(),
            RenderState::Write => todo!(),
        }
    }

    fn handle_play_click(&mut self) {
        if self.engine_state == EngineState::Playing {
            self.stop()
        } else {
            self.play()
        }
    }

    fn handle_sound_click(&mut self) {
        eprintln!("does nothing")
    }

    fn handle_pattern_click(&mut self) {
        eprintln!("does nothing")
    }

    fn handle_bpm_click(&mut self) {
        if self.render_state == RenderState::Bpm {
            self.reset_render_state();
        }
        self.tempo_override = None;
        self.tempo = match self.tempo {
            Tempo::HipHop => Tempo::Disco,
            Tempo::Disco => Tempo::Techno,
            Tempo::Techno => Tempo::HipHop,
        };
        self.update_bpm();
        eprintln!("BPM is {}", self.clock.bpm());
    }

    fn reset_render_state(&mut self) {
        self.render_state = RenderState::Normal;
    }

    fn update_bpm(&mut self) {
        self.clock.set_bpm(self.bpm() as f64);
    }

    fn bpm(&self) -> u8 {
        if let Some(bpm) = self.tempo_override {
            bpm
        } else {
            Self::bpm_values(self.tempo)
        }
    }

    fn bpm_values(tempo: Tempo) -> u8 {
        match tempo {
            Tempo::HipHop => 80,
            Tempo::Disco => 120,
            Tempo::Techno => 160,
        }
    }

    fn handle_solo_click(&mut self) {
        todo!()
    }

    fn handle_fx_click(&mut self) {
        todo!()
    }

    fn handle_write_click(&mut self) {
        todo!()
    }

    fn change_render_state(&mut self, new_state: RenderState) {
        if self.render_state == new_state {
            self.render_state = RenderState::Normal;
        } else {
            self.render_state = new_state
        }
    }

    pub fn a(&self) -> f32 {
        self.a
    }

    pub fn b(&self) -> f32 {
        self.b
    }

    pub fn set_a(&mut self, a: f32) {
        self.a = a;
    }

    pub fn set_b(&mut self, b: f32) {
        self.b = b;
    }

    fn tempo_for_knob_as_normal(&self) -> f32 {
        let bpm = self.bpm() as f32;
        (bpm - 60.0) / 180.0
    }

    fn set_tempo_from_normal(&mut self, value: f32) {
        self.tempo_override = Some((value * 180.0).floor() as u8 + 60);
        eprintln!("set tempo manually to {}", self.tempo_override.unwrap());
    }

    fn set_swing_from_knob(&mut self, value: f32) {
        self.swing = (value * 100.0).floor() as u8
    }

    pub fn swing(&self) -> u8 {
        self.swing
    }
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
struct Pattern {
    notes: [Note; 16],
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
struct Note {
    key: u8,
}

#[cfg(feature = "egui-framework")]
mod gui {
    use super::{ButtonState, EngineState, Integrated, RenderState};
    use eframe::{
        egui::{Button, Grid, Response},
        epaint::{Color32, Vec2},
    };
    use egui_extras_xt::displays::SegmentedDisplayWidget;
    use groove_core::traits::gui::Shows;
    use strum_macros::FromRepr;

    #[derive(FromRepr, PartialEq)]
    enum ButtonLabel {
        Sound,
        Pattern,
        Bpm,
        A,
        B,
        Pad1,
        Pad2,
        Pad3,
        Pad4,
        Solo,
        Pad5,
        Pad6,
        Pad7,
        Pad8,
        Fx,
        Pad9,
        Pad10,
        Pad11,
        Pad12,
        Play,
        Pad13,
        Pad14,
        Pad15,
        Pad16,
        Write,
    }

    impl Integrated {
        fn add_named_button(
            &mut self,
            ui: &mut eframe::egui::Ui,
            label: &str,
            state: ButtonState,
            is_highlighted: bool,
        ) -> Response {
            let cell_size = Vec2::new(60.0, 60.0);
            let color = if is_highlighted {
                Color32::LIGHT_YELLOW
            } else {
                match state {
                    ButtonState::Idle => Color32::DARK_GRAY,
                    ButtonState::Held => Color32::GRAY,
                    ButtonState::Blinking => {
                        self.blink_is_on = !self.blink_is_on;
                        if self.blink_is_on {
                            Color32::RED
                        } else {
                            Color32::DARK_RED
                        }
                    }
                }
            };
            ui.add_sized(cell_size, Button::new(label).fill(color))
        }
    }

    impl Shows for Integrated {
        fn show(&mut self, ui: &mut eframe::egui::Ui) {
            let highlighted_button = if self.engine_state == EngineState::Playing {
                Some((((self.clock.beats() * 4.0).floor() as i32) % 16) as u8)
            } else {
                None
            };
            ui.set_min_size(Vec2::new(320.0, 560.0)); // 1.75 aspect ratio
            ui.add_space(64.0);
            Grid::new(ui.next_auto_id())
                .num_columns(4)
                .min_col_width(80.0)
                .max_col_width(80.0)
                .spacing(Vec2 { x: 0.0, y: 0.0 })
                .show(ui, |ui| {
                    ui.label(format!("A: {:<3}", (self.a() * 100.0) as u8));
                    ui.label(format!("B: {:<3}", (self.b() * 100.0) as u8));
                    ui.label(format!("Swing: {:<3}", self.swing()));
                    ui.label(format!("BPM: {:<3}", self.bpm()));
                });
            ui.add(SegmentedDisplayWidget::sixteen_segment("MUSIC").digit_height(72.0));
            ui.add_space(16.0);
            Grid::new(ui.next_auto_id()).num_columns(5).show(ui, |ui| {
                let labels = vec![
                    "sound", "pattern", "bpm", "A", "B", "1", "2", "3", "4", "solo", "5", "6", "7",
                    "8", "FX", "9", "10", "11", "12", "play", "13", "14", "15", "16", "write",
                ];
                let button_index = vec![
                    u8::MAX,
                    u8::MAX,
                    u8::MAX,
                    u8::MAX,
                    u8::MAX,
                    0,
                    1,
                    2,
                    3,
                    u8::MAX,
                    4,
                    5,
                    6,
                    7,
                    u8::MAX,
                    8,
                    9,
                    10,
                    11,
                    u8::MAX,
                    12,
                    13,
                    14,
                    15,
                    u8::MAX,
                ];
                let cell_size = Vec2::new(60.0, 60.0);
                for (index, label) in labels.iter().enumerate() {
                    let pad_index = button_index[index];
                    let is_highlighted = if let Some(hb) = highlighted_button {
                        pad_index == hb
                    } else {
                        false
                    };
                    let button = ButtonLabel::from_repr(index).unwrap();
                    match button {
                        ButtonLabel::A => {
                            ui.set_min_size(cell_size);
                            let mut value = if self.render_state == RenderState::Bpm {
                                self.swing() as f32 / 100.0
                            } else {
                                self.a()
                            };
                            ui.centered_and_justified(|ui| {
                                if ui
                                    .add(
                                        egui_extras_xt::knobs::AudioKnob::new(&mut value)
                                            .diameter(cell_size.x / 2.0)
                                            .animated(true)
                                            .range(0.0..=1.0),
                                    )
                                    .changed()
                                {
                                    match self.render_state {
                                        RenderState::Normal
                                        | RenderState::Pattern
                                        | RenderState::Solo
                                        | RenderState::Fx
                                        | RenderState::Write => self.set_a(value),
                                        RenderState::Sound => {
                                            // nothing
                                        }
                                        RenderState::Bpm => self.set_swing_from_knob(value),
                                    }
                                };
                            });
                        }
                        ButtonLabel::B => {
                            ui.set_min_size(cell_size);
                            let mut value = if self.render_state == RenderState::Bpm {
                                self.tempo_for_knob_as_normal()
                            } else {
                                self.b()
                            };
                            ui.centered_and_justified(|ui| {
                                if ui
                                    .add(
                                        egui_extras_xt::knobs::AudioKnob::new(&mut value)
                                            .diameter(cell_size.x / 2.0)
                                            .animated(true)
                                            .range(0.0..=1.0),
                                    )
                                    .changed()
                                {
                                    match self.render_state {
                                        RenderState::Normal
                                        | RenderState::Pattern
                                        | RenderState::Solo
                                        | RenderState::Fx
                                        | RenderState::Write => self.set_b(value),
                                        RenderState::Sound => {
                                            // nothing
                                        }
                                        RenderState::Bpm => self.set_tempo_from_normal(value),
                                    }
                                };
                            });
                        }
                        _ => {
                            let button_state = {
                                match button {
                                    ButtonLabel::Sound => {
                                        if self.render_state == RenderState::Sound {
                                            ButtonState::Held
                                        } else {
                                            ButtonState::Idle
                                        }
                                    }
                                    ButtonLabel::Pattern => {
                                        if self.render_state == RenderState::Pattern {
                                            ButtonState::Held
                                        } else {
                                            ButtonState::Idle
                                        }
                                    }
                                    ButtonLabel::Bpm => {
                                        if self.render_state == RenderState::Bpm {
                                            ButtonState::Held
                                        } else {
                                            ButtonState::Idle
                                        }
                                    }
                                    ButtonLabel::Fx => {
                                        if self.render_state == RenderState::Fx {
                                            ButtonState::Held
                                        } else {
                                            ButtonState::Idle
                                        }
                                    }
                                    ButtonLabel::Solo => {
                                        if self.render_state == RenderState::Solo {
                                            ButtonState::Held
                                        } else {
                                            ButtonState::Idle
                                        }
                                    }
                                    ButtonLabel::Write => {
                                        if self.render_state == RenderState::Write {
                                            ButtonState::Held
                                        } else {
                                            ButtonState::Idle
                                        }
                                    }
                                    ButtonLabel::Play => ButtonState::Idle,
                                    ButtonLabel::Pad1
                                    | ButtonLabel::Pad2
                                    | ButtonLabel::Pad3
                                    | ButtonLabel::Pad4
                                    | ButtonLabel::Pad5
                                    | ButtonLabel::Pad6
                                    | ButtonLabel::Pad7
                                    | ButtonLabel::Pad8
                                    | ButtonLabel::Pad9
                                    | ButtonLabel::Pad10
                                    | ButtonLabel::Pad11
                                    | ButtonLabel::Pad12
                                    | ButtonLabel::Pad13
                                    | ButtonLabel::Pad14
                                    | ButtonLabel::Pad15
                                    | ButtonLabel::Pad16 => {
                                        if self.render_state == RenderState::Pattern {
                                            if self.active_pattern == pad_index {
                                                ButtonState::Blinking
                                            } else {
                                                ButtonState::Idle
                                            }
                                        } else {
                                            ButtonState::Idle
                                        }
                                    }
                                    ButtonLabel::A => ButtonState::Idle,
                                    ButtonLabel::B => ButtonState::Idle,
                                }
                            };
                            let response =
                                self.add_named_button(ui, label, button_state, is_highlighted);
                            if response.clicked() {
                                match button {
                                    ButtonLabel::Sound => self.handle_sound_click(),
                                    ButtonLabel::Pattern => self.handle_pattern_click(),
                                    ButtonLabel::Bpm => self.handle_bpm_click(),
                                    ButtonLabel::A => panic!(),
                                    ButtonLabel::B => panic!(),
                                    ButtonLabel::Pad1
                                    | ButtonLabel::Pad2
                                    | ButtonLabel::Pad3
                                    | ButtonLabel::Pad4
                                    | ButtonLabel::Pad5
                                    | ButtonLabel::Pad6
                                    | ButtonLabel::Pad7
                                    | ButtonLabel::Pad8
                                    | ButtonLabel::Pad9
                                    | ButtonLabel::Pad10
                                    | ButtonLabel::Pad11
                                    | ButtonLabel::Pad12
                                    | ButtonLabel::Pad13
                                    | ButtonLabel::Pad14
                                    | ButtonLabel::Pad15
                                    | ButtonLabel::Pad16 => {
                                        self.handle_pad_click(button_index[index]);
                                    }
                                    ButtonLabel::Solo => self.handle_solo_click(),
                                    ButtonLabel::Fx => self.handle_fx_click(),
                                    ButtonLabel::Play => self.handle_play_click(),
                                    ButtonLabel::Write => self.handle_write_click(),
                                }
                            }
                            if response.clicked_by(eframe::egui::PointerButton::Secondary) {
                                match button {
                                    ButtonLabel::Sound => {
                                        self.change_render_state(RenderState::Sound)
                                    }
                                    ButtonLabel::Pattern => {
                                        self.change_render_state(RenderState::Pattern)
                                    }
                                    ButtonLabel::Bpm => self.change_render_state(RenderState::Bpm),
                                    ButtonLabel::Solo => {
                                        self.change_render_state(RenderState::Solo)
                                    }
                                    ButtonLabel::Fx => self.change_render_state(RenderState::Fx),
                                    ButtonLabel::Write => {
                                        self.change_render_state(RenderState::Write)
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    if (index + 1) % 5 == 0 {
                        ui.end_row();
                    }
                }
            });
        }
    }
}
