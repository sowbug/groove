pub mod message;
pub mod persistence;
pub mod style;
pub mod to_be_obsolete;

use crate::traits::SourcesAudio;
use iced::{alignment, button, Alignment, Button, Checkbox, Element, Font, Length, Row, Text};
use std::cell::RefCell;
use std::rc::Rc;

use self::message::Message;

#[derive(Debug, Clone)]
pub enum AudioSourceMessage {
    EditButtonPressed,
    IsMuted(bool),
}

#[derive(Debug, Clone)]
pub enum AudioSourceState {
    Idle { button: button::State },
}

impl Default for AudioSourceState {
    fn default() -> Self {
        AudioSourceState::Idle {
            button: button::State::new(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct AudioSource {
    pub name: String,
    pub is_active: bool,
    pub state: AudioSourceState,
}

impl AudioSource {
    pub fn instantiate(source: Rc<RefCell<dyn SourcesAudio>>) -> Self {
        Self {
            name: source.borrow().name().to_string(),
            ..Default::default()
        }
    }

    pub fn update(&mut self, message: AudioSourceMessage) {
        match message {
            AudioSourceMessage::EditButtonPressed => {
                println!("AudioSourceMessage::SomethingHappened");
            }
            AudioSourceMessage::IsMuted(is_active) => {
                println!("{:?} {:?}", message, self);
                self.is_active = is_active;
            }
        }
    }

    pub fn view(&mut self) -> Element<AudioSourceMessage> {
        match &mut self.state {
            AudioSourceState::Idle { button } => {
                let checkbox =
                    Checkbox::new(self.is_active, &self.name, AudioSourceMessage::IsMuted)
                        .width(Length::Fill);

                Row::new()
                    .spacing(20)
                    .align_items(Alignment::Center)
                    .push(checkbox)
                    .push(
                        Button::new(button, edit_icon())
                            .on_press(AudioSourceMessage::EditButtonPressed)
                            .padding(10)
                            .style(style::Button::Icon),
                    )
                    .into()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ControlBarMessage {
    Play,
    Stop,
}

#[derive(Debug, Default, Clone)]
pub struct ControlBar {
    play_button: button::State,
    stop_button: button::State,
}

impl ControlBar {
    pub fn update(&mut self) {}

    pub fn view(&mut self) -> Row<Message> {
        Row::new()
            .spacing(20)
            .align_items(Alignment::Center)
            .push(
                Text::new(format!("{} {} left", 3, "plumbuses"))
                    .width(Length::Fill)
                    .size(16),
            )
            .push(
                Row::new()
                    .width(Length::Shrink)
                    .spacing(10)
                    .push(
                        Button::new(&mut self.play_button, Text::new("play"))
                            .on_press(Message::ControlBarMessage(ControlBarMessage::Play)),
                    )
                    .push(
                        Button::new(&mut self.stop_button, Text::new("stop"))
                            .on_press(Message::ControlBarMessage(ControlBarMessage::Stop)),
                    )
                    .push(Text::new("everyone")),
            )
    }
}

// Fonts
const ICONS: Font = Font::External {
    name: "Icons",
    bytes: include_bytes!("../../fonts/icons.ttf"),
};

fn icon(unicode: char) -> Text {
    Text::new(unicode.to_string())
        .font(ICONS)
        .width(Length::Units(20))
        .horizontal_alignment(alignment::Horizontal::Center)
        .size(20)
}

pub fn edit_icon() -> Text {
    icon('\u{F303}')
}

pub fn delete_icon() -> Text {
    icon('\u{F1F8}')
}
