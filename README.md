# Groove

A digital audio workstation (DAW) engine.

## High-level project status

- There are CLI (command-line interface) and GUI (graphical user interface)
  versions of the app. The CLI is theoretically capable of producing a song, if
  tediously. The GUI mostly proves that I know how to write a GUI, but it's
  useless for anything else right now.
- Aside from the CLI workflow being difficult, there aren't many components --
  just a couple instruments, a few effects, and a controller (automation)
  system. So even if you liked the workflow, you don't have a rich library of
  tools to work with.

## Getting started (music producers)

1. Don't get your hopes up.
2. Download the [release](https://github.com/sowbug/groove/releases) for your OS
   and unzip it somewhere. If you're on an ARM Chromebook or a Raspberry Pi, try
   the `aarch64` build first, and if that doesn't work, try `armv7`. You can
   also try one of the installers (currently `.deb` for Linux and `.msi` for
   Windows).
3. Using the command line, `cd` to the directory you just unzipped.
4. Render `projects/demos/effects/drums-filtered-24db.json5` with `groove-cli`,
   passing the `--debug` flag. For Windows, that's `groove-cli --debug
   projects\demos\effects\drums-filtered-24db.json5`, and for Linux/OSX it's
   `./groove-cli --debug projects/demos/effects/drums-filtered-24db.json5`. You
   should hear a 707 beat through a rising low-pass filter. If you don't, file a
   bug.
5. Open `projects/demos/effects/drum-filtered-24db.json5` in your favorite text
   editor, and change `bpm: 128.0` to `bpm: 200.0`. Play the track again.
   Congratulations, you're now the world's newest
   [DnB](https://en.wikipedia.org/wiki/Drum_and_bass) producer.
6. Take a look at all the other projects in the `projects` directory. Render
   them, tweak them, and make new ones!
7. Launch the `groove-egui` executable. It won't do anything useful, but you
   should see a DAW-ish window appear, and if you press the play button, you
   should hear `projects/default.json5` played through your speakers. If not,
   please file a bug so I can be aware of GUI problems on different OSes.

## Getting started (developers)

I use VSCode on Ubuntu 22.04 LTS for development.

- Visit <https://rustup.rs/> to install the Rust toolchain. We build the
  official releases with `nightly`, but for now we aren't using anything more
  advanced than what `stable` provides, so it's OK to install just that one.
- If you're developing on Linux, `apt install` the packages you find buried
  somewhere in `.github/workflows/build.yml`.
- Also `apt install mold`, which provides faster linking during development. If
  you don't want to use mold, or can't, then comment out the section in
  `.cargo/config.toml` that specifies use of mold.
- `cargo build`, and then try the commands listed in the other Getting Started
  section. Or try `cargo install` if you want the current binaries installed in
  your PATH (not recommended).

### Useful developer tools (not specific to this project)

- `cargo-deb` produces Debian `.deb` packages from your crate.
- `cargo-expand` helps with macro debugging; try `cargo-expand --lib entities`
  in the `entities` subcrate. Also per [this
  advice](https://stackoverflow.com/a/63149819/344467) try `RUSTFLAGS="-Z
  macro-backtrace" cargo build --workspace` to get just a bit more info on macro
  issues.
- `cargo-license` lists crate licenses.
- `cargo-machete` helps find unused crates listed as `Cargo.toml` dependencies.
- `cargo-tree` lists crate dependencies.
- `cross` produces cross-compiled builds.

## Current Features

- Simple additive/subtractive synthesizer. Dual oscillators with a low-pass
  filter and LFO. Design target is to properly implement the patches listed in
  [Welsh's Synthesizer
  Cookbook](https://www.amazon.com/Welshs-Synthesizer-Cookbook-Programming-Universal/dp/B000ERHA4S/),
  3rd edition, by Fred Welsh ([website](https://synthesizer-cookbook.com/)). As
  of this README's last update, most of them sound OK, though they could all use
  some tuning. Portamento/unison aren't implemented, and some sounds are
  inoperable because I haven't yet implemented less common LFO routing paths.
- A single-operator FM synthesizer. The modulator even has its own envelope!
- Sampler. If it can figure out the root frequency from the WAV file's metadata,
  then it will play the sample at the right adjusted frequency for whichever
  note it's playing. That means there will be sampling artifacts.
- Sampler-based drumkit.
- Sequencer with a MIDI SMF reader (the MIDI reader is broken right now).
- A declarative project language, which makes it easy to produce songs in JSON5.
- A few audio effects (gain, limiter, bitcrusher, chorus, compressor, delay,
  reverb, filters). Some of them are just plain wrong.
- Basic automation.
- Output to WAV file or speaker.
- An [egui](https://www.egui.rs/)-based GUI that is read-only and very
  incomplete.
- Plenty of bugs.

## On the roadmap

- More of everything: synths, filters, effects. High-priority gaps to fill are a
  wavetable synth, a time-stretching sampler, and effects that deserve their
  name.
- A better automation design. The vision is intuitively configurable envelopes
  that can be used throughout the system.
- Scripting. I experimented with [rhai](https://rhai.rs/), but I don't know
  whether a JavaScript-y language is right for this domain. It might be better
  just to turn the whole thing into a crate and let others wrap it in scripting
  technology.
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
- [Nannou](https://nannou.cc/) is a library that aims to make it easy for
  artists to express themselves with simple, fast, reliable code.
- [Noise2Music](https://noise2music.github.io/): A series of diffusion models
  trained to generate high-quality 30-second music clips from text prompts.
- [Sonic Pi](https://sonic-pi.net/)
- [Soundraw](https://soundraw.io/) "Stop searching for the song you need. Create
  it."
- [SunVox](https://www.warmplace.ru/soft/sunvox/)
- [Supercollider](https://github.com/supercollider/supercollider): An audio
  server, programming language, and IDE for sound synthesis and algorithmic
  composition.
- [Surge XT](https://surge-synthesizer.github.io/): a free and open-source
  hybrid synthesizer.
- [Synth6581](https://www.raspberrypi.com/news/commodore-64-raspberry-pi-4-synth6581/)
- [SynthLab](https://www.willpirkle.com/synthlab-landing/)
- [Tidal Cycles](https://tidalcycles.org/): Live coding music with algorithmic
  patterns
- [Vital](https://github.com/mtytel/vital): a spectral warping wavetable
  synthesizer.
- [Welsh's Synthesizer Cookbook](https://synthesizer-cookbook.com/)
- [Zynthian](https://www.zynthian.org/) Open synth platform
