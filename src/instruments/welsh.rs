use super::{
    envelopes::{Envelope, EnvelopeGenerator},
    oscillators::Oscillator,
    Dca, IsStereoSampleVoice, IsVoice, PlaysNotes, SimpleVoiceStore, Synthesizer,
};
use crate::{
    common::{F32ControlValue, Normal, Sample},
    effects::filter::{BiQuadFilter, FilterParams},
    instruments::HandlesMidi,
    midi::{GeneralMidiProgram, MidiChannel, MidiMessage, MidiUtils},
    settings::{
        patches::{LfoRouting, SynthPatch, WaveformType},
        LoadError,
    },
    traits::{Controllable, Generates, HasUid, IsInstrument, Resets, Ticks, TransformsAudio},
    utils::Paths,
    BipolarNormal, StereoSample,
};
use convert_case::{Boundary, Case, Casing};
use groove_macros::{Control, Uid};
use num_traits::FromPrimitive;
use std::str::FromStr;
use strum_macros::{Display, EnumString, FromRepr};

// TODO: cache these as they're loaded
impl SynthPatch {
    pub fn patch_name_to_settings_name(name: &str) -> String {
        name.from_case(Case::Camel)
            .without_boundaries(&[Boundary::DigitLower])
            .to_case(Case::Kebab)
    }

    pub fn new_from_yaml(yaml: &str) -> Result<Self, LoadError> {
        serde_yaml::from_str(yaml).map_err(|e| {
            println!("{e}");
            LoadError::FormatError
        })
    }

    pub fn by_name(name: &str) -> Self {
        let mut filename = Paths::asset_path();
        filename.push("patches");
        filename.push("welsh");
        filename.push(format!(
            "{}.yaml",
            Self::patch_name_to_settings_name(name.to_string().as_str())
        ));
        if let Ok(contents) = std::fs::read_to_string(&filename) {
            match Self::new_from_yaml(&contents) {
                Ok(patch) => patch,
                Err(err) => {
                    // TODO: this should return a failsafe patch, maybe a boring
                    // square wave
                    panic!("couldn't parse patch file: {err:?}");
                }
            }
        } else {
            panic!("couldn't read patch file named {:?}", &filename);
        }
    }
}

impl WelshSynth {
    pub fn general_midi_preset(program: &GeneralMidiProgram) -> anyhow::Result<SynthPatch> {
        let mut delegated = false;
        let preset = match program {
            GeneralMidiProgram::AcousticGrand => "Piano",
            GeneralMidiProgram::BrightAcoustic => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::ElectricGrand => "ElectricPiano",
            GeneralMidiProgram::HonkyTonk => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::ElectricPiano1 => "ElectricPiano",
            GeneralMidiProgram::ElectricPiano2 => "ElectricPiano",
            GeneralMidiProgram::Harpsichord => "Harpsichord",
            GeneralMidiProgram::Clav => "Clavichord",
            GeneralMidiProgram::Celesta => "Celeste",
            GeneralMidiProgram::Glockenspiel => "Glockenspiel",
            GeneralMidiProgram::MusicBox => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Vibraphone => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Marimba => "Marimba",
            GeneralMidiProgram::Xylophone => "Xylophone",
            GeneralMidiProgram::TubularBells => "Bell",
            GeneralMidiProgram::Dulcimer => "Dulcimer",
            GeneralMidiProgram::DrawbarOrgan => {
                "Organ" // TODO dup
            }
            GeneralMidiProgram::PercussiveOrgan => {
                "Organ" // TODO dup
            }
            GeneralMidiProgram::RockOrgan => {
                "Organ" // TODO dup
            }
            GeneralMidiProgram::ChurchOrgan => {
                "Organ" // TODO dup
            }
            GeneralMidiProgram::ReedOrgan => {
                "Organ" // TODO dup
            }
            GeneralMidiProgram::Accordion => "Accordion",
            GeneralMidiProgram::Harmonica => "Harmonica",
            GeneralMidiProgram::TangoAccordion => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::AcousticGuitarNylon => "GuitarAcoustic",
            GeneralMidiProgram::AcousticGuitarSteel => {
                "GuitarAcoustic" // TODO dup
            }
            GeneralMidiProgram::ElectricGuitarJazz => {
                "GuitarElectric" // TODO dup
            }
            GeneralMidiProgram::ElectricGuitarClean => {
                "GuitarElectric" // TODO dup
            }
            GeneralMidiProgram::ElectricGuitarMuted => {
                "GuitarElectric" // TODO dup
            }
            GeneralMidiProgram::OverdrivenGuitar => {
                "GuitarElectric" // TODO dup
            }
            GeneralMidiProgram::DistortionGuitar => {
                "GuitarElectric" // TODO dup
            }
            GeneralMidiProgram::GuitarHarmonics => {
                "GuitarElectric" // TODO dup
            }
            GeneralMidiProgram::AcousticBass => "DoubleBass",
            GeneralMidiProgram::ElectricBassFinger => "StandupBass",
            GeneralMidiProgram::ElectricBassPick => "AcidBass",
            GeneralMidiProgram::FretlessBass => {
                "DetroitBass" // TODO same?
            }
            GeneralMidiProgram::SlapBass1 => "FunkBass",
            GeneralMidiProgram::SlapBass2 => "FunkBass",
            GeneralMidiProgram::SynthBass1 => "DigitalBass",
            GeneralMidiProgram::SynthBass2 => "DigitalBass",
            GeneralMidiProgram::Violin => "Violin",
            GeneralMidiProgram::Viola => "Viola",
            GeneralMidiProgram::Cello => "Cello",
            GeneralMidiProgram::Contrabass => "Contrabassoon",
            GeneralMidiProgram::TremoloStrings => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::PizzicatoStrings => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::OrchestralHarp => "Harp",
            GeneralMidiProgram::Timpani => "Timpani",
            GeneralMidiProgram::StringEnsemble1 => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::StringEnsemble2 => {
                "StringsPwm" // TODO same?
            }
            GeneralMidiProgram::Synthstrings1 => "StringsPwm", // TODO same?

            GeneralMidiProgram::Synthstrings2 => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::ChoirAahs => "Angels",

            GeneralMidiProgram::VoiceOohs => "Choir",
            GeneralMidiProgram::SynthVoice => "VocalFemale",

            GeneralMidiProgram::OrchestraHit => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Trumpet => "Trumpet",
            GeneralMidiProgram::Trombone => "Trombone",
            GeneralMidiProgram::Tuba => "Tuba",
            GeneralMidiProgram::MutedTrumpet => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::FrenchHorn => "FrenchHorn",

            GeneralMidiProgram::BrassSection => "BrassSection",

            GeneralMidiProgram::Synthbrass1 => {
                "BrassSection" // TODO dup
            }
            GeneralMidiProgram::Synthbrass2 => {
                "BrassSection" // TODO dup
            }
            GeneralMidiProgram::SopranoSax => {
                "Saxophone" // TODO dup
            }
            GeneralMidiProgram::AltoSax => "Saxophone",
            GeneralMidiProgram::TenorSax => {
                "Saxophone" // TODO dup
            }
            GeneralMidiProgram::BaritoneSax => {
                "Saxophone" // TODO dup
            }
            GeneralMidiProgram::Oboe => "Oboe",
            GeneralMidiProgram::EnglishHorn => "EnglishHorn",
            GeneralMidiProgram::Bassoon => "Bassoon",
            GeneralMidiProgram::Clarinet => "Clarinet",
            GeneralMidiProgram::Piccolo => "Piccolo",
            GeneralMidiProgram::Flute => "Flute",
            GeneralMidiProgram::Recorder => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::PanFlute => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::BlownBottle => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Shakuhachi => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Whistle => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Ocarina => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Lead1Square => {
                "MonoSolo" // TODO: same?
            }
            GeneralMidiProgram::Lead2Sawtooth => {
                "Trance5th" // TODO: same?
            }
            GeneralMidiProgram::Lead3Calliope => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Lead4Chiff => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Lead5Charang => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Lead6Voice => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Lead7Fifths => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Lead8BassLead => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Pad1NewAge => {
                "NewAgeLead" // TODO pad or lead?
            }
            GeneralMidiProgram::Pad2Warm => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Pad3Polysynth => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Pad4Choir => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Pad5Bowed => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Pad6Metallic => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Pad7Halo => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Pad8Sweep => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Fx1Rain => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Fx2Soundtrack => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Fx3Crystal => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Fx4Atmosphere => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Fx5Brightness => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Fx6Goblins => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Fx7Echoes => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Fx8SciFi => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Sitar => "Sitar",
            GeneralMidiProgram::Banjo => "Banjo",
            GeneralMidiProgram::Shamisen => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Koto => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Kalimba => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Bagpipe => "Bagpipes",
            GeneralMidiProgram::Fiddle => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Shanai => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::TinkleBell => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Agogo => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::SteelDrums => {
                "WheelsOfSteel" // TODO same?
            }
            GeneralMidiProgram::Woodblock => "SideStick",
            GeneralMidiProgram::TaikoDrum => {
                // XXXXXXXXXXXXX TMP
                "Cello" // TODO substitute.....
            }
            GeneralMidiProgram::MelodicTom => "Bongos",
            GeneralMidiProgram::SynthDrum => "SnareDrum",
            GeneralMidiProgram::ReverseCymbal => "Cymbal",
            GeneralMidiProgram::GuitarFretNoise => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::BreathNoise => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Seashore => "OceanWavesWithFoghorn",
            GeneralMidiProgram::BirdTweet => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::TelephoneRing => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Helicopter => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Applause => {
                delegated = true;
                "Piano"
            }
            GeneralMidiProgram::Gunshot => {
                delegated = true;
                "Piano"
            }
        };
        if delegated {
            eprintln!("Delegated {program} to {preset}");
        }
        Ok(SynthPatch::by_name(preset))
    }
}

#[derive(Debug)]
pub struct WelshVoice {
    oscillators: Vec<Oscillator>,
    oscillator_2_sync: bool,
    amp_envelope: EnvelopeGenerator,
    dca: Dca,

    lfo: Oscillator,
    lfo_routing: LfoRouting,
    lfo_depth: Normal,

    filter: BiQuadFilter,
    filter_cutoff_start: f32,
    filter_cutoff_end: f32,
    filter_envelope: EnvelopeGenerator,

    is_playing: bool,
    note_on_is_pending: bool,
    note_on_velocity: u8,
    note_off_is_pending: bool,
    note_off_velocity: u8,
    aftertouch_is_pending: bool,
    aftertouch_velocity: u8,

    sample: StereoSample,
    ticks: usize,
}
impl IsStereoSampleVoice for WelshVoice {}
impl IsVoice<StereoSample> for WelshVoice {}
impl PlaysNotes for WelshVoice {
    fn is_playing(&self) -> bool {
        self.is_playing
    }

    fn are_events_pending(&self) -> bool {
        self.note_on_is_pending || self.note_off_is_pending || self.aftertouch_is_pending
    }

    fn set_frequency_hz(&mut self, frequency_hz: f32) {
        // It's safe to set the frequency on a fixed-frequency oscillator; the
        // fixed frequency is stored separately and takes precedence.
        self.oscillators.iter_mut().for_each(|o| {
            o.set_frequency(frequency_hz);
        });
    }

    fn enqueue_note_on(&mut self, velocity: u8) {
        self.note_on_is_pending = true;
        self.note_on_velocity = velocity;
    }

    fn enqueue_aftertouch(&mut self, velocity: u8) {
        self.aftertouch_is_pending = true;
        self.aftertouch_velocity = velocity;
    }

    fn enqueue_note_off(&mut self, velocity: u8) {
        self.note_off_is_pending = true;
        self.note_off_velocity = velocity;
    }

    fn set_pan(&mut self, value: f32) {
        self.dca.set_pan(BipolarNormal::from(value))
    }
}
impl Generates<StereoSample> for WelshVoice {
    fn value(&self) -> StereoSample {
        self.sample
    }

    fn batch_values(&mut self, _samples: &mut [StereoSample]) {
        todo!()
    }
}
impl Resets for WelshVoice {
    fn reset(&mut self, sample_rate: usize) {
        self.ticks = 0;
        self.lfo.reset(sample_rate);
        self.amp_envelope.reset(sample_rate);
        self.filter_envelope.reset(sample_rate);
        self.oscillators
            .iter_mut()
            .for_each(|o| o.reset(sample_rate));
    }
}
impl Ticks for WelshVoice {
    fn tick(&mut self, tick_count: usize) {
        for _ in 0..tick_count {
            self.ticks += 1;
            self.handle_pending_note_events();
            // It's important for the envelope tick() methods to be called after
            // their handle_note_* methods are called, but before we check whether
            // amp_envelope.is_idle(), because the tick() methods are what determine
            // the current idle state.
            //
            // TODO: this seems like an implementation detail that maybe should be
            // hidden from the caller.
            self.lfo.tick(1);
            let (amp_env_amplitude, filter_env_amplitude) = self.tick_envelopes();
            self.oscillators.iter_mut().for_each(|o| o.tick(1));

            self.sample = if self.is_playing() {
                // LFO
                let lfo = self.lfo.value();
                if matches!(self.lfo_routing, LfoRouting::Pitch) {
                    let lfo_for_pitch = lfo * self.lfo_depth.value();
                    for o in self.oscillators.iter_mut() {
                        o.set_frequency_modulation(lfo_for_pitch as f32);
                    }
                }

                // Oscillators
                let len = self.oscillators.len();
                let osc_sum = match len {
                    0 => 0.0,
                    1 => self.oscillators[0].value(),
                    2 => {
                        if self.oscillator_2_sync && self.oscillators[0].should_sync() {
                            self.oscillators[1].sync();
                        }
                        (self.oscillators[0].value() + self.oscillators[1].value()) / 2.0
                    }
                    _ => todo!(),
                };

                // Filters
                //
                // https://aempass.blogspot.com/2014/09/analog-and-welshs-synthesizer-cookbook.html
                if self.filter_cutoff_end != 0.0 {
                    let new_cutoff_percentage = self.filter_cutoff_start
                        + (1.0 - self.filter_cutoff_start)
                            * self.filter_cutoff_end
                            * filter_env_amplitude.value() as f32;
                    self.filter.set_cutoff_pct(new_cutoff_percentage);
                } else if matches!(self.lfo_routing, LfoRouting::FilterCutoff) {
                    let lfo_for_cutoff = lfo * self.lfo_depth.value();
                    self.filter
                        .set_cutoff_pct(self.filter_cutoff_start * (1.0 + lfo_for_cutoff as f32));
                }
                let filtered_mix = self.filter.transform_channel(0, Sample::from(osc_sum)).0;

                // LFO amplitude modulation
                let lfo_for_amplitude = if matches!(self.lfo_routing, LfoRouting::Amplitude) {
                    // LFO ranges from [-1, 1], so convert to something that can silence or double the volume.
                    lfo * self.lfo_depth.value() + 1.0
                } else {
                    1.0
                };

                // Final
                self.dca.transform_audio_to_stereo(Sample(
                    filtered_mix * amp_env_amplitude.value() * lfo_for_amplitude,
                ))
            } else {
                StereoSample::SILENCE
            };
        }
    }
}
impl WelshVoice {
    pub fn new_with(sample_rate: usize, preset: &SynthPatch) -> Self {
        let mut r = Self {
            amp_envelope: EnvelopeGenerator::new_with(sample_rate, &preset.amp_envelope),

            lfo: Oscillator::new_lfo(sample_rate, &preset.lfo),
            lfo_routing: preset.lfo.routing,
            lfo_depth: preset.lfo.depth.into(),

            filter: BiQuadFilter::new_with(
                &FilterParams::LowPass12db {
                    cutoff: preset.filter_type_12db.cutoff_hz,
                    q: BiQuadFilter::denormalize_q(preset.filter_resonance),
                },
                sample_rate,
            ),
            filter_cutoff_start: BiQuadFilter::frequency_to_percent(
                preset.filter_type_12db.cutoff_hz,
            ),
            filter_cutoff_end: preset.filter_envelope_weight,
            filter_envelope: EnvelopeGenerator::new_with(sample_rate, &preset.filter_envelope),
            oscillators: Default::default(),
            oscillator_2_sync: Default::default(),
            dca: Default::default(),
            is_playing: Default::default(),
            note_on_is_pending: Default::default(),
            note_on_velocity: Default::default(),
            note_off_is_pending: Default::default(),
            note_off_velocity: Default::default(),
            aftertouch_is_pending: Default::default(),
            aftertouch_velocity: Default::default(),
            sample: Default::default(),
            ticks: Default::default(),
        };
        if !matches!(preset.oscillator_1.waveform, WaveformType::None) {
            r.oscillators.push(Oscillator::new_from_preset(
                sample_rate,
                &preset.oscillator_1,
            ));
        }
        if !matches!(preset.oscillator_2.waveform, WaveformType::None) {
            let mut o = Oscillator::new_from_preset(sample_rate, &preset.oscillator_2);
            if !preset.oscillator_2_track {
                if let crate::settings::patches::OscillatorTune::Note(note) =
                    preset.oscillator_2.tune
                {
                    o.set_fixed_frequency(MidiUtils::note_to_frequency(note));
                } else {
                    panic!("Patch configured without oscillator 2 tracking, but tune is not a note specification");
                }
            }
            r.oscillator_2_sync = preset.oscillator_2_sync;
            r.oscillators.push(o);
        }
        if preset.noise > 0.0 {
            r.oscillators.push(Oscillator::new_with_waveform(
                sample_rate,
                WaveformType::Noise,
            ));
        }
        r
    }

    fn handle_pending_note_events(&mut self) {
        if self.note_on_is_pending && self.note_off_is_pending {
            // Handle the case where both are pending at the same time.
            if self.is_playing {
                self.handle_note_off_event();
                self.handle_note_on_event();
            } else {
                self.handle_note_on_event();
                self.handle_note_off_event();
            }
        } else {
            if self.note_off_is_pending {
                self.handle_note_off_event();
            }
            if self.note_on_is_pending {
                self.handle_note_on_event();
            }
        }
        if self.aftertouch_is_pending {
            self.handle_aftertouch_event();
        }
    }

    fn tick_envelopes(&mut self) -> (Normal, Normal) {
        self.amp_envelope.tick(1);
        let amp_amplitude = self.amp_envelope.value();
        self.filter_envelope.tick(1);
        let filter_amplitude = self.filter_envelope.value();

        // TODO: I think this is setting is_playing a tick too early, but when I
        // moved it, it broke something else (the synth was deleting the note
        // because it no longer appeared to be playing). Fragile. Fix.
        self.is_playing = !self.amp_envelope.is_idle();

        (amp_amplitude, filter_amplitude)
    }

    fn handle_aftertouch_event(&mut self) {
        self.aftertouch_is_pending = false;
        // TODO: do something
    }

    fn handle_note_on_event(&mut self) {
        self.note_on_is_pending = false;
        self.amp_envelope.enqueue_attack();
        self.filter_envelope.enqueue_attack();
    }

    fn handle_note_off_event(&mut self) {
        self.note_off_is_pending = false;
        self.amp_envelope.enqueue_release();
        self.filter_envelope.enqueue_release();
    }
}

#[derive(Control, Debug, Uid)]
pub struct WelshSynth {
    uid: usize,
    inner_synth: Synthesizer<WelshVoice>,

    // TODO: will it be common for #[controllable] to represent a fake value
    // that's actually propagated to things underneath? If so, do we need a
    // better way to handle this?
    #[controllable]
    #[allow(dead_code)]
    pan: f32,
}
impl IsInstrument for WelshSynth {}
impl Generates<StereoSample> for WelshSynth {
    fn value(&self) -> StereoSample {
        self.inner_synth.value()
    }

    fn batch_values(&mut self, values: &mut [StereoSample]) {
        self.inner_synth.batch_values(values);
    }
}
impl Resets for WelshSynth {
    fn reset(&mut self, sample_rate: usize) {
        self.inner_synth.reset(sample_rate);
    }
}
impl Ticks for WelshSynth {
    fn tick(&mut self, tick_count: usize) {
        self.inner_synth.tick(tick_count);
    }
}
impl HandlesMidi for WelshSynth {
    fn handle_midi_message(
        &mut self,
        message: &MidiMessage,
    ) -> Option<Vec<(MidiChannel, MidiMessage)>> {
        match message {
            MidiMessage::ProgramChange { program } => {
                if let Some(program) = GeneralMidiProgram::from_u8(program.as_int()) {
                    if let Ok(_preset) = WelshSynth::general_midi_preset(&program) {
                        //  self.preset = preset;
                    } else {
                        println!("unrecognized patch from MIDI program change: {}", &program);
                    }
                }
                None
            }
            _ => self.inner_synth.handle_midi_message(&message),
        }
    }
}
impl WelshSynth {
    pub(crate) fn new_with(sample_rate: usize, preset: SynthPatch) -> Self {
        let mut voice_store = Box::new(SimpleVoiceStore::<WelshVoice>::new_with(sample_rate));
        for _ in 0..8 {
            voice_store.add_voice(Box::new(WelshVoice::new_with(sample_rate, &preset)));
        }
        Self {
            uid: Default::default(),
            inner_synth: Synthesizer::<WelshVoice>::new_with(sample_rate, voice_store),
            pan: Default::default(),
        }
    }

    pub fn preset_name(&self) -> &str {
        "none"
        //        self.preset.name.as_str()
    }

    pub fn pan(&self) -> f32 {
        self.inner_synth.pan()
    }

    pub fn set_pan(&mut self, pan: f32) {
        self.inner_synth.set_pan(pan);
    }

    pub fn set_control_pan(&mut self, value: F32ControlValue) {
        // TODO: more toil. Let me say this is a bipolar normal
        self.set_pan(value.0 * 2.0 - 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::WelshVoice;
    use super::*;
    use crate::{
        clock::Clock,
        common::{SampleType, DEFAULT_SAMPLE_RATE},
        instruments::{tests::is_voice_makes_any_sound_at_all, welsh::WaveformType},
        midi::{MidiNote, MidiUtils},
        settings::patches::{
            EnvelopeSettings, FilterPreset, LfoPreset, LfoRouting, OscillatorSettings,
            PolyphonySettings,
        },
        utils::tests::canonicalize_filename,
    };

    // TODO: refactor out to common test utilities
    #[allow(dead_code)]
    fn write_voice(voice: &mut WelshVoice, duration: f32, basename: &str) {
        let mut clock = Clock::default();

        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: clock.sample_rate() as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        const AMPLITUDE: SampleType = i16::MAX as SampleType;
        let mut writer = hound::WavWriter::create(canonicalize_filename(basename), spec).unwrap();

        voice.set_frequency_hz(MidiUtils::note_type_to_frequency(MidiNote::C4));
        let mut last_recognized_time_point = -1.;
        let time_note_off = duration / 2.0;
        while clock.seconds() < duration {
            if clock.seconds() >= 0.0 && last_recognized_time_point < 0.0 {
                last_recognized_time_point = clock.seconds();
                voice.enqueue_note_on(127);
                voice.handle_pending_note_events();
                voice.tick_envelopes();
            } else if clock.seconds() >= time_note_off && last_recognized_time_point < time_note_off
            {
                last_recognized_time_point = clock.seconds();
                voice.enqueue_note_off(127);
                voice.handle_pending_note_events();
                voice.tick_envelopes();
            }

            voice.tick(1);
            let sample = voice.value();
            let _ = writer.write_sample((sample.0 .0 * AMPLITUDE) as i16);
            let _ = writer.write_sample((sample.1 .0 * AMPLITUDE) as i16);
            clock.tick();
        }
    }

    // use std::panic;
    // use strum::IntoEnumIterator;
    // #[test]
    // #[should_panic]
    // fn test_presets() {
    //     let clock = Clock::new(&ClockSettings::new_defaults());
    //     for preset in PresetName::iter() {
    //         let result = panic::catch_unwind(|| {
    //             Voice::new(
    //                 MIDI_CHANNEL_RECEIVE_ALL,
    //                 clock.sample_rate(),
    //                 &super::SynthPreset::by_name(&preset),
    //             )
    //         });
    //         if result.is_ok() {
    //             let mut voice = result.unwrap();
    //             let preset_name = preset.to_string();
    //             write_voice(&mut voice, 2.0, &format!("voice_{}", preset_name));
    //         }
    //     }
    // }

    // This code was used to convert Rust representation of 26 Welsh patches to serde YAML.
    // #[derive(Serialize)]
    // struct Foo {
    //     x: Vec<SynthPreset>,
    // }

    // #[test]
    // #[should_panic]
    // fn test_presets() {
    //     for preset in PresetName::iter() {
    //         if let Ok(result) = panic::catch_unwind(|| super::SynthPreset::by_name(&preset)) {
    //             if let Ok(s) = serde_yaml::to_string(&result) {
    //                 if let Ok(_) = std::fs::write(format!("{}.yaml", result.name), s) {
    //                     // great
    //                 }
    //             }
    //         }
    //     }
    // }

    // TODO: get rid of this
    fn write_sound(
        source: &mut WelshVoice,
        clock: &mut Clock,
        duration: f32,
        when: f32,
        basename: &str,
    ) {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: clock.sample_rate() as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        const AMPLITUDE: SampleType = i16::MAX as SampleType;
        let mut writer = hound::WavWriter::create(canonicalize_filename(basename), spec).unwrap();

        let mut is_message_sent = false;
        while clock.seconds() < duration {
            if when <= clock.seconds() && !is_message_sent {
                is_message_sent = true;
                source.enqueue_note_off(0);
                source.handle_pending_note_events();
                source.tick_envelopes();
            }
            source.tick(1);
            let sample = source.value();
            let _ = writer.write_sample((sample.0 .0 * AMPLITUDE) as i16);
            let _ = writer.write_sample((sample.1 .0 * AMPLITUDE) as i16);
            clock.tick();
        }
    }

    fn cello_patch() -> SynthPatch {
        SynthPatch {
            name: SynthPatch::patch_name_to_settings_name("Cello"),
            oscillator_1: OscillatorSettings {
                waveform: WaveformType::PulseWidth(0.1),
                ..Default::default()
            },
            oscillator_2: OscillatorSettings {
                waveform: WaveformType::Square,
                ..Default::default()
            },
            oscillator_2_track: true,
            oscillator_2_sync: false,
            noise: 0.0,
            lfo: LfoPreset {
                routing: LfoRouting::Amplitude,
                waveform: WaveformType::Sine,
                frequency: 7.5,
                depth: crate::settings::patches::LfoDepth::Pct(5.0),
            },
            glide: 0.0,
            unison: false,
            polyphony: PolyphonySettings::Multi,
            filter_type_24db: FilterPreset {
                cutoff_hz: 40.0,
                cutoff_pct: 0.1,
            },
            filter_type_12db: FilterPreset {
                cutoff_hz: 40.0,
                cutoff_pct: 0.1,
            },
            filter_resonance: 0.0,
            filter_envelope_weight: 0.9,
            filter_envelope: EnvelopeSettings {
                attack: 0.0,
                decay: 3.29,
                sustain: 0.78,
                release: EnvelopeSettings::MAX,
            },
            amp_envelope: EnvelopeSettings {
                attack: 0.06,
                decay: EnvelopeSettings::MAX,
                sustain: 1.0,
                release: 0.3,
            },
        }
    }

    fn test_patch() -> SynthPatch {
        SynthPatch {
            name: SynthPatch::patch_name_to_settings_name("Test"),
            oscillator_1: OscillatorSettings {
                waveform: WaveformType::Sawtooth,
                ..Default::default()
            },
            oscillator_2: OscillatorSettings {
                waveform: WaveformType::None,
                ..Default::default()
            },
            oscillator_2_track: true,
            oscillator_2_sync: false,
            noise: 0.0,
            lfo: LfoPreset {
                routing: LfoRouting::None,
                ..Default::default()
            },
            glide: 0.0,
            unison: false,
            polyphony: PolyphonySettings::Multi,
            filter_type_24db: FilterPreset {
                cutoff_hz: 40.0,
                cutoff_pct: 0.1,
            },
            filter_type_12db: FilterPreset {
                cutoff_hz: 20.0,
                cutoff_pct: 0.05,
            },
            filter_resonance: 0.0,
            filter_envelope_weight: 1.0,
            filter_envelope: EnvelopeSettings {
                attack: 5.0,
                decay: EnvelopeSettings::MAX,
                sustain: 1.0,
                release: EnvelopeSettings::MAX,
            },
            amp_envelope: EnvelopeSettings {
                attack: 0.5,
                decay: EnvelopeSettings::MAX,
                sustain: 1.0,
                release: EnvelopeSettings::MAX,
            },
        }
    }

    #[test]
    fn welsh_makes_any_sound_at_all() {
        let mut voice = WelshVoice::new_with(DEFAULT_SAMPLE_RATE, &test_patch());
        voice.set_frequency_hz(MidiUtils::note_type_to_frequency(MidiNote::C4));
        voice.enqueue_note_on(127);

        assert!(
            is_voice_makes_any_sound_at_all(&mut voice),
            "once triggered, Welsh voice should make a sound"
        );
    }

    #[test]
    fn test_basic_synth_patch() {
        let mut clock = Clock::default();
        let mut voice = WelshVoice::new_with(clock.sample_rate(), &test_patch());
        voice.set_frequency_hz(MidiUtils::note_type_to_frequency(MidiNote::C4));
        voice.enqueue_note_on(127);
        voice.handle_pending_note_events();
        voice.tick_envelopes();
        write_sound(&mut voice, &mut clock, 5.0, 5.0, "voice_basic_test_c4");
    }

    #[test]
    fn test_basic_cello_patch() {
        let mut clock = Clock::default();
        let mut voice = WelshVoice::new_with(clock.sample_rate(), &cello_patch());
        voice.set_frequency_hz(MidiUtils::note_type_to_frequency(MidiNote::C4));
        voice.enqueue_note_on(127);
        voice.handle_pending_note_events();
        voice.tick_envelopes();
        write_sound(&mut voice, &mut clock, 5.0, 1.0, "voice_cello_c4");
    }
}
