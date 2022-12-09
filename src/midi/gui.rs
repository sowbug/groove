use super::{
    patterns::{Note, Pattern, PatternManager},
    MidiOutputHandler,
};
use crate::{
    gui::{GuiStuff, Viewable},
    messages::EntityMessage,
    MidiHandler, MidiHandlerMessage,
};
use iced::{
    widget::{button, column, container, pick_list, row, text},
    Element,
};
use std::{
    any::type_name,
    time::{Duration, Instant},
};

impl Viewable for MidiOutputHandler {
    type ViewMessage = MidiHandlerMessage;
}
impl Viewable for MidiHandler {
    type ViewMessage = MidiHandlerMessage;

    fn view(&self) -> Element<'_, Self::ViewMessage, iced::Renderer> {
        let activity_text = container(text(
            if Instant::now().duration_since(self.activity_tick) > Duration::from_millis(250) {
                " "
            } else {
                "•"
            },
        ))
        .width(iced::Length::FillPortion(1));
        let (input_selected, input_options) = self.midi_input.as_ref().unwrap().labels();
        let input_menu = row![
            text("Input").width(iced::Length::FillPortion(1)),
            pick_list(
                input_options,
                input_selected.clone(),
                MidiHandlerMessage::InputSelected,
            )
            .width(iced::Length::FillPortion(3))
        ];
        let (output_selected, output_options) = self.midi_output.as_ref().unwrap().labels();
        let output_menu = row![
            text("Output").width(iced::Length::FillPortion(1)),
            pick_list(
                output_options,
                output_selected.clone(),
                MidiHandlerMessage::OutputSelected,
            )
            .width(iced::Length::FillPortion(3))
        ];
        let port_menus =
            container(column![input_menu, output_menu]).width(iced::Length::FillPortion(7));
        GuiStuff::titled_container("MIDI", container(row![activity_text, port_menus]).into())
    }
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
