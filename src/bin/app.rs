mod gui;

use async_std::task::block_on;
use groove::{
    gui::{GuiStuff, IsViewable, ViewableMessage, NUMBERS_FONT, NUMBERS_FONT_SIZE},
    AudioOutput, IOHelper, Orchestrator,
};
use gui::{
    persistence::{LoadError, SavedState},
    play_icon, skip_to_prev_icon, stop_icon,
};
use iced::{
    alignment, executor,
    theme::{self, Theme},
    time,
    widget::{button, column, container, row, scrollable, text, text_input},
    Alignment, Application, Color, Command, Element, Length, Settings, Subscription,
};
use iced_native::{window, Event};
use std::time::{Duration, Instant};

pub fn main() -> iced::Result {
    GrooveApp::run(Settings {
        // This override is needed so that we get the CloseRequested event that
        // we need to turn off the cpal Stream. See
        // https://github.com/iced-rs/iced/blob/master/examples/events/ for an
        // example.
        exit_on_close_request: false,
        ..Settings::default()
    })
}

#[derive(Default)]
struct GrooveApp {
    // Overhead
    theme: Theme,
    state: State,
    should_exit: bool,

    // UI components
    control_bar: ControlBar,

    // Model
    project_name: String,
    orchestrator: Orchestrator,
    viewables: Vec<Box<dyn IsViewable<Message = ViewableMessage>>>,
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
    ControlBarMessage(ControlBarMessage),
    ControlBarBpm(String),
    ViewableMessage(usize, ViewableMessage),

    Tick(Instant),
    EventOccurred(iced::Event),
}

#[derive(Debug, Clone)]
pub enum ControlBarMessage {
    Play,
    Stop,
    SkipToStart,
}

#[derive(Debug, Default, Clone)]
pub struct ControlBar {
    clock: Clock,
}

impl ControlBar {
    pub fn view(&self, orchestrator: &Orchestrator) -> Element<Message> {
        container(
            row![
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
            ]
            .padding(8)
            .spacing(4)
            .align_items(Alignment::Center),
        )
        .width(Length::Fill)
        .padding(4)
        .style(theme::Container::Box)
        .into()
    }
}

#[derive(Debug, Clone)]
pub enum ClockMessage {
    Time(f32),
}

#[derive(Debug, Clone)]
struct Clock {
    seconds: f32,
}

impl Default for Clock {
    fn default() -> Self {
        Self { seconds: 0.0 }
    }
}

impl Clock {
    pub fn update(&mut self, message: ClockMessage) {
        match message {
            ClockMessage::Time(value) => {
                self.seconds = value;
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let minutes: u8 = (self.seconds / 60.0).floor() as u8;
        let seconds = self.seconds as usize % 60;
        let thousandths = (self.seconds.fract() * 1000.0) as u16;
        container(
            text(format!("{:02}:{:02}:{:03}", minutes, seconds, thousandths))
                .font(NUMBERS_FONT)
                .size(NUMBERS_FONT_SIZE),
        )
        .style(theme::Container::Custom(GuiStuff::number_box_style))
        .into()
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
            #[allow(unused_variables)]
            Message::Tick(now) => {
                if let State::Ticking { last_tick } = &mut self.state {
                    self.control_bar
                        .clock
                        .update(ClockMessage::Time(self.orchestrator.elapsed_seconds()));
                    block_on(IOHelper::fill_audio_buffer(
                        self.audio_output.recommended_buffer_size(),
                        &mut self.orchestrator,
                        &mut self.audio_output,
                    ));
                }
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
            Message::EventOccurred(event) => {
                if let Event::Window(window::Event::CloseRequested) = event {
                    // See https://github.com/iced-rs/iced/pull/804
                    // and https://github.com/iced-rs/iced/blob/master/examples/events/src/main.rs#L55
                    //
                    // This is needed to stop an ALSA buffer underrun on close
                    (*self).audio_output.stop();

                    self.should_exit = true;
                }
            }
        }

        Command::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced_native::subscription::events().map(Message::EventOccurred),
            match self.state {
                State::Idle => Subscription::none(),
                State::Ticking { .. } => time::every(Duration::from_millis(10)).map(Message::Tick),
            },
        ])
    }

    fn should_exit(&self) -> bool {
        self.should_exit
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
