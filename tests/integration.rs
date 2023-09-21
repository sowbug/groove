// Copyright (c) 2023 Mike Tsao. All rights reserved.

// use ensnare::prelude::*;
// use groove_core::{util::tests::TestOnlyPaths, SAMPLE_BUFFER_SIZE};
// use groove_orchestration::helpers::IOHelper;
// use groove_utils::{PathType, Paths};
// use std::{fs::File, io::prelude::*, path::Path, time::Instant};

#[cfg(obsolete)]
#[test]
fn project_loads_and_parses() {
    let mut paths = Paths::default();
    paths.push_hive(&Paths::hive(PathType::Test));

    let path = Path::new("kitchen-sink.json");
    let json = paths
        .search_and_read_to_string(path)
        .unwrap_or_else(|err| panic!("loading JSON failed: {:?}", err));
    let song_settings = SongSettings::new_from_json5(json.as_str())
        .unwrap_or_else(|err| panic!("parsing settings for {} failed: {:?}", path.display(), err));
    let mut orchestrator = song_settings
        .instantiate(&paths, false)
        .unwrap_or_else(|err| panic!("instantiation failed: {:?}", err));
    orchestrator.update_sample_rate(SampleRate::DEFAULT);
    let mut sample_buffer = [StereoSample::SILENCE; SAMPLE_BUFFER_SIZE];
    if let Ok(samples) = orchestrator.run(&mut sample_buffer) {
        assert!(
            !samples.is_empty(),
            "Orchestrator reported successful performance, but performance is empty."
        );

        assert!(
            samples
                .iter()
                .any(|sample| { *sample != StereoSample::SILENCE }),
            "Performance contains only silence."
        );
    } else {
        panic!("run failed")
    }
}

#[cfg(obsolete)]
#[test]
#[ignore = "orchestrator - control_message_for_index is incomplete. re-enable when macroized"]
fn spit_out_perf_data() {
    let mut paths = Paths::default();
    paths.push_hive(&Paths::hive(PathType::Test));

    let path = Path::new("perf-1.json5");
    let contents = paths
        .search_and_read_to_string(path)
        .unwrap_or_else(|err| panic!("loading project failed: {:?}", err));
    let song_settings = SongSettings::new_from_json5(contents.as_str())
        .unwrap_or_else(|err| panic!("parsing settings for {} failed: {:?}", path.display(), err));
    let mut orchestrator = song_settings
        .instantiate(&paths, false)
        .unwrap_or_else(|err| panic!("instantiation failed: {:?}", err));

    let start_instant = Instant::now();
    let mut samples = [StereoSample::SILENCE; SAMPLE_BUFFER_SIZE];
    let performance = orchestrator
        .run_performance(&mut samples, false)
        .unwrap_or_else(|err| panic!("performance failed: {:?}", err));
    let elapsed = start_instant.elapsed();
    let frame_count = performance.worker.len();

    let mut out_path = TestOnlyPaths::writable_out_path();
    out_path.push("perf-output.txt");
    let mut file = File::create(out_path).unwrap();
    let output = format!(
        "Elapsed    : {:0.3}s\n\
Frames     : {}\n\
Frames/msec: {:.2?} (goal >{:.2?})\n\
usec/frame : {:.2?} (goal <{:.2?})",
        elapsed.as_secs_f32(),
        frame_count,
        frame_count as f32 / start_instant.elapsed().as_millis() as f32,
        performance.sample_rate.value() as f32 / 1000.0,
        start_instant.elapsed().as_micros() as f32 / frame_count as f32,
        1000000.0 / performance.sample_rate.value() as f32
    );
    let _ = file.write(output.as_bytes());

    let mut path = TestOnlyPaths::data_path();
    path.push("perf-1.wav");
    assert!(IOHelper::send_performance_to_file(&performance, &path).is_ok());
}

#[cfg(obsolete)]
#[test]
fn patching_to_device_with_no_input_fails_with_proper_error() {
    let mut paths = Paths::default();
    paths.push_hive(&Paths::hive(PathType::Test));

    let path = Path::new("instruments-have-no-inputs.json5");
    let contents = paths
        .search_and_read_to_string(path)
        .unwrap_or_else(|err| panic!("loading project failed: {:?}", err));
    let song_settings = SongSettings::new_from_json5(contents.as_str())
        .unwrap_or_else(|err| panic!("parsing settings for {} failed: {:?}", path.display(), err));
    let r = song_settings.instantiate(&paths, false);
    assert_eq!(
        r.unwrap_err().to_string(),
        "Input device doesn't transform audio and can't be patched from output device"
    );
}
