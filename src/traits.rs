use crate::clock::Clock;
use crate::clock::ClockTimeUnit;
use crate::common::{F32ControlValue, Sample, StereoSample};
use crate::instruments::oscillators::GeneratesSignal;
use crate::instruments::{Dca, HandlesMidi};
use crate::messages::EntityMessage;
use crate::{
    instruments::oscillators::Oscillator,
    midi::{MidiChannel, MidiUtils},
    settings::patches::WaveformType,
};
use assert_approx_eq::assert_approx_eq;
use groove_macros::Control;
use midly::MidiMessage;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::{marker::PhantomData, str::FromStr};
use strum_macros::{Display, EnumString, FromRepr};

/// An IsController creates events that drive other things in the system.
/// Examples are sequencers, arpeggiators, and discrete LFOs. They get called on
/// each time slice so that they can do work and send any needed messages. An
/// IsController implements Terminates, which indicates that it's done emitting
/// events (and, in the case of timers and sequencers, done waiting for other
/// work in the system to complete).
pub trait IsController: Updateable + Terminates + HasUid + Send + Debug {}

/// An IsEffect transforms audio. It takes audio inputs and produces audio
/// output. It does not get called unless there is audio input to provide to it
/// (which can include silence, e.g., in the case of a muted instrument).
///
/// IsEffects don't terminate in the Terminates sense. This might become an
/// issue in the future, for example if a composition's end is determined by the
/// amount of time it takes for an effect to finish processing inputs (e.g., a
/// delay effect), and it turns out to be inconvenient for an IsController to
/// track the end. In this case, we might add a Terminates bound for IsEffect.
/// But right now I'm not sure that's the right solution.
pub trait IsEffect: TransformsAudio + Controllable + HasUid + Send + Debug {}

/// An IsInstrument produces audio, usually upon request from MIDI or
/// InController input. Like IsEffect, IsInstrument doesn't implement Terminates
/// because it continues to create audio as long as asked.
pub trait IsInstrument: SourcesAudio + HandlesMidi + Controllable + HasUid + Send + Debug {}

/// A future fourth trait might be named something like IsWidget or
/// IsGuiElement. These exist only to interact with the user of a GUI app, but
/// don't actually create or control audio.

/// An Updateable accepts new information through update() (i.e., Messages) or
/// control parameters.
///
/// Methods and messages are isomorphic, and everything could have been done
/// through update(), but sometimes a direct method is the right solution.
pub trait Updateable {
    #[allow(unused_variables)]
    fn update(&mut self, clock: &Clock, message: EntityMessage) -> Response<EntityMessage> {
        Response::none()
    }
}

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

/// A HasUid has an ephemeral but globally unique numeric identifier, which is
/// useful for one entity to refer to another without getting into icky Rust
/// ownership questions. It's the foundation of any ECS
/// (entity/component/system) design.
pub trait HasUid {
    fn uid(&self) -> usize;
    fn set_uid(&mut self, uid: usize);
}

pub trait Ticks: Send + Debug {
    /// The entity should reset its internal state.
    ///
    /// The system will call reset() when the global sample rate changes, and
    /// whenever the global clock is reset. Since most entities that care about
    /// sample rate need to know it during construction, the system *won't* call
    /// reset on entity construction; instead, entities can require the sample
    /// rate as part of their new() functions, and if desired call reset()
    /// within that function.
    fn reset(&mut self, sample_rate: usize);

    /// The entity should perform work for the current frame (or frames if
    /// frame_count > 1). Under normal circumstances, successive tick()s
    /// represent successive frames. Exceptions include, for example, restarting
    /// a performance, which would reset the global clock, which the entity
    /// learns about via reset().
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

/// A SourcesAudio provides audio in the form of digital samples.
pub trait SourcesAudio: Debug + Send {
    fn source_audio(&mut self, clock: &Clock) -> StereoSample;
}

/// A TransformsAudio takes input audio, which is typically produced by
/// SourcesAudio, does something to it, and then outputs it. It's what effects
/// do.
pub trait TransformsAudio: Debug {
    fn transform_audio(&mut self, clock: &Clock, input_sample: StereoSample) -> StereoSample {
        // Beware: converting from mono to stereo isn't just doing the work
        // twice! You'll also have to double whatever state you maintain from
        // tick to tick that has to do with a single channel's audio data.
        StereoSample(
            self.transform_channel(clock, 0, input_sample.0),
            self.transform_channel(clock, 1, input_sample.1),
        )
    }

    /// channel: 0 is left, 1 is right. Use the value as an index into arrays.
    fn transform_channel(&mut self, clock: &Clock, channel: usize, input_sample: Sample) -> Sample;
}

// A Terminates has a point in time where it would be OK never being called or
// continuing to exist.
//
// If you're required to implement Terminates, but you don't know when you need
// to terminate, then you should always return true. For example, an arpeggiator
// would be happy to keep responding to MIDI input forever. Which is (a little
// strangely) the same as saying it would be happy to quit at any time. Thus it
// should always return true.
//
// The reason to choose true rather than false is that the system uses
// is_finished() to determine whether a song is complete. If a Terminates never
// returns true, the loop will never end. Thus, "is_finished" is more like "is
// unaware of any reason to continue existing" rather than "is certain there is
// no more work to do."
pub trait Terminates: Debug {
    fn is_finished(&self) -> bool;
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

pub trait TestsValues {
    fn check_values(&mut self, clock: &Clock) {
        // If we've been asked to assert values at checkpoints, do so.
        if self.has_checkpoint_values()
            && clock.time_for(self.time_unit()) >= self.checkpoint_time()
        {
            const SAD_FLOAT_DIFF: f32 = 1.0e-4;
            if let Some(value) = self.pop_checkpoint_value() {
                assert_approx_eq!(self.value_to_check(), value, SAD_FLOAT_DIFF);
            }
            self.advance_checkpoint_time();
        }
    }

    fn has_checkpoint_values(&self) -> bool;
    fn time_unit(&self) -> &ClockTimeUnit;
    fn checkpoint_time(&self) -> f32;
    fn advance_checkpoint_time(&mut self);
    fn value_to_check(&self) -> f32;
    fn pop_checkpoint_value(&mut self) -> Option<f32>;
}

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

#[derive(Debug, Default)]
pub struct TestController {
    uid: usize,
    midi_channel_out: MidiChannel,
    pub tempo: f32,
    is_enabled: bool,
    is_playing: bool,

    pub checkpoint_values: VecDeque<f32>,
    pub checkpoint: f32,
    pub checkpoint_delta: f32,
    pub time_unit: ClockTimeUnit,
}
impl IsController for TestController {}
impl Updateable for TestController {
    fn update(&mut self, clock: &Clock, message: EntityMessage) -> Response<EntityMessage> {
        match message {
            EntityMessage::Tick => {
                self.check_values(clock);
                return match self.what_to_do(clock) {
                    TestControllerAction::Nothing => Response::none(),
                    TestControllerAction::NoteOn => {
                        // This is elegant, I hope. If the arpeggiator is
                        // disabled during play, and we were playing a note,
                        // then we still send the off note,
                        if self.is_enabled {
                            self.is_playing = true;
                            Response::single(EntityMessage::Midi(
                                self.midi_channel_out,
                                MidiMessage::NoteOn {
                                    key: 60.into(),
                                    vel: 127.into(),
                                },
                            ))
                        } else {
                            Response::none()
                        }
                    }
                    TestControllerAction::NoteOff => {
                        if self.is_playing {
                            Response::single(EntityMessage::Midi(
                                self.midi_channel_out,
                                MidiMessage::NoteOff {
                                    key: 60.into(),
                                    vel: 0.into(),
                                },
                            ))
                        } else {
                            Response::none()
                        }
                    }
                };
            }
            EntityMessage::Enable(enabled) => {
                self.is_enabled = enabled;
            }
            #[allow(unused_variables)]
            EntityMessage::Midi(channel, message) => {
                //dbg!(&channel, &message);
            }
            _ => todo!(),
        }
        Response::none()
    }
}
impl Terminates for TestController {
    fn is_finished(&self) -> bool {
        true
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
    pub fn new_with(midi_channel_out: MidiChannel) -> Self {
        Self {
            midi_channel_out,
            ..Default::default()
        }
    }

    pub fn new_with_test_values(
        midi_channel_out: MidiChannel,
        values: &[f32],
        checkpoint: f32,
        checkpoint_delta: f32,
        time_unit: ClockTimeUnit,
    ) -> Self {
        Self {
            midi_channel_out,
            checkpoint_values: VecDeque::from(Vec::from(values)),
            checkpoint,
            checkpoint_delta,
            time_unit,
            ..Default::default()
        }
    }

    fn what_to_do(&self, clock: &Clock) -> TestControllerAction {
        let beat_slice_start = clock.beats();
        let beat_slice_end = clock.next_slice_in_beats();
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
impl TestsValues for TestController {
    fn has_checkpoint_values(&self) -> bool {
        !self.checkpoint_values.is_empty()
    }

    fn time_unit(&self) -> &ClockTimeUnit {
        &self.time_unit
    }

    fn checkpoint_time(&self) -> f32 {
        self.checkpoint
    }

    fn advance_checkpoint_time(&mut self) {
        self.checkpoint += self.checkpoint_delta;
    }

    fn value_to_check(&self) -> f32 {
        self.tempo
    }

    fn pop_checkpoint_value(&mut self) -> Option<f32> {
        self.checkpoint_values.pop_front()
    }
}

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
    fn transform_channel(
        &mut self,
        clock: &Clock,
        _channel: usize,
        input_sample: Sample,
    ) -> Sample {
        self.check_values(clock);
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
impl TestsValues for TestEffect {
    fn has_checkpoint_values(&self) -> bool {
        !self.checkpoint_values.is_empty()
    }

    fn time_unit(&self) -> &ClockTimeUnit {
        &self.time_unit
    }

    fn checkpoint_time(&self) -> f32 {
        self.checkpoint
    }

    fn advance_checkpoint_time(&mut self) {
        self.checkpoint += self.checkpoint_delta;
    }

    fn value_to_check(&self) -> f32 {
        self.my_value()
    }

    fn pop_checkpoint_value(&mut self) -> Option<f32> {
        self.checkpoint_values.pop_front()
    }
}
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
impl HasUid for TestInstrument {
    fn uid(&self) -> usize {
        self.uid
    }

    fn set_uid(&mut self, uid: usize) {
        self.uid = uid;
    }
}
impl HandlesMidi for TestInstrument {
    fn handle_midi_message(&mut self, message: &MidiMessage) {
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
    }
}
impl TestsValues for TestInstrument {
    fn has_checkpoint_values(&self) -> bool {
        !self.checkpoint_values.is_empty()
    }

    fn time_unit(&self) -> &ClockTimeUnit {
        &self.time_unit
    }

    fn checkpoint_time(&self) -> f32 {
        self.checkpoint
    }

    fn advance_checkpoint_time(&mut self) {
        self.checkpoint += self.checkpoint_delta;
    }

    fn value_to_check(&self) -> f32 {
        self.fake_value
    }

    fn pop_checkpoint_value(&mut self) -> Option<f32> {
        self.checkpoint_values.pop_front()
    }
}
impl TestInstrument {
    pub fn new_with(sample_rate: usize) -> Self {
        let mut r = Self {
            uid: Default::default(),
            waveform: Default::default(),
            sample_rate,
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

impl SourcesAudio for TestInstrument {
    fn source_audio(&mut self, clock: &Clock) -> StereoSample {
        if clock.was_reset() {
            self.oscillator.reset(self.sample_rate);
        }
        self.oscillator.tick(1);
        // If we've been asked to assert values at checkpoints, do so.
        if !self.checkpoint_values.is_empty() && clock.time_for(&self.time_unit) >= self.checkpoint
        {
            const SAD_FLOAT_DIFF: f32 = 1.0e-2;
            assert_approx_eq!(self.fake_value, self.checkpoint_values[0], SAD_FLOAT_DIFF);
            self.checkpoint += self.checkpoint_delta;
            self.checkpoint_values.pop_front();
        }
        if self.is_playing {
            self.dca
                .transform_audio_to_stereo(clock, Sample::from(self.oscillator.signal_value()))
        } else {
            StereoSample::SILENCE
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::SourcesAudio;
    use super::TestInstrument;
    use crate::clock::Clock;
    use rand::random;

    // TODO: restore tests that test basic trait behavior, then figure out how
    // to run everyone implementing those traits through that behavior. For now,
    // this one just tests that a generic instrument doesn't panic when accessed
    // for non-consecutive time slices.
    #[test]
    fn test_sources_audio_random_access() {
        let mut instrument = TestInstrument::new_with(Clock::DEFAULT_SAMPLE_RATE);
        for _ in 0..100 {
            let mut clock = Clock::default();
            clock.set_samples(random());
            let _ = instrument.source_audio(&clock);
        }
    }

    impl TestInstrument {
        pub fn dump_messages(&self) {
            dbg!(&self.debug_messages);
        }
    }
}
