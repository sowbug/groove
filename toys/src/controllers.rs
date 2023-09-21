// Copyright (c) 2023 Mike Tsao. All rights reserved.

use ensnare::{midi::prelude::*, prelude::*, traits::prelude::*};
use ensnare_proc_macros::{IsController, Uid};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Uid, IsController, Serialize, Deserialize)]
pub struct ToyControllerAlwaysSendsMidiMessage {
    uid: Uid,

    #[serde(skip)]
    midi_note: u8,

    #[serde(skip)]
    is_performing: bool,
}
impl Displays for ToyControllerAlwaysSendsMidiMessage {}
impl HandlesMidi for ToyControllerAlwaysSendsMidiMessage {}
impl Controls for ToyControllerAlwaysSendsMidiMessage {
    fn update_time(&mut self, _range: &std::ops::Range<MusicalTime>) {}

    fn work(&mut self, control_events_fn: &mut ControlEventsFn) {
        if self.is_performing {
            control_events_fn(
                self.uid,
                EntityEvent::Midi(
                    MidiChannel(0),
                    MidiMessage::NoteOn {
                        key: u7::from(self.midi_note),
                        vel: u7::from(127),
                    },
                ),
            );
            self.midi_note += 1;
            if self.midi_note > 127 {
                self.midi_note = 1;
            }
        }
    }

    fn is_finished(&self) -> bool {
        false
    }

    fn play(&mut self) {
        self.is_performing = true;
    }

    fn stop(&mut self) {
        self.is_performing = false;
    }

    fn skip_to_start(&mut self) {
        todo!()
    }

    fn is_performing(&self) -> bool {
        self.is_performing
    }
}
impl Configurable for ToyControllerAlwaysSendsMidiMessage {}
impl Serializable for ToyControllerAlwaysSendsMidiMessage {}
