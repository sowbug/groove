// Copyright (c) 2023 Mike Tsao. All rights reserved.

use crate::subscriptions::{EngineInput, EngineSubscription};
use cpal::{
    traits::{DeviceTrait, StreamTrait},
    FromSample, Sample as CpalSample, Stream, StreamConfig,
};
use crossbeam::{deque::Steal, queue::ArrayQueue};
use groove_core::StereoSample;
use groove_orchestration::{helpers::IOHelper, Performance};
use std::sync::{
    mpsc::{self, Sender},
    Arc, Condvar, Mutex,
};

/// Wraps the [cpal] crate to send audio to the PC's audio output.
pub struct AudioOutput {
    sample_rate: usize,
    ring_buffer: Arc<ArrayQueue<StereoSample>>,
    stream: Option<Stream>,
    sync_pair: Arc<(Mutex<bool>, Condvar)>,
    input_sender: Sender<EngineInput>,
}

// TODO: make this smart and not start playing audio until it has enough samples
// buffered up.
impl AudioOutput {
    // TODO: this should be more like 2 * 2700, based on empirically observed
    // typical callback size.
    const RING_BUFFER_CAPACITY: usize = EngineSubscription::ENGINE_BUFFER_SIZE * 48;

    pub(crate) fn new_with(
        ring_buffer: Arc<ArrayQueue<StereoSample>>,
        input_sender: Sender<EngineInput>,
    ) -> Self {
        Self {
            sample_rate: 0,
            ring_buffer,
            stream: None,
            sync_pair: Arc::new((Mutex::new(false), Condvar::new())),
            input_sender,
        }
    }

    // This is a public function because we need to create the queue in a different
    // threat from the AudioOutput struct
    pub(crate) fn create_ring_buffer() -> Arc<ArrayQueue<StereoSample>> {
        Arc::new(ArrayQueue::new(Self::RING_BUFFER_CAPACITY))
    }

    pub(crate) fn push(&mut self, sample: StereoSample) -> Result<(), StereoSample> {
        self.ring_buffer.push(sample)
    }

    pub(crate) fn push_buffer(&mut self, samples: &[StereoSample]) -> Result<(), StereoSample> {
        for sample in samples {
            if let Err(e) = self.ring_buffer.push(*sample) {
                return Result::Err(e);
            }
        }
        Ok(())
    }

    pub(crate) fn start(&mut self) {
        let device = IOHelper::default_output_device();
        let config = IOHelper::default_output_config(&device);
        let sample_format = config.sample_format();
        let channels: usize = config.channels() as usize;
        let config: StreamConfig = config.into();
        self.sample_rate = config.sample_rate.0 as usize;

        let sync_pair_clone = Arc::clone(&self.sync_pair);
        let ring_buffer_clone = Arc::clone(&self.ring_buffer);

        // It's weird that I had to make a clone to be cloned. The problem was
        // referring to self inside the closures below, even though it was just
        // to make a clone.
        let input_sender_clone = self.input_sender.clone();

        let err_fn = |err| eprintln!("an error occurred on the output audio stream: {err}");
        if let Ok(result) = match sample_format {
            cpal::SampleFormat::F32 => device.build_output_stream(
                &config,
                move |data, output_callback_info| {
                    Self::sample_from_queue::<f32>(
                        channels,
                        &ring_buffer_clone,
                        &sync_pair_clone,
                        input_sender_clone.clone(),
                        data,
                        output_callback_info,
                    )
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::I16 => device.build_output_stream(
                &config,
                move |data, output_callback_info| {
                    Self::sample_from_queue::<i16>(
                        channels,
                        &ring_buffer_clone,
                        &sync_pair_clone,
                        input_sender_clone.clone(),
                        data,
                        output_callback_info,
                    )
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::U16 => device.build_output_stream(
                &config,
                move |data, output_callback_info| {
                    Self::sample_from_queue::<u16>(
                        channels,
                        &ring_buffer_clone,
                        &sync_pair_clone,
                        input_sender_clone.clone(),
                        data,
                        output_callback_info,
                    )
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::I8 => todo!(),
            cpal::SampleFormat::I32 => todo!(),
            cpal::SampleFormat::I64 => todo!(),
            cpal::SampleFormat::U8 => todo!(),
            cpal::SampleFormat::U32 => todo!(),
            cpal::SampleFormat::U64 => todo!(),
            cpal::SampleFormat::F64 => todo!(),
            _ => todo!(),
        } {
            self.stream = Some(result);
            self.play();
        }
    }

    /// End the audio output and this thread.
    pub(crate) fn stop(&mut self) {
        if let Some(stream) = &self.stream {
            if stream.pause().is_ok() {
                // hooray again
            }
        }
        let lock = &self.sync_pair.0;
        let cvar = &self.sync_pair.1;
        let mut finished = lock.lock().unwrap();
        *finished = true;
        cvar.notify_one();
    }

    /// Ask the audio output to stop handling the stream.
    #[allow(dead_code)]
    pub(crate) fn pause(&mut self) {
        if let Some(stream) = &self.stream {
            let _ = stream.pause();
        }
    }

    /// Ask the audio output to start handling the stream.
    pub(crate) fn play(&mut self) {
        if let Some(stream) = &self.stream {
            let _ = stream.play();
        }
    }

    /// Returns the percentage of the audio ring buffer that's currently filled
    /// with good data. 0.0 means we're about to experience an audio-device
    /// underrun. 1.0 means the engine isn't going to be asked to produce
    /// samples right now, because we wouldn't have anywhere to put it.
    pub(crate) fn buffer_utilization(&self) -> f64 {
        if self.ring_buffer.capacity() == 0 {
            0.0
        } else {
            self.ring_buffer.len() as f64 / self.ring_buffer.capacity() as f64
        }
    }

    fn sample_from_queue<T: cpal::Sample>(
        channels: usize,
        queue: &Arc<ArrayQueue<StereoSample>>,
        sync_pair: &Arc<(Mutex<bool>, Condvar)>,
        input_sender: Sender<EngineInput>,
        data: &mut [T],
        _info: &cpal::OutputCallbackInfo,
    ) where
        T: CpalSample + FromSample<f32>,
    {
        // On my Beelink SER4 Ryzen 7 4700U running Ubuntu 20.04 with standard
        // sound that I assume is PulseAudio, this callback requests around
        // 2,600-2,700 samples per callback. That is very close to 16
        // milliseconds of audio at 44.1KHz. That's a larger request than I
        // expected. It means our buffer size targets should be bigger than we
        // had (3 x 1K is a good guess), and our latency expectations should be
        // more forgiving. Now I feel in my bones why
        // [JACK](https://jackaudio.org/) exists!

        for frame in data.chunks_mut(channels) {
            let mut sample = StereoSample::default();
            // TODO: do we really need to check on every spin of the loop?
            if let Ok(finished) = sync_pair.0.lock() {
                let finished: bool = *finished;
                if !finished {
                    if let Some(popped_sample) = queue.pop() {
                        sample = popped_sample;
                    }
                }
            }
            // I haven't really looked at what a frame is, so although this
            // works, I'm not sure it's right. TODO: spend more than 10 minutes
            // looking at cpal to understand how to use it robustly.
            let left_value = T::from_sample(sample.0 .0 as f32);
            let right_value = T::from_sample(sample.1 .0 as f32);
            frame[0] = left_value;
            if channels > 1 {
                frame[1] = right_value;
            }
        }

        // TODO: this algorithm should take into account the typical request
        // size of this callback. If we know we're going to be asked for around
        // 2,700 frames next time, it's a waste to produce any amount that still
        // leaves us short of a full request.
        let len = queue.len();
        if len < Self::RING_BUFFER_CAPACITY {
            let shortfall = Self::RING_BUFFER_CAPACITY - len;
            let shortfall_in_buffers = (shortfall / EngineSubscription::ENGINE_BUFFER_SIZE) as u8;
            if shortfall_in_buffers > 0 {
                let _ = input_sender.send(EngineInput::GenerateAudio(shortfall_in_buffers));
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) fn sample_rate(&self) -> usize {
        self.sample_rate
    }

}

/// This utility function assumes the caller is cool with blocking.
pub fn send_performance_to_output_device(performance: &Performance) -> anyhow::Result<()> {
    let (input_sender, _) = mpsc::channel::<EngineInput>();
    let mut audio_output = AudioOutput::new_with(AudioOutput::create_ring_buffer(), input_sender);
    let stealer = performance.worker.stealer();
    audio_output.start();
    while let Steal::Success(sample) = stealer.steal() {
        loop {
            if audio_output.push(sample).is_ok() {
                break;
            } else {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
    }
    Ok(())
}
