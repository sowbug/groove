use std::sync::{Arc, Condvar, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleRate, StreamConfig};
use crossbeam::deque::{Stealer, Worker};
use iced::button::{self, Button};
use iced::{Alignment, Column, Element, Sandbox, Settings, Text};
use libgroove::common::MonoSample;
use libgroove::devices::orchestrator::Orchestrator;
use libgroove::settings::song::SongSettings;

pub fn main() -> iced::Result {
    Groove::run(Settings::default())
}

#[derive(Default)]
struct Groove {
    orchestrator: Orchestrator,
    value: i32,
    increment_button: button::State,
    decrement_button: button::State,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    IncrementPressed,
    DecrementPressed,
}

impl Groove {
    fn get_sample_from_queue<T: cpal::Sample>(
        stealer: &Stealer<MonoSample>,
        sync_pair: &Arc<(Mutex<bool>, Condvar)>,
        data: &mut [T],
        _info: &cpal::OutputCallbackInfo,
    ) {
        let lock = &sync_pair.0;
        let cvar = &sync_pair.1;
        let mut finished = lock.lock().unwrap();

        for next_sample in data.iter_mut() {
            let sample_option = stealer.steal();
            let sample: MonoSample = if sample_option.is_success() {
                sample_option.success().unwrap_or_default()
            } else {
                // TODO(miket): this isn't great, because we don't know whether
                // the steal failure was because of a spurious error (buffer underrun)
                // or complete processing.
                *finished = true;
                cvar.notify_one();
                0.
            };
            // This is where MonoSample becomes an f32.
            let sample_crossover: f32 = sample as f32;
            *next_sample = cpal::Sample::from(&sample_crossover);
        }
    }

    fn send_performance_to_output_device(&self, worker: &Worker<MonoSample>) -> anyhow::Result<()> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("no output device available");

        let mut supported_configs_range = device
            .supported_output_configs()
            .expect("error while querying configs");
        let supported_config = supported_configs_range
            .next()
            .expect("no supported config?!")
            .with_sample_rate(SampleRate(
                self.orchestrator.settings().clock.sample_rate() as u32
            ));

        let err_fn = |err| eprintln!("an error occurred on the output audio stream: {}", err);
        let sample_format = supported_config.sample_format();
        let config: StreamConfig = supported_config.into();

        let stealer = worker.stealer();

        let sync_pair = Arc::new((Mutex::new(false), Condvar::new()));
        let sync_pair_clone = Arc::clone(&sync_pair);
        let stream = match sample_format {
            cpal::SampleFormat::F32 => device.build_output_stream(
                &config,
                move |data, output_callback_info| {
                    Self::get_sample_from_queue::<f32>(
                        &stealer,
                        &sync_pair_clone,
                        data,
                        output_callback_info,
                    )
                },
                err_fn,
            ),
            cpal::SampleFormat::I16 => device.build_output_stream(
                &config,
                move |data, output_callback_info| {
                    Self::get_sample_from_queue::<i16>(
                        &stealer,
                        &sync_pair_clone,
                        data,
                        output_callback_info,
                    )
                },
                err_fn,
            ),
            cpal::SampleFormat::U16 => device.build_output_stream(
                &config,
                move |data, output_callback_info| {
                    Self::get_sample_from_queue::<u16>(
                        &stealer,
                        &sync_pair_clone,
                        data,
                        output_callback_info,
                    )
                },
                err_fn,
            ),
        }
        .unwrap();

        stream.play()?;

        // See https://doc.rust-lang.org/stable/std/sync/struct.Condvar.html for origin of this
        // code.
        let &(ref lock, ref cvar) = &*sync_pair;
        let mut finished = lock.lock().unwrap();
        while !*finished {
            finished = cvar.wait(finished).unwrap();
        }
        Ok(())
    }
}

impl Sandbox for Groove {
    type Message = Message;

    fn new() -> Self {
        let yaml = std::fs::read_to_string("scripts/everything.yaml").unwrap();
        let song_settings = SongSettings::new_from_yaml(yaml.as_str());
        Self {
            orchestrator: Orchestrator::new(song_settings.unwrap()),
            ..Default::default()
        }
    }

    fn title(&self) -> String {
        String::from("Counter - Iced")
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::IncrementPressed => {
                self.orchestrator = Orchestrator::new(self.orchestrator.settings().clone());
                let worker = Worker::<MonoSample>::new_fifo();
                let result = self.orchestrator.perform_to_queue(&worker);
                if result.is_ok() {
                    let result = self.send_performance_to_output_device(&worker);
                    if result.is_ok() {
                        // great
                    }
                }
            }
            Message::DecrementPressed => {
                self.value -= 1;
            }
        }
    }

    fn view(&mut self) -> Element<Message> {
        Column::new()
            .padding(20)
            .align_items(Alignment::Center)
            .push(
                Button::new(&mut self.increment_button, Text::new("Increment"))
                    .on_press(Message::IncrementPressed),
            )
            .push(Text::new(self.value.to_string()).size(50))
            .push(
                Button::new(&mut self.decrement_button, Text::new("Decrement"))
                    .on_press(Message::DecrementPressed),
            )
            .into()
    }
}
