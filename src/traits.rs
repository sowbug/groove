use crate::{controllers::F32ControlValue, messages::EntityMessage, midi::MidiChannel};
use groove_core::{Sample, StereoSample};
use midly::MidiMessage;
use std::fmt::Debug;

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

#[cfg(test)]
pub mod tests {
    use super::{Generates, Ticks};
    use crate::{common::DEFAULT_SAMPLE_RATE, instruments::TestInstrument};
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
