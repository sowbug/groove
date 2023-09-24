// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::panels::{audio_panel::AudioSettings, AudioPanelEvent};
use crossbeam_channel::{Receiver, Sender};
use eframe::egui::{CollapsingHeader, Ui};
use ensnare_core::core::StereoSample;
use groove_audio::{AudioInterfaceEvent, AudioInterfaceInput, AudioQueue, AudioStreamService};
use groove_core::{traits::gui::Displays, SAMPLE_BUFFER_SIZE};
use std::{
    fmt::Debug,
    sync::{Arc, Mutex, MutexGuard},
};

/// [AudioPanel] manages the audio interface.
#[derive(Debug)]
#[deprecated]
pub struct AudioPanel {
    sender: Sender<AudioInterfaceInput>,
    app_receiver: Receiver<AudioPanelEvent>, // to give to the app to receive what we sent
    app_sender: Sender<AudioPanelEvent>,     // for us to send to the app
    orchestrator: Arc<Mutex<Orchestrator>>,

    config: Arc<Mutex<Option<AudioSettings>>>,
}
impl AudioPanel {
    /// Construct a new [AudioPanel].
    pub fn new_with(orchestrator: Arc<Mutex<Orchestrator>>) -> Self {
        let audio_stream_service = AudioStreamService::default();
        let sender = audio_stream_service.sender().clone();

        let (app_sender, app_receiver) = crossbeam_channel::unbounded();

        let r = Self {
            sender,
            app_sender,
            app_receiver,
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
        let app_sender = self.app_sender.clone();
        std::thread::spawn(move || {
            let mut queue_opt = None;
            loop {
                if let Ok(event) = receiver.recv() {
                    match event {
                        AudioInterfaceEvent::Reset(sample_rate, channel_count, queue) => {
                            if let Ok(mut config) = config.lock() {
                                *config = Some(AudioSettings::new_with(sample_rate, channel_count));
                            }
                            let _ = app_sender.send(AudioPanelEvent::InterfaceChanged);
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
                } else {
                    eprintln!("Unexpected end of AudioInterfaceEvent channel");
                    break;
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

    /// The receive side of the [AudioPanelEvent] channel
    pub fn receiver(&self) -> &Receiver<AudioPanelEvent> {
        &self.app_receiver
    }

    /// The audio interface's current sample rate
    pub fn sample_rate(&self) -> SampleRate {
        if let Ok(config) = self.config.lock() {
            if let Some(config) = config.as_ref() {
                return config.sample_rate();
            }
        }
        eprintln!("Warning: returning default sample rate because actual was not available");
        SampleRate::DEFAULT
    }

    /// The audio interface's current number of channels. 1 = mono, 2 = stereo
    pub fn channel_count(&self) -> u16 {
        if let Ok(config) = self.config.lock() {
            if let Some(config) = config.as_ref() {
                return config.channel_count();
            }
        }
        0
    }
}
impl Displays for AudioPanel {
    fn ui(&mut self, ui: &mut Ui) -> eframe::egui::Response {
        CollapsingHeader::new("Audio")
            .default_open(true)
            .show(ui, |ui| {
                if let Ok(Some(config)) = self.config.lock().as_deref() {
                    ui.label(format!("Sample rate: {}", config.sample_rate()));
                    ui.label(format!("Channels: {}", config.channel_count()));
                }
            })
            .header_response
    }
}
