use std::fmt::Debug;

use iced::{container, Container, Element, Text};

use crate::effects::mixer::Mixer;

#[derive(Debug, Clone)]
pub enum GrooveMessage {
    Null,
    Something,
}

pub trait IsViewable: Debug {
    fn view(&mut self) -> Element<GrooveMessage> {
        Container::new(
            Text::new("under construction".clone())
                .horizontal_alignment(iced::alignment::Horizontal::Center)
                .vertical_alignment(iced::alignment::Vertical::Center),
        )
        .padding(4)
        .style(BorderedContainer::default())
        .into()
    }
    fn get_string(&mut self) -> String {
        "trait".to_string()
    }
    fn update(&mut self, message: GrooveMessage) {
        dbg!(message);
    }
}

#[derive(Default)]
pub struct BorderedContainer {}

impl container::StyleSheet for BorderedContainer {
    fn style(&self) -> container::Style {
        container::Style {
            border_color: iced::Color::BLACK,
            border_width: 1.0,
            ..Default::default()
        }
    }
}

impl IsViewable for Mixer {
    fn view(&mut self) -> Element<GrooveMessage> {
        Container::new(
            Text::new("under construction")
                .horizontal_alignment(iced::alignment::Horizontal::Center)
                .vertical_alignment(iced::alignment::Vertical::Center),
        )
        .padding(4)
        .style(BorderedContainer::default())
        .into()
    }

    fn get_string(&mut self) -> String {
        "mixer".to_string()
    }

    fn update(&mut self, _message: GrooveMessage) {
        dbg!(_message);
    }
}
