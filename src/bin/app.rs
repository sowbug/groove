// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! This crate is a rudimentary DAW.

#![windows_subsystem = "windows"]

mod gui;

use groove::{
    app_version,
    subscriptions::{
        EngineEvent, EngineInput, EngineSubscription, MidiHandler, MidiHandlerEvent,
        MidiHandlerInput, MidiSubscription,
    },
    Entity, Orchestrator, {DEFAULT_BPM, DEFAULT_MIDI_TICKS_PER_SECOND, DEFAULT_SAMPLE_RATE},
};
use groove_core::{
    time::{Clock, TimeSignature},
    Normal, Sample,
};
use groove_entities::EntityMessage;
use groove_orchestration::messages::GrooveEvent;
use gui::{
    persistence::{LoadError, Preferences, SaveError},
    play_icon, skip_to_prev_icon, stop_icon,
    views::{EntityViewGenerator, EntityViewState},
    GuiStuff,
};
use iced::{
    alignment, executor,
    futures::channel::mpsc,
    theme::{self, Theme},
    widget::{
        button,
        canvas::{self, Cache, Cursor},
        column, container, pick_list, row, scrollable, text, text_input, Canvas, Container,
    },
    window, Alignment, Application, Color, Command, Element, Event, Length, Point, Rectangle,
    Renderer, Settings, Size, Subscription,
};
use iced_audio::FloatRange;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Default, Debug)]
enum MainViews {
    #[default]
    Unstructured,
    New,
    Session,
    Arrangement,
    Preferences,
}

struct GrooveApp {
    // Overhead
    preferences: Preferences,
    is_pref_load_complete: bool,
    theme: Theme,
    state: State,

    // We won't set this until all tasks (orchestrator thread, MIDI interface
    // thread, etc.) have been asked to shut down. If we forget, then the Iced
    // process hangs after the main window disappears.
    should_exit: bool,

    // View
    current_view: MainViews,

    // Model
    project_title: Option<String>,
    orchestrator_sender: Option<mpsc::Sender<EngineInput>>,
    orchestrator: Arc<Mutex<Orchestrator>>,
    clock_mirror: Clock, // this clock is just a cache of the real clock in Orchestrator.
    time_signature_mirror: TimeSignature, // same

    // This is true when playback went all the way to the end of the song. The
    // reason it's nice to track this is that after pressing play and listening
    // to the song, the user can press play again without manually resetting the
    // clock to the start. But we don't want to just reset the clock at the end
    // of playback, because that means the clock would read zero at the end of
    // playback, which is undesirable because it's natural to want to know how
    // long the song was after listening, and it's nice to be able to glance at
    // the stopped clock and get that answer.
    reached_end_of_playback: bool,

    midi_handler_sender: Option<mpsc::Sender<MidiHandlerInput>>,
    midi_handler: Option<Arc<Mutex<MidiHandler>>>,

    gui_state: GuiState,

    entity_view_generator: EntityViewGenerator,
}
impl Default for GrooveApp {
    fn default() -> Self {
        // TODO: these are (probably) temporary until the project is
        // loaded. Make sure they really need to be instantiated.
        let clock = Clock::new_with(
            DEFAULT_SAMPLE_RATE,
            DEFAULT_BPM,
            DEFAULT_MIDI_TICKS_PER_SECOND,
        );
        let orchestrator = Orchestrator::new_with(DEFAULT_SAMPLE_RATE, DEFAULT_BPM);
        let orchestrator = Arc::new(Mutex::new(orchestrator));
        Self {
            preferences: Default::default(),
            is_pref_load_complete: false,
            theme: Default::default(),
            state: Default::default(),
            should_exit: Default::default(),
            current_view: Default::default(),
            project_title: None,
            orchestrator_sender: Default::default(),
            orchestrator: orchestrator.clone(),
            clock_mirror: clock,
            time_signature_mirror: Default::default(),
            reached_end_of_playback: Default::default(),
            midi_handler_sender: Default::default(),
            midi_handler: Default::default(),
            gui_state: GuiState::new(orchestrator),
            entity_view_generator: Default::default(),
        }
    }
}

#[derive(Default)]
enum State {
    #[default]
    Idle,
    Playing,
}

#[derive(Clone, Debug)]
pub enum AppMessage {
    PrefsLoaded(Result<Preferences, LoadError>),
    PrefsSaved(Result<(), SaveError>),
    ControlBarMessage(ControlBarMessage),
    GrooveEvent(GrooveEvent),
    EngineEvent(EngineEvent),
    MidiHandlerInput(MidiHandlerInput),
    MidiHandlerEvent(MidiHandlerEvent),
    Tick(Instant),
    Event(iced::Event),
}

#[derive(Debug, Clone)]
pub enum ControlBarMessage {
    Play,
    Stop,
    SkipToStart,
    Bpm(String),
}

impl Application for GrooveApp {
    type Executor = executor::Default;
    type Message = AppMessage;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (GrooveApp, Command<AppMessage>) {
        (
            GrooveApp {
                theme: Theme::Dark,
                ..Default::default()
            },
            Command::perform(Preferences::load_prefs(), AppMessage::PrefsLoaded),
        )
    }

    fn title(&self) -> String {
        if let Some(title) = &self.project_title {
            title.clone()
        } else {
            String::from("New Project")
        }
    }

    fn update(&mut self, message: AppMessage) -> Command<AppMessage> {
        match message {
            AppMessage::PrefsLoaded(Ok(preferences)) => {
                self.preferences = preferences;
                self.is_pref_load_complete = true;
            }
            AppMessage::PrefsLoaded(Err(_)) => {
                self.is_pref_load_complete = true;
                self.preferences = Preferences::default();
            }
            AppMessage::Tick(_now) => {
                // if let Ok(o) = self.orchestrator.lock() {
                //     self.gui_state.update_state(o);
                // }
                self.gui_state.update_state();
            }
            AppMessage::ControlBarMessage(message) => match message {
                // TODO: not sure if we need ticking for now. it's playing OR
                // midi
                ControlBarMessage::Play => {
                    if self.reached_end_of_playback {
                        self.post_to_orchestrator(EngineInput::SkipToStart);
                        self.reached_end_of_playback = false;
                    }
                    self.post_to_orchestrator(EngineInput::Play);
                    self.state = State::Playing
                }
                ControlBarMessage::Stop => {
                    self.post_to_orchestrator(EngineInput::Pause);
                    self.reached_end_of_playback = false;
                    match self.state {
                        State::Idle => {
                            self.post_to_orchestrator(EngineInput::SkipToStart);
                        }
                        State::Playing => self.state = State::Idle,
                    }
                }
                ControlBarMessage::SkipToStart => todo!(),
                ControlBarMessage::Bpm(value) => {
                    if let Ok(bpm) = value.parse() {
                        self.post_to_orchestrator(EngineInput::SetBpm(bpm));
                    }
                }
            },
            AppMessage::Event(event) => {
                if let Event::Window(window::Event::CloseRequested) = event {
                    println!("got CloseRequested");
                    return self.handle_close_requested_event();
                }
                if let Event::Keyboard(e) = event {
                    self.handle_keyboard_event(e);
                }
            }
            AppMessage::MidiHandlerInput(message) => match message {
                MidiHandlerInput::SelectMidiInput(which) => {
                    self.post_to_midi_handler(MidiHandlerInput::SelectMidiInput(which));
                }
                MidiHandlerInput::SelectMidiOutput(which) => {
                    self.post_to_midi_handler(MidiHandlerInput::SelectMidiOutput(which));
                }
                _ => todo!("Remaining MidiHandlerInput messages should be handled internally"),
            },
            AppMessage::EngineEvent(event) => match event {
                EngineEvent::Ready(sender, orchestrator) => {
                    self.orchestrator_sender = Some(sender);
                    self.orchestrator = orchestrator.clone();
                    self.gui_state.set_orchestrator(orchestrator);

                    // We don't start the GrooveSubscription until prefs are
                    // done loading, so this boolean and the corresponding
                    // filename should be set by the time we look at it.
                    if self.preferences.should_reload_last_project {
                        if let Some(last_project_filename) = &self.preferences.last_project_filename
                        {
                            self.post_to_orchestrator(EngineInput::LoadProject(
                                last_project_filename.to_string(),
                            ));
                        }
                    }
                }
                EngineEvent::SetClock(samples) => self.clock_mirror.seek(samples),
                EngineEvent::SetBpm(bpm) => self.clock_mirror.set_bpm(bpm),
                EngineEvent::SetTimeSignature(time_signature) => {
                    self.time_signature_mirror = time_signature;
                }
                EngineEvent::MidiToExternal(channel, message) => {
                    self.post_to_midi_handler(MidiHandlerInput::Midi(channel, message));
                }
                EngineEvent::AudioOutput(_) => todo!(),
                EngineEvent::OutputComplete => {
                    self.reached_end_of_playback = true;
                    self.state = State::Idle;
                }
                EngineEvent::Quit => todo!(),
                EngineEvent::ProjectLoaded(filename, title) => {
                    self.preferences.last_project_filename = Some(filename);
                    self.project_title = title;
                    self.entity_view_generator.reset();
                }
            },
            AppMessage::MidiHandlerEvent(event) => match event {
                MidiHandlerEvent::Ready(sender, midi_handler) => {
                    self.midi_handler_sender = Some(sender);
                    self.midi_handler = Some(midi_handler);
                }
                #[allow(unused_variables)]
                MidiHandlerEvent::Midi(channel, event) => {
                    // TODO
                }
                MidiHandlerEvent::Quit => {
                    // TODO: If we were waiting for this to shut down, then
                    // record that we're ready. For now, it's nice to know, but
                    // we won't do anything about it.
                }
            },
            AppMessage::GrooveEvent(event) => match event {
                GrooveEvent::EntityMessage(uid, message) => match message {
                    EntityMessage::ExpandPressed => {
                        // Find whoever else is expanded and maybe collapse them
                        self.entity_view_generator
                            .set_entity_view_state(uid, EntityViewState::Expanded);
                    }
                    EntityMessage::CollapsePressed => {
                        self.entity_view_generator
                            .set_entity_view_state(uid, EntityViewState::Collapsed);
                    }
                    _ => {
                        self.entity_update(uid, message);
                    }
                },
                _ => todo!(),
            },
            AppMessage::PrefsSaved(r) => {
                if self.should_exit {
                    eprintln!("calling close()");
                    return window::close::<Self::Message>();
                } else {
                    match r {
                        Ok(_) => {}
                        Err(_) => todo!(),
                    }
                }
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<AppMessage> {
        match self.state {
            State::Idle | State::Playing => {}
        }

        let control_bar = self.control_bar_view().map(AppMessage::ControlBarMessage);
        let main_content = match self.current_view {
            MainViews::Unstructured => {
                let project_view: Element<AppMessage> =
                    self.orchestrator_view().map(AppMessage::GrooveEvent);
                let midi_view: Element<AppMessage> =
                    self.midi_view().map(AppMessage::MidiHandlerInput);
                let scrollable_content = column![midi_view, project_view];
                let scrollable =
                    container(scrollable(scrollable_content)).width(Length::FillPortion(1));
                container(row![Self::under_construction("Unstructured"), scrollable])
            }
            MainViews::New => {
                let project_view: Element<AppMessage> =
                    self.orchestrator_new_view().map(AppMessage::GrooveEvent);
                let scrollable = container(scrollable(project_view)).width(Length::FillPortion(1));
                container(scrollable)
            }
            MainViews::Session => container(Self::under_construction("Session")),
            MainViews::Arrangement => container(Self::under_construction("Arrangement")),
            MainViews::Preferences => container(Self::under_construction("Preferences")),
        };
        let full_view = column![control_bar, main_content]
            .align_items(Alignment::Center)
            .spacing(20);

        container(full_view)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .align_y(alignment::Vertical::Top)
            .into()
    }

    fn theme(&self) -> Self::Theme {
        self.theme.clone()
    }

    fn subscription(&self) -> Subscription<AppMessage> {
        let mut v = vec![
            iced_native::subscription::events().map(AppMessage::Event),
            MidiSubscription::subscription().map(AppMessage::MidiHandlerEvent),
            window::frames().map(AppMessage::Tick),
        ];
        if self.is_pref_load_complete {
            v.push(EngineSubscription::subscription().map(AppMessage::EngineEvent));
        }
        Subscription::batch(v)
    }
}

impl GrooveApp {
    fn under_construction(section_name: &str) -> Container<AppMessage> {
        container(GuiStuff::<AppMessage>::container_text(
            format!("Coming soon: {}", section_name).as_str(),
        ))
        .width(Length::FillPortion(1))
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
    }

    fn post_to_midi_handler(&mut self, input: MidiHandlerInput) {
        if let Some(sender) = self.midi_handler_sender.as_mut() {
            // TODO: deal with this
            let _ = sender.try_send(input);
        }
    }

    fn post_to_orchestrator(&mut self, input: EngineInput) {
        if let Some(sender) = self.orchestrator_sender.as_mut() {
            // TODO: deal with this
            let _ = sender.try_send(input);
        }
    }

    fn entity_update(&mut self, uid: usize, message: EntityMessage) {
        if let Ok(mut o) = self.orchestrator.lock() {
            if let Some(entity) = o.get_mut(uid) {
                // TODO: we don't have a real clock here... solve this.
                match entity {
                    Entity::BiQuadFilter(e) => match message {
                        EntityMessage::HSliderInt(value) => {
                            e.set_cutoff_pct(value.as_f32());
                        }
                        _ => todo!(),
                    },
                    Entity::Bitcrusher(e) => match message {
                        EntityMessage::HSliderInt(value) => {
                            e.set_bits_to_crush(value.as_f32() as u8);
                        }
                        _ => todo!(),
                    },
                    Entity::Compressor(e) => match message {
                        EntityMessage::HSliderInt(v) => e.set_threshold(v.as_f32()),
                        _ => todo!(),
                    },
                    Entity::Gain(e) => match message {
                        EntityMessage::HSliderInt(value) => {
                            e.set_ceiling(Normal::new_from_f32(value.as_f32()));
                        }
                        _ => todo!(),
                    },
                    Entity::WelshSynth(e) => match message {
                        EntityMessage::IcedKnob(value) => {
                            // TODO: it's annoying to have to plumb this through. I want
                            // everything #controllable to automatically generate the
                            // scaffolding for UI.
                            e.set_control_pan(groove_core::control::F32ControlValue(
                                value.as_f32(),
                            ));
                        }
                        _ => todo!(),
                    },
                    Entity::Arpeggiator(_) => todo!(),
                    Entity::Sequencer(_) => todo!(),
                    Entity::ControlTrip(_) => todo!(),
                    Entity::MidiTickSequencer(_) => todo!(),
                    Entity::LfoController(_) => todo!(),
                    Entity::PatternManager(_) => todo!(),
                    Entity::SignalPassthroughController(_) => todo!(),
                    Entity::ToyController(_) => todo!(),
                    Entity::Timer(_) => todo!(),
                    Entity::Chorus(_) => todo!(),
                    Entity::Delay(_) => todo!(),
                    Entity::Limiter(_) => todo!(),
                    Entity::Mixer(_) => todo!(),
                    Entity::Reverb(_) => todo!(),
                    Entity::ToyEffect(_) => todo!(),
                    Entity::Drumkit(_) => todo!(),
                    Entity::FmSynthesizer(e) => match message {
                        EntityMessage::Midi(_, _) => todo!(),
                        EntityMessage::ControlF32(_) => todo!(),
                        EntityMessage::PatternMessage(_, _) => todo!(),
                        EntityMessage::HSliderInt(depth) => {
                            e.set_depth(Normal::from(depth.as_f32()))
                        }
                        EntityMessage::HSliderInt1(ratio) => {
                            e.set_ratio(FloatRange::new(0.5, 32.0).unmap_to_value(ratio) as f64)
                        }
                        EntityMessage::HSliderInt2(beta) => e.set_beta(beta.as_f32() as f64),
                        EntityMessage::HSliderInt3(_) => todo!(),
                        EntityMessage::HSliderInt4(_) => todo!(),
                        EntityMessage::IcedKnob(_) => todo!(),
                        EntityMessage::PickListSelected(_) => todo!(),
                        EntityMessage::ExpandPressed => todo!(),
                        EntityMessage::CollapsePressed => todo!(),
                    },
                    Entity::Sampler(_) => todo!(),
                    Entity::ToyAudioSource(_) => todo!(),
                    Entity::ToyInstrument(_) => todo!(),
                    Entity::ToySynth(_) => todo!(),
                }
            }
        }
    }

    fn orchestrator_view(&self) -> Element<GrooveEvent> {
        if let Ok(orchestrator) = self.orchestrator.lock() {
            let canvas: Element<'_, GrooveEvent, Renderer<<GrooveApp as Application>::Theme>> =
                Canvas::new(&self.gui_state)
                    .width(Length::Fill)
                    .height(Length::Fixed((32 * 4) as f32))
                    .into();

            let mut views = orchestrator
                .entity_iter()
                .fold(Vec::new(), |mut v, (&uid, e)| {
                    v.push(
                        self.entity_view_generator
                            .entity_view(e)
                            .map(move |message| GrooveEvent::EntityMessage(uid, message)),
                    );
                    v
                });
            views.push(canvas);
            column(views).into()
        } else {
            panic!()
        }
    }

    fn orchestrator_new_view(&self) -> Element<GrooveEvent> {
        if let Ok(_orchestrator) = self.orchestrator.lock() {
            let canvas: Element<'_, GrooveEvent, Renderer<<GrooveApp as Application>::Theme>> =
                Canvas::new(&self.gui_state)
                    .width(Length::Fill)
                    .height(Length::Fixed((32 * 4) as f32))
                    .into();
            canvas.into()
        } else {
            panic!()
        }
    }

    fn midi_view(&self) -> Element<MidiHandlerInput> {
        if let Some(midi_handler) = &self.midi_handler {
            if let Ok(midi_handler) = midi_handler.lock() {
                let activity_text = container(GuiStuff::<EntityMessage>::container_text(
                    if Instant::now().duration_since(midi_handler.activity_tick())
                        > Duration::from_millis(250)
                    {
                        " "
                    } else {
                        "•"
                    },
                ))
                .width(iced::Length::FillPortion(1));
                let (input_selected, input_options) =
                    midi_handler.midi_input().as_ref().unwrap().labels();

                let input_menu = row![
                    GuiStuff::<EntityMessage>::container_text("Input")
                        .width(iced::Length::FillPortion(1)),
                    pick_list(
                        input_options,
                        input_selected.clone(),
                        MidiHandlerInput::SelectMidiInput,
                    )
                    .font(gui::SMALL_FONT)
                    .width(iced::Length::FillPortion(3))
                ];
                let (output_selected, output_options) =
                    midi_handler.midi_output().as_ref().unwrap().labels();
                let x = pick_list(
                    output_options,
                    output_selected.clone(),
                    MidiHandlerInput::SelectMidiOutput,
                );
                let output_menu = row![
                    GuiStuff::<EntityMessage>::container_text("Output")
                        .width(iced::Length::FillPortion(1)),
                    x.font(gui::SMALL_FONT).width(iced::Length::FillPortion(3))
                ];
                let port_menus =
                    container(column![input_menu, output_menu]).width(iced::Length::FillPortion(7));
                GuiStuff::titled_container(
                    "MIDI",
                    container(row![activity_text, port_menus]).into(),
                )
            } else {
                panic!()
            }
        } else {
            GuiStuff::titled_container(
                "MIDI",
                GuiStuff::<EntityMessage>::container_text("Initializing...").into(),
            )
        }
    }

    fn control_bar_view(&self) -> Element<ControlBarMessage> {
        container(
            row![
                text_input(
                    "BPM",
                    self.clock_mirror.bpm().round().to_string().as_str(),
                    ControlBarMessage::Bpm
                )
                .font(gui::SMALL_FONT)
                .size(gui::SMALL_FONT_SIZE)
                .width(Length::Fixed(60.0)),
                container(row![
                    button(skip_to_prev_icon())
                        .width(Length::Fixed(32.0))
                        .on_press(ControlBarMessage::SkipToStart),
                    button(play_icon())
                        .width(Length::Fixed(32.0))
                        .on_press(ControlBarMessage::Play),
                    button(stop_icon())
                        .width(Length::Fixed(32.0))
                        .on_press(ControlBarMessage::Stop)
                ])
                .align_x(alignment::Horizontal::Center)
                .width(Length::FillPortion(1)),
                container(self.clock_view()).width(Length::FillPortion(1)),
                container(text(app_version())).align_x(alignment::Horizontal::Right)
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

    fn clock_view(&self) -> Element<ControlBarMessage> {
        let time_counter = {
            let minutes: u8 = (self.clock_mirror.seconds() / 60.0).floor() as u8;
            let seconds = self.clock_mirror.seconds() as usize % 60;
            let thousandths = (self.clock_mirror.seconds().fract() * 1000.0) as u16;
            container(
                text(format!("{minutes:02}:{seconds:02}:{thousandths:03}"))
                    .font(gui::NUMBERS_FONT)
                    .size(gui::NUMBERS_FONT_SIZE),
            )
            .style(theme::Container::Custom(
                GuiStuff::<ControlBarMessage>::number_box_style(&Theme::Dark),
            ))
        };

        let time_signature_view = {
            container(column![
                text(format!("{}", self.time_signature_mirror.top))
                    .font(gui::SMALL_FONT)
                    .size(gui::SMALL_FONT_SIZE),
                text(format!("{}", self.time_signature_mirror.bottom))
                    .font(gui::SMALL_FONT)
                    .size(gui::SMALL_FONT_SIZE)
            ])
        };

        let beat_counter = {
            let denom = self.time_signature_mirror.top as f64;

            let measures = (self.clock_mirror.beats() / denom) as usize;
            let beats = (self.clock_mirror.beats() % denom) as usize;
            let fractional = (self.clock_mirror.beats().fract() * 10000.0) as usize;
            container(
                text(format!("{measures:04}m{beats:02}b{fractional:03}"))
                    .font(gui::NUMBERS_FONT)
                    .size(gui::NUMBERS_FONT_SIZE),
            )
            .style(theme::Container::Custom(
                GuiStuff::<AppMessage>::number_box_style(&Theme::Dark),
            ))
        };
        row![time_counter, time_signature_view, beat_counter].into()
    }

    fn switch_main_view(&mut self) {
        self.current_view = match self.current_view {
            MainViews::Unstructured => MainViews::New,
            MainViews::New => MainViews::Session,
            MainViews::Session => MainViews::Arrangement,
            MainViews::Arrangement => MainViews::Preferences,
            MainViews::Preferences => MainViews::Unstructured,
        }
    }

    fn handle_close_requested_event(&mut self) -> Command<AppMessage> {
        // See https://github.com/iced-rs/iced/pull/804 and
        // https://github.com/iced-rs/iced/blob/master/examples/events/src/main.rs#L55
        //
        // This is needed to stop an ALSA buffer underrun on close
        self.post_to_midi_handler(MidiHandlerInput::QuitRequested);
        self.post_to_orchestrator(EngineInput::QuitRequested);

        // Let the PrefsSaved message handler know that it's time to go.
        self.should_exit = true;

        Command::perform(
            Preferences::save_prefs(Preferences {
                selected_midi_input: self.preferences.selected_midi_input.clone(),
                selected_midi_output: self.preferences.selected_midi_output.clone(),
                should_reload_last_project: self.preferences.should_reload_last_project,
                last_project_filename: self.preferences.last_project_filename.clone(),
            }),
            AppMessage::PrefsSaved,
        )
    }

    fn handle_keyboard_event(&mut self, event: iced::keyboard::Event) {
        // This recently changed, and I don't get KeyPressed anymore. Maybe this
        // is a new event that processes KeyPressed/KeyReleased, so they're no
        // longer "ignored runtime events."
        if let iced::keyboard::Event::CharacterReceived(char) = event {
            if char == '\t' {
                self.switch_main_view();
            }
        }
    }
}

/// GuiState helps with GUI drawing. It gets called during AppMessage::Tick with
/// a ref to a mutex-locked Orchestrator. It grabs whatever information it needs
/// to handle canvas draw() operations.
struct GuiState {
    background: Cache,
    foreground: Cache,

    orchestrator: Arc<Mutex<Orchestrator>>,
}
impl GuiState {
    fn new(orchestrator: Arc<Mutex<Orchestrator>>) -> Self {
        Self {
            background: Default::default(),
            foreground: Default::default(),
            orchestrator,
        }
    }

    // I don't know how
    // https://github.com/iced-rs/iced/blob/master/examples/solar_system/src/main.rs
    // was able to call its method update() without getting an error about
    // stomping on the Program trait's update().
    fn update_state(&mut self) {
        // TODO: we can be smarter about when to redraw. We can also store more
        // stuff in GuiState that won't change that often if we think that'll be
        // more efficient than querying Orchestrator each time. Unless we're
        // doing rendering-specific transformations of Orchestrator data,
        // though, I'm not sure that's a win.

        self.foreground.clear();
    }

    fn set_orchestrator(&mut self, orchestrator: Arc<Mutex<Orchestrator>>) {
        self.orchestrator = orchestrator;
        self.background.clear();
        self.foreground.clear();
    }
}
impl<GrooveMessage> canvas::Program<GrooveMessage> for GuiState {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        _theme: &iced_native::Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<iced::widget::canvas::Geometry> {
        if let Ok(orchestrator) = self.orchestrator.lock() {
            let track_samples = orchestrator.track_samples().to_vec();
            let background = self.background.draw(bounds.size(), |frame| {
                let track_size = Size {
                    width: bounds.width,
                    height: 32.0,
                };
                for (i, _sample) in track_samples.iter().enumerate() {
                    let top_left = Point {
                        x: 0.0,
                        y: (i * 32) as f32,
                    };
                    frame.fill_rectangle(top_left, track_size, Color::BLACK);
                }
            });

            let foreground = self.foreground.draw(bounds.size(), |frame| {
                let track_size = Size {
                    width: bounds.width,
                    height: 16.0,
                };
                for (i, sample) in track_samples.iter().enumerate() {
                    let top_left = Point {
                        x: 0.0,
                        y: (i * 32) as f32,
                    };

                    // TODO: need to map because it doesn't seem linear
                    let amplitude_mono: Sample = (*sample).into();
                    let amplitude_magnitude = amplitude_mono.0.abs() as f32;

                    frame.fill_rectangle(
                        top_left,
                        track_size,
                        Color::from_rgb(amplitude_magnitude, amplitude_magnitude, 0.0),
                    );
                }
            });

            vec![background, foreground]
        } else {
            Vec::default()
        }
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
