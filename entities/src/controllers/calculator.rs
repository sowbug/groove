// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! Emulates a certain handheld music instrument that looks like a calculator.

// TODO
//
// - Swing
// - Effects
// - step multiplier
// - live record
// - should BPM be global?
// - a better LCD

use crate::instruments::{Sampler, SamplerVoice};
use ensnare::prelude::*;
use groove_core::{
    instruments::Synthesizer,
    midi::{note_to_frequency, MidiChannel, MidiMessage, MidiMessagesFn},
    time::{Clock, ClockParams, MusicalTime, PerfectTimeUnit, SampleRate, TimeSignatureParams},
    traits::{
        Configurable, ControlEventsFn, Controls, Generates, HandlesMidi, Serializable, Ticks,
    },
    voices::VoicePerNoteStore,
};
use groove_proc_macros::{Control, IsControllerInstrument, Params, Uid};
use groove_utils::Paths;
use std::{ops::Range, path::Path, sync::Arc};
use strum_macros::Display;

#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

/// Tempo is a u8 that ranges from 60..=240
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
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
    fn maximum() -> Self {
        Self(100)
    }
    fn minimum() -> Self {
        Self(0)
    }
}

#[derive(Debug, Default, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
enum EngineState {
    #[default]
    Idle,
    Playing,
}

#[derive(Debug)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
struct Chains {
    indexes: Vec<u8>,
}
impl Default for Chains {
    fn default() -> Self {
        let mut r = Self {
            indexes: Vec::with_capacity(128),
        };
        r.reset();
        r
    }
}
impl Chains {
    fn reset(&mut self) {
        self.indexes.clear();
    }

    fn add(&mut self, number: u8) {
        if self.indexes.len() < self.capacity() as usize {
            self.indexes.push(number);
        }
    }

    fn len(&self) -> u8 {
        self.indexes.len() as u8
    }

    fn capacity(&self) -> u8 {
        self.indexes.capacity() as u8
    }

    fn index(&self, offset: u8) -> u8 {
        if offset as usize >= self.indexes.len() {
            u8::MAX
        } else {
            self.indexes[offset as usize]
        }
    }
}

/// [Engine] contains the musical data other than the samples.
#[derive(Debug)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
struct Engine {
    swing: Percentage,
    tempo: Tempo,
    tempo_override: Option<TempoValue>,

    a: Percentage,
    b: Percentage,

    active_pattern: u8,
    patterns: [Pattern; 16],

    chains: Chains,

    active_sound: u8,

    state: EngineState,

    solo_states: [bool; 16],

    // Which chain slot we're currently playing
    pb_chain_index: u8,

    // Which pattern we're currently playing (different from active pattern, which is used for editing)
    pb_pattern_index: u8,

    // Which step we're currently playing in the pattern
    pb_step_index: u8,
}
impl Default for Engine {
    fn default() -> Self {
        Self {
            swing: Percentage::from(0),
            tempo: Tempo::Disco,
            tempo_override: None,
            a: Percentage::from(0.5),
            b: Percentage::from(0.5),

            active_pattern: 0,
            patterns: [Pattern::default(); 16],
            chains: Default::default(),

            active_sound: 0,

            state: EngineState::Idle,

            solo_states: Default::default(),

            pb_chain_index: Default::default(),
            pb_pattern_index: Default::default(),
            pb_step_index: Default::default(),
        }
    }
}
impl Engine {
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

    fn state(&self) -> &EngineState {
        &self.state
    }

    fn set_state(&mut self, state: EngineState) {
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
        self.patterns[self.active_pattern() as usize].is_active(self.active_sound(), index)
    }

    fn active_sound(&self) -> u8 {
        self.active_sound
    }

    fn set_active_sound(&mut self, sound: u8) {
        self.active_sound = sound;
    }

    fn pattern(&self, index: u8) -> &Pattern {
        &self.patterns[index as usize]
    }

    fn pattern_mut(&mut self, index: u8) -> &mut Pattern {
        &mut self.patterns[index as usize]
    }

    fn clear_pattern(&mut self, index: u8) {
        self.patterns[index as usize].clear();
    }

    fn clear_active_pattern(&mut self) {
        self.clear_pattern(self.active_pattern());
    }

    fn toggle_sound_at_step(&mut self, step_index: u8) {
        let active_sound = self.active_sound();
        let active_pattern = self.active_pattern();
        self.pattern_mut(active_pattern)
            .toggle_active(active_sound, step_index);
    }

    fn is_solo(&self, index: u8) -> bool {
        self.solo_states[index as usize]
    }

    fn toggle_solo(&mut self, index: u8) {
        self.solo_states[index as usize] = !self.solo_states[index as usize];
    }

    fn chain_pattern(&mut self, number: u8) {
        self.chains.add(number);
    }

    fn chain_cursor(&self) -> u8 {
        self.chains.len()
    }

    fn reset_chain_cursor(&mut self) {
        self.chains.reset();
    }

    fn next_step(&mut self) -> &Step {
        if self.pb_chain_index == u8::MAX {
            // We're about to start the song. We know pattern/step were already set to zero.
            self.pb_chain_index = 0;
        } else {
            self.pb_step_index += 1;
            if self.pb_step_index == 16 {
                self.pb_step_index = 0;
                if self.pb_chain_index < self.chains.capacity() - 1 {
                    self.pb_chain_index += 1;
                }
                if self.chains.index(self.pb_chain_index) == u8::MAX {
                    // "the entire sequence then repeats"
                    self.pb_chain_index = 0;
                }
                self.pb_pattern_index = self.chains.index(self.pb_chain_index);
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
impl Controls for Engine {
    fn update_time(&mut self, _range: &Range<MusicalTime>) {
        todo!()
    }

    fn work(&mut self, _control_events_fn: &mut ControlEventsFn) {
        todo!()
    }

    fn is_finished(&self) -> bool {
        todo!()
    }

    fn play(&mut self) {
        self.set_state(EngineState::Playing);
    }

    fn stop(&mut self) {
        self.set_state(EngineState::Idle);
    }

    fn skip_to_start(&mut self) {
        self.pb_chain_index = u8::MAX;
        self.pb_pattern_index = 0;
        self.pb_step_index = 0;
        self.play();
    }

    fn is_performing(&self) -> bool {
        self.state() == &EngineState::Playing
    }
    // This instrument is all about looping, so we ignore this section.
    fn set_loop(&mut self, _range: &Range<PerfectTimeUnit>) {}
    fn clear_loop(&mut self) {}

    fn set_loop_enabled(&mut self, _is_enabled: bool) {}
}
impl Configurable for Engine {
    fn sample_rate(&self) -> SampleRate {
        SampleRate::default()
    }

    fn update_sample_rate(&mut self, _sample_rate: SampleRate) {}

    fn update_tempo(&mut self, _tempo: groove_core::time::Tempo) {}

    fn update_time_signature(&mut self, _time_signature: groove_core::time::TimeSignature) {}
}

#[derive(Clone, Copy, Debug, Default, Display, PartialEq)]
enum UiState {
    #[default]
    Normal, // press a pad to play that sound
    Sound,   // press a pad to select that sound
    Pattern, // press a pad to select that pattern
    Bpm,     // adjust swing/bpm with knobs
    Solo,    // during play, toggle solo play for a pad to copy
    Fx,      // press a pad to punch in effect
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub enum Tempo {
    HipHop,
    #[default]
    Disco,
    Techno,
}

/// [Calculator] is the top-level musical instrument. It contains an [Engine]
/// that has the song data, as well as a sampler synth that can generate digital
/// audio. It draws the GUI and handles user input.
#[derive(Control, IsControllerInstrument, Params, Debug, Uid)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Calculator {
    /// Required for the [groove_core::traits::HasUid] trait.
    uid: groove_core::Uid,

    /// Keeps the music data (notes, sequences, tempo).
    engine: Engine,

    /// The final output volume, ranging 0..16.
    volume: u8,

    /// Timekeeper, maps audio-sample time to musical time.
    #[params]
    clock: Clock,

    /// Generates audio data.
    #[cfg_attr(feature = "serialization", serde(skip))]
    inner_synth: Synthesizer<SamplerVoice>,

    /// Which mode the UI is in.
    #[cfg_attr(feature = "serialization", serde(skip))]
    ui_state: UiState,

    /// Whether write is enabled.
    #[cfg_attr(feature = "serialization", serde(skip))]
    is_write_enabled: bool,

    /// Controls LED blinking.
    #[cfg_attr(feature = "serialization", serde(skip))]
    blink_counter: u8,

    /// Whether the pattern is used anywhere in the current chain. Used for
    /// chaining UI.
    #[cfg_attr(feature = "serialization", serde(skip))]
    pattern_usages: [bool; 16],

    /// The last step we handled during playback. Used to tell whether it's time
    /// to process a new step.
    #[cfg_attr(feature = "serialization", serde(skip))]
    last_handled_step: usize,
}
impl Serializable for Calculator {}
impl HandlesMidi for Calculator {
    fn handle_midi_message(
        &mut self,
        channel: MidiChannel,
        message: MidiMessage,
        midi_messages_fn: &mut MidiMessagesFn,
    ) {
        self.inner_synth
            .handle_midi_message(channel, message, midi_messages_fn)
    }
}
impl Ticks for Calculator {
    fn tick(&mut self, tick_count: usize) {
        self.inner_synth.tick(tick_count);
    }
}
impl Controls for Calculator {
    fn update_time(&mut self, _range: &Range<MusicalTime>) {
        todo!()
    }

    fn work(&mut self, _: &mut ControlEventsFn) {
        self.handle_tick();
    }

    fn is_finished(&self) -> bool {
        todo!()
    }

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

    // This instrument is all about looping, so we ignore this.
    fn set_loop(&mut self, _range: &Range<PerfectTimeUnit>) {}
    fn clear_loop(&mut self) {}
    fn set_loop_enabled(&mut self, _is_enabled: bool) {}

    fn is_performing(&self) -> bool {
        self.engine.is_performing()
    }
}
impl Configurable for Calculator {
    fn sample_rate(&self) -> SampleRate {
        self.clock.sample_rate()
    }
    fn update_sample_rate(&mut self, sample_rate: SampleRate) {
        self.clock.update_sample_rate(sample_rate);
    }
}
impl Generates<StereoSample> for Calculator {
    fn value(&self) -> StereoSample {
        self.inner_synth.value()
    }

    fn generate_batch_values(&mut self, values: &mut [StereoSample]) {
        self.inner_synth.generate_batch_values(values);
    }
}
impl Default for Calculator {
    fn default() -> Self {
        let e = Engine::default();
        Self {
            uid: Default::default(),
            engine: Default::default(),
            volume: 5,

            clock: Clock::new_with(&ClockParams {
                bpm: e.tempo_by_value().0 as ParameterType,
                midi_ticks_per_second: 960,
                time_signature: TimeSignatureParams { top: 4, bottom: 4 },
            }),
            inner_synth: Self::load_sampler_voices(),
            ui_state: Default::default(),
            blink_counter: Default::default(),
            is_write_enabled: Default::default(),
            pattern_usages: Default::default(),
            last_handled_step: Default::default(),
        }
    }
}
impl Calculator {
    pub fn new_with(params: &CalculatorParams) -> Self {
        Self {
            clock: Clock::new_with(params.clock()),
            ..Default::default()
        }
    }

    pub fn volume(&self) -> u8 {
        self.volume
    }

    pub fn set_volume(&mut self, volume: u8) {
        self.volume = volume;
    }

    fn handle_pad_click(&mut self, number: u8) {
        match self.ui_state {
            UiState::Normal => {
                if self.is_write_enabled {
                    self.engine.toggle_sound_at_step(number);
                } else {
                    self.trigger_note(number);
                }
            }
            UiState::Sound => {
                self.engine.set_active_sound(number);
                eprintln!("selected sound {}", self.engine.active_sound());
            }
            UiState::Pattern => {
                if self.is_write_enabled {
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
                    if self.engine.chain_cursor() == 0 {
                        self.engine.set_active_pattern(number);
                    }
                    // We save this so the debug output handles both 0 and 127
                    // easily.
                    let current_cursor = self.engine.chain_cursor();
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
                self.set_volume(number);
                eprintln!("volume {}", self.volume());
            }
            UiState::Solo => self.engine.toggle_solo(number),
            UiState::Fx => self.punch_effect(number),
        }
    }

    fn handle_play_click(&mut self) {
        if self.engine.state() == &EngineState::Playing {
            self.stop()
        } else {
            self.play()
        }
    }

    fn handle_sound_click(&mut self) {
        eprintln!("does nothing")
    }

    fn handle_pattern_click(&mut self) {
        if self.ui_state == UiState::Solo {
            self.engine.clear_active_pattern();
            eprintln!("cleared active pattern");
        } else {
            eprintln!("nothing");
        }
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
        self.is_write_enabled = !self.is_write_enabled;
    }

    fn change_ui_state(&mut self, new_state: UiState) {
        if self.ui_state == new_state {
            self.ui_state = UiState::Normal;
        } else {
            self.ui_state = new_state
        }
        eprintln!("New render state: {}", self.ui_state);
    }

    fn punch_effect(&self, _number: u8) {
        todo!()
    }

    fn handle_knob_b_change(&mut self, value: f32) {
        match self.ui_state {
            UiState::Normal | UiState::Pattern | UiState::Solo | UiState::Fx => {
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
            UiState::Normal | UiState::Pattern | UiState::Solo | UiState::Fx => {
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
        // TODO: update this to the new global clock in Controls
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
            let step = self.engine.next_step().clone(); // TODO: this is costly
            for i in 0..16 {
                if step.is_sound_active(i)
                    && (self.ui_state != UiState::Solo || self.engine.is_solo(i))
                {
                    self.trigger_note(i.into());
                }
            }
        }
    }

    fn trigger_note(&mut self, key: u8) {
        let key = key.into();
        let vel = 127.into();
        self.inner_synth.handle_midi_message(
            MidiChannel(2),
            midly::MidiMessage::NoteOff { key, vel },
            &mut |_, _| {},
        );
        self.inner_synth.handle_midi_message(
            MidiChannel(4),
            midly::MidiMessage::NoteOn { key, vel },
            &mut |_, _| {},
        );
    }

    fn load_sampler_voices() -> Synthesizer<SamplerVoice> {
        let samples = vec![
            "01-5-inch-floppy",
            "02-3.5-drive-eject",
            "03-3.5-floppy-read",
            "04-keyboard",
            "05-dot-matrix-printer",
            "06-joystick",
            "07-mouse-click",
            "08-toggle-switch",
            "09-bass-drum",
            "10-dtmf-tones",
            "11-hardsync-tone",
            "12-hardsync-noise",
            "13-ring-modulation",
            "14-bass",
            "15-glitch-fx",
            "16-noise-fx",
        ];
        //            "04-keyboard-2",
        //            "08-toggle-switch-2",

        let sample_dirs = vec!["pocket-calculator-24"];

        let paths = Paths::default();

        let voice_store = VoicePerNoteStore::<SamplerVoice>::new_with_voices(
            samples.into_iter().enumerate().map(|(index, asset_name)| {
                let filename =
                    paths.build_sample(&sample_dirs, Path::new(&format!("{asset_name}.wav")));
                if let Ok(file) = paths.search_and_open(filename.as_path()) {
                    if let Ok(samples) = Sampler::read_samples_from_file(&file) {
                        (
                            (index as u8).into(),
                            SamplerVoice::new_with_samples(
                                Arc::new(samples),
                                note_to_frequency(index as u8),
                            ),
                        )
                    } else {
                        panic!()
                    }
                } else {
                    panic!()
                }
            }),
        );
        Synthesizer::<SamplerVoice>::new_with(Box::new(voice_store))
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
struct Pattern {
    steps: [Step; 16],
}
impl Default for Pattern {
    fn default() -> Self {
        let sound_patterns = vec![
            // 1 - 5.25 floppy
            vec![
                false, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false,
            ],
            // 2 - 3.5 eject
            vec![
                true, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false,
            ],
            // 3 - 3.5 floppy
            vec![
                false, false, false, false, true, false, false, false, true, false, false, false,
                true, false, true, false,
            ],
            // 4 - keyboard
            vec![
                false, false, false, false, false, false, true, false, false, false, false, false,
                false, false, false, false,
            ],
            // 5 - matrix printer
            vec![
                false, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false,
            ],
            // 6 - joystick
            vec![
                false, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false,
            ],
            // 7 - mouse click
            vec![
                false, false, false, false, false, false, false, false, false, true, false, true,
                false, false, false, false,
            ],
            // 8 - toggle switch
            vec![
                false, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false,
            ],
            // 9 - bass drum
            vec![
                false, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false,
            ],
            // 10 - pc beeper
            vec![
                false, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false,
            ],
            // 11 - hardsync tone
            vec![
                false, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false,
            ],
            // 12 - hardsync noise
            vec![
                false, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false,
            ],
            // 13 - ring modulation
            vec![
                false, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false,
            ],
            // 14 - bass
            vec![
                false, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false,
            ],
            // 15 - glitch fx
            vec![
                false, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false,
            ],
            // 16 - noise fx
            vec![
                false, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false,
            ],
        ];
        let mut r = Self {
            steps: [Step::default(); 16],
        };
        for i in 0..16 {
            let mut step = [false; 16];
            for j in 0..16 {
                step[j] = sound_patterns[j][i];
            }
            r.steps[i] = Step::new_with(step);
        }
        r
    }
}
impl Pattern {
    fn step(&self, step: u8) -> &Step {
        &self.steps[step as usize]
    }
    fn step_mut(&mut self, step: u8) -> &mut Step {
        &mut self.steps[step as usize]
    }
    fn is_active(&self, sound: u8, step: u8) -> bool {
        self.steps[step as usize].is_sound_active(sound)
    }
    #[allow(dead_code)]
    fn a(&self, sound: u8, step: u8) -> Percentage {
        let step = self.step(step);
        step.a[sound as usize]
    }
    #[allow(dead_code)]
    fn b(&self, sound: u8, step: u8) -> Percentage {
        let step = self.step(step);
        step.b[sound as usize]
    }
    fn clear(&mut self) {
        for note in &mut self.steps {
            note.clear();
        }
    }
    fn toggle_active(&mut self, sound: u8, step: u8) {
        self.step_mut(step).toggle_sound(sound);
    }
    #[allow(dead_code)]
    fn set_sound(&mut self, sound: u8, step: u8, is_active: bool) {
        self.step_mut(step).set_active(sound, is_active);
    }
    #[allow(dead_code)]
    fn set_a(&mut self, sound: u8, step: u8, a: &Percentage) {
        self.step_mut(step).set_a(sound, a);
    }
    #[allow(dead_code)]
    fn set_b(&mut self, sound: u8, step: u8, b: &Percentage) {
        self.step_mut(step).set_b(sound, b);
    }
    #[allow(dead_code)]
    fn set_all(&mut self, sound: u8, step: u8, is_set: bool, a: &Percentage, b: &Percentage) {
        self.set_sound(sound, step, is_set);
        self.set_a(sound, step, a);
        self.set_b(sound, step, b);
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
            a: [Percentage::maximum(); 16],
            b: [Percentage::minimum(); 16],
        }
    }
}
impl Step {
    fn new_with(sounds: [bool; 16]) -> Self {
        Self {
            sounds,
            a: [Percentage::default(); 16],
            b: [Percentage::default(); 16],
        }
    }
    fn is_sound_active(&self, sound: u8) -> bool {
        self.sounds[sound as usize]
    }
    #[allow(dead_code)]
    fn a(&self, sound: u8) -> Percentage {
        self.a[sound as usize]
    }
    #[allow(dead_code)]
    fn b(&self, sound: u8) -> Percentage {
        self.b[sound as usize]
    }
    fn set_active(&mut self, sound: u8, is_active: bool) {
        self.sounds[sound as usize] = is_active;
    }
    fn set_a(&mut self, sound: u8, a: &Percentage) {
        self.a[sound as usize] = *a;
    }
    fn set_b(&mut self, sound: u8, b: &Percentage) {
        self.b[sound as usize] = *b;
    }
    fn clear(&mut self) {
        self.sounds = [false; 16];
    }
    fn toggle_sound(&mut self, sound: u8) {
        self.set_active(sound, !self.is_sound_active(sound));
    }
}

#[cfg(feature = "egui-framework")]
mod gui {
    use super::{Calculator, EngineState, UiState};
    use eframe::{
        egui::{self, Button, Grid, Response, Sense, Ui},
        epaint::{Color32, Stroke, Vec2},
    };
    use egui_extras_xt::displays::SegmentedDisplayWidget;
    use groove_core::traits::gui::Displays;
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

    impl Calculator {
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
            ui: &mut Ui,
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
                ButtonState::Indicated => Color32::DARK_RED,
                ButtonState::Active => Color32::RED,
                ButtonState::Blinking => {
                    self.blink_counter = (self.blink_counter + 1) % 4;
                    if self.blink_counter >= 2 {
                        Color32::RED
                    } else {
                        Color32::DARK_RED
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
        fn create_knob(ui: &mut Ui, value: &mut f32) -> Response {
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

        fn create_dashboard(&self, ui: &mut Ui) {
            ui.add(
                SegmentedDisplayWidget::sixteen_segment(&format!(
                    "W: {}",
                    if self.is_write_enabled { "+" } else { "-" }
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

        fn create_knob_a(&mut self, ui: &mut Ui) {
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

        fn create_knob_b(&mut self, ui: &mut Ui) {
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
                    if self.is_write_enabled {
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
                                ButtonState::Active
                            } else {
                                ButtonState::Indicated
                            }
                        }
                    }
                    UiState::Bpm => {
                        if pad_index <= self.volume() {
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
                },
            }
        }
    }

    impl Displays for Calculator {
        fn ui(&mut self, ui: &mut Ui) -> egui::Response {
            let highlighted_button = if self.engine.state() == &EngineState::Playing {
                Some(self.current_step())
            } else {
                None
            };
            ui.set_min_size(Vec2::new(320.0, 560.0)); // 1.75 aspect ratio
            ui.add_space(64.0);
            self.create_dashboard(ui);
            ui.add(SegmentedDisplayWidget::sixteen_segment("MUSIC").digit_height(72.0));
            ui.add_space(16.0);
            Grid::new(ui.next_auto_id())
                .num_columns(5)
                .show(ui, |ui| {
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
                })
                .response
        }
    }
}

#[cfg(test)]
mod tests {
    use groove_core::traits::Controls;

    use super::{Engine, Pattern, Step};
    use crate::controllers::calculator::{Percentage, Tempo, TempoValue};

    impl Engine {
        fn chain_active_pattern(&mut self) {
            self.chain_pattern(self.active_pattern());
        }
    }

    impl Pattern {
        fn steps(&self) -> &[Step; 16] {
            &self.steps
        }
        fn is_clear(&self) -> bool {
            self.steps().iter().all(|n| n.is_clear())
        }
    }

    impl Step {
        fn is_clear(&self) -> bool {
            self.sounds.iter().all(|s| !s)
        }
    }

    #[test]
    fn tempo() {
        let mut e = Engine::default();

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
        let mut e = Engine::default();

        assert_eq!(e.swing().0, 0, "swing should start out at 0");
        e.set_swing(Percentage(50));
        assert_eq!(e.swing().0, 50, "set_swing should work");
    }

    #[test]
    fn a_and_b() {
        let mut e = Engine::default();

        assert_eq!(e.a().0, 50, "should start out at 50");
        assert_eq!(e.b().0, 50, "should start out at 50");

        e.set_a(Percentage(40));
        assert_eq!(e.a().0, 40, "set_a should work");
        e.set_b(Percentage(100));
        assert_eq!(e.b().0, 100, "set_b should work");
    }

    #[test]
    fn pattern_crud() {
        let mut e = Engine::default();

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
        let p2 = e.pattern_mut(2);
        p2.toggle_active(0, 0);
        p2.set_a(0, 0, &Percentage(33));
        p2.set_b(0, 0, &Percentage(66));

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
            .set_all(13, 15, false, &Percentage(0), &Percentage(0));
        assert!(!e.pattern(2).is_active(13, 15));
        assert_ne!(e.pattern(2).a(13, 15).0, 42);
        assert_ne!(e.pattern(2).b(13, 15).0, 84);
        e.pattern_mut(2)
            .set_all(13, 15, true, &Percentage(42), &Percentage(84));
        assert!(e.pattern(2).is_active(13, 15));
        assert_eq!(e.pattern(2).a(13, 15).0, 42);
        assert_eq!(e.pattern(2).b(13, 15).0, 84);

        e.set_active_pattern(15);
        assert_eq!(e.active_pattern(), 15, "set active pattern works");
    }

    #[test]
    fn play_stop() {
        let mut e = Engine::default();

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
        let mut e = Engine::default();
        for index in 0..16 {
            assert!(!e.is_solo(index), "no solos at startup");
        }
        e.toggle_solo(7);
        assert!(e.is_solo(7), "toggle_solo() works");
    }

    #[test]
    fn chaining() {
        let mut e = Engine::default();

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
            assert_eq!(e.chains.index(i), 7, "successive chaining should work");
        }
        assert_eq!(e.chain_cursor(), 128, "chaining should work up to maximum");

        e.set_active_pattern(8);
        e.chain_active_pattern();
        assert_eq!(
            e.chains.index(127),
            7,
            "chaining should ignore adds beyond capacity"
        );

        e.reset_chain_cursor();
        assert_eq!(e.chain_cursor(), 0, "chain cursor at zero after reset");

        for i in 0..128 {
            assert_eq!(
                e.chains.index(i),
                u8::MAX,
                "resetting chain cursor also overwrites slots"
            );
        }
    }
}
