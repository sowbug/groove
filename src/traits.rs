use crate::{
    clock::{Clock, ClockTimeUnit},
    common::{F32ControlValue, Sample, StereoSample},
    instruments::{oscillators::Oscillator, Dca},
    messages::EntityMessage,
    midi::{MidiChannel, MidiUtils},
    settings::patches::WaveformType,
    ClockSettings,
};
use groove_macros::Control;
use midly::MidiMessage;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::{marker::PhantomData, str::FromStr};
use strum_macros::{Display, EnumString, FromRepr};

/// An IsController controls things in the system that implement Controllable.
/// Examples are sequencers, arpeggiators, and discrete LFOs (as contrasted with
/// LFOs that are integrated into other instruments).
///
/// An IsController implements Terminates, which indicates that it's done
/// emitting events (and, in the case of timers and sequencers, done waiting for
/// other work in the system to complete).
///
/// An IsController necessarily implements TicksWithMessages, rather than just
/// Ticks, because messages are how controllers control other things in the
/// system.
pub trait IsController: TicksWithMessages + HandlesMidi + HasUid + Send + Debug {}

/// An IsEffect transforms audio. It takes audio inputs and produces audio
/// output. It does not get called unless there is audio input to provide to it
/// (which can include silence, e.g., in the case of a muted instrument).
///
/// IsEffects don't implement Terminates. They process audio indefinitely, and
/// don't have a sense of the length of the performance.
pub trait IsEffect: TransformsAudio + Controllable + HasUid + Send + Debug {}

/// An IsInstrument produces audio, usually upon request from MIDI or
/// IsController input. Like IsEffect, IsInstrument doesn't implement Terminates
/// because it continues to create audio as long as asked.
pub trait IsInstrument:
    Generates<StereoSample> + Ticks + HandlesMidi + Controllable + HasUid + Send + Debug
{
}

pub trait Generates<V>: Send + Debug + Ticks {
    /// The value for the current frame. Advance the frame by calling
    /// Ticks::tick().
    fn value(&self) -> V;

    /// The batch version of value(). To deliver each value, this method will
    /// typically call tick() internally. If you don't want this, then call
    /// value() on your own.
    fn batch_values(&mut self, values: &mut [V]);
}

/// Something that is Controllable exposes a set of attributes, each with a text
/// name, that IsControllers can change. If you're familiar with DAWs, this is
/// typically called "automation."
///
/// The Controllable trait is more powerful than ordinary getters/setters
/// because it allows runtime binding of an IsController to a Controllable.
pub trait Controllable {
    #[allow(unused_variables)]
    fn control_index_for_name(&self, name: &str) -> usize {
        unimplemented!()
    }
    #[allow(unused_variables)]
    fn set_by_control_index(&mut self, index: usize, value: F32ControlValue) {
        unimplemented!()
    }
}

/// Takes standard MIDI messages. Implementers can ignore MidiChannel if it's
/// not important, as the virtual cabling model tries to route only relevant
/// traffic to individual devices.
pub trait HandlesMidi {
    #[allow(unused_variables)]
    fn handle_midi_message(
        &mut self,
        message: &MidiMessage,
    ) -> Option<Vec<(MidiChannel, MidiMessage)>> {
        None
    }
}

/// A HasUid has an ephemeral but globally unique numeric identifier, which is
/// useful for one entity to refer to another without getting into icky Rust
/// ownership questions. It's the foundation of any ECS
/// (entity/component/system) design.
pub trait HasUid {
    fn uid(&self) -> usize;
    fn set_uid(&mut self, uid: usize);
}

/// Something that Resets also either Ticks or TicksWithMessages. Since the
/// Ticks family of traits don't get access to a global clock, they have to
/// maintain internal clocks and trust that they'll be asked to tick exactly the
/// same number of times as everyone else in the system. Resets::reset() ensures
/// that everyone starts from the beginning at the same time, and that everyone
/// agrees how long a tick lasts.
///
/// Sometimes we'll refer to a tick's "time slice" or "frame." These all mean
/// the same thing.
pub trait Resets {
    /// The entity should reset its internal state.
    ///
    /// The system will call reset() when the global sample rate changes, and
    /// whenever the global clock is reset. Since most entities that care about
    /// sample rate need to know it during construction, the system *won't* call
    /// reset() on entity construction; entities can require the sample rate as
    /// part of their new() functions, and if desired call reset() within that
    /// function.
    #[allow(unused_variables)]
    fn reset(&mut self, sample_rate: usize) {}
}

pub trait Ticks: Resets + Send + Debug {
    /// The entity should perform work for the current frame or frames. Under
    /// normal circumstances, successive tick()s represent successive frames.
    /// Exceptions include, for example, restarting a performance, which would
    /// reset the global clock, which the entity learns about via reset().
    ///
    /// Entities are responsible for tracking their own notion of time, which
    /// they should update during tick().
    ///
    /// tick() guarantees that any state for the current frame is valid *after*
    /// tick() has been called for the current frame. This means that Ticks
    /// implementers must treat the first frame as special. Normally, entity
    /// state is correct for the first frame after entity construction, so
    /// tick() must be careful not to update state on the first frame, because
    /// that would cause the state to represent the second frame, not the first.
    fn tick(&mut self, tick_count: usize);
}

pub trait TicksWithMessages: Resets + Send + Debug {
    /// Similar to Ticks::tick().
    ///
    /// Returns zero or more EntityMessages.
    ///
    /// Returns the number of requested ticks handled before terminating.
    fn tick(&mut self, tick_count: usize) -> (Option<Vec<EntityMessage>>, usize);
}

/// A TransformsAudio takes input audio, which is typically produced by
/// SourcesAudio, does something to it, and then outputs it. It's what effects
/// do.
pub trait TransformsAudio: Debug {
    fn transform_audio(&mut self, input_sample: StereoSample) -> StereoSample {
        // Beware: converting from mono to stereo isn't just doing the work
        // twice! You'll also have to double whatever state you maintain from
        // tick to tick that has to do with a single channel's audio data.
        StereoSample(
            self.transform_channel(0, input_sample.0),
            self.transform_channel(1, input_sample.1),
        )
    }

    /// channel: 0 is left, 1 is right. Use the value as an index into arrays.
    fn transform_channel(&mut self, channel: usize, input_sample: Sample) -> Sample;
}

#[derive(Debug)]
pub struct Response<T>(pub Internal<T>);

#[derive(Debug)]
pub enum Internal<T> {
    None,
    Single(T),
    Batch(Vec<T>),
}

impl<T> Response<T> {
    pub const fn none() -> Self {
        Self(Internal::None)
    }

    pub const fn single(action: T) -> Self {
        Self(Internal::Single(action))
    }

    pub fn batch(commands: impl IntoIterator<Item = Response<T>>) -> Self {
        let mut batch = Vec::new();

        for Response(command) in commands {
            match command {
                Internal::None => {}
                Internal::Single(command) => batch.push(command),
                Internal::Batch(commands) => batch.extend(commands),
            }
        }
        if batch.is_empty() {
            Self(Internal::None)
        } else {
            Self(Internal::Batch(batch))
        }
    }
}

// NOTE: The Test... entities are in the non-tests module because they're
// sometimes useful as simple real entities to substitute in for production
// ones, for example if we're trying to determine whether an entity is
// responsible for a performance issue.

// TODO: redesign this for clockless operation
// pub trait TestsValues {
//     fn check_values(&mut self, clock: &Clock) {
//         // If we've been asked to assert values at checkpoints, do so.
//         if self.has_checkpoint_values()
//             && clock.time_for(self.time_unit()) >= self.checkpoint_time()
//         {
//             const SAD_FLOAT_DIFF: f32 = 1.0e-4;
//             if let Some(value) = self.pop_checkpoint_value() {
//                 assert_approx_eq!(self.value_to_check(), value, SAD_FLOAT_DIFF);
//             }
//             self.advance_checkpoint_time();
//         }
//     }

//     fn has_checkpoint_values(&self) -> bool;
//     fn time_unit(&self) -> &ClockTimeUnit;
//     fn checkpoint_time(&self) -> f32;
//     fn advance_checkpoint_time(&mut self);
//     fn value_to_check(&self) -> f32;
//     fn pop_checkpoint_value(&mut self) -> Option<f32>;
// }

#[derive(Display, Debug, EnumString)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum TestControllerControlParams {
    Tempo,
}

enum TestControllerAction {
    Nothing,
    NoteOn,
    NoteOff,
}

#[derive(Debug)]
pub struct TestController {
    uid: usize,
    midi_channel_out: MidiChannel,

    clock_settings: ClockSettings,
    clock: Clock,

    pub tempo: f32,
    is_enabled: bool,
    is_playing: bool,

    pub checkpoint_values: VecDeque<f32>,
    pub checkpoint: f32,
    pub checkpoint_delta: f32,
    pub time_unit: ClockTimeUnit,
}
impl IsController for TestController {}
impl TicksWithMessages for TestController {
    fn tick(&mut self, tick_count: usize) -> (Option<Vec<EntityMessage>>, usize) {
        let mut v = Vec::default();
        for _ in 0..tick_count {
            self.clock.tick(1);
            // TODO self.check_values(clock);

            match self.what_to_do() {
                TestControllerAction::Nothing => {}
                TestControllerAction::NoteOn => {
                    // This is elegant, I hope. If the arpeggiator is
                    // disabled during play, and we were playing a note,
                    // then we still send the off note,
                    if self.is_enabled {
                        self.is_playing = true;
                        v.push(EntityMessage::Midi(
                            self.midi_channel_out,
                            MidiUtils::new_note_on(60, 127),
                        ));
                    }
                }
                TestControllerAction::NoteOff => {
                    if self.is_playing {
                        v.push(EntityMessage::Midi(
                            self.midi_channel_out,
                            MidiUtils::new_note_off(60, 0),
                        ));
                    }
                }
            }
        }
        if v.is_empty() {
            (None, 0)
        } else {
            (Some(v), 0)
        }
    }
}
impl Resets for TestController {
    fn reset(&mut self, sample_rate: usize) {
        self.clock_settings.set_sample_rate(sample_rate);
        self.clock = Clock::new_with(&self.clock_settings);
    }
}
impl HandlesMidi for TestController {
    fn handle_midi_message(
        &mut self,
        message: &MidiMessage,
    ) -> Option<Vec<(MidiChannel, MidiMessage)>> {
        #[allow(unused_variables)]
        match message {
            MidiMessage::NoteOff { key, vel } => self.is_enabled = false,
            MidiMessage::NoteOn { key, vel } => self.is_enabled = true,
            _ => todo!(),
        }
        None
    }
}
impl HasUid for TestController {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid
    }
}
impl TestController {
    pub fn new_with(clock_settings: &ClockSettings, midi_channel_out: MidiChannel) -> Self {
        Self::new_with_test_values(
            clock_settings,
            midi_channel_out,
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        )
    }

    pub fn new_with_test_values(
        clock_settings: &ClockSettings,
        midi_channel_out: MidiChannel,
        values: &[f32],
        checkpoint: f32,
        checkpoint_delta: f32,
        time_unit: ClockTimeUnit,
    ) -> Self {
        Self {
            uid: Default::default(),
            midi_channel_out,
            clock_settings: clock_settings.clone(),
            clock: Clock::new_with(clock_settings),
            tempo: Default::default(),
            is_enabled: Default::default(),
            is_playing: Default::default(),
            checkpoint_values: VecDeque::from(Vec::from(values)),
            checkpoint,
            checkpoint_delta,
            time_unit,
        }
    }

    fn what_to_do(&self) -> TestControllerAction {
        let beat_slice_start = self.clock.beats();
        let beat_slice_end = self.clock.next_slice_in_beats();
        let next_exact_beat = beat_slice_start.floor();
        let next_exact_half_beat = next_exact_beat + 0.5;
        if next_exact_beat >= beat_slice_start && next_exact_beat < beat_slice_end {
            return TestControllerAction::NoteOn;
        }
        if next_exact_half_beat >= beat_slice_start && next_exact_half_beat < beat_slice_end {
            return TestControllerAction::NoteOff;
        }
        TestControllerAction::Nothing
    }
}
// impl TestsValues for TestController {
//     fn has_checkpoint_values(&self) -> bool {
//         !self.checkpoint_values.is_empty()
//     }

//     fn time_unit(&self) -> &ClockTimeUnit {
//         &self.time_unit
//     }

//     fn checkpoint_time(&self) -> f32 {
//         self.checkpoint
//     }

//     fn advance_checkpoint_time(&mut self) {
//         self.checkpoint += self.checkpoint_delta;
//     }

//     fn value_to_check(&self) -> f32 {
//         self.tempo
//     }

//     fn pop_checkpoint_value(&mut self) -> Option<f32> {
//         self.checkpoint_values.pop_front()
//     }
// }

#[derive(Control, Debug, Default)]
pub struct TestEffect {
    uid: usize,

    #[controllable]
    my_value: f32,

    pub checkpoint_values: VecDeque<f32>,
    pub checkpoint: f32,
    pub checkpoint_delta: f32,
    pub time_unit: ClockTimeUnit,
}
impl IsEffect for TestEffect {}
impl TransformsAudio for TestEffect {
    fn transform_channel(&mut self, _channel: usize, input_sample: Sample) -> Sample {
        /////////////////////// TODO        self.check_values(clock);
        -input_sample
    }
}
impl HasUid for TestEffect {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}
// impl TestsValues for TestEffect {
//     fn has_checkpoint_values(&self) -> bool {
//         !self.checkpoint_values.is_empty()
//     }

//     fn time_unit(&self) -> &ClockTimeUnit {
//         &self.time_unit
//     }

//     fn checkpoint_time(&self) -> f32 {
//         self.checkpoint
//     }

//     fn advance_checkpoint_time(&mut self) {
//         self.checkpoint += self.checkpoint_delta;
//     }

//     fn value_to_check(&self) -> f32 {
//         self.my_value()
//     }

//     fn pop_checkpoint_value(&mut self) -> Option<f32> {
//         self.checkpoint_values.pop_front()
//     }
// }
impl TestEffect {
    pub fn new_with_test_values(
        values: &[f32],
        checkpoint: f32,
        checkpoint_delta: f32,
        time_unit: ClockTimeUnit,
    ) -> Self {
        Self {
            checkpoint_values: VecDeque::from(Vec::from(values)),
            checkpoint,
            checkpoint_delta,
            time_unit,
            ..Default::default()
        }
    }

    pub fn set_my_value(&mut self, my_value: f32) {
        self.my_value = my_value;
    }

    pub fn my_value(&self) -> f32 {
        self.my_value
    }

    pub(crate) fn set_control_my_value(&mut self, my_value: F32ControlValue) {
        self.set_my_value(my_value.0);
    }
}

/// A simple implementation of IsInstrument that's useful for testing and
/// debugging. Uses a default Oscillator to produce sound, and its "envelope" is
/// just a boolean that responds to MIDI NoteOn/NoteOff.
///
/// To act as a controller target, it has two parameters: Oscillator waveform
/// and frequency.
#[derive(Control, Debug)]
pub struct TestInstrument {
    uid: usize,
    sample_rate: usize,
    sample: StereoSample,

    /// -1.0 is Sawtooth, 1.0 is Square, anything else is Sine.
    #[controllable]
    pub waveform: PhantomData<WaveformType>, // interesting use of PhantomData

    #[controllable]
    pub fake_value: f32,

    oscillator: Oscillator,
    dca: Dca,
    pub is_playing: bool,
    pub received_count: usize,
    pub handled_count: usize,

    pub checkpoint_values: VecDeque<f32>,
    pub checkpoint: f32,
    pub checkpoint_delta: f32,
    pub time_unit: ClockTimeUnit,

    pub debug_messages: Vec<MidiMessage>,
}
impl IsInstrument for TestInstrument {}
impl Generates<StereoSample> for TestInstrument {
    fn value(&self) -> StereoSample {
        self.sample
    }

    #[allow(unused_variables)]
    fn batch_values(&mut self, values: &mut [StereoSample]) {
        todo!()
    }
}
impl Resets for TestInstrument {
    fn reset(&mut self, sample_rate: usize) {
        self.oscillator.reset(sample_rate);
    }
}
impl Ticks for TestInstrument {
    fn tick(&mut self, tick_count: usize) {
        self.oscillator.tick(tick_count);
        // If we've been asked to assert values at checkpoints, do so.

        // TODODODODO
        // if !self.checkpoint_values.is_empty() && clock.time_for(&self.time_unit) >= self.checkpoint
        // {
        //     const SAD_FLOAT_DIFF: f32 = 1.0e-2;
        //     assert_approx_eq!(self.fake_value, self.checkpoint_values[0], SAD_FLOAT_DIFF);
        //     self.checkpoint += self.checkpoint_delta;
        //     self.checkpoint_values.pop_front();
        // }
        self.sample = if self.is_playing {
            self.dca
                .transform_audio_to_stereo(Sample::from(self.oscillator.value()))
        } else {
            StereoSample::SILENCE
        };
    }
}
impl HasUid for TestInstrument {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}
impl HandlesMidi for TestInstrument {
    fn handle_midi_message(
        &mut self,
        message: &MidiMessage,
    ) -> Option<Vec<(MidiChannel, MidiMessage)>> {
        self.debug_messages.push(*message);
        self.received_count += 1;

        match message {
            MidiMessage::NoteOn { key, vel: _ } => {
                self.is_playing = true;
                self.oscillator
                    .set_frequency(MidiUtils::note_to_frequency(key.as_int()));
            }
            MidiMessage::NoteOff { key: _, vel: _ } => {
                self.is_playing = false;
            }
            _ => {}
        }
        None
    }
}
// impl TestsValues for TestInstrument {
//     fn has_checkpoint_values(&self) -> bool {
//         !self.checkpoint_values.is_empty()
//     }

//     fn time_unit(&self) -> &ClockTimeUnit {
//         &self.time_unit
//     }

//     fn checkpoint_time(&self) -> f32 {
//         self.checkpoint
//     }

//     fn advance_checkpoint_time(&mut self) {
//         self.checkpoint += self.checkpoint_delta;
//     }

//     fn value_to_check(&self) -> f32 {
//         self.fake_value
//     }

//     fn pop_checkpoint_value(&mut self) -> Option<f32> {
//         self.checkpoint_values.pop_front()
//     }
// }
impl TestInstrument {
    pub fn new_with(sample_rate: usize) -> Self {
        let mut r = Self {
            uid: Default::default(),
            waveform: Default::default(),
            sample_rate,
            sample: Default::default(),
            fake_value: Default::default(),
            oscillator: Oscillator::new_with(sample_rate),
            dca: Default::default(),
            is_playing: Default::default(),
            received_count: Default::default(),
            handled_count: Default::default(),
            checkpoint_values: Default::default(),
            checkpoint: Default::default(),
            checkpoint_delta: Default::default(),
            time_unit: Default::default(),
            debug_messages: Default::default(),
        };
        r.sample_rate = sample_rate;

        r
    }

    pub fn new_with_test_values(
        sample_rate: usize,
        values: &[f32],
        checkpoint: f32,
        checkpoint_delta: f32,
        time_unit: ClockTimeUnit,
    ) -> Self {
        let mut r = Self::new_with(sample_rate);
        r.checkpoint_values = VecDeque::from(Vec::from(values));
        r.checkpoint = checkpoint;
        r.checkpoint_delta = checkpoint_delta;
        r.time_unit = time_unit;
        r
    }

    #[allow(dead_code)]
    fn waveform(&self) -> f32 {
        match self.oscillator.waveform() {
            WaveformType::Sawtooth => -1.0,
            WaveformType::Square => 1.0,
            _ => 0.0,
        }
    }

    pub fn set_control_waveform(&mut self, value: F32ControlValue) {
        self.oscillator.set_waveform(if value.0 == -1.0 {
            WaveformType::Sawtooth
        } else if value.0 == 1.0 {
            WaveformType::Square
        } else {
            WaveformType::Sine
        });
    }

    pub fn set_fake_value(&mut self, fake_value: f32) {
        self.fake_value = fake_value;
    }

    pub fn fake_value(&self) -> f32 {
        self.fake_value
    }

    pub(crate) fn set_control_fake_value(&mut self, fake_value: F32ControlValue) {
        self.set_fake_value(fake_value.0);
    }
}

#[cfg(test)]
pub mod tests {
    use super::{Generates, TestInstrument, Ticks};
    use crate::common::DEFAULT_SAMPLE_RATE;
    use rand::random;

    pub trait DebugTicks: Ticks {
        fn debug_tick_until(&mut self, tick_number: usize);
    }

    // TODO: restore tests that test basic trait behavior, then figure out how
    // to run everyone implementing those traits through that behavior. For now,
    // this one just tests that a generic instrument doesn't panic when accessed
    // for non-consecutive time slices.
    #[test]
    fn test_sources_audio_random_access() {
        let mut instrument = TestInstrument::new_with(DEFAULT_SAMPLE_RATE);
        for _ in 0..100 {
            instrument.tick(random::<usize>() % 10);
            let _ = instrument.value();
        }
    }

    impl TestInstrument {
        pub fn dump_messages(&self) {
            dbg!(&self.debug_messages);
        }
    }
}
