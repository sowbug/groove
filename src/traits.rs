use std::fmt::Debug;

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
