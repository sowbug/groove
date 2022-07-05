use super::clock::Clock;
use super::devices::DeviceTrait;
use super::midi::{MidiMessage, MidiMessageType, OrderedMidiMessage};
use crate::backend::midi;
use sorted_vec::SortedVec;
use std::cell::RefCell;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::rc::Rc;

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum Waveform {
    Sine,
    Square,
    Triangle,
    Sawtooth,
}

impl Default for Waveform {
    fn default() -> Self {
        Waveform::Sine
    }
}

#[derive(Default)]
pub struct Oscillator {
    waveform: Waveform,
    current_sample: f32,
    frequency: f32,
}

// TODO: these oscillators are pure in a logical sense, but they alias badly in the real world
// of discrete sampling. Investigate replacing with smoothed waveforms.
impl Oscillator {
    pub fn new(waveform: Waveform) -> Oscillator {
        Oscillator {
            waveform,
            ..Default::default()
        }
    }
    pub fn set_frequency(&mut self, frequency: f32) {
        self.frequency = frequency;
    }
}
impl DeviceTrait for Oscillator {
    fn sinks_midi(&self) -> bool {
        true
    }
    fn sources_audio(&self) -> bool {
        true
    }
    fn tick(&mut self, clock: &Clock) -> bool {
        if self.frequency > 0. {
            let phase_normalized = self.frequency * (clock.seconds as f32);
            self.current_sample = match self.waveform {
                // https://en.wikipedia.org/wiki/Sine_wave
                Waveform::Sine => (phase_normalized * 2.0 * PI).sin(),
                // https://en.wikipedia.org/wiki/Square_wave
                Waveform::Square => (phase_normalized * 2.0 * PI).sin().signum(),
                // https://en.wikipedia.org/wiki/Triangle_wave
                Waveform::Triangle => {
                    4.0 * (phase_normalized - (0.75 + phase_normalized).floor() + 0.25).abs() - 1.0
                }
                // https://en.wikipedia.org/wiki/Sawtooth_wave
                Waveform::Sawtooth => 2.0 * (phase_normalized - (0.5 + phase_normalized).floor()),
            }
        } else {
            self.current_sample = 0.
        }
        true
    }
    fn handle_midi_message(&mut self, message: &MidiMessage, _clock: &Clock) {
        match message.status {
            midi::MidiMessageType::NoteOn => {
                self.frequency = message.to_frequency();
            }
            midi::MidiMessageType::NoteOff => {
                // TODO(miket): now that oscillators are in envelopes, they generally turn on but don't turn off.
                // these might not end up being full DeviceTrait devices, but rather owned/managed by synths.
                //self.frequency = 0.;
            }
        }
    }

    fn get_audio_sample(&self) -> f32 {
        self.current_sample
    }
}

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

enum EnvelopeState {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}
pub struct Envelope {
    child_device: Oscillator,
    amplitude: f32,
    amplitude_delta: f32,
    amplitude_target: f32,
    attack: f32,  // seconds
    decay: f32,   // seconds
    sustain: f32, // amplitude
    release: f32, // seconds

    state: EnvelopeState,
}

impl<'a> Envelope {
    pub fn new(
        child_device: Oscillator,
        attack: f32,
        decay: f32,
        sustain: f32,
        release: f32,
    ) -> Envelope {
        if !child_device.sources_audio() {
            panic!("Envelope created with non-audio-producing child device");
        }
        Envelope {
            child_device,
            amplitude: 0.,
            amplitude_delta: 0.,
            amplitude_target: 0.,
            attack,
            decay,
            sustain,
            release,
            state: EnvelopeState::Idle,
        }
    }

    fn update_amplitude_delta(&mut self, target: f32, state_duration: f32, clock: &Clock) {
        self.amplitude_target = target;
        if state_duration > 0. {
            self.amplitude_delta = (self.amplitude_target - self.amplitude)
                / (state_duration * clock.sample_rate() as f32);
        } else {
            self.amplitude_delta = self.amplitude_target - self.amplitude;
        }
    }

    fn change_to_attack_state(&mut self, clock: &Clock) {
        self.state = EnvelopeState::Attack;
        self.amplitude = 0.;
        self.update_amplitude_delta(1.0, self.attack, clock);
    }

    fn change_to_decay_state(&mut self, clock: &Clock) {
        self.state = EnvelopeState::Decay;
        self.amplitude = 1.;
        self.update_amplitude_delta(self.sustain, self.decay, clock);
    }

    fn change_to_sustain_state(&mut self, _clock: &Clock) {
        self.state = EnvelopeState::Sustain;
        self.amplitude = self.sustain;
        self.amplitude_target = self.sustain;
        self.amplitude_delta = 0.;
    }

    fn change_to_release_state(&mut self, clock: &Clock) {
        self.state = EnvelopeState::Release;
        self.update_amplitude_delta(0., self.release, clock);
    }

    fn change_to_idle_state(&mut self, _clock: &Clock) {
        self.state = EnvelopeState::Idle;
        self.amplitude = 0.;
        self.amplitude_delta = 0.;
    }

    fn has_amplitude_reached_target(&self) -> bool {
        (self.amplitude == self.amplitude_target)
            || (self.amplitude_delta < 0. && self.amplitude < self.amplitude_target)
            || (self.amplitude_delta > 0. && self.amplitude > self.amplitude_target)
    }

    fn is_active(&self) -> bool {
        !matches!(self.state, EnvelopeState::Idle)
    }
}

impl<'a> DeviceTrait for Envelope {
    fn sources_midi(&self) -> bool {
        true
    }

    fn tick(&mut self, clock: &Clock) -> bool {
        self.amplitude += self.amplitude_delta;
        if self.has_amplitude_reached_target() {
            match self.state {
                EnvelopeState::Idle => {
                    // Nothing to do but wait for note on
                }
                EnvelopeState::Attack => {
                    self.change_to_decay_state(clock);
                }
                EnvelopeState::Decay => {
                    self.change_to_sustain_state(clock);
                }
                EnvelopeState::Sustain => {
                    // Nothing to do but wait for note off
                }
                EnvelopeState::Release => {
                    self.change_to_idle_state(clock);
                }
            }
        }
        // TODO(miket): introduce notion of weak ref so that we can make sure nobody has two parents
        self.child_device.tick(clock);

        matches!(self.state, EnvelopeState::Idle)
    }

    fn get_audio_sample(&self) -> f32 {
        self.child_device.get_audio_sample() * self.amplitude
    }

    fn handle_midi_message(&mut self, message: &MidiMessage, clock: &Clock) {
        match message.status {
            MidiMessageType::NoteOn => {
                self.change_to_attack_state(clock);
            }
            MidiMessageType::NoteOff => {
                self.change_to_release_state(clock);
            }
        }
        self.child_device.handle_midi_message(message, clock);
    }
}

pub struct Voice {
    //    sound_source: Oscillator,
    envelope: Envelope,
}

impl Voice {
    pub fn new(waveform: Waveform) -> Voice {
        let sound_source = Oscillator::new(waveform);
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
