// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! This crate is a rudimentary DAW.

#![windows_subsystem = "windows"]

mod gui;

use crossbeam::queue::ArrayQueue;
use groove::{
    subscriptions::{
        EngineEvent, EngineInput, EngineSubscription, MidiHandlerEvent, MidiHandlerInput,
        MidiPortDescriptor, MidiSubscription,
    },
    util::{PathType, Paths},
    Orchestrator, {DEFAULT_BPM, DEFAULT_MIDI_TICKS_PER_SECOND},
};
use groove_core::{
    time::{Clock, ClockNano, TimeSignature},
    traits::Resets,
    Sample, StereoSample, SAMPLE_BUFFER_SIZE,
};
use groove_entities::EntityMessage;
use groove_orchestration::messages::{GrooveEvent, GrooveInput};
use groove_settings::SongSettings;
use gui::{
    persistence::{LoadError, OpenError, Preferences, SaveError},
    views::{ControlBar, ControlBarEvent, View, ViewMessage},
    GuiStuff,
};
use iced::{
    alignment, executor,
    theme::Theme,
    widget::{
        canvas::{self, Cache, Cursor},
        column, container, pick_list, row, Column,
    },
    window, Alignment, Application, Color, Command, Element, Event, Length, Point, Rectangle,
    Settings, Size, Subscription,
};
use native_dialog::{MessageDialog, MessageType};
use std::{
    path::PathBuf,
    sync::{mpsc, Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Default)]
enum State {
    #[default]
    Idle,
    Playing,
}

#[derive(Clone, Debug)]
enum AppMessage {
    ViewMessage(ViewMessage),
    ControlBarEvent(ControlBarEvent),
    EngineEvent(EngineEvent),
    Event(iced::Event),
    ExportComplete(Result<(), SaveError>),
    // GrooveEvent(GrooveEvent),
    MidiHandlerInput(MidiHandlerInput),
    MidiHandlerEvent(MidiHandlerEvent),
    OpenDialogComplete(Result<Option<PathBuf>, OpenError>),
    PrefsLoaded(Result<Preferences, LoadError>),
    PrefsSaved(Result<(), SaveError>),
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
    control_bar: ControlBar,
    views: View,
    show_settings: bool,

    // Model
    project_title: Option<String>,
    orchestrator: Orchestrator,
    engine_sender: Option<mpsc::Sender<EngineInput>>,
    ring_buffer: Option<Arc<ArrayQueue<StereoSample>>>,
    last_midi_activity: Instant,
    midi_handler_sender: Option<mpsc::Sender<MidiHandlerInput>>,
    midi_input_ports: Vec<MidiPortDescriptor>,
    midi_input_port_active: Option<MidiPortDescriptor>,
    midi_output_ports: Vec<MidiPortDescriptor>,
    midi_output_port_active: Option<MidiPortDescriptor>,
    // gui_state: GuiState,

    /////////////////////

    // This is true when playback went all the way to the end of the song. The
    // reason it's nice to track this is that after pressing play and listening
    // to the song, the user can press play again without manually resetting the
    // clock to the start. But we don't want to just reset the clock at the end
    // of playback, because that means the clock would read zero at the end of
    // playback, which is undesirable because it's natural to want to know how
    // long the song was after listening, and it's nice to be able to glance at
    // the stopped clock and get that answer.
    reached_end_of_playback: bool,

    received_midi_quit: bool,
    received_audio_quit: bool,
}
impl Default for GrooveApp {
    fn default() -> Self {
        // TODO: these are (probably) temporary until the project is
        // loaded. Make sure they really need to be instantiated.
        let clock_params = ClockNano {
            bpm: DEFAULT_BPM,
            midi_ticks_per_second: DEFAULT_MIDI_TICKS_PER_SECOND,
            time_signature: TimeSignature { top: 4, bottom: 4 },
        };
        let orchestrator = Orchestrator::new_with(clock_params.clone());
        Self {
            preferences: Default::default(),
            is_pref_load_complete: false,
            theme: Default::default(),
            state: Default::default(),
            should_exit: Default::default(),
            control_bar: ControlBar::new_with(Clock::new_with(clock_params)),
            views: View::new(),
            show_settings: Default::default(),
            project_title: None,
            engine_sender: Default::default(),
            ring_buffer: Default::default(),
            orchestrator,
            last_midi_activity: Instant::now(),
            midi_handler_sender: Default::default(),
            midi_input_ports: Default::default(),
            midi_input_port_active: Default::default(),
            midi_output_ports: Default::default(),
            midi_output_port_active: Default::default(),
            // gui_state: GuiState::new(orchestrator),
            reached_end_of_playback: false,
            //             audio_output: AudioOutput::new_with(input_sender.clone());
            received_midi_quit: false,
            received_audio_quit: false,
        }
    }
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
                eprintln!("prefs have loaded successfully.")
            }
            AppMessage::PrefsLoaded(Err(_)) => {
                self.is_pref_load_complete = true;
                self.preferences = Preferences::default();
            }
            // AppMessage::Tick(_now) => {
            //     // if let Ok(o) = self.orchestrator.lock() {
            //     //     self.gui_state.update_state(o);
            //     // }
            //     self.gui_state.update_state();
            // }
            AppMessage::ControlBarEvent(event) => {
                if let Some(command) = self.handle_control_bar_event(event) {
                    return command;
                }
            }
            AppMessage::Event(event) => {
                if let Some(value) = self.handle_system_event(event) {
                    return value;
                }
            }
            AppMessage::MidiHandlerInput(message) => self.handle_midi_handler_input(message),
            AppMessage::EngineEvent(event) => {
                if let Some(command) = self.handle_engine_event(event) {
                    return command;
                }
            }
            AppMessage::MidiHandlerEvent(event) => self.handle_midi_handler_event(event),
            // AppMessage::GrooveEvent(event) => self.handle_groove_event(event),
            AppMessage::PrefsSaved(r) => {
                if self.should_exit {
                    eprintln!("about to call window::close");
                    if !self.received_audio_quit {
                        eprintln!("haven't gotten audio quit");
                    }
                    if !self.received_midi_quit {
                        eprintln!("haven't gotten midi quit");
                    }
                    return window::close::<Self::Message>();
                } else {
                    match r {
                        Ok(_) => {}
                        Err(_) => todo!(),
                    }
                }
            }
            AppMessage::OpenDialogComplete(path_buf) => match path_buf {
                Ok(path) => {
                    if let Some(path) = path {
                        if let Some(path) = path.to_str() {
                            self.load_project(path);
                        }
                    }
                }
                Err(_) => todo!(),
            },
            AppMessage::ExportComplete(_) => {
                // great
            }
            AppMessage::ViewMessage(message) => {
                if let Some(response) = self.views.update(&mut self.orchestrator, message) {
                    match response {
                        ViewMessage::AddControlLink(link) => {
                            let _ = self.orchestrator.link_control_by_id(
                                link.source_uid,
                                link.target_uid,
                                link.control_index,
                            );
                        }
                        ViewMessage::RemoveControlLink(link) => {
                            self.orchestrator.unlink_control_by_id(
                                link.source_uid,
                                link.target_uid,
                                link.control_index,
                            );
                        }
                        ViewMessage::NextView
                        | ViewMessage::OtherEntityMessage(_, _)
                        | ViewMessage::MouseIn(_)
                        | ViewMessage::MouseOut(_)
                        | ViewMessage::MouseDown(_)
                        | ViewMessage::MouseUp(_) => panic!(),
                    }
                }
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<AppMessage> {
        let control_bar: Element<AppMessage> = self
            .control_bar
            .view(matches!(self.state, State::Playing))
            .map(AppMessage::ControlBarEvent);
        let main_content = match self.show_settings {
            true => self
                .midi_view()
                .map(move |m| AppMessage::MidiHandlerInput(m)),
            false => self
                .views
                .view(&self.orchestrator)
                .map(move |m| AppMessage::ViewMessage(m)),
        };
        container(
            Column::new()
                .push(control_bar)
                .push(main_content)
                .align_items(Alignment::Center)
                .spacing(20),
        )
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
            // TODO: this is for the canvas experiment I was doing. It's
            //disabled for now because it's spinning the CPU (which is as
            //expected because we weren't trying to be intelligent about when to
            //clear the canvas).
            #[cfg(disabled)]
            window::frames().map(AppMessage::Tick),
        ];
        if self.is_pref_load_complete {
            v.push(EngineSubscription::subscription().map(AppMessage::EngineEvent));
        }
        Subscription::batch(v)
    }
}

impl GrooveApp {
    fn handle_groove_event(&mut self, event: GrooveEvent) {
        match event {
            GrooveEvent::PlaybackStarted => {
                self.state = State::Playing;
            }
            GrooveEvent::PlaybackStopped => {
                self.state = State::Idle;
            }
            GrooveEvent::MidiToExternal(channel, message) => {
                self.post_to_midi_handler(MidiHandlerInput::Midi(channel, message));
            }
            GrooveEvent::ProjectLoaded(filename, title) => {
                self.preferences.last_project_filename = Some(filename);
                self.project_title = title;
                // self.entity_view.reset();
            }
            GrooveEvent::EntityMessage(uid, message) => match message {
                EntityMessage::ExpandPressed => {
                    // Find whoever else is expanded and maybe collapse them
                    // self.entity_view
                    //     .set_entity_view_state(uid, EntityViewState::Expanded);
                }
                EntityMessage::CollapsePressed => {
                    // self.entity_view
                    //     .set_entity_view_state(uid, EntityViewState::Collapsed);
                }
                EntityMessage::EnablePressed(_enabled) => {
                    // self.entity_view.set_entity_enabled_state(uid, enabled);
                }
                _ => {
                    self.update_entity(uid, message);
                }
            },
        }
    }

    fn handle_control_bar_event(&mut self, event: ControlBarEvent) -> Option<Command<AppMessage>> {
        match event {
            // TODO: try to get in the habit of putting less logic in the
            // user-action messages like Play/Stop, and putting more logic
            // in the system-action (react?) messages like most
            // EngineEvents. The engine is the model, and it's in charge of
            // consistency/consequences. If you spray logic in the view,
            // then it gets lost when the GUI evolves, and it's too tempting
            // to develop critical parts of the logic in the view.
            //
            // The Play logic is a good example: it's intricate, and there
            // isn't any good reason why the model wouldn't know how to
            // handle these actions properly.
            ControlBarEvent::Play => {
                self.start_or_pause_playback();
            }
            ControlBarEvent::Stop => {
                self.stop_playback();
            }
            ControlBarEvent::SkipToStart => self.skip_to_start(),
            ControlBarEvent::Bpm(value) => {
                if let Ok(bpm) = value.parse() {
                    self.orchestrator.set_bpm(bpm);
                }
            }
            ControlBarEvent::OpenProject => {
                return Some(Command::perform(
                    gui::persistence::open_dialog(),
                    AppMessage::OpenDialogComplete,
                ))
            }
            ControlBarEvent::ExportWav => {
                MessageDialog::new()
                    .set_type(MessageType::Info)
                    .set_title("Export WAV")
                    .set_text("Hold on a moment while we render the project!")
                    .show_alert()
                    .unwrap();
                let mut sample_buffer = [StereoSample::SILENCE; SAMPLE_BUFFER_SIZE];
                if let Ok(performance) = self.orchestrator.run_performance(&mut sample_buffer, true)
                {
                    return Some(Command::perform(
                        Preferences::export_to_wav(performance),
                        AppMessage::ExportComplete,
                    ));
                }
            }
            ControlBarEvent::ExportMp3 => {
                let mut sample_buffer = [StereoSample::SILENCE; SAMPLE_BUFFER_SIZE];
                if let Ok(performance) = self.orchestrator.run_performance(&mut sample_buffer, true)
                {
                    return Some(Command::perform(
                        Preferences::export_to_mp3(performance),
                        AppMessage::ExportComplete,
                    ));
                }
            }
            ControlBarEvent::ToggleSettings => self.show_settings = !self.show_settings,
        }
        None
    }

    fn handle_engine_event(&mut self, event: EngineEvent) -> Option<Command<AppMessage>> {
        match event {
            EngineEvent::Ready(sender, ring_buffer) => {
                self.engine_sender = Some(sender);
                self.ring_buffer = Some(ring_buffer);

                // We don't start the GrooveSubscription until prefs are
                // done loading, so this boolean and the corresponding
                // filename should be set by the time we look at it.
                if self.preferences.should_reload_last_project {
                    if let Some(last_project_filename) = &self.preferences.last_project_filename {
                        self.load_project(last_project_filename.clone().as_str());
                    }
                }
            }
            EngineEvent::Quit => {
                // Our EngineInput::QuitRequested has been handled. We have
                // nothing to do at this point.
                self.received_audio_quit = true;
            }
            EngineEvent::AudioBufferFullness(percentage) => {
                self.control_bar.set_audio_buffer_fullness(percentage);
            }
            EngineEvent::SampleRateChanged(sample_rate) => {
                self.orchestrator.reset(sample_rate);
                self.control_bar.set_sample_rate(sample_rate);
            }
            EngineEvent::GenerateAudio(buffer_count) => {
                self.generate_audio(buffer_count);
            }
        }
        None
    }

    fn load_project(&mut self, filename: &str) {
        let mut path = Paths::projects_path(PathType::Global);
        path.push(filename);
        if let Ok(settings) = SongSettings::new_from_yaml_file(path.to_str().unwrap()) {
            if let Ok(instance) = settings.instantiate(&Paths::assets_path(PathType::Global), false)
            {
                let title = instance.title();

                // Tell the app we've loaded the project
                self.handle_groove_event(GrooveEvent::ProjectLoaded(filename.to_string(), title));
                self.orchestrator = instance;
            }
        }
    }

    fn generate_audio(&mut self, buffer_count: u8) {
        let mut samples = [StereoSample::SILENCE; SAMPLE_BUFFER_SIZE];
        for i in 0..buffer_count {
            let is_last_iteration = i == buffer_count - 1;

            let (response, ticks_completed) = self.orchestrator.tick(&mut samples);
            if ticks_completed < samples.len() {
                self.stop_playback();
                self.reached_end_of_playback = true;
            }

            if let Some(ring_buffer) = &self.ring_buffer {
                for sample in samples {
                    let _ = ring_buffer.push(sample);
                }
            }
            match response.0 {
                groove_orchestration::messages::Internal::None => {}
                groove_orchestration::messages::Internal::Single(event) => {
                    self.handle_groove_event(event);
                }
                groove_orchestration::messages::Internal::Batch(events) => {
                    for event in events {
                        self.handle_groove_event(event)
                    }
                }
            }
            if is_last_iteration {
                // This clock is used to tell the app where we are in the song, so
                // even though it looks like it's not helping here in the loop, it's
                // necessary.
                self.update_control_bar_clock();
            }
        }
    }

    fn start_or_pause_playback(&mut self) {
        if matches!(self.state, State::Playing) {
            self.stop_playback();
        } else {
            if self.reached_end_of_playback {
                self.skip_to_start();
                self.reached_end_of_playback = false;
            }
            self.state = State::Playing;
            self.orchestrator.update(GrooveInput::Play);
        }
    }

    fn stop_playback(&mut self) {
        // This logic allows the user to press stop twice as shorthand for going
        // back to the start.
        if matches!(self.state, State::Playing) {
            self.state = State::Idle;
            self.orchestrator.update(GrooveInput::Stop);
        } else {
            self.skip_to_start();
        }
        self.update_control_bar_clock();
    }

    fn skip_to_start(&mut self) {
        self.orchestrator.update(GrooveInput::SkipToStart);
        self.update_control_bar_clock();
    }

    fn post_to_midi_handler(&mut self, input: MidiHandlerInput) {
        if let Some(sender) = self.midi_handler_sender.as_mut() {
            let i2 = input.clone(); // TODO: don't check this in
            if let Err(e) = sender.send(input) {
                eprintln!("sending failed... why? {:?} {:?}", i2, e);
            }
        }
    }

    fn post_to_engine(&mut self, input: EngineInput) {
        if let Some(sender) = self.engine_sender.as_mut() {
            // TODO: deal with this
            let _ = sender.send(input);
        }
    }

    fn update_entity(&mut self, _uid: usize, message: EntityMessage) {
        match message {
            EntityMessage::Midi(_, _) => todo!(),
            EntityMessage::ControlF32(_) => todo!(),
            EntityMessage::PatternMessage(_, _) => todo!(),
            EntityMessage::PickListSelected(_) => todo!(),
            EntityMessage::ExpandPressed => todo!(),
            EntityMessage::CollapsePressed => todo!(),
            EntityMessage::EnablePressed(_) => todo!(),
        }
    }

    // fn orchestrator_view(&self) -> Element<GrooveEvent> {
    //     // if let Ok(orchestrator) = self.orchestrator.lock() {
    //     // let canvas: Element<'_, GrooveEvent, Renderer<<GrooveApp as Application>::Theme>> =
    //     //     Canvas::new(&self.gui_state)
    //     //         .width(Length::Fill)
    //     //         .height(Length::Fixed((32 * 4) as f32))
    //     //         .into();

    //     let mut views = self.views.view()orchestrator
    //         .entity_iter()
    //         .fold(Vec::new(), |mut v, (&uid, e)| {
    //             v.push(
    //                 self.entity_view
    //                     .view(e)
    //                     .map(move |message| GrooveEvent::EntityMessage(uid, message)),
    //             );
    //             v
    //         });
    //     //            views.push(canvas);
    //     column(views).into()
    //     // } else {
    //     //     panic!()
    //     // }
    // }

    // fn orchestrator_new_view(&self) -> Element<GrooveEvent> {
    //     if let Ok(_orchestrator) = self.orchestrator.lock() {
    //         let canvas: Element<'_, GrooveEvent, Renderer<<GrooveApp as Application>::Theme>> =
    //             Canvas::new(&self.gui_state)
    //                 .width(Length::Fill)
    //                 .height(Length::Fixed((32 * 4) as f32))
    //                 .into();
    //         canvas.into()
    //     } else {
    //         panic!()
    //     }
    // }

    // TODO: this should be another entity
    fn midi_view(&self) -> Element<MidiHandlerInput> {
        let activity_text = container(GuiStuff::<EntityMessage>::container_text(
            if Instant::now().duration_since(self.last_midi_activity) > Duration::from_millis(250) {
                " "
            } else {
                "•"
            },
        ))
        .width(iced::Length::FillPortion(1));
        let input_menu = row![
            GuiStuff::<EntityMessage>::container_text("Input").width(iced::Length::FillPortion(1)),
            pick_list(
                &self.midi_input_ports,
                self.midi_input_port_active.clone(),
                MidiHandlerInput::SelectMidiInput,
            )
            .font(gui::SMALL_FONT)
            .width(iced::Length::FillPortion(3))
        ];

        let output_menu = row![
            GuiStuff::<EntityMessage>::container_text("Output").width(iced::Length::FillPortion(1)),
            pick_list(
                &self.midi_output_ports,
                self.midi_output_port_active.clone(),
                MidiHandlerInput::SelectMidiOutput,
            )
            .font(gui::SMALL_FONT)
            .width(iced::Length::FillPortion(3))
        ];
        let port_menus =
            container(column![input_menu, output_menu]).width(iced::Length::FillPortion(7));
        GuiStuff::titled_container("MIDI", container(row![activity_text, port_menus]).into())
    }

    fn handle_close_requested_event(&mut self) -> Option<Command<AppMessage>> {
        // See https://github.com/iced-rs/iced/pull/804 and
        // https://github.com/iced-rs/iced/blob/master/examples/events/src/main.rs#L55
        //
        // This is needed to stop an ALSA buffer underrun on close
        self.post_to_midi_handler(MidiHandlerInput::QuitRequested);
        self.post_to_engine(EngineInput::QuitRequested);

        // Let the PrefsSaved message handler know that it's time to go.
        self.should_exit = true;

        Some(Command::perform(
            Preferences::save_prefs(Preferences {
                selected_midi_input: self.preferences.selected_midi_input.clone(),
                selected_midi_output: self.preferences.selected_midi_output.clone(),
                should_reload_last_project: self.preferences.should_reload_last_project,
                last_project_filename: self.preferences.last_project_filename.clone(),
            }),
            AppMessage::PrefsSaved,
        ))
    }

    fn handle_keyboard_event(
        &mut self,
        event: iced::keyboard::Event,
    ) -> Option<Command<AppMessage>> {
        // This recently changed, and I don't get KeyPressed anymore. Maybe this
        // is a new event that processes KeyPressed/KeyReleased, so they're no
        // longer "ignored runtime events."
        if let iced::keyboard::Event::CharacterReceived(char) = event {
            if char == '\t' {
                // TODO: I don't know if this is smart. Are there better
                // patterns than calling update()?
                self.views
                    .update(&mut self.orchestrator, ViewMessage::NextView);
            }
        }
        None
    }

    // fn next_view(view: MainViews) -> MainViews {
    //     match FromPrimitive::from_u8(view as u8 + 1) {
    //         Some(view) => view,
    //         None => FromPrimitive::from_u8(0).unwrap(),
    //     }
    // }

    // fn switch_main_view(&mut self) {
    //     self.current_view = Self::next_view(self.current_view)
    // }

    // fn main_view(&self) -> Element<AppMessage> {
    //     match self.current_view {
    //         MainViews::Unstructured => {
    //             let midi_view: Element<AppMessage> =
    //                 self.midi_view().map(AppMessage::MidiHandlerInput);
    //             let project_view: Element<AppMessage> =
    //                 self.orchestrator_view().map(AppMessage::GrooveEvent);
    //             let scrollable_content = column![midi_view, project_view];
    //             let scrollable =
    //                 container(scrollable(scrollable_content)).width(Length::FillPortion(1));
    //             row![Self::under_construction("Unstructured"), scrollable].into()
    //         }
    //         MainViews::New => {
    //             let project_view: Element<AppMessage> =
    //                 self.orchestrator_new_view().map(AppMessage::GrooveEvent);
    //             scrollable(project_view).into()
    //         }
    //         MainViews::Session => Self::under_construction("Session").into(),
    //         MainViews::Arrangement => Self::under_construction("Arrangement").into(),
    //         MainViews::Preferences => Self::under_construction("Preferences").into(),
    //         MainViews::AudioLanes | MainViews::Automation => self
    //             .views
    //             .view()
    //             .map(move |m| AppMessage::MainViewThingyMessage(m)),
    //     }
    // }

    fn handle_midi_handler_event(&mut self, event: MidiHandlerEvent) {
        match event {
            MidiHandlerEvent::Ready(sender) => {
                self.midi_handler_sender = Some(sender);
            }
            MidiHandlerEvent::Midi(channel, message) => {
                self.last_midi_activity = Instant::now();
                self.orchestrator
                    .update(GrooveInput::MidiFromExternal(channel, message));
            }
            MidiHandlerEvent::Quit => {
                // TODO: If we were waiting for this to shut down, then
                // record that we're ready. For now, it's nice to know, but
                // we won't do anything about it.
                self.received_midi_quit = true;
            }
            MidiHandlerEvent::InputPorts(ports) => {
                self.midi_input_ports = ports;
                if self.midi_input_port_active.is_none() {
                    if let Some(selected) = &self.preferences.selected_midi_input {
                        if let Some(sender) = self.midi_handler_sender.as_mut() {
                            for descriptor in &self.midi_input_ports {
                                if selected == descriptor.name() {
                                    let _ = sender.send(MidiHandlerInput::SelectMidiInput(
                                        descriptor.clone(),
                                    ));
                                    break;
                                }
                            }
                        }
                    }
                    self.preferences.selected_midi_input = None; // to prevent loops
                }
            }
            MidiHandlerEvent::InputPortSelected(port) => {
                if let Some(port) = &port {
                    self.preferences.selected_midi_input = Some(port.name().to_string());
                } else {
                    self.preferences.selected_midi_input = None;
                }
                self.midi_input_port_active = port;
            }
            MidiHandlerEvent::OutputPorts(ports) => {
                self.midi_output_ports = ports;
                if self.midi_output_port_active.is_none() {
                    if let Some(selected) = &self.preferences.selected_midi_output {
                        if let Some(sender) = self.midi_handler_sender.as_mut() {
                            for descriptor in &self.midi_output_ports {
                                if selected == descriptor.name() {
                                    let _ = sender.send(MidiHandlerInput::SelectMidiOutput(
                                        descriptor.clone(),
                                    ));
                                    break;
                                }
                            }
                        }
                    }
                    self.preferences.selected_midi_output = None; // to prevent loops
                }
            }
            MidiHandlerEvent::OutputPortSelected(port) => {
                if let Some(port) = &port {
                    self.preferences.selected_midi_output = Some(port.name().to_string());
                } else {
                    self.preferences.selected_midi_output = None;
                }
                self.midi_output_port_active = port;
            }
        }
    }

    fn handle_midi_handler_input(&mut self, message: MidiHandlerInput) {
        match message {
            MidiHandlerInput::SelectMidiInput(which) => {
                self.post_to_midi_handler(MidiHandlerInput::SelectMidiInput(which));
            }
            MidiHandlerInput::SelectMidiOutput(which) => {
                self.post_to_midi_handler(MidiHandlerInput::SelectMidiOutput(which));
            }
            _ => panic!("Remaining MidiHandlerInput messages should be handled internally"),
        }
    }

    fn handle_system_event(&mut self, event: Event) -> Option<Command<AppMessage>> {
        if let Event::Window(window::Event::CloseRequested) = event {
            return self.handle_close_requested_event();
        }
        if let Event::Keyboard(e) = event {
            return self.handle_keyboard_event(e);
        }
        None
    }

    fn update_control_bar_clock(&mut self) {
        self.control_bar
            .set_clock(self.orchestrator.clock().frames());
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
#[allow(dead_code)]
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
    #[allow(dead_code)]
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
