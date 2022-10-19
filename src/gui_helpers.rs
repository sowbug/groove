use std::{fmt::Debug, rc::Weak};

use iced::{container, Container, Element, Text};

use crate::{
    common::Ww,
    effects::mixer::Mixer,
    traits::{MakesIsViewable, SinksAudio},
};

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

    fn update(&mut self, _message: GrooveMessage) {
        dbg!(_message);
    }
}

#[derive(Debug)]
pub struct MixerIcedResponder {
    target: Ww<Mixer>,
}
impl IsViewable for MixerIcedResponder {
    fn view(&mut self) -> Element<GrooveMessage> {
        if let Some(target) = self.target.upgrade() {
            Container::new(
                Text::new(format!("sources: {}", target.borrow().sources().len()))
                    .horizontal_alignment(iced::alignment::Horizontal::Center)
                    .vertical_alignment(iced::alignment::Vertical::Center),
            )
            .padding(4)
            .style(BorderedContainer::default())
            .into()
        } else {
            panic!()
        }
    }

    fn update(&mut self, message: GrooveMessage) {
        dbg!(message);
    }
}

impl MakesIsViewable for Mixer {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(MixerIcedResponder {
                target: Weak::clone(&self.me),
            }))
        } else {
            None
        }
    }
}
