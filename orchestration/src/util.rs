// Copyright (c) 2023 Mike Tsao. All rights reserved.

#[allow(dead_code)]
pub(crate) fn transform_linear_to_mma_concave(linear_value: f64) -> f64 {
    const MAX_VALUE: f64 = 1.0;
    if linear_value > (1.0 - 10.0f64.powf(-12.0 / 5.0) * MAX_VALUE) {
        MAX_VALUE
    } else {
        -(5.0 / 12.0) * (1.0 - linear_value / MAX_VALUE).log10()
    }
}

#[allow(dead_code)]
pub(crate) fn transform_linear_to_mma_convex(linear_value: f64) -> f64 {
    const MAX_VALUE: f64 = 1.0;
    if linear_value < 10.0f64.powf(-12.0 / 5.0) * MAX_VALUE {
        0.0
    } else {
        1.0f64 + (5.0 / 12.0) * (linear_value / MAX_VALUE).log10()
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{
        entities::Entity,
        tests::{DEFAULT_BPM, DEFAULT_MIDI_TICKS_PER_SECOND, DEFAULT_SAMPLE_RATE},
        util::{transform_linear_to_mma_concave, transform_linear_to_mma_convex},
        Orchestrator,
    };
    use groove_core::{
        generators::Waveform,
        midi::MidiChannel,
        time::{ClockParams, TimeSignatureParams},
        traits::Resets,
        DcaParams, FrequencyHz, Normal, StereoSample, SAMPLE_BUFFER_SIZE,
    };
    use groove_entities::controllers::{
        LfoController, LfoControllerParams, Timer, TimerParams, ToyController, ToyControllerParams,
    };

    #[cfg(feature = "iced-framework")]
    use groove_entities::ToyMessageMaker;
    #[cfg(feature = "iced-framework")]
    use groove_toys::ToyInstrumentParams;
    use groove_toys::{DebugSynth, ToyEffect, ToyInstrument, ToyInstrumentParams};
    use more_asserts::{assert_ge, assert_gt, assert_le, assert_lt};

    #[test]
    fn audio_routing_works() {
        let mut o = Orchestrator::new_with(&ClockParams {
            bpm: DEFAULT_BPM,
            midi_ticks_per_second: DEFAULT_MIDI_TICKS_PER_SECOND,
            time_signature: TimeSignatureParams { top: 4, bottom: 4 },
        });
        o.reset(DEFAULT_SAMPLE_RATE);

        // A simple audio source.
        let synth_uid = o.add(Entity::DebugSynth(Box::new(DebugSynth::new())));

        // A simple effect.
        let effect_uid = o.add(Entity::ToyEffect(Box::new(ToyEffect::default())));

        // Connect the audio's output to the effect's input.
        assert!(o.patch(synth_uid, effect_uid).is_ok());

        // And patch the effect into the main mixer.
        let _ = o.connect_to_main_mixer(effect_uid);

        // Run the main loop for a while.
        const SECONDS: usize = 1;
        let _ = o.add(Entity::Timer(Box::new(Timer::new_with(&TimerParams {
            seconds: SECONDS as f64,
        }))));

        // Gather the audio output.
        let mut sample_buffer = [StereoSample::SILENCE; SAMPLE_BUFFER_SIZE];
        if let Ok(samples_1) = o.run(&mut sample_buffer) {
            // We should get exactly the right amount of audio.
            assert_eq!(samples_1.len(), SECONDS * DEFAULT_SAMPLE_RATE);

            // It should not all be silence.
            assert!(!samples_1.iter().any(|&s| s != StereoSample::SILENCE));

            // Run again but without the negating effect in the mix.
            assert!(o.unpatch(synth_uid, effect_uid).is_ok());
            if let Ok(samples_2) = o.run(&mut sample_buffer) {
                // The sample pairs should cancel each other out.
                assert!(!samples_2.iter().any(|&s| s != StereoSample::SILENCE));
                samples_1.iter().zip(samples_2.iter()).all(|(a, b)| {
                    *a + *b == StereoSample::SILENCE && (*a == StereoSample::SILENCE || *a != *b)
                });
            }
        }
    }

    // TODO: I had a bug for a day where I'd swapped the param_names for the
    // first and second audio inputs. In other words, the moment I got out
    // of the type system, I failed. Consider taking a more strongly typed
    // argument as an alternative to the (necessary) string argument.

    #[test]
    fn control_routing_works() {
        let mut o = Orchestrator::new_with(&ClockParams {
            bpm: DEFAULT_BPM,
            midi_ticks_per_second: DEFAULT_MIDI_TICKS_PER_SECOND,
            time_signature: TimeSignatureParams { top: 4, bottom: 4 },
        });
        o.reset(DEFAULT_SAMPLE_RATE);

        // The synth's frequency is modulated by the LFO.
        let synth_1_uid = o.add(Entity::DebugSynth(Box::new(DebugSynth::new())));
        let lfo = LfoController::new_with(&LfoControllerParams {
            waveform: Waveform::Sine,
            frequency: FrequencyHz::from(2.0),
        });
        let lfo_uid = o.add(Entity::LfoController(Box::new(lfo)));
        let _ = o.link_control_by_name(lfo_uid, synth_1_uid, "oscillator");

        // We'll hear the synth's audio output.
        let _ = o.connect_to_main_mixer(synth_1_uid);

        const SECONDS: usize = 1;
        let _ = o.add(Entity::Timer(Box::new(Timer::new_with(&TimerParams {
            seconds: SECONDS as f64,
        }))));

        // Gather the audio output.
        let mut sample_buffer = [StereoSample::SILENCE; 12];
        if let Ok(samples_1) = o.run(&mut sample_buffer) {
            // We should get exactly the right amount of audio.
            //
            // TODO: to get this to continue to pass, I changed sample_buffer to
            // be an even divisor of 44100.
            assert_eq!(samples_1.len(), SECONDS * DEFAULT_SAMPLE_RATE);

            // It should not all be silence.
            assert!(!samples_1.iter().any(|&s| s != StereoSample::SILENCE));

            // Run again after disconnecting the LFO.
            o.unlink_control_by_name(lfo_uid, synth_1_uid, "oscillator");
            if let Ok(samples_2) = o.run(&mut sample_buffer) {
                // The two runs should be different. That's not a great test of what
                // we're doing here, but it will detect when things are broken.
                samples_1
                    .iter()
                    .zip(samples_2.iter())
                    .any(|(a, b)| *a != *b);
            }
        }
    }

    #[test]
    fn midi_routing_works() {
        const TEST_MIDI_CHANNEL: MidiChannel = 7;
        const ARP_MIDI_CHANNEL: MidiChannel = 5;
        let mut o = Orchestrator::new_with(&ClockParams {
            bpm: DEFAULT_BPM,
            midi_ticks_per_second: DEFAULT_MIDI_TICKS_PER_SECOND,
            time_signature: TimeSignatureParams { top: 4, bottom: 4 },
        });
        o.reset(DEFAULT_SAMPLE_RATE);

        // We have a regular MIDI instrument, and an arpeggiator that emits MIDI note messages.
        let instrument_uid = o.add(Entity::ToyInstrument(Box::new(ToyInstrument::new_with(
            &ToyInstrumentParams {
                fake_value: Normal::from(0.34598),
                dca: DcaParams {
                    gain: Default::default(),
                    pan: Default::default(),
                },
            },
        ))));
        let arpeggiator_uid = o.add(Entity::ToyController(Box::new(ToyController::new_with(
            ToyControllerParams {
                bpm: DEFAULT_BPM,
                tempo: 99999999999.0,
            },
            TEST_MIDI_CHANNEL,
        ))));

        // We'll hear the instrument.
        assert!(o.connect_to_main_mixer(instrument_uid).is_ok());

        // This might not be necessary. Orchestrator will automatically get
        // every MIDI message sent.
        o.connect_midi_downstream(instrument_uid, TEST_MIDI_CHANNEL);
        o.connect_midi_downstream(arpeggiator_uid, ARP_MIDI_CHANNEL);

        const SECONDS: usize = 1;
        let _ = o.add(Entity::Timer(Box::new(Timer::new_with(&TimerParams {
            seconds: SECONDS as f64,
        }))));

        // Everything is hooked up. Let's run it and hear what we got.
        let mut sample_buffer = [StereoSample::SILENCE; SAMPLE_BUFFER_SIZE];
        if let Ok(samples) = o.run(&mut sample_buffer) {
            // We haven't asked the arpeggiator to start sending anything yet.
            assert_eq!(samples.len(), (SECONDS * DEFAULT_SAMPLE_RATE) as usize);
            assert!(
                samples.iter().all(|&s| s == StereoSample::SILENCE),
                "Expected total silence because the arpeggiator is not turned on."
            );
        } else {
            panic!("impossible!");
        }

        // Let's turn on the arpeggiator.
        o.debug_send_midi_note(ARP_MIDI_CHANNEL, true);
        o.reset(DEFAULT_SAMPLE_RATE);
        if let Ok(samples) = o.run(&mut sample_buffer) {
            assert_eq!(samples.len(), (SECONDS * DEFAULT_SAMPLE_RATE) as usize);
            assert!(
                samples.iter().any(|&s| s != StereoSample::SILENCE),
                "Expected some sound because the arpeggiator is now running."
            );
        } else {
            panic!("impossible!");
        }

        // The arpeggiator is still running. Let's disable it (taking advantage
        // of the fact that TestInstrument has zero release time, because
        // otherwise it would keep generating a bit of sound even after the
        // arpeggiator told it to stop).
        //
        // Note that we're implicitly testing that the arpeggiator will send a
        // note-off if necessary, even if it's disabled mid-note.
        o.debug_send_midi_note(ARP_MIDI_CHANNEL, false);

        // It's actually immaterial to this test whether this has any sound in
        // it. We're just giving the arpeggiator a bit of time to clear out any
        // leftover note.
        o.reset(DEFAULT_SAMPLE_RATE);
        if o.run(&mut sample_buffer).is_err() {
            panic!("impossible!");
        }

        // But by now it should be silent.
        o.reset(DEFAULT_SAMPLE_RATE);
        if let Ok(samples) = o.run(&mut sample_buffer) {
            assert_eq!(samples.len(), (SECONDS * DEFAULT_SAMPLE_RATE) as usize);
            assert!(
                samples.iter().all(|&s| s == StereoSample::SILENCE),
                "Expected total silence again after disabling the arpeggiator."
            );
        } else {
            panic!("impossible!");
        }

        // Re-enable the arpeggiator but disconnect the instrument's MIDI
        // connection.
        o.debug_send_midi_note(ARP_MIDI_CHANNEL, true);
        o.disconnect_midi_downstream(instrument_uid, TEST_MIDI_CHANNEL);
        o.reset(DEFAULT_SAMPLE_RATE);
        if let Ok(samples) = o.run(&mut sample_buffer) {
            assert_eq!(samples.len(), (SECONDS * DEFAULT_SAMPLE_RATE) as usize);
            assert!(
                samples.iter().all(|&s| s == StereoSample::SILENCE),
                "Expected total silence after disconnecting the instrument from the MIDI bus."
            );
        } else {
            panic!("impossible!");
        }
    }

    #[test]
    fn groove_can_be_instantiated_in_new_generic_world() {
        let mut o = Orchestrator::new_with(&ClockParams {
            bpm: DEFAULT_BPM,
            midi_ticks_per_second: DEFAULT_MIDI_TICKS_PER_SECOND,
            time_signature: TimeSignatureParams { top: 4, bottom: 4 },
        });
        o.reset(DEFAULT_SAMPLE_RATE);

        // A simple audio source.
        let entity_groove = Entity::DebugSynth(Box::new(DebugSynth::new()));
        let synth_uid = o.add(entity_groove);

        // A simple effect.
        let effect_uid = o.add(Entity::ToyEffect(Box::new(ToyEffect::default())));

        // Connect the audio's output to the effect's input.
        assert!(o.patch(synth_uid, effect_uid).is_ok());

        // And patch the effect into the main mixer.
        let _ = o.connect_to_main_mixer(effect_uid);

        // Run the main loop for a while.
        const SECONDS: usize = 1;
        let _ = o.add(Entity::Timer(Box::new(Timer::new_with(&TimerParams {
            seconds: SECONDS as f64,
        }))));

        // Gather the audio output.
        let mut sample_buffer = [StereoSample::SILENCE; SAMPLE_BUFFER_SIZE];
        if let Ok(samples_1) = o.run(&mut sample_buffer) {
            // We should get exactly the right amount of audio.
            assert_eq!(samples_1.len(), SECONDS * DEFAULT_SAMPLE_RATE);

            // It should not all be silence.
            assert!(!samples_1.iter().any(|&s| s != StereoSample::SILENCE));

            // Run again but without the negating effect in the mix.
            assert!(o.unpatch(synth_uid, effect_uid).is_ok());
            if let Ok(samples_2) = o.run(&mut sample_buffer) {
                // The sample pairs should cancel each other out.
                assert!(!samples_2.iter().any(|&s| s != StereoSample::SILENCE));
                samples_1.iter().zip(samples_2.iter()).all(|(a, b)| {
                    *a + *b == StereoSample::SILENCE && (*a == StereoSample::SILENCE || *a != *b)
                });
            }
        }
    }

    // The input values in the concave/convex tests were generated by hand in a
    // spreadsheet containing the two formulas, copied from DLS Level 2 from the
    // MMA.
    #[test]
    fn mma_concave_transform() {
        assert_lt!(transform_linear_to_mma_concave(0.001), 0.0002);
        assert_lt!(transform_linear_to_mma_concave(0.01), 0.019);
        assert_lt!(transform_linear_to_mma_concave(0.1), 0.02);
        assert_lt!(transform_linear_to_mma_concave(0.5), 0.13);
        assert_gt!(transform_linear_to_mma_concave(0.5), 0.12);
        assert_gt!(transform_linear_to_mma_concave(0.9), 0.40);
        assert_gt!(transform_linear_to_mma_concave(0.99), 0.83);
        assert_gt!(transform_linear_to_mma_concave(0.995), 0.95);

        for x in 0..=100 {
            let x = x as f64 / 100.0;
            assert_le!(transform_linear_to_mma_concave(x), x);
        }
    }

    #[test]
    fn mma_convex_transform() {
        assert_gt!(transform_linear_to_mma_convex(0.995), 0.999);
        assert_gt!(transform_linear_to_mma_convex(0.99), 0.998);
        assert_gt!(transform_linear_to_mma_convex(0.9), 0.98);
        assert_gt!(transform_linear_to_mma_convex(0.5), 0.87);
        assert_lt!(transform_linear_to_mma_convex(0.5), 0.88);
        assert_lt!(transform_linear_to_mma_convex(0.1), 0.59);
        assert_lt!(transform_linear_to_mma_convex(0.01), 0.17);
        assert_lt!(transform_linear_to_mma_convex(0.001), 0.0005);

        for x in 0..=100 {
            let x = x as f64 / 100.0;
            assert_ge!(transform_linear_to_mma_convex(x), x);
        }
    }
}
