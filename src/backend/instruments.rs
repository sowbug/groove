use super::clock::Clock;
use super::devices::DeviceTrait;
use super::midi::{MidiMessage, MidiMessageType, OrderedMidiMessage};
use crate::effects::filter::AudioFilter;
use crate::effects::mixer::Mixer;
use crate::primitives::envelopes::{Envelope, EnvelopeState};
use crate::primitives::lfos::Lfo;
use crate::primitives::oscillators::{Oscillator, Waveform};
use sorted_vec::SortedVec;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Sequencer {
    midi_ticks_per_second: u32,
    sinks: Vec<Rc<RefCell<dyn DeviceTrait>>>,
    midi_messages: SortedVec<OrderedMidiMessage>,
}

impl Sequencer {
    pub fn new() -> Sequencer {
        let result = Sequencer {
            midi_ticks_per_second: 960,
            sinks: Vec::new(),
            midi_messages: SortedVec::new(),
        };
        // for channel in 0..16 {
        //     result.channels_to_sink_vecs.insert(channel, Vec::new());
        // }
        result
    }

    pub fn set_midi_ticks_per_second(&mut self, tps: u32) {
        self.midi_ticks_per_second = tps;
    }

    pub fn add_message(&mut self, message: OrderedMidiMessage) {
        self.midi_messages.insert(message);
    }
    pub fn add_note_on(&mut self, when: u32, channel: u8, which: u8) {
        let midi_message = OrderedMidiMessage {
            when,
            message: MidiMessage {
                status: MidiMessageType::NoteOn,
                channel: channel,
                data1: which,
                data2: 0,
            },
        };
        self.midi_messages.insert(midi_message);
    }
    pub fn add_note_off(&mut self, when: u32, channel: u8, which: u8) {
        let midi_message = OrderedMidiMessage {
            when,
            message: MidiMessage {
                status: MidiMessageType::NoteOff,
                channel: channel,
                data1: which,
                data2: 0,
            },
        };
        self.midi_messages.insert(midi_message);
    }

    pub fn connect_midi_sink_for_channel(
        &mut self,
        device: Rc<RefCell<dyn DeviceTrait>>,
        channel: u32,
    ) {
        // https://users.rust-lang.org/t/lots-of-references-when-using-hashmap/68754
        // discusses why we have to do strange &u32 keys.
        self.sinks.push(device);
        // let sink_vec = self.channels_to_sink_vecs.get_mut(&channel).unwrap();
        // sink_vec.push(device);
    }

    fn dispatch_midi_message(&self, midi_message: &OrderedMidiMessage, clock: &Clock) {
        for sink in self.sinks.clone() {
            sink.borrow_mut()
                .handle_midi_message(&midi_message.message, clock);
        }
        // for (channel, sink_vec) in self.channels_to_sink_vecs.iter() {
        //     if *channel == midi_message.message.channel as u32 {
        //         for one_sink in sink_vec {
        //             one_sink
        //                 .borrow_mut()
        //                 .handle_midi_message(&midi_message.message, clock);
        //         }
        //     }
        // }
    }

    pub(crate) fn tick_for_beat(&self, clock: &Clock, beat: u32) -> u32 {
        let tpb = self.midi_ticks_per_second as f32 / (clock.beats_per_minute / 60.0);
        (tpb * beat as f32) as u32
    }
}

impl DeviceTrait for Sequencer {
    fn sources_midi(&self) -> bool {
        true
    }

    fn tick(&mut self, clock: &Clock) -> bool {
        if self.midi_messages.is_empty() {
            return true;
        }
        let elapsed_midi_ticks = (clock.seconds * self.midi_ticks_per_second as f32) as u32;
        while !self.midi_messages.is_empty() {
            let midi_message = self.midi_messages.first().unwrap();

            // TODO(miket): should Clock manage elapsed_midi_ticks?
            if elapsed_midi_ticks >= midi_message.when {
                dbg!("dispatching {:?}", midi_message);
                self.dispatch_midi_message(midi_message, clock);
                self.midi_messages.remove_index(0);
            } else {
                break;
            }
        }
        false
    }

    // TODO: should this always require a channel? Or does the channel-less version mean sink all events?
    // fn connect_midi_sink(&mut self, device: Rc<RefCell<dyn DeviceTrait>>) {
    //     self.sinks[&0].push(device);
    // }
}

pub struct Voice {
    //    sound_source: Oscillator,
    envelope: Envelope,
}

impl Voice {
    pub fn new(waveform: Waveform) -> Voice {
        let sound_source = Rc::new(RefCell::new(Oscillator::new(waveform)));
        let envelope = Envelope::new(sound_source, 0.1, 0.1, 0.5, 0.3);
        Voice {
            //            sound_source,
            envelope,
        }
    }
    fn is_active(&self) -> bool {
        self.envelope.is_active()
    }
}

impl DeviceTrait for Voice {
    fn sources_audio(&self) -> bool {
        true
    }
    fn sinks_midi(&self) -> bool {
        true
    }
    fn handle_midi_message(&mut self, message: &MidiMessage, clock: &Clock) {
        self.envelope.handle_midi_message(message, clock);
    }
    fn tick(&mut self, clock: &Clock) -> bool {
        self.envelope.tick(clock)
    }
    fn get_audio_sample(&self) -> f32 {
        self.envelope.get_audio_sample()
    }
}
pub struct SimpleSynth {
    voices: Vec<Voice>,
    note_to_voice: HashMap<u8, usize>,
    channel: u32,
}

impl SimpleSynth {
    pub fn new(waveform: Waveform, channel: u32) -> SimpleSynth {
        const VOICE_COUNT: usize = 32;
        let mut synth = SimpleSynth {
            voices: Vec::new(),
            note_to_voice: HashMap::<u8, usize>::new(),
            channel,
        };
        for _ in 0..VOICE_COUNT {
            synth.voices.push(Voice::new(waveform));
        }
        synth
    }
    fn next_available_voice(&self) -> usize {
        for i in 0..self.voices.len() {
            if !self.voices[i].is_active() {
                return i;
            }
        }
        // TODO: voice stealing
        0
    }

    pub fn temp_set_oscillator_frequency(&mut self, value: f32) {
        //self.voices[0].envelope.child_device.borrow_mut().set_frequency(value);
    }
}

impl DeviceTrait for SimpleSynth {
    fn sources_audio(&self) -> bool {
        true
    }
    fn sinks_midi(&self) -> bool {
        true
    }
    fn handle_midi_message(&mut self, message: &MidiMessage, clock: &Clock) {
        if message.channel as u32 != self.channel {
            // TODO: temp, eventually put responsibility on sender to filter
            return;
        }
        match message.status {
            MidiMessageType::NoteOn => {
                let index = self.next_available_voice();
                self.voices[index].handle_midi_message(message, clock);
                self.note_to_voice.insert(message.data1, index);
            }
            MidiMessageType::NoteOff => {
                let note = message.data1;
                let index: usize = *self.note_to_voice.get(&note).unwrap();
                self.voices[index].handle_midi_message(message, clock);
                self.note_to_voice.remove(&note);
            }
        }
    }
    fn tick(&mut self, clock: &Clock) -> bool {
        let mut is_everyone_done = true;
        for voice in self.voices.iter_mut() {
            is_everyone_done = voice.tick(clock) && is_everyone_done;
        }
        is_everyone_done
    }
    fn get_audio_sample(&self) -> f32 {
        let mut total_sample = 0.;
        for voice in self.voices.iter() {
            if voice.is_active() {
                total_sample += voice.get_audio_sample();
            }
        }
        // See https://www.kvraudio.com/forum/viewtopic.php?t=529789 for one discussion of
        // how to handle polyphonic note mixing (TLDR: just sum them and deal with > 1.0 in
        // a later limiter). If we do nothing then we get hard clipping for free (see
        // https://manual.audacityteam.org/man/limiter.html for terminology).
        total_sample
    }
}

pub struct CelloSynth {
    // From Welsh's Synthesizer Cookbook, page 53
    //
    // Osc1: PW 10%, mix 100%
    // Osc2: Square, mix 100%, track on, sync off
    // noise off
    // LFO: route -> amplitude, sine, 7.5hz/moderate, depth 5%
    // glide off unison off voices multi
    // LP filter
    //   24db cutoff 40hz 10%, resonance 0%, envelope 90%
    //   12db cutoff 40hz 10%
    //   ADSR 0s, 3.29s, 78%, max
    // Amp envelope
    //   ADSR 0.06x, max, 100%, 0.30s
    //
    // alternate: osc 1 sawtooth
    osc1: Rc<RefCell<Oscillator>>,
    osc2: Rc<RefCell<Oscillator>>,
    amp_envelope: Rc<RefCell<Envelope>>,
    amp_lfo: Rc<RefCell<Lfo>>,
    filter1: Rc<RefCell<AudioFilter>>,
    filter2: Rc<RefCell<AudioFilter>>,
}

impl CelloSynth {
    pub fn new() -> Self {
        let osc1 = Rc::new(RefCell::new(Oscillator::new(Waveform::Sawtooth)));
        let osc2 = Rc::new(RefCell::new(Oscillator::new(Waveform::Square)));
        let prefilter_mixer = Rc::new(RefCell::new(Mixer::new()));
        prefilter_mixer.borrow_mut().add_audio_source(osc1.clone());
        prefilter_mixer.borrow_mut().add_audio_source(osc2.clone());

        let filter1 = Rc::new(RefCell::new(AudioFilter::new(prefilter_mixer.clone(), 0.1)));
        let filter2 = Rc::new(RefCell::new(AudioFilter::new(prefilter_mixer, 0.1)));

        let postfilter_mixer = Rc::new(RefCell::new(Mixer::new()));
        postfilter_mixer
            .borrow_mut()
            .add_audio_source(filter1.clone());
        postfilter_mixer
            .borrow_mut()
            .add_audio_source(filter2.clone());

        // TODO: this is an automation thing.
        // maybe LFOs and envelopes shouldn't have audio output, but only value outputs.
        // Then they don't have to get into the business of understanding the rest of DeviceTraits,
        // and can be reused for more things.
        let filter_envelope = Envelope::new(filter1.clone(), 0., 3.29, 0.78, 0.);
        Self {
            osc1,
            osc2,
            amp_envelope: Rc::new(RefCell::new(Envelope::new(
                postfilter_mixer,
                0.06,
                0.,
                1.0,
                0.3,
            ))),
            amp_lfo: Rc::new(RefCell::new(Lfo::new(7.5))),
            filter1,
            filter2,
        }
    }
}

impl DeviceTrait for CelloSynth {
    fn sources_audio(&self) -> bool {
        true
    }
    fn sinks_midi(&self) -> bool {
        true
    }
    fn handle_midi_message(&mut self, message: &MidiMessage, clock: &Clock) {}
    fn get_audio_sample(&self) -> f32 {
        0.
    }
}
#[derive(Default)]
struct MiniEnvelope {
    attack_seconds: f32,
    decay_seconds: f32,
    sustain_percentage: f32,
    release_seconds: f32,

    state: EnvelopeState,
    amplitude: f32,
    delta: f32,
    target: f32,
}

impl MiniEnvelope {
    pub fn new(
        attack_seconds: f32,
        decay_seconds: f32,
        sustain_percentage: f32,
        release_seconds: f32,
    ) -> Self {
        Self {
            attack_seconds,
            decay_seconds,
            sustain_percentage,
            release_seconds,
            ..Default::default()
        }
    }

    pub fn is_idle(&self) -> bool {
        matches!(self.state, EnvelopeState::Idle)
    }

    pub fn value(&self) -> f32 {
        self.amplitude
    }

    fn has_value_reached_target(&self) -> bool {
        self.amplitude == self.target
            || (self.delta > 0. && self.amplitude > self.target)
            || (self.delta < 0. && self.amplitude < self.target)
    }

    fn delta_for_transition(current: f32, target: f32, seconds: f32, ticks_per_second: f32) -> f32 {
        if seconds == 0. {
            return 0.;
        }
        (target - current) / (seconds * ticks_per_second)
    }

    fn switch_to_attack(&mut self, clock: &Clock) {
        if self.attack_seconds == 0. {
            self.switch_to_decay(clock);
        } else {
            self.state = EnvelopeState::Attack;
            self.amplitude = 0.;
            self.target = 1.;
            self.delta = MiniEnvelope::delta_for_transition(
                self.amplitude,
                self.target,
                self.attack_seconds,
                clock.sample_rate() as f32,
            );
        }
    }

    fn switch_to_decay(&mut self, clock: &Clock) {
        if self.decay_seconds == 0. {
            self.switch_to_sustain(clock);
        } else {
            self.state = EnvelopeState::Decay;
            self.target = self.sustain_percentage;
            self.delta = MiniEnvelope::delta_for_transition(
                self.amplitude,
                self.target,
                self.decay_seconds,
                clock.sample_rate() as f32,
            );
        }
    }

    fn switch_to_sustain(&mut self, clock: &Clock) {
        self.state = EnvelopeState::Sustain;
        self.target = self.sustain_percentage; // irrelevant
        self.delta = 0.;
    }

    fn switch_to_release(&mut self, clock: &Clock) {
        if self.release_seconds == 0. {
            self.switch_to_idle(clock);
        } else {
            self.state = EnvelopeState::Release;
            self.target = 0.;
            self.delta = MiniEnvelope::delta_for_transition(
                self.amplitude,
                self.target,
                self.release_seconds,
                clock.sample_rate() as f32,
            );
        }
    }

    fn switch_to_idle(&mut self, clock: &Clock) {
        self.state = EnvelopeState::Idle;
        self.amplitude = 0.;
        self.target = 0.; // irrelevant
        self.delta = 0.;
    }

    fn handle_midi_message(&mut self, message: &MidiMessage, clock: &Clock) {
        match message.status {
            MidiMessageType::NoteOn => {
                self.switch_to_attack(clock);
            }
            MidiMessageType::NoteOff => {
                if !matches!(self.state, EnvelopeState::Idle) {
                    self.switch_to_release(clock);
                } else {
                    // Already in idle state. Shouldn't happen.
                }
            }
        }
    }

    fn tick(&mut self, clock: &Clock) {
        if self.delta != 0.0 && self.target != self.amplitude {
            self.amplitude += self.delta;
        }
        match self.state {
            EnvelopeState::Idle => {}
            EnvelopeState::Attack => {
                if (self.has_value_reached_target()) {
                    self.switch_to_decay(clock);
                }
            }
            EnvelopeState::Decay => {
                if (self.has_value_reached_target()) {
                    self.switch_to_sustain(clock);
                }
            }
            EnvelopeState::Sustain => {
                // Just wait
            }
            EnvelopeState::Release => {
                if (self.has_value_reached_target()) {
                    self.switch_to_idle(clock);
                }
            }
            _ => {}
        }
    }
}

#[derive(Default)]
struct MiniFilter {
    are_coefficients_calculated: bool,
    ai0: f32,
    ai1: f32,
    ai2: f32,
    ao1: f32,
    ao2: f32,
    sample_minus_one: f32,
    sample_minus_two: f32,
    out_minus_one: f32,
    out_minus_two: f32,
}

impl MiniFilter {
    pub fn new(cutoff: f32, sample_rate: f32) -> Self {
        let mut result = Self {
            ..Default::default()
        };
        result.calc_filter_coefficients(cutoff, sample_rate);
        result
    }

    fn filter(&mut self, sample: f32) -> f32 {
        let result =
            self.ai0 * sample + self.ai1 * self.sample_minus_one + self.ai2 * self.sample_minus_two
                - self.ao1 * self.out_minus_one
                - self.ao2 * self.out_minus_two;
        self.sample_minus_two = self.sample_minus_one;
        self.sample_minus_one = sample;
        self.out_minus_two = self.out_minus_one;
        self.out_minus_one = result;
        result
    }

    fn calc_filter_coefficients(&mut self, cutoff: f32, sample_rate: f32) {
        let c = 1. / ((PI / sample_rate) * cutoff).tan();
        let c_squared = c.powi(2);
        let c_sqr2 = c * 2.0_f32.sqrt();
        let d = c_squared + c_sqr2 + 1.;
        self.ai0 = 1. / d;
        self.ai1 = self.ai0 * 2.;
        self.ai2 = self.ai0;
        self.ao1 = (2. * (1. - c_squared)) / d;
        self.ao2 = (c_squared - c_sqr2 + 1.) / d;
    }
}

use std::f32::consts::PI;
#[derive(Default)]
pub struct CelloSynth2 {
    is_playing: bool,
    frequency: f32,
    current_value: f32,

    amp_envelope: MiniEnvelope,
    filter_envelope: MiniEnvelope,

    filter_1: MiniFilter,
    filter_2: MiniFilter,
}

impl CelloSynth2 {
    const AMP_ENV_ATTACK_SECONDS: f32 = 0.06;
    const AMP_ENV_DECAY_SECONDS: f32 = 0.0;
    const AMP_ENV_SUSTAIN_PERCENTAGE: f32 = 1.;
    const AMP_ENV_RELEASE_SECONDS: f32 = 0.3;

    const FILTER_ENV_ATTACK_SECONDS: f32 = 0.06;
    const FILTER_ENV_DECAY_SECONDS: f32 = 0.0;
    const FILTER_ENV_SUSTAIN_PERCENTAGE: f32 = 1.;

    const FILTER_ENV_RELEASE_SECONDS: f32 = 0.3;

    const LFO_FREQUENCY: f32 = 7.5;
    const LFO_DEPTH: f32 = 0.05;

    pub fn new() -> Self {
        Self {
            amp_envelope: MiniEnvelope::new(
                Self::AMP_ENV_ATTACK_SECONDS,
                Self::AMP_ENV_DECAY_SECONDS,
                Self::AMP_ENV_SUSTAIN_PERCENTAGE,
                Self::AMP_ENV_RELEASE_SECONDS,
            ),
            filter_envelope: MiniEnvelope::new(
                Self::FILTER_ENV_ATTACK_SECONDS,
                Self::FILTER_ENV_DECAY_SECONDS,
                Self::FILTER_ENV_SUSTAIN_PERCENTAGE,
                Self::FILTER_ENV_RELEASE_SECONDS,
            ),
            filter_1: MiniFilter::new(40., 44100.),
            filter_2: MiniFilter::new(40., 44100.),
            ..Default::default()
        }
    }
}

impl DeviceTrait for CelloSynth2 {
    fn sources_audio(&self) -> bool {
        true
    }
    fn sinks_midi(&self) -> bool {
        true
    }

    fn handle_midi_message(&mut self, message: &MidiMessage, clock: &Clock) {
        self.amp_envelope.handle_midi_message(message, clock);
        self.filter_envelope.handle_midi_message(message, clock);
        match message.status {
            MidiMessageType::NoteOn => {
                self.is_playing = true;
                self.frequency = message.to_frequency();
            }
            MidiMessageType::NoteOff => {}
        }

        if self.amp_envelope.is_idle() {
            self.is_playing = false;
        }
    }

    fn tick(&mut self, clock: &Clock) -> bool {
        self.amp_envelope.tick(clock);
        self.filter_envelope.tick(clock);

        if self.amp_envelope.is_idle() {
            self.is_playing = false;
        }

        let phase_normalized_pitch = self.frequency * (clock.seconds as f32);

        let osc1 = {
            if self.is_playing {
                // (phase_normalized_pitch * 2.0 * PI).sin().signum()  // TODO: implement pulse-width modulation
                2.0 * (phase_normalized_pitch - (0.5 + phase_normalized_pitch).floor())
                // Welsh's says sawtooth is acceptable substitution
            } else {
                0.
            }
        };
        let osc2 = {
            if self.is_playing {
                (phase_normalized_pitch * 2.0 * PI).sin().signum()
            } else {
                0.
            }
        };
        let phase_normalized_lfo = Self::LFO_FREQUENCY * (clock.seconds as f32);
        let lfo = (phase_normalized_lfo * 2.0 * PI).sin();

        let osc_mix = (osc1 + osc2) / 2.;

        let filter_1_weight = 0.1 * self.filter_envelope.value();
        let filter_2_weight = 0.1 * self.filter_envelope.value();
        let filter1 =
            self.filter_1.filter(osc_mix) * filter_1_weight + osc_mix * (1.0 - filter_1_weight);
        let filter2 =
            self.filter_2.filter(osc_mix) * filter_2_weight + osc_mix * (1.0 - filter_2_weight);
        let filter_mix = (filter1 + filter2) / 2.;

        let amplitude = self.amp_envelope.value()
            * filter_mix
            * (1. + lfo * Self::LFO_DEPTH - Self::LFO_DEPTH / 2.0);

        self.current_value = amplitude;

        // TODO temp
        self.amp_envelope.is_idle()
    }
    fn get_audio_sample(&self) -> f32 {
        self.current_value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct NullDevice {
        is_playing: bool,
        midi_channel: u8,
        midi_messages_received: usize,
        midi_messages_handled: usize,
    }

    impl NullDevice {
        fn new() -> NullDevice {
            NullDevice {
                ..Default::default()
            }
        }
        fn set_channel(&mut self, channel: u8) {
            self.midi_channel = channel;
        }
    }
    impl DeviceTrait for NullDevice {
        fn sinks_midi(&self) -> bool {
            true
        }
        fn handle_midi_message(&mut self, message: &MidiMessage, clock: &Clock) {
            self.midi_messages_received += 1;

            // TODO: be more efficient about this -- don't dispatch in the first place!
            if message.channel != self.midi_channel {
                return;
            }

            match message.status {
                MidiMessageType::NoteOn => {
                    self.is_playing = true;
                    self.midi_messages_handled += 1;
                }
                MidiMessageType::NoteOff => {
                    self.is_playing = false;
                    self.midi_messages_handled += 1;
                }
            }
        }
    }

    fn advance_one_beat(clock: &mut Clock, sequencer: &mut Sequencer) {
        let old_time = clock.seconds;
        let beat = clock.beats;
        while clock.beats == beat {
            clock.tick();
            sequencer.tick(&clock);
        }
        dbg!("Beat clock is now {} {}", beat, clock.beats);
        dbg!("Time clock is now {} {}", old_time, clock.seconds);
        let _d = true;
    }

    #[test]
    fn test_sequencer() {
        const SAMPLES_PER_SECOND: u32 = 256;
        let mut clock = Clock::new(SAMPLES_PER_SECOND, 4, 4, 128.);
        let mut sequencer = Sequencer::new();
        assert!(sequencer.sources_midi());
        assert!(!sequencer.sources_audio());

        let device = Rc::new(RefCell::new(NullDevice::new()));
        assert!(!device.borrow().is_playing);

        sequencer.add_note_on(sequencer.tick_for_beat(&clock, 0), 0, 60);
        sequencer.add_note_off(sequencer.tick_for_beat(&clock, 1), 0, 60);

        sequencer.connect_midi_sink_for_channel(device.clone(), 0);

        sequencer.tick(&clock);
        {
            let dp = device.borrow();
            assert!(dp.is_playing);
            assert_eq!(dp.midi_messages_received, 1);
            assert_eq!(dp.midi_messages_handled, 1);
        }

        advance_one_beat(&mut clock, &mut sequencer);
        {
            let dp = device.borrow();
            assert!(!dp.is_playing);
            assert_eq!(dp.midi_messages_received, 2);
            assert_eq!(dp.midi_messages_handled, 2);
        }
    }

    #[test]
    fn test_sequencer_multichannel() {
        const SAMPLES_PER_SECOND: u32 = 256;
        let mut clock = Clock::new(SAMPLES_PER_SECOND, 4, 4, 128.);
        let mut sequencer = Sequencer::new();
        assert!(sequencer.sources_midi());
        assert!(!sequencer.sources_audio());

        let device_1 = Rc::new(RefCell::new(NullDevice::new()));
        assert!(!device_1.borrow().is_playing);
        device_1.borrow_mut().set_channel(0);
        sequencer.connect_midi_sink_for_channel(device_1.clone(), 0);

        let device_2 = Rc::new(RefCell::new(NullDevice::new()));
        assert!(!device_2.borrow().is_playing);
        device_2.borrow_mut().set_channel(1);
        sequencer.connect_midi_sink_for_channel(device_2.clone(), 1);

        sequencer.add_note_on(sequencer.tick_for_beat(&clock, 0), 0, 60);
        sequencer.add_note_on(sequencer.tick_for_beat(&clock, 1), 1, 60);
        sequencer.add_note_off(sequencer.tick_for_beat(&clock, 2), 0, 60);
        sequencer.add_note_off(sequencer.tick_for_beat(&clock, 3), 1, 60);

        // TODO: this tick() doesn't match the Clock tick() in the sense that the clock is in the right state
        // right after init (without tick()), but the sequencer isn't (needs tick()). Maybe they shouldn't both
        // be called tick().
        assert_eq!(sequencer.midi_messages.len(), 4);
        sequencer.tick(&clock);
        assert_eq!(clock.beats, 0);
        assert_eq!(sequencer.midi_messages.len(), 3);
        {
            let dp_1 = device_1.borrow();
            assert!(dp_1.is_playing);
            assert_eq!(dp_1.midi_messages_received, 1);
            assert_eq!(dp_1.midi_messages_handled, 1);

            let dp_2 = device_2.borrow();
            assert!(!dp_2.is_playing);
            assert_eq!(dp_2.midi_messages_received, 1); // TODO: this should be 0 to indicate the sequencer is directing messages only to the listening devices.
            assert_eq!(dp_2.midi_messages_handled, 0);
        }

        advance_one_beat(&mut clock, &mut sequencer);
        assert_eq!(clock.beats, 1);
        assert_eq!(sequencer.midi_messages.len(), 2);
        {
            let dp = device_1.borrow();
            assert!(dp.is_playing);
            assert_eq!(dp.midi_messages_received, 2);
            assert_eq!(dp.midi_messages_handled, 1);

            let dp_2 = device_2.borrow();
            assert!(dp_2.is_playing);
            assert_eq!(dp_2.midi_messages_received, 2);
            assert_eq!(dp_2.midi_messages_handled, 1);
        }

        advance_one_beat(&mut clock, &mut sequencer);
        assert_eq!(clock.beats, 2);
        assert_eq!(sequencer.midi_messages.len(), 1);
        {
            let dp = device_1.borrow();
            assert!(!dp.is_playing);
            assert_eq!(dp.midi_messages_received, 3);
            assert_eq!(dp.midi_messages_handled, 2);

            let dp_2 = device_2.borrow();
            assert!(dp_2.is_playing);
            assert_eq!(dp_2.midi_messages_received, 3);
            assert_eq!(dp_2.midi_messages_handled, 1);
        }

        advance_one_beat(&mut clock, &mut sequencer);
        assert_eq!(clock.beats, 3);
        assert_eq!(sequencer.midi_messages.len(), 0);
        {
            let dp = device_1.borrow();
            assert!(!dp.is_playing);
            assert_eq!(dp.midi_messages_received, 4);
            assert_eq!(dp.midi_messages_handled, 2);

            let dp_2 = device_2.borrow();
            assert!(!dp_2.is_playing);
            assert_eq!(dp_2.midi_messages_received, 4);
            assert_eq!(dp_2.midi_messages_handled, 2);
        }
    }
}
