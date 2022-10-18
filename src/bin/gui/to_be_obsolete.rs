use super::message::Message;
use super::persistence::Filter;
use super::{delete_icon, edit_icon, style};
use iced::alignment::{self, Alignment};
use iced::button::{self, Button};
use iced::text_input::{self, TextInput};
use iced::{Checkbox, Container, Element, Length, Row, Text};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub description: String,
    pub completed: bool,

    #[serde(skip)]
    state: TaskState,
}

#[derive(Debug, Clone)]
pub enum TaskState {
    Idle {
        edit_button: button::State,
    },
    Editing {
        text_input: text_input::State,
        delete_button: button::State,
    },
}

impl Default for TaskState {
    fn default() -> Self {
        TaskState::Idle {
            edit_button: button::State::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TaskMessage {
    Completed(bool),
    Edit,
    DescriptionEdited(String),
    FinishEdition,
    Delete,
}

impl Task {
    pub fn new(description: String) -> Self {
        Task {
            description,
            completed: false,
            state: TaskState::Idle {
                edit_button: button::State::new(),
            },
        }
    }

    pub fn update(&mut self, message: TaskMessage) {
        match message {
            TaskMessage::Completed(completed) => {
                self.completed = completed;
            }
            TaskMessage::Edit => {
                let mut text_input = text_input::State::focused();
                text_input.select_all();

                self.state = TaskState::Editing {
                    text_input,
                    delete_button: button::State::new(),
                };
            }
            TaskMessage::DescriptionEdited(new_description) => {
                self.description = new_description;
            }
            TaskMessage::FinishEdition => {
                if !self.description.is_empty() {
                    self.state = TaskState::Idle {
                        edit_button: button::State::new(),
                    }
                }
            }
            TaskMessage::Delete => {}
        }
    }

    pub fn view(&mut self) -> Element<TaskMessage> {
        match &mut self.state {
            TaskState::Idle { edit_button } => {
                let checkbox =
                    Checkbox::new(self.completed, &self.description, TaskMessage::Completed)
                        .width(Length::Fill);

                Row::new()
                    .spacing(20)
                    .align_items(Alignment::Center)
                    .push(checkbox)
                    .push(
                        Button::new(edit_button, edit_icon())
                            .on_press(TaskMessage::Edit)
                            .padding(10)
                            .style(style::Button::Icon),
                    )
                    .into()
            }
            TaskState::Editing {
                text_input,
                delete_button,
            } => {
                let text_input = TextInput::new(
                    text_input,
                    "Describe your task...",
                    &self.description,
                    TaskMessage::DescriptionEdited,
                )
                .on_submit(TaskMessage::FinishEdition)
                .padding(10);

                Row::new()
                    .spacing(20)
                    .align_items(Alignment::Center)
                    .push(text_input)
                    .push(
                        Button::new(
                            delete_button,
                            Row::new()
                                .spacing(10)
                                .push(delete_icon())
                                .push(Text::new("Delete")),
                        )
                        .on_press(TaskMessage::Delete)
                        .padding(10)
                        .style(style::Button::Destructive),
                    )
                    .into()
            }
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Controls {
    all_button: button::State,
    active_button: button::State,
    completed_button: button::State,
}

impl Controls {
    pub fn view(&mut self, tasks: &[Task], current_filter: Filter) -> Row<Message> {
        let Controls {
            all_button,
            active_button,
            completed_button,
        } = self;

        let tasks_left = tasks.iter().filter(|task| !task.completed).count();

        let filter_button = |state, label, filter, current_filter| {
            let label = Text::new(label).size(16);
            let button = Button::new(state, label).style(if filter == current_filter {
                style::Button::FilterSelected
            } else {
                style::Button::FilterActive
            });

            button.on_press(Message::FilterChanged(filter)).padding(8)
        };

        Row::new()
            .spacing(20)
            .align_items(Alignment::Center)
            .push(
                Text::new(format!(
                    "{} {} left",
                    tasks_left,
                    if tasks_left == 1 { "task" } else { "tasks" }
                ))
                .width(Length::Fill)
                .size(16),
            )
            .push(
                Row::new()
                    .width(Length::Shrink)
                    .spacing(10)
                    .push(filter_button(
                        all_button,
                        "All",
                        Filter::All,
                        current_filter,
                    ))
                    .push(filter_button(
                        active_button,
                        "Active",
                        Filter::Active,
                        current_filter,
                    ))
                    .push(filter_button(
                        completed_button,
                        "Completed",
                        Filter::Completed,
                        current_filter,
                    )),
            )
    }
}

pub fn loading_message<'a>() -> Element<'a, Message> {
    Container::new(
        Text::new("Loading...")
            .horizontal_alignment(alignment::Horizontal::Center)
            .size(50),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_y()
    .into()
}

pub fn empty_message<'a>(message: &str) -> Element<'a, Message> {
    Container::new(
        Text::new(message)
            .width(Length::Fill)
            .size(25)
            .horizontal_alignment(alignment::Horizontal::Center)
            .color([0.7, 0.7, 0.7]),
    )
    .width(Length::Fill)
    .height(Length::Units(200))
    .center_y()
    .into()
}
