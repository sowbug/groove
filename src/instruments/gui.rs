use super::{
    drumkit_sampler::DrumkitSampler, envelopes::AdsrEnvelope, sampler::Sampler, welsh::WelshSynth,
};
use crate::{
    gui::{GuiStuff, Viewable},
    messages::EntityMessage,
};
use iced::Element;
use std::any::type_name;

impl Viewable for Sampler {
    type ViewMessage = EntityMessage;

    fn view(&self) -> Element<Self::ViewMessage> {
        let title = type_name::<Sampler>();
        let contents = format!("name: {}", self.filename);
        GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
    }
}

impl Viewable for DrumkitSampler {
    type ViewMessage = EntityMessage;

    fn view(&self) -> Element<Self::ViewMessage> {
        let title = type_name::<DrumkitSampler>();
        let contents = format!("kit name: {}", self.kit_name);
        GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
    }
}

impl Viewable for WelshSynth {
    type ViewMessage = EntityMessage;

    fn view(&self) -> Element<Self::ViewMessage> {
        let title = type_name::<WelshSynth>();
        let contents = format!("name: {}", self.preset.name);
        GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
    }
}

impl Viewable for AdsrEnvelope {
    type ViewMessage = EntityMessage;
}
