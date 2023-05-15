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
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
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
impl Default for Percentage {
    fn default() -> Self {
        Self(50)
    }
}
impl Percentage {
    fn midway() -> Self {
        Self(50)
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

    chains: [u8; 128],
    chain_cursor: u8,

    active_sound: u8,

    state: IntegratedEngineState,

    solo_states: [bool; 16],

    // Which chain slot we're currently playing
    pb_chain_index: u8,

    // Which pattern we're currently playing (different from active pattern, which is used for editing)
    pb_pattern_index: u8,

    // Which step we're currently playing in the pattern
    pb_step_index: u8,
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
            chains: [u8::MAX; 128],
            chain_cursor: 0,

            active_sound: 0,

            state: IntegratedEngineState::Idle,

            solo_states: Default::default(),

            pb_chain_index: Default::default(),
            pb_pattern_index: Default::default(),
            pb_step_index: Default::default(),
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
        self.patterns[self.active_pattern() as usize].sound_set_at_step(self.active_sound(), index)
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
        let a = self.a().clone();
        let b = self.b().clone();
        self.pattern_mut(active_pattern)
            .toggle_sound_at_step(active_sound, step_index, &a, &b);
    }

    fn is_solo(&self, index: u8) -> bool {
        self.solo_states[index as usize]
    }

    fn toggle_solo(&mut self, index: u8) {
        self.solo_states[index as usize] = !self.solo_states[index as usize];
    }

    fn chain_pattern(&mut self, number: u8) {
        self.chains[self.chain_cursor as usize] = number;
        self.chain_cursor += 1;
        if self.chain_cursor >= self.chains.len() as u8 {
            self.chain_cursor = self.chains.len() as u8 - 1;
        }
    }

    fn chain_active_pattern(&mut self) {
        self.chain_pattern(self.active_pattern());
    }

    fn chain_cursor(&self) -> u8 {
        self.chain_cursor
    }

    fn reset_chain_cursor(&mut self) {
        self.chain_cursor = 0;
        self.chains = [u8::MAX; 128];
    }

    fn chains(&self, index: u8) -> u8 {
        self.chains[index as usize]
    }

    fn next_step(&mut self) -> &Step {
        if self.pb_chain_index == u8::MAX {
            // We're about to start the song. We know pattern/step were already set to zero.
            self.pb_chain_index = 0;
        } else {
            self.pb_step_index += 1;
            if self.pb_step_index == 16 {
                self.pb_step_index = 0;
                self.pb_chain_index += 1;
                if self.pb_chain_index == 128 {
                    self.pb_chain_index = 127;
                }
                if self.chains(self.pb_chain_index) == u8::MAX {
                    // "the entire sequence then repeats"
                    self.pb_chain_index = 0;
                }
                self.pb_pattern_index = self.chains(self.pb_chain_index);
                if self.pb_pattern_index == u8::MAX {
                    // The user hasn't set up any chained patterns. We'll just
                    // keep recycling the active one. This is a little more
                    // elegant than initializing the chain memory with the
                    // currently active pattern.
                    self.pb_pattern_index = self.active_pattern();
                }
            }
        }
        let pattern = self.pattern(self.pb_pattern_index);
        pattern.step(self.pb_step_index)
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
        self.pb_chain_index = u8::MAX;
        self.pb_pattern_index = 0;
        self.pb_step_index = 0;
        self.play();
    }

    fn is_performing(&self) -> bool {
        self.state() == &IntegratedEngineState::Playing
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum UiState {
    #[default]
    Normal, // press a pad to play that sound
    Sound,   // press a pad to select that sound
    Pattern, // press a pad to select that pattern
    Bpm,     // adjust swing/bpm with knobs
    Solo,    // during play, toggle solo play for a pad to copy
    Fx,      // press a pad to punch in effect
    //    Write,   // during play, change sound params with knobs **over time**
    Copy, // hold write + pattern, press pad to copy active to that slot
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

    #[cfg_attr(feature = "serialization", serde(skip))]
    value: StereoSample,

    #[cfg_attr(feature = "serialization", serde(skip))]
    ui_state: UiState,

    #[cfg_attr(feature = "serialization", serde(skip))]
    blink_counter: u8,

    #[cfg_attr(feature = "serialization", serde(skip))]
    write_mode: bool,

    /// Whether the pattern is used anywhere in the current chain.
    #[cfg_attr(feature = "serialization", serde(skip))]
    pattern_usages: [bool; 16],

    /// The last step we handled during playback.
    #[cfg_attr(feature = "serialization", serde(skip))]
    last_handled_step: usize,
}
impl IsController for Integrated {}
impl IsInstrument for Integrated {}
impl Performs for Integrated {
    fn play(&mut self) {
        // We don't have resume, so play always skips to start.
        self.skip_to_start();
        self.engine.play();
    }

    fn stop(&mut self) {
        self.engine.stop();
    }

    fn skip_to_start(&mut self) {
        self.clock.seek(0);
        self.last_handled_step = usize::MAX;
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
        self.handle_tick();
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

            value: Default::default(),
            ui_state: Default::default(),
            blink_counter: Default::default(),
            write_mode: Default::default(),
            pattern_usages: Default::default(),
            last_handled_step: Default::default(),
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
        match self.ui_state {
            UiState::Normal => {
                if self.write_mode {
                    self.engine.toggle_sound_at_step(number);
                } else {
                    eprintln!("demoing sound {}", number);
                }
            }
            UiState::Sound => {
                self.engine.set_active_sound(number);
                eprintln!("selected sound {}", self.engine.active_sound());
            }
            UiState::Pattern => {
                if self.write_mode {
                    self.engine.copy_active_pattern_to(number);
                    eprintln!(
                        "copied active pattern {} to {}",
                        self.engine.active_pattern(),
                        number
                    );
                } else {
                    // The active pattern changes only on the first pattern
                    // selection. This is how the UI consistently shows that the
                    // active pattern is the next one to be played.
                    if self.engine.chain_cursor == 0 {
                        self.engine.set_active_pattern(number);
                    }
                    // We save this so the debug output handles both 0 and 127
                    // easily.
                    let current_cursor = self.engine.chain_cursor;
                    self.engine.chain_pattern(number);

                    // TODO: check behavior when overwriting causes a pattern to
                    // vanish from the chain. The way we're doing it now is
                    // expensive to handle correctly.
                    self.pattern_usages[number as usize] = true;

                    eprintln!(
                        "pattern {} active, pattern {} added at position {} to chain",
                        self.engine.active_pattern(),
                        number,
                        current_cursor,
                    );
                }
            }
            UiState::Bpm => {
                self.engine.set_volume(number);
                eprintln!("volume {}", self.engine.volume());
            }
            UiState::Solo => self.engine.toggle_solo(number),
            UiState::Fx => self.punch_effect(number),
            UiState::Copy => self.engine.copy_active_pattern_to(number),
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
        if self.ui_state == UiState::Bpm {
            self.reset_render_state();
        }
        self.engine.advance_tempo();
        self.update_bpm();
        eprintln!("BPM is {}", self.clock.bpm());
    }

    fn reset_render_state(&mut self) {
        self.ui_state = UiState::Normal;
    }

    fn update_bpm(&mut self) {
        self.clock.set_bpm(self.engine.tempo_by_value().0 as f64);
    }

    fn handle_solo_click(&mut self) {
        self.change_ui_state(UiState::Solo);
    }

    fn handle_fx_click(&mut self) {
        todo!()
    }

    fn handle_write_click(&mut self) {
        self.write_mode = !self.write_mode;
    }

    fn change_ui_state(&mut self, new_state: UiState) {
        if self.ui_state == new_state {
            self.ui_state = UiState::Normal;
        } else {
            self.ui_state = new_state
        }
        eprintln!("New render state: ")
    }

    fn punch_effect(&self, number: u8) {
        todo!()
    }

    fn handle_knob_b_change(&mut self, value: f32) {
        match self.ui_state {
            UiState::Normal | UiState::Pattern | UiState::Solo | UiState::Fx | UiState::Copy => {
                self.engine.set_b(Percentage::from(value))
            }
            UiState::Sound => {
                // nothing
            }
            UiState::Bpm => self.engine.set_tempo_by_value(TempoValue::from(value)),
        }
    }

    fn handle_knob_a_change(&mut self, value: f32) {
        match self.ui_state {
            UiState::Normal | UiState::Pattern | UiState::Solo | UiState::Fx | UiState::Copy => {
                self.engine.set_a(Percentage::from(value))
            }
            UiState::Sound => {
                // nothing
            }
            UiState::Bpm => self.engine.set_swing(Percentage::from(value)),
        }
    }

    fn reset_pattern_usages(&mut self) {
        self.pattern_usages = Default::default();
    }

    // How many steps we are into the song.
    fn total_steps(&self) -> usize {
        ((self.clock.beats() * 4.0).floor() as i32) as usize
    }

    // How many steps we are into the current pattern.
    fn current_step(&self) -> u8 {
        (self.total_steps() % 16) as u8
    }

    fn handle_tick(&mut self) {
        if self.is_performing() {
            // We use this only as a marker whether it's time to do work. We don't use it as a song cursor.
            let total_steps = self.total_steps();
            if self.last_handled_step == total_steps {
                return;
            }
            self.last_handled_step = total_steps;
            let step = self.engine.next_step();
            eprintln!("{} {} {:?}", total_steps, total_steps % 16, &step);
        }
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
    fn sound_set_at_step(&self, sound: u8, index: u8) -> bool {
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

    fn toggle_sound_at_step(&mut self, sound: u8, step: u8, a: &Percentage, b: &Percentage) {
        self.step_mut(step).toggle_sound(sound, a, b);
    }

    fn set_sound_at_step(
        &mut self,
        sound: u8,
        step: u8,
        is_set: bool,
        a: Percentage,
        b: Percentage,
    ) {
        let step = self.step_mut(step);
        step.set_sound(sound, is_set, &a, &b);
    }

    fn a_at_step(&self, sound: u8, step: u8) -> Percentage {
        let step = self.step(step);
        step.a[sound as usize]
    }

    fn b_at_step(&self, sound: u8, step: u8) -> Percentage {
        let step = self.step(step);
        step.b[sound as usize]
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
struct Step {
    sounds: [bool; 16],
    a: [Percentage; 16],
    b: [Percentage; 16],
}
impl Default for Step {
    fn default() -> Self {
        Self {
            sounds: [false; 16],
            a: [Percentage::midway(); 16],
            b: [Percentage::midway(); 16],
        }
    }
}
impl Step {
    fn new_with(active_sounds: [bool; 16]) -> Self {
        Self {
            sounds: active_sounds,
            a: [Percentage::default(); 16],
            b: [Percentage::default(); 16],
        }
    }
    fn is_sound_set(&self, index: u8) -> bool {
        self.sounds[index as usize]
    }
    fn set_sound(&mut self, index: u8, is_set: bool, a: &Percentage, b: &Percentage) {
        let index = index as usize;
        self.sounds[index] = is_set;
        self.a[index] = *a;
        self.b[index] = *b;
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
    fn toggle_sound(&mut self, index: u8, a: &Percentage, b: &Percentage) {
        self.set_sound(index, !self.is_sound_set(index), a, b);
    }
}

#[cfg(feature = "egui-framework")]
mod gui {
    use super::{Integrated, IntegratedEngineState, UiState};
    use eframe::{
        egui::{Button, Grid, Response, Sense},
        epaint::{Color32, Stroke, Vec2},
    };
    use egui_extras_xt::displays::SegmentedDisplayWidget;
    use groove_core::traits::gui::Shows;
    use strum_macros::FromRepr;

    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    enum ButtonState {
        #[default]
        Idle, // Off
        Held,      // This is only for modifier buttons like sound/pattern/bpm
        Indicated, // on but dim
        Active,    // on and bright
        Blinking,  // on and attention-getting
    }

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
        const BUTTON_INDEX_TO_PAD_INDEX: [u8; 25] = [
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
        const BUTTON_LABELS: [&'static str; 25] = [
            "sound", "pattern", "bpm", "A", "B", "1", "2", "3", "4", "solo", "5", "6", "7", "8",
            "FX", "9", "10", "11", "12", "play", "13", "14", "15", "16", "write",
        ];
        const CELL_SIZE: Vec2 = Vec2::new(60.0, 60.0);
        const LED_SIZE: Vec2 = Vec2::splat(5.0);

        fn create_button(
            &mut self,
            ui: &mut eframe::egui::Ui,
            label: &str,
            state: ButtonState,
            is_highlighted: bool,
            has_led: bool,
        ) -> Response {
            let button_color = if state == ButtonState::Held {
                Color32::DARK_BLUE
            } else {
                Color32::GRAY
            };
            let led_color = match state {
                ButtonState::Idle => {
                    if is_highlighted {
                        Color32::RED
                    } else {
                        Color32::BLACK
                    }
                }
                ButtonState::Held => Color32::BLACK,
                ButtonState::Indicated => Color32::RED,
                ButtonState::Active => Color32::LIGHT_RED,
                ButtonState::Blinking => {
                    self.blink_counter = (self.blink_counter + 1) % 4;
                    if self.blink_counter >= 2 {
                        Color32::LIGHT_RED
                    } else {
                        Color32::RED
                    }
                }
            };
            ui.vertical_centered(|ui| {
                let (rect, _response) = ui.allocate_exact_size(Self::LED_SIZE, Sense::hover());
                if has_led {
                    ui.painter().rect(
                        rect,
                        ui.style().visuals.noninteractive().rounding,
                        led_color,
                        Stroke::NONE,
                    );
                }
                ui.add_sized(Self::CELL_SIZE, Button::new(label).fill(button_color))
            })
            .inner
        }

        // TODO: I can't get this knob to be the same size as the other buttons,
        // so the second button is not correctly centered on the grid.
        fn create_knob(ui: &mut eframe::egui::Ui, value: &mut f32) -> Response {
            ui.vertical_centered_justified(|ui| {
                // This is clumsy to try to keep all the widgets evenly spaced
                let (_rect, _response) = ui.allocate_exact_size(Self::LED_SIZE, Sense::hover());
                ui.add_sized(
                    Self::CELL_SIZE,
                    egui_extras_xt::knobs::AudioKnob::new(value)
                        .animated(true)
                        .range(0.0..=1.0),
                )
            })
            .inner
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
                ButtonLabel::Sound => self.change_ui_state(UiState::Sound),
                ButtonLabel::Pattern => {
                    if self.ui_state != UiState::Pattern {
                        self.engine.reset_chain_cursor();
                        self.reset_pattern_usages();
                    }
                    self.change_ui_state(UiState::Pattern);
                }
                ButtonLabel::Bpm => self.change_ui_state(UiState::Bpm),
                ButtonLabel::Solo => self.change_ui_state(UiState::Solo),
                ButtonLabel::Fx => self.change_ui_state(UiState::Fx),
                ButtonLabel::Write => {
                    self.handle_write_click();
                }
                _ => {}
            }
        }

        fn create_dashboard(&self, ui: &mut eframe::egui::Ui) {
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
        }

        fn create_knob_a(&mut self, ui: &mut eframe::egui::Ui) {
            ui.set_min_size(Self::CELL_SIZE);
            let mut value = if self.ui_state == UiState::Bpm {
                self.engine.swing().clone().into()
            } else {
                self.engine.a().clone().into()
            };
            if Self::create_knob(ui, &mut value).changed() {
                self.handle_knob_a_change(value);
            }
        }

        fn create_knob_b(&mut self, ui: &mut eframe::egui::Ui) {
            ui.set_min_size(Self::CELL_SIZE);
            let mut value = if self.ui_state == UiState::Bpm {
                self.engine.tempo_by_value().into()
            } else {
                self.engine.b().clone().into()
            };
            if Self::create_knob(ui, &mut value).changed() {
                self.handle_knob_b_change(value);
            }
        }

        fn calculate_button_state(&self, button: &ButtonLabel, pad_index: u8) -> ButtonState {
            match *button {
                ButtonLabel::Sound => {
                    if self.ui_state == UiState::Sound {
                        ButtonState::Held
                    } else {
                        ButtonState::Idle
                    }
                }
                ButtonLabel::Pattern => {
                    if self.ui_state == UiState::Pattern {
                        ButtonState::Held
                    } else {
                        ButtonState::Idle
                    }
                }
                ButtonLabel::Bpm => {
                    if self.ui_state == UiState::Bpm {
                        ButtonState::Held
                    } else {
                        ButtonState::Idle
                    }
                }
                ButtonLabel::Fx => {
                    if self.ui_state == UiState::Fx {
                        ButtonState::Held
                    } else {
                        ButtonState::Idle
                    }
                }
                ButtonLabel::Solo => {
                    if self.ui_state == UiState::Solo {
                        ButtonState::Held
                    } else {
                        ButtonState::Idle
                    }
                }
                ButtonLabel::Write => {
                    if self.write_mode {
                        ButtonState::Held
                    } else {
                        ButtonState::Idle
                    }
                }
                ButtonLabel::Play => ButtonState::Idle,
                ButtonLabel::A => ButtonState::Idle,
                ButtonLabel::B => ButtonState::Idle,
                _ => match self.ui_state {
                    UiState::Normal | UiState::Sound => {
                        if self.engine.is_sound_selected(pad_index) {
                            ButtonState::Indicated
                        } else {
                            ButtonState::Idle
                        }
                    }
                    UiState::Pattern => {
                        if self.engine.is_pattern_active(pad_index) {
                            ButtonState::Blinking
                        } else {
                            if self.pattern_usages[pad_index as usize] {
                                ButtonState::Indicated
                            } else {
                                ButtonState::Idle
                            }
                        }
                    }
                    UiState::Bpm => {
                        if pad_index <= self.engine.volume() {
                            ButtonState::Indicated
                        } else {
                            ButtonState::Idle
                        }
                    }
                    UiState::Solo => {
                        if self.engine.is_solo(pad_index) {
                            ButtonState::Indicated
                        } else {
                            ButtonState::Idle
                        }
                    }
                    UiState::Fx => ButtonState::Idle,
                    UiState::Copy => ButtonState::Idle,
                },
            }
        }
    }

    impl Shows for Integrated {
        fn show(&mut self, ui: &mut eframe::egui::Ui) {
            let highlighted_button = if self.engine.state() == &IntegratedEngineState::Playing {
                Some(self.current_step())
            } else {
                None
            };
            ui.set_min_size(Vec2::new(320.0, 560.0)); // 1.75 aspect ratio
            ui.add_space(64.0);
            self.create_dashboard(ui);
            ui.add(SegmentedDisplayWidget::sixteen_segment("MUSIC").digit_height(72.0));
            ui.add_space(16.0);
            Grid::new(ui.next_auto_id()).num_columns(5).show(ui, |ui| {
                for (index, label) in Self::BUTTON_LABELS.iter().enumerate() {
                    let pad_index = Self::BUTTON_INDEX_TO_PAD_INDEX[index];
                    let is_highlighted = if let Some(hb) = highlighted_button {
                        pad_index == hb
                    } else {
                        false
                    };
                    let button = ButtonLabel::from_repr(index).unwrap();
                    match button {
                        ButtonLabel::A => {
                            self.create_knob_a(ui);
                        }
                        ButtonLabel::B => {
                            self.create_knob_b(ui);
                        }
                        _ => {
                            let button_state = self.calculate_button_state(&button, pad_index);
                            let response = self.create_button(
                                ui,
                                label,
                                button_state,
                                is_highlighted,
                                pad_index != u8::MAX,
                            );
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
    use crate::controllers::integrated::{Percentage, Tempo, TempoValue};
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
        e.pattern_mut(2)
            .toggle_sound_at_step(0, 0, &Percentage(33), &Percentage(66));

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

        e.pattern_mut(2)
            .set_sound_at_step(13, 15, false, Percentage(0), Percentage(0));
        assert!(!e.pattern(2).sound_set_at_step(13, 15));
        assert_ne!(e.pattern(2).a_at_step(13, 15).0, 42);
        assert_ne!(e.pattern(2).b_at_step(13, 15).0, 84);
        e.pattern_mut(2)
            .set_sound_at_step(13, 15, true, Percentage(42), Percentage(84));
        assert!(e.pattern(2).sound_set_at_step(13, 15));
        assert_eq!(e.pattern(2).a_at_step(13, 15).0, 42);
        assert_eq!(e.pattern(2).b_at_step(13, 15).0, 84);

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

    #[test]
    fn solo() {
        let mut e = IntegratedEngine::default();
        for index in 0..16 {
            assert!(!e.is_solo(index), "no solos at startup");
        }
        e.toggle_solo(7);
        assert!(e.is_solo(7), "toggle_solo() works");
    }

    #[test]
    fn chaining() {
        let mut e = IntegratedEngine::default();

        assert_eq!(e.chain_cursor(), 0, "chain cursor at zero at startup");

        e.set_active_pattern(7);
        e.chain_active_pattern();
        assert_eq!(e.chain_cursor(), 1, "chaining active should work");

        e.reset_chain_cursor();
        assert_eq!(e.chain_cursor(), 0, "chain cursor at zero after reset");

        for _ in 0..128 {
            e.chain_active_pattern();
        }
        for i in 0..128 {
            assert_eq!(e.chains(i), 7, "successive chaining should work");
        }
        assert_eq!(e.chain_cursor(), 127, "chaining should work up to maximum");

        e.set_active_pattern(8);
        e.chain_active_pattern();
        assert_eq!(
            e.chains(127),
            8,
            "when at chain capacity, last one should overwrite itself"
        );

        e.reset_chain_cursor();
        assert_eq!(e.chain_cursor(), 0, "chain cursor at zero after reset");

        for i in 0..128 {
            assert_eq!(
                e.chains(i),
                u8::MAX,
                "resetting chain cursor also overwrites slots"
            );
        }
    }
}
