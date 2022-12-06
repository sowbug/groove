use super::{
    arpeggiator::Arpeggiator,
    sequencers::{BeatSequencer, MidiTickSequencer},
    ControlTrip,
};
use crate::{
    gui::{GuiStuff, Viewable},
    messages::{EntityMessage, GrooveMessage, MessageBounds},
    Orchestrator,
};
use iced::{
    widget::{column, container, text},
    Element,
};
use std::any::type_name;

impl Viewable for Arpeggiator {
    type ViewMessage = EntityMessage;

    fn view(&self) -> Element<Self::ViewMessage> {
        let title = type_name::<Arpeggiator>();
        let contents = format!("cutoff: {}", "Foo TODO");
        GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
    }
}

impl<M: MessageBounds> Viewable for BeatSequencer<M> {
    default type ViewMessage = M;

    default fn view(&self) -> Element<Self::ViewMessage> {
        let title = type_name::<BeatSequencer<M>>();
        let contents = format!("cursor point: {}", "tOdO");
        GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
    }
}

impl Viewable for BeatSequencer<EntityMessage> {
    type ViewMessage = EntityMessage;

    fn view(&self) -> Element<Self::ViewMessage> {
        let title = type_name::<BeatSequencer<EntityMessage>>();
        let contents = format!("cursor point: {}", "tOdO");
        GuiStuff::titled_container(title, GuiStuff::container_text(contents.as_str()))
    }
}

impl<M: MessageBounds> Viewable for Orchestrator<M> {
    default type ViewMessage = M;

    default fn view(&self) -> Element<Self::ViewMessage> {
        container(text("not implemented")).into()
    }
}

impl Viewable for Orchestrator<GrooveMessage> {
    type ViewMessage = GrooveMessage;

    fn view(&self) -> Element<Self::ViewMessage> {
        let views = self
            .store()
            .iter()
            .fold(Vec::new(), |mut v, (uid, e)| match e {
                crate::traits::BoxedEntity::Controller(entity) => {
                    v.push(
                        entity
                            .view()
                            .map(move |message| GrooveMessage::EntityMessage(*uid, message)),
                    );
                    v
                }
                crate::traits::BoxedEntity::Effect(entity) => {
                    v.push(
                        entity
                            .view()
                            .map(move |message| GrooveMessage::EntityMessage(*uid, message)),
                    );
                    v
                }
                crate::traits::BoxedEntity::Instrument(entity) => {
                    v.push(
                        entity
                            .view()
                            .map(move |message| GrooveMessage::EntityMessage(*uid, message)),
                    );
                    v
                }
            });
        //        let pattern_view = self.pattern_manager().view();
        column(views).into()
    }
}

impl<M: MessageBounds> Viewable for ControlTrip<M> {
    type ViewMessage = M;
}

impl<M: MessageBounds> Viewable for MidiTickSequencer<M> {
    type ViewMessage = M;
}
