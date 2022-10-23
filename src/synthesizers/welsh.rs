use super::general_midi::GeneralMidiProgram;
use convert_case::{Case, Casing};
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap, f32::consts::FRAC_1_SQRT_2, rc::Rc};
use strum_macros::{Display, EnumIter};

use crate::{
    common::{rrc, MonoSample, Rrc, Ww},
    effects::filter::{Filter, FilterType},
    midi::{MidiChannel, MidiMessage, MidiMessageType},
    settings::{
        patches::{
            EnvelopePreset, FilterPreset, GlidePreset, LfoPreset, LfoRouting, OscillatorPreset,
            PolyphonyPreset, WaveformType,
        },
        LoadError,
    },
    traits::{IsMidiInstrument, IsMutable, SinksMidi, SourcesAudio, TransformsAudio},
    {clock::Clock, envelopes::AdsrEnvelope, oscillators::Oscillator},
};

#[derive(Clone, Debug, Deserialize, Display, EnumIter, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PresetName {
    // -------------------- Strings
    Banjo,
    Cello,
    DoubleBass,
    Dulcimer,
    GuitarAcoustic,
    GuitarElectric,
    Harp,
    HurdyGurdy,
    Kora,
    Lute,
    Mandocello,
    Mandolin,
    Riti,
    Sitar,
    StandupBass,
    Viola,
    Violin,
    // -------------------- Woodwinds
    Bagpipes,
    BassClarinet,
    Bassoon,
    Clarinet,
    ConchShell,
    Contrabassoon,
    Digeridoo,
    EnglishHorn,
    Flute,
    Oboe,
    Piccolo,
    // -------------------- Brass
    FrenchHorn,
    Harmonica,
    PennyWhistle,
    Saxophone,
    Trombone,
    Trumpet,
    Tuba,
    // -------------------- Keyboards
    Accordion,
    Celeste,
    Clavichord,
    ElectricPiano,
    Harpsichord,
    Organ,
    Piano,
    // -------------------- Vocals
    Angels,
    Choir,
    VocalFemale,
    VocalMale,
    Whistling,
    // -------------------- Tuned Percussion
    Bell,
    Bongos,
    Conga,
    Glockenspiel,
    Marimba,
    Timpani,
    Xylophone,
    // -------------------- Untuned Percussion
    BassDrum,
    Castanets,
    Clap,
    Claves,
    Cowbell,
    CowbellAnalog,
    Cymbal,
    SideStick,
    SnareDrum,
    Tambourine,
    WheelsOfSteel,
    // -------------------- Leads
    BrassSection,
    Mellow70sLead,
    MonoSolo,
    NewAgeLead,
    RAndBSlide,
    ScreamingSync,
    StringsPwm,
    Trance5th,
    // -------------------- Bass
    AcidBass,
    BassOfTheTimeLords,
    DetroitBass,
    DeutscheBass,
    DigitalBass,
    FunkBass,
    GrowlingBass,
    RezBass,
    // -------------------- Pads
    AndroidDreams,
    CelestialWash,
    DarkCity,
    Aurora,
    GalacticCathedral,
    GalacticChapel,
    Portus,
    PostApocalypticSyncSweep,
    TerraEnceladus,
    // -------------------- Sound Effects
    Cat,
    DigitalAlarmClock,
    JourneyToTheCore,
    Kazoo,
    Laser,
    Motor,
    NerdOTron2000,
    OceanWavesWithFoghorn,
    PositronicRhythm,
    SpaceAttack,
    Toad,
    Wind,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SynthPreset {
    pub name: String,
    pub oscillator_1_preset: OscillatorPreset,
    pub oscillator_2_preset: OscillatorPreset,
    pub oscillator_2_track: bool,
    pub oscillator_2_sync: bool,

    pub noise: f32,

    pub lfo_preset: LfoPreset,

    pub glide: GlidePreset,
    pub has_unison: bool,
    pub polyphony: PolyphonyPreset,

    // There is meant to be only one filter, but the Welsh book
    // provides alternate settings depending on the kind of filter
    // your synthesizer has.
    pub filter_type_24db: FilterPreset,
    pub filter_type_12db: FilterPreset,
    pub filter_resonance: f32, // This should be an appropriate interpretation of a linear 0..1
    pub filter_envelope_weight: f32,
    pub filter_envelope_preset: EnvelopePreset,

    pub amp_envelope_preset: EnvelopePreset,
}

// TODO: cache these as they're loaded
impl SynthPreset {
    pub fn patch_name_to_settings_name(name: &str) -> String {
        name.to_case(Case::Kebab)
    }

    pub fn new_from_yaml(yaml: &str) -> Result<Self, LoadError> {
        serde_yaml::from_str(yaml).map_err(|e| {
            println!("{}", e);
            LoadError::FormatError
        })
    }

    pub fn by_name(name: &PresetName) -> Self {
        let filename = format!(
            "resources/patches/welsh/{}.yaml",
            Self::patch_name_to_settings_name(name.to_string().as_str())
        );
        if let Ok(contents) = std::fs::read_to_string(filename) {
            if let Ok(patch) = Self::new_from_yaml(&contents) {
                patch
            } else {
                panic!("couldn't load that patch");
            }
        } else {
            panic!("couldn't read that file");
        }
    }
}

impl Synth {
    pub fn general_midi_preset(program: GeneralMidiProgram) -> SynthPreset {
        let mut delegated = false;
        let preset = match program {
            GeneralMidiProgram::AcousticGrand => PresetName::Piano,
            GeneralMidiProgram::BrightAcoustic => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::ElectricGrand => PresetName::ElectricPiano,
            GeneralMidiProgram::HonkyTonk => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::ElectricPiano1 => PresetName::ElectricPiano,
            GeneralMidiProgram::ElectricPiano2 => PresetName::ElectricPiano,
            GeneralMidiProgram::Harpsichord => PresetName::Harpsichord,
            GeneralMidiProgram::Clav => PresetName::Clavichord,
            GeneralMidiProgram::Celesta => PresetName::Celeste,
            GeneralMidiProgram::Glockenspiel => PresetName::Glockenspiel,
            GeneralMidiProgram::MusicBox => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Vibraphone => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Marimba => PresetName::Marimba,
            GeneralMidiProgram::Xylophone => PresetName::Xylophone,
            GeneralMidiProgram::TubularBells => PresetName::Bell,
            GeneralMidiProgram::Dulcimer => PresetName::Dulcimer,
            GeneralMidiProgram::DrawbarOrgan => {
                PresetName::Organ // TODO dup
            }
            GeneralMidiProgram::PercussiveOrgan => {
                PresetName::Organ // TODO dup
            }
            GeneralMidiProgram::RockOrgan => {
                PresetName::Organ // TODO dup
            }
            GeneralMidiProgram::ChurchOrgan => {
                PresetName::Organ // TODO dup
            }
            GeneralMidiProgram::ReedOrgan => {
                PresetName::Organ // TODO dup
            }
            GeneralMidiProgram::Accordion => PresetName::Accordion,
            GeneralMidiProgram::Harmonica => PresetName::Harmonica,
            GeneralMidiProgram::TangoAccordion => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::AcousticGuitarNylon => PresetName::GuitarAcoustic,
            GeneralMidiProgram::AcousticGuitarSteel => {
                PresetName::GuitarAcoustic // TODO dup
            }
            GeneralMidiProgram::ElectricGuitarJazz => {
                PresetName::GuitarElectric // TODO dup
            }
            GeneralMidiProgram::ElectricGuitarClean => {
                PresetName::GuitarElectric // TODO dup
            }
            GeneralMidiProgram::ElectricGuitarMuted => {
                PresetName::GuitarElectric // TODO dup
            }
            GeneralMidiProgram::OverdrivenGuitar => {
                PresetName::GuitarElectric // TODO dup
            }
            GeneralMidiProgram::DistortionGuitar => {
                PresetName::GuitarElectric // TODO dup
            }
            GeneralMidiProgram::GuitarHarmonics => {
                PresetName::GuitarElectric // TODO dup
            }
            GeneralMidiProgram::AcousticBass => PresetName::DoubleBass,
            GeneralMidiProgram::ElectricBassFinger => PresetName::StandupBass,
            GeneralMidiProgram::ElectricBassPick => PresetName::AcidBass,
            GeneralMidiProgram::FretlessBass => {
                PresetName::DetroitBass // TODO same?
            }
            GeneralMidiProgram::SlapBass1 => PresetName::FunkBass,
            GeneralMidiProgram::SlapBass2 => PresetName::FunkBass,
            GeneralMidiProgram::SynthBass1 => PresetName::DigitalBass,
            GeneralMidiProgram::SynthBass2 => PresetName::DigitalBass,
            GeneralMidiProgram::Violin => PresetName::Violin,
            GeneralMidiProgram::Viola => PresetName::Viola,
            GeneralMidiProgram::Cello => PresetName::Cello,
            GeneralMidiProgram::Contrabass => PresetName::Contrabassoon,
            GeneralMidiProgram::TremoloStrings => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::PizzicatoStrings => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::OrchestralHarp => PresetName::Harp,
            GeneralMidiProgram::Timpani => PresetName::Timpani,
            GeneralMidiProgram::StringEnsemble1 => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::StringEnsemble2 => {
                PresetName::StringsPwm // TODO same?
            }
            GeneralMidiProgram::Synthstrings1 => PresetName::StringsPwm, // TODO same?

            GeneralMidiProgram::Synthstrings2 => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::ChoirAahs => PresetName::Angels,

            GeneralMidiProgram::VoiceOohs => PresetName::Choir,
            GeneralMidiProgram::SynthVoice => PresetName::VocalFemale,

            GeneralMidiProgram::OrchestraHit => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Trumpet => PresetName::Trumpet,
            GeneralMidiProgram::Trombone => PresetName::Trombone,
            GeneralMidiProgram::Tuba => PresetName::Tuba,
            GeneralMidiProgram::MutedTrumpet => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::FrenchHorn => PresetName::FrenchHorn,

            GeneralMidiProgram::BrassSection => PresetName::BrassSection,

            GeneralMidiProgram::Synthbrass1 => {
                PresetName::BrassSection // TODO dup
            }
            GeneralMidiProgram::Synthbrass2 => {
                PresetName::BrassSection // TODO dup
            }
            GeneralMidiProgram::SopranoSax => {
                PresetName::Saxophone // TODO dup
            }
            GeneralMidiProgram::AltoSax => PresetName::Saxophone,
            GeneralMidiProgram::TenorSax => {
                PresetName::Saxophone // TODO dup
            }
            GeneralMidiProgram::BaritoneSax => {
                PresetName::Saxophone // TODO dup
            }
            GeneralMidiProgram::Oboe => PresetName::Oboe,
            GeneralMidiProgram::EnglishHorn => PresetName::EnglishHorn,
            GeneralMidiProgram::Bassoon => PresetName::Bassoon,
            GeneralMidiProgram::Clarinet => PresetName::Clarinet,
            GeneralMidiProgram::Piccolo => PresetName::Piccolo,
            GeneralMidiProgram::Flute => PresetName::Flute,
            GeneralMidiProgram::Recorder => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::PanFlute => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::BlownBottle => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Shakuhachi => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Whistle => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Ocarina => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Lead1Square => {
                PresetName::MonoSolo // TODO: same?
            }
            GeneralMidiProgram::Lead2Sawtooth => {
                PresetName::Trance5th // TODO: same?
            }
            GeneralMidiProgram::Lead3Calliope => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Lead4Chiff => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Lead5Charang => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Lead6Voice => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Lead7Fifths => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Lead8BassLead => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Pad1NewAge => {
                PresetName::NewAgeLead // TODO pad or lead?
            }
            GeneralMidiProgram::Pad2Warm => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Pad3Polysynth => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Pad4Choir => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Pad5Bowed => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Pad6Metallic => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Pad7Halo => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Pad8Sweep => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Fx1Rain => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Fx2Soundtrack => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Fx3Crystal => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Fx4Atmosphere => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Fx5Brightness => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Fx6Goblins => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Fx7Echoes => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Fx8SciFi => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Sitar => PresetName::Sitar,
            GeneralMidiProgram::Banjo => PresetName::Banjo,
            GeneralMidiProgram::Shamisen => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Koto => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Kalimba => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Bagpipe => PresetName::Bagpipes,
            GeneralMidiProgram::Fiddle => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Shanai => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::TinkleBell => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Agogo => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::SteelDrums => {
                PresetName::WheelsOfSteel // TODO same?
            }
            GeneralMidiProgram::Woodblock => PresetName::SideStick,
            GeneralMidiProgram::TaikoDrum => {
                // XXXXXXXXXXXXX TMP
                PresetName::Cello // TODO substitute.....
            }
            GeneralMidiProgram::MelodicTom => PresetName::Bongos,
            GeneralMidiProgram::SynthDrum => PresetName::SnareDrum,
            GeneralMidiProgram::ReverseCymbal => PresetName::Cymbal,
            GeneralMidiProgram::GuitarFretNoise => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::BreathNoise => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Seashore => PresetName::OceanWavesWithFoghorn,
            GeneralMidiProgram::BirdTweet => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::TelephoneRing => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Helicopter => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Applause => {
                delegated = true;
                PresetName::Piano
            }
            GeneralMidiProgram::Gunshot => {
                delegated = true;
                PresetName::Piano
            }
        };
        if delegated {
            println!("Delegated {} to {}", program, preset);
        }
        SynthPreset::by_name(&preset)
    }
}

#[derive(Debug, Default)]
pub struct Voice {
    midi_channel: MidiChannel,
    oscillators: Vec<Oscillator>,
    osc_mix: Vec<f32>,
    amp_envelope: AdsrEnvelope,

    lfo: Oscillator,
    lfo_routing: LfoRouting,
    lfo_depth: f32,

    filter: Filter,
    filter_cutoff_start: f32,
    filter_cutoff_end: f32,
    filter_envelope: AdsrEnvelope,

    is_muted: bool,
}

impl Voice {
    pub fn new(midi_channel: MidiChannel, sample_rate: usize, preset: &SynthPreset) -> Self {
        let mut r = Self {
            midi_channel,
            oscillators: Vec::new(),
            osc_mix: Vec::new(),
            amp_envelope: AdsrEnvelope::new_with(&preset.amp_envelope_preset),

            lfo: Oscillator::new_lfo(&preset.lfo_preset),
            lfo_routing: preset.lfo_preset.routing,
            lfo_depth: preset.lfo_preset.depth,

            filter: Filter::new(&FilterType::LowPass {
                sample_rate,
                cutoff: preset.filter_type_12db.cutoff,
                q: FRAC_1_SQRT_2, // TODO: resonance
            }),
            filter_cutoff_start: Filter::frequency_to_percent(preset.filter_type_12db.cutoff),
            filter_cutoff_end: preset.filter_envelope_weight,
            filter_envelope: AdsrEnvelope::new_with(&preset.filter_envelope_preset),

            is_muted: false,
        };
        if !matches!(preset.oscillator_1_preset.waveform, WaveformType::None) {
            r.oscillators
                .push(Oscillator::new_from_preset(&preset.oscillator_1_preset));
            r.osc_mix.push(preset.oscillator_1_preset.mix);
        }
        if !matches!(preset.oscillator_2_preset.waveform, WaveformType::None) {
            let mut o = Oscillator::new_from_preset(&preset.oscillator_2_preset);
            if !preset.oscillator_2_track {
                o.set_fixed_frequency(MidiMessage::note_to_frequency(
                    preset.oscillator_2_preset.tune as u8,
                ));
            }
            r.oscillators.push(o);
            r.osc_mix.push(preset.oscillator_2_preset.mix);
        }
        if preset.noise > 0.0 {
            r.oscillators
                .push(Oscillator::new_with(WaveformType::Noise));
            r.osc_mix.push(preset.noise);
        }
        r
    }

    pub(crate) fn is_playing(&self, clock: &Clock) -> bool {
        !self.amp_envelope.is_idle(clock)
    }
}

impl SinksMidi for Voice {
    fn midi_channel(&self) -> MidiChannel {
        self.midi_channel
    }

    fn set_midi_channel(&mut self, midi_channel: MidiChannel) {
        self.midi_channel = midi_channel;
    }

    fn handle_midi_for_channel(&mut self, clock: &Clock, message: &MidiMessage) {
        match message.status {
            MidiMessageType::NoteOn => {
                let frequency = message.message_to_frequency();
                for o in self.oscillators.iter_mut() {
                    o.set_frequency(frequency);
                }
                self.amp_envelope.handle_note_event(clock, true);
                self.filter_envelope.handle_note_event(clock, true);
            }
            MidiMessageType::NoteOff => {
                self.amp_envelope.handle_note_event(clock, false);
                self.filter_envelope.handle_note_event(clock, false);
            }
            MidiMessageType::ProgramChange => {}
        }
    }
}
impl SourcesAudio for Voice {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        // LFO
        let lfo = self.lfo.source_audio(clock) * self.lfo_depth as MonoSample;
        if matches!(self.lfo_routing, LfoRouting::Pitch) {
            let lfo_for_pitch = lfo / 10000.0;
            // TODO: divide by 10,000 until we figure out how pitch depth is supposed to go
            // TODO: this could leave a side effect if we reuse voices and forget to clean up.
            for o in self.oscillators.iter_mut() {
                o.set_frequency_modulation(lfo_for_pitch);
            }
        }

        // Oscillators
        let osc_sum = if self.oscillators.is_empty() {
            0.0
        } else {
            let t: MonoSample = self
                .oscillators
                .iter_mut()
                .map(|o| o.source_audio(clock))
                .sum();
            t / self.oscillators.len() as MonoSample
        };

        // Filters
        let new_cutoff_percentage = self.filter_cutoff_start
            + (self.filter_cutoff_end - self.filter_cutoff_start)
                * self.filter_envelope.source_audio(clock);
        let new_cutoff = Filter::percent_to_frequency(new_cutoff_percentage);
        self.filter.set_cutoff(new_cutoff);
        let filtered_mix = self.filter.transform_audio(osc_sum);

        // LFO amplitude modulation
        let lfo_amplitude_modulation = if matches!(self.lfo_routing, LfoRouting::Amplitude) {
            // LFO ranges from [-1, 1], so convert to something that can silence or double the volume.
            lfo + 1.0
        } else {
            1.0
        };

        // Final
        filtered_mix * self.amp_envelope.source_audio(clock) * lfo_amplitude_modulation
    }
}
impl IsMutable for Voice {
    fn is_muted(&self) -> bool {
        self.is_muted
    }

    fn set_muted(&mut self, is_muted: bool) {
        self.is_muted = is_muted;
    }
}

#[derive(Debug, Default, Clone)]
pub struct Synth {
    pub(crate) me: Ww<Self>,
    midi_channel: MidiChannel,
    sample_rate: usize,
    pub(crate) preset: SynthPreset,
    note_to_voice: HashMap<u8, Rc<RefCell<Voice>>>,
    is_muted: bool,

    debug_last_seconds: f32,
}
impl IsMidiInstrument for Synth {}

impl Synth {
    fn new(midi_channel: MidiChannel, sample_rate: usize, preset: SynthPreset) -> Self {
        Self {
            midi_channel,
            sample_rate,
            preset,
            note_to_voice: HashMap::new(),
            is_muted: false,

            debug_last_seconds: -1.0,

            ..Default::default()
        }
    }

    pub fn new_wrapped_with(
        midi_channel: MidiChannel,
        sample_rate: usize,
        preset: SynthPreset,
    ) -> Rrc<Self> {
        let wrapped = rrc(Self::new(midi_channel, sample_rate, preset));
        wrapped.borrow_mut().me = Rc::downgrade(&wrapped);
        wrapped
    }

    fn voice_for_note(&mut self, note: u8) -> Rc<RefCell<Voice>> {
        let opt = self.note_to_voice.get(&note);
        if let Some(voice) = opt {
            Rc::clone(voice)
        } else {
            let voice = Rc::new(RefCell::new(Voice::new(
                self.midi_channel(),
                self.sample_rate,
                &self.preset,
            )));
            self.note_to_voice.insert(note, Rc::clone(&voice));
            voice
        }
    }
}
impl SinksMidi for Synth {
    fn midi_channel(&self) -> MidiChannel {
        self.midi_channel
    }

    fn set_midi_channel(&mut self, midi_channel: MidiChannel) {
        self.midi_channel = midi_channel;
    }

    fn handle_midi_for_channel(&mut self, clock: &Clock, message: &MidiMessage) {
        match message.status {
            MidiMessageType::NoteOn => {
                let note = message.data1;
                let voice = self.voice_for_note(note);
                voice.borrow_mut().handle_midi_for_channel(clock, message);
            }
            MidiMessageType::NoteOff => {
                let note = message.data1;
                let voice = self.voice_for_note(note);
                voice.borrow_mut().handle_midi_for_channel(clock, message);

                // TODO: this is incorrect because it kills voices before release is complete
                self.note_to_voice.remove(&note);
            }
            MidiMessageType::ProgramChange => {
                self.preset =
                    Synth::general_midi_preset(GeneralMidiProgram::from_u8(message.data1).unwrap());
            }
        }
    }
}

impl SourcesAudio for Synth {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample {
        if clock.seconds() == self.debug_last_seconds {
            panic!();
        } else {
            self.debug_last_seconds = clock.seconds();
        }

        let mut done = true;
        let mut current_value = 0.0;
        for (_note, voice) in self.note_to_voice.iter_mut() {
            current_value += voice.borrow_mut().source_audio(clock);
            done = done && !voice.borrow().is_playing(clock);
        }
        if !self.note_to_voice.is_empty() {
            current_value /= self.note_to_voice.len() as MonoSample;
        }
        current_value
    }
}
impl IsMutable for Synth {
    fn is_muted(&self) -> bool {
        self.is_muted
    }

    fn set_muted(&mut self, is_muted: bool) {
        self.is_muted = is_muted;
    }
}

#[cfg(test)]
mod tests {
    use super::Voice;
    use super::*;
    use crate::{
        clock::Clock,
        midi::{MidiMessage, MIDI_CHANNEL_RECEIVE_ALL},
        settings::patches::{
            EnvelopePreset, FilterPreset, LfoPreset, LfoRouting, OscillatorPreset,
        },
        synthesizers::welsh::WaveformType,
        utils::tests::canonicalize_filename,
    };

    const SAMPLE_RATE: usize = 44100;

    // TODO: refactor out to common test utilities
    #[allow(dead_code)]
    fn write_voice(voice: &mut Voice, duration: f32, basename: &str) {
        let mut clock = Clock::new();

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: clock.settings().sample_rate() as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        const AMPLITUDE: MonoSample = i16::MAX as MonoSample;
        let mut writer = hound::WavWriter::create(canonicalize_filename(basename), spec).unwrap();

        let midi_on = MidiMessage::note_on_c4();
        let midi_off = MidiMessage::note_off_c4();

        let mut last_recognized_time_point = -1.;
        let time_note_off = duration / 2.0;
        while clock.seconds() < duration {
            if clock.seconds() >= 0.0 && last_recognized_time_point < 0.0 {
                last_recognized_time_point = clock.seconds();
                voice.handle_midi_for_channel(&clock, &midi_on);
            } else {
                if clock.seconds() >= time_note_off && last_recognized_time_point < time_note_off {
                    last_recognized_time_point = clock.seconds();
                    voice.handle_midi_for_channel(&clock, &midi_off);
                }
            }

            let sample = voice.source_audio(&clock);
            let _ = writer.write_sample((sample * AMPLITUDE) as i16);
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
    //                 clock.settings().sample_rate(),
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
        source: &mut Voice,
        clock: &mut Clock,
        duration: f32,
        message: &MidiMessage,
        when: f32,
        basename: &str,
    ) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: clock.settings().sample_rate() as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        const AMPLITUDE: MonoSample = i16::MAX as MonoSample;
        let mut writer = hound::WavWriter::create(canonicalize_filename(basename), spec).unwrap();

        let mut is_message_sent = false;
        while clock.seconds() < duration {
            if when <= clock.seconds() && !is_message_sent {
                is_message_sent = true;
                source.handle_midi_for_channel(clock, message);
            }
            let sample = source.source_audio(clock);
            let _ = writer.write_sample((sample * AMPLITUDE) as i16);
            clock.tick();
        }
    }

    fn cello_patch() -> SynthPreset {
        SynthPreset {
            name: SynthPreset::patch_name_to_settings_name("Cello"),
            oscillator_1_preset: OscillatorPreset {
                waveform: WaveformType::PulseWidth(0.1),
                ..Default::default()
            },
            oscillator_2_preset: OscillatorPreset {
                waveform: WaveformType::Square,
                ..Default::default()
            },
            oscillator_2_track: true,
            oscillator_2_sync: false,
            noise: 0.0,
            lfo_preset: LfoPreset {
                routing: LfoRouting::Amplitude,
                waveform: WaveformType::Sine,
                frequency: 7.5,
                depth: LfoPreset::percent(5.0),
            },
            glide: GlidePreset::Off,
            has_unison: false,
            polyphony: PolyphonyPreset::Multi,
            filter_type_24db: FilterPreset {
                cutoff: 40.0,
                weight: 0.1,
            },
            filter_type_12db: FilterPreset {
                cutoff: 40.0,
                weight: 0.1,
            },
            filter_resonance: 0.0,
            filter_envelope_weight: 0.9,
            filter_envelope_preset: EnvelopePreset {
                attack: 0.0,
                decay: 3.29,
                sustain: 0.78,
                release: EnvelopePreset::MAX,
            },
            amp_envelope_preset: EnvelopePreset {
                attack: 0.06,
                decay: EnvelopePreset::MAX,
                sustain: 1.0,
                release: 0.3,
            },
        }
    }

    fn test_patch() -> SynthPreset {
        SynthPreset {
            name: SynthPreset::patch_name_to_settings_name("Test"),
            oscillator_1_preset: OscillatorPreset {
                waveform: WaveformType::Sawtooth,
                ..Default::default()
            },
            oscillator_2_preset: OscillatorPreset {
                waveform: WaveformType::None,
                ..Default::default()
            },
            oscillator_2_track: true,
            oscillator_2_sync: false,
            noise: 0.0,
            lfo_preset: LfoPreset {
                routing: LfoRouting::None,
                ..Default::default()
            },
            glide: GlidePreset::Off,
            has_unison: false,
            polyphony: PolyphonyPreset::Multi,
            filter_type_24db: FilterPreset {
                cutoff: 40.0,
                weight: 0.1,
            },
            filter_type_12db: FilterPreset {
                cutoff: 20.0,
                weight: 0.1,
            },
            filter_resonance: 0.0,
            filter_envelope_weight: 1.0,
            filter_envelope_preset: EnvelopePreset {
                attack: 5.0,
                decay: EnvelopePreset::MAX,
                sustain: 1.0,
                release: EnvelopePreset::MAX,
            },
            amp_envelope_preset: EnvelopePreset {
                attack: 0.5,
                decay: EnvelopePreset::MAX,
                sustain: 1.0,
                release: EnvelopePreset::MAX,
            },
        }
    }

    #[test]
    fn test_basic_synth_patch() {
        let message_on = MidiMessage::note_on_c4();
        let message_off = MidiMessage::note_off_c4();

        let mut clock = Clock::new();
        let mut voice = Voice::new(MIDI_CHANNEL_RECEIVE_ALL, SAMPLE_RATE, &test_patch());
        voice.handle_midi_for_channel(&clock, &message_on);
        write_sound(
            &mut voice,
            &mut clock,
            5.0,
            &message_off,
            5.0,
            "voice_basic_test_c4",
        );
    }

    #[test]
    fn test_basic_cello_patch() {
        let message_on = MidiMessage::note_on_c4();
        let message_off = MidiMessage::note_off_c4();

        let mut clock = Clock::new();
        let mut voice = Voice::new(MIDI_CHANNEL_RECEIVE_ALL, SAMPLE_RATE, &cello_patch());
        voice.handle_midi_for_channel(&clock, &message_on);
        write_sound(
            &mut voice,
            &mut clock,
            5.0,
            &message_off,
            1.0,
            "voice_cello_c4",
        );
    }
}
