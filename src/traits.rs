use crate::{
    clock::Clock,
    common::MonoSample,
    gui::{IsViewable, ViewableMessage},
    messages::MessageBounds,
};

pub trait NewIsController: NewUpdateable + Terminates + HasUid + std::fmt::Debug {}
pub trait NewIsEffect: TransformsAudio + NewUpdateable + HasUid + std::fmt::Debug {}
pub trait NewIsInstrument: SourcesAudio + NewUpdateable + HasUid + std::fmt::Debug {}

#[derive(Debug)]
pub enum BoxedEntity<M> {
    Controller(Box<dyn NewIsController<Message = M>>),
    Effect(Box<dyn NewIsEffect<Message = M>>),
    Instrument(Box<dyn NewIsInstrument<Message = M>>),
}

pub trait NewUpdateable {
    type Message: MessageBounds;

    #[allow(unused_variables)]
    fn update(&mut self, clock: &Clock, message: Self::Message) -> EvenNewerCommand<Self::Message> {
        EvenNewerCommand::none()
    }
    #[allow(unused_variables)]
    fn handle_message(&mut self, clock: &Clock, message: Self::Message) {
        todo!()
    }
    #[allow(unused_variables)]
    fn param_id_for_name(&self, param_name: &str) -> usize {
        usize::MAX
    }
}
pub trait HasUid {
    fn uid(&self) -> usize;
    fn set_uid(&mut self, uid: usize);
}

/// Provides audio in the form of digital samples.
pub trait SourcesAudio: std::fmt::Debug {
    fn source_audio(&mut self, clock: &Clock) -> MonoSample;
}

/// TransformsAudio can be thought of as SourcesAudio + SinksAudio, but it's an
/// important third trait because it exposes the business logic that happens
/// between the sinking and sourcing, which is useful for testing.
pub trait TransformsAudio: std::fmt::Debug {
    fn transform_audio(&mut self, clock: &Clock, input_sample: MonoSample) -> MonoSample;
}

// Something that Terminates has a point in time where it would be OK never
// being called or continuing to exist.
//
// If you're required to implement Terminates, but you don't know when you need
// to terminate, then you should always return true. For example, an arpeggiator
// is a WatchesClock, which means it is also a Terminates, but it would be happy
// to keep responding to MIDI input forever. It should return true.
//
// The reason to choose true rather than false is that the caller uses
// is_finished() to determine whether a song is complete. If a Terminates never
// returns true, the loop will never end. Thus, "is_finished" is more like "is
// unaware of any reason to continue existing" rather than "is certain there is
// no more work to do."
pub trait Terminates: std::fmt::Debug {
    fn is_finished(&self) -> bool;
}

#[derive(Debug)]
pub struct EvenNewerCommand<T>(pub Internal<T>);

#[derive(Debug)]
pub enum Internal<T> {
    None,
    Single(T),
    Batch(Vec<T>),
}

impl<T> EvenNewerCommand<T> {
    pub const fn none() -> Self {
        Self(Internal::None)
    }

    pub const fn single(action: T) -> Self {
        Self(Internal::Single(action))
    }

    pub fn batch(commands: impl IntoIterator<Item = EvenNewerCommand<T>>) -> Self {
        let mut batch = Vec::new();

        for EvenNewerCommand(command) in commands {
            match command {
                Internal::None => {}
                Internal::Single(command) => batch.push(command),
                Internal::Batch(commands) => batch.extend(commands),
            }
        }

        Self(Internal::Batch(batch))
    }
}

pub trait MakesIsViewable: std::fmt::Debug {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable<Message = ViewableMessage>>>;
}

#[cfg(test)]
pub mod tests {

    use rand::random;

    use crate::{messages::tests::TestMessage, utils::tests::TestInstrument, Clock};

    use super::SourcesAudio;

    // TODO: restore tests that test basic trait behavior, then figure out how
    // to run everyone implementing those traits through that behavior. For now,
    // this one just tests that a generic instrument doesn't panic when accessed
    // for non-consecutive time slices.
    #[test]
    fn test_sources_audio_random_access() {
        let mut instrument = TestInstrument::<TestMessage>::default();
        for _ in 0..100 {
            let mut clock = Clock::new();
            clock.debug_set_samples(random());
            let _ = instrument.source_audio(&clock);
        }
    }
}
