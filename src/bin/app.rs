#![feature(trait_upcasting)]
#![allow(incomplete_features)]

use iced::button::{self, Button};
use iced::text_input::{self, TextInput};
use iced::{Alignment, Column, Element, Row, Sandbox, Settings, Text};
use iced_audio::{knob, IntRange, Knob, Normal};
use libgroove::Orchestrator;
use libgroove::{IOHelper, SongSettings};

pub fn main() -> iced::Result {
    Groove::run(Settings::default())
}
struct Groove {
    filename: String,
    song_settings: SongSettings,
    orchestrator: Orchestrator,
    play_button: button::State,
    stop_button: button::State,
    bpm_state: knob::State,
    bpm_text_state: text_input::State,
    misc_value: i32,
}

#[derive(Debug, Clone)]
enum Message {
    PlayPressed,
    StopPressed,
    BpmKnobChanged(Normal),
    BpmTextInputChanged(String),
}

impl<'a> Groove {
    fn container(title: &str) -> Column<'a, Message> {
        Column::new().push(Text::new(title).size(50)).spacing(20)
    }
}

impl Sandbox for Groove {
    type Message = Message;

    fn new() -> Self {
        let filename = "scripts/everything.yaml";
        let song_settings = IOHelper::song_settings_from_yaml_file(filename);
        Self {
            filename: filename.to_string(),
            song_settings: song_settings.clone(),
            orchestrator: Orchestrator::new_with(&song_settings),
            play_button: button::State::new(),
            stop_button: button::State::new(),
            bpm_state: knob::State::new(IntRange::new(1, 256).normal_param(128, 128)),
            bpm_text_state: text_input::State::new(),
            misc_value: 69,
        }
    }

    fn title(&self) -> String {
        self.filename.clone()
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::PlayPressed => {
                self.orchestrator = Orchestrator::new_with(&self.song_settings);
                let performance = self.orchestrator.perform();
                if let Ok(performance) = performance {
                    if IOHelper::send_performance_to_output_device(performance).is_ok() {
                        // great
                    }
                }
            }
            Message::StopPressed => {
                todo!();
            }
            Message::BpmKnobChanged(value) => {
                let new_value = value.scale(128.0 * 2.0);
                dbg!(value, new_value);
                // TODO: Orchestrator explodes if we try changing settings midway,
                // so for now we're keeping two copies of the things we change!
                self.song_settings.clock.set_bpm(new_value);
                self.orchestrator.set_bpm(new_value);
                let bpm = self.orchestrator.bpm();
                dbg!(bpm);
            }
            Message::BpmTextInputChanged(value) => {
                if let Ok(value_f32) = value.parse::<f32>() {
                    let bpm = value_f32 * 128.0 * 2.0;
                    self.song_settings.clock.set_bpm(bpm);
                    self.orchestrator.set_bpm(bpm);
                    self.misc_value += 1; // = value_f32.to_string();
                }
                dbg!(value, self.misc_value);
            }
        }
    }

    fn view(&mut self) -> Element<Message> {
        let bpm = self.orchestrator.bpm();
        dbg!(bpm);
        let top_row = Row::new()
            .padding(20)
            .align_items(Alignment::Start)
            .push(Knob::new(
                &mut self.bpm_state,
                Message::BpmKnobChanged,
                || None,
                || None,
            ))
            .push(TextInput::new(
                &mut self.bpm_text_state,
                "huh?",
                format!("{}", bpm).as_str(),
                Message::BpmTextInputChanged,
            ))
            .push(
                Button::new(&mut self.play_button, Text::new("Play"))
                    .on_press(Message::PlayPressed),
            )
            .push(
                Button::new(&mut self.stop_button, Text::new("Stop"))
                    .on_press(Message::StopPressed),
            );

        let _source_row: Row<Message> = Row::new()
            .padding(20)
            .align_items(Alignment::Start)
            .push(Column::new());

        let mut button_state_vec = Vec::<button::State>::new();
        for _i in self.orchestrator.main_mixer().sources().iter().enumerate() {
            let button_state = button::State::new();
            dbg!(button_state);
            // source_row = source_row.push(Button::new(
            //     &mut button_state,
            //     Text::new(format!("{}", i.0).as_str()),
            // ));
            button_state_vec.push(button_state);
        }

        // //        Container::new(Column::new().push(top_row).push(source_row)).into()

        Self::container("this is a column")
            .push(Text::new("asdfadfs"))
            .push(Text::new("aopisdopdfgio"))
            .push(top_row)
            .push(Text::new(self.misc_value.to_string()))
            .into()
    }
}
