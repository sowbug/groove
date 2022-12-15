use super::{
    patterns::{Note, Pattern, PatternManager},
    MidiChannel, MidiOutputHandler,
};
use crate::{
    gui::{GuiStuff, Viewable},
    messages::EntityMessage,
    MidiHandler, MidiHandlerMessage,
};
use iced::{
    futures::channel::mpsc,
    subscription,
    widget::{button, column, container, pick_list, row, text},
    Element, Subscription,
};
use midly::MidiMessage;
use std::{
    any::type_name,
    thread::JoinHandle,
    time::{Duration, Instant},
};

impl Viewable for MidiOutputHandler {
    type ViewMessage = MidiHandlerMessage;
}
impl Viewable for MidiHandler {
    type ViewMessage = MidiHandlerMessage;

    fn view(&self) -> Element<'_, Self::ViewMessage, iced::Renderer> {
        let activity_text = container(text(
            if Instant::now().duration_since(self.activity_tick) > Duration::from_millis(250) {
                " "
            } else {
                "•"
            },
        ))
        .width(iced::Length::FillPortion(1));
        let (input_selected, input_options) = self.midi_input.as_ref().unwrap().labels();
        let input_menu = row![
            text("Input").width(iced::Length::FillPortion(1)),
            pick_list(
                input_options,
                input_selected.clone(),
                MidiHandlerMessage::InputSelected,
            )
            .width(iced::Length::FillPortion(3))
        ];
        let (output_selected, output_options) = self.midi_output.as_ref().unwrap().labels();
        let output_menu = row![
            text("Output").width(iced::Length::FillPortion(1)),
            pick_list(
                output_options,
                output_selected.clone(),
                MidiHandlerMessage::OutputSelected,
            )
            .width(iced::Length::FillPortion(3))
        ];
        let port_menus =
            container(column![input_menu, output_menu]).width(iced::Length::FillPortion(7));
        GuiStuff::titled_container("MIDI", container(row![activity_text, port_menus]).into())
    }
}

#[derive(Clone, Debug)]
pub enum PatternMessage {
    SomethingHappened,
    ButtonPressed,
}

impl Pattern<Note> {
    fn view<'a>(&self) -> Element<'a, PatternMessage> {
        let mut note_rows = Vec::new();
        for track in self.notes.iter() {
            let mut note_row = Vec::new();
            for note in track {
                let cell = text(format!("{:02} ", note.key).to_string());
                note_row.push(cell.into());
            }
            let row_note_row = row(note_row).into();
            note_rows.push(row_note_row);
        }
        column(vec![
            button(text(format!("{:?}", self.note_value)))
                .on_press(PatternMessage::ButtonPressed)
                .into(),
            column(note_rows).into(),
        ])
        .into()
    }
}

impl Viewable for PatternManager {
    type ViewMessage = EntityMessage;

    fn view(&self) -> Element<Self::ViewMessage> {
        let title = type_name::<PatternManager>();
        let contents = {
            let pattern_views = self.patterns().iter().enumerate().map(|(i, item)| {
                item.view()
                    .map(move |message| Self::ViewMessage::PatternMessage(i, message))
            });
            column(pattern_views.collect())
        };
        GuiStuff::titled_container(title, contents.into())
    }
}

enum State {
    Start,
    Ready(JoinHandle<()>, mpsc::Receiver<MidiHandlerEvent>),
    Ending(JoinHandle<()>),
    Idle,
}

#[derive(Clone, Debug)]
pub enum MidiHandlerInput {
    ChangeTheChannel,
    MidiMessage(MidiChannel, MidiMessage),
    QuitRequested,
}

#[derive(Clone, Debug)]
pub enum MidiHandlerEvent {
    Ready(mpsc::Sender<MidiHandlerInput>),
    MidiMessage(MidiChannel, MidiMessage),
    Quit,
}

pub struct MidiSubscription {}
impl MidiSubscription {
    pub fn subscription() -> Subscription<MidiHandlerEvent> {
        subscription::unfold(
            std::any::TypeId::of::<MidiSubscription>(),
            State::Start,
            |state| async move {
                match state {
                    State::Start => {
                        let (app_sender, app_receiver) = mpsc::channel::<MidiHandlerInput>(1024);
                        let (thread_sender, thread_receiver) =
                            mpsc::channel::<MidiHandlerEvent>(1024);

                        let handler = std::thread::spawn(move || {
                            let mut midi_handler =
                                MidiHandler::new_with(thread_sender, app_receiver);
                            midi_handler.do_loop();
                        });

                        (
                            Some(MidiHandlerEvent::Ready(app_sender)),
                            State::Ready(handler, thread_receiver),
                        )
                    }
                    State::Ready(handler, mut receiver) => {
                        use iced_native::futures::StreamExt;

                        let event = receiver.select_next_some().await;
                        let mut done = false;
                        match event {
                            MidiHandlerEvent::Ready(_) => todo!(),
                            MidiHandlerEvent::MidiMessage(_, _) => {}
                            MidiHandlerEvent::Quit => {
                                done = true;
                            }
                        }

                        (
                            Some(event),
                            if done {
                                State::Ending(handler)
                            } else {
                                State::Ready(handler, receiver)
                            },
                        )
                    }
                    State::Ending(handler) => {
                        if let Ok(_) = handler.join() {
                            println!("Subscription handler.join()");
                        }
                        // See https://github.com/iced-rs/iced/issues/1348
                        return (None, State::Idle);
                    }
                    State::Idle => {
                        // I took this line from
                        // https://github.com/iced-rs/iced/issues/336, but I
                        // don't understand why it helps. I think it's necessary
                        // for the system to get a chance to process all the
                        // subscription results.
                        let _: () = iced::futures::future::pending().await;
                        (None, State::Idle)
                    }
                }
            },
        )
    }
}
