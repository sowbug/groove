pub mod style;

use iced::{alignment, button, Alignment, Button, Checkbox, Element, Font, Length, Row, Text};

#[derive(Debug, Clone)]
pub enum AudioSourceMessage {
    SomethingHappened,
    IsActive(bool),
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
    pub fn update(&mut self, message: AudioSourceMessage) {
        match message {
            AudioSourceMessage::SomethingHappened => {
                println!("AudioSourceMessage::SomethingHappened");
            }
            AudioSourceMessage::IsActive(is_active) => {
                println!("{:?} {:?}", message, self);
                self.is_active = is_active;
            }
        }
    }

    pub fn view(&mut self) -> Element<AudioSourceMessage> {
        match &mut self.state {
            AudioSourceState::Idle { button } => {
                let checkbox =
                    Checkbox::new(self.is_active, &self.name, AudioSourceMessage::IsActive)
                        .width(Length::Fill);

                Row::new()
                    .spacing(20)
                    .align_items(Alignment::Center)
                    .push(checkbox)
                    .push(
                        Button::new(button, edit_icon())
                            .on_press(AudioSourceMessage::SomethingHappened)
                            .padding(10)
                            .style(style::Button::Icon),
                    )
                    .into()
            }
        }
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
