mod gui;

use groove::gui::{GrooveMessage, GuiStuff, IsViewable};
use groove::{BorderedContainer, IOHelper, Orchestrator, SongSettings};
use gui::persistence::{LoadError, SaveError, SavedState};
use gui::{mute_icon, play_icon, skip_to_prev_icon, stop_icon};
use iced::alignment::{self};
use iced::scrollable::{self, Scrollable};
use iced::text_input::{self, TextInput};
use iced::{
    button, Alignment, Application, Button, Column, Command, Container, Element, Length, Row,
    Settings, Text,
};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum Message {
    Loaded(Result<SavedState, LoadError>),
    Saved(Result<(), SaveError>),
    TrackMessage(usize, TrackMessage),
    ControlBarPlay,
    ControlBarStop,
    ControlBarBpmChange(String),
    GrooveMessage(usize, GrooveMessage),
    PlayComplete(Result<bool, &'static str>),
}

pub fn main() -> iced::Result {
    Groove::run(Settings::default())
}

#[derive(Debug)]
enum Groove {
    Loading,
    Loaded(State),
}

#[derive(Debug, Default, Clone)]
pub struct ControlBar {
    skip_to_start_button: button::State,
    play_button: button::State,
    stop_button: button::State,

    bpm: f32,
    bpm_text_input: text_input::State,

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

    pub fn view(&mut self) -> Container<Message> {
        Container::new(
            Row::new()
                .spacing(4)
                .align_items(Alignment::Start)
                .push(
                    TextInput::new(
                        &mut self.bpm_text_input,
                        "BPM",
                        format!("{}", self.bpm.floor()).as_str(),
                        Message::ControlBarBpmChange,
                    )
                    .width(Length::Units(40)),
                )
                .push(
                    Button::new(&mut self.skip_to_start_button, skip_to_prev_icon())
                        .width(Length::Units(32))
                        .on_press(Message::ControlBarPlay),
                )
                .push(
                    Button::new(&mut self.play_button, play_icon())
                        .width(Length::Units(32))
                        .on_press(Message::ControlBarPlay),
                )
                .push(
                    Button::new(&mut self.stop_button, stop_icon())
                        .width(Length::Units(32))
                        .on_press(Message::ControlBarStop),
                )
                .push(self.clock.view()),
        )
        .style(BorderedContainer::default())
        .width(Length::Fill)
        .padding(4)
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
    pub fn update(&mut self, _message: ControlBarMessage) {}

    pub fn view(&mut self) -> Container<Message> {
        Container::new(Text::new(self.current_time.clone()))
            .padding(4)
            .style(BorderedContainer::default())
    }
}

#[derive(Clone, Debug)]
enum TrackState {
    Idle { mute_button: button::State },
}

impl Default for TrackState {
    fn default() -> Self {
        TrackState::Idle {
            mute_button: button::State::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Track {
    name: String,
    muted: bool,

    #[serde(skip)]
    state: TrackState,
}

#[derive(Debug, Clone)]
pub enum TrackMessage {
    Mute,
}

impl Track {
    pub fn update(&mut self, message: TrackMessage) {
        println!("->>>>> {:?} {:?}", message, self);
    }

    pub fn view(&mut self) -> Element<TrackMessage> {
        let is_muted = self.muted;
        match &mut self.state {
            TrackState::Idle { mute_button } => {
                let content: Container<TrackMessage> =
                    Container::new(Text::new("your sounds here"))
                        .style(BorderedContainer::default())
                        .width(Length::Fill);
                let mute_button =
                    Button::new(mute_button, mute_icon(is_muted)).on_press(TrackMessage::Mute);

                Row::new()
                    .spacing(20)
                    .align_items(Alignment::Center)
                    .push(mute_button)
                    .push(content)
                    .into()
            }
        }
    }

    fn name(&self) -> &str {
        self.name.as_ref()
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn muted(&self) -> bool {
        self.muted
    }

    fn set_muted(&mut self, is_muted: bool) {
        self.muted = is_muted;
    }

    fn new() -> Self {
        Self::default()
    }

    fn new_with_fake_data() -> Self {
        let mut r = Self::new();
        let mut rng = rand::thread_rng();
        r.name = format!("Track #{}", rng.gen::<u32>());
        r
    }
}

#[derive(Debug, Default)]
struct State {
    scroll: scrollable::State,
    control_bar: ControlBar,
    dirty: bool,
    saving: bool,

    project_name: String,
    orchestrator: Orchestrator,
    viewables: Vec<Box<dyn IsViewable>>,
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
        let project_name = match self {
            Groove::Loading => "new",
            Groove::Loaded(state) => state.project_name.as_str(),
        };

        format!("Groove - {}", project_name)
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match self {
            Groove::Loading => {
                match message {
                    Message::Loaded(Ok(state)) => {
                        // TODO: handle error
                        let orchestrator = state.song_settings.instantiate().unwrap();
                        let viewables = orchestrator
                            .viewables()
                            .iter()
                            .map(|item| {
                                if let Some(item) = item.upgrade() {
                                    if let Some(responder) = item.borrow_mut().make_is_viewable() {
                                        responder
                                    } else {
                                        panic!(
                                            "make responder failed. Probably forgot new_wrapped()"
                                        )
                                    }
                                } else {
                                    panic!("upgrade failed")
                                }
                            })
                            .collect();
                        *self = Groove::Loaded(State {
                            project_name: state.project_name.clone(),
                            orchestrator,
                            viewables,
                            ..State::default()
                        });
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
                    Message::Saved(_) => {
                        state.saving = false;
                        saved = true;
                    }
                    Message::Loaded(_) => todo!(),
                    Message::TrackMessage(i, track_message) => {
                        // if let Some(track) = state.fake_state.tracks.get_mut(i) {
                        //     track.update(track_message.clone());
                        // }
                        // match track_message {
                        //     TrackMessage::Mute => {
                        //         state.fake_state.toggle_audio_source_mute(i);
                        //     }
                        // }
                    }
                    Message::ControlBarPlay => {
                        dbg!("Message::ControlBarPlay");
                        return Command::perform(IOHelper::perform_async(), Message::PlayComplete)
                        // if let Ok(performance) = state.orchestrator.perform() {
                        //     IOHelper::send_performance_to_output_device(performance);
                        // }
                    }
                    Message::ControlBarStop => {
                        dbg!("Message::ControlBarStop");
                    }
                    Message::ControlBarBpmChange(new_value) => {
                        if let Ok(new_value) = new_value.parse() {
                            state.control_bar.update(ControlBarMessage::Bpm(new_value));
                            state.orchestrator.set_bpm(new_value);
                        }
                    }
                    Message::GrooveMessage(i, groove_message) => {
                        state.viewables[i].update(groove_message);
                    }
                    Message::PlayComplete(result) => {
                        dbg!("got {:?}", result.is_ok());
                    }
                }

                // TODO: we probably don't want this dirty/save logic for our app
                if !saved {
                    state.dirty = true;
                }

                if state.dirty && !state.saving {
                    state.dirty = false;
                    state.saving = true;

                    Command::perform(
                        SavedState {
                            project_name: state.project_name.clone(),
                            song_settings: SongSettings::default(), // TODO orchestrator ->>>
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
                control_bar,
                orchestrator,
                viewables,
                ..
            }) => {
                let control_bar: Column<_> = Column::new().spacing(20).push(control_bar.view());

                let views: Element<_> = if orchestrator.viewables().is_empty() {
                    empty_message("nothing yet")
                } else {
                    viewables
                        .iter_mut()
                        .enumerate()
                        .fold(Column::new().spacing(0), |column, (i, item)| {
                            column.push(
                                item.view()
                                    .map(move |message| Message::GrooveMessage(i, message)),
                            )
                        })
                        .into()
                };

                let scrollable_content = Column::new().spacing(0).padding(0).push(views);

                let scrollable = Scrollable::new(scroll).padding(0).push(scrollable_content);

                let under_construction = GuiStuff::container_text("Under Construction")
                    .horizontal_alignment(alignment::Horizontal::Center)
                    .vertical_alignment(alignment::Vertical::Center)
                    .height(Length::Fill);
                let main_workspace = Row::new()
                    .push(under_construction.width(Length::FillPortion(1)))
                    .push(scrollable.width(Length::FillPortion(1)));

                Column::new()
                    .push(control_bar)
                    .push(main_workspace)
                    .spacing(4)
                    .into()
            }
        }
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
