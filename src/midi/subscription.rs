use super::MidiChannel;
use crate::MidiHandler;
use iced::{futures::channel::mpsc, subscription, Subscription};
use midly::MidiMessage;
use std::thread::JoinHandle;

#[derive(Clone, Debug)]
pub enum PatternMessage {
    SomethingHappened,
    ButtonPressed,
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
                        let _ = handler.join();
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
