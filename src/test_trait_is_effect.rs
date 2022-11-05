// IsEffect = SourcesAudio + SinksAudio + TransformsAudio + MakesControlSink + MakesIsViewable
#[test]
fn test_is_effect_trait() {
    let s = instance();
    assert!(s.midi_sinks().is_empty());
}
