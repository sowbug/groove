use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, Sample, SizedSample, Stream, SupportedStreamConfig,
};
use crossbeam::queue::ArrayQueue;
use crossbeam_channel::{unbounded, Receiver, Sender};
use groove_core::StereoSample;
use std::{fmt::Debug, result::Result::Ok, sync::Arc, thread::JoinHandle, time::Instant};

pub enum AudioInterfaceInput {
    SetBufferSize(usize),
    Play,
    Pause,
    Quit,
}

#[derive(Clone, Debug)]
pub enum AudioInterfaceEvent {
    Reset(usize, AudioQueue),
    NeedsAudio(Instant, usize),
    Quit,
}

/// The producer-consumer queue of stereo samples that the audio stream consumes.
pub type AudioQueue = Arc<ArrayQueue<StereoSample>>;

pub struct AudioStreamService {
    input_sender: Sender<AudioInterfaceInput>,
    event_receiver: Receiver<AudioInterfaceEvent>, // AudioStream events

    handler: JoinHandle<()>, // The AudioStream thread
}
impl AudioStreamService {
    pub fn new() -> Self {
        // Sends input from the app to the service.
        let (input_sender, input_receiver) = unbounded();

        // Sends events from the service to the app.
        let (event_sender, event_receiver) = unbounded();

        let handler = std::thread::spawn(move || {
            if let Ok(mut audio_stream) = AudioStream::create_default_stream(
                AudioStream::REASONABLE_BUFFER_SIZE,
                event_sender.clone(),
            ) {
                loop {
                    if let Ok(input) = input_receiver.recv() {
                        match input {
                            AudioInterfaceInput::SetBufferSize(_) => todo!(),
                            AudioInterfaceInput::Play => audio_stream.play(),
                            AudioInterfaceInput::Pause => audio_stream.pause(),
                            AudioInterfaceInput::Quit => {
                                audio_stream.quit();
                                break;
                            }
                        }
                    }
                }
            }
        });
        Self {
            input_sender,
            event_receiver,
            handler,
        }
    }

    pub fn sender(&self) -> &Sender<AudioInterfaceInput> {
        &self.input_sender
    }

    pub fn receiver(&self) -> &Receiver<AudioInterfaceEvent> {
        &self.event_receiver
    }
}

/// Encapsulates the connection to the audio interface.
pub struct AudioStream {
    // cpal config describing the current audio stream.
    config: SupportedStreamConfig,

    // The cpal audio stream.
    stream: Stream,

    // The queue of samples that the stream consumes.
    queue: AudioQueue,

    // The sending half of the channel that the audio stream uses to send
    // updates to the subscription.
    sender: Sender<AudioInterfaceEvent>,
}
impl Debug for AudioStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioStream")
            .field("config", &"(skipped)")
            .field("stream", &"(skipped)")
            .field("queue", &self.queue)
            .field("sender", &self.sender)
            .finish()
    }
}
impl AudioStream {
    /// This constant is provided to prevent decision paralysis when picking a
    /// `buffer_size` argument. At a typical sample rate of 44.1KHz, a value of
    /// 2048 would mean that samples at the end of a full buffer wouldn't reach
    /// the audio interface for 46.44 milliseconds, which is arguably not
    /// reasonable because audio latency is perceptible at as few as 10
    /// milliseconds. However, on my Ubuntu 20.04 machine, the audio interface
    /// asks for around 2,600 samples (1,300 stereo samples) at once, which
    /// means that 2,048 leaves a cushion of less than a single callback of
    /// samples.
    pub const REASONABLE_BUFFER_SIZE: usize = 2048;

    pub fn create_default_stream(
        buffer_size: usize,
        audio_stream_event_sender: Sender<AudioInterfaceEvent>,
    ) -> Result<Self, ()> {
        if let Ok((_host, device, config)) = Self::host_device_setup() {
            let queue = Arc::new(ArrayQueue::new(buffer_size));
            if let Ok(stream) = Self::stream_setup_for(
                &device,
                &config,
                &Arc::clone(&queue),
                audio_stream_event_sender.clone(),
            ) {
                let r = Self {
                    config,
                    stream,
                    queue,
                    sender: audio_stream_event_sender,
                };
                r.send_reset();
                Ok(r)
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }

    /// Returns the sample rate of the current audio stream.
    pub fn sample_rate(&self) -> usize {
        let config: &cpal::StreamConfig = &self.config.clone().into();
        config.sample_rate.0 as usize
    }

    /// Tells the audio stream to stop playing audio (which means it will also
    /// stop consuming samples from the queue).
    pub fn play(&self) {
        let _ = self.stream.play();
    }

    /// Tells the audio stream to resume playing audio (and consuming samples
    /// from the queue).
    pub fn pause(&self) {
        let _ = self.stream.pause();
    }

    /// Gives the audio stream a chance to clean up before the thread exits.
    pub fn quit(&mut self) {
        let _ = self.sender.send(AudioInterfaceEvent::Quit);
    }

    /// Returns the default host, device, and stream config (all of which are
    /// cpal concepts).
    fn host_device_setup(
    ) -> anyhow::Result<(cpal::Host, cpal::Device, cpal::SupportedStreamConfig), anyhow::Error>
    {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow::Error::msg("Default output device is not available"))?;
        let config = device.default_output_config()?;
        Ok((host, device, config))
    }

    /// Creates and returns a Stream for the given device and config. The Stream
    /// will consume the supplied ArrayQueue<f32>. This function is actually a
    /// wrapper around the generic stream_make<T>().
    fn stream_setup_for(
        device: &cpal::Device,
        config: &SupportedStreamConfig,
        queue: &AudioQueue,
        audio_stream_event_sender: Sender<AudioInterfaceEvent>,
    ) -> anyhow::Result<Stream, anyhow::Error> {
        let config = config.clone();

        match config.sample_format() {
            cpal::SampleFormat::I8 => todo!(),
            cpal::SampleFormat::I16 => todo!(),
            cpal::SampleFormat::I32 => todo!(),
            cpal::SampleFormat::I64 => todo!(),
            cpal::SampleFormat::U8 => todo!(),
            cpal::SampleFormat::U16 => todo!(),
            cpal::SampleFormat::U32 => todo!(),
            cpal::SampleFormat::U64 => todo!(),
            cpal::SampleFormat::F32 => {
                Self::stream_make::<f32>(&config.into(), &device, queue, audio_stream_event_sender)
            }
            cpal::SampleFormat::F64 => todo!(),
            _ => todo!(),
        }
    }

    /// Generic portion of stream_setup_for().
    fn stream_make<T>(
        config: &cpal::StreamConfig,
        device: &cpal::Device,
        queue: &AudioQueue,
        audio_stream_event_sender: Sender<AudioInterfaceEvent>,
    ) -> Result<Stream, anyhow::Error>
    where
        T: SizedSample + FromSample<f32>,
    {
        let err_fn = |err| eprintln!("Error building output sound stream: {}", err);

        let queue = Arc::clone(&queue);
        let channel_count = config.channels as usize;
        let stream = device.build_output_stream(
            config,
            move |output: &mut [T], _: &cpal::OutputCallbackInfo| {
                Self::on_window(
                    output,
                    channel_count,
                    &queue,
                    audio_stream_event_sender.clone(),
                )
            },
            err_fn,
            None,
        )?;
        Ok(stream)
    }

    /// cpal callback that supplies samples from the ArrayQueue<f32>, converting
    /// them if needed to the stream's expected data type.
    fn on_window<T>(
        output: &mut [T],
        channel_count: usize,
        queue: &AudioQueue,
        audio_stream_event_sender: Sender<AudioInterfaceEvent>,
    ) where
        T: Sample + FromSample<f32>,
    {
        for frame in output.chunks_exact_mut(channel_count) {
            let sample = queue.pop().unwrap_or_default();
            let left = sample.0 .0 as f32;
            let right = sample.1 .0 as f32;
            frame[0] = T::from_sample(left);
            if channel_count > 0 {
                frame[1] = T::from_sample(right);
            }
        }
        let capacity = queue.capacity();
        let len = queue.len();
        if len < capacity {
            let _ = audio_stream_event_sender.send(AudioInterfaceEvent::NeedsAudio(
                Instant::now(),
                capacity - len,
            ));
        }
    }

    fn send_reset(&self) {
        let _ = self.sender.send(AudioInterfaceEvent::Reset(
            self.sample_rate(),
            Arc::clone(&self.queue),
        ));
    }
}
