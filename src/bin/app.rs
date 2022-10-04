#![feature(trait_upcasting)]
#![allow(incomplete_features)]

use iced::button::{self, Button};
use iced::{Alignment, Column, Element, Sandbox, Settings, Text};
use libgroove::devices::orchestrator::Orchestrator;
use libgroove::helpers::IOHelper;
use libgroove::settings::song::SongSettings;

pub fn main() -> iced::Result {
    Groove::run(Settings::default())
}

#[derive(Default)]
struct Groove {
    filename: String,
    orchestrator: Orchestrator,
    play_button: button::State,
    stop_button: button::State,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    PlayPressed,
    StopPressed,
}

impl Groove {}

impl Sandbox for Groove {
    type Message = Message;

    fn new() -> Self {
        let filename = "scripts/everything.yaml";
        let yaml = std::fs::read_to_string(filename).unwrap();
        let song_settings = SongSettings::new_from_yaml(yaml.as_str());
        Self {
            filename: filename.to_string(),
            orchestrator: Orchestrator::new(song_settings.unwrap()),
            ..Default::default()
        }
    }

    fn title(&self) -> String {
        self.filename.clone()
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::PlayPressed => {
                self.orchestrator = Orchestrator::new(self.orchestrator.settings().clone());
                let performance = self.orchestrator.perform();
                if performance.is_ok() {
                    let result = IOHelper::send_performance_to_output_device(performance.unwrap());
                    if result.is_ok() {
                        // great
                    }
                }
            }
            Message::StopPressed => {
                todo!();
            }
        }
    }

    fn view(&mut self) -> Element<Message> {
        Column::new()
            .padding(20)
            .align_items(Alignment::Center)
            .push(
                Button::new(&mut self.play_button, Text::new("Play"))
                    .on_press(Message::PlayPressed),
            )
            .push(Text::new(self.orchestrator.settings().clock.bpm().to_string()).size(50))
            .push(
                Button::new(&mut self.stop_button, Text::new("Stop"))
                    .on_press(Message::StopPressed),
            )
            .into()
    }
}
