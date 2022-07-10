pub mod clock;
pub mod envelopes;
pub mod filter;
pub mod gain;
pub mod lfos;
pub mod limiter;
pub mod mixer;
pub mod oscillators;

#[cfg(test)]
pub mod tests {
    pub struct TestAlwaysTooLoudDevice {}
    impl TestAlwaysTooLoudDevice {
        pub fn new() -> Self {
            Self {}
        }
        pub fn get_audio_sample(&self) -> f32 {
            1.1
        }
    }

    pub struct TestAlwaysLoudDevice {}
    impl TestAlwaysLoudDevice {
        pub fn new() -> Self {
            Self {}
        }
        pub fn get_audio_sample(&self) -> f32 {
            1.
        }
    }

    pub struct TestAlwaysSilentDevice {}
    impl TestAlwaysSilentDevice {
        pub fn new() -> Self {
            Self {}
        }
        pub fn get_audio_sample(&self) -> f32 {
            0.
        }
    }
}
