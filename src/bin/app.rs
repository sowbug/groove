mod gui;

use groove::gui::GrooveMessage;
use groove::gui::IsViewable;
use groove::Orchestrator;
use gui::persistence::LoadError;
use gui::persistence::SavedState;
use gui::play_icon;
use gui::skip_to_prev_icon;
use gui::stop_icon;
use iced::alignment;
use iced::executor;
use iced::theme;
use iced::theme::Theme;
use iced::widget::button;
use iced::widget::scrollable;
use iced::widget::text_input;
use iced::widget::{column, container, row, text};
use iced::Color;
use iced::{Alignment, Application, Command, Element, Length, Settings};
use std::time::Instant;

pub fn main() -> iced::Result {
    GrooveApp::run(Settings::default())
}

#[derive(Default)]
struct GrooveApp {
    theme: Theme,
    state: State,

    project_name: String,
    #[allow(dead_code)]
    orchestrator: Orchestrator,
    viewables: Vec<Box<dyn IsViewable>>,
    control_bar: ControlBar,
}

#[derive(Default)]
enum State {
    #[default]
    Idle,
    Ticking {
        last_tick: Instant,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    Loaded(Result<SavedState, LoadError>),
    Toggle,
    Reset,
    ControlBarBpmChange(String),
    ControlBarPlay,
    ControlBarStop,
    GrooveMessage(usize, GrooveMessage),

    Tick(Instant),
}

#[derive(Debug, Default, Clone)]
pub struct ControlBar {
    bpm: f32,

    clock: Clock,
}

pub enum ControlBarMessage {
    Bpm(f32),
    Clock(String),
}

impl ControlBar {
    pub fn update(&mut self, message: ControlBarMessage) {
        match message {
            ControlBarMessage::Bpm(new_value) => {
                self.bpm = new_value;
            }
            ControlBarMessage::Clock(new_time) => {
                self.clock.current_time = new_time;
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        container(row![
            text_input(
                "BPM",
                self.bpm.to_string().as_str(),
                Message::ControlBarBpmChange
            )
            .width(Length::Units(40)),
            button(skip_to_prev_icon())
                .width(Length::Units(32))
                .on_press(Message::ControlBarPlay),
            button(play_icon())
                .width(Length::Units(32))
                .on_press(Message::ControlBarPlay),
            button(stop_icon())
                .width(Length::Units(32))
                .on_press(Message::ControlBarStop),
            self.clock.view()
        ])
        .width(Length::Fill)
        .padding(4)
        .style(theme::Container::Box)
        .into()
    }
}

#[derive(Debug, Clone)]
struct Clock {
    current_time: String,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            current_time: "00:00:00".to_string(),
        }
    }
}

impl Clock {
    pub fn view(&self) -> Element<Message> {
        container(text(self.current_time.clone())).into()
    }
}

impl Application for GrooveApp {
    type Message = Message;
    type Theme = Theme;
    type Executor = executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (GrooveApp, Command<Message>) {
        (
            GrooveApp {
                theme: Theme::Dark,
                state: State::Idle,
                ..Default::default()
            },
            Command::perform(SavedState::load(), Message::Loaded),
        )
    }

    fn theme(&self) -> Theme {
        self.theme
    }

    fn title(&self) -> String {
        String::from(&self.project_name)
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Loaded(Ok(state)) => {
                let orchestrator = state.song_settings.instantiate().unwrap();
                let viewables = orchestrator
                    .viewables()
                    .iter()
                    .map(|item| {
                        if let Some(item) = item.upgrade() {
                            if let Some(responder) = item.borrow_mut().make_is_viewable() {
                                responder
                            } else {
                                panic!("make responder failed. Probably forgot new_wrapped()")
                            }
                        } else {
                            panic!("upgrade failed")
                        }
                    })
                    .collect();
                *self = Self {
                    theme: self.theme,
                    project_name: state.project_name.clone(),
                    orchestrator,
                    viewables,
                    ..Default::default()
                };
            }
            Message::Loaded(Err(_)) => {
                todo!()
            }
            Message::Toggle => match self.state {
                State::Idle => {
                    self.state = State::Ticking {
                        last_tick: Instant::now(),
                    };
                }
                State::Ticking { .. } => {
                    self.state = State::Idle;
                }
            },
            Message::Tick(now) => {
                if let State::Ticking { last_tick } = &mut self.state {
                    //                    self.duration += now - *last_tick;
                    *last_tick = now;
                }
            }
            Message::Reset => {
                //              self.duration = Duration::default();
            }
            Message::GrooveMessage(_, _) => todo!(),
            Message::ControlBarBpmChange(_) => todo!(),
            Message::ControlBarPlay => todo!(),
            Message::ControlBarStop => todo!(),
        }

        Command::none()
    }

    // fn subscription(&self) -> Subscription<Message> {
    //     match self.state {
    //         State::Idle => Subscription::none(),
    //         State::Ticking { .. } => time::every(Duration::from_millis(10)).map(Message::Tick),
    //     }
    // }

    fn view(&self) -> Element<Message> {
        match self.state {
            State::Idle => {}
            #[allow(unused)]
            State::Ticking { last_tick } => {}
        }

        let control_bar = self.control_bar.view();

        let views: Element<_> = if self.viewables.is_empty() {
            empty_message("nothing yet")
        } else {
            column(
                self.viewables
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        item.view()
                            .map(move |message| Message::GrooveMessage(i, message))
                    })
                    .collect(),
            )
            .spacing(10)
            .into()
        };
        let scrollable_content = column![views];
        let under_construction = text("Under Construction").width(Length::FillPortion(1));
        let scrollable = container(scrollable(scrollable_content)).width(Length::FillPortion(1));
        let main_workspace = row![under_construction, scrollable];
        let content = column![control_bar, main_workspace]
            .align_items(Alignment::Center)
            .spacing(20);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}

fn empty_message(message: &str) -> Element<'_, Message> {
    container(
        text(message)
            .width(Length::Fill)
            .size(25)
            .horizontal_alignment(alignment::Horizontal::Center)
            .style(Color::from([0.7, 0.7, 0.7])),
    )
    .width(Length::Fill)
    .height(Length::Units(200))
    .center_y()
    .into()
}
