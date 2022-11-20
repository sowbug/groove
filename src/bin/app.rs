mod gui;

use async_std::task::block_on;
use crossbeam::deque::Steal; // TODO: this leaks into the app. Necessary?
use groove::{
    gui::{GuiStuff, IsViewable, ViewableMessage, NUMBERS_FONT, NUMBERS_FONT_SIZE},
    traits::BoxedEntity,
    AudioOutput, Clock, GrooveOrchestrator, IOHelper, MidiHandler, TimeSignature,
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

struct GrooveApp {
    // Overhead
    theme: Theme,
    state: State,
    should_exit: bool,
    last_tick: Instant, // A monotonically increasing value to tell when things have happened

    // UI components
    control_bar: ControlBar,

    // Model
    project_name: String,
    orchestrator: Box<GrooveOrchestrator>,
    clock: Clock,
    viewables: Vec<Box<dyn IsViewable<Message = ViewableMessage>>>,
    audio_output: AudioOutput,

    // Extra
    midi_handler_uid: usize, // MidiHandler
}

impl Default for GrooveApp {
    fn default() -> Self {
        Self {
            theme: Default::default(),
            state: Default::default(),
            should_exit: Default::default(),
            last_tick: Instant::now(),
            control_bar: Default::default(),
            project_name: Default::default(),
            orchestrator: Default::default(),
            clock: Default::default(),
            viewables: Default::default(),
            audio_output: Default::default(),
            midi_handler_uid: usize::default(),
        }
    }
}

#[derive(Default)]
enum State {
    #[default]
    Idle,
    Playing,
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
    gui_clock: GuiClock,
    midi: Midi,
}

impl ControlBar {
    pub fn view(&self, clock: &Clock, last_tick: Instant) -> Element<Message> {
        container(
            row![
                text_input(
                    "BPM",
                    clock.settings().bpm().round().to_string().as_str(),
                    Message::ControlBarBpm
                )
                .width(Length::Units(60)),
                container(row![
                    button(skip_to_prev_icon())
                        .width(Length::Units(32))
                        .on_press(Message::ControlBarMessage(ControlBarMessage::SkipToStart)),
                    button(play_icon())
                        .width(Length::Units(32))
                        .on_press(Message::ControlBarMessage(ControlBarMessage::Play)),
                    button(stop_icon())
                        .width(Length::Units(32))
                        .on_press(Message::ControlBarMessage(ControlBarMessage::Stop))
                ])
                .align_x(alignment::Horizontal::Center)
                .width(Length::FillPortion(1)),
                container(self.gui_clock.view()).width(Length::FillPortion(1)),
                container(self.midi.view(last_tick)).width(Length::FillPortion(1)),
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
    TimeSignature(u8, u8),
    Time(f32),
    Beats(f32),
}

#[derive(Debug, Default, Clone)]
struct GuiClock {
    time_signature: TimeSignature,
    seconds: f32,
    beats: f32,
}

impl GuiClock {
    pub fn update(&mut self, message: ClockMessage) {
        match message {
            ClockMessage::TimeSignature(top, bottom) => {
                // TODO: nobody sends this message. In order to send this
                // message correctly, either Clock needs a live pointer to
                // orchestrator, or we need to look into some way to subscribe
                // to orchestrator changes.
                self.time_signature = TimeSignature::new_with(top.into(), bottom.into());
            }
            ClockMessage::Time(value) => {
                self.seconds = value;
            }
            ClockMessage::Beats(value) => {
                self.beats = value;
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let time_counter = {
            let minutes: u8 = (self.seconds / 60.0).floor() as u8;
            let seconds = self.seconds as usize % 60;
            let thousandths = (self.seconds.fract() * 1000.0) as u16;
            container(
                text(format!("{minutes:02}:{seconds:02}:{thousandths:03}"))
                    .font(NUMBERS_FONT)
                    .size(NUMBERS_FONT_SIZE),
            )
            .style(theme::Container::Custom(
                GuiStuff::<Message>::number_box_style(&Theme::Dark),
            ))
        };

        let time_signature = {
            container(column![
                text(format!("{}", self.time_signature.top)),
                text(format!("{}", self.time_signature.bottom))
            ])
        };

        let beat_counter = {
            let denom = self.time_signature.top as f32;

            let measures = (self.beats / denom) as usize;
            let beats = (self.beats % denom) as usize;
            let fractional = (self.beats.fract() * 10000.0) as usize;
            container(
                text(format!("{measures:04}m{beats:02}b{fractional:03}"))
                    .font(NUMBERS_FONT)
                    .size(NUMBERS_FONT_SIZE),
            )
            .style(theme::Container::Custom(
                GuiStuff::<Message>::number_box_style(&Theme::Dark),
            ))
        };
        row![time_counter, time_signature, beat_counter].into()
    }
}

#[derive(Debug, Clone)]
pub enum MidiControlBarMessage {
    Inputs(Vec<(usize, String)>),
    Activity(Instant),
}

#[derive(Debug, Clone)]
struct Midi {
    inputs: Vec<(usize, String)>,
    activity_tick: Instant,
}

impl Default for Midi {
    fn default() -> Self {
        Self {
            inputs: Vec::default(),
            activity_tick: Instant::now(),
        }
    }
}

impl Midi {
    pub fn update(&mut self, message: MidiControlBarMessage) {
        match message {
            MidiControlBarMessage::Inputs(inputs) => {
                self.inputs = inputs;
            }
            MidiControlBarMessage::Activity(now) => self.activity_tick = now,
        }
    }

    pub fn view(&self, last_tick: Instant) -> Element<Message> {
        let mut s = String::new();
        for input in self.inputs.iter() {
            s = format!("{s} {}", input.1);
        }
        let input_dropdown = container(text(
            if last_tick.duration_since(self.activity_tick) > Duration::from_millis(250) {
                " "
            } else {
                "x"
            },
        ))
        .width(Length::FillPortion(1));
        let activity_indicator = container(text(s)).width(Length::FillPortion(7));
        row![input_dropdown, activity_indicator].into()
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
                last_tick: Instant::now(),
                ..Default::default()
            },
            Command::perform(SavedState::load(), Message::Loaded),
        )
    }

    fn theme(&self) -> Self::Theme {
        self.theme.clone()
    }

    fn title(&self) -> String {
        String::from(&self.project_name)
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Loaded(Ok(state)) => {
                let mut orchestrator = state.song_settings.instantiate().unwrap();

                // TODO BROKEN
                // let viewables = orchestrator
                //     .viewables()
                //     .iter()
                //     .map(|item| {
                //         if let Some(item) = item.upgrade() {
                //             if let Some(responder) = item.borrow_mut().make_is_viewable() {
                //                 responder
                //             } else {
                //                 panic!("make responder failed. Probably forgot new_wrapped()")
                //             }
                //         } else {
                //             panic!("upgrade failed")
                //         }
                //     })
                //     .collect();
                let midi_handler_uid = orchestrator.add(
                    None,
                    BoxedEntity::Controller(Box::new(MidiHandler::default())),
                );
                *self = Self {
                    theme: self.theme.clone(),
                    project_name: state.project_name,
                    orchestrator,
                    viewables: Vec::new(), // viewables, // TODO BROKEN
                    midi_handler_uid,
                    ..Default::default()
                };

                self.audio_output.start();
                // TODO BROKEN
                // //orchestrator.send_m
                // match self.midi.start() {
                //     Err(err) => println!("error starting MIDI: {}", err),
                //     _ => {}
                // }

                // TODO: no outputs because MidiOutputHandler is held inside a
                // RefCell, and thus can't give out addresses to any of its
                // data. I think the right model here is to make
                // RefreshMidiDevices and MidiDevicesRefreshed messages, and
                // then ask for updates when needed.

                // TODO: this should be via messages
                // BROKEN RIGHT NOW!
                // let inputs = self.midi.available_devices();
                // self.control_bar
                //     .midi
                //     .update(MidiControlBarMessage::Inputs(inputs.to_vec()));
            }
            Message::Loaded(Err(_)) => {
                todo!()
            }
            Message::Tick(now) => {
                self.last_tick = now;
                if let State::Playing = &mut self.state {
                    self.update_clock();
                    let done = block_on(IOHelper::fill_audio_buffer(
                        self.audio_output.recommended_buffer_size(),
                        &mut self.orchestrator,
                        &mut self.audio_output,
                    ));
                    if done {
                        self.state = State::Idle;
                    }
                }
                // TODO BROKEN
                // if let Some(stealer) = &self.midi.input_stealer() {
                //     while !stealer.is_empty() {
                //         if let Steal::Success((stamp, channel, message)) = stealer.steal() {
                //             // TODO: what does "now" mean to Orchestrator?
                //             let very_bad_temp_hack_clock = Clock::default();
                //             self.orchestrator.update(
                //                 &very_bad_temp_hack_clock,
                //                 GrooveMessage::Midi(channel, message),
                //             );
                //             self.control_bar
                //                 .midi
                //                 .update(MidiControlBarMessage::Activity(Instant::now()));
                //         }
                //     }
                // }
            }
            Message::ControlBarMessage(message) => match message {
                // TODO: not sure if we need ticking for now. it's playing OR
                // midi
                ControlBarMessage::Play => self.state = State::Playing,
                ControlBarMessage::Stop => match self.state {
                    State::Idle => {
                        self.clock.reset();
                        self.update_clock();
                    }
                    State::Playing => self.state = State::Idle,
                },
                ControlBarMessage::SkipToStart => todo!(),
            },
            Message::ControlBarBpm(new_value) => {
                if let Ok(bpm) = new_value.parse() {
                    self.clock.settings_mut().set_bpm(bpm);
                }
            }
            Message::ViewableMessage(i, message) => {
                if i == 999 {
                    // TODO: short-term hack!
                    self.orchestrator.pattern_manager_mut().update(message);
                } else {
                    let _ = self.viewables[i].update(message);
                    // TODO: deal with this command after wrapping it.
                }
            }
            Message::EventOccurred(event) => {
                if let Event::Window(window::Event::CloseRequested) = event {
                    // See https://github.com/iced-rs/iced/pull/804 and
                    // https://github.com/iced-rs/iced/blob/master/examples/events/src/main.rs#L55
                    //
                    // This is needed to stop an ALSA buffer underrun on close
                    dbg!("Close requested. I'm asking everyone to stop.");
                    // TODO BROKEN self.midi.stop();
                    self.audio_output.stop();

                    self.should_exit = true;
                }
            }
        }

        Command::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced_native::subscription::events().map(Message::EventOccurred),
            // This is duplicative because I think we'll want different activity
            // levels depending on whether we're playing
            match self.state {
                State::Idle => time::every(Duration::from_millis(10)).map(Message::Tick),
                State::Playing { .. } => time::every(Duration::from_millis(10)).map(Message::Tick),
            },
        ])
    }

    fn should_exit(&self) -> bool {
        if self.should_exit {
            // I think that self.midi or self.audio_output are causing the app
            // to hang randomly on exit. I'm going to keep this here to be
            // certain that the close code is really running.
            dbg!("I'm trying to exit!", self.should_exit);
        }
        self.should_exit
    }

    fn view(&self) -> Element<Message> {
        match self.state {
            State::Idle => {}
            State::Playing => {}
        }

        let control_bar = self.control_bar.view(&self.clock, self.last_tick);

        let views: Element<_> = if self.viewables.is_empty() {
            empty_message("nothing yet")
        } else {
            // Start the views from the IsViewables views.
            let mut view_vec: Vec<Element<Message>> = self
                .viewables
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    item.view()
                        .map(move |message| Message::ViewableMessage(i, message))
                })
                .collect();

            // TODO BROKEN
            // Add in the view of the non-IsViewable PatternManager.
            // view_vec.push(
            //     self.orchestrator
            //         .view()
            //         .map(move |message| Message::ViewableMessage(999, message)),
            // );
            column(view_vec).spacing(10).into()
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

impl GrooveApp {
    fn update_clock(&mut self) {
        // TODO law of demeter - should we be reaching in here, or just tell
        // ControlBar?
        self.control_bar
            .gui_clock
            .update(ClockMessage::Time(self.clock.seconds()));
        self.control_bar
            .gui_clock
            .update(ClockMessage::Beats(self.clock.beats()));
    }
}

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
