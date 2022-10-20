use std::{any::type_name, fmt::Debug, rc::Weak};

use iced::{container, Container, Element, Text};

use crate::{
    common::Ww,
    effects::{gain::Gain, mixer::Mixer},
    synthesizers::{drumkit_sampler::Sampler as DrumkitSampler, sampler::Sampler, welsh::Synth},
    traits::{MakesIsViewable, SinksAudio},
};

#[derive(Clone, Debug)]
pub enum GrooveMessage {
    Null,
    Something,
    GainMessage(GainMessage),
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

#[derive(Debug)]
pub struct GainViewableResponder {
    target: Ww<Gain>,
}
impl IsViewable for GainViewableResponder {
    fn view(&mut self) -> Element<GrooveMessage> {
        if let Some(target) = self.target.upgrade() {
            Container::new(
                Text::new(format!("level: {}", target.borrow().level()))
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
        match message {
            GrooveMessage::GainMessage(message) => match message {
                GainMessage::Level(level) => {
                    if let Some(target) = self.target.upgrade() {
                        if let Ok(level) = level.parse() {
                            target.borrow_mut().set_level(level);
                        }
                    }
                }
            },
            _ => {}
        }
    }
}

#[derive(Clone, Debug)]
pub enum GainMessage {
    Level(String),
}
impl MakesIsViewable for Gain {
    fn make_is_viewable(&self) -> Option<Box<dyn IsViewable>> {
        if self.me.strong_count() != 0 {
            Some(Box::new(GainViewableResponder {
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

#[cfg(test)]
mod tests {
    use crate::{
        common::Rrc,
        effects::{gain::Gain, mixer::Mixer},
        synthesizers::{
            drumkit_sampler::Sampler as DrumkitSampler,
            sampler::Sampler,
            welsh::{PresetName, Synth, SynthPreset},
        },
        traits::MakesIsViewable,
    };

    use super::GrooveMessage;

    // There aren't many assertions in this method, but we know it'll panic or spit out debug
    // messages if something's wrong.
    fn test_one_viewable(factory: Rrc<dyn MakesIsViewable>, message: Option<GrooveMessage>) {
        let is_viewable = factory.borrow_mut().make_is_viewable();
        if let Some(mut viewable) = is_viewable {
            let _ = viewable.view();
            if let Some(message) = message {
                viewable.update(message);
            }
        } else {
            assert!(false, "factory failed {:?}", factory);
        }
    }

    #[test]
    fn test_viewables() {
        test_one_viewable(
            Synth::new_wrapped_with(0, 44100, SynthPreset::by_name(&PresetName::Trombone)),
            None,
        );
        test_one_viewable(DrumkitSampler::new_wrapped_from_files(0), None);
        test_one_viewable(Sampler::new_wrapped_with(0, 1024), None);
        test_one_viewable(Mixer::new_wrapped(), None);
        test_one_viewable(
            Gain::new_wrapped(),
            Some(GrooveMessage::GainMessage(super::GainMessage::Level(
                "0.5".to_string(),
            ))),
        );
    }
}
