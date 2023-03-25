// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::{
    control::F32ControlValue,
    midi::{u7, HandlesMidi},
    BipolarNormal, Normal, Sample, StereoSample,
};

pub trait MessageBounds: Clone + std::fmt::Debug + Send {}

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
pub trait IsController<V>:
    TicksWithMessages<V> + HandlesMidi + HasUid + Send + std::fmt::Debug
{
}

/// An IsEffect transforms audio. It takes audio inputs and produces audio
/// output. It does not get called unless there is audio input to provide to it
/// (which can include silence, e.g., in the case of a muted instrument).
///
/// IsEffects don't implement Terminates. They process audio indefinitely, and
/// don't have a sense of the length of the performance.
pub trait IsEffect: TransformsAudio + Controllable + HasUid + Send + std::fmt::Debug {}

/// An IsInstrument produces audio, usually upon request from MIDI or
/// IsController input. Like IsEffect, IsInstrument doesn't implement Terminates
/// because it continues to create audio as long as asked.
pub trait IsInstrument:
    Generates<StereoSample> + Ticks + HandlesMidi + Controllable + HasUid + Send + std::fmt::Debug
{
}

pub trait Generates<V>: Send + std::fmt::Debug + Ticks {
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
#[allow(unused_variables)]
pub trait Controllable {
    // See https://stackoverflow.com/a/71988904/344467 to show that we could
    // have made these functions rather than methods (no self). But then we'd
    // lose the ability to query an object without knowing its struct, which is
    // important for the loose binding that the automation system provides.
    fn control_index_count(&self) -> usize {
        unimplemented!()
    }
    fn control_index_for_name(&self, name: &str) -> usize {
        unimplemented!("Controllable trait methods are implemented by a macro")
    }
    fn control_name_for_index(&self, index: usize) -> Option<&'static str> {
        unimplemented!()
    }
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
    fn name(&self) -> &'static str;
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

pub trait Ticks: Resets + Send + std::fmt::Debug {
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

pub trait TicksWithMessages<V>: Resets + Send + std::fmt::Debug {
    type Message;

    /// Similar to Ticks::tick().
    ///
    /// Returns zero or more EntityMessages.
    ///
    /// Returns the number of requested ticks handled before terminating.
    fn tick(&mut self, tick_count: usize) -> (Option<Vec<V>>, usize);
}

/// A TransformsAudio takes input audio, which is typically produced by
/// SourcesAudio, does something to it, and then outputs it. It's what effects
/// do.
pub trait TransformsAudio: std::fmt::Debug {
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

/// Describes the public interface of an envelope generator, which provides a
/// normalized amplitude (0.0..=1.0) that changes over time according to its
/// internal parameters, external triggers, and the progression of time.
pub trait GeneratesEnvelope: Generates<Normal> + Send + std::fmt::Debug + Ticks {
    /// Triggers the envelope's active stage.
    fn trigger_attack(&mut self);

    /// Triggers the end of the envelope's active stage.
    fn trigger_release(&mut self);

    /// Requests a fast decrease to zero amplitude. Upon reaching zero, switches
    /// to idle. If the EG is already idle, then does nothing. For normal EGs,
    /// the EG's settings (ADSR, etc.) don't affect the rate of shutdown decay.
    ///
    /// See DSSPC, 4.5 Voice Stealing, for an understanding of how the shutdown
    /// state helps. TL;DR: if we have to steal one voice to play a different
    /// note, it sounds better if the voice very briefly stops and restarts.
    fn trigger_shutdown(&mut self);

    /// Whether the envelope generator is in the idle state, which usually means
    /// quiescent and zero amplitude.
    fn is_idle(&self) -> bool;
}

/// A [PlaysNotes] turns note events into sound. It seems to overlap with
/// [HandlesMidi]; the reason it exists is to allow the two interfaces to evolve
/// independently, because MIDI is unlikely to be perfect for all our needs.
pub trait PlaysNotes {
    /// Whether the entity is currently making sound.
    fn is_playing(&self) -> bool;

    /// Initiates a note-on event. Depending on implementation, might initiate a
    /// steal (tell envelope to go to shutdown state, then do note-on when
    /// that's done).
    fn note_on(&mut self, key: u8, velocity: u8);

    /// Initiates an aftertouch event.
    fn aftertouch(&mut self, velocity: u8);

    /// Initiates a note-off event, which can take a long time to complete,
    /// depending on how long the envelope's release is.
    fn note_off(&mut self, velocity: u8);

    /// Sets this entity's left-right balance.
    fn set_pan(&mut self, value: BipolarNormal);
}

// TODO: I didn't want StoresVoices to know anything about audio (i.e.,
// SourcesAudio), but I couldn't figure out how to return an IterMut from a
// HashMap, so I couldn't define a trait method that allowed the implementation
// to return an iterator from either a Vec or a HashMap.
//
// Maybe what I really want is for Synthesizers to have the StoresVoices trait.
pub trait StoresVoices: Generates<StereoSample> + Send + std::fmt::Debug {
    type Voice;

    /// Generally, this value won't change after initialization, because we try
    /// not to dynamically allocate new voices.
    fn voice_count(&self) -> usize;

    /// The number of voices reporting is_playing() true.
    fn active_voice_count(&self) -> usize;

    /// Fails if we run out of idle voices and can't steal any active ones.
    fn get_voice(&mut self, key: &u7) -> anyhow::Result<&mut Box<Self::Voice>>;

    /// All the voices.
    // Thanks to https://stackoverflow.com/a/58612273/344467 for the lifetime magic
    fn voices<'a>(&'a self) -> Box<dyn Iterator<Item = &Box<Self::Voice>> + 'a>;

    /// All the voices as a mutable iterator.
    fn voices_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &mut Box<Self::Voice>> + 'a>;
}

/// A synthesizer is composed of Voices. Ideally, a synth will know how to
/// construct Voices, and then handle all the MIDI events properly for them.
pub trait IsVoice<V>: Generates<V> + PlaysNotes + Send {}
pub trait IsStereoSampleVoice: IsVoice<StereoSample> {}

#[cfg(test)]
pub(crate) mod tests {
    use super::Ticks;

    pub trait DebugTicks: Ticks {
        fn debug_tick_until(&mut self, tick_number: usize);
    }
}
