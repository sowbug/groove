# Groove

A digital audio workstation (DAW) engine.

## Getting started (music producers)

1. Don't get your hopes up.
2. Download the [release](https://github.com/sowbug/groove/releases) for your OS
   and unzip it somewhere. If you're on an ARM Chromebook or a Raspberry Pi, try
   the `aarch64` build first, and if that doesn't work, try `armv7`.
3. Using the command line, `cd` to the directory you just unzipped.
4. Render `projects/drums-filtered.yaml` with `groove-cli`. For Windows, that's
   `groove-cli projects\drums-filtered.yaml`, and for Linux/OSX it's
   `./groove-cli projects/drums-filtered.yaml`). You should hear a 707 beat
   through a rising low-pass filter. If you don't, file a bug.
5. Open `projects/drum-filtered.yaml` in your favorite text editor, and change
   `bpm: 128.0` to `bpm: 200.0`. Play the track again. Congratulations, you're
   now the world's newest [DnB](https://en.wikipedia.org/wiki/Drum_and_bass)
   producer.
6. Take a look at all the other projects in the `projects` directory. Render
   them, tweak them, and make new ones!
7. Launch the `groove-gui` executable. It won't do anything useful, but you
   should see a DAW-ish window appear, and if you press the play button, you
   should hear `projects/default.yaml` played through your speakers. If not,
   please file a bug so I can be aware of GUI problems on different OSes.

## Getting started (developers)

I use VSCode on Ubuntu 20.04 for development.

- Visit <https://rustup.rs/> to install the Rust toolchain. We build the
  official releases with `nightly`, but for now we aren't using anything more
  advanced than what `stable` provides.
- If you're developing on Linux, `apt install` the packages you find buried
  somewhere in `.github/workflows/build.yml`
- `cargo build`, and then try the commands listed in the other Getting Started
  section. Or try `cargo install` if you want the current binaries installed in
  your PATH (not recommended).

### Useful developer tools

- `cargo-deb` produces Debian `.deb` packages from your crate.
- `cargo-expand` helps with macro debugging.
- `cargo-license` lists crate licenses.
- `cargo-machete` helps find unused crates listed as `Cargo.toml` dependencies.
- `cargo-tree` lists crate dependencies.
- `cross` produces cross-compiled builds.

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

- Simple additive/subtractive synthesizer. Dual oscillators with a low-pass
  filter and LFO. Design target is to properly implement the patches listed in
  [Welsh's Synthesizer
  Cookbook](https://www.amazon.com/Welshs-Synthesizer-Cookbook-Programming-Universal/dp/B000ERHA4S/),
  3rd edition, by Fred Welsh ([website](https://synthesizer-cookbook.com/)). As
  of this README's last update, most of them sound OK, though they could all use
  some tuning, and portamento/unison aren't implemented.
- Sampler. Doesn't yet know anything about tones; in other words, it just plays
  back WAV data at the original speed, which works fine for a drumkit but not so
  well for tonal sounds that you expect to use melodically.
- Sequencer with a MIDI SMF reader (the MIDI reader is broken right now).
- A declarative project language, which makes it easy to produce songs in YAML
  or JSON format (JSON only in theory, but we get it for free thanks to
  [serde](https://serde.rs/)).
- A few audio effects (gain, limiter, bitcrusher, chorus, compressor, delay,
  filters). Some of them are just plain wrong.
- Basic automation.
- Output to WAV file or speaker.
- A very simple [Iced](https://iced.rs/)-based GUI.
- Plenty of bugs.

## On the roadmap

- More of everything: synths, filters, effects. High-priority gaps to fill are a
  wavetable synth, an FM synth, a proper sampler, and effects that deserve their
  name.
- A better automation design. The vision is intuitively configurable envelopes
  that can be used throughout the system.
- Scripting. Currently I'm experimenting with [rhai](https://rhai.rs/), but I
  don't know whether a JavaScript-y language is right for this domain. It might
  be better just to turn the whole thing into a crate and let others wrap it in
  scripting technology.
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
- Plugin support (VST, [CLAP](https://u-he.com/community/clap/)).
- Assistance with song composition. Suggesting pleasant chord progressions,
  identifying the current scale and highlighting the full set of consonant
  notes, a [hyperspace](https://en.wikipedia.org/wiki/Asteroids_(video_game))
  button to suggest new directions, etc.
- A review of other DAW features, then playing catchup.

## Other projects/resources of interest

Many of these overlap with this project's goals. The [Baader–Meinhof
phenomenon](https://en.wikipedia.org/wiki/Frequency_illusion) is marvelous.

- [Acoustico](https://github.com/rmichela/Acoustico)
- [BasicSynth](https://basicsynth.com/)
- [Bela](https://bela.io/)
- [Bespoke Synth](https://www.bespokesynth.com/)
- [Dirtywave M8](https://dirtywave.com/)
- [Electro-Smith Daisy](https://www.electro-smith.com/daisy)
- [GNU Octave](https://octave.org/) for prototyping signals
- [Glicol](https://github.com/chaosprint/glicol)
- [Kiro Synth](https://github.com/chris-zen/kiro-synth)
- [LMN 3](https://github.com/FundamentalFrequency) and
  [Tracktion](https://github.com/Tracktion/tracktion_engine)
- [MicroDexed-touch](https://codeberg.org/positionhigh/MicroDexed-touch)
- [MicroDexed](https://www.parasitstudio.de/)
- [MilkyTracker](https://milkytracker.org/about/) is an open source,
  multi-platform music application for creating .MOD and .XM module files.
- [MiniDexed](https://github.com/probonopd/MiniDexed)
- [minisynth](https://github.com/rsta2/minisynth)
- [mt32-pi](https://github.com/dwhinham/mt32-pi): A baremetal kernel that turns
  your Raspberry Pi 3 or later into a Roland MT-32 emulator and SoundFont
  synthesizer based on Circle, Munt, and FluidSynth.
- [musikcube](https://github.com/clangen/musikcube) a cross-platform,
  terminal-based music player, audio engine, metadata indexer, and server in c++
- [Nannou](https://nannou.cc/) is a library that aims to make it easy for artists to express themselves with simple, fast, reliable code.
- [Noise2Music](https://noise2music.github.io/): A series of diffusion models
  trained to generate high-quality 30-second music clips from text prompts.
- [Sonic Pi](https://sonic-pi.net/)
- [Soundraw](https://soundraw.io/) "Stop searching for the song you need. Create
  it."
- [SunVox](https://www.warmplace.ru/soft/sunvox/)
- [Supercollider](https://github.com/supercollider/supercollider): An audio
  server, programming language, and IDE for sound synthesis and algorithmic
  composition.
- [Surge XT](https://surge-synthesizer.github.io/) a free and open-source hybrid
  synthesizer.
- [Synth6581](https://www.raspberrypi.com/news/commodore-64-raspberry-pi-4-synth6581/)
- [SynthLab](https://www.willpirkle.com/synthlab-landing/)
- [Vital](https://github.com/mtytel/vital): a spectral warping wavetable
  synthesizer.
- [Welsh's Synthesizer Cookbook](https://synthesizer-cookbook.com/)
- [Zynthian](https://www.zynthian.org/) Open synth platform
