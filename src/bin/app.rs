use groove::gui::message::Message;
use groove::gui::persistence::{Filter, SavedState};
use groove::gui::to_be_obsolete::{empty_message, loading_message, Task, TaskMessage};
use groove::gui::{AudioSource, ControlBar};
use groove::{IOHelper, Orchestrator, SongSettings};
use iced::alignment::{self};
use iced::scrollable::{self, Scrollable};
use iced::text_input::{self, TextInput};
use iced::{Application, Column, Command, Container, Element, Length, Settings, Text};

pub fn main() -> iced::Result {
    Groove::run(Settings::default())
}

#[derive(Debug)]
enum Groove {
    Loading,
    Loaded(State),
}

#[derive(Debug, Default)]
struct State {
    scroll: scrollable::State,
    input: text_input::State,
    input_value: String,
    filter: Filter,
    tasks: Vec<Task>,
    control_bar: ControlBar,
    dirty: bool,
    saving: bool,
    ////
    song_settings: SongSettings,
    orchestrator: Orchestrator,
    audio_sources: Vec<AudioSource>,
}

impl Application for Groove {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Groove, Command<Message>) {
        (
            Groove::Loading,
            Command::perform(SavedState::load(), Message::Loaded),
        )
    }

    fn title(&self) -> String {
        let dirty = match self {
            Groove::Loading => false,
            Groove::Loaded(state) => state.dirty,
        };

        format!("Todos{} - Iced", if dirty { "*" } else { "" })
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match self {
            Groove::Loading => {
                match message {
                    Message::Loaded(Ok(state)) => {
                        if let Ok(orchestrator) = state.song_settings.instantiate() {
                            let mut audio_sources = Vec::<AudioSource>::new();
                            for (_, source) in
                                orchestrator.main_mixer().sources().iter().enumerate()
                            {
                                if let Some(source) = source.upgrade() {
                                    audio_sources.push(AudioSource::instantiate(source));
                                }
                            }

                            *self = Groove::Loaded(State {
                                // input_value: state.input_value,
                                // filter: state.filter,
                                // tasks: state.tasks,
                                song_settings: state.song_settings.clone(),
                                orchestrator,
                                audio_sources,
                                ..State::default()
                            });
                        } else {
                            panic!("song_settings.instantiate() failed")
                        }
                    }
                    Message::Loaded(Err(_)) => {
                        *self = Groove::Loaded(State::default());
                    }
                    _ => {}
                }

                Command::none()
            }
            Groove::Loaded(state) => {
                let mut saved = false;

                match message {
                    Message::InputChanged(value) => {
                        state.input_value = value;
                    }
                    Message::CreateTask => {
                        if !state.input_value.is_empty() {
                            state.tasks.push(Task::new(state.input_value.clone()));
                            state.input_value.clear();
                        }
                    }
                    Message::FilterChanged(filter) => {
                        state.filter = filter;
                    }
                    Message::TaskMessage(i, TaskMessage::Delete) => {
                        state.tasks.remove(i);
                    }
                    Message::TaskMessage(i, task_message) => {
                        if let Some(task) = state.tasks.get_mut(i) {
                            task.update(task_message);
                        }
                    }
                    Message::Saved(_) => {
                        state.saving = false;
                        saved = true;
                    }
                    Message::Loaded(_) => todo!(),
                    Message::AudioSourceMessage(i, audio_source_message) => {
                        if let Some(audio_source) = state.audio_sources.get_mut(i) {
                            audio_source.update(audio_source_message.clone());
                        }
                        match audio_source_message {
                            groove::gui::AudioSourceMessage::EditButtonPressed => todo!(),
                            groove::gui::AudioSourceMessage::IsMuted(is_muted) => {
                                state.orchestrator.mute_audio_source(i, is_muted);
                            }
                        }
                    }
                    Message::ControlBarMessage(control_bar_message) => match control_bar_message {
                        groove::gui::ControlBarMessage::Play => {
                            if let Ok(performance) = state.orchestrator.perform() {
                                IOHelper::send_performance_to_output_device(performance);
                            };
                        }
                        groove::gui::ControlBarMessage::Stop => todo!(),
                    },
                }

                if !saved {
                    state.dirty = true;
                }

                if state.dirty && !state.saving {
                    state.dirty = false;
                    state.saving = true;

                    Command::perform(
                        SavedState {
                            // input_value: state.input_value.clone(),
                            // filter: state.filter,
                            // tasks: state.tasks.clone(),
                            song_settings: state.song_settings.clone(),
                        }
                        .save(),
                        Message::Saved,
                    )
                } else {
                    Command::none()
                }
            }
        }
    }

    fn view(&mut self) -> Element<Message> {
        match self {
            Groove::Loading => loading_message(),
            Groove::Loaded(State {
                scroll,
                input,
                input_value,
                filter,
                tasks,
                control_bar,
                song_settings,
                orchestrator,
                audio_sources,
                ..
            }) => {
                let title = Text::new(format!("BPM {}", song_settings.clock.bpm()))
                    .width(Length::Fill)
                    .size(100)
                    .color([0.5, 0.5, 0.5])
                    .horizontal_alignment(alignment::Horizontal::Center);

                let input = TextInput::new(
                    input,
                    format!("other {}", orchestrator.bpm()).as_str(),
                    input_value,
                    Message::InputChanged,
                )
                .padding(15)
                .size(30)
                .on_submit(Message::CreateTask);

                let filtered_tasks = tasks.iter().filter(|task| filter.matches(task));

                let control_bar = control_bar.view();

                let sources1: Element<_> = if audio_sources.is_empty() {
                    empty_message("nothing yet")
                } else {
                    audio_sources
                        .iter_mut()
                        .enumerate()
                        .fold(Column::new().spacing(20), |column, (i, source)| {
                            column.push(
                                source
                                    .view()
                                    .map(move |message| Message::AudioSourceMessage(i, message)),
                            )
                        })
                        .into()
                };

                let sources2: Element<_> = empty_message("You have not created a task yet...");

                let tasks: Element<_> = if filtered_tasks.count() > 0 {
                    tasks
                        .iter_mut()
                        .enumerate()
                        .filter(|(_, task)| filter.matches(task))
                        .fold(Column::new().spacing(20), |column, (i, task)| {
                            column.push(
                                task.view()
                                    .map(move |message| Message::TaskMessage(i, message)),
                            )
                        })
                        .into()
                } else {
                    empty_message(match filter {
                        Filter::All => "You have not created a task yet...",
                        Filter::Active => "All your tasks are done! :D",
                        Filter::Completed => "You have not completed a task yet...",
                    })
                };

                let content = Column::new()
                    .max_width(800)
                    .spacing(20)
                    .push(title)
                    .push(input)
                    .push(control_bar)
                    .push(sources1)
                    .push(sources2)
                    .push(tasks);

                Scrollable::new(scroll)
                    .padding(40)
                    .push(Container::new(content).width(Length::Fill).center_x())
                    .into()
            }
        }
    }
}
