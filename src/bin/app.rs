mod gui;

use groove::gui::IsViewable;
use groove::gui::ViewableMessage;
use groove::AudioOutput;
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
use iced::time;
use iced::widget::button;
use iced::widget::scrollable;
use iced::widget::text_input;
use iced::widget::{column, container, row, text};
use iced::Color;
use iced::Subscription;
use iced::{Alignment, Application, Command, Element, Length, Settings};
use std::time::{Duration, Instant};

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
    viewables: Vec<Box<dyn IsViewable<Message = ViewableMessage>>>,
    control_bar: ControlBar,
    audio_output: AudioOutput,
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
    ControlBarMessage(ControlBarMessage),
    ControlBarBpm(String),
    ViewableMessage(usize, ViewableMessage),

    Tick(Instant),
}

#[derive(Debug, Default, Clone)]
pub struct ControlBar {
    clock: Clock,
}

#[derive(Debug, Clone)]
pub enum ControlBarMessage {
    Play,
    Stop,
    SkipToStart,
}

impl ControlBar {
    pub fn view(&self, orchestrator: &Orchestrator) -> Element<Message> {
        container(row![
            text_input(
                "BPM",
                orchestrator.bpm().round().to_string().as_str(),
                Message::ControlBarBpm
            )
            .width(Length::Units(40)),
            button(skip_to_prev_icon())
                .width(Length::Units(32))
                .on_press(Message::ControlBarMessage(ControlBarMessage::SkipToStart)),
            button(play_icon())
                .width(Length::Units(32))
                .on_press(Message::ControlBarMessage(ControlBarMessage::Play)),
            button(stop_icon())
                .width(Length::Units(32))
                .on_press(Message::ControlBarMessage(ControlBarMessage::Stop)),
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
                (*self).audio_output.start();
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
                    while self.audio_output.worker().len() < 2048 {
                        let (sample, done) = self.orchestrator.tick();
                        self.audio_output.worker_mut().push(sample);
                        if done {
                            // TODO - this needs to be stickier
                            break;
                        }
                    }
                }
            }
            Message::Reset => {
                //              self.duration = Duration::default();
            }
            Message::ControlBarMessage(message) => match message {
                ControlBarMessage::Play => {
                    self.state = State::Ticking {
                        last_tick: Instant::now(),
                    }
                }
                ControlBarMessage::Stop => self.state = State::Idle,
                ControlBarMessage::SkipToStart => todo!(),
            },
            Message::ControlBarBpm(new_value) => {
                if let Ok(bpm) = new_value.parse() {
                    self.orchestrator.set_bpm(bpm);
                }
            }
            Message::ViewableMessage(i, message) => self.viewables[i].update(message),
        }

        Command::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        match self.state {
            State::Idle => Subscription::none(),
            State::Ticking { .. } => time::every(Duration::from_millis(10)).map(Message::Tick),
        }
    }

    fn view(&self) -> Element<Message> {
        match self.state {
            State::Idle => {}
            #[allow(unused)]
            State::Ticking { last_tick } => {}
        }

        let control_bar = self.control_bar.view(&self.orchestrator);

        let views: Element<_> = if self.viewables.is_empty() {
            empty_message("nothing yet")
        } else {
            column(
                self.viewables
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        item.view()
                            .map(move |message| Message::ViewableMessage(i, message))
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
            .align_y(alignment::Vertical::Top)
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
