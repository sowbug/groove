// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crossbeam_channel::{Receiver, Sender};
use eframe::egui::{self, Window};
use groove_audio::{AudioInterfaceEvent, AudioInterfaceInput, AudioQueue, AudioStreamService};
use groove_core::{
    traits::{Resets, ShowsTopLevel},
    StereoSample, SAMPLE_BUFFER_SIZE,
};
use groove_orchestration::Orchestrator;
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Debug)]
struct AudioInterfaceConfig {
    sample_rate: usize,
    channel_count: u16,
}

impl AudioInterfaceConfig {
    fn sample_rate(&self) -> usize {
        self.sample_rate
    }

    fn channel_count(&self) -> u16 {
        self.channel_count
    }
}

/// [AudioPanel] manages the audio interface.
#[derive(Debug)]
pub struct AudioPanel {
    sender: Sender<AudioInterfaceInput>,
    orchestrator: Arc<Mutex<Orchestrator>>,

    config: Arc<Mutex<Option<AudioInterfaceConfig>>>,
}
impl AudioPanel {
    /// Construct a new [AudioPanel].
    pub fn new_with(orchestrator: Arc<Mutex<Orchestrator>>) -> Self {
        let audio_stream_service = AudioStreamService::default();
        let sender = audio_stream_service.sender().clone();

        let r = Self {
            sender,
            orchestrator: Arc::clone(&orchestrator),
            config: Default::default(),
        };
        r.start_audio_stream(audio_stream_service.receiver().clone());
        r
    }

    #[allow(dead_code)]
    pub(crate) fn send(&mut self, input: AudioInterfaceInput) {
        let _ = self.sender.send(input);
    }

    fn start_audio_stream(&self, receiver: Receiver<AudioInterfaceEvent>) {
        let orchestrator = Arc::clone(&self.orchestrator);
        let config = Arc::clone(&self.config);
        std::thread::spawn(move || {
            let mut queue_opt = None;
            loop {
                if let Ok(event) = receiver.recv() {
                    match event {
                        AudioInterfaceEvent::Reset(sample_rate, channel_count, queue) => {
                            if let Ok(mut config) = config.lock() {
                                *config = Some(AudioInterfaceConfig {
                                    sample_rate,
                                    channel_count,
                                });
                            }

                            // TODO: there must be a better way to propagate this information.
                            if let Ok(mut o) = orchestrator.lock() {
                                o.reset(sample_rate);
                            }
                            queue_opt = Some(queue);
                        }
                        AudioInterfaceEvent::NeedsAudio(_when, count) => {
                            if let Some(queue) = queue_opt.as_ref() {
                                if let Ok(o) = orchestrator.lock() {
                                    Self::generate_audio(o, queue, (count / 64) as u8);
                                }
                            }
                        }
                        AudioInterfaceEvent::Quit => todo!(),
                    }
                }
            }
        });
    }

    fn generate_audio(
        mut orchestrator: MutexGuard<Orchestrator>,
        queue: &AudioQueue,
        buffer_count: u8,
    ) {
        let mut samples = [StereoSample::SILENCE; SAMPLE_BUFFER_SIZE];
        for _ in 0..buffer_count {
            let (response, ticks_completed) = orchestrator.tick(&mut samples);
            if ticks_completed < samples.len() {
                // self.stop_playback();
                // self.reached_end_of_playback = true;
            }

            for sample in samples {
                let _ = queue.push(sample);
            }

            match response.0 {
                groove_orchestration::messages::Internal::None => {}
                groove_orchestration::messages::Internal::Single(_event) => {
                    //                    self.handle_groove_event(event);
                }
                groove_orchestration::messages::Internal::Batch(events) => {
                    for _event in events {
                        //                      self.handle_groove_event(event)
                    }
                }
            }
        }
    }
}
impl ShowsTopLevel for AudioPanel {
    fn show(&mut self, ctx: &egui::Context) {
        Window::new("Audio").default_open(true).show(ctx, |ui| {
            if let Ok(Some(config)) = self.config.lock().as_deref() {
                ui.label(format!("Sample rate: {}", config.sample_rate()));
                ui.label(format!("Channels: {}", config.channel_count()));
            }
        });
    }
}
