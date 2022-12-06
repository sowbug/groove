use super::{
    patterns::{Note, Pattern, PatternManager},
    MidiOutputHandler,
};
use crate::{
    gui::{GuiStuff, Viewable},
    messages::EntityMessage,
    MidiHandler,
};
use iced::{
    widget::{button, column, row, text},
    Element,
};
use std::any::type_name;

impl Viewable for MidiOutputHandler {
    type ViewMessage = EntityMessage;
}
impl Viewable for MidiHandler {
    type ViewMessage = EntityMessage;
}

#[derive(Clone, Debug)]
pub enum PatternMessage {
    SomethingHappened,
    ButtonPressed,
}

impl Pattern<Note> {
    fn view<'a>(&self) -> Element<'a, PatternMessage> {
        let mut note_rows = Vec::new();
        for track in self.notes.iter() {
            let mut note_row = Vec::new();
            for note in track {
                let cell = text(format!("{:02} ", note.key).to_string());
                note_row.push(cell.into());
            }
            let row_note_row = row(note_row).into();
            note_rows.push(row_note_row);
        }
        column(vec![
            button(text(format!("{:?}", self.note_value)))
                .on_press(PatternMessage::ButtonPressed)
                .into(),
            column(note_rows).into(),
        ])
        .into()
    }
}

impl Viewable for PatternManager {
    type ViewMessage = EntityMessage;

    fn view(&self) -> Element<Self::ViewMessage> {
        let title = type_name::<PatternManager>();
        let contents = {
            let pattern_views = self.patterns().iter().enumerate().map(|(i, item)| {
                item.view()
                    .map(move |message| Self::ViewMessage::PatternMessage(i, message))
            });
            column(pattern_views.collect())
        };
        GuiStuff::titled_container(title, contents.into())
    }
}
