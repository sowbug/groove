// Copyright (c) 2023 Mike Tsao. All rights reserved.

//! The egui app is an [egui](https://github.com/emilk/egui)-based DAW.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crossbeam_channel::Sender;
use eframe::egui::{self, CollapsingHeader, ComboBox, DragValue, RichText, Slider, Ui};
use groove_audio::{AudioInterfaceEvent, AudioInterfaceInput, AudioQueue, AudioStreamService};
use groove_core::{
    generators::{Envelope, Waveform},
    time::ClockNano,
    traits::{Performs, Resets},
    BipolarNormal, FrequencyHz, ParameterType, StereoSample, SAMPLE_BUFFER_SIZE,
};
use groove_entities::{
    controllers::LfoController,
    effects::{BiQuadFilterLowPass24db, Mixer},
    instruments::{Metronome, WelshSynth},
};
use groove_orchestration::Orchestrator;
use groove_settings::SongSettings;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};
use strum::IntoEnumIterator;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1920.0, 1080.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Audio Prototype (egui)",
        options,
        Box::new(|_cc| Box::<GrooveApp>::default()),
    )
}

struct GrooveApp {
    orchestrator: Arc<Mutex<Orchestrator>>,
    bpm: ParameterType,
    sample_rate: Arc<Mutex<usize>>,

    audio_stream_sender: Sender<AudioInterfaceInput>,
    control_bar: ControlBar,

    tree: Tree,
}
impl Default for GrooveApp {
    fn default() -> Self {
        let clock_settings = ClockNano::default();
        let audio_stream_service = AudioStreamService::new();
        let audio_stream_sender = audio_stream_service.sender().clone();
        let orchestrator = Arc::new(Mutex::new(Orchestrator::new_with(clock_settings)));
        let orchestrator_clone = Arc::clone(&orchestrator);
        const SAMPLE_RATE: usize = 44100;
        let sample_rate = Arc::new(Mutex::new(SAMPLE_RATE));
        Self::start_audio_stream(
            orchestrator_clone,
            audio_stream_service,
            Arc::clone(&sample_rate),
        );
        Self {
            bpm: Default::default(),
            orchestrator,

            sample_rate,
            audio_stream_sender,
            control_bar: ControlBar::default(),
            tree: Tree::demo(),
        }
    }
}

impl eframe::App for GrooveApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(o) = self.orchestrator.lock() {
            self.bpm = o.bpm();
        }
        let top = egui::TopBottomPanel::top("control-bar");
        let bottom = egui::TopBottomPanel::bottom("orchestrator");
        let left = egui::SidePanel::left("left-sidebar");
        let center = egui::CentralPanel::default();

        top.show(ctx, |ui| {
            if let Ok(mut o) = self.orchestrator.lock() {
                self.control_bar.show(ui, &mut o);
            }
        });
        bottom.show(ctx, |ui| {
            if let Ok(o) = self.orchestrator.lock() {
                ui.label(format!("clock: {:?}", o.clock()));
            }
            if ui.button("load").clicked() {
                self.handle_load();
            }
        });
        left.show(ctx, |ui| {
            CollapsingHeader::new("File browser")
                .default_open(true)
                .show(ui, |ui| self.tree.ui(ui));
        });
        center.show(ctx, |ui| {
            if let Ok(mut o) = self.orchestrator.lock() {
                o.show(ui);
            }
        });

        // TODO: this is how to keep redrawing when the system doesn't otherwise
        // know that a repaint is needed. This is fine for now, but it's
        // expensive, and we should be smarter about it.
        ctx.request_repaint();
    }
}
impl GrooveApp {
    fn start_audio_stream(
        orchestrator_clone: Arc<Mutex<Orchestrator>>,
        audio_stream_service: AudioStreamService,
        sample_rate_clone: Arc<Mutex<usize>>,
    ) {
        std::thread::spawn(move || {
            let orchestrator = orchestrator_clone;
            let mut queue_opt = None;
            loop {
                if let Ok(event) = audio_stream_service.receiver().recv() {
                    match event {
                        AudioInterfaceEvent::Reset(sample_rate, queue) => {
                            if let Ok(mut sr) = sample_rate_clone.lock() {
                                *sr = sample_rate;
                            }
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
        for i in 0..buffer_count {
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
                groove_orchestration::messages::Internal::Single(event) => {
                    //                    self.handle_groove_event(event);
                }
                groove_orchestration::messages::Internal::Batch(events) => {
                    for event in events {
                        //                      self.handle_groove_event(event)
                    }
                }
            }
        }
    }

    fn handle_load(&mut self) {
        let filename = "/home/miket/src/groove/projects/demos/controllers/stereo-automation.yaml";
        match SongSettings::new_from_yaml_file(filename) {
            Ok(s) => {
                let pb = PathBuf::from("/home/miket/src/groove/assets");
                match s.instantiate(&pb, false) {
                    Ok(instance) => {
                        if let Ok(mut o) = self.orchestrator.lock() {
                            if let Ok(sample_rate) = self.sample_rate.lock() {
                                *o = instance;
                                self.bpm = o.bpm();
                                o.reset(*sample_rate);
                            }
                        }
                    }
                    Err(err) => eprintln!("instantiate: {}", err),
                }
            }
            Err(err) => eprintln!("new_from_yaml: {}", err),
        }
    }
}

#[derive(Debug, Default)]
struct ControlBar {}
impl ControlBar {
    fn show(&self, ui: &mut egui::Ui, orchestrator: &mut Orchestrator) {
        ui.horizontal(|ui| {
            let mut bpm = orchestrator.bpm();
            ui.label("BPM");
            if ui.add(DragValue::new(&mut bpm).speed(0.1)).changed() {
                orchestrator.set_bpm(bpm);
            }
            if ui.button("start over").clicked() {
                orchestrator.skip_to_start();
            }
            if ui.button("play").clicked() {
                orchestrator.play();
            }
            if ui.button("pause").clicked() {
                orchestrator.stop();
            }

            let clock = orchestrator.clock();
            let minutes: u8 = (clock.seconds() / 60.0).floor() as u8;
            let seconds = clock.seconds() as usize % 60;
            let thousandths = (clock.seconds().fract() * 1000.0) as u16;
            ui.label(format!("{minutes:03}:{seconds:02}:{thousandths:03}"));
        });
    }
}

trait Shows {
    fn show(&mut self, ui: &mut egui::Ui);
}

impl Shows for Envelope {
    fn show(&mut self, ui: &mut egui::Ui) {
        let mut attack = self.attack();
        let mut decay = self.decay();
        let mut sustain = self.sustain().value();
        let mut release = self.release();
        ui.label("Attack");
        if ui.add(DragValue::new(&mut attack).speed(0.1)).changed() {
            self.set_attack(attack);
        }
        ui.end_row();
        ui.label("Decay");
        if ui.add(DragValue::new(&mut decay).speed(0.1)).changed() {
            self.set_decay(decay);
        }
        ui.end_row();
        ui.label("Sustain");
        if ui.add(DragValue::new(&mut sustain).speed(0.1)).changed() {
            self.set_sustain(sustain.into());
        }
        ui.end_row();
        ui.label("Release");
        if ui.add(DragValue::new(&mut release).speed(0.1)).changed() {
            self.set_release(release);
        }
        ui.end_row();
    }
}

impl Shows for WelshSynth {
    fn show(&mut self, ui: &mut egui::Ui) {
        let mut pan = self.pan().value();
        if ui
            .add(
                Slider::new(&mut pan, BipolarNormal::range())
                    .text("Pan")
                    .max_decimals(1),
            )
            .changed()
        {
            self.set_pan(pan.into());
        };
        Envelope::new_with(self.envelope().clone()).show(ui);
    }
}

impl Shows for BiQuadFilterLowPass24db {
    fn show(&mut self, ui: &mut egui::Ui) {
        let mut cutoff = self.cutoff().value();
        let mut pbr = self.passband_ripple();
        if ui
            .add(Slider::new(&mut cutoff, FrequencyHz::range()).text("Cutoff"))
            .changed()
        {
            self.set_cutoff(cutoff.into());
        };
        if ui
            .add(Slider::new(&mut pbr, 0.0..=10.0).text("Passband"))
            .changed()
        {
            self.set_passband_ripple(pbr)
        };
    }
}

impl Shows for LfoController {
    fn show(&mut self, ui: &mut egui::Ui) {
        let mut frequency = self.frequency().value();
        let mut waveform = self.waveform();
        if ui
            .add(Slider::new(&mut frequency, LfoController::frequency_range()).text("Frequency"))
            .changed()
        {
            self.set_frequency(frequency.into());
        };
        ComboBox::new(ui.next_auto_id(), "Waveform")
            .selected_text(waveform.to_string())
            .show_ui(ui, |ui| {
                for w in Waveform::iter() {
                    ui.selectable_value(&mut waveform, w, w.to_string());
                }
            });
        if waveform != self.waveform() {
            eprintln!("changed {} {}", self.waveform(), waveform);
            self.set_waveform(waveform);
        }
    }
}

impl Shows for Metronome {
    fn show(&mut self, ui: &mut egui::Ui) {
        ui.label(format!("BPM: {:0.1}", self.bpm()));
        ui.label(format!(
            "Time Signature: {}/{}",
            self.clock().time_signature().top,
            self.clock().time_signature().bottom
        ));
        ui.label(if self.is_playing() { "X" } else { " " });
    }
}

impl Shows for Mixer {
    fn show(&mut self, ui: &mut egui::Ui) {
        // Mixer doesn't have any UI
    }
}

impl Shows for Orchestrator {
    fn show(&mut self, ui: &mut egui::Ui) {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
            let uids: Vec<usize> = self.entity_iter().map(|(uid, _entity)| *uid).collect();
            for uid in uids {
                let entity = self.get_mut(uid).unwrap();
                CollapsingHeader::new(entity.as_has_uid().name())
                    .default_open(true)
                    .show(ui, |ui| {
                        egui::Frame::none()
                            .fill(egui::Color32::DARK_GRAY)
                            .show(ui, |ui| {
                                ui.vertical(|ui| match entity {
                                    groove_orchestration::Entity::Arpeggiator(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::BiQuadFilterAllPass(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::BiQuadFilterBandPass(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::BiQuadFilterBandStop(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::BiQuadFilterHighPass(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::BiQuadFilterHighShelf(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::BiQuadFilterLowPass12db(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::BiQuadFilterLowPass24db(e) => {
                                        e.show(ui);
                                    }
                                    groove_orchestration::Entity::BiQuadFilterLowShelf(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::BiQuadFilterNone(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::BiQuadFilterPeakingEq(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::Bitcrusher(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::Chorus(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::Clock(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::Compressor(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::ControlTrip(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::DebugSynth(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::Delay(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::Drumkit(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::FmSynth(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::Gain(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::LfoController(e) => {
                                        e.show(ui);
                                    }
                                    groove_orchestration::Entity::Limiter(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::Metronome(e) => {
                                        e.show(ui);
                                    }
                                    groove_orchestration::Entity::MidiTickSequencer(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::Mixer(e) => {
                                        e.show(ui);
                                    }
                                    groove_orchestration::Entity::PatternManager(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::Reverb(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::Sampler(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::Sequencer(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::SignalPassthroughController(
                                        e,
                                    ) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::Timer(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::ToyAudioSource(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::ToyController(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::ToyEffect(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::ToyInstrument(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::ToySynth(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::Trigger(e) => {
                                        ui.label(entity.as_has_uid().name());
                                    }
                                    groove_orchestration::Entity::WelshSynth(e) => {
                                        e.show(ui);
                                    }
                                })
                            });
                    });
            }
        });
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Action {
    Keep,
    Delete,
}

#[derive(Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct Tree(Vec<Tree>);

impl Tree {
    pub fn demo() -> Self {
        Self(vec![
            Tree(vec![Tree::default(); 4]),
            Tree(vec![Tree(vec![Tree::default(); 2]); 3]),
        ])
    }

    pub fn ui(&mut self, ui: &mut Ui) -> Action {
        self.ui_impl(ui, 0, "root")
    }
}

impl Tree {
    fn ui_impl(&mut self, ui: &mut Ui, depth: usize, name: &str) -> Action {
        CollapsingHeader::new(name)
            .default_open(depth < 1)
            .show(ui, |ui| self.children_ui(ui, depth))
            .body_returned
            .unwrap_or(Action::Keep)
    }

    fn children_ui(&mut self, ui: &mut Ui, depth: usize) -> Action {
        if depth > 0
            && ui
                .button(RichText::new("delete").color(ui.visuals().warn_fg_color))
                .clicked()
        {
            return Action::Delete;
        }

        self.0 = std::mem::take(self)
            .0
            .into_iter()
            .enumerate()
            .filter_map(|(i, mut tree)| {
                if tree.ui_impl(ui, depth + 1, &format!("child #{}", i)) == Action::Keep {
                    Some(tree)
                } else {
                    None
                }
            })
            .collect();

        if ui.button("+").clicked() {
            self.0.push(Tree::default());
        }

        Action::Keep
    }
}
