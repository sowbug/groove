// Copyright (c) 2023 Mike Tsao. All rights reserved.

use ensnare_core::midi::prelude::*;
use serde::{Deserialize, Serialize};

pub use calculator::Calculator;
pub use control_trip::{ControlPath, ControlStep};

mod calculator;
mod control_trip;
mod lfo;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename = "midi", rename_all = "kebab-case")]
pub struct MidiChannelParams {
    pub midi_in: MidiChannel,
    pub midi_out: MidiChannel,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename = "midi", rename_all = "kebab-case")]
pub struct MidiChannelInputParams {
    pub midi_in: MidiChannel,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename = "midi", rename_all = "kebab-case")]
pub struct MidiChannelOutputParams {
    pub midi_out: MidiChannel,
}

#[cfg(test)]
mod tests {
    use ensnare_core::{prelude::*, temp_impls::prelude::*, traits::prelude::*};
    use std::ops::Range;

    #[test]
    fn instantiate_trigger() {
        let ts = TimeSignature::default();
        let mut trigger = Trigger::new_with(
            Timer::new_with(MusicalTime::new_with_bars(&ts, 1)),
            ControlValue::from(0.5),
        );
        trigger.update_sample_rate(SampleRate::DEFAULT);
        trigger.play();

        trigger.update_time(&Range {
            start: MusicalTime::default(),
            end: MusicalTime::new_with_parts(1),
        });
        let mut count = 0;
        trigger.work(&mut |_, _| {
            count += 1;
        });
        assert_eq!(count, 0);
        assert!(!trigger.is_finished());

        trigger.update_time(&Range {
            start: MusicalTime::new_with_bars(&ts, 1),
            end: MusicalTime::new(&ts, 1, 0, 0, 1),
        });
        let mut count = 0;
        trigger.work(&mut |_, _| {
            count += 1;
        });
        assert!(count != 0);
        assert!(trigger.is_finished());
    }
}
