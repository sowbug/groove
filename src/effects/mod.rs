pub mod filter;
pub mod gain;
pub mod limiter;
pub mod mixer;

#[cfg(test)]
mod tests {
    use crate::backend::devices::DeviceTrait;

    pub struct TestAlwaysTooLoudDevice {}
    impl DeviceTrait for TestAlwaysTooLoudDevice {
        fn get_audio_sample(&self) -> f32 {
            1.1
        }
    }

    pub struct TestAlwaysLoudDevice {}
    impl DeviceTrait for TestAlwaysLoudDevice {
        fn get_audio_sample(&self) -> f32 {
            1.
        }
    }

    pub struct TestAlwaysSilentDevice {}
    impl DeviceTrait for TestAlwaysSilentDevice {
        fn get_audio_sample(&self) -> f32 {
            0.
        }
    }
}
