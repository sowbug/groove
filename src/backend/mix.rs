use crate::backend::clock::Clock;

use super::instruments::old_Oscillator;

//pub struct AudioOutput {
//}

// impl AudioOutput {
//     pub fn get_sample(&self, clock: &Clock) -> f32 {
//         return 0.;
//     }
// }

struct MixableSource {
    source: old_Oscillator,
    weight: f32,
    normalized_weight: f32,
}

pub struct old_Mixer {
    //source1: MixableSource,
    //source2: MixableSource,
}

impl old_Mixer {
//    pub fn new(source1: Oscillator, source2: Oscillator) -> Mixer {
        pub fn new() -> old_Mixer {
            old_Mixer {
            // source1: MixableSource { source: source1, weight: 0.5, normalized_weight: 0.5 },
            // source2: MixableSource { source: source2, weight: 0.5, normalized_weight: 0.5 },
        }
    }

    pub fn add_source(&mut self, source: old_Oscillator, weight: f32) {
        // self.source1 = MixableSource {
        //     source: source,
        //     weight: weight,
        //     normalized_weight: todo!(),
        // };
    }

    pub fn get_sample(&self, clock: &Clock) -> f32 {
        // &self.source1.source.get_sample(clock) * &self.source1.weight
        //     + &self.source2.source.get_sample(clock) * &self.source2.weight
        0.
    }
}
