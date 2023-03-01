use super::{MidiHandler, MidiHandlerMessage, MidiPortLabel};
use crate::{clock::Clock, traits::Response};
use groove_core::midi::{MidiChannel, MidiMessage};
use iced::{futures::channel::mpsc, subscription, Subscription};
use std::{
    sync::{Arc, Mutex},
    thread::JoinHandle,
    time::Duration,
};

#[derive(Clone, Debug)]
pub enum MidiHandlerInput {
    /// The user has picked a MIDI input. Switch to it.
    SelectMidiInput(MidiPortLabel),

    /// The user has picked a MIDI output. Switch to it.
    SelectMidiOutput(MidiPortLabel),

    /// We've been asked to send a MIDI message to external hardware.
    Midi(MidiChannel, MidiMessage),

    /// This subscription thread should end.
    QuitRequested,
}

#[derive(Clone, Debug)]
pub enum MidiHandlerEvent {
    /// Our subscription thread has successfully started. Here is what the app
    /// will need to communicate with it.
    Ready(mpsc::Sender<MidiHandlerInput>, Arc<Mutex<MidiHandler>>),

    /// A MIDI message has arrived from external hardware. Handle it in the app.
    Midi(MidiChannel, MidiMessage),

    /// We have successfully processed MidiHandlerInput::QuitRequested.
    Quit,
}

enum State {
    Start,
    Ready(JoinHandle<()>, mpsc::Receiver<MidiHandlerEvent>),
    Ending(JoinHandle<()>),
    Idle,
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

                        let mut t = MidiHandler::default();
                        let _ = t.start();
                        let midi_handler = Arc::new(Mutex::new(t));
                        let midi_handler_for_app = Arc::clone(&midi_handler);
                        let handler = std::thread::spawn(move || {
                            let mut runner =
                                Runner::new_with(midi_handler, thread_sender, app_receiver);
                            runner.do_loop();
                        });

                        (
                            Some(MidiHandlerEvent::Ready(app_sender, midi_handler_for_app)),
                            State::Ready(handler, thread_receiver),
                        )
                    }
                    State::Ready(handler, mut receiver) => {
                        use iced_native::futures::StreamExt;

                        let event = receiver.select_next_some().await;
                        let mut done = false;
                        match event {
                            MidiHandlerEvent::Ready(_, _) => todo!(),
                            MidiHandlerEvent::Midi(_, _) => {}
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
                        let _ = handler.join();
                        // See https://github.com/iced-rs/iced/issues/1348
                        (None, State::Idle)
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

struct Runner {
    midi_handler: Arc<Mutex<MidiHandler>>,
    sender: mpsc::Sender<MidiHandlerEvent>,
    receiver: mpsc::Receiver<MidiHandlerInput>,
    events: Vec<MidiHandlerEvent>,
}
impl Runner {
    fn new_with(
        midi_handler: Arc<Mutex<MidiHandler>>,
        thread_sender: mpsc::Sender<MidiHandlerEvent>,
        app_receiver: mpsc::Receiver<MidiHandlerInput>,
    ) -> Self {
        Self {
            midi_handler,
            sender: thread_sender,
            receiver: app_receiver,
            events: Default::default(),
        }
    }

    fn do_loop(&mut self) {
        let clock = Clock::default();
        loop {
            let response = if let Ok(mut midi_handler) = self.midi_handler.lock() {
                midi_handler.update(&clock, MidiHandlerMessage::Tick)
            } else {
                Response::none()
            };
            self.push_response(response);
            self.send_pending_messages();
            if let Ok(Some(input)) = self.receiver.try_next() {
                match input {
                    MidiHandlerInput::Midi(channel, message) => {
                        if let Ok(mut midi_handler) = self.midi_handler.lock() {
                            midi_handler.update(&clock, MidiHandlerMessage::Midi(channel, message));
                        }
                    }
                    MidiHandlerInput::QuitRequested => {
                        self.push_response(Response::single(MidiHandlerEvent::Quit));
                        self.send_pending_messages();
                        break;
                    }
                    MidiHandlerInput::SelectMidiInput(which) => {
                        if let Ok(mut midi_handler) = self.midi_handler.lock() {
                            midi_handler.select_input(which);
                        }
                    }
                    MidiHandlerInput::SelectMidiOutput(which) => {
                        if let Ok(mut midi_handler) = self.midi_handler.lock() {
                            midi_handler.select_output(which);
                        }
                    }
                }
            }

            // TODO: convert this to select on either self.receiver or midi input
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn push_response(&mut self, response: Response<MidiHandlerEvent>) {
        match response.0 {
            crate::traits::Internal::None => {}
            crate::traits::Internal::Single(message) => {
                self.events.push(message);
            }
            crate::traits::Internal::Batch(messages) => {
                self.events.extend(messages);
            }
        }
    }

    fn send_pending_messages(&mut self) {
        while let Some(message) = self.events.pop() {
            let _ = self.sender.try_send(message);
        }
    }
}
