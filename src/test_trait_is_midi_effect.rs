// IsMidiEffect = SourcesMidi + SinksMidi + WatchesClock + MakesControlSink + MakesIsViewable
#[test]
fn test_is_midi_effect_trait() {
    let s = instance();
    assert!(s.midi_sinks().is_empty());
}
