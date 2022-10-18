use super::{
    persistence::{Filter, LoadError, SaveError, SavedState},
    to_be_obsolete::TaskMessage,
    AudioSourceMessage, ControlBarMessage,
};

#[derive(Debug, Clone)]
pub enum Message {
    Loaded(Result<SavedState, LoadError>),
    Saved(Result<(), SaveError>),
    InputChanged(String),
    CreateTask,
    FilterChanged(Filter),
    TaskMessage(usize, TaskMessage),
    AudioSourceMessage(usize, AudioSourceMessage),
    ControlBarMessage(ControlBarMessage),
}
