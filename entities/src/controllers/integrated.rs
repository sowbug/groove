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

/// Tempo is a u8 that ranges from 60..=240
#[derive(Clone, Debug)]
struct TempoValue(u8);
impl From<f32> for TempoValue {
    fn from(value: f32) -> Self {
        Self((value * 180.0).floor() as u8 + 60)
    }
}
impl Into<f32> for TempoValue {
    fn into(self) -> f32 {
        ((self.0 as f32) - 60.0) / 180.0
    }
}

/// Percentage is a u8 that ranges from 0..=100
#[derive(Clone, Debug)]
struct Percentage(u8);
impl From<u8> for Percentage {
    fn from(value: u8) -> Self {
        Self(value)
    }
}
impl From<f32> for Percentage {
    fn from(value: f32) -> Self {
        Self((value * 100.0) as u8)
    }
}
impl Into<f32> for Percentage {
    fn into(self) -> f32 {
        (self.0 as f32) / 100.0
    }
}

#[derive(Debug, Default, PartialEq)]
enum IntegratedEngineState {
    #[default]
    Idle,
    Playing,
}

#[derive(Debug)]
struct IntegratedEngine {
    volume: u8,

    swing: Percentage,
    tempo: Tempo,
    tempo_override: Option<TempoValue>,

    a: Percentage,
    b: Percentage,

    active_pattern: u8,
    patterns: [Pattern; 16],

    active_sound: u8,

    state: IntegratedEngineState,
}
impl Default for IntegratedEngine {
    fn default() -> Self {
        Self {
            volume: 7, // half
            swing: Percentage::from(0),
            tempo: Tempo::Disco,
            tempo_override: None,
            a: Percentage::from(0.5),
            b: Percentage::from(0.5),

            active_pattern: 0,
            patterns: [Pattern::default(); 16],
            active_sound: 0,

            state: IntegratedEngineState::Idle,
        }
    }
}
impl IntegratedEngine {
    pub fn a(&self) -> &Percentage {
        &self.a
    }

    pub fn b(&self) -> &Percentage {
        &self.b
    }

    pub fn set_a(&mut self, a: Percentage) {
        self.a = a;
    }

    pub fn set_b(&mut self, b: Percentage) {
        self.b = b;
    }

    pub fn swing(&self) -> &Percentage {
        &self.swing
    }

    pub fn set_swing(&mut self, swing: Percentage) {
        self.swing = swing;
    }

    pub fn volume(&self) -> u8 {
        self.volume
    }

    pub fn set_volume(&mut self, volume: u8) {
        self.volume = volume;
    }

    #[allow(dead_code)]
    pub fn tempo(&self) -> Option<Tempo> {
        if self.tempo_override.is_some() {
            None
        } else {
            Some(self.tempo)
        }
    }

    pub fn tempo_by_value(&self) -> TempoValue {
        if let Some(tempo) = &self.tempo_override {
            tempo.clone()
        } else {
            Self::tempo_to_value(self.tempo)
        }
    }

    pub fn set_tempo_by_value(&mut self, value: TempoValue) {
        if let Some(tempo) = Self::value_to_tempo(&value) {
            self.tempo = tempo;
            self.tempo_override = None;
        } else {
            self.tempo_override = Some(value);
        }
    }

    pub fn set_tempo_by_name(&mut self, tempo: Tempo) {
        self.tempo = tempo;
    }

    pub fn advance_tempo(&mut self) {
        self.set_tempo_by_name(match self.tempo {
            Tempo::HipHop => Tempo::Disco,
            Tempo::Disco => Tempo::Techno,
            Tempo::Techno => Tempo::HipHop,
        });
        self.tempo_override = None;
    }

    pub fn tempo_to_value(tempo: Tempo) -> TempoValue {
        TempoValue(match tempo {
            Tempo::HipHop => 80,
            Tempo::Disco => 120,
            Tempo::Techno => 160,
        })
    }

    fn value_to_tempo(value: &TempoValue) -> Option<Tempo> {
        match value.0 {
            80 => Some(Tempo::HipHop),
            120 => Some(Tempo::Disco),
            160 => Some(Tempo::Techno),
            _ => None,
        }
    }

    fn state(&self) -> &IntegratedEngineState {
        &self.state
    }

    fn set_state(&mut self, state: IntegratedEngineState) {
        self.state = state;
    }

    fn active_pattern(&self) -> u8 {
        self.active_pattern
    }

    fn set_active_pattern(&mut self, active_pattern: u8) {
        self.active_pattern = active_pattern;
    }

    fn is_pattern_active(&self, pattern: u8) -> bool {
        self.active_pattern == pattern
    }

    fn copy_active_pattern_to(&mut self, number: u8) {
        self.patterns[number as usize] = self.patterns[self.active_pattern() as usize];
    }

    // Assumes active pattern and active sound
    fn is_sound_selected(&self, index: u8) -> bool {
        self.patterns[self.active_pattern() as usize].is_sound_selected(self.active_sound(), index)
    }

    fn active_sound(&self) -> u8 {
        self.active_sound
    }

    fn set_active_sound(&mut self, active_sound: u8) {
        self.active_sound = active_sound;
    }

    fn pattern(&self, index: u8) -> &Pattern {
        &self.patterns[index as usize]
    }

    fn pattern_mut(&mut self, index: u8) -> &mut Pattern {
        &mut self.patterns[index as usize]
    }

    fn clear_pattern(&mut self, arg: u8) {
        self.patterns[arg as usize].clear();
    }

    fn clear_active_pattern(&mut self) {
        self.patterns[self.active_pattern() as usize].clear();
    }

    fn toggle_sound_at_step(&mut self, step_index: u8) {
        let active_sound = self.active_sound();
        let active_pattern = self.active_pattern();
        self.pattern_mut(active_pattern)
            .toggle_sound_at_step(active_sound, step_index);
    }
}
impl Performs for IntegratedEngine {
    fn play(&mut self) {
        self.set_state(IntegratedEngineState::Playing);
    }

    fn stop(&mut self) {
        self.set_state(IntegratedEngineState::Idle);
    }

    fn skip_to_start(&mut self) {
        self.play();
    }

    fn is_performing(&self) -> bool {
        self.state() == &IntegratedEngineState::Playing
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum ButtonState {
    #[default]
    Idle,
    Held,
    Blinking,
    Indicated,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum RenderState {
    #[default]
    Normal, // No mode active
    Sound,   // press a pad to select that sound
    Pattern, // press a pad to select that pattern
    Bpm,     // adjust swing/bpm with knobs
    Solo,    // during play, toggle solo play for a pad to copy
    Fx,      // press a pad to punch in effect
    Write,   // during play, change sound params with knobs **over time**
    Copy,    // hold write + pattern, press pad to copy active to that slot
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
    #[cfg_attr(feature = "serialization", serde(skip))]
    engine: IntegratedEngine,

    #[params]
    clock: Clock,

    arrangements: Vec<u8>,

    #[cfg_attr(feature = "serialization", serde(skip))]
    value: StereoSample,

    #[cfg_attr(feature = "serialization", serde(skip))]
    render_state: RenderState,

    #[cfg_attr(feature = "serialization", serde(skip))]
    blink_counter: u8,

    #[cfg_attr(feature = "serialization", serde(skip))]
    write_mode: bool,

    #[cfg_attr(feature = "serialization", serde(skip))]
    sound_solo_states: [bool; 16],
}
impl IsController for Integrated {}
impl IsInstrument for Integrated {}
impl Performs for Integrated {
    fn play(&mut self) {
        self.clock.seek(0);
        self.engine.play();
    }

    fn stop(&mut self) {
        self.engine.stop();
    }

    fn skip_to_start(&mut self) {
        self.engine.skip_to_start();
    }

    fn is_performing(&self) -> bool {
        self.engine.is_performing()
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
        let e = IntegratedEngine::default();
        Self {
            uid: Default::default(),
            clock: Clock::new_with(&ClockParams {
                bpm: e.tempo_by_value().0 as ParameterType,
                midi_ticks_per_second: 960,
                time_signature: TimeSignatureParams { top: 4, bottom: 4 },
            }),
            engine: Default::default(),

            arrangements: vec![u8::MAX; 128],

            value: Default::default(),
            render_state: Default::default(),
            blink_counter: Default::default(),
            write_mode: Default::default(),
            sound_solo_states: Default::default(),
        }
    }
}
impl Integrated {
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
                self.engine.set_active_sound(number);
                eprintln!("selected sound {}", self.engine.active_sound());
            }
            RenderState::Pattern => {
                self.engine.set_active_pattern(number);
                eprintln!("selected pattern {}", self.engine.active_pattern());
            }
            RenderState::Bpm => {
                self.engine.set_volume(number);
                eprintln!("volume {}", self.engine.volume());
            }
            RenderState::Solo => self.toggle_solo(number),
            RenderState::Fx => self.punch_effect(number),
            RenderState::Write => todo!(),
            RenderState::Copy => self.engine.copy_active_pattern_to(number),
        }
    }

    fn handle_play_click(&mut self) {
        if self.engine.state() == &IntegratedEngineState::Playing {
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
        self.engine.advance_tempo();
        self.update_bpm();
        eprintln!("BPM is {}", self.clock.bpm());
    }

    fn reset_render_state(&mut self) {
        self.render_state = RenderState::Normal;
    }

    fn update_bpm(&mut self) {
        self.clock.set_bpm(self.engine.tempo_by_value().0 as f64);
    }

    fn handle_solo_click(&mut self) {
        todo!()
    }

    fn handle_fx_click(&mut self) {
        todo!()
    }

    fn handle_write_click(&mut self) {
        self.write_mode = !self.write_mode;
    }

    fn change_render_state(&mut self, new_state: RenderState) {
        if self.render_state == new_state {
            self.render_state = RenderState::Normal;
        } else {
            self.render_state = new_state
        }
        eprintln!("New render state: ")
    }

    fn toggle_solo(&mut self, number: u8) {
        self.sound_solo_states[number as usize] = !self.sound_solo_states[number as usize];
        eprintln!("toggled solo for {}", number);
    }

    fn punch_effect(&self, number: u8) {
        todo!()
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
struct Pattern {
    steps: [Step; 16],
}
impl Default for Pattern {
    fn default() -> Self {
        Self {
            steps: [
                Step::new_with([
                    true, false, true, false, true, false, true, false, true, false, true, false,
                    true, false, true, false,
                ]),
                Step::new_with([
                    false, true, false, true, false, true, false, true, false, true, false, true,
                    false, true, false, true,
                ]),
                Step::new_with([
                    true, true, false, false, true, true, false, false, true, true, false, false,
                    true, true, false, false,
                ]),
                Step::new_with([
                    false, false, true, true, false, false, true, true, false, false, true, true,
                    false, false, true, true,
                ]),
                Step::new_with([
                    true, false, true, false, true, false, true, false, true, false, true, false,
                    true, false, true, false,
                ]),
                Step::new_with([
                    false, true, false, true, false, true, false, true, false, true, false, true,
                    false, true, false, true,
                ]),
                Step::new_with([
                    true, true, false, false, true, true, false, false, true, true, false, false,
                    true, true, false, false,
                ]),
                Step::new_with([
                    false, false, true, true, false, false, true, true, false, false, true, true,
                    false, false, true, true,
                ]),
                Step::new_with([
                    true, false, true, false, true, false, true, false, true, false, true, false,
                    true, false, true, false,
                ]),
                Step::new_with([
                    false, true, false, true, false, true, false, true, false, true, false, true,
                    false, true, false, true,
                ]),
                Step::new_with([
                    true, true, false, false, true, true, false, false, true, true, false, false,
                    true, true, false, false,
                ]),
                Step::new_with([
                    false, false, true, true, false, false, true, true, false, false, true, true,
                    false, false, true, true,
                ]),
                Step::new_with([
                    true, false, true, false, true, false, true, false, true, false, true, false,
                    true, false, true, false,
                ]),
                Step::new_with([
                    false, true, false, true, false, true, false, true, false, true, false, true,
                    false, true, false, true,
                ]),
                Step::new_with([
                    true, true, false, false, true, true, false, false, true, true, false, false,
                    true, true, false, false,
                ]),
                Step::new_with([
                    false, false, true, true, false, false, true, true, false, false, true, true,
                    false, false, true, true,
                ]),
            ],
        }
    }
}
impl Pattern {
    pub fn steps(&self) -> &[Step; 16] {
        &self.steps
    }
    pub fn step(&self, index: u8) -> &Step {
        &self.steps[index as usize]
    }
    pub fn step_mut(&mut self, index: u8) -> &mut Step {
        &mut self.steps[index as usize]
    }
    fn is_sound_selected(&self, sound: u8, index: u8) -> bool {
        self.steps[index as usize].is_sound_set(sound)
    }
    fn clear(&mut self) {
        for note in &mut self.steps {
            note.clear();
        }
    }
    fn is_clear(&self) -> bool {
        self.steps().iter().all(|n| n.is_clear())
    }

    fn toggle_sound_at_step(&mut self, sound: u8, step: u8) {
        self.step_mut(step).toggle_sound(sound);
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
struct Step {
    sounds: [bool; 16],
}
impl Default for Step {
    fn default() -> Self {
        Self {
            sounds: [false; 16],
        }
    }
}
impl Step {
    fn new_with(active_sounds: [bool; 16]) -> Self {
        Self {
            sounds: active_sounds,
        }
    }
    fn is_sound_set(&self, index: u8) -> bool {
        self.sounds[index as usize]
    }
    fn set_sound(&mut self, index: u8, is_set: bool) {
        self.sounds[index as usize] = is_set;
    }
    fn sounds(&self) -> &[bool; 16] {
        &self.sounds
    }
    fn clear(&mut self) {
        self.sounds = [false; 16];
    }
    fn is_clear(&self) -> bool {
        self.sounds.iter().all(|s| !s)
    }
    fn toggle_sound(&mut self, index: u8) {
        self.set_sound(index, !self.is_sound_set(index));
    }
}

#[cfg(feature = "egui-framework")]
mod gui {
    use super::{
        ButtonState, Integrated, IntegratedEngineState, Percentage, RenderState, TempoValue,
    };
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
                        self.blink_counter = (self.blink_counter + 1) % 4;
                        if self.blink_counter >= 2 {
                            Color32::RED
                        } else {
                            Color32::DARK_RED
                        }
                    }
                    ButtonState::Indicated => Color32::DARK_RED,
                }
            };
            ui.add_sized(cell_size, Button::new(label).fill(color))
        }

        fn handle_button_click(&mut self, button: &ButtonLabel, pad_index: u8) {
            match *button {
                ButtonLabel::Sound => self.handle_sound_click(),
                ButtonLabel::Pattern => self.handle_pattern_click(),
                ButtonLabel::Bpm => self.handle_bpm_click(),
                ButtonLabel::A => panic!(),
                ButtonLabel::B => panic!(),
                ButtonLabel::Solo => self.handle_solo_click(),
                ButtonLabel::Fx => self.handle_fx_click(),
                ButtonLabel::Play => self.handle_play_click(),
                ButtonLabel::Write => self.handle_write_click(),
                _ => {
                    self.handle_pad_click(pad_index);
                }
            }
        }

        fn handle_second_button_click(&mut self, button: ButtonLabel) {
            match button {
                ButtonLabel::Sound => self.change_render_state(RenderState::Sound),
                ButtonLabel::Pattern => {
                    if self.render_state == RenderState::Write {
                        self.change_render_state(RenderState::Copy);
                    } else {
                        self.change_render_state(RenderState::Pattern);
                    }
                }
                ButtonLabel::Bpm => self.change_render_state(RenderState::Bpm),
                ButtonLabel::Solo => self.change_render_state(RenderState::Solo),
                ButtonLabel::Fx => self.change_render_state(RenderState::Fx),
                ButtonLabel::Write => {
                    if self.render_state == RenderState::Pattern {
                        self.change_render_state(RenderState::Copy);
                    } else {
                        self.change_render_state(RenderState::Write)
                    }
                }
                _ => {}
            }
        }
    }

    impl Shows for Integrated {
        fn show(&mut self, ui: &mut eframe::egui::Ui) {
            let highlighted_button = if self.engine.state() == &IntegratedEngineState::Playing {
                Some((((self.clock.beats() * 4.0).floor() as i32) % 16) as u8)
            } else {
                None
            };
            ui.set_min_size(Vec2::new(320.0, 560.0)); // 1.75 aspect ratio
            ui.add_space(64.0);
            ui.add(
                SegmentedDisplayWidget::sixteen_segment(&format!(
                    "W: {}",
                    if self.write_mode { "+" } else { "-" }
                ))
                .digit_height(14.0),
            );
            ui.add(
                SegmentedDisplayWidget::sixteen_segment(&format!(
                    "A {:<3} B {:<3} SW {:<3} BPM {:<3}",
                    self.engine.a().0,
                    self.engine.b().0,
                    self.engine.swing().0,
                    self.engine.tempo_by_value().0,
                ))
                .digit_height(14.0),
            );
            ui.add(SegmentedDisplayWidget::sixteen_segment("MUSIC").digit_height(72.0));
            ui.add_space(16.0);
            Grid::new(ui.next_auto_id()).num_columns(5).show(ui, |ui| {
                let labels = vec![
                    "sound", "pattern", "bpm", "A", "B", "1", "2", "3", "4", "solo", "5", "6", "7",
                    "8", "FX", "9", "10", "11", "12", "play", "13", "14", "15", "16", "write",
                ];
                let button_to_pad_index = vec![
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
                    let pad_index = button_to_pad_index[index];
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
                                self.engine.swing().clone().into()
                            } else {
                                self.engine.a().clone().into()
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
                                        | RenderState::Write
                                        | RenderState::Copy => {
                                            self.engine.set_a(Percentage::from(value))
                                        }
                                        RenderState::Sound => {
                                            // nothing
                                        }
                                        RenderState::Bpm => {
                                            self.engine.set_swing(Percentage::from(value))
                                        }
                                    }
                                };
                            });
                        }
                        ButtonLabel::B => {
                            ui.set_min_size(cell_size);
                            let mut value = if self.render_state == RenderState::Bpm {
                                self.engine.tempo_by_value().into()
                            } else {
                                self.engine.b().clone().into()
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
                                        | RenderState::Write
                                        | RenderState::Copy => {
                                            self.engine.set_b(Percentage::from(value))
                                        }
                                        RenderState::Sound => {
                                            // nothing
                                        }
                                        RenderState::Bpm => {
                                            self.engine.set_tempo_by_value(TempoValue::from(value))
                                        }
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
                                    | ButtonLabel::Pad16 => match self.render_state {
                                        RenderState::Normal => ButtonState::Idle,
                                        RenderState::Pattern => {
                                            if self.engine.is_pattern_active(pad_index) {
                                                ButtonState::Blinking
                                            } else {
                                                // TODO: bright if in anywhere in the chain, dim otherwise
                                                ButtonState::Idle
                                            }
                                        }
                                        RenderState::Sound => {
                                            if self.engine.is_sound_selected(pad_index) {
                                                ButtonState::Indicated
                                            } else {
                                                ButtonState::Idle
                                            }
                                        }
                                        RenderState::Bpm => {
                                            if pad_index <= self.engine.volume() {
                                                ButtonState::Indicated
                                            } else {
                                                ButtonState::Idle
                                            }
                                        }
                                        RenderState::Solo => ButtonState::Idle,
                                        RenderState::Fx => ButtonState::Idle,
                                        RenderState::Write => ButtonState::Idle,
                                        RenderState::Copy => ButtonState::Idle,
                                    },
                                    ButtonLabel::A => ButtonState::Idle,
                                    ButtonLabel::B => ButtonState::Idle,
                                }
                            };
                            let response =
                                self.add_named_button(ui, label, button_state, is_highlighted);
                            if response.clicked() {
                                self.handle_button_click(&button, pad_index);
                            }
                            if response.clicked_by(eframe::egui::PointerButton::Secondary) {
                                self.handle_second_button_click(button);
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

#[cfg(test)]
mod tests {
    use super::IntegratedEngine;
    use crate::controllers::integrated::{Percentage, Step, Tempo, TempoValue};
    use groove_core::traits::Performs;

    #[test]
    fn volume() {
        let mut e = IntegratedEngine::default();

        assert_eq!(e.volume(), 7, "should start out at 7");
        e.set_volume(0);
        assert_eq!(e.volume(), 0, "set volume should work");
        e.set_volume(15);
        assert_eq!(e.volume(), 15, "set volume should work");
    }

    #[test]
    fn tempo() {
        let mut e = IntegratedEngine::default();

        assert_eq!(e.tempo_by_value().0, 120, "should start out as 120");
        assert_eq!(e.tempo(), Some(Tempo::Disco), "should start out as disco");
        e.advance_tempo();
        assert_eq!(e.tempo(), Some(Tempo::Techno), "techno follows disco");
        assert_eq!(e.tempo_by_value().0, 160, "techno is 160");
        e.advance_tempo();
        assert_eq!(e.tempo(), Some(Tempo::HipHop), "hiphop follows techno");
        assert_eq!(e.tempo_by_value().0, 80, "hiphop is 80");

        e.set_tempo_by_value(TempoValue(120));
        assert_eq!(e.tempo(), Some(Tempo::Disco), "120 sets disco");
        e.set_tempo_by_value(TempoValue(160));
        assert_eq!(e.tempo(), Some(Tempo::Techno), "160 sets techno");
        e.set_tempo_by_value(TempoValue(80));
        assert_eq!(e.tempo(), Some(Tempo::HipHop), "80 sets hiphop");

        e.set_tempo_by_value(TempoValue(121));
        assert_eq!(e.tempo(), None, "other value sets no named tempo");
        assert_eq!(e.tempo_by_value().0, 121, "setting respects other value");
        e.advance_tempo();
        assert_eq!(
            e.tempo(),
            Some(Tempo::Disco),
            "prior named tempo is restored when advance follows other"
        );

        e.set_tempo_by_value(TempoValue::from(0.0));
        assert_eq!(e.tempo_by_value().0, 60, "conversion from f32 works");
        e.set_tempo_by_value(TempoValue::from(1.0));
        assert_eq!(e.tempo_by_value().0, 240, "conversion from f32 works");
        e.set_tempo_by_value(TempoValue::from(0.5));
        assert_eq!(
            e.tempo_by_value().0,
            (240 - 60) / 2 + 60,
            "conversion from f32 works"
        );
    }

    #[test]
    fn percentage_type() {
        let p = Percentage(0);
        assert_eq!(p.0, 0);
        let p = Percentage(100);
        assert_eq!(p.0, 100);
        let p = Percentage(50);
        assert_eq!(p.0, 50);
        let p = Percentage::from(0.0);
        assert_eq!(p.0, 0);
        let p = Percentage::from(1.0);
        assert_eq!(p.0, 100);
        let p = Percentage::from(0.5);
        assert_eq!(p.0, 50);

        let mut pp: f32 = p.into();
        assert_eq!(pp, 0.5);
        let p = Percentage::from(1.0);
        pp = p.into();
        assert_eq!(pp, 1.0);
    }

    #[test]
    fn swing() {
        let mut e = IntegratedEngine::default();

        assert_eq!(e.swing().0, 0, "swing should start out at 0");
        e.set_swing(Percentage(50));
        assert_eq!(e.swing().0, 50, "set_swing should work");
    }

    #[test]
    fn a_and_b() {
        let mut e = IntegratedEngine::default();

        assert_eq!(e.a().0, 50, "should start out at 50");
        assert_eq!(e.b().0, 50, "should start out at 50");

        e.set_a(Percentage(40));
        assert_eq!(e.a().0, 40, "set_a should work");
        e.set_b(Percentage(100));
        assert_eq!(e.b().0, 100, "set_b should work");
    }

    #[test]
    fn pattern_crud() {
        let mut e = IntegratedEngine::default();

        assert_eq!(e.active_pattern(), 0, "first pattern active at startup");
        assert!(
            !e.pattern(e.active_pattern()).is_clear(),
            "active pattern is non-empty"
        );
        e.clear_active_pattern();
        assert!(
            e.pattern(e.active_pattern()).is_clear(),
            "after clear(), active pattern is empty"
        );

        // Make Pattern #2 different
        e.pattern_mut(2).toggle_sound_at_step(0, 0);

        assert!(
            *e.pattern(1) != *e.pattern(2),
            "second and third patterns are initially different"
        );
        e.set_active_pattern(1);
        e.copy_active_pattern_to(2);
        assert!(
            *e.pattern(1) == *e.pattern(2),
            "after copy-active operation, second and third patterns are identical"
        );

        e.set_active_pattern(15);
        assert_eq!(e.active_pattern(), 15, "set active pattern works");
    }

    #[test]
    fn play_stop() {
        let mut e = IntegratedEngine::default();

        assert_eq!(e.is_performing(), false, "not performing at startup");
        e.play();
        assert_eq!(e.is_performing(), true, "is performing after play()");
        e.stop();
        assert_eq!(e.is_performing(), false, "is not performing after stop()");
        e.skip_to_start();
        assert_eq!(
            e.is_performing(),
            true,
            "resumes performing after skip_to_start()"
        );
    }
}
