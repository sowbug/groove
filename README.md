# Groove

A digital audio workstation (DAW) engine.

## Getting started (music producers)

1. Don't get your hopes up.
2. Download the [release](https://github.com/sowbug/groove/releases) for your OS
   and unzip it somewhere.
3. Using the command line, `cd` to the directory you just unzipped.
4. Render `projects/drum-filtered.yaml` with `groove-cli`. For Windows, that's
   `groove-cli projects\drum-filtered.yaml`, and for Linux/OSX it's
   `./groove-cli projects/drum-filtered.yaml`). You should hear a 707 beat
   through a rising low-pass filter. If you don't, file a bug.
5. Open `projects/drum-filtered.yaml` in your favorite text editor, and change
   `bpm: 128.0` to `bpm: 200.0`. Play the track again. Congratulations, you're
   now the world's newest [DnB](https://en.wikipedia.org/wiki/Drum_and_bass)
   producer.
6. Take a look at all the other projects in the `projects` directory. Render
   them, tweak them, and make new ones!
6. Launch the `groove-gui` executable. It won't do anything useful, but you
   should see a DAW-ish window appear. If not, please file a bug so I can be
   aware of GUI problems on different OSes.

## Getting started (developers)

I use VSCode on Ubuntu 20.04 for development.

- Visit <https://rustup.rs/> to install the Rust toolchain.
- `rustup default nightly` (we're using trait upcasting and specialization).
- `apt install` the packages listed in `.github/workflows/build.yml`
- `cargo build`, and then try the command listed in the other Getting Started
  section.

## High-level project status

- There are CLI (command-line interface) and GUI (graphical user interface)
  versions of the app. The CLI is theoretically capable of producing a song, if
  tediously. The GUI mostly proves that I know how to write a GUI, but it's
  useless for anything else right now.
- Aside from the CLI workflow being difficult, there aren't many components --
  just a couple instruments, a few effects, and a controller (automation)
  system. So even if you liked the workflow, you don't have a rich library of
  tools to work with.

## Current Features

- Simple subtractive synthesizer. Dual oscillators with a low-pass filter and
  LFO. Design target is to properly implement the patches listed in [Welsh's
  Synthesizer
  Cookbook](https://www.amazon.com/Welshs-Synthesizer-Cookbook-Programming-Universal/dp/B000ERHA4S/),
  3rd edition, by Fred Welsh ([website](https://synthesizer-cookbook.com/)).
- Sampler. Doesn't yet know anything about tones; in other words, it just plays
  back WAV data at the original speed, which works fine for a drumkit but not so
  well for tonal sounds that you expect to use melodically.
- Sequencer with a MIDI SMF reader (the MIDI reader is broken right now).
- A declarative project language, which makes it easy to produce songs in YAML
  or JSON format (JSON only in theory, but we get it for free thanks to
  [serde](https://serde.rs/)).
- A few audio effects (gain, limiter, bitcrusher, filters).
- Basic automation.
- Output to WAV file or speaker.
- A very simple [Iced](https://iced.rs/)-based GUI, which doesn't do much yet,
  but has been useful to constrain the architecture to something that can
  eventually be integrated with a GUI.
- Plenty of bugs.

## On the roadmap

- More of everything: synths, filters, effects. High-priority gaps to fill are a
  wavetable synth, an FM synth, a proper sampler, 24db filters, and must-have
  effects like chorus, reverb, delay, etc.
- A better automation design. The vision is intuitively configurable envelopes
  that can be used throughout the system.
- Scripting. Currently I'm experimenting with [rhai](https://rhai.rs/), but I
  don't know whether a JavaScript-y language is right for this domain.
- MIDI input/output (this works very crudely right now; all MIDI notes are
  routed externally, and a small white dot appears in the GUI when it detects
  any MIDI input).
- A GUI that at a minimum provides visual feedback on the project (read-only),
  and ideally also allows editing of the project (interactive).
- Audio tools. Visualizing audio in a frequency-domain format is top priority.
- Better sound quality. Since the basic oscillators are generated using pure
  algorithms (sine, sawtooth, square, etc.), they suffer from aliasing and
  unwanted transients. The efficient state-of-the-art solution these days seems
  to be to generate them offline and delegate to wavetable synthesis.
- Plugin support (VST, [CLAP](https://u-he.com/community/clap/)). I think it's
  possible to carry out the vision even if components of a project are
  closed-source. If a plugin's persistent state is a set of values that can be
  represented in a config file, then it should fit in.
- Assistance with song composition. Suggesting pleasant chord progressions,
  identifying the current scale and highlighting the full set of consonant
  notes, a [hyperspace](https://en.wikipedia.org/wiki/Asteroids_(video_game))
  button to suggest new directions, etc.
- A review of other DAW features, then playing catchup.

## Other projects/resources of interest

- [MiniDexed](https://github.com/probonopd/MiniDexed)
- [minisynth](https://github.com/rsta2/minisynth)
- [Dirtywave M8](https://dirtywave.com/)
- [BasicSynth](https://basicsynth.com/)
- [Welsh's Synthesizer Cookbook](https://synthesizer-cookbook.com/)
- [SynthLab](https://www.willpirkle.com/synthlab-landing/)
- [Glicol](https://github.com/chaosprint/glicol) is consistent with the vision.
- [Sonic Pi](https://sonic-pi.net/), which I somehow missed until just now.
