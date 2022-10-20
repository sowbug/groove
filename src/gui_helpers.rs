use std::{any::type_name, fmt::Debug, rc::Weak};

use iced::{container, Container, Element, Text};

use crate::{
    common::Ww,
    effects::mixer::Mixer,
    synthesizers::{drumkit_sampler::Sampler as DrumkitSampler, sampler::Sampler, welsh::Synth},
    traits::{MakesIsViewable, SinksAudio},
};

#[derive(Debug, Clone)]
pub enum GrooveMessage {
    Null,
    Something,
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

pub trait IsViewable: Debug {
    fn view(&mut self) -> Element<GrooveMessage> {
        Container::new(
            Text::new(format!("{}: under construction", self.name()))
                .horizontal_alignment(iced::alignment::Horizontal::Center)
                .vertical_alignment(iced::alignment::Vertical::Center),
        )
        .padding(4)
        .style(BorderedContainer::default())
        .into()
    }

    fn name(&mut self) -> String {
        type_name::<Self>().to_string()
    }

    fn update(&mut self, message: GrooveMessage) {
        dbg!(message);
    }
}

#[derive(Debug)]
pub struct MixerViewableResponder {
    target: Ww<Mixer>,
}
impl IsViewable for MixerViewableResponder {
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
            Some(Box::new(MixerViewableResponder {
                target: Weak::clone(&self.me),
            }))
        } else {
            println!(
                "{}: probably forgot to call new_wrapped...()",
                type_name::<Self>()
            );
            None
        }
    }
}

#[derive(Debug)]
pub struct SamplerViewableResponder {
    target: Ww<Sampler>,
}
impl IsViewable for SamplerViewableResponder {
    fn view(&mut self) -> Element<GrooveMessage> {
        if let Some(target) = self.target.upgrade() {
            Container::new(
                Text::new(format!("name: {}", target.borrow().filename))
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

impl MakesIsViewable for Sampler {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(SamplerViewableResponder {
                target: Weak::clone(&self.me),
            }))
        } else {
            println!(
                "{}: probably forgot to call new_wrapped...()",
                type_name::<Self>()
            );
            None
        }
    }
}

#[derive(Debug)]
pub struct DrumkitSamplerViewableResponder {
    target: Ww<DrumkitSampler>,
}
impl IsViewable for DrumkitSamplerViewableResponder {
    fn view(&mut self) -> Element<GrooveMessage> {
        if let Some(target) = self.target.upgrade() {
            Container::new(
                Text::new(format!("kit name: {}", target.borrow().kit_name))
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

impl MakesIsViewable for DrumkitSampler {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(DrumkitSamplerViewableResponder {
                target: Weak::clone(&self.me),
            }))
        } else {
            println!(
                "{}: probably forgot to call new_wrapped...()",
                type_name::<Self>()
            );
            None
        }
    }
}

#[derive(Debug)]
pub struct SynthViewableResponder {
    target: Ww<Synth>,
}
impl IsViewable for SynthViewableResponder {
    fn view(&mut self) -> Element<GrooveMessage> {
        if let Some(target) = self.target.upgrade() {
            Container::new(
                Text::new(format!("name: {}", target.borrow().preset.name))
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

impl MakesIsViewable for Synth {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(SynthViewableResponder {
                target: Weak::clone(&self.me),
            }))
        } else {
            println!(
                "{}: probably forgot to call new_wrapped...()",
                type_name::<Self>()
            );
            None
        }
    }
}
