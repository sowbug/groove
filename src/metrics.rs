use dipstick::{stats_all, AtomicBucket, Input, InputScope, Marker, Stream, Timer};
use rustc_hash::FxHashMap;
use std::fmt::Debug;

pub(crate) struct DipstickWrapper {
    pub(crate) bucket: AtomicBucket,
    pub(crate) entity_count: Marker,
    pub(crate) gather_audio_fn_timer: Timer,
    pub(crate) mark_stack_loop_entry: Marker,
    pub(crate) mark_stack_loop_iteration: Marker,
    pub(crate) entity_audio_times: FxHashMap<usize, Timer>,
}
impl Debug for DipstickWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DipstickWrapper")
            .field("metrics", &false)
            .finish()
    }
}
impl Default for DipstickWrapper {
    fn default() -> Self {
        let bucket = AtomicBucket::default();
        Self {
            entity_count: bucket.marker("total entities"),
            gather_audio_fn_timer: bucket.timer("gather_audio"),
            mark_stack_loop_entry: bucket.marker("mark_stack_loop_entry"),
            mark_stack_loop_iteration: bucket.marker("mark_stack_loop_iteration"),
            entity_audio_times: Default::default(),
            bucket, // hehe do this last
        }
    }
}
impl DipstickWrapper {
    pub(crate) fn report(&self) {
        self.bucket.stats(stats_all);
        self.bucket
            .flush_to(&Stream::write_to_stdout().metrics())
            .unwrap();
    }
}
