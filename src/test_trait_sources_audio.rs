use crate::clock::Clock;
use crate::common::MONO_SAMPLE_SILENCE;
use crate::traits::SourcesAudio;

#[test]
fn test_trait_sources_audio() {
    let mut s = instance();
    assert_eq!(s.source_audio(&Clock::new()), MONO_SAMPLE_SILENCE);
}
